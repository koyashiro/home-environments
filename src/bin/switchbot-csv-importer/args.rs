use std::path::PathBuf;

use chrono_tz::Tz;
use clap::Parser;
use macaddr::MacAddr6;

#[derive(Debug, Parser)]
pub struct Args {
    #[arg(long)]
    pub device_id: MacAddr6,

    #[arg(long)]
    pub file: PathBuf,

    #[arg(long)]
    pub timezone: Tz,

    #[arg(long, env = "DATABASE_URL")]
    pub database_url: String,
}
