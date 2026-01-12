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

    let temperature_dc = decode_temperature_dc([manufacturer_data[13], manufacturer_data[14]])?;
    let humidity_p = decode_humidity_p(manufacturer_data[15])?;
    let light_level = Some(decode_light_level(manufacturer_data[12])?);

    Some(SwitchBotMeasurement {
        temperature_dc,
        humidity_p,
        co2_ppm: None,
        light_level,
    })
}

fn parse_meter_plus(manufacturer_data: &[u8]) -> Option<SwitchBotMeasurement> {
    if manufacturer_data.len() < 11 {
        return None;
    }

    let temperature_dc = decode_temperature_dc([manufacturer_data[8], manufacturer_data[9]])?;
    let humidity_p = decode_humidity_p(manufacturer_data[10])?;

    Some(SwitchBotMeasurement {
        temperature_dc,
        humidity_p,
        co2_ppm: None,
        light_level: None,
    })
}

fn parse_wo_io_sensor(manufacturer_data: &[u8]) -> Option<SwitchBotMeasurement> {
    if manufacturer_data.len() < 12 {
        return None;
    }

    let temperature_dc = decode_temperature_dc([manufacturer_data[8], manufacturer_data[9]])?;
    let humidity_p = decode_humidity_p(manufacturer_data[10])?;

    Some(SwitchBotMeasurement {
        temperature_dc,
        humidity_p,
        co2_ppm: None,
        light_level: None,
    })
}

fn parse_meter_pro_co2(manufacturer_data: &[u8]) -> Option<SwitchBotMeasurement> {
    if manufacturer_data.len() < 16 {
        return None;
    }

    let temperature_dc = decode_temperature_dc([manufacturer_data[8], manufacturer_data[9]])?;
    let humidity_p = decode_humidity_p(manufacturer_data[10])?;
    let co2_ppm = decode_co2_ppm([manufacturer_data[13], manufacturer_data[14]])?;

    Some(SwitchBotMeasurement {
        temperature_dc,
        humidity_p,
        co2_ppm: Some(co2_ppm),
        light_level: None,
    })
}

/// Decode temperature from 2 bytes, returns decidegrees Celsius (dC, 0.1°C units)
/// - bytes[0] & 0x0f: fractional part
/// - bytes[1] & 0x7f: integral part
/// - bytes[1] & 0x80: sign (non-zero = positive, zero = negative)
fn decode_temperature_dc(v: [u8; 2]) -> Option<i16> {
    let fractional_part = (v[0] & 0x0f) as i16;
    let integral_part = (v[1] & 0x7f) as i16;
    let sign = if (v[1] & 0x80) != 0 { 1i16 } else { -1i16 };
    let temperature_dc = sign * (integral_part * 10 + fractional_part);
    if !(-200..=800).contains(&temperature_dc) {
        return None;
    }

    Some(temperature_dc)
}

/// Decode humidity (lower 7 bits, 0-100)
fn decode_humidity_p(v: u8) -> Option<u8> {
    let humidity_p = v & 0x7f;
    if humidity_p > 99 {
        return None;
    }

    Some(humidity_p)
}

/// Decode light level (lower 7 bits, 0-20)
fn decode_light_level(v: u8) -> Option<u8> {
    let light_level = v & 0x7f;
    if light_level > 20 {
        return None;
    }

    Some(light_level)
}

/// Decode CO2 concentration in ppm (2 bytes, big endian)
fn decode_co2_ppm(v: [u8; 2]) -> Option<u16> {
    let co2_ppm = u16::from_be_bytes(v);
    if !(400..=9999).contains(&co2_ppm) {
        return None;
    }

    Some(co2_ppm)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_temperature_dc() {
        // 25.5°C (255 dC): integral=25 (0x19), fractional=5, sign=positive (0x80)
        // v[0] = 0x05, v[1] = 0x99 (0x80 | 0x19)
        assert_eq!(decode_temperature_dc([0x05, 0x99]), Some(255));
        // -5.2°C (-52 dC): integral=5, fractional=2, sign=negative (no 0x80)
        // v[0] = 0x02, v[1] = 0x05
        assert_eq!(decode_temperature_dc([0x02, 0x05]), Some(-52));
        // 80.0°C (800 dC): maximum valid
        assert_eq!(decode_temperature_dc([0x00, 0xd0]), Some(800));
        // -20.0°C (-200 dC): minimum valid
        assert_eq!(decode_temperature_dc([0x00, 0x14]), Some(-200));
        // 80.1°C (801 dC): above maximum
        assert_eq!(decode_temperature_dc([0x01, 0xd0]), None);
        // -20.1°C (-201 dC): below minimum
        assert_eq!(decode_temperature_dc([0x01, 0x14]), None);
    }

    #[test]
    fn test_decode_humidity_p() {
        assert_eq!(decode_humidity_p(0x3c), Some(60));
        assert_eq!(decode_humidity_p(0xbc), Some(60)); // with high bit set
        // 99% (maximum valid)
        assert_eq!(decode_humidity_p(0x63), Some(99));
        // 100% (above maximum)
        assert_eq!(decode_humidity_p(0x64), None);
    }

    #[test]
    fn test_decode_light_level() {
        assert_eq!(decode_light_level(0x0a), Some(10));
        assert_eq!(decode_light_level(0x14), Some(20));
        assert_eq!(decode_light_level(0x15), None); // 21 > 20
    }

    #[test]
    fn test_decode_co2_ppm() {
        // 1000 ppm = 0x03E8
        assert_eq!(decode_co2_ppm([0x03, 0xe8]), Some(1000));
        // 400 ppm = 0x0190 (minimum valid)
        assert_eq!(decode_co2_ppm([0x01, 0x90]), Some(400));
        // 9999 ppm = 0x270F (maximum valid)
        assert_eq!(decode_co2_ppm([0x27, 0x0f]), Some(9999));
        // 399 ppm = 0x018F (below minimum)
        assert_eq!(decode_co2_ppm([0x01, 0x8f]), None);
        // 10000 ppm = 0x2710 (above maximum)
        assert_eq!(decode_co2_ppm([0x27, 0x10]), None);
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
