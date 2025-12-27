use std::collections::HashMap;

use anyhow::{Context as _, Result, anyhow, bail};
use home_environments::switchbot::DeviceType;
use uuid::{Uuid, uuid};

#[derive(Debug)]
pub struct DecodedMeasurement {
    pub temperature_celsius: f32,
    pub humidity_percent: u8,
    pub co2_ppm: Option<u16>,
    pub light_level: Option<u8>,
}

// Ref: https://github.com/OpenWonderLabs/SwitchBotAPI-BLE/blob/2bd727ecf7c0898b25ac2df58a4886b5930c9138/README.md?plain=1#L44
const SWITCHBOT_MANUFACTURER_DATA_COMPANY_ID: u16 = 0x0969;

// Ref: https://github.com/OpenWonderLabs/SwitchBotAPI-BLE/blob/2bd727ecf7c0898b25ac2df58a4886b5930c9138/README.md?plain=1#L45
const SWITCHBOT_SERVICE_DATA_UUID: Uuid = uuid!("0000fd3d-0000-1000-8000-00805f9b34fb");

pub fn decode_ble_data(
    manufacturer_data: &HashMap<u16, Vec<u8>>,
    service_data: &HashMap<Uuid, Vec<u8>>,
) -> Result<DecodedMeasurement> {
    let switchbot_service_data = get_switch_bot_service_data(service_data)
        .context("failed to get SwitchBot service data")?;

    let device_type = detect_device_type(switchbot_service_data)
        .context("failed to detect SwitchBot device type")?;

    let switchbot_manufacturer_data = get_switch_bot_manufacturer_data(manufacturer_data)
        .context("failed to get SwitchBot manufacturer data")?;

    decode_manufacturer_data(&device_type, switchbot_manufacturer_data)
        .context("failed to decode SwitchBot manufacturer data")
}

pub fn decode_manufacturer_data(
    device_type: &DeviceType,
    manufacturer_data: &[u8],
) -> Result<DecodedMeasurement> {
    match device_type {
        DeviceType::Hub => decode_hub_manufacturer_data(manufacturer_data),
        DeviceType::HubMini => decode_hub_mini_manufacturer_data(manufacturer_data),
        DeviceType::Hub2 => decode_hub2_manufacturer_data(manufacturer_data),
        DeviceType::Hub3 => decode_hub3_manufacturer_data(manufacturer_data),
        DeviceType::Meter => decode_meter_manufacturer_data(manufacturer_data),
        DeviceType::MeterPlus => decode_meter_plus_manufacturer_data(manufacturer_data),
        DeviceType::WoIOSensor => decode_wo_io_sensor_manufacturer_data(manufacturer_data),
        DeviceType::MeterPro => decode_meter_pro_manufacturer_data(manufacturer_data),
        DeviceType::MeterProCO2 => decode_meter_pro_co2_manufacturer_data(manufacturer_data),
    }
}

pub fn decode_hub_manufacturer_data(_manufacturer_data: &[u8]) -> Result<DecodedMeasurement> {
    bail!("todo")
}

pub fn decode_hub_mini_manufacturer_data(_manufacturer_data: &[u8]) -> Result<DecodedMeasurement> {
    bail!("todo")
}

pub fn decode_hub2_manufacturer_data(manufacturer_data: &[u8]) -> Result<DecodedMeasurement> {
    if manufacturer_data.len() < 17 {
        bail!(
            "Hub2 manufacturer data too short: expected at least 17 bytes, got {}",
            manufacturer_data.len()
        )
    }

    let temperature_celsius = decode_temperature([manufacturer_data[13], manufacturer_data[14]])
        .context("failed to decode temperature")?;
    let humidity_percent =
        decode_humidity(manufacturer_data[15]).context("failed to decode humidity")?;
    let co2_ppm = None;
    let light_level =
        Some(decode_light_level(manufacturer_data[12]).context("failed to decode light level")?);

    Ok(DecodedMeasurement {
        temperature_celsius,
        humidity_percent,
        co2_ppm,
        light_level,
    })
}

pub fn decode_hub3_manufacturer_data(_manufacturer_data: &[u8]) -> Result<DecodedMeasurement> {
    bail!("todo")
}

pub fn decode_meter_manufacturer_data(_manufacturer_data: &[u8]) -> Result<DecodedMeasurement> {
    bail!("todo")
}

pub fn decode_meter_plus_manufacturer_data(manufacturer_data: &[u8]) -> Result<DecodedMeasurement> {
    if manufacturer_data.len() < 11 {
        bail!(
            "Meter Plus manufacturer data too short: expected at least 11 bytes, got {}",
            manufacturer_data.len()
        )
    }

    let temperature_celsius = decode_temperature([manufacturer_data[8], manufacturer_data[9]])
        .context("failed to decode temperature")?;
    let humidity_percent =
        decode_humidity(manufacturer_data[10]).context("failed to decode humidity")?;
    let co2_ppm = None;
    let light_level = None;

    Ok(DecodedMeasurement {
        temperature_celsius,
        humidity_percent,
        co2_ppm,
        light_level,
    })
}

pub fn decode_wo_io_sensor_manufacturer_data(
    manufacturer_data: &[u8],
) -> Result<DecodedMeasurement> {
    if manufacturer_data.len() < 12 {
        bail!(
            "WoIOSensor manufacturer data too short: expected at least 12 bytes, got {}",
            manufacturer_data.len()
        )
    }

    let temperature_celsius = decode_temperature([manufacturer_data[8], manufacturer_data[9]])
        .context("failed to decode temperature")?;
    let humidity_percent =
        decode_humidity(manufacturer_data[10]).context("failed to decode humidity")?;
    let co2_ppm = None;
    let light_level = None;

    Ok(DecodedMeasurement {
        temperature_celsius,
        humidity_percent,
        co2_ppm,
        light_level,
    })
}

pub fn decode_meter_pro_manufacturer_data(_manufacturer_data: &[u8]) -> Result<DecodedMeasurement> {
    bail!("todo")
}

pub fn decode_meter_pro_co2_manufacturer_data(
    manufacturer_data: &[u8],
) -> Result<DecodedMeasurement> {
    if manufacturer_data.len() < 16 {
        bail!(
            "Meter Pro CO2 manufacturer data too short: expected at least 16 bytes, got {}",
            manufacturer_data.len()
        )
    }

    let temperature_celsius = decode_temperature([manufacturer_data[8], manufacturer_data[9]])
        .context("failed to decode temperature")?;
    let humidity_percent =
        decode_humidity(manufacturer_data[10]).context("failed to decode humidity")?;
    let co2_ppm = Some(
        decode_co2([manufacturer_data[13], manufacturer_data[14]])
            .context("failed to decode CO2")?,
    );
    let light_level = None;

    Ok(DecodedMeasurement {
        temperature_celsius,
        humidity_percent,
        co2_ppm,
        light_level,
    })
}

fn get_switch_bot_manufacturer_data(manufacturer_data: &HashMap<u16, Vec<u8>>) -> Result<&[u8]> {
    Ok(manufacturer_data
        .get(&SWITCHBOT_MANUFACTURER_DATA_COMPANY_ID)
        .ok_or_else(|| {
            anyhow!(
                "SwitchBot manufacturer data not found: {SWITCHBOT_MANUFACTURER_DATA_COMPANY_ID}"
            )
        })?)
}

fn get_switch_bot_service_data(service_data: &HashMap<Uuid, Vec<u8>>) -> Result<&[u8]> {
    Ok(service_data
        .get(&SWITCHBOT_SERVICE_DATA_UUID)
        .ok_or_else(|| {
            anyhow!("SwitchBot service data not found: {SWITCHBOT_SERVICE_DATA_UUID}")
        })?)
}

fn detect_device_type(service_data: &[u8]) -> Result<DeviceType> {
    let Some(&device_type_raw) = service_data.first() else {
        bail!("SwitchBot service data is empty");
    };

    let device_type =
        decode_device_type(device_type_raw).context("failed to decode SwitchBot device type")?;

    Ok(device_type)
}

fn decode_device_type(v: u8) -> Result<DeviceType> {
    match v {
        0x76 => Ok(DeviceType::Hub2),
        0x54 => Ok(DeviceType::Meter),
        0x69 => Ok(DeviceType::MeterPlus),
        0x77 => Ok(DeviceType::WoIOSensor),
        0x35 => Ok(DeviceType::MeterProCO2),
        _ => bail!("unknown SwitchBot device type: 0x{v:02x}"),
    }
}

fn decode_temperature(v: [u8; 2]) -> Result<f32> {
    let fractional_part = (v[0] & 0x0f) as i16;
    let integral_part = (v[1] & 0x7f) as i16;
    let positive_negative_flag = v[1] & 0x80;

    let sign = if positive_negative_flag != 0 {
        1i16
    } else {
        -1i16
    };

    Ok((sign * (integral_part * 10 + fractional_part)) as f32 / 10f32)
}

fn decode_humidity(v: u8) -> Result<u8> {
    let humidity = v & 0x7f;
    if humidity > 100 {
        bail!("humidity out of range: expected 0-100, got {humidity}");
    }

    Ok(humidity)
}

fn decode_co2(v: [u8; 2]) -> Result<u16> {
    Ok(u16::from_be_bytes([v[0], v[1]]))
}

fn decode_light_level(v: u8) -> Result<u8> {
    let light_level = v & 0x7f;
    if light_level > 20 {
        bail!("light level out of range: expected 0-20, got {light_level}");
    }

    Ok(light_level)
}
