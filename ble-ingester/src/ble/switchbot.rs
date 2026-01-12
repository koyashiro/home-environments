use std::collections::HashMap;

use uuid::{Uuid, uuid};

use crate::SwitchBotMeasurement;

// SwitchBot Company ID for manufacturer data
const SWITCHBOT_MANUFACTURER_DATA_COMPANY_ID: u16 = 0x0969;

// SwitchBot Service Data UUID
const SWITCHBOT_SERVICE_DATA_UUID: Uuid = uuid!("0000fd3d-0000-1000-8000-00805f9b34fb");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum SwitchBotDeviceType {
    Hub,     // TODO: unknown device type byte
    HubMini, // TODO: unknown device type byte
    Hub2,
    Hub3, // TODO: unknown device type byte
    Meter,
    MeterPlus,
    WoIOSensor,
    MeterPro, // TODO: unknown device type byte
    MeterProCO2,
}

/// Parse SwitchBot BLE advertisement data
pub fn parse_switchbot(
    manufacturer_data: &HashMap<u16, Vec<u8>>,
    service_data: &HashMap<Uuid, Vec<u8>>,
) -> Option<SwitchBotMeasurement> {
    let service_data = service_data.get(&SWITCHBOT_SERVICE_DATA_UUID)?;
    let device_type = detect_device_type(service_data)?;
    let manufacturer_data = manufacturer_data.get(&SWITCHBOT_MANUFACTURER_DATA_COMPANY_ID)?;

    match device_type {
        SwitchBotDeviceType::Hub => None,     // TODO: implement
        SwitchBotDeviceType::HubMini => None, // TODO: implement
        SwitchBotDeviceType::Hub2 => parse_hub2(manufacturer_data),
        SwitchBotDeviceType::Hub3 => None,  // TODO: implement
        SwitchBotDeviceType::Meter => None, // TODO: implement
        SwitchBotDeviceType::MeterPlus => parse_meter_plus(manufacturer_data),
        SwitchBotDeviceType::WoIOSensor => parse_wo_io_sensor(manufacturer_data),
        SwitchBotDeviceType::MeterPro => None, // TODO: implement
        SwitchBotDeviceType::MeterProCO2 => parse_meter_pro_co2(manufacturer_data),
    }
}

fn detect_device_type(service_data: &[u8]) -> Option<SwitchBotDeviceType> {
    let device_type_byte = *service_data.first()?;
    match device_type_byte {
        0x76 => Some(SwitchBotDeviceType::Hub2),
        0x54 => Some(SwitchBotDeviceType::Meter),
        0x69 => Some(SwitchBotDeviceType::MeterPlus),
        0x77 => Some(SwitchBotDeviceType::WoIOSensor),
        0x35 => Some(SwitchBotDeviceType::MeterProCO2),
        _ => None,
    }
}

fn parse_hub2(manufacturer_data: &[u8]) -> Option<SwitchBotMeasurement> {
    if manufacturer_data.len() < 17 {
        return None;
    }

    let temperature_celsius = decode_temperature(manufacturer_data[13], manufacturer_data[14])?;
    let humidity_percent = decode_humidity(manufacturer_data[15])?;
    let light_level = Some(decode_light_level(manufacturer_data[12])?);

    Some(SwitchBotMeasurement {
        temperature_celsius,
        humidity_percent,
        co2_ppm: None,
        light_level,
    })
}

fn parse_meter_plus(manufacturer_data: &[u8]) -> Option<SwitchBotMeasurement> {
    if manufacturer_data.len() < 11 {
        return None;
    }

    let temperature_celsius = decode_temperature(manufacturer_data[8], manufacturer_data[9])?;
    let humidity_percent = decode_humidity(manufacturer_data[10])?;

    Some(SwitchBotMeasurement {
        temperature_celsius,
        humidity_percent,
        co2_ppm: None,
        light_level: None,
    })
}

fn parse_wo_io_sensor(manufacturer_data: &[u8]) -> Option<SwitchBotMeasurement> {
    if manufacturer_data.len() < 12 {
        return None;
    }

    let temperature_celsius = decode_temperature(manufacturer_data[8], manufacturer_data[9])?;
    let humidity_percent = decode_humidity(manufacturer_data[10])?;

    Some(SwitchBotMeasurement {
        temperature_celsius,
        humidity_percent,
        co2_ppm: None,
        light_level: None,
    })
}

fn parse_meter_pro_co2(manufacturer_data: &[u8]) -> Option<SwitchBotMeasurement> {
    if manufacturer_data.len() < 16 {
        return None;
    }

    let temperature_celsius = decode_temperature(manufacturer_data[8], manufacturer_data[9])?;
    let humidity_percent = decode_humidity(manufacturer_data[10])?;
    let co2_ppm = Some(u16::from_be_bytes([
        manufacturer_data[13],
        manufacturer_data[14],
    ]));

    Some(SwitchBotMeasurement {
        temperature_celsius,
        humidity_percent,
        co2_ppm,
        light_level: None,
    })
}

/// Decode temperature from 2 bytes
/// - v0 & 0x0f: fractional part
/// - v1 & 0x7f: integral part
/// - v1 & 0x80: sign (1 = positive, 0 = negative)
fn decode_temperature(v0: u8, v1: u8) -> Option<f32> {
    let fractional_part = (v0 & 0x0f) as i16;
    let integral_part = (v1 & 0x7f) as i16;
    let sign = if (v1 & 0x80) != 0 { 1i16 } else { -1i16 };

    Some((sign * (integral_part * 10 + fractional_part)) as f32 / 10.0)
}

/// Decode humidity (lower 7 bits, 0-100)
fn decode_humidity(v: u8) -> Option<u8> {
    let humidity = v & 0x7f;
    if humidity > 100 {
        return None;
    }
    Some(humidity)
}

/// Decode light level (lower 7 bits, 0-20)
fn decode_light_level(v: u8) -> Option<u8> {
    let light_level = v & 0x7f;
    if light_level > 20 {
        return None;
    }
    Some(light_level)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_temperature_positive() {
        // 25.5°C: integral=25 (0x19), fractional=5, sign=positive (0x80)
        // v0 = 0x05, v1 = 0x99 (0x80 | 0x19)
        let temp = decode_temperature(0x05, 0x99).unwrap();
        assert!((temp - 25.5).abs() < 0.01);
    }

    #[test]
    fn test_decode_temperature_negative() {
        // -5.2°C: integral=5, fractional=2, sign=negative (no 0x80)
        // v0 = 0x02, v1 = 0x05
        let temp = decode_temperature(0x02, 0x05).unwrap();
        assert!((temp - (-5.2)).abs() < 0.01);
    }

    #[test]
    fn test_decode_humidity() {
        assert_eq!(decode_humidity(0x3c), Some(60));
        assert_eq!(decode_humidity(0xbc), Some(60)); // with high bit set
        assert_eq!(decode_humidity(0x65), None); // 101 > 100
    }

    #[test]
    fn test_decode_light_level() {
        assert_eq!(decode_light_level(0x0a), Some(10));
        assert_eq!(decode_light_level(0x14), Some(20));
        assert_eq!(decode_light_level(0x15), None); // 21 > 20
    }

    #[test]
    fn test_detect_device_type() {
        assert_eq!(detect_device_type(&[0x76]), Some(SwitchBotDeviceType::Hub2));
        assert_eq!(
            detect_device_type(&[0x69]),
            Some(SwitchBotDeviceType::MeterPlus)
        );
        assert_eq!(
            detect_device_type(&[0x35]),
            Some(SwitchBotDeviceType::MeterProCO2)
        );
        assert_eq!(detect_device_type(&[0xff]), None);
        assert_eq!(detect_device_type(&[]), None);
    }
}
