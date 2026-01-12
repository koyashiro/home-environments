use std::fmt;
use std::str::FromStr;

use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer};

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct MacAddress([u8; 6]);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseMacAddressError;

impl fmt::Display for ParseMacAddressError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid MAC address format")
    }
}

impl std::error::Error for ParseMacAddressError {}

impl<'de> Deserialize<'de> for MacAddress {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct MacAddressVisitor;

        impl Visitor<'_> for MacAddressVisitor {
            type Value = MacAddress;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a MAC address string like \"AA:BB:CC:DD:EE:FF\"")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                v.parse().map_err(de::Error::custom)
            }
        }

        deserializer.deserialize_str(MacAddressVisitor)
    }
}

const fn parse_hex_digit(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'A'..=b'F' => Some(c - b'A' + 10),
        b'a'..=b'f' => Some(c - b'a' + 10),
        _ => None,
    }
}

/// Parses MAC address bytes from a string slice.
/// This function is public for use by the `mac_address!` macro.
#[doc(hidden)]
pub const fn parse_mac_bytes(s: &[u8]) -> Option<[u8; 6]> {
    if s.len() != 17 {
        return None;
    }
    if s[2] != b':' || s[5] != b':' || s[8] != b':' || s[11] != b':' || s[14] != b':' {
        return None;
    }

    let (Some(d0), Some(d1)) = (parse_hex_digit(s[0]), parse_hex_digit(s[1])) else {
        return None;
    };
    let (Some(d2), Some(d3)) = (parse_hex_digit(s[3]), parse_hex_digit(s[4])) else {
        return None;
    };
    let (Some(d4), Some(d5)) = (parse_hex_digit(s[6]), parse_hex_digit(s[7])) else {
        return None;
    };
    let (Some(d6), Some(d7)) = (parse_hex_digit(s[9]), parse_hex_digit(s[10])) else {
        return None;
    };
    let (Some(d8), Some(d9)) = (parse_hex_digit(s[12]), parse_hex_digit(s[13])) else {
        return None;
    };
    let (Some(d10), Some(d11)) = (parse_hex_digit(s[15]), parse_hex_digit(s[16])) else {
        return None;
    };

    Some([
        (d0 << 4) | d1,
        (d2 << 4) | d3,
        (d4 << 4) | d5,
        (d6 << 4) | d7,
        (d8 << 4) | d9,
        (d10 << 4) | d11,
    ])
}

impl FromStr for MacAddress {
    type Err = ParseMacAddressError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse_mac_bytes(s.as_bytes())
            .map(MacAddress::new)
            .ok_or(ParseMacAddressError)
    }
}

/// Macro to create a MacAddress from a string literal at compile time.
///
/// # Example
/// ```
/// let mac = mac_address!("AA:BB:CC:DD:EE:FF");
/// ```
#[macro_export]
macro_rules! mac_address {
    ($s:literal) => {{
        const BYTES: [u8; 6] = $crate::mac_address::parse_mac_bytes($s.as_bytes())
            .expect("invalid MAC address format (expected XX:XX:XX:XX:XX:XX)");
        $crate::mac_address::MacAddress::new(BYTES)
    }};
}

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
