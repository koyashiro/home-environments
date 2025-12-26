use macaddr::MacAddr6;

use crate::switchbot::DeviceType;

#[derive(Debug)]
pub struct Device {
    pub id: MacAddr6,

    pub r#type: DeviceType,

    pub name: String,

    pub sort_order: u8,
}
