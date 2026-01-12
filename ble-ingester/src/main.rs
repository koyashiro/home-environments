use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
    time::Duration,
};

use chrono::{DateTime, DurationRound, TimeDelta, Timelike, Utc};
use tokio::sync::{Mutex, mpsc};
use tracing::{debug, error, info, instrument};
use tracing_subscriber::{EnvFilter, fmt::time::ChronoLocal};

const RECEIVE_RANGE: u8 = 10;

pub type MacAddress = [u8; 6];

#[derive(Debug, Default)]
pub struct InMemoryDb {
    pub switchbot_db: HashMap<MacAddress, BTreeMap<DateTime<Utc>, (u8, SwitchBotMeasurements)>>,
    pub ratoc_systems_db:
        HashMap<MacAddress, BTreeMap<DateTime<Utc>, (u8, RatocSystemsMeasurements)>>,
}

impl InMemoryDb {
    pub fn new() -> InMemoryDb {
        InMemoryDb {
            switchbot_db: [([0x02, 0x00, 0x00, 0x00, 0x00, 0x01], BTreeMap::new())].into(),
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
pub struct RatocSystemsMeasurements {}

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
        let mac = [0x02, 0x00, 0x00, 0x00, 0x00, 0x01];
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
    loop {
        {
            let in_memory_db = in_memory_db.lock().await;

            // TODO: write the data from in in-memory database to SQLite database

            info!("write to SQLite db");
            info!("{in_memory_db:?}");
        }

        // Wait until next hh:mm:11
        tokio::time::sleep(Duration::from_secs(
            (59 - Utc::now().second() as u64 + 11) % 60 + 1,
        ))
        .await;
    }
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
