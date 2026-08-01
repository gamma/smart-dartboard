//! Transport-independent companion pairing and replica rules.
//!
//! Bonjour, TLS and platform key stores live in host adapters. This crate owns
//! the security-sensitive product rules that must behave identically on every
//! host: short-lived one-time pairing, projector-only grants, revocation and
//! gap-free state replication.

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use thiserror::Error;

pub const COMPANION_PROTOCOL_VERSION: u16 = 1;
pub const PAIRING_LIFETIME_MS: u64 = 5 * 60 * 1_000;
pub const MAX_PAIRING_ATTEMPTS: u8 = 5;
pub const CERTIFICATE_SHA256_HEX_LENGTH: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompanionRole {
    Projector,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PairingOffer {
    pub code: String,
    pub expires_at_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PairingBootstrap {
    pub protocol_version: u16,
    pub host_id: String,
    pub certificate_sha256: String,
    pub offer: PairingOffer,
}

impl PairingBootstrap {
    /// Binds an offer to the TLS identity shown or scanned by the user.
    ///
    /// # Errors
    ///
    /// Rejects malformed host IDs and non-canonical SHA-256 fingerprints.
    pub fn new(
        host_id: impl Into<String>,
        certificate_sha256: impl Into<String>,
        offer: PairingOffer,
    ) -> Result<Self, PairingError> {
        let host_id = host_id.into();
        let certificate_sha256 = certificate_sha256.into();
        if host_id.is_empty()
            || host_id.len() > 128
            || !host_id
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
            || certificate_sha256.len() != CERTIFICATE_SHA256_HEX_LENGTH
            || !certificate_sha256
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
            || offer.code.len() != 6
            || !offer.code.bytes().all(|byte| byte.is_ascii_digit())
        {
            return Err(PairingError::InvalidIdentity);
        }
        Ok(Self {
            protocol_version: COMPANION_PROTOCOL_VERSION,
            host_id,
            certificate_sha256,
            offer,
        })
    }

    #[must_use]
    pub fn manual_fingerprint(&self) -> String {
        let mut output = String::with_capacity(19);
        for (index, character) in self.certificate_sha256.chars().take(16).enumerate() {
            if index > 0 && index % 4 == 0 {
                output.push('-');
            }
            output.push(character.to_ascii_uppercase());
        }
        output
    }
}

#[derive(Deserialize)]
struct PairingBootstrapWire {
    protocol_version: u16,
    host_id: String,
    certificate_sha256: String,
    offer: PairingOffer,
}

impl<'de> Deserialize<'de> for PairingBootstrap {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let wire = PairingBootstrapWire::deserialize(deserializer)?;
        if wire.protocol_version != COMPANION_PROTOCOL_VERSION {
            return Err(serde::de::Error::custom(
                "incompatible companion protocol version",
            ));
        }
        Self::new(wire.host_id, wire.certificate_sha256, wire.offer)
            .map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PairingRequest {
    pub device_id: String,
    pub device_name: String,
    pub code: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PairingGrant {
    pub device_id: String,
    pub role: CompanionRole,
    pub token: String,
    pub paired_at_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PairedDevice {
    pub device_id: String,
    pub device_name: String,
    pub role: CompanionRole,
    pub token_hash: String,
    pub paired_at_ms: u64,
}

#[derive(Debug)]
struct PendingPairing {
    code_hash: [u8; 32],
    salt: [u8; 16],
    expires_at_ms: u64,
    remaining_attempts: u8,
}

#[derive(Debug, Default)]
pub struct PairingAuthority {
    pending: Option<PendingPairing>,
    devices: HashMap<String, PairedDevice>,
}

impl PairingAuthority {
    #[must_use]
    pub fn from_devices(devices: impl IntoIterator<Item = PairedDevice>) -> Self {
        Self {
            pending: None,
            devices: devices
                .into_iter()
                .map(|device| (device.device_id.clone(), device))
                .collect(),
        }
    }

    /// Opens one five-minute pairing window and invalidates an older code.
    ///
    /// # Errors
    ///
    /// Returns [`PairingError::EntropyUnavailable`] if the operating system
    /// cannot provide cryptographically secure random bytes.
    pub fn open(&mut self, now_ms: u64) -> Result<PairingOffer, PairingError> {
        let mut entropy = [0_u8; 20];
        getrandom::fill(&mut entropy).map_err(|_| PairingError::EntropyUnavailable)?;
        Ok(self.open_with_entropy(now_ms, entropy))
    }

    /// Exchanges a valid one-time code for a projector-only bearer token.
    /// The plaintext token is returned once and only its SHA-256 digest remains
    /// in the authority.
    ///
    /// # Errors
    ///
    /// Rejects closed, expired or exhausted pairing windows, malformed device
    /// metadata, a wrong code, or missing operating-system entropy.
    pub fn pair(
        &mut self,
        request: PairingRequest,
        now_ms: u64,
    ) -> Result<PairingGrant, PairingError> {
        validate_device(&request)?;
        let pending = self.pending.as_mut().ok_or(PairingError::Closed)?;
        if now_ms >= pending.expires_at_ms {
            self.pending = None;
            return Err(PairingError::Expired);
        }
        if pending.remaining_attempts == 0 {
            self.pending = None;
            return Err(PairingError::AttemptsExhausted);
        }
        if hash_code(&pending.salt, &request.code) != pending.code_hash {
            pending.remaining_attempts -= 1;
            if pending.remaining_attempts == 0 {
                self.pending = None;
                return Err(PairingError::AttemptsExhausted);
            }
            return Err(PairingError::InvalidCode);
        }

        let mut token_bytes = [0_u8; 32];
        getrandom::fill(&mut token_bytes).map_err(|_| PairingError::EntropyUnavailable)?;
        let token = URL_SAFE_NO_PAD.encode(token_bytes);
        let token_hash = hex_digest(&token);
        let device = PairedDevice {
            device_id: request.device_id.clone(),
            device_name: request.device_name,
            role: CompanionRole::Projector,
            token_hash,
            paired_at_ms: now_ms,
        };
        self.devices.insert(request.device_id.clone(), device);
        self.pending = None;
        Ok(PairingGrant {
            device_id: request.device_id,
            role: CompanionRole::Projector,
            token,
            paired_at_ms: now_ms,
        })
    }

    #[must_use]
    pub fn authenticate(&self, token: &str) -> Option<&PairedDevice> {
        let digest = hex_digest(token);
        self.devices
            .values()
            .find(|device| constant_time_eq(device.token_hash.as_bytes(), digest.as_bytes()))
    }

    #[must_use]
    pub fn revoke(&mut self, device_id: &str) -> bool {
        self.devices.remove(device_id).is_some()
    }

    #[must_use]
    pub fn devices(&self) -> Vec<PairedDevice> {
        let mut devices: Vec<_> = self.devices.values().cloned().collect();
        devices.sort_by(|left, right| left.device_id.cmp(&right.device_id));
        devices
    }

    #[must_use]
    pub fn device(&self, device_id: &str) -> Option<&PairedDevice> {
        self.devices.get(device_id)
    }

    fn open_with_entropy(&mut self, now_ms: u64, entropy: [u8; 20]) -> PairingOffer {
        let number = u32::from_le_bytes(entropy[..4].try_into().expect("four bytes")) % 1_000_000;
        let code = format!("{number:06}");
        let salt: [u8; 16] = entropy[4..].try_into().expect("sixteen bytes");
        let expires_at_ms = now_ms.saturating_add(PAIRING_LIFETIME_MS);
        self.pending = Some(PendingPairing {
            code_hash: hash_code(&salt, &code),
            salt,
            expires_at_ms,
            remaining_attempts: MAX_PAIRING_ATTEMPTS,
        });
        PairingOffer {
            code,
            expires_at_ms,
        }
    }
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum PairingError {
    #[error("pairing is not open")]
    Closed,
    #[error("pairing code expired")]
    Expired,
    #[error("pairing code is invalid")]
    InvalidCode,
    #[error("pairing attempts exhausted")]
    AttemptsExhausted,
    #[error("device metadata is invalid")]
    InvalidDevice,
    #[error("companion TLS identity is invalid")]
    InvalidIdentity,
    #[error("secure operating-system entropy is unavailable")]
    EntropyUnavailable,
}

fn validate_device(request: &PairingRequest) -> Result<(), PairingError> {
    if request.device_id.is_empty()
        || request.device_id.len() > 128
        || request.device_name.trim().is_empty()
        || request.device_name.len() > 80
        || request.code.len() != 6
        || !request.code.bytes().all(|byte| byte.is_ascii_digit())
    {
        return Err(PairingError::InvalidDevice);
    }
    Ok(())
}

fn hash_code(salt: &[u8; 16], code: &str) -> [u8; 32] {
    let mut hash = Sha256::new();
    hash.update(salt);
    hash.update(code.as_bytes());
    hash.finalize().into()
}

fn hex_digest(value: &str) -> String {
    let digest = Sha256::digest(value.as_bytes());
    let mut output = String::with_capacity(digest.len() * 2);
    for byte in digest {
        use std::fmt::Write as _;
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

fn constant_time_eq(expected: &[u8], supplied: &[u8]) -> bool {
    let mut difference = expected.len() ^ supplied.len();
    for (index, expected_byte) in expected.iter().enumerate() {
        difference |= usize::from(*expected_byte ^ supplied.get(index).copied().unwrap_or(0));
    }
    difference == 0
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompanionFrameKind {
    Snapshot,
    State,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompanionFrame {
    pub protocol_version: u16,
    pub runtime_instance_id: String,
    pub revision: u64,
    pub kind: CompanionFrameKind,
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReplicaCursor {
    runtime_instance_id: Option<String>,
    revision: Option<u64>,
}

impl ReplicaCursor {
    #[must_use]
    pub fn runtime_instance_id(&self) -> Option<&str> {
        self.runtime_instance_id.as_deref()
    }

    #[must_use]
    pub const fn revision(&self) -> Option<u64> {
        self.revision
    }

    /// Validates ordering before a Projector applies a received frame.
    ///
    /// # Errors
    ///
    /// Returns a stable decision when protocol, runtime or revision continuity
    /// requires a new full snapshot.
    pub fn accept(&mut self, frame: &CompanionFrame) -> Result<ReplicaDecision, ReplicaError> {
        if frame.protocol_version != COMPANION_PROTOCOL_VERSION {
            return Err(ReplicaError::IncompatibleProtocol);
        }
        match frame.kind {
            CompanionFrameKind::Snapshot => self.accept_snapshot(frame),
            CompanionFrameKind::State => self.accept_state(frame),
        }
    }

    fn accept_snapshot(&mut self, frame: &CompanionFrame) -> Result<ReplicaDecision, ReplicaError> {
        if self.runtime_instance_id.as_deref() == Some(&frame.runtime_instance_id)
            && self
                .revision
                .is_some_and(|revision| frame.revision < revision)
        {
            return Err(ReplicaError::Stale);
        }
        self.runtime_instance_id = Some(frame.runtime_instance_id.clone());
        self.revision = Some(frame.revision);
        Ok(ReplicaDecision::Apply)
    }

    fn accept_state(&mut self, frame: &CompanionFrame) -> Result<ReplicaDecision, ReplicaError> {
        let (Some(runtime_instance_id), Some(revision)) =
            (self.runtime_instance_id.as_deref(), self.revision)
        else {
            return Err(ReplicaError::SnapshotRequired);
        };
        if runtime_instance_id != frame.runtime_instance_id {
            return Err(ReplicaError::SnapshotRequired);
        }
        if frame.revision == revision {
            return Ok(ReplicaDecision::Duplicate);
        }
        if frame.revision != revision.saturating_add(1) {
            return Err(if frame.revision < revision {
                ReplicaError::Stale
            } else {
                ReplicaError::SnapshotRequired
            });
        }
        self.revision = Some(frame.revision);
        Ok(ReplicaDecision::Apply)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplicaDecision {
    Apply,
    Duplicate,
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum ReplicaError {
    #[error("companion protocol is incompatible")]
    IncompatibleProtocol,
    #[error("a full snapshot is required")]
    SnapshotRequired,
    #[error("companion frame is stale")]
    Stale,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(code: &str) -> PairingRequest {
        PairingRequest {
            device_id: "ipad-projector".into(),
            device_name: "Arcade iPad".into(),
            code: code.into(),
        }
    }

    #[test]
    fn pairing_code_is_short_lived_single_use_and_projector_only() {
        let mut authority = PairingAuthority::default();
        let offer = authority.open_with_entropy(1_000, [7; 20]);
        assert_eq!(offer.code.len(), 6);
        let grant = authority
            .pair(request(&offer.code), 2_000)
            .expect("valid pairing");
        assert_eq!(grant.role, CompanionRole::Projector);
        assert_eq!(grant.token.len(), 43);
        assert_eq!(
            authority
                .authenticate(&grant.token)
                .map(|device| device.role),
            Some(CompanionRole::Projector)
        );
        assert_eq!(
            authority.pair(request(&offer.code), 2_001),
            Err(PairingError::Closed)
        );
    }

    #[test]
    fn bootstrap_binds_pairing_to_a_canonical_tls_fingerprint() {
        let bootstrap = PairingBootstrap::new(
            "host-1",
            "ab12cd34ef56ab78".repeat(4),
            PairingOffer {
                code: "123456".into(),
                expires_at_ms: 42,
            },
        )
        .expect("bootstrap");
        assert_eq!(bootstrap.protocol_version, COMPANION_PROTOCOL_VERSION);
        assert_eq!(bootstrap.manual_fingerprint(), "AB12-CD34-EF56-AB78");
        assert!(
            PairingBootstrap::new(
                "host-1",
                "AB12CD34EF56AB78".repeat(4),
                bootstrap.offer.clone(),
            )
            .is_err()
        );
        assert!(PairingBootstrap::new("../host", "ab".repeat(32), bootstrap.offer).is_err());
        assert!(
            serde_json::from_value::<PairingBootstrap>(serde_json::json!({
                "protocol_version": COMPANION_PROTOCOL_VERSION,
                "host_id": "host-1",
                "certificate_sha256": "not-a-fingerprint",
                "offer": {"code": "123456", "expires_at_ms": 42}
            }))
            .is_err()
        );
    }

    #[test]
    fn pairing_expires_and_limits_online_guesses() {
        let mut authority = PairingAuthority::default();
        let offer = authority.open_with_entropy(10, [9; 20]);
        for _ in 1..MAX_PAIRING_ATTEMPTS {
            assert_eq!(
                authority.pair(request("000000"), 20),
                Err(PairingError::InvalidCode)
            );
        }
        assert_eq!(
            authority.pair(request("000000"), 20),
            Err(PairingError::AttemptsExhausted)
        );
        assert_eq!(
            authority.pair(request(&offer.code), 20),
            Err(PairingError::Closed)
        );

        let offer = authority.open_with_entropy(100, [3; 20]);
        assert_eq!(
            authority.pair(request(&offer.code), offer.expires_at_ms),
            Err(PairingError::Expired)
        );
    }

    #[test]
    fn revocation_invalidates_a_grant_without_exposing_its_token() {
        let mut authority = PairingAuthority::default();
        let offer = authority.open_with_entropy(0, [5; 20]);
        let grant = authority.pair(request(&offer.code), 1).expect("pair");
        let persisted = authority.devices();
        assert_eq!(persisted.len(), 1);
        assert_ne!(persisted[0].token_hash, grant.token);
        assert!(authority.revoke("ipad-projector"));
        assert!(authority.authenticate(&grant.token).is_none());
    }

    fn frame(instance: &str, revision: u64, kind: CompanionFrameKind) -> CompanionFrame {
        CompanionFrame {
            protocol_version: COMPANION_PROTOCOL_VERSION,
            runtime_instance_id: instance.into(),
            revision,
            kind,
            payload: serde_json::json!({"revision": revision}),
        }
    }

    #[test]
    fn replica_requires_snapshot_then_gap_free_revisions() {
        let mut replica = ReplicaCursor::default();
        assert_eq!(
            replica.accept(&frame("runtime-a", 4, CompanionFrameKind::State)),
            Err(ReplicaError::SnapshotRequired)
        );
        assert_eq!(
            replica.accept(&frame("runtime-a", 4, CompanionFrameKind::Snapshot)),
            Ok(ReplicaDecision::Apply)
        );
        assert_eq!(
            replica.accept(&frame("runtime-a", 5, CompanionFrameKind::State)),
            Ok(ReplicaDecision::Apply)
        );
        assert_eq!(
            replica.accept(&frame("runtime-a", 5, CompanionFrameKind::State)),
            Ok(ReplicaDecision::Duplicate)
        );
        assert_eq!(
            replica.accept(&frame("runtime-a", 7, CompanionFrameKind::State)),
            Err(ReplicaError::SnapshotRequired)
        );
        assert_eq!(
            replica.accept(&frame("runtime-b", 1, CompanionFrameKind::State)),
            Err(ReplicaError::SnapshotRequired)
        );
        assert_eq!(
            replica.accept(&frame("runtime-b", 1, CompanionFrameKind::Snapshot)),
            Ok(ReplicaDecision::Apply)
        );
    }
}
