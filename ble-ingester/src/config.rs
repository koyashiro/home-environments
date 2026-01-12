use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use serde::Deserialize;

use crate::mac_address::MacAddress;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum DeviceType {
    #[serde(rename = "switchbot")]
    SwitchBot,
    #[serde(rename = "ratoc_systems")]
    RatocSystems,
}

#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub devices: Vec<DeviceConfig>,
}

#[derive(Debug, Deserialize)]
pub struct DeviceConfig {
    pub mac_address: MacAddress,
    pub device_type: DeviceType,
}

impl Config {
    pub fn load() -> Result<Self, ConfigError> {
        let config_path = Self::config_path()?;

        if !config_path.exists() {
            return Ok(Config { devices: vec![] });
        }

        let content = fs::read_to_string(&config_path)
            .map_err(|e| ConfigError::Read(config_path.clone(), e))?;

        serde_yaml::from_str(&content).map_err(|e| ConfigError::Parse(config_path, e))
    }

    pub fn devices(&self) -> HashMap<MacAddress, DeviceType> {
        self.devices
            .iter()
            .map(|d| (d.mac_address, d.device_type))
            .collect()
    }

    fn config_path() -> Result<PathBuf, ConfigError> {
        let home = std::env::var("HOME").map_err(|_| ConfigError::NoHome)?;
        Ok(PathBuf::from(home)
            .join(".config")
            .join("ble-ingester")
            .join("config.yaml"))
    }
}

#[derive(Debug)]
pub enum ConfigError {
    NoHome,
    Read(PathBuf, std::io::Error),
    Parse(PathBuf, serde_yaml::Error),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::NoHome => write!(f, "HOME environment variable not set"),
            ConfigError::Read(path, e) => write!(f, "failed to read {}: {}", path.display(), e),
            ConfigError::Parse(path, e) => write!(f, "failed to parse {}: {}", path.display(), e),
        }
    }
}

impl std::error::Error for ConfigError {}
