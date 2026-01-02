use std::collections::HashMap;

use anyhow::{Context as _, Result, anyhow, bail};

const RATOCSYSTEMS_MANUFACTURER_DATA_COMPANY_ID: u16 = 0x0b60;

#[derive(Debug)]
pub struct RatocsystemsMeasurement {
    pub voltage_v: f32,
    pub current_ma: u16,
    pub power_w: f32,
}

pub fn decode_rsbtwattch2_ble_data(
    manufacturer_data: &HashMap<u16, Vec<u8>>,
) -> Result<RatocsystemsMeasurement> {
    let ratocsystems_manufacturer_data =
        get_ratocsystems_manufacturer_data(manufacturer_data).context("failed to get RATOC Systems manufacturer data")?;

    decode_ratocsystems_manufacturer_data(ratocsystems_manufacturer_data)
        .context("failed to decode RATOC Systems manufacturer data")
}

fn get_ratocsystems_manufacturer_data(manufacturer_data: &HashMap<u16, Vec<u8>>) -> Result<&[u8]> {
    Ok(manufacturer_data
        .get(&RATOCSYSTEMS_MANUFACTURER_DATA_COMPANY_ID)
        .ok_or_else(|| {
            anyhow!(
                "RATOC Systems manufacturer data not found: {RATOCSYSTEMS_MANUFACTURER_DATA_COMPANY_ID}"
            )
        })?)
}

fn decode_ratocsystems_manufacturer_data(
    manufacturer_data: &[u8],
) -> Result<RatocsystemsMeasurement> {
    if manufacturer_data.len() < 8 {
        bail!(
            "RATOC Systems manufacturer data too short: expected at least 8 bytes, got {}",
            manufacturer_data.len()
        )
    }

    let _relay = manufacturer_data[0] != 0;
    let voltage_v =
        (u16::from_le_bytes([manufacturer_data[1], manufacturer_data[2]]) as f32) / 10f32;
    let current_ma = u16::from_le_bytes([manufacturer_data[3], manufacturer_data[4]]);
    let power_w = (u32::from_le_bytes([
        0x00,
        manufacturer_data[5],
        manufacturer_data[6],
        manufacturer_data[7],
    ]) as f32)
        / 1000f32;

    Ok(RatocsystemsMeasurement {
        voltage_v,
        current_ma,
        power_w,
    })
}
