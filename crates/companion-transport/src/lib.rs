//! Secure transport primitives shared by native and headless companion hosts.
//!
//! Platform adapters own Keychain, Keystore or file permissions. This crate
//! owns certificate generation, fingerprinting and one atomic opaque identity
//! blob so no host needs to understand or log private-key material.

use rcgen::{CertifiedKey, generate_simple_self_signed};
use rustls::{
    ServerConfig,
    pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer},
};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use thiserror::Error;
use zeroize::Zeroizing;

const IDENTITY_KEY: &str = "companion.tls.identity.v1";
const MAGIC: &[u8; 8] = b"SDBTLS1\0";
const MAX_HOST_ID_BYTES: usize = 128;
const MAX_CERTIFICATE_BYTES: usize = 64 * 1_024;
const MAX_PRIVATE_KEY_BYTES: usize = 16 * 1_024;

pub trait SecretStore {
    /// Loads one opaque secret value.
    ///
    /// # Errors
    ///
    /// Returns a platform-specific diagnostic when secure storage is
    /// unavailable. The diagnostic must not contain secret bytes.
    fn load(&self, key: &str) -> Result<Option<Vec<u8>>, String>;

    /// Atomically creates or replaces one opaque secret value.
    ///
    /// # Errors
    ///
    /// Returns a platform-specific diagnostic when secure storage is
    /// unavailable. The diagnostic must not contain secret bytes.
    fn save(&self, key: &str, value: &[u8]) -> Result<(), String>;
}

pub struct TlsIdentity {
    host_id: String,
    certificate_der: Vec<u8>,
    private_key_pkcs8_der: Zeroizing<Vec<u8>>,
    certificate_sha256: String,
}

impl std::fmt::Debug for TlsIdentity {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("TlsIdentity")
            .field("host_id", &self.host_id)
            .field("certificate_der_bytes", &self.certificate_der.len())
            .field("private_key_pkcs8_der", &"[REDACTED]")
            .field("certificate_sha256", &self.certificate_sha256)
            .finish()
    }
}

impl TlsIdentity {
    #[must_use]
    pub fn host_id(&self) -> &str {
        &self.host_id
    }

    #[must_use]
    pub fn certificate_der(&self) -> &[u8] {
        &self.certificate_der
    }

    #[must_use]
    pub fn private_key_pkcs8_der(&self) -> &[u8] {
        &self.private_key_pkcs8_der
    }

    #[must_use]
    pub fn certificate_sha256(&self) -> &str {
        &self.certificate_sha256
    }

    /// Builds a server-only `rustls` configuration and verifies that the
    /// persisted certificate and private key form a usable pair.
    ///
    /// # Errors
    ///
    /// Returns [`IdentityError::Malformed`] when either DER value is invalid or
    /// the private key does not match the certificate.
    pub fn rustls_server_config(&self) -> Result<Arc<ServerConfig>, IdentityError> {
        let provider = Arc::new(rustls::crypto::ring::default_provider());
        let certificate = CertificateDer::from(self.certificate_der.clone());
        let private_key = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(
            self.private_key_pkcs8_der.to_vec(),
        ));
        let mut config = ServerConfig::builder_with_provider(provider)
            .with_safe_default_protocol_versions()
            .map_err(|_| IdentityError::Malformed)?
            .with_no_client_auth()
            .with_single_cert(vec![certificate], private_key)
            .map_err(|_| IdentityError::Malformed)?;
        config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];
        Ok(Arc::new(config))
    }
}

/// Restores the host identity or generates and atomically stores it once.
///
/// # Errors
///
/// Rejects invalid host IDs, malformed persisted blobs, host-ID mismatches,
/// unavailable cryptographic entropy or secure-store failures.
pub fn load_or_create_identity(
    store: &impl SecretStore,
    host_id: &str,
) -> Result<TlsIdentity, IdentityError> {
    validate_host_id(host_id)?;
    if let Some(identity) = load_identity(store)? {
        if identity.host_id != host_id {
            return Err(IdentityError::HostMismatch);
        }
        return Ok(identity);
    }

    let identity = generate_identity(host_id)?;
    let blob = encode_identity(&identity)?;
    store
        .save(IDENTITY_KEY, &blob)
        .map_err(IdentityError::Store)?;
    Ok(identity)
}

/// Loads a previously generated identity without requiring an external host
/// ID. This allows secure storage to restore `SQLite` metadata after app-data
/// loss without rotating the TLS key.
///
/// # Errors
///
/// Rejects malformed persisted blobs or secure-store failures.
pub fn load_identity(store: &impl SecretStore) -> Result<Option<TlsIdentity>, IdentityError> {
    store
        .load(IDENTITY_KEY)
        .map_err(IdentityError::Store)?
        .map(|blob| {
            let blob = Zeroizing::new(blob);
            decode_identity(&blob)
        })
        .transpose()
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum IdentityError {
    #[error("companion host id is invalid")]
    InvalidHostId,
    #[error("stored companion identity is malformed")]
    Malformed,
    #[error("stored companion identity belongs to a different host id")]
    HostMismatch,
    #[error("TLS identity generation failed")]
    Generation,
    #[error("secure identity store failed: {0}")]
    Store(String),
}

fn generate_identity(host_id: &str) -> Result<TlsIdentity, IdentityError> {
    let dns_name = format!("{host_id}.local");
    let CertifiedKey { cert, signing_key } =
        generate_simple_self_signed(vec![dns_name]).map_err(|_| IdentityError::Generation)?;
    let certificate_der = cert.der().to_vec();
    let private_key_pkcs8_der = Zeroizing::new(signing_key.serialize_der());
    let certificate_sha256 = hex_digest(&certificate_der);
    Ok(TlsIdentity {
        host_id: host_id.into(),
        certificate_der,
        private_key_pkcs8_der,
        certificate_sha256,
    })
}

fn encode_identity(identity: &TlsIdentity) -> Result<Zeroizing<Vec<u8>>, IdentityError> {
    let host = identity.host_id.as_bytes();
    let host_length = u16::try_from(host.len()).map_err(|_| IdentityError::Malformed)?;
    let certificate_length =
        u32::try_from(identity.certificate_der.len()).map_err(|_| IdentityError::Malformed)?;
    let key_length = u32::try_from(identity.private_key_pkcs8_der.len())
        .map_err(|_| IdentityError::Malformed)?;
    let mut blob = Zeroizing::new(Vec::with_capacity(
        MAGIC.len() + 2 + host.len() + 4 + identity.certificate_der.len() + 4 + key_length as usize,
    ));
    blob.extend_from_slice(MAGIC);
    blob.extend_from_slice(&host_length.to_be_bytes());
    blob.extend_from_slice(host);
    blob.extend_from_slice(&certificate_length.to_be_bytes());
    blob.extend_from_slice(&identity.certificate_der);
    blob.extend_from_slice(&key_length.to_be_bytes());
    blob.extend_from_slice(&identity.private_key_pkcs8_der);
    Ok(blob)
}

fn decode_identity(blob: &[u8]) -> Result<TlsIdentity, IdentityError> {
    let mut cursor = Cursor::new(blob);
    if cursor.take(MAGIC.len())? != MAGIC {
        return Err(IdentityError::Malformed);
    }
    let host_length = usize::from(cursor.u16()?);
    if host_length == 0 || host_length > MAX_HOST_ID_BYTES {
        return Err(IdentityError::Malformed);
    }
    let host_id = std::str::from_utf8(cursor.take(host_length)?)
        .map_err(|_| IdentityError::Malformed)?
        .to_owned();
    validate_host_id(&host_id).map_err(|_| IdentityError::Malformed)?;
    let certificate_length =
        usize::try_from(cursor.u32()?).map_err(|_| IdentityError::Malformed)?;
    if certificate_length == 0 || certificate_length > MAX_CERTIFICATE_BYTES {
        return Err(IdentityError::Malformed);
    }
    let certificate_der = cursor.take(certificate_length)?.to_vec();
    let key_length = usize::try_from(cursor.u32()?).map_err(|_| IdentityError::Malformed)?;
    if key_length == 0 || key_length > MAX_PRIVATE_KEY_BYTES {
        return Err(IdentityError::Malformed);
    }
    let private_key_pkcs8_der = Zeroizing::new(cursor.take(key_length)?.to_vec());
    if !cursor.is_empty() {
        return Err(IdentityError::Malformed);
    }
    let certificate_sha256 = hex_digest(&certificate_der);
    Ok(TlsIdentity {
        host_id,
        certificate_der,
        private_key_pkcs8_der,
        certificate_sha256,
    })
}

fn validate_host_id(host_id: &str) -> Result<(), IdentityError> {
    if host_id.is_empty()
        || host_id.len() > MAX_HOST_ID_BYTES
        || !host_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return Err(IdentityError::InvalidHostId);
    }
    Ok(())
}

fn hex_digest(value: &[u8]) -> String {
    let digest = Sha256::digest(value);
    let mut output = String::with_capacity(digest.len() * 2);
    for byte in digest {
        use std::fmt::Write as _;
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

struct Cursor<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl<'a> Cursor<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, position: 0 }
    }

    fn take(&mut self, length: usize) -> Result<&'a [u8], IdentityError> {
        let end = self
            .position
            .checked_add(length)
            .ok_or(IdentityError::Malformed)?;
        let value = self
            .bytes
            .get(self.position..end)
            .ok_or(IdentityError::Malformed)?;
        self.position = end;
        Ok(value)
    }

    fn u16(&mut self) -> Result<u16, IdentityError> {
        let bytes: [u8; 2] = self
            .take(2)?
            .try_into()
            .map_err(|_| IdentityError::Malformed)?;
        Ok(u16::from_be_bytes(bytes))
    }

    fn u32(&mut self) -> Result<u32, IdentityError> {
        let bytes: [u8; 4] = self
            .take(4)?
            .try_into()
            .map_err(|_| IdentityError::Malformed)?;
        Ok(u32::from_be_bytes(bytes))
    }

    const fn is_empty(&self) -> bool {
        self.position == self.bytes.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::RefCell, collections::HashMap};

    #[derive(Default)]
    struct MemoryStore(RefCell<HashMap<String, Vec<u8>>>);

    impl SecretStore for MemoryStore {
        fn load(&self, key: &str) -> Result<Option<Vec<u8>>, String> {
            Ok(self.0.borrow().get(key).cloned())
        }

        fn save(&self, key: &str, value: &[u8]) -> Result<(), String> {
            self.0.borrow_mut().insert(key.into(), value.to_vec());
            Ok(())
        }
    }

    #[test]
    fn identity_is_generated_once_and_restored_with_the_same_fingerprint() {
        let store = MemoryStore::default();
        let first = load_or_create_identity(&store, "arcade-host").expect("first identity");
        let second = load_or_create_identity(&store, "arcade-host").expect("restored identity");
        assert_eq!(first.certificate_der(), second.certificate_der());
        assert_eq!(
            first.private_key_pkcs8_der(),
            second.private_key_pkcs8_der()
        );
        assert_eq!(first.certificate_sha256(), second.certificate_sha256());
        assert_eq!(first.certificate_sha256().len(), 64);
        assert!(!format!("{first:?}").contains("PRIVATE KEY"));
        first.rustls_server_config().expect("valid TLS pair");
    }

    #[test]
    fn identity_blob_rejects_corruption_and_host_switches() {
        let store = MemoryStore::default();
        load_or_create_identity(&store, "host-one").expect("identity");
        assert_eq!(
            load_or_create_identity(&store, "host-two").expect_err("host mismatch"),
            IdentityError::HostMismatch
        );
        store
            .0
            .borrow_mut()
            .insert(IDENTITY_KEY.into(), b"truncated".to_vec());
        assert_eq!(
            load_or_create_identity(&store, "host-one").expect_err("corrupt"),
            IdentityError::Malformed
        );
    }

    #[test]
    fn secure_blob_can_restore_host_metadata_after_database_loss() {
        let store = MemoryStore::default();
        let created = load_or_create_identity(&store, "surviving-host").expect("identity");
        let restored = load_identity(&store)
            .expect("load")
            .expect("stored identity");
        assert_eq!(restored.host_id(), "surviving-host");
        assert_eq!(restored.certificate_sha256(), created.certificate_sha256());
    }

    #[test]
    fn tls_configuration_rejects_a_mismatched_private_key() {
        let first_store = MemoryStore::default();
        let second_store = MemoryStore::default();
        let first = load_or_create_identity(&first_store, "first-host").expect("first identity");
        let second =
            load_or_create_identity(&second_store, "second-host").expect("second identity");
        let mismatched = TlsIdentity {
            host_id: first.host_id,
            certificate_der: first.certificate_der,
            private_key_pkcs8_der: second.private_key_pkcs8_der,
            certificate_sha256: first.certificate_sha256,
        };
        assert_eq!(
            mismatched
                .rustls_server_config()
                .expect_err("mismatched key"),
            IdentityError::Malformed
        );
    }
}
