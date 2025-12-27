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
    api::{Central, CentralEvent, Manager as _, Peripheral, ScanFilter},
    platform::Manager,
};
use chrono::{DateTime, DurationRound, TimeDelta, Utc};
use chrono_tz::Tz;
use clap::Parser as _;
use home_environments::{
    db::{get_switchbot_devices, new_pool},
    switchbot::Device,
};
use indexmap::IndexMap;
use macaddr::MacAddr6;
use tokio_stream::StreamExt;

use crate::ble::switchbot::{DecodedMeasurement, decode_ble_data, decode_manufacturer_data};

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

    type MeasurementEntry = (DateTime<Tz>, DecodedMeasurement);
    type MeasurementMap = BTreeMap<DateTime<Tz>, MeasurementEntry>;
    let db: HashMap<MacAddr6, Arc<Mutex<MeasurementMap>>> = devices
        .iter()
        .map(|(id, _)| (*id, Arc::new(Mutex::new(BTreeMap::new()))))
        .collect();

    let mut events = adapter.events().await?;

    while let Some(event) = events.next().await {
        let peripheral_id = match &event {
            CentralEvent::DeviceDiscovered(id) | CentralEvent::DeviceUpdated(id) => id.clone(),
            _ => continue,
        };

        let peripheral = match adapter.peripheral(&peripheral_id).await {
            Ok(p) => p,
            Err(err) => {
                eprintln!("failed to get peripheral {peripheral_id}: {err:#}");
                continue;
            }
        };

        let measured_at = Utc::now().with_timezone(&args.timezone);

        let Ok(rounded_measured_at) = measured_at.duration_round(TimeDelta::minutes(1)) else {
            eprintln!("failed to round measured_at to 1 minute: {measured_at}");
            continue;
        };

        let diff = (measured_at - rounded_measured_at).num_milliseconds().abs();
        if diff > TimeDelta::seconds(20).num_milliseconds() {
            continue;
        }

        let mac_address: MacAddr6 = peripheral.address().into_inner().into();
        let Some(device) = devices.get(&mac_address) else {
            continue;
        };

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
            eprintln!("BLE peripheral properties not available: {peripheral_id} ({mac_address})");
            continue;
        };

        let decoded = match decode_ble_data(&properties.manufacturer_data, &properties.service_data)
            .inspect_err(|err| {
                eprintln!("failed to decode BLE service data, falling back to manufacturer data: {peripheral_id} ({mac_address}) {err:#}");
            })
            .or_else(|_| decode_manufacturer_data(&device.r#type, &properties.manufacturer_data))
        {
            Ok(m) => m,
            Err(err) => {
                eprintln!(
                    "failed to decode manufacturer data: {peripheral_id} ({mac_address}): {err:#}"
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

        if let Some((existing_measured_at, _)) = l.get(&rounded_measured_at) {
            let existing_diff = (*existing_measured_at - rounded_measured_at)
                .num_milliseconds()
                .abs();

            if diff >= existing_diff {
                continue;
            }
        }

        l.insert(rounded_measured_at, (measured_at, decoded));

        dbg!(&l);
    }

    Ok(())
}
