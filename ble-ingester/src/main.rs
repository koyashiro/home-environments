mod handler;
mod mac_address;

use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
};

use chrono::{DateTime, Utc};
use handler::{ble_receiver, external_db_writer, in_memory_db_writer, sqlite_writer};
use mac_address::MacAddress;
use tokio::sync::{Mutex, mpsc};
use tracing_subscriber::{EnvFilter, fmt::time::ChronoLocal};

#[derive(Debug, Default)]
pub struct InMemoryDb {
    pub switchbot_db: HashMap<MacAddress, BTreeMap<DateTime<Utc>, (u8, SwitchBotMeasurement)>>,
    pub ratoc_systems_db:
        HashMap<MacAddress, BTreeMap<DateTime<Utc>, (u8, RatocSystemsMeasurement)>>,
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
pub struct SwitchBotMeasurement {
    pub temperature_celsius: f32,
    pub humidity_percent: u8,
    pub co2_ppm: Option<u16>,
    pub light_level: Option<u8>,
}

#[derive(Debug)]
pub struct RatocSystemsMeasurement {
    pub relay: bool,
    pub voltage_v: u16,
    pub current_ma: u16,
    pub power_w: u32,
}

#[derive(Debug)]
pub struct BleData {
    pub mac: MacAddress,
    pub measured_at: DateTime<Utc>,
    pub measurement: BleMeasurement,
}

#[derive(Debug)]
pub enum BleMeasurement {
    SwitchBot(SwitchBotMeasurement),
    RatocSystems(RatocSystemsMeasurement),
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
