//! Transport-neutral board ingress.
//!
//! Platform adapters deliver raw FFF1 notifications and connection status.
//! This crate remains the single place that decodes, interprets and
//! deduplicates those notifications before they reach the runtime.

use sdb_contracts::{DartEvent, Ring};
use sdb_protocol::{DecodedPacket, EventInterpreter, decode_packet};
use serde::{Deserialize, Serialize};
use std::collections::{HashSet, VecDeque};

const RECENT_PACKET_LIMIT: usize = 256;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BoardPhase {
    Disabled,
    #[default]
    Unavailable,
    PermissionRequired,
    BluetoothOff,
    Scanning,
    Connecting,
    Discovering,
    Subscribing,
    Ready,
    Reconnecting,
    Error,
}

impl BoardPhase {
    #[must_use]
    pub const fn is_healthy(self) -> bool {
        matches!(self, Self::Disabled | Self::Ready)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BoardFailureCode {
    AdapterUnavailable,
    PermissionDenied,
    BluetoothPoweredOff,
    DeviceNotFound,
    ConnectionFailed,
    ServiceMissing,
    CharacteristicMissing,
    SubscriptionFailed,
    QueueOverflow,
    RuntimeUnavailable,
    TransportError,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BoardStatus {
    pub enabled: bool,
    pub phase: BoardPhase,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failure_code: Option<BoardFailureCode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connection_id: Option<String>,
}

impl BoardStatus {
    #[must_use]
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            phase: BoardPhase::Disabled,
            failure_code: None,
            detail: None,
            connection_id: None,
        }
    }

    #[must_use]
    pub fn unavailable() -> Self {
        Self {
            enabled: true,
            phase: BoardPhase::Unavailable,
            failure_code: Some(BoardFailureCode::AdapterUnavailable),
            detail: None,
            connection_id: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BoardIngressOutcome {
    Dart {
        event: DartEvent,
        command_id: String,
    },
    Button {
        button: String,
        action: String,
    },
    Duplicate,
    Rejected {
        reason: BoardRejectReason,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BoardRejectReason {
    InvalidLength,
    Checksum,
    UnknownPacket,
}

#[derive(Debug)]
pub struct BoardIngress {
    interpreter: EventInterpreter,
    recent_order: VecDeque<String>,
    recent: HashSet<String>,
}

impl Default for BoardIngress {
    fn default() -> Self {
        Self::new()
    }
}

impl BoardIngress {
    #[must_use]
    pub fn new() -> Self {
        Self {
            interpreter: EventInterpreter::new(),
            recent_order: VecDeque::with_capacity(RECENT_PACKET_LIMIT),
            recent: HashSet::with_capacity(RECENT_PACKET_LIMIT),
        }
    }

    #[must_use]
    pub fn ingest(&mut self, connection_id: &str, raw: &[u8]) -> BoardIngressOutcome {
        let decoded = decode_packet(raw);
        let identity = match packet_identity(connection_id, &decoded) {
            Ok(identity) => identity,
            Err(reason) => return BoardIngressOutcome::Rejected { reason },
        };
        if self.recent.contains(&identity) {
            return BoardIngressOutcome::Duplicate;
        }
        self.remember(identity.clone());

        match self.interpreter.interpret(decoded) {
            DecodedPacket::Hit {
                field,
                ring,
                multiplier,
                label,
                score,
                base,
            } => BoardIngressOutcome::Dart {
                event: DartEvent::Hit {
                    seq: u64::from(base.seq),
                    field,
                    ring: contract_ring(&ring),
                    multiplier,
                    label,
                    score,
                },
                command_id: format!("ble:{connection_id}:{}:{}", base.seq, base.raw),
            },
            DecodedPacket::Miss { label, score, base } => BoardIngressOutcome::Dart {
                event: DartEvent::Miss {
                    seq: u64::from(base.seq),
                    label,
                    score,
                },
                command_id: format!("ble:{connection_id}:{}:{}", base.seq, base.raw),
            },
            DecodedPacket::Button { button, action, .. } => {
                BoardIngressOutcome::Button { button, action }
            }
            _ => BoardIngressOutcome::Rejected {
                reason: BoardRejectReason::UnknownPacket,
            },
        }
    }

    fn remember(&mut self, identity: String) {
        if self.recent_order.len() == RECENT_PACKET_LIMIT
            && let Some(oldest) = self.recent_order.pop_front()
        {
            self.recent.remove(&oldest);
        }
        self.recent.insert(identity.clone());
        self.recent_order.push_back(identity);
    }
}

fn packet_identity(
    connection_id: &str,
    packet: &DecodedPacket,
) -> Result<String, BoardRejectReason> {
    let base = match packet {
        DecodedPacket::InvalidLength { .. } => return Err(BoardRejectReason::InvalidLength),
        DecodedPacket::ChecksumError { .. } => return Err(BoardRejectReason::Checksum),
        DecodedPacket::Unknown { .. } => return Err(BoardRejectReason::UnknownPacket),
        DecodedPacket::Button { base, .. }
        | DecodedPacket::Neutral { base, .. }
        | DecodedPacket::Miss { base, .. }
        | DecodedPacket::Hit { base, .. } => base,
    };
    Ok(format!("{connection_id}:{}:{}", base.seq, base.raw))
}

fn contract_ring(ring: &str) -> Ring {
    match ring {
        "single_inner" => Ring::SingleInner,
        "single_outer" => Ring::SingleOuter,
        "triple" => Ring::Triple,
        "double" => Ring::Double,
        "single_bull" => Ring::SingleBull,
        "double_bull" => Ring::DoubleBull,
        _ => unreachable!("decoder emitted unsupported ring"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sdb_protocol::normalize_hex;

    fn packet(hex: &str) -> Vec<u8> {
        normalize_hex(hex).expect("valid fixture")
    }

    #[test]
    fn converts_a_valid_hit_to_the_shared_contract() {
        let outcome = BoardIngress::new().ingest("link-a", &packet("0100000005000d00020f"));
        assert!(matches!(
            outcome,
            BoardIngressOutcome::Dart {
                event: DartEvent::Hit {
                    seq: 1,
                    field: 20,
                    ring: Ring::Double,
                    score: 40,
                    ..
                },
                ..
            }
        ));
    }

    #[test]
    fn interprets_neutral_as_miss_without_a_button_press() {
        let outcome = BoardIngress::new().ingest("link-a", &packet("0200000000000000eeee"));
        assert!(matches!(
            outcome,
            BoardIngressOutcome::Dart {
                event: DartEvent::Miss { seq: 2, .. },
                ..
            }
        ));
    }

    #[test]
    fn button_release_is_not_misreported_as_a_miss() {
        let mut ingress = BoardIngress::new();
        assert!(matches!(
            ingress.ingest("link-a", &packet("0100000000000000ffff")),
            BoardIngressOutcome::Button { action, .. } if action == "press"
        ));
        assert!(matches!(
            ingress.ingest("link-a", &packet("0200000000000000eeee")),
            BoardIngressOutcome::Button { action, .. } if action == "release"
        ));
    }

    #[test]
    fn rejects_invalid_and_checksum_failed_packets() {
        let mut ingress = BoardIngress::new();
        assert_eq!(
            ingress.ingest("link-a", &[1, 2]),
            BoardIngressOutcome::Rejected {
                reason: BoardRejectReason::InvalidLength
            }
        );
        assert_eq!(
            ingress.ingest("link-a", &packet("0100000005000d000200")),
            BoardIngressOutcome::Rejected {
                reason: BoardRejectReason::Checksum
            }
        );
    }

    #[test]
    fn deduplicates_only_within_the_same_transport_connection() {
        let raw = packet("0100000005000d00020f");
        let mut ingress = BoardIngress::new();
        assert!(matches!(
            ingress.ingest("link-a", &raw),
            BoardIngressOutcome::Dart { .. }
        ));
        assert_eq!(
            ingress.ingest("link-a", &raw),
            BoardIngressOutcome::Duplicate
        );
        assert!(matches!(
            ingress.ingest("link-b", &raw),
            BoardIngressOutcome::Dart { .. }
        ));
    }
}
