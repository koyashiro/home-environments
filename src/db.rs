use anyhow::{Context as _, Result, anyhow};
use chrono::DateTime;
use chrono_tz::Tz;
use macaddr::MacAddr6;
use sqlx::{PgPool, postgres::PgPoolOptions};

use crate::switchbot::{Device, DeviceType, Measurement};

pub async fn new_pool(database_url: &str) -> Result<PgPool> {
    Ok(PgPoolOptions::new().connect(database_url).await?)
}

struct DeviceRow {
    id: Vec<u8>,
    r#type: String,
    name: String,
    sort_order: i64,
}

impl TryFrom<DeviceRow> for Device {
    type Error = anyhow::Error;

    fn try_from(row: DeviceRow) -> Result<Self> {
        let id_bytes: [u8; 6] = row
            .id
            .try_into()
            .map_err(|v: Vec<u8>| anyhow!("invalid MAC address length: {}", v.len()))?;
        Ok(Device {
            id: MacAddr6::from(id_bytes),
            r#type: row.r#type.parse::<DeviceType>()?,
            name: row.name,
            sort_order: row.sort_order as u8,
        })
    }
}

pub async fn get_switchbot_devices(pool: &PgPool) -> Result<Vec<Device>> {
    let rows = sqlx::query_as!(
        DeviceRow,
        r#"
        SELECT id, type::TEXT as "type!", name, sort_order FROM switchbot_devices ORDER BY sort_order
        "#,
    )
    .fetch_all(pool)
    .await
    .context("failed to select switchbot_devices")?;

    rows.into_iter()
        .map(Device::try_from)
        .collect::<Result<Vec<_>>>()
}

pub async fn bulk_insert_switchbot_measurements(
    pool: &PgPool,
    measurments: &[Measurement],
) -> Result<()> {
    if measurments.is_empty() {
        return Ok(());
    }

    let device_ids: Vec<&[u8]> = measurments.iter().map(|m| m.device_id.as_bytes()).collect();
    let measured_ats: Vec<DateTime<Tz>> = measurments.iter().map(|m| m.measured_at).collect();
    let temperature_celsiuses: Vec<f32> =
        measurments.iter().map(|m| m.temperature_celsius).collect();
    let humidity_percents: Vec<i16> = measurments
        .iter()
        .map(|m| m.humidity_percent as _)
        .collect();
    let co2_ppms: Vec<Option<i16>> = measurments
        .iter()
        .map(|m| m.co2_ppm.map(|v| v as _))
        .collect();
    let light_levels: Vec<Option<i16>> = measurments
        .iter()
        .map(|m| m.light_level.map(|v| v as _))
        .collect();

    let mut tx = pool.begin().await.context("failed to begin transaction")?;

    sqlx::query!(
        r#"
        INSERT INTO switchbot_measurements (device_id, measured_at, temperature_celsius, humidity_percent, co2_ppm, light_level)
        SELECT * FROM UNNEST($1::BYTEA[], $2::TIMESTAMPTZ[], $3::FLOAT4[], $4::INT2[], $5::INT2[], $6::INT2[])
        ON CONFLICT (device_id, measured_at) DO NOTHING
        "#,
        &device_ids as _,
        &measured_ats,
        &temperature_celsiuses,
        &humidity_percents,
        &co2_ppms as  _,
        &light_levels as  _,
    )
    .execute(&mut *tx)
    .await
    .context("failed to bulk insert to switchbot_measurements")?;

    tx.commit().await.context("failed to commit transaction")?;

    Ok(())
}
