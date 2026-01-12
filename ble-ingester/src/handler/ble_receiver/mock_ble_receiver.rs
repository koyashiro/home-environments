use std::collections::HashMap;

use chrono::Utc;
use rand::Rng;
use tokio::sync::mpsc;
use tokio::time::{Duration, interval};
use tracing::{debug, error, info, instrument};

use crate::config::{Config, DeviceType};
use crate::mac_address::MacAddress;
use crate::{BleData, BleMeasurement, RatocSystemsMeasurement, SwitchBotMeasurement};

/// Task to receive BLE data (mock implementation)
#[instrument(skip_all)]
pub async fn ble_receiver(tx: mpsc::Sender<BleData>) {
    let devices: HashMap<MacAddress, DeviceType> = match Config::load() {
        Ok(config) => {
            let devices = config.devices();
            info!("loaded {} devices from config (mock mode)", devices.len());
            devices
        }
        Err(e) => {
            error!("failed to load config: {e}");
            return;
        }
    };

    info!("starting mock BLE receiver");

    let mut interval = interval(Duration::from_secs(2));

    loop {
        interval.tick().await;

        for (mac, device_type) in &devices {
            let data = {
                let measured_at = Utc::now();
                let mut rng = rand::rng();

                let measurement = match device_type {
                    DeviceType::SwitchBot => {
                        debug!("SwitchBot mock received: {mac}");
                        BleMeasurement::SwitchBot(SwitchBotMeasurement {
                            temperature_dc: rng.random_range(200..=300),
                            humidity_p: rng.random_range(40..=60),
                            co2_ppm: if rng.random_bool(0.8) {
                                Some(rng.random_range(600..=1200))
                            } else {
                                None
                            },
                            light_level: if rng.random_bool(0.8) {
                                Some(rng.random_range(0..=20))
                            } else {
                                None
                            },
                        })
                    }
                    DeviceType::RatocSystems => {
                        debug!("RATOC Systems mock received: {mac}");
                        let voltage_dv: u16 = rng.random_range(970..=1030);
                        let current_ma: u16 = rng.random_range(0..=15000);
                        let power_mw = voltage_dv as u32 * current_ma as u32 / 10;
                        BleMeasurement::RatocSystems(RatocSystemsMeasurement {
                            relay: rng.random_bool(0.5),
                            voltage_dv,
                            current_ma,
                            power_mw,
                        })
                    }
                };

                BleData {
                    mac: *mac,
                    measured_at,
                    measurement,
                }
            };

            if let Err(e) = tx.send(data).await {
                error!("failed to send BLE data: {e}");
                return;
            }
        }
    }
}
