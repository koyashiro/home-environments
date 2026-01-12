use std::collections::HashMap;

use crate::RatocSystemsMeasurement;

// RATOC Systems company ID
const RATOC_SYSTEMS_COMPANY_ID: u16 = 0x0b60;

/// Parse RATOC Systems RS-BTWATTCH2 advertisement data
pub fn parse_ratoc_systems(
    manufacturer_data: &HashMap<u16, Vec<u8>>,
) -> Option<RatocSystemsMeasurement> {
    let data = manufacturer_data.get(&RATOC_SYSTEMS_COMPANY_ID)?;
    parse_manufacturer_data(data)
}

/// RS-BTWATTCH2 manufacturer data format:
/// - Byte 0: Status (non-zero = relay on)
/// - Byte 1-2: Voltage in 0.1V units (little endian)
/// - Byte 3-4: Current in mA (little endian)
/// - Byte 5-7: Power in mW (little endian, 3 bytes)
fn parse_manufacturer_data(manufacturer_data: &[u8]) -> Option<RatocSystemsMeasurement> {
    if manufacturer_data.len() < 8 {
        return None;
    }

    let relay = manufacturer_data[0] != 0;

    // Voltage in 0.1V units, convert to V
    let voltage_raw = u16::from_le_bytes([manufacturer_data[1], manufacturer_data[2]]);
    let voltage_v = voltage_raw / 10;

    // Current in mA
    let current_ma = u16::from_le_bytes([manufacturer_data[3], manufacturer_data[4]]);

    // Power in mW (3 bytes), convert to W
    let power_raw = u32::from_le_bytes([
        0x00,
        manufacturer_data[5],
        manufacturer_data[6],
        manufacturer_data[7],
    ]);
    let power_w = power_raw / 1000;

    Some(RatocSystemsMeasurement {
        relay,
        voltage_v,
        current_ma,
        power_w,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_manufacturer_data() {
        // Example: relay on, 100.1V, 1500mA, 150W
        let data = [
            0x01, // Status (relay on)
            0xe9, 0x03, // Voltage: 1001 (100.1V)
            0xdc, 0x05, // Current: 1500mA
            0x4c, 0x02, 0x00, // Power: 150000mW (3 bytes, little endian with leading 0x00)
        ];
        let result = parse_manufacturer_data(&data).unwrap();

        assert!(result.relay);
        assert_eq!(result.voltage_v, 100);
        assert_eq!(result.current_ma, 1500);
        assert_eq!(result.power_w, 150);
    }

    #[test]
    fn test_parse_manufacturer_data_relay_off() {
        let data = [
            0x00, // Status (relay off)
            0xe8, 0x03, // Voltage: 1000 (100.0V)
            0x00, 0x00, // Current: 0mA
            0x00, 0x00, 0x00, // Power: 0mW
        ];
        let result = parse_manufacturer_data(&data).unwrap();

        assert!(!result.relay);
        assert_eq!(result.voltage_v, 100);
        assert_eq!(result.current_ma, 0);
        assert_eq!(result.power_w, 0);
    }

    #[test]
    fn test_parse_manufacturer_data_too_short() {
        let data = [0x01, 0xe8, 0x03, 0x00, 0x00];
        assert!(parse_manufacturer_data(&data).is_none());
    }
}
