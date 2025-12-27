mod args;
mod ble;

use std::{
    collections::{BTreeMap, HashMap},
    process::ExitCode,
    sync::Arc,
    time::Duration,
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
    switchbot::{Device, Measurement},
};
use indexmap::IndexMap;
use macaddr::MacAddr6;
use tokio::sync::Mutex;
use tokio_stream::StreamExt;

use home_environments::db::bulk_insert_switchbot_measurements;

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

    type Db = HashMap<MacAddr6, BTreeMap<DateTime<Tz>, (DateTime<Tz>, DecodedMeasurement)>>;
    let db: Arc<Mutex<Db>> = Arc::new(Mutex::new(
        devices.keys().map(|id| (*id, BTreeMap::new())).collect(),
    ));

    let mut events = adapter.events().await?;

    let db_for_ingester = db.clone();
    let ingester_handle = tokio::spawn(async move {
        while let Some(event) = events.next().await {
            let peripheral_id = match &event {
                CentralEvent::DeviceDiscovered(id) | CentralEvent::DeviceUpdated(id) => id,
                _ => continue,
            };

            let peripheral = match adapter.peripheral(peripheral_id).await {
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
                eprintln!(
                    "BLE peripheral properties not available: {peripheral_id} ({mac_address})"
                );
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

            let mut db = db_for_ingester.lock().await;

            let Some(measurements) = db.get_mut(&mac_address) else {
                eprintln!("unknown device: {mac_address}");
                continue;
            };

            if let Some((existing_measured_at, _)) = measurements.get(&rounded_measured_at) {
                let existing_diff = (*existing_measured_at - rounded_measured_at)
                    .num_milliseconds()
                    .abs();

                if diff >= existing_diff {
                    continue;
                }
            }

            measurements.insert(rounded_measured_at, (measured_at, decoded));
        }
    });

    let db_for_printer = db.clone();
    let printer_handle = tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_mins(5));
        loop {
            interval.tick().await;
            let mut db = db_for_printer.lock().await;

            let now = Utc::now().with_timezone(&args.timezone);

            let keys_to_insert: Vec<(MacAddr6, DateTime<Tz>)> = db
                .iter()
                .flat_map(|(&device_id, measurements)| {
                    measurements
                        .iter()
                        .filter(|&(&measured_at, _)| {
                            (measured_at - now).num_milliseconds()
                                > TimeDelta::seconds(30).num_milliseconds()
                        })
                        .map(move |(&measured_at, _)| (device_id, measured_at))
                })
                .collect();

            let measurments: Vec<Measurement> = keys_to_insert
                .iter()
                .filter_map(|(device_id, measured_at)| {
                    db.get(device_id)
                        .and_then(|m| m.get(measured_at))
                        .map(|(_, m)| Measurement {
                            device_id: *device_id,
                            measured_at: *measured_at,
                            temperature_celsius: m.temperature_celsius,
                            humidity_percent: m.humidity_percent,
                            co2_ppm: m.co2_ppm,
                            light_level: m.light_level,
                        })
                })
                .collect();

            if let Err(e) = bulk_insert_switchbot_measurements(&pool, &measurments).await {
                eprintln!("failed to bulk insert measurements: {e:#}");
                continue;
            }

            for (device_id, measured_at) in keys_to_insert {
                if let Some(measurements) = db.get_mut(&device_id) {
                    measurements.remove(&measured_at);
                }
            }
        }
    });

    let _ = tokio::join!(ingester_handle, printer_handle);

    Ok(())
}
