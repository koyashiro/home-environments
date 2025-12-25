use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};

use anyhow::{Context as _, Result, bail};
use chrono::{LocalResult, NaiveDateTime};
use chrono_tz::Tz;
use csv::Reader;
use home_environments::switchbot::Measurement;
use macaddr::MacAddr6;

const MEASURED_AT_INDEX: usize = 0;
const TEMPERATURE_CELSIUS_INDEX: usize = 1;
const HUMIDITY_PERCENT_INDEX: usize = 2;
const CO2_PPM_INDEX: usize = 3;
const LIGHT_LEVEL_INDEX: usize = 6;

#[derive(Debug, Clone, Copy)]
enum CsvFormat {
    TemperatureHumidity,
    TemperatureHumidityCo2,
    TemperatureHumidityLightLevel,
}

#[derive(Debug)]
pub struct CsvMeasurementIter {
    reader: Reader<File>,
    format: CsvFormat,
    device_id: MacAddr6,
    timezone: Tz,
}

impl CsvMeasurementIter {
    pub fn new(mut file: File, device_id: MacAddr6, timezone: Tz) -> Result<Self> {
        let mut buf_reader = BufReader::new(&file);
        let mut header = String::new();
        buf_reader
            .read_line(&mut header)
            .context("failed to read CSV header")?;

        let format = detect_format(&header);

        file.seek(SeekFrom::Start(0))
            .context("failed to seek to start of file")?;
        let reader = Reader::from_reader(file);

        Ok(Self {
            reader,
            format,
            device_id,
            timezone,
        })
    }
}

impl Iterator for CsvMeasurementIter {
    type Item = Result<Measurement>;

    fn next(&mut self) -> Option<Self::Item> {
        let row = match self.reader.records().next()? {
            Ok(row) => row,
            Err(e) => return Some(Err(e.into())),
        };

        let record = (|| -> Result<Measurement> {
            let naive = NaiveDateTime::parse_from_str(&row[MEASURED_AT_INDEX], "%Y-%m-%d %H:%M")
                .with_context(|| {
                    format!("failed to parse timestamp: {}", &row[MEASURED_AT_INDEX])
                })?;
            let measured_at = match naive.and_local_timezone(self.timezone) {
                LocalResult::Single(dt) => dt,
                LocalResult::Ambiguous(dt, _) => dt,
                LocalResult::None => bail!("invalid timestamp: {}", &row[MEASURED_AT_INDEX]),
            };

            let temperature_celsius =
                row[TEMPERATURE_CELSIUS_INDEX].parse().with_context(|| {
                    format!(
                        "failed to parse temperature: {}",
                        &row[TEMPERATURE_CELSIUS_INDEX]
                    )
                })?;
            let humidity_percent = row[HUMIDITY_PERCENT_INDEX].parse().with_context(|| {
                format!("failed to parse humidity: {}", &row[HUMIDITY_PERCENT_INDEX])
            })?;
            let co2_ppm = match self.format {
                CsvFormat::TemperatureHumidity => None,
                CsvFormat::TemperatureHumidityCo2 => Some(
                    row[CO2_PPM_INDEX]
                        .parse()
                        .with_context(|| format!("failed to parse CO2: {}", &row[CO2_PPM_INDEX]))?,
                ),
                CsvFormat::TemperatureHumidityLightLevel => None,
            };
            let light_level = match self.format {
                CsvFormat::TemperatureHumidity => None,
                CsvFormat::TemperatureHumidityCo2 => None,
                CsvFormat::TemperatureHumidityLightLevel => {
                    Some(row[LIGHT_LEVEL_INDEX].parse().with_context(|| {
                        format!("failed to parse light level: {}", &row[LIGHT_LEVEL_INDEX])
                    })?)
                }
            };

            Ok(Measurement {
                device_id: self.device_id,
                measured_at,
                temperature_celsius,
                humidity_percent,
                co2_ppm,
                light_level,
            })
        })();

        Some(record)
    }
}

fn detect_format(header: &str) -> CsvFormat {
    if header.contains("Co2") {
        return CsvFormat::TemperatureHumidityCo2;
    }

    if header.contains("Light_Value") {
        return CsvFormat::TemperatureHumidityLightLevel;
    }

    CsvFormat::TemperatureHumidity
}
