use std::collections::HashMap;

use btleplug::api::{Central, Manager as _, Peripheral as _, ScanFilter};
use btleplug::platform::Manager;
use chrono::{Timelike as _, Utc};
use tokio::sync::mpsc;
use tokio_stream::StreamExt as _;
use tracing::{debug, error, info, instrument, warn};

use crate::ble::{parse_ratoc_systems, parse_switchbot};
use crate::config::{Config, DeviceType};
use crate::mac_address::MacAddress;
use crate::{BleData, BleMeasurement};

const RECEIVE_RANGE: u8 = 10;

/// Task to receive BLE data
#[instrument(skip_all)]
pub async fn ble_receiver(tx: mpsc::Sender<BleData>) {
    let devices: HashMap<MacAddress, DeviceType> = match Config::load() {
        Ok(config) => {
            let devices = config.devices();
            info!("loaded {} devices from config", devices.len());
            devices
        }
        Err(e) => {
            error!("failed to load config: {e}");
            return;
        }
    };

    let manager = match Manager::new().await {
        Ok(m) => m,
        Err(e) => {
            error!("failed to create BLE manager: {e}");
            return;
        }
    };

    let adapters = match manager.adapters().await {
        Ok(a) => a,
        Err(e) => {
            error!("failed to get BLE adapters: {e}");
            return;
        }
    };

    let adapter = match adapters.into_iter().next() {
        Some(a) => a,
        None => {
            error!("no BLE adapter found");
            return;
        }
    };

    info!("using BLE adapter: {:?}", adapter.adapter_info().await);

    if let Err(e) = adapter.start_scan(ScanFilter::default()).await {
        error!("failed to start BLE scan: {e}");
        return;
    }

    let mut events = match adapter.events().await {
        Ok(e) => e,
        Err(e) => {
            error!("failed to get BLE events: {e}");
            return;
        }
    };

    while let Some(event) = events.next().await {
        let btleplug::api::CentralEvent::DeviceDiscovered(id) = event else {
            continue;
        };

        let peripheral = match adapter.peripheral(&id).await {
            Ok(p) => p,
            Err(e) => {
                warn!("failed to get peripheral: {e}");
                continue;
            }
        };

        let properties = match peripheral.properties().await {
            Ok(Some(p)) => p,
            Ok(None) => continue,
            Err(e) => {
                warn!("failed to get peripheral properties: {e}");
                continue;
            }
        };

        let measured_at = Utc::now();
        let measured_at_seconds = measured_at.second();

        // Skip if not within receive range of minute boundary
        if (RECEIVE_RANGE as u32) < measured_at_seconds
            && measured_at_seconds < 60 - (RECEIVE_RANGE as u32)
        {
            debug!("BLE skipped (outside receive range)");
            continue;
        }

        let mac = MacAddress::new(properties.address.into_inner());

        // Skip if not in devices list
        let Some(device_type) = devices.get(&mac) else {
            continue;
        };

        let measurement = match device_type {
            DeviceType::SwitchBot => {
                let Some(measurement) =
                    parse_switchbot(&properties.manufacturer_data, &properties.service_data)
                else {
                    continue;
                };
                debug!("SwitchBot received: {mac}");
                BleMeasurement::SwitchBot(measurement)
            }
            DeviceType::RatocSystems => {
                let Some(measurement) = parse_ratoc_systems(&properties.manufacturer_data) else {
                    continue;
                };
                debug!("RATOC Systems received: {mac}");
                BleMeasurement::RatocSystems(measurement)
            }
        };

        let data = BleData {
            mac,
            measured_at,
            measurement,
        };

        if let Err(e) = tx.send(data).await {
            error!("failed to send BLE data: {e}");
            break;
        }
    }
}
