CREATE INDEX ON rooms (home_id) STORING (name);

CREATE INDEX ON switchbot_device_locations (room_id) STORING (removed_at);
