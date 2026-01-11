use std::time::Duration;

use chrono::{Timelike, Utc};
use tracing::{info, instrument};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_target(false)
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let ble_receiver = tokio::spawn(ble_receiver());
    let sqlite_writer = tokio::spawn(sqlite_writer());
    let external_db_writer = tokio::spawn(external_db_writer());

    let _ = tokio::join!(ble_receiver, sqlite_writer, external_db_writer);
}

/// Task to receive BLE data and store it in in-memory database
#[instrument]
async fn ble_receiver() {
    loop {
        let now_seconds = Utc::now().second();

        // Write data to in-memory database during hh:mm:50 to hh:mm:10 window
        if 50 <= now_seconds || now_seconds <= 10 {
            info!("BLE received");
        } else {
            info!("BLE skipped");
        }

        tokio::time::sleep(Duration::from_secs(2)).await;
    }
}

/// Task to write the data closest to 0 seconds from in-memory database to SQLite
#[instrument]
async fn sqlite_writer() {
    loop {
        info!("write to SQLite");

        // Wait until next hh:mm:11
        tokio::time::sleep(Duration::from_secs(
            (59 - Utc::now().second() as u64 + 11) % 60 + 1,
        ))
        .await;
    }
}

/// Task to write data from SQLite to external database
#[instrument]
async fn external_db_writer() {
    loop {
        info!("write to external DB");

        // Wait until next hh:mm:12
        tokio::time::sleep(Duration::from_secs(
            (59 - Utc::now().second() as u64 + 12) % 60 + 1,
        ))
        .await;
    }
}
