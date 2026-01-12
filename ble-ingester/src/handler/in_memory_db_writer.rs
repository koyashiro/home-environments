use std::sync::Arc;

use chrono::{DurationRound, TimeDelta};
use tokio::sync::{Mutex, mpsc};
use tracing::{debug, instrument};

use crate::{BleData, BleMeasurement, InMemoryDb};

/// Task to store it in in-memory database
#[instrument(skip_all)]
pub async fn in_memory_db_writer(
    mut rx: mpsc::Receiver<BleData>,
    in_memory_db: Arc<Mutex<InMemoryDb>>,
) {
    while let Some(data) = rx.recv().await {
        let BleData {
            mac,
            measured_at,
            measurement,
        } = data;

        let Ok(rounded_measured_at) = measured_at.duration_round(TimeDelta::minutes(1)) else {
            continue;
        };

        let diff = (measured_at - rounded_measured_at)
            .num_seconds()
            .unsigned_abs() as u8;

        let mut in_memory_db = in_memory_db.lock().await;

        match measurement {
            BleMeasurement::SwitchBot(measurement) => {
                let Some(m) = in_memory_db.switchbot_db.get_mut(&mac) else {
                    continue;
                };

                if let Some((existing_diff, _)) = m.get(&rounded_measured_at)
                    && diff >= *existing_diff
                {
                    continue;
                }

                m.insert(rounded_measured_at, (diff, measurement));
            }
            BleMeasurement::RatocSystems(measurement) => {
                let m = in_memory_db.ratoc_systems_db.entry(mac).or_default();

                if let Some((existing_diff, _)) = m.get(&rounded_measured_at)
                    && diff >= *existing_diff
                {
                    continue;
                }

                m.insert(rounded_measured_at, (diff, measurement));
            }
        }

        debug!("write to in-memory db");
    }
}
