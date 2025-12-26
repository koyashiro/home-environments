mod args;
mod ble;

use std::{
    collections::{BTreeMap, HashMap},
    process::ExitCode,
    sync::{Arc, Mutex},
};

use anyhow::{Context as _, Result, anyhow};
use args::Args;
use btleplug::{
    api::{Central, Manager as _, Peripheral, ScanFilter},
    platform::Manager,
};
use chrono::{DateTime, TimeDelta};
use chrono_tz::Tz;
use clap::Parser as _;
use home_environments::{
    db::{get_switchbot_devices, new_pool},
    switchbot::{Device, Measurement},
};
use indexmap::IndexMap;
use macaddr::MacAddr6;
use tokio::time::{Duration, sleep};

use crate::ble::switchbot::decode_switchbot_ble_data;

#[tokio::main]
async fn main() -> ExitCode {
    if let Err(e) = run().await {
        eprintln!("{e:#}");
        return ExitCode::from(1);
    }

    ExitCode::from(0)
}

async fn run() -> Result<()> {
    let args = Args::parse();

    let pool = new_pool(&args.database_url)
        .await
        .context("failed to connect to database")?;

    let devices: IndexMap<MacAddr6, Device> = get_switchbot_devices(&pool)
        .await
        .context("failed to get SwitchBot devices")?
        .into_iter()
        .map(|d| (d.id, d))
        .collect();

    let manager = Manager::new()
        .await
        .context("failed to initialize Bluetooth manager")?;

    let adapters = manager
        .adapters()
        .await
        .context("failed to get Bluetooth adapters")?;

    let adapter = adapters
        .into_iter()
        .next()
        .ok_or_else(|| anyhow!("no Bluetooth adapters found"))?;

    adapter
        .start_scan(ScanFilter::default())
        .await
        .context("failed to start BLE scan")?;

    type MeasurementMap = BTreeMap<DateTime<Tz>, Measurement>;
    let db: HashMap<MacAddr6, Arc<Mutex<MeasurementMap>>> = devices
        .iter()
        .map(|(id, _)| (*id, Arc::new(Mutex::new(BTreeMap::new()))))
        .collect();

    loop {
        sleep(Duration::from_secs(2)).await;

        let peripherals = adapter
            .peripherals()
            .await
            .context("failed to get BLE peripherals")?;

        println!();
        for peripheral in peripherals.iter() {
            let peripheral_id = peripheral.id();

            let mac_address: MacAddr6 = peripheral.address().into_inner().into();

            let maybe_properties = match peripheral.properties().await {
                Ok(p) => p,
                Err(err) => {
                    eprintln!(
                        "failed to get BLE peripheral properties: {peripheral_id} ({mac_address}): {err:#}"
                    );
                    continue;
                }
            };

            let Some(properties) = maybe_properties else {
                eprintln!(
                    "BLE peripheral properties not available: {peripheral_id} ({mac_address})"
                );
                continue;
            };

            let measurement = match decode_switchbot_ble_data(
                mac_address,
                &properties.manufacturer_data,
                &properties.service_data,
                args.timezone,
            ) {
                Ok(m) => m,
                Err(err) => {
                    eprintln!(
                        "failed to decode SwitchBot BLE data: {peripheral_id} ({mac_address}): {properties:?} {err:#}"
                    );
                    continue;
                }
            };

            let Some(measurements) = db.get(&mac_address) else {
                eprintln!("unknown device: {mac_address}");
                continue;
            };

            let Ok(mut l) = measurements.lock() else {
                eprintln!("failed to acquire lock for device: {mac_address}");
                continue;
            };

            if let Some((&last_measured_at, _)) = l.last_key_value()
                && measurement.measured_at - last_measured_at < TimeDelta::minutes(1)
            {
                continue;
            }

            l.insert(measurement.measured_at, measurement);

            dbg!(&db);
        }
        println!();
    }
}
