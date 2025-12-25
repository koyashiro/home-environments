use chrono::DateTime;
use chrono_tz::Tz;
use macaddr::MacAddr6;

#[derive(Debug, Clone)]
pub struct Measurement {
    pub device_id: MacAddr6,

    pub measured_at: DateTime<Tz>,

    pub temperature_celsius: f32,

    pub humidity_percent: u8,

    pub co2_ppm: Option<u16>,

    pub light_level: Option<u8>,
}
