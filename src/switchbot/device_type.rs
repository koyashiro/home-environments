use std::str::FromStr;

use anyhow::{Error, bail};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceType {
    Hub,
    HubMini,
    Hub2,
    Hub3,
    Meter,
    MeterPlus,
    WoIOSensor,
    MeterPro,
    MeterProCO2,
}

impl DeviceType {
    pub fn as_str(&self) -> &'static str {
        match self {
            DeviceType::Hub => "Hub",
            DeviceType::HubMini => "Hub Mini",
            DeviceType::Hub2 => "Hub 2",
            DeviceType::Hub3 => "Hub 3",
            DeviceType::Meter => "Meter",
            DeviceType::MeterPlus => "MeterPlus",
            DeviceType::WoIOSensor => "WoIOSensor",
            DeviceType::MeterPro => "MeterPro",
            DeviceType::MeterProCO2 => "MeterPro(CO2)",
        }
    }
}

impl FromStr for DeviceType {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Hub" => Ok(DeviceType::Hub),
            "Hub Mini" => Ok(DeviceType::HubMini),
            "Hub 2" => Ok(DeviceType::Hub2),
            "Hub 3" => Ok(DeviceType::Hub3),
            "Meter" => Ok(DeviceType::Meter),
            "MeterPlus" => Ok(DeviceType::MeterPlus),
            "WoIOSensor" => Ok(DeviceType::WoIOSensor),
            "MeterPro" => Ok(DeviceType::MeterPro),
            "MeterPro(CO2)" => Ok(DeviceType::MeterProCO2),
            _ => bail!("unknown device type: {}", s),
        }
    }
}
