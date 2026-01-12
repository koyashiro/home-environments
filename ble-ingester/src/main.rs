mod ble;
mod config;
mod handler;
mod mac_address;

use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
};

use chrono::{DateTime, Utc};
use config::{Config, DeviceType};
use handler::{ble_receiver, external_db_writer, in_memory_db_writer, sqlite_writer};
use mac_address::MacAddress;
use tokio::sync::{Mutex, mpsc};
use tracing::{error, info};
use tracing_subscriber::{EnvFilter, fmt::time::ChronoLocal};

#[derive(Debug, Default)]
pub struct InMemoryDb {
    pub switchbot_db: HashMap<MacAddress, BTreeMap<DateTime<Utc>, (u8, SwitchBotMeasurement)>>,
    pub ratoc_systems_db:
        HashMap<MacAddress, BTreeMap<DateTime<Utc>, (u8, RatocSystemsMeasurement)>>,
}

impl InMemoryDb {
    pub fn from_config(config: &Config) -> InMemoryDb {
        let mut switchbot_db = HashMap::new();
        let mut ratoc_systems_db = HashMap::new();

        for device in &config.devices {
            match device.device_type {
                DeviceType::SwitchBot => {
                    switchbot_db.insert(device.mac_address, BTreeMap::new());
                }
                DeviceType::RatocSystems => {
                    ratoc_systems_db.insert(device.mac_address, BTreeMap::new());
                }
            }
        }

        InMemoryDb {
            switchbot_db,
            ratoc_systems_db,
        }
    }
}

#[derive(Debug)]
pub struct SwitchBotMeasurement {
    pub temperature_dc: i16,
    pub humidity_p: u8,
    pub co2_ppm: Option<u16>,
    pub light_level: Option<u8>,
}

#[derive(Debug)]
pub struct RatocSystemsMeasurement {
    pub relay: bool,
    pub voltage_dv: u16,
    pub current_ma: u16,
    pub power_mw: u32,
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

    let config = match Config::load() {
        Ok(config) => {
            info!("loaded {} devices from config", config.devices.len());
            config
        }
        Err(e) => {
            error!("failed to load config: {e}");
            return;
        }
    };

    let (tx, rx) = mpsc::channel::<BleData>(128);
    let in_memory_db: Arc<Mutex<InMemoryDb>> =
        Arc::new(Mutex::new(InMemoryDb::from_config(&config)));

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
