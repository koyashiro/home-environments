use anyhow::{Context as _, Result};
use chrono::DateTime;
use chrono_tz::Tz;
use sqlx::PgPool;

use crate::switchbot::Measurement;

pub async fn bulk_insert_measurements(pool: &PgPool, measurments: &[Measurement]) -> Result<()> {
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
    .context("failed to execute bulk insert query")?;

    tx.commit().await.context("failed to commit transaction")?;

    Ok(())
}
