use chrono_tz::Tz;
use clap::Parser;

#[derive(Debug, Parser)]
pub struct Args {
    #[arg(long, env = "TZ")]
    pub timezone: Tz,

    #[arg(long, env = "DATABASE_URL")]
    pub database_url: String,
}
