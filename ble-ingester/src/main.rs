use std::{
    collections::{BTreeMap, HashMap},
    fmt,
    sync::Arc,
    time::Duration,
};

use chrono::{DateTime, DurationRound, TimeDelta, Timelike, Utc};
use sqlx::{SqlitePool, sqlite::SqlitePoolOptions};
use tokio::sync::{Mutex, mpsc};
use tracing::{debug, error, info, instrument};
use tracing_subscriber::{EnvFilter, fmt::time::ChronoLocal};

const RECEIVE_RANGE: u8 = 10;

#[derive(PartialEq, Eq, Clone, Copy, Hash)]
pub struct MacAddress([u8; 6]);

impl MacAddress {
    pub const fn new(v: [u8; 6]) -> MacAddress {
        MacAddress(v)
    }

    pub const fn as_bytes(&self) -> &[u8; 6] {
        &self.0
    }
}

impl From<[u8; 6]> for MacAddress {
    fn from(v: [u8; 6]) -> Self {
        MacAddress::new(v)
    }
}

impl AsRef<[u8; 6]> for MacAddress {
    fn as_ref(&self) -> &[u8; 6] {
        self.as_bytes()
    }
}

impl fmt::Debug for MacAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
            self.0[0], self.0[1], self.0[2], self.0[3], self.0[4], self.0[5]
        )
    }
}

impl fmt::Display for MacAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
            self.0[0], self.0[1], self.0[2], self.0[3], self.0[4], self.0[5]
        )
    }
}

#[derive(Debug, Default)]
pub struct InMemoryDb {
    pub switchbot_db: HashMap<MacAddress, BTreeMap<DateTime<Utc>, (u8, SwitchBotMeasurements)>>,
    pub ratoc_systems_db:
        HashMap<MacAddress, BTreeMap<DateTime<Utc>, (u8, RatocSystemsMeasurements)>>,
}

impl InMemoryDb {
    pub fn new() -> InMemoryDb {
        InMemoryDb {
            switchbot_db: [([0x02, 0x00, 0x00, 0x00, 0x00, 0x01].into(), BTreeMap::new())].into(),
            ratoc_systems_db: HashMap::new(),
        }
    }
}

#[derive(Debug)]
pub struct SwitchBotMeasurements {
    pub temperature_celsius: f32,
    pub humidity_percent: u8,
    pub co2_ppm: Option<u16>,
    pub light_level: Option<u8>,
}

#[derive(Debug)]
pub struct RatocSystemsMeasurements {
    pub relay: bool,
    pub voltage_v: u16,
    pub current_ma: u16,
    pub power_w: u32,
}

#[derive(Debug)]
pub enum BleData {
    SwitchBot {
        mac: MacAddress,
        measured_at: DateTime<Utc>,
        measurements: SwitchBotMeasurements,
    },
    RatocSystems {
        mac: MacAddress,
        measured_at: DateTime<Utc>,
        measurements: RatocSystemsMeasurements,
    },
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_target(false)
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_timer(ChronoLocal::new("%Y-%m-%dT%H:%M:%S%.3f%:z".to_string()))
        .init();

    let (tx, rx) = mpsc::channel::<BleData>(128);
    let in_memory_db: Arc<Mutex<InMemoryDb>> = Arc::new(Mutex::new(InMemoryDb::new()));

    let ble_receiver_handle = tokio::spawn(ble_receiver(tx));
    let in_memory_db_writer_handle = tokio::spawn(in_memory_db_writer(rx, in_memory_db.clone()));
    let sqlite_writer_handle = tokio::spawn(sqlite_writer(in_memory_db.clone()));
    let external_db_writer_handle = tokio::spawn(external_db_writer());

    let _ = tokio::join!(
        ble_receiver_handle,
        in_memory_db_writer_handle,
        sqlite_writer_handle,
        external_db_writer_handle
    );
}

/// Task to receive BLE data
#[instrument(skip_all)]
async fn ble_receiver(tx: mpsc::Sender<BleData>) {
    loop {
        // dummy data
        tokio::time::sleep(Duration::from_secs(2)).await;
        let mac = MacAddress::new([0x02, 0x00, 0x00, 0x00, 0x00, 0x01]);
        let measurements = SwitchBotMeasurements {
            temperature_celsius: 23.2,
            humidity_percent: 33,
            co2_ppm: Some(526),
            light_level: None,
        };

        let measured_at = Utc::now();

        let measured_at_seconds = measured_at.second();
        if (RECEIVE_RANGE as u32) < measured_at_seconds
            && measured_at_seconds < 60 - (RECEIVE_RANGE as u32)
        {
            debug!("BLE skipped");

            continue;
        }

        debug!("BLE received");

        let data = BleData::SwitchBot {
            mac,
            measured_at,
            measurements,
        };

        if let Err(e) = tx.send(data).await {
            error!("failed to send BLE data: {e}");
            break;
        }
    }
}

/// Task to store it in in-memory database
#[instrument(skip_all)]
async fn in_memory_db_writer(
    mut rx: mpsc::Receiver<BleData>,
    in_memory_db: Arc<Mutex<InMemoryDb>>,
) {
    while let Some(data) = rx.recv().await {
        match data {
            BleData::SwitchBot {
                mac,
                measured_at,
                measurements,
            } => {
                let mut in_memory_db = in_memory_db.lock().await;
                let Some(m) = in_memory_db.switchbot_db.get_mut(&mac) else {
                    continue;
                };

                let Ok(rounded_measured_at) = measured_at.duration_round(TimeDelta::minutes(1))
                else {
                    continue;
                };

                let diff = (measured_at - rounded_measured_at)
                    .num_seconds()
                    .unsigned_abs() as u8;
                if let Some((existing_diff, _)) = m.get(&rounded_measured_at)
                    && diff >= *existing_diff
                {
                    continue;
                }

                m.insert(rounded_measured_at, (diff, measurements));

                debug!("write to in-memory db");
            }
            BleData::RatocSystems {
                mac: _,
                measured_at: _,
                measurements: _,
            } => {
                // TODO: handle RatocSystems data
            }
        }
    }
}

/// Task to write the data closest to 0 seconds from in-memory database to SQLite database
#[instrument(skip_all)]
async fn sqlite_writer(in_memory_db: Arc<Mutex<InMemoryDb>>) {
    let pool = match SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite:data.db?mode=rwc")
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
                    INSERT OR REPLACE INTO switchbot_measurements (device_id, measured_at, temperature_celsius, humidity_percent, co2_ppm, light_level)
                    VALUES (?, ?, ?, ?, ?, ?)
                    "#,
                )
                .bind(mac.to_string())
                .bind(measured_at.to_rfc3339())
                .bind(m.temperature_celsius)
                .bind(m.humidity_percent as i32)
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
                .bind(m.voltage_v as i32)
                .bind(m.current_ma as i32)
                .bind(m.power_w as i32)
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
            temperature_celsius REAL NOT NULL,
            humidity_percent INTEGER NOT NULL,
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

/// Task to write data from SQLite to external database
#[instrument(skip_all)]
async fn external_db_writer() {
    loop {
        // TODO: write the data from SQLite databaes to external database

        info!("write to external DB");

        // Wait until next hh:mm:12
        tokio::time::sleep(Duration::from_secs(
            (59 - Utc::now().second() as u64 + 12) % 60 + 1,
        ))
        .await;
    }
}
