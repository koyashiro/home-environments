use std::{env, fs, path::PathBuf, sync::Arc, time::Duration};

use chrono::{Timelike, Utc};
use sqlx::{SqlitePool, sqlite::SqlitePoolOptions};
use tokio::sync::Mutex;
use tracing::{debug, error, info, instrument};

use crate::InMemoryDb;

fn db_path() -> Option<PathBuf> {
    let home = env::var("HOME").ok()?;
    Some(
        PathBuf::from(home)
            .join(".local")
            .join("share")
            .join("ble-ingester")
            .join("data.db"),
    )
}

/// Task to write the data closest to 0 seconds from in-memory database to SQLite database
#[instrument(skip_all)]
pub async fn sqlite_writer(in_memory_db: Arc<Mutex<InMemoryDb>>) {
    let Some(path) = db_path() else {
        error!("failed to get database path: HOME not set");
        return;
    };

    if let Some(parent) = path.parent()
        && let Err(e) = fs::create_dir_all(parent)
    {
        error!("failed to create database directory: {e}");
        return;
    }

    let url = format!("sqlite:{}?mode=rwc", path.display());
    let pool = match SqlitePoolOptions::new()
        .max_connections(1)
        .connect(&url)
        .await
    {
        Ok(pool) => pool,
        Err(e) => {
            error!("failed to connect to SQLite: {e}");
            return;
        }
    };

    if let Err(e) = create_tables(&pool).await {
        error!("failed to create tables: {e}");
        return;
    }

    loop {
        // Wait until next hh:mm:11
        tokio::time::sleep(Duration::from_secs(
            (59 - Utc::now().second() as u64 + 11) % 60 + 1,
        ))
        .await;

        let mut in_memory_db = in_memory_db.lock().await;

        // Write SwitchBot measurements
        for (mac, measurements) in in_memory_db.switchbot_db.iter_mut() {
            let mut keys_to_remove = Vec::new();

            for (measured_at, (_, m)) in measurements.iter() {
                let result = sqlx::query(
                    r#"
                    INSERT OR REPLACE INTO switchbot_measurements (device_id, measured_at, temperature_dc, humidity_p, co2_ppm, light_level)
                    VALUES (?, ?, ?, ?, ?, ?)
                    "#,
                )
                .bind(mac.to_string())
                .bind(measured_at.to_rfc3339())
                .bind(m.temperature_dc as i32)
                .bind(m.humidity_p as i32)
                .bind(m.co2_ppm.map(|v| v as i32))
                .bind(m.light_level.map(|v| v as i32))
                .execute(&pool)
                .await;

                match result {
                    Ok(_) => {
                        debug!(
                            "inserted switchbot measurement: mac={mac}, measured_at={measured_at}"
                        );
                        keys_to_remove.push(*measured_at);
                    }
                    Err(e) => {
                        error!("failed to insert switchbot measurement: {e}");
                    }
                }
            }

            for key in keys_to_remove {
                measurements.remove(&key);
            }
        }

        // Write RATOC Systems measurements
        for (mac, measurements) in in_memory_db.ratoc_systems_db.iter_mut() {
            let mut keys_to_remove = Vec::new();

            for (measured_at, (_, m)) in measurements.iter() {
                let result = sqlx::query(
                    r#"
                    INSERT OR REPLACE INTO ratoc_systems_measurements (device_id, measured_at, relay, voltage_v, current_ma, power_w)
                    VALUES (?, ?, ?, ?, ?, ?)
                    "#,
                )
                .bind(mac.to_string())
                .bind(measured_at.to_rfc3339())
                .bind(m.relay)
                .bind(m.voltage_dv as i32)
                .bind(m.current_ma as i32)
                .bind(m.power_mw as i32)
                .execute(&pool)
                .await;

                match result {
                    Ok(_) => {
                        debug!(
                            "inserted ratoc_systems measurement: mac={mac}, measured_at={measured_at}"
                        );
                        keys_to_remove.push(*measured_at);
                    }
                    Err(e) => {
                        error!("failed to insert ratoc_systems measurement: {e}");
                    }
                }
            }

            for key in keys_to_remove {
                measurements.remove(&key);
            }
        }

        info!("write to SQLite db");
    }
}

async fn create_tables(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS switchbot_measurements (
            device_id TEXT NOT NULL,
            measured_at TEXT NOT NULL,
            temperature_dc INTEGER NOT NULL,
            humidity_p INTEGER NOT NULL,
            co2_ppm INTEGER,
            light_level INTEGER,
            PRIMARY KEY (device_id, measured_at)
        )
        "#,
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS ratoc_systems_measurements (
            device_id TEXT NOT NULL,
            measured_at TEXT NOT NULL,
            relay INTEGER NOT NULL,
            voltage_v INTEGER NOT NULL,
            current_ma INTEGER NOT NULL,
            power_w INTEGER NOT NULL,
            PRIMARY KEY (device_id, measured_at)
        )
        "#,
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(())
}
