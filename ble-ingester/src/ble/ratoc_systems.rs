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
/// - Byte 0: Status (non-zero = relay on, zero = relay off)
/// - Byte 1-2: Voltage in dV (0.1V) (little endian, 2 bytes)
/// - Byte 3-4: Current in mA (little endian, 2 bytes)
/// - Byte 5-7: Power in mW (little endian, 3 bytes)
fn parse_manufacturer_data(manufacturer_data: &[u8]) -> Option<RatocSystemsMeasurement> {
    if manufacturer_data.len() < 8 {
        return None;
    }

    let relay = manufacturer_data[0] != 0;

    // Voltage in dV (0.1V) (little endian, 2 bytes)
    let voltage_dv = u16::from_le_bytes([manufacturer_data[1], manufacturer_data[2]]);

    // Current in mA (little endian, 2 bytes)
    let current_ma = u16::from_le_bytes([manufacturer_data[3], manufacturer_data[4]]);

    // Power in mW (little endian, 3 bytes)
    let power_mw = u32::from_le_bytes([
        manufacturer_data[5],
        manufacturer_data[6],
        manufacturer_data[7],
        0x00,
    ]);

    Some(RatocSystemsMeasurement {
        relay,
        voltage_dv,
        current_ma,
        power_mw,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_manufacturer_data() {
        // Example: relay on, 1028 dV (102.8 V), 4388 mA, 451494 mW
        let data = [
            0x01, // Status (relay on)
            0x04, 0x04, // Voltage: 1028 dV (102.8V)
            0x24, 0x11, // Current: 4388 mA
            0xa6, 0xe3, 0x06, // Power: 451494 mW
        ];
        let result = parse_manufacturer_data(&data).unwrap();

        assert!(result.relay);
        assert_eq!(result.voltage_dv, 1028);
        assert_eq!(result.current_ma, 4388);
        assert_eq!(result.power_mw, 451494);
    }
}
