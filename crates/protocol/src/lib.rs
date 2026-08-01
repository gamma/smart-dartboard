//! Decoder for the ten-byte notifications emitted by the board's FFF1
//! characteristic and the stateful interpretation of its ambiguous neutral
//! packet.

use serde::{Deserialize, Serialize};
use std::fmt::Write as _;
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum HexError {
    #[error("odd hex length: {0}")]
    OddLength(usize),
    #[error("invalid hex digit")]
    InvalidDigit,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PacketBase {
    pub seq: u32,
    pub event_type: u16,
    pub raw: String,
    pub code: String,
    pub checksum: u8,
    pub checksum_ok: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DecodedPacket {
    InvalidLength {
        length: usize,
        raw: String,
    },
    ChecksumError {
        expected_checksum: u8,
        #[serde(flatten)]
        base: PacketBase,
    },
    Button {
        button: String,
        action: String,
        #[serde(flatten)]
        base: PacketBase,
    },
    Neutral {
        meaning: String,
        #[serde(flatten)]
        base: PacketBase,
    },
    Miss {
        score: u16,
        label: String,
        #[serde(flatten)]
        base: PacketBase,
    },
    Hit {
        field: u8,
        ring: String,
        multiplier: u8,
        label: String,
        score: u16,
        #[serde(flatten)]
        base: PacketBase,
    },
    Unknown {
        #[serde(flatten)]
        base: PacketBase,
    },
}

/// Removes separators from a hexadecimal board packet and decodes its bytes.
///
/// # Errors
///
/// Returns [`HexError::OddLength`] when the normalized input contains an odd
/// number of digits, or [`HexError::InvalidDigit`] when decoding a pair fails.
pub fn normalize_hex(input: &str) -> Result<Vec<u8>, HexError> {
    let compact: String = input.chars().filter(char::is_ascii_hexdigit).collect();
    if !compact.len().is_multiple_of(2) {
        return Err(HexError::OddLength(compact.len()));
    }
    (0..compact.len())
        .step_by(2)
        .map(|index| {
            u8::from_str_radix(&compact[index..index + 2], 16).map_err(|_| HexError::InvalidDigit)
        })
        .collect()
}

#[must_use]
pub fn decode_packet(data: &[u8]) -> DecodedPacket {
    if data.len() != 10 {
        return DecodedPacket::InvalidLength {
            length: data.len(),
            raw: hex(data),
        };
    }

    let ring = data[6];
    let ones = data[7];
    let tens = data[8];
    let checksum = data[9];
    let expected_checksum = ring.wrapping_add(ones).wrapping_add(tens);
    let base = PacketBase {
        seq: u32::from_le_bytes([data[0], data[1], data[2], data[3]]),
        event_type: u16::from_le_bytes([data[4], data[5]]),
        raw: hex(data),
        code: format!("{ring:02x} {ones:02x} {tens:02x}"),
        checksum,
        checksum_ok: checksum == expected_checksum,
    };

    if checksum != expected_checksum {
        return DecodedPacket::ChecksumError {
            expected_checksum,
            base,
        };
    }

    match (ring, ones, tens) {
        (0x00, 0x00, 0xff) => DecodedPacket::Button {
            button: "menu".into(),
            action: "press".into(),
            base,
        },
        (0x00, 0x00, 0xcc) => DecodedPacket::Button {
            button: "menu".into(),
            action: "long_press".into(),
            base,
        },
        (0x00, 0x00, 0xee) => DecodedPacket::Neutral {
            meaning: "miss_or_button_release".into(),
            base,
        },
        (0x0c, 0x00, 0x0e) => DecodedPacket::Hit {
            field: 25,
            ring: "single_bull".into(),
            multiplier: 1,
            label: "SBull".into(),
            score: 25,
            base,
        },
        (0x0d, 0x00, 0x0f) => DecodedPacket::Hit {
            field: 25,
            ring: "double_bull".into(),
            multiplier: 2,
            label: "DBull".into(),
            score: 50,
            base,
        },
        _ => decode_number(ring, ones, tens, base),
    }
}

fn decode_number(ring: u8, ones: u8, tens: u8, base: PacketBase) -> DecodedPacket {
    let field = ones.saturating_add(10_u8.saturating_mul(tens));
    let mapping = match ring {
        0x0a => Some(("single_inner", 1, "S")),
        0x0b => Some(("triple", 3, "T")),
        0x0c => Some(("single_outer", 1, "S")),
        0x0d => Some(("double", 2, "D")),
        _ => None,
    };
    if let Some((ring_name, multiplier, prefix)) = mapping
        && (1..=20).contains(&field)
    {
        return DecodedPacket::Hit {
            field,
            ring: ring_name.into(),
            multiplier,
            label: format!("{prefix}{field}"),
            score: u16::from(field) * u16::from(multiplier),
            base,
        };
    }
    DecodedPacket::Unknown { base }
}

fn hex(data: &[u8]) -> String {
    let mut output = String::with_capacity(data.len() * 2);
    for byte in data {
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

#[derive(Debug, Default)]
pub struct EventInterpreter {
    menu_button_down: bool,
}

impl EventInterpreter {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            menu_button_down: false,
        }
    }

    pub fn interpret(&mut self, packet: DecodedPacket) -> DecodedPacket {
        match packet {
            DecodedPacket::Button { action, base, .. } if action == "press" => {
                self.menu_button_down = true;
                DecodedPacket::Button {
                    button: "menu".into(),
                    action,
                    base,
                }
            }
            DecodedPacket::Button {
                button,
                action,
                base,
            } => DecodedPacket::Button {
                button,
                action,
                base,
            },
            DecodedPacket::Neutral { base, .. } if self.menu_button_down => {
                self.menu_button_down = false;
                DecodedPacket::Button {
                    button: "menu".into(),
                    action: "release".into(),
                    base,
                }
            }
            DecodedPacket::Neutral { base, .. } => DecodedPacket::Miss {
                score: 0,
                label: "MISS".into(),
                base,
            },
            other => other,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn neutral_is_miss_without_button_press() {
        let bytes = normalize_hex("0100000000000000eeee").expect("hex");
        let packet = EventInterpreter::new().interpret(decode_packet(&bytes));
        assert!(matches!(packet, DecodedPacket::Miss { .. }));
    }

    #[test]
    fn neutral_releases_pressed_button() {
        let press = normalize_hex("0100000000000000ffff").expect("hex");
        let release = normalize_hex("0200000000000000eeee").expect("hex");
        let mut interpreter = EventInterpreter::new();
        interpreter.interpret(decode_packet(&press));
        assert!(matches!(
            interpreter.interpret(decode_packet(&release)),
            DecodedPacket::Button { action, .. } if action == "release"
        ));
    }
}
