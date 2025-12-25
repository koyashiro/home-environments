CREATE TABLE homes (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid (),
  name STRING NOT NULL,
  sort_order INT NOT NULL UNIQUE
);

CREATE TABLE rooms (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid (),
  home_id UUID NOT NULL REFERENCES homes (id),
  name STRING NOT NULL,
  sort_order INT NOT NULL,
  UNIQUE (home_id, sort_order)
);

CREATE TYPE switchbot_device_type AS ENUM (
  'Hub',
  'Hub Plus',
  'Hub Mini',
  'Hub 2',
  'Hub 3',
  'Meter',
  'MeterPlus',
  'WoIOSensor',
  'MeterPro',
  'MeterPro(CO2)'
);

CREATE TABLE switchbot_devices (
  id BYTES PRIMARY KEY,
  type switchbot_device_type NOT NULL,
  name STRING NOT NULL,
  sort_order INT NOT NULL UNIQUE,
  CHECK (length (id) = 6)
);

CREATE TABLE switchbot_device_locations (
  device_id BYTES NOT NULL REFERENCES switchbot_devices (id),
  placed_at TIMESTAMPTZ NOT NULL,
  removed_at TIMESTAMPTZ,
  room_id UUID NOT NULL REFERENCES rooms (id),
  PRIMARY KEY (device_id, placed_at),
  CHECK (
    removed_at IS NULL
    OR placed_at < removed_at
  )
);

CREATE UNIQUE INDEX ON switchbot_device_locations (device_id)
WHERE
  removed_at IS NULL;

CREATE TABLE switchbot_measurements (
  device_id BYTES NOT NULL REFERENCES switchbot_devices (id),
  measured_at TIMESTAMPTZ NOT NULL,
  temperature_celsius FLOAT NOT NULL,
  humidity_percent INT NOT NULL,
  co2_ppm INT,
  light_level INT,
  PRIMARY KEY (device_id, measured_at),
  CHECK (
    0 <= light_level
    AND light_level <= 20
  )
);
