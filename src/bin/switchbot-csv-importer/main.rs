mod args;
mod csv;

use std::fs::File;

use anyhow::Context as _;
use args::Args;
use clap::Parser as _;
use home_environments::db::bulk_insert_measurements;
use sqlx::postgres::PgPoolOptions;

use crate::csv::CsvMeasurementIter;

const BULK_INSERT_SIZE: usize = 1000;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let file =
        File::open(&args.file).with_context(|| format!("failed to open file: {:?}", args.file))?;
    let iter = CsvMeasurementIter::new(file, args.device_id, args.timezone)
        .context("failed to create CSV measurement iterator")?;

    let pool = PgPoolOptions::new()
        .connect(&args.database_url)
        .await
        .context("failed to connect to database")?;

    let mut buffer = Vec::with_capacity(BULK_INSERT_SIZE);
    let mut total = 0;

    for result in iter {
        let record = result.context("failed to parse CSV record")?;
        buffer.push(record);

        if buffer.len() >= BULK_INSERT_SIZE {
            bulk_insert_measurements(&pool, &buffer)
                .await
                .context("failed to bulk insert measurements")?;
            total += buffer.len();
            buffer.clear();
        }
    }

    if !buffer.is_empty() {
        bulk_insert_measurements(&pool, &buffer)
            .await
            .context("failed to bulk insert remaining measurements")?;
        total += buffer.len();
    }

    println!("Inserted {} records from {:?}", total, args.file);

    Ok(())
}
