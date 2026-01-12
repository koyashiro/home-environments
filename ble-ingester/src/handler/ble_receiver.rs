use std::time::Duration;

use chrono::{Timelike, Utc};
use tokio::sync::mpsc;
use tracing::{debug, error, instrument};

use crate::mac_address::MacAddress;
use crate::{BleData, BleMeasurement, SwitchBotMeasurement};

const RECEIVE_RANGE: u8 = 10;

/// Task to receive BLE data
#[instrument(skip_all)]
pub async fn ble_receiver(tx: mpsc::Sender<BleData>) {
    loop {
        // dummy data
        tokio::time::sleep(Duration::from_secs(2)).await;
        let mac = MacAddress::new([0x02, 0x00, 0x00, 0x00, 0x00, 0x01]);
        let measurement = SwitchBotMeasurement {
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

        let data = BleData {
            mac,
            measured_at,
            measurement: BleMeasurement::SwitchBot(measurement),
        };

        if let Err(e) = tx.send(data).await {
            error!("failed to send BLE data: {e}");
            break;
        }
    }
}
