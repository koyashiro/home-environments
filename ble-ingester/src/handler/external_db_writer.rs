use std::time::Duration;

use chrono::{Timelike, Utc};
use tracing::{info, instrument};

/// Task to write data from SQLite to external database
#[instrument(skip_all)]
pub async fn external_db_writer() {
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
