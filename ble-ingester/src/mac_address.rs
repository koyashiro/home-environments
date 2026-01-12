use std::fmt;

#[derive(PartialEq, Eq, Clone, Copy, Hash)]
pub struct MacAddress([u8; 6]);

impl MacAddress {
    pub const fn new(v: [u8; 6]) -> MacAddress {
        MacAddress(v)
    }

    pub const fn as_bytes(&self) -> &[u8; 6] {
        &self.0
    }
}

impl From<[u8; 6]> for MacAddress {
    fn from(v: [u8; 6]) -> Self {
        MacAddress::new(v)
    }
}

impl AsRef<[u8; 6]> for MacAddress {
    fn as_ref(&self) -> &[u8; 6] {
        self.as_bytes()
    }
}

impl fmt::Debug for MacAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
            self.0[0], self.0[1], self.0[2], self.0[3], self.0[4], self.0[5]
        )
    }
}

impl fmt::Display for MacAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
            self.0[0], self.0[1], self.0[2], self.0[3], self.0[4], self.0[5]
        )
    }
}
