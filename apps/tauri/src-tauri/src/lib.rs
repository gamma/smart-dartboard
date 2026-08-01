use sdb_board::BoardStatus;
#[cfg(any(target_os = "ios", target_os = "macos"))]
use sdb_board::{BoardFailureCode, BoardPhase};
#[cfg(any(target_os = "ios", target_os = "macos", test))]
use sdb_board::{BoardIngress, BoardIngressOutcome};
use sdb_companion::{
    CompanionRole, PairedDevice, PairingAuthority, PairingBootstrap, PairingGrant, PairingRequest,
};
use sdb_companion_transport::{SecretStore, TlsIdentity, load_identity, load_or_create_identity};
use sdb_contracts::{DartEvent, DartSource, Ring};
use sdb_runtime::{Runtime, RuntimeAction, RuntimeGameState};
use sdb_storage::SqliteRepository;
use serde::{Deserialize, Serialize};
use std::{
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};
use tauri::{Emitter, Manager, State};
use tokio::sync::broadcast;
use uuid::Uuid;

const PROJECTOR_OUTPUT_PREFERENCE: &str = "projector.output";
const COMPANION_HOST_ID_PREFERENCE: &str = "companion.host_id";

#[cfg(any(target_os = "ios", target_os = "macos"))]
use std::sync::OnceLock;
#[cfg(target_os = "ios")]
use std::sync::atomic::AtomicU32;
#[cfg(any(target_os = "ios", target_os = "macos"))]
use std::sync::atomic::{AtomicU16, Ordering};
#[cfg(any(target_os = "ios", target_os = "macos"))]
static APP_HANDLE: OnceLock<tauri::AppHandle> = OnceLock::new();

#[cfg(target_os = "ios")]
static EXTERNAL_DISPLAY_COUNT: AtomicU32 = AtomicU32::new(0);
#[cfg(any(target_os = "ios", target_os = "macos"))]
static COMPANION_PORT: AtomicU16 = AtomicU16::new(0);

struct NativeState {
    runtime: Runtime<SqliteRepository>,
    next_dart_seq: u64,
    #[cfg(any(target_os = "ios", target_os = "macos", test))]
    board_ingress: BoardIngress,
    board_status: BoardStatus,
    companions: PairingAuthority,
    projector_output: ProjectorOutput,
    companion_identity: CompanionIdentity,
    companion_states: broadcast::Sender<PublicState>,
    companion_changes: broadcast::Sender<()>,
}

type SharedNativeState = Arc<Mutex<NativeState>>;

#[derive(Debug, Clone)]
struct CompanionIdentity {
    host_id: String,
    certificate_sha256: String,
    available: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum ProjectorOutput {
    ExternalDisplay,
    Companion,
    LocalPreview,
}

impl ProjectorOutput {
    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "external_display" => Ok(Self::ExternalDisplay),
            "companion" => Ok(Self::Companion),
            "local_preview" => Ok(Self::LocalPreview),
            _ => Err("unsupported projector output".into()),
        }
    }

    const fn storage_value(self) -> &'static str {
        match self {
            Self::ExternalDisplay => "external_display",
            Self::Companion => "companion",
            Self::LocalPreview => "local_preview",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
struct PublicState {
    runtime_instance_id: String,
    revision: u64,
    counter: u64,
    external_display_count: u32,
    board: BoardStatus,
    game: Option<RuntimeGameState>,
    projector_output: ProjectorOutput,
    companion_port: Option<u16>,
    companion_available: bool,
}

#[derive(Debug, Clone, Serialize)]
struct CompanionDeviceView {
    device_id: String,
    device_name: String,
    role: CompanionRole,
    paired_at_ms: u64,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
struct DiscoveredCompanionHost {
    service_name: String,
    host_name: String,
    port: u16,
    host_id: String,
    protocol_version: u16,
    tls: bool,
}

fn is_valid_mdns_hostname(host_name: &str) -> bool {
    if host_name.len() > 253 || !host_name.ends_with(".local") {
        return false;
    }
    host_name.split('.').all(|label| {
        !label.is_empty()
            && label.len() <= 63
            && !label.starts_with('-')
            && !label.ends_with('-')
            && label
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    })
}

impl From<PairedDevice> for CompanionDeviceView {
    fn from(device: PairedDevice) -> Self {
        Self {
            device_id: device.device_id,
            device_name: device.device_name,
            role: device.role,
            paired_at_ms: device.paired_at_ms,
        }
    }
}

impl NativeState {
    fn restore(
        repository: SqliteRepository,
        companion_identity: CompanionIdentity,
    ) -> Result<Self, String> {
        let companion_devices = repository
            .companion_devices()
            .map_err(|error| error.to_string())?;
        let projector_output = repository
            .preference(PROJECTOR_OUTPUT_PREFERENCE)
            .map_err(|error| error.to_string())?
            .map_or(Ok(ProjectorOutput::ExternalDisplay), |value| {
                ProjectorOutput::parse(&value)
            })?;
        let (companion_states, _) = broadcast::channel(64);
        let (companion_changes, _) = broadcast::channel(16);
        let runtime_instance_id = Uuid::new_v4().to_string();
        let mut runtime = Runtime::restore(runtime_instance_id.clone(), repository)
            .map_err(|error| error.to_string())?;
        if runtime.snapshot().revision == 0 {
            runtime
                .dispatch(
                    &runtime_instance_id,
                    "bootstrap-countup",
                    Some(0),
                    RuntimeAction::StartCountUp {
                        players: vec![("test-player".into(), "Test Player".into())],
                        rounds: 20,
                    },
                )
                .map_err(|error| error.to_string())?;
        }
        Ok(Self {
            runtime,
            next_dart_seq: 1,
            #[cfg(any(target_os = "ios", target_os = "macos", test))]
            board_ingress: BoardIngress::new(),
            board_status: BoardStatus::unavailable(),
            companions: PairingAuthority::from_devices(companion_devices),
            projector_output,
            companion_identity,
            companion_states,
            companion_changes,
        })
    }

    fn public(&self) -> PublicState {
        let game = self
            .runtime
            .snapshot()
            .game
            .as_ref()
            .map(|game| game.state());
        let counter = game.as_ref().map_or(0, |state| match state {
            RuntimeGameState::CountUp(state) => state.players[0].score.into(),
            RuntimeGameState::X01(state) => state.players[0].score.into(),
        });
        PublicState {
            runtime_instance_id: self.runtime.instance_id().into(),
            revision: self.runtime.snapshot().revision,
            counter,
            external_display_count: external_display_count(),
            board: self.board_status.clone(),
            game,
            projector_output: self.projector_output,
            companion_port: companion_port(),
            companion_available: self.companion_identity.available,
        }
    }

    fn ingest_test_hit(&mut self) -> Result<PublicState, String> {
        let seq = self.next_dart_seq;
        self.next_dart_seq += 1;
        let runtime_instance_id = self.runtime.instance_id().to_owned();
        let command_id = format!("test-hit:{runtime_instance_id}:{seq}");
        self.runtime
            .dispatch(
                &runtime_instance_id,
                &command_id,
                Some(self.runtime.snapshot().revision),
                RuntimeAction::Dart {
                    event: DartEvent::Hit {
                        seq,
                        field: 20,
                        ring: Ring::Triple,
                        multiplier: 3,
                        label: "T20".into(),
                        score: 60,
                    },
                    source: DartSource::ProjectorTest,
                },
            )
            .map_err(|error| error.to_string())?;
        Ok(self.public())
    }

    #[allow(dead_code)] // Native network ingress calls this; it must never become a WebView command.
    fn pair_companion(
        &mut self,
        request: PairingRequest,
        paired_at_ms: u64,
    ) -> Result<PairingGrant, String> {
        let grant = self
            .companions
            .pair(request, paired_at_ms)
            .map_err(|error| error.to_string())?;
        let device = self
            .companions
            .device(&grant.device_id)
            .cloned()
            .ok_or_else(|| "paired companion is missing".to_owned())?;
        if self
            .runtime
            .repository_mut()
            .save_companion_device(&device)
            .is_err()
        {
            let persisted = self
                .runtime
                .repository()
                .companion_devices()
                .map_err(|error| error.to_string())?;
            self.companions = PairingAuthority::from_devices(persisted);
            return Err("companion grant persistence failed".into());
        }
        Ok(grant)
    }

    fn companion_devices(&self) -> Vec<CompanionDeviceView> {
        self.companions
            .devices()
            .into_iter()
            .map(CompanionDeviceView::from)
            .collect()
    }

    fn revoke_companion(&mut self, device_id: &str, revoked_at_ms: u64) -> Result<(), String> {
        if self.companions.device(device_id).is_none() {
            return Err("companion device not found".into());
        }
        let persisted = self
            .runtime
            .repository_mut()
            .revoke_companion_device(device_id, revoked_at_ms)
            .map_err(|error| error.to_string())?;
        if !persisted || !self.companions.revoke(device_id) {
            return Err("companion grant state is inconsistent".into());
        }
        let _ = self.companion_changes.send(());
        Ok(())
    }

    fn select_projector_output(&mut self, output: ProjectorOutput) -> Result<PublicState, String> {
        self.runtime
            .repository_mut()
            .save_preference(PROJECTOR_OUTPUT_PREFERENCE, output.storage_value())
            .map_err(|error| error.to_string())?;
        self.projector_output = output;
        Ok(self.public())
    }

    #[cfg(any(target_os = "ios", target_os = "macos", test))]
    fn ingest_board_packet(&mut self, connection_id: &str, raw: &[u8]) -> Result<bool, String> {
        let BoardIngressOutcome::Dart { event, command_id } =
            self.board_ingress.ingest(connection_id, raw)
        else {
            return Ok(false);
        };
        let runtime_instance_id = self.runtime.instance_id().to_owned();
        self.runtime
            .dispatch(
                &runtime_instance_id,
                &command_id,
                Some(self.runtime.snapshot().revision),
                RuntimeAction::Dart {
                    event,
                    source: DartSource::Board,
                },
            )
            .map_err(|error| error.to_string())?;
        Ok(true)
    }
}

#[cfg(target_os = "ios")]
fn external_display_count() -> u32 {
    EXTERNAL_DISPLAY_COUNT.load(Ordering::Relaxed)
}

#[cfg(not(target_os = "ios"))]
const fn external_display_count() -> u32 {
    0
}

#[cfg(any(target_os = "ios", target_os = "macos"))]
fn companion_port() -> Option<u16> {
    match COMPANION_PORT.load(Ordering::Acquire) {
        0 => None,
        port => Some(port),
    }
}

#[cfg(not(any(target_os = "ios", target_os = "macos")))]
const fn companion_port() -> Option<u16> {
    None
}

#[cfg(target_os = "ios")]
#[allow(unsafe_code)]
mod ios_display {
    use super::PublicState;
    use std::ffi::{CString, c_char, c_void};

    type ProjectorUpdate = unsafe extern "C" fn(*const c_char);
    fn lookup(symbol: &std::ffi::CStr) -> Option<*mut c_void> {
        let address = unsafe { libc::dlsym(libc::RTLD_DEFAULT, symbol.as_ptr()) };
        if address.is_null() {
            None
        } else {
            Some(address)
        }
    }

    fn projector_update() -> Option<ProjectorUpdate> {
        lookup(c"sdb_projector_update")
            .map(|address| unsafe { std::mem::transmute::<*mut c_void, ProjectorUpdate>(address) })
    }

    pub fn publish(state: &PublicState) {
        let Ok(json) = serde_json::to_string(state) else {
            return;
        };
        let Ok(json) = CString::new(json) else {
            return;
        };
        if let Some(update) = projector_update() {
            unsafe { update(json.as_ptr()) };
        }
    }
}

#[cfg(any(target_os = "ios", target_os = "macos"))]
#[allow(unsafe_code)]
mod apple_keychain {
    use super::SecretStore;
    #[cfg(target_os = "ios")]
    use std::ffi::{CStr, c_void};
    use std::ffi::{CString, c_char};

    const MAX_SECRET_BYTES: usize = 128 * 1_024;

    #[cfg(target_os = "macos")]
    unsafe extern "C" {
        fn sdb_keychain_load(
            account: *const c_char,
            bytes: *mut *mut u8,
            length: *mut usize,
        ) -> i32;
        fn sdb_keychain_save(account: *const c_char, bytes: *const u8, length: usize) -> bool;
        fn sdb_keychain_free(bytes: *mut u8, length: usize);
    }

    type KeychainLoad = unsafe extern "C" fn(*const c_char, *mut *mut u8, *mut usize) -> i32;
    type KeychainSave = unsafe extern "C" fn(*const c_char, *const u8, usize) -> bool;
    type KeychainFree = unsafe extern "C" fn(*mut u8, usize);

    #[cfg(target_os = "ios")]
    fn lookup(symbol: &CStr) -> Option<*mut c_void> {
        let address = unsafe { libc::dlsym(libc::RTLD_DEFAULT, symbol.as_ptr()) };
        (!address.is_null()).then_some(address)
    }

    #[cfg(target_os = "ios")]
    fn keychain_load() -> Option<KeychainLoad> {
        lookup(c"sdb_keychain_load")
            .map(|address| unsafe { std::mem::transmute::<*mut c_void, KeychainLoad>(address) })
    }

    #[cfg(target_os = "macos")]
    const fn keychain_load() -> Option<KeychainLoad> {
        Some(sdb_keychain_load)
    }

    #[cfg(target_os = "ios")]
    fn keychain_save() -> Option<KeychainSave> {
        lookup(c"sdb_keychain_save")
            .map(|address| unsafe { std::mem::transmute::<*mut c_void, KeychainSave>(address) })
    }

    #[cfg(target_os = "macos")]
    const fn keychain_save() -> Option<KeychainSave> {
        Some(sdb_keychain_save)
    }

    #[cfg(target_os = "ios")]
    fn keychain_free() -> Option<KeychainFree> {
        lookup(c"sdb_keychain_free")
            .map(|address| unsafe { std::mem::transmute::<*mut c_void, KeychainFree>(address) })
    }

    #[cfg(target_os = "macos")]
    const fn keychain_free() -> Option<KeychainFree> {
        Some(sdb_keychain_free)
    }

    pub struct AppleKeychainStore;

    impl SecretStore for AppleKeychainStore {
        fn load(&self, key: &str) -> Result<Option<Vec<u8>>, String> {
            let account = CString::new(key).map_err(|_| "invalid Keychain account".to_owned())?;
            let mut bytes = std::ptr::null_mut();
            let mut length = 0_usize;
            let load = keychain_load().ok_or_else(|| "Keychain host is unavailable".to_owned())?;
            let free = keychain_free().ok_or_else(|| "Keychain host is unavailable".to_owned())?;
            let status = unsafe { load(account.as_ptr(), &mut bytes, &mut length) };
            if status == 0 {
                return Ok(None);
            }
            if status != 1 || bytes.is_null() || length == 0 {
                return Err("Keychain read failed".into());
            }
            if length > MAX_SECRET_BYTES {
                unsafe { free(bytes, length) };
                return Err("Keychain secret exceeds size limit".into());
            }
            let value = unsafe { std::slice::from_raw_parts(bytes, length) }.to_vec();
            unsafe { free(bytes, length) };
            Ok(Some(value))
        }

        fn save(&self, key: &str, value: &[u8]) -> Result<(), String> {
            if value.is_empty() || value.len() > MAX_SECRET_BYTES {
                return Err("Keychain secret has invalid size".into());
            }
            let account = CString::new(key).map_err(|_| "invalid Keychain account".to_owned())?;
            let save = keychain_save().ok_or_else(|| "Keychain host is unavailable".to_owned())?;
            if unsafe { save(account.as_ptr(), value.as_ptr(), value.len()) } {
                Ok(())
            } else {
                Err("Keychain write failed".into())
            }
        }
    }
}

#[cfg(any(target_os = "ios", target_os = "macos"))]
#[allow(unsafe_code)]
mod apple_bonjour {
    #[cfg(target_os = "ios")]
    use std::ffi::{CStr, c_void};
    use std::ffi::{CString, c_char};

    type BonjourStart = unsafe extern "C" fn(u16, *const c_char, u16) -> i32;
    type BonjourStop = unsafe extern "C" fn();
    type BrowserStart = unsafe extern "C" fn() -> i32;
    type BrowserSnapshot = unsafe extern "C" fn(*mut *mut u8, *mut usize) -> i32;
    type BrowserSnapshotFree = unsafe extern "C" fn(*mut u8, usize);

    #[cfg(target_os = "macos")]
    unsafe extern "C" {
        fn sdb_companion_bonjour_start(
            port: u16,
            host_id: *const c_char,
            protocol_version: u16,
        ) -> i32;
        fn sdb_companion_bonjour_stop();
        fn sdb_companion_bonjour_browser_start() -> i32;
        fn sdb_companion_bonjour_browser_stop();
        fn sdb_companion_bonjour_browser_snapshot(bytes: *mut *mut u8, length: *mut usize) -> i32;
        fn sdb_companion_bonjour_browser_snapshot_free(bytes: *mut u8, length: usize);
    }

    #[cfg(target_os = "ios")]
    fn lookup(symbol: &CStr) -> Option<*mut c_void> {
        let address = unsafe { libc::dlsym(libc::RTLD_DEFAULT, symbol.as_ptr()) };
        (!address.is_null()).then_some(address)
    }

    #[cfg(target_os = "ios")]
    fn start_function() -> Option<BonjourStart> {
        lookup(c"sdb_companion_bonjour_start")
            .map(|address| unsafe { std::mem::transmute::<*mut c_void, BonjourStart>(address) })
    }

    #[cfg(target_os = "macos")]
    const fn start_function() -> Option<BonjourStart> {
        Some(sdb_companion_bonjour_start)
    }

    #[cfg(target_os = "ios")]
    fn stop_function() -> Option<BonjourStop> {
        lookup(c"sdb_companion_bonjour_stop")
            .map(|address| unsafe { std::mem::transmute::<*mut c_void, BonjourStop>(address) })
    }

    #[cfg(target_os = "macos")]
    const fn stop_function() -> Option<BonjourStop> {
        Some(sdb_companion_bonjour_stop)
    }

    #[cfg(target_os = "ios")]
    fn browser_start_function() -> Option<BrowserStart> {
        lookup(c"sdb_companion_bonjour_browser_start")
            .map(|address| unsafe { std::mem::transmute::<*mut c_void, BrowserStart>(address) })
    }

    #[cfg(target_os = "macos")]
    const fn browser_start_function() -> Option<BrowserStart> {
        Some(sdb_companion_bonjour_browser_start)
    }

    #[cfg(target_os = "ios")]
    fn browser_stop_function() -> Option<BonjourStop> {
        lookup(c"sdb_companion_bonjour_browser_stop")
            .map(|address| unsafe { std::mem::transmute::<*mut c_void, BonjourStop>(address) })
    }

    #[cfg(target_os = "macos")]
    const fn browser_stop_function() -> Option<BonjourStop> {
        Some(sdb_companion_bonjour_browser_stop)
    }

    #[cfg(target_os = "ios")]
    fn browser_snapshot_function() -> Option<BrowserSnapshot> {
        lookup(c"sdb_companion_bonjour_browser_snapshot")
            .map(|address| unsafe { std::mem::transmute::<*mut c_void, BrowserSnapshot>(address) })
    }

    #[cfg(target_os = "macos")]
    const fn browser_snapshot_function() -> Option<BrowserSnapshot> {
        Some(sdb_companion_bonjour_browser_snapshot)
    }

    #[cfg(target_os = "ios")]
    fn browser_snapshot_free_function() -> Option<BrowserSnapshotFree> {
        lookup(c"sdb_companion_bonjour_browser_snapshot_free").map(|address| unsafe {
            std::mem::transmute::<*mut c_void, BrowserSnapshotFree>(address)
        })
    }

    #[cfg(target_os = "macos")]
    const fn browser_snapshot_free_function() -> Option<BrowserSnapshotFree> {
        Some(sdb_companion_bonjour_browser_snapshot_free)
    }

    pub fn start(port: u16, host_id: &str, protocol_version: u16) -> Result<(), String> {
        let host_id = CString::new(host_id).map_err(|_| "invalid Bonjour host ID".to_owned())?;
        let start = start_function().ok_or_else(|| "Bonjour host is unavailable".to_owned())?;
        let status = unsafe { start(port, host_id.as_ptr(), protocol_version) };
        if status == 0 {
            Ok(())
        } else {
            Err(format!("Bonjour registration failed with status {status}"))
        }
    }

    pub fn stop() {
        if let Some(stop) = stop_function() {
            unsafe { stop() };
        }
    }

    pub fn browser_start() -> Result<(), String> {
        let start =
            browser_start_function().ok_or_else(|| "Bonjour browser is unavailable".to_owned())?;
        let status = unsafe { start() };
        if status == 0 {
            Ok(())
        } else {
            Err(format!("Bonjour browse failed with status {status}"))
        }
    }

    pub fn browser_stop() {
        if let Some(stop) = browser_stop_function() {
            unsafe { stop() };
        }
    }

    pub fn browser_snapshot() -> Result<Vec<super::DiscoveredCompanionHost>, String> {
        const MAX_SNAPSHOT_BYTES: usize = 256 * 1_024;
        let snapshot = browser_snapshot_function()
            .ok_or_else(|| "Bonjour browser is unavailable".to_owned())?;
        let free = browser_snapshot_free_function()
            .ok_or_else(|| "Bonjour browser is unavailable".to_owned())?;
        let mut bytes = std::ptr::null_mut();
        let mut length = 0_usize;
        let status = unsafe { snapshot(&mut bytes, &mut length) };
        if status != 0 || bytes.is_null() || length == 0 {
            return Err("Bonjour browser snapshot failed".into());
        }
        if length > MAX_SNAPSHOT_BYTES {
            unsafe { free(bytes, length) };
            return Err("Bonjour browser snapshot exceeds size limit".into());
        }
        let json = unsafe { std::slice::from_raw_parts(bytes, length) }.to_vec();
        unsafe { free(bytes, length) };
        let hosts: Vec<super::DiscoveredCompanionHost> =
            serde_json::from_slice(&json).map_err(|_| "invalid Bonjour browser snapshot")?;
        if hosts.iter().any(|host| {
            host.service_name.is_empty()
                || host.service_name.len() > 128
                || !super::is_valid_mdns_hostname(&host.host_name)
                || uuid::Uuid::parse_str(&host.host_id).is_err()
                || host.port == 0
                || !host.tls
        }) {
            return Err("invalid Bonjour host metadata".into());
        }
        Ok(hosts)
    }
}

#[cfg(any(target_os = "ios", target_os = "macos"))]
mod native_companion_transport {
    use super::{
        COMPANION_PORT, NativeState, PublicState, SharedNativeState, TlsIdentity, apple_bonjour,
        now_ms,
    };
    use axum::{
        Json, Router,
        extract::{State, WebSocketUpgrade, ws::Message},
        http::{HeaderMap, HeaderValue, StatusCode, header},
        response::{IntoResponse, Response},
        routing::{get, post},
    };
    use axum_server::{Handle, tls_rustls::RustlsConfig};
    use sdb_companion::{
        COMPANION_PROTOCOL_VERSION, CompanionFrame, CompanionFrameKind, PairingGrant,
        PairingRequest,
    };
    use serde::Serialize;
    use std::{
        net::{SocketAddr, TcpListener},
        sync::atomic::Ordering,
        time::Duration,
    };
    use tokio::sync::{Mutex as AsyncMutex, broadcast};
    use tower_http::set_header::SetResponseHeaderLayer;

    #[derive(Clone)]
    struct ServiceState {
        native: SharedNativeState,
    }

    struct RunningTransport {
        port: u16,
        handle: Handle<SocketAddr>,
    }

    pub struct NativeCompanionTransport {
        tls: RustlsConfig,
        host_id: String,
        running: AsyncMutex<Option<RunningTransport>>,
    }

    impl NativeCompanionTransport {
        pub fn new(identity: &TlsIdentity) -> Result<Self, String> {
            let config = identity
                .rustls_server_config()
                .map_err(|error| error.to_string())?;
            Ok(Self {
                tls: RustlsConfig::from_config(config),
                host_id: identity.host_id().into(),
                running: AsyncMutex::new(None),
            })
        }

        pub async fn port(&self) -> Option<u16> {
            self.running
                .lock()
                .await
                .as_ref()
                .map(|running| running.port)
        }

        pub async fn start(&self, native: SharedNativeState) -> Result<u16, String> {
            let mut running = self.running.lock().await;
            if let Some(running) = running.as_ref() {
                return Ok(running.port);
            }
            let listener = TcpListener::bind("[::]:0")
                .map_err(|error| format!("companion listener bind failed: {error}"))?;
            listener
                .set_nonblocking(true)
                .map_err(|error| format!("companion listener setup failed: {error}"))?;
            let address = listener
                .local_addr()
                .map_err(|error| format!("companion listener address failed: {error}"))?;
            let handle = Handle::new();
            let server = axum_server::from_tcp_rustls(listener, self.tls.clone())
                .map_err(|error| format!("companion TLS listener failed: {error}"))?
                .handle(handle.clone())
                .serve(router(native).into_make_service());
            apple_bonjour::start(address.port(), &self.host_id, COMPANION_PROTOCOL_VERSION)
                .inspect_err(|_| handle.shutdown())?;
            tauri::async_runtime::spawn(async move {
                if let Err(error) = server.await {
                    eprintln!("companion TLS server stopped: {error}");
                }
            });
            COMPANION_PORT.store(address.port(), Ordering::Release);
            *running = Some(RunningTransport {
                port: address.port(),
                handle,
            });
            Ok(address.port())
        }

        pub async fn stop(&self) {
            let running = self.running.lock().await.take();
            COMPANION_PORT.store(0, Ordering::Release);
            apple_bonjour::stop();
            if let Some(running) = running {
                running
                    .handle
                    .graceful_shutdown(Some(Duration::from_secs(1)));
            }
        }
    }

    fn router(native: SharedNativeState) -> Router {
        Router::new()
            .route("/api/v2/companion/pairing", post(pair))
            .route("/api/v2/companion/runtime/bootstrap", get(bootstrap))
            .route("/api/v2/companion/runtime/events", get(websocket))
            .layer(SetResponseHeaderLayer::if_not_present(
                header::X_CONTENT_TYPE_OPTIONS,
                HeaderValue::from_static("nosniff"),
            ))
            .layer(SetResponseHeaderLayer::if_not_present(
                header::REFERRER_POLICY,
                HeaderValue::from_static("no-referrer"),
            ))
            .with_state(ServiceState { native })
    }

    async fn pair(
        State(service): State<ServiceState>,
        headers: HeaderMap,
        Json(request): Json<PairingRequest>,
    ) -> Result<Json<PairingGrant>, TransportError> {
        reject_browser_origin(&headers)?;
        let grant = service
            .native
            .lock()
            .map_err(|_| TransportError::internal())?
            .pair_companion(request, now_ms())
            .map_err(|_| TransportError::bad_request("pairing rejected"))?;
        Ok(Json(grant))
    }

    async fn bootstrap(
        State(service): State<ServiceState>,
        headers: HeaderMap,
    ) -> Result<Json<CompanionFrame>, TransportError> {
        reject_browser_origin(&headers)?;
        let token = bearer_token(&headers)?;
        let state = service
            .native
            .lock()
            .map_err(|_| TransportError::internal())?;
        authenticate(&state, token)?;
        Ok(Json(snapshot_frame(&state)?))
    }

    async fn websocket(
        State(service): State<ServiceState>,
        headers: HeaderMap,
        upgrade: WebSocketUpgrade,
    ) -> Result<Response, TransportError> {
        reject_browser_origin(&headers)?;
        let token = bearer_token(&headers)?.to_owned();
        let states = {
            let state = service
                .native
                .lock()
                .map_err(|_| TransportError::internal())?;
            authenticate(&state, &token)?;
            state.companion_states.subscribe()
        };
        let changes = {
            let state = service
                .native
                .lock()
                .map_err(|_| TransportError::internal())?;
            state.companion_changes.subscribe()
        };
        let initial = {
            let state = service
                .native
                .lock()
                .map_err(|_| TransportError::internal())?;
            authenticate(&state, &token)?;
            snapshot_frame(&state)?
        };
        Ok(upgrade.on_upgrade(move |socket| {
            stream(socket, service.native, token, initial, states, changes)
        }))
    }

    async fn stream(
        mut socket: axum::extract::ws::WebSocket,
        native: SharedNativeState,
        token: String,
        initial: CompanionFrame,
        mut states: broadcast::Receiver<PublicState>,
        mut changes: broadcast::Receiver<()>,
    ) {
        if send_frame(&mut socket, &initial).await.is_err() {
            return;
        }
        let runtime_instance_id = initial.runtime_instance_id.clone();
        let mut revision = initial.revision;
        loop {
            tokio::select! {
                state = states.recv() => {
                    let Ok(state) = state else {
                        break;
                    };
                    if !token_is_active(&native, &token) {
                        break;
                    }
                    if state.runtime_instance_id != runtime_instance_id
                        || state.revision > revision.saturating_add(1)
                    {
                        break;
                    }
                    if state.revision <= revision {
                        continue;
                    }
                    let Ok(frame) = state_frame(&state) else {
                        break;
                    };
                    if send_frame(&mut socket, &frame).await.is_err() {
                        return;
                    }
                    revision = state.revision;
                }
                changed = changes.recv() => {
                    if changed.is_err() || !token_is_active(&native, &token) {
                        break;
                    }
                }
            }
        }
        let _ = socket.send(Message::Close(None)).await;
    }

    async fn send_frame(
        socket: &mut axum::extract::ws::WebSocket,
        frame: &CompanionFrame,
    ) -> Result<(), axum::Error> {
        let json = serde_json::to_string(frame).map_err(axum::Error::new)?;
        socket.send(Message::Text(json.into())).await
    }

    fn snapshot_frame(state: &NativeState) -> Result<CompanionFrame, TransportError> {
        frame(&state.public(), CompanionFrameKind::Snapshot)
    }

    fn state_frame(state: &PublicState) -> Result<CompanionFrame, TransportError> {
        frame(state, CompanionFrameKind::State)
    }

    fn frame(
        state: &PublicState,
        kind: CompanionFrameKind,
    ) -> Result<CompanionFrame, TransportError> {
        Ok(CompanionFrame {
            protocol_version: COMPANION_PROTOCOL_VERSION,
            runtime_instance_id: state.runtime_instance_id.clone(),
            revision: state.revision,
            kind,
            payload: serde_json::to_value(state).map_err(|_| TransportError::internal())?,
        })
    }

    fn reject_browser_origin(headers: &HeaderMap) -> Result<(), TransportError> {
        if headers.contains_key(header::ORIGIN) {
            return Err(TransportError::forbidden("browser origins are not allowed"));
        }
        Ok(())
    }

    fn bearer_token(headers: &HeaderMap) -> Result<&str, TransportError> {
        headers
            .get(header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.strip_prefix("Bearer "))
            .filter(|value| !value.is_empty())
            .ok_or_else(|| TransportError::forbidden("missing companion token"))
    }

    fn authenticate(state: &NativeState, token: &str) -> Result<(), TransportError> {
        state
            .companions
            .authenticate(token)
            .map(|_| ())
            .ok_or_else(|| TransportError::forbidden("invalid companion token"))
    }

    fn token_is_active(native: &SharedNativeState, token: &str) -> bool {
        native
            .lock()
            .is_ok_and(|state| state.companions.authenticate(token).is_some())
    }

    #[derive(Serialize)]
    struct ErrorBody {
        error: &'static str,
    }

    struct TransportError {
        status: StatusCode,
        message: &'static str,
    }

    impl TransportError {
        const fn bad_request(message: &'static str) -> Self {
            Self {
                status: StatusCode::BAD_REQUEST,
                message,
            }
        }

        const fn forbidden(message: &'static str) -> Self {
            Self {
                status: StatusCode::FORBIDDEN,
                message,
            }
        }

        const fn internal() -> Self {
            Self {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                message: "companion transport unavailable",
            }
        }
    }

    impl IntoResponse for TransportError {
        fn into_response(self) -> Response {
            (
                self.status,
                Json(ErrorBody {
                    error: self.message,
                }),
            )
                .into_response()
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::CompanionIdentity;
        use axum::{
            body::{Body, to_bytes},
            http::Request,
        };
        use sdb_companion::{CompanionFrame, PairingGrant};
        use sdb_companion_transport::{SecretStore, load_or_create_identity};
        use sdb_storage::SqliteRepository;
        use std::sync::{Arc, Mutex as StdMutex};
        use tokio::{
            io::{AsyncReadExt, AsyncWriteExt},
            net::TcpStream,
            time::{Duration, timeout},
        };
        use tokio_rustls::TlsConnector;
        use tokio_rustls::rustls::{
            ClientConfig, RootCertStore,
            pki_types::{CertificateDer, ServerName},
        };
        use tower::ServiceExt;

        fn test_state(host_id: &str, fingerprint: &str) -> SharedNativeState {
            Arc::new(StdMutex::new(
                NativeState::restore(
                    SqliteRepository::in_memory().expect("repository"),
                    CompanionIdentity {
                        host_id: host_id.into(),
                        certificate_sha256: fingerprint.into(),
                        available: true,
                    },
                )
                .expect("native state"),
            ))
        }

        #[tokio::test]
        async fn router_pairs_bootstraps_and_rejects_browser_origins_and_revocation() {
            let state = test_state("native-host", &"ab".repeat(32));
            let offer = state
                .lock()
                .expect("state")
                .companions
                .open(now_ms())
                .expect("pairing offer");
            let app = router(state.clone());
            let request = PairingRequest {
                device_id: "projector-ipad".into(),
                device_name: "Arcade iPad".into(),
                code: offer.code,
            };
            let response = app
                .clone()
                .oneshot(
                    Request::post("/api/v2/companion/pairing")
                        .header(header::CONTENT_TYPE, "application/json")
                        .body(Body::from(serde_json::to_vec(&request).expect("request")))
                        .expect("request"),
                )
                .await
                .expect("pairing response");
            assert_eq!(response.status(), StatusCode::OK);
            let grant: PairingGrant = serde_json::from_slice(
                &to_bytes(response.into_body(), 16 * 1_024)
                    .await
                    .expect("grant body"),
            )
            .expect("grant");

            let response = app
                .clone()
                .oneshot(
                    Request::get("/api/v2/companion/runtime/bootstrap")
                        .header(header::AUTHORIZATION, format!("Bearer {}", grant.token))
                        .body(Body::empty())
                        .expect("request"),
                )
                .await
                .expect("bootstrap response");
            assert_eq!(response.status(), StatusCode::OK);
            let frame: CompanionFrame = serde_json::from_slice(
                &to_bytes(response.into_body(), 128 * 1_024)
                    .await
                    .expect("frame body"),
            )
            .expect("frame");
            assert_eq!(frame.kind, CompanionFrameKind::Snapshot);

            let response = app
                .clone()
                .oneshot(
                    Request::get("/api/v2/companion/runtime/bootstrap")
                        .header(header::AUTHORIZATION, format!("Bearer {}", grant.token))
                        .header(header::ORIGIN, "https://evil.invalid")
                        .body(Body::empty())
                        .expect("request"),
                )
                .await
                .expect("origin response");
            assert_eq!(response.status(), StatusCode::FORBIDDEN);

            state
                .lock()
                .expect("state")
                .revoke_companion("projector-ipad", now_ms())
                .expect("revoke");
            let response = app
                .oneshot(
                    Request::get("/api/v2/companion/runtime/bootstrap")
                        .header(header::AUTHORIZATION, format!("Bearer {}", grant.token))
                        .body(Body::empty())
                        .expect("request"),
                )
                .await
                .expect("revoked response");
            assert_eq!(response.status(), StatusCode::FORBIDDEN);
        }

        #[derive(Default)]
        struct MemorySecretStore(StdMutex<Option<Vec<u8>>>);

        impl SecretStore for MemorySecretStore {
            fn load(&self, _key: &str) -> Result<Option<Vec<u8>>, String> {
                Ok(self.0.lock().map_err(|error| error.to_string())?.clone())
            }

            fn save(&self, _key: &str, value: &[u8]) -> Result<(), String> {
                *self.0.lock().map_err(|error| error.to_string())? = Some(value.to_vec());
                Ok(())
            }
        }

        #[tokio::test]
        async fn listener_serves_http_over_the_pinned_tls_identity() {
            let identity = load_or_create_identity(
                &MemorySecretStore::default(),
                "0dc0b075-e6b5-4d6d-a6cf-ef4dbb61f2f7",
            )
            .expect("identity");
            let state = test_state(identity.host_id(), identity.certificate_sha256());
            let transport = NativeCompanionTransport::new(&identity).expect("transport");
            let port = transport.start(state).await.expect("start");

            let mut roots = RootCertStore::empty();
            roots
                .add(CertificateDer::from(identity.certificate_der().to_vec()))
                .expect("root certificate");
            let provider = Arc::new(tokio_rustls::rustls::crypto::ring::default_provider());
            let config = ClientConfig::builder_with_provider(provider)
                .with_safe_default_protocol_versions()
                .expect("protocol versions")
                .with_root_certificates(roots)
                .with_no_client_auth();
            let connector = TlsConnector::from(Arc::new(config));
            let server_name =
                ServerName::try_from(format!("{}.local", identity.host_id())).expect("server name");
            let tcp = timeout(Duration::from_secs(5), TcpStream::connect(("::1", port)))
                .await
                .expect("connect timeout")
                .expect("connect");
            let mut tls = timeout(Duration::from_secs(5), connector.connect(server_name, tcp))
                .await
                .expect("TLS timeout")
                .expect("TLS handshake");
            tls.write_all(
                b"GET /api/v2/companion/runtime/bootstrap HTTP/1.1\r\nHost: tls-host.local\r\nConnection: close\r\n\r\n",
            )
            .await
            .expect("HTTP request");
            let mut response = Vec::new();
            timeout(Duration::from_secs(5), tls.read_to_end(&mut response))
                .await
                .expect("response timeout")
                .expect("HTTP response");
            let response = String::from_utf8(response).expect("UTF-8 response");
            assert!(response.starts_with("HTTP/1.1 403 Forbidden"));

            apple_bonjour::browser_start().expect("start Bonjour browser");
            let discovered = timeout(Duration::from_secs(5), async {
                loop {
                    if let Some(host) = apple_bonjour::browser_snapshot()
                        .expect("Bonjour snapshot")
                        .into_iter()
                        .find(|host| host.host_id == identity.host_id())
                    {
                        break host;
                    }
                    tokio::time::sleep(Duration::from_millis(50)).await;
                }
            })
            .await
            .expect("Bonjour discovery timeout");
            assert_eq!(discovered.port, port);
            assert_eq!(discovered.protocol_version, COMPANION_PROTOCOL_VERSION);
            assert!(discovered.tls);
            apple_bonjour::browser_stop();
            transport.stop().await;
        }
    }
}

#[cfg(any(target_os = "ios", target_os = "macos"))]
struct NativeCompanionService {
    transport: Option<Arc<native_companion_transport::NativeCompanionTransport>>,
}

#[cfg(any(target_os = "ios", target_os = "macos"))]
#[allow(unsafe_code)]
mod apple_board {
    use super::{
        APP_HANDLE, BoardFailureCode, BoardPhase, BoardStatus, SharedNativeState,
        publish_public_state,
    };
    use std::ffi::{CStr, c_char};
    use tauri::Manager;

    fn phase(value: u32) -> BoardPhase {
        match value {
            1 => BoardPhase::PermissionRequired,
            2 => BoardPhase::BluetoothOff,
            3 => BoardPhase::Scanning,
            4 => BoardPhase::Connecting,
            5 => BoardPhase::Discovering,
            6 => BoardPhase::Subscribing,
            7 => BoardPhase::Ready,
            8 => BoardPhase::Reconnecting,
            9 => BoardPhase::Error,
            _ => BoardPhase::Unavailable,
        }
    }

    fn failure(value: i32) -> Option<BoardFailureCode> {
        match value {
            1 => Some(BoardFailureCode::AdapterUnavailable),
            2 => Some(BoardFailureCode::PermissionDenied),
            3 => Some(BoardFailureCode::BluetoothPoweredOff),
            4 => Some(BoardFailureCode::DeviceNotFound),
            5 => Some(BoardFailureCode::ConnectionFailed),
            6 => Some(BoardFailureCode::ServiceMissing),
            7 => Some(BoardFailureCode::CharacteristicMissing),
            8 => Some(BoardFailureCode::SubscriptionFailed),
            9 => Some(BoardFailureCode::QueueOverflow),
            10 => Some(BoardFailureCode::RuntimeUnavailable),
            11 => Some(BoardFailureCode::TransportError),
            _ => None,
        }
    }

    unsafe fn string(pointer: *const c_char) -> Option<String> {
        if pointer.is_null() {
            return None;
        }
        unsafe { CStr::from_ptr(pointer) }
            .to_str()
            .ok()
            .map(str::to_owned)
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn sdb_board_status_changed(
        phase_value: u32,
        failure_value: i32,
        detail: *const c_char,
        connection_id: *const c_char,
    ) {
        let detail = unsafe { string(detail) }.filter(|value| value.len() <= 256);
        let connection_id = unsafe { string(connection_id) }.filter(|value| value.len() <= 64);
        let Some(app) = APP_HANDLE.get() else {
            return;
        };
        let Some(state) = app.try_state::<SharedNativeState>() else {
            return;
        };
        let public = {
            let Ok(mut state) = state.lock() else {
                return;
            };
            state.board_status = BoardStatus {
                enabled: true,
                phase: phase(phase_value),
                failure_code: failure(failure_value),
                detail,
                connection_id,
            };
            state.public()
        };
        publish_public_state(app, &public);
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn sdb_board_notification(
        bytes: *const u8,
        length: usize,
        connection_id: *const c_char,
    ) {
        if bytes.is_null() || length > 64 {
            return;
        }
        let Some(connection_id) = (unsafe { string(connection_id) }) else {
            return;
        };
        if connection_id.len() > 64 {
            return;
        }
        let raw = unsafe { std::slice::from_raw_parts(bytes, length) };
        let Some(app) = APP_HANDLE.get() else {
            return;
        };
        let Some(state) = app.try_state::<SharedNativeState>() else {
            return;
        };
        let public = {
            let Ok(mut state) = state.lock() else {
                return;
            };
            match state.ingest_board_packet(&connection_id, raw) {
                Ok(true) => state.public(),
                Ok(false) => return,
                Err(error) => {
                    state.board_status.failure_code = Some(BoardFailureCode::RuntimeUnavailable);
                    state.board_status.detail = Some(error);
                    state.public()
                }
            }
        };
        publish_public_state(app, &public);
    }
}

#[cfg(target_os = "macos")]
#[allow(unsafe_code)]
mod macos_board_host {
    #[link(name = "sdb_apple_board_transport", kind = "static")]
    unsafe extern "C" {
        fn sdb_install_board_transport_host();
    }

    pub fn install() {
        unsafe { sdb_install_board_transport_host() };
    }
}

#[cfg(target_os = "ios")]
fn publish_to_external_projector(state: &PublicState) {
    ios_display::publish(state);
}

#[cfg(not(target_os = "ios"))]
fn publish_to_external_projector(_state: &PublicState) {}

#[cfg(target_os = "ios")]
#[derive(Clone, Serialize)]
struct DisplayStatus {
    external_display_count: u32,
}

#[cfg(target_os = "ios")]
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn sdb_external_display_changed(display_count: u32) {
    EXTERNAL_DISPLAY_COUNT.store(display_count, Ordering::Relaxed);
    if let Some(app) = APP_HANDLE.get() {
        let _ = app.emit(
            "display-status",
            DisplayStatus {
                external_display_count: display_count,
            },
        );
    }
}

#[tauri::command]
fn runtime_bootstrap(state: State<'_, SharedNativeState>) -> Result<PublicState, String> {
    state
        .lock()
        .map(|state| state.public())
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn runtime_query(state: State<'_, SharedNativeState>) -> Result<PublicState, String> {
    runtime_bootstrap(state)
}

#[tauri::command]
fn companion_pairing_open(state: State<'_, SharedNativeState>) -> Result<PairingBootstrap, String> {
    let mut state = state.lock().map_err(|error| error.to_string())?;
    if !state.companion_identity.available {
        return Err("Companion ist auf diesem Gerät derzeit nicht verfügbar".into());
    }
    if state.projector_output != ProjectorOutput::Companion || companion_port().is_none() {
        return Err("select and start the Companion projector output first".into());
    }
    let offer = state
        .companions
        .open(now_ms())
        .map_err(|error| error.to_string())?;
    PairingBootstrap::new(
        state.companion_identity.host_id.clone(),
        state.companion_identity.certificate_sha256.clone(),
        offer,
    )
    .map_err(|error| error.to_string())
}

#[tauri::command]
fn companion_devices(
    state: State<'_, SharedNativeState>,
) -> Result<Vec<CompanionDeviceView>, String> {
    state
        .lock()
        .map(|state| state.companion_devices())
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn companion_revoke(
    state: State<'_, SharedNativeState>,
    device_id: String,
) -> Result<Vec<CompanionDeviceView>, String> {
    let mut state = state.lock().map_err(|error| error.to_string())?;
    state.revoke_companion(&device_id, now_ms())?;
    Ok(state.companion_devices())
}

#[tauri::command]
#[cfg(any(target_os = "ios", target_os = "macos"))]
fn companion_discovery_start() -> Result<(), String> {
    apple_bonjour::browser_start()
}

#[tauri::command]
#[cfg(not(any(target_os = "ios", target_os = "macos")))]
fn companion_discovery_start() -> Result<(), String> {
    Err("native Companion discovery is not available on this platform".into())
}

#[tauri::command]
#[cfg(any(target_os = "ios", target_os = "macos"))]
fn companion_discovered_hosts() -> Result<Vec<DiscoveredCompanionHost>, String> {
    apple_bonjour::browser_snapshot()
}

#[tauri::command]
#[cfg(not(any(target_os = "ios", target_os = "macos")))]
fn companion_discovered_hosts() -> Result<Vec<DiscoveredCompanionHost>, String> {
    Err("native Companion discovery is not available on this platform".into())
}

#[tauri::command]
#[cfg(any(target_os = "ios", target_os = "macos"))]
fn companion_discovery_stop() {
    apple_bonjour::browser_stop();
}

#[tauri::command]
#[cfg(not(any(target_os = "ios", target_os = "macos")))]
fn companion_discovery_stop() {}

#[tauri::command]
#[cfg(any(target_os = "ios", target_os = "macos"))]
async fn projector_output_select(
    app: tauri::AppHandle,
    state: State<'_, SharedNativeState>,
    service: State<'_, NativeCompanionService>,
    output: String,
) -> Result<PublicState, String> {
    let output = ProjectorOutput::parse(&output)?;
    let transport = service.transport.clone();
    let shared_state = state.inner().clone();
    let public = if output == ProjectorOutput::Companion {
        let transport = transport
            .as_ref()
            .ok_or_else(|| "Companion ist auf diesem Gerät derzeit nicht verfügbar".to_owned())?;
        let was_running = transport.port().await.is_some();
        transport.start(shared_state.clone()).await?;
        let selection = {
            let mut state = shared_state.lock().map_err(|error| error.to_string())?;
            state.select_projector_output(output)
        };
        match selection {
            Ok(public) => public,
            Err(error) => {
                if !was_running {
                    transport.stop().await;
                }
                return Err(error);
            }
        }
    } else {
        shared_state
            .lock()
            .map_err(|error| error.to_string())?
            .select_projector_output(output)?;
        if let Some(transport) = transport {
            transport.stop().await;
        }
        shared_state
            .lock()
            .map_err(|error| error.to_string())?
            .public()
    };
    publish_public_state(&app, &public);
    Ok(public)
}

#[tauri::command]
#[cfg(not(any(target_os = "ios", target_os = "macos")))]
fn projector_output_select(
    app: tauri::AppHandle,
    state: State<'_, SharedNativeState>,
    output: String,
) -> Result<PublicState, String> {
    let output = ProjectorOutput::parse(&output)?;
    if output == ProjectorOutput::Companion {
        return Err("native Companion transport is not available on this platform".into());
    }
    let public = state
        .lock()
        .map_err(|error| error.to_string())?
        .select_projector_output(output)?;
    publish_public_state(&app, &public);
    Ok(public)
}

#[tauri::command]
fn runtime_dispatch(
    app: tauri::AppHandle,
    state: State<'_, SharedNativeState>,
    action: String,
) -> Result<PublicState, String> {
    if action != "increment" {
        return Err("unsupported M0 action".into());
    }
    increment_runtime(&app, &state)
}

fn increment_runtime(
    app: &tauri::AppHandle,
    state: &SharedNativeState,
) -> Result<PublicState, String> {
    let public = {
        let mut state = state.lock().map_err(|error| error.to_string())?;
        state.ingest_test_hit()?
    };
    publish_public_state(app, &public);
    Ok(public)
}

fn publish_public_state(app: &tauri::AppHandle, public: &PublicState) {
    publish_to_external_projector(public);
    if let Some(state) = app.try_state::<SharedNativeState>()
        && let Ok(state) = state.lock()
    {
        let _ = state.companion_states.send(public.clone());
    }
    let _ = app.emit("runtime-state", public);
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

#[cfg(any(target_os = "ios", target_os = "macos"))]
fn prepare_companion_identity(repository: &mut SqliteRepository) -> Result<TlsIdentity, String> {
    if let Some(host_id) = repository
        .preference(COMPANION_HOST_ID_PREFERENCE)
        .map_err(|error| error.to_string())?
    {
        return load_or_create_identity(&apple_keychain::AppleKeychainStore, &host_id)
            .map_err(|error| error.to_string());
    }
    if let Some(identity) =
        load_identity(&apple_keychain::AppleKeychainStore).map_err(|error| error.to_string())?
    {
        repository
            .save_preference(COMPANION_HOST_ID_PREFERENCE, identity.host_id())
            .map_err(|error| error.to_string())?;
        return Ok(identity);
    }
    let host_id = Uuid::new_v4().to_string();
    repository
        .save_preference(COMPANION_HOST_ID_PREFERENCE, &host_id)
        .map_err(|error| error.to_string())?;
    load_or_create_identity(&apple_keychain::AppleKeychainStore, &host_id)
        .map_err(|error| error.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            let mut repository = SqliteRepository::open(data_dir.join("runtime.sqlite"))
                .map_err(std::io::Error::other)?;
            #[cfg(any(target_os = "ios", target_os = "macos"))]
            let companion_setup =
                prepare_companion_identity(&mut repository).and_then(|identity| {
                    let companion_identity = CompanionIdentity {
                        host_id: identity.host_id().into(),
                        certificate_sha256: identity.certificate_sha256().into(),
                        available: true,
                    };
                    let transport =
                        native_companion_transport::NativeCompanionTransport::new(&identity)?;
                    Ok((companion_identity, Arc::new(transport)))
                });
            #[cfg(any(target_os = "ios", target_os = "macos"))]
            let (companion_identity, companion_transport) = match companion_setup {
                Ok(setup) => (setup.0, Some(setup.1)),
                Err(error) => {
                    eprintln!("Companion setup is unavailable: {error}");
                    (
                        CompanionIdentity {
                            host_id: "unavailable-native-host".into(),
                            certificate_sha256: "00".repeat(32),
                            available: false,
                        },
                        None,
                    )
                }
            };
            #[cfg(not(any(target_os = "ios", target_os = "macos")))]
            let companion_identity = CompanionIdentity {
                host_id: "unsupported-native-host".into(),
                certificate_sha256: "00".repeat(32),
                available: false,
            };
            let mut native_state = NativeState::restore(repository, companion_identity)
                .map_err(std::io::Error::other)?;
            #[cfg(any(target_os = "ios", target_os = "macos"))]
            let restore_companion = native_state.projector_output == ProjectorOutput::Companion
                && companion_transport.is_some();
            #[cfg(any(target_os = "ios", target_os = "macos"))]
            if native_state.projector_output == ProjectorOutput::Companion
                && companion_transport.is_none()
            {
                native_state.projector_output = ProjectorOutput::ExternalDisplay;
            }
            let native_state = Arc::new(Mutex::new(native_state));
            app.manage(native_state.clone());
            #[cfg(any(target_os = "ios", target_os = "macos"))]
            app.manage(NativeCompanionService {
                transport: companion_transport.clone(),
            });
            #[cfg(any(target_os = "ios", target_os = "macos"))]
            let _ = APP_HANDLE.set(app.handle().clone());
            #[cfg(target_os = "ios")]
            {
                if let Some(state) = app.try_state::<SharedNativeState>() {
                    if let Ok(state) = state.lock() {
                        publish_to_external_projector(&state.public());
                    }
                }
            }
            #[cfg(target_os = "macos")]
            macos_board_host::install();
            #[cfg(any(target_os = "ios", target_os = "macos"))]
            if restore_companion {
                let app_handle = app.handle().clone();
                let companion_transport = companion_transport
                    .expect("restored Companion output requires an available transport");
                tauri::async_runtime::spawn(async move {
                    if let Err(error) = companion_transport.start(native_state.clone()).await {
                        eprintln!("companion TLS startup failed: {error}");
                        return;
                    }
                    let public = native_state.lock().ok().map(|state| state.public());
                    if let Some(public) = public {
                        publish_public_state(&app_handle, &public);
                    }
                });
            }
            #[cfg(debug_assertions)]
            if std::env::args().any(|argument| argument == "--m0-test-hit-after-start") {
                let app = app.handle().clone();
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_secs(3));
                    if let Some(state) = app.try_state::<SharedNativeState>() {
                        let _ = increment_runtime(&app, &state);
                    }
                });
            }
            #[cfg(desktop)]
            tauri::WebviewWindowBuilder::new(
                app,
                "projector",
                tauri::WebviewUrl::App("index.html?role=projector".into()),
            )
            .title("Smart Dartboard · Projector M0")
            .inner_size(1280.0, 720.0)
            .build()?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            runtime_bootstrap,
            runtime_query,
            runtime_dispatch,
            companion_pairing_open,
            companion_devices,
            companion_revoke,
            companion_discovery_start,
            companion_discovered_hosts,
            companion_discovery_stop,
            projector_output_select
        ])
        .run(tauri::generate_context!())
        .expect("error while running Smart Dartboard native M0");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_companion_identity() -> CompanionIdentity {
        CompanionIdentity {
            host_id: "test-native-host".into(),
            certificate_sha256: "ab".repeat(32),
            available: true,
        }
    }

    #[test]
    fn companion_discovery_accepts_only_local_dns_hostnames() {
        assert!(is_valid_mdns_hostname("arcade-mac.local"));
        assert!(is_valid_mdns_hostname("smart-dartboard-2.local"));
        assert!(!is_valid_mdns_hostname("example.com"));
        assert!(!is_valid_mdns_hostname("https://arcade.local"));
        assert!(!is_valid_mdns_hostname("-arcade.local"));
        assert!(!is_valid_mdns_hostname("arcade..local"));
    }

    #[test]
    fn raw_board_packet_uses_shared_ingress_and_runtime_once() {
        let repository = SqliteRepository::in_memory().expect("repository");
        let mut state =
            NativeState::restore(repository, test_companion_identity()).expect("native state");
        let packet = [1, 0, 0, 0, 5, 0, 0x0d, 0, 2, 0x0f];
        assert!(
            state
                .ingest_board_packet("test-link", &packet)
                .expect("first packet")
        );
        assert!(
            !state
                .ingest_board_packet("test-link", &packet)
                .expect("duplicate")
        );
        let public = state.public();
        assert_eq!(public.revision, 2);
        assert_eq!(public.counter, 40);
    }

    #[test]
    fn native_runtime_recovers_committed_state_with_a_new_instance_id() {
        let path = std::env::temp_dir().join(format!("sdb-native-{}.sqlite", Uuid::new_v4()));
        let first_instance = {
            let repository = SqliteRepository::open(&path).expect("first repository");
            let mut state =
                NativeState::restore(repository, test_companion_identity()).expect("first state");
            state.ingest_test_hit().expect("committed hit");
            state
                .select_projector_output(ProjectorOutput::LocalPreview)
                .expect("persist output");
            state.runtime.instance_id().to_owned()
        };
        let repository = SqliteRepository::open(&path).expect("reopened repository");
        let state =
            NativeState::restore(repository, test_companion_identity()).expect("restored state");
        assert_ne!(state.runtime.instance_id(), first_instance);
        assert_eq!(state.public().revision, 2);
        assert_eq!(state.public().counter, 60);
        assert_eq!(
            state.public().projector_output,
            ProjectorOutput::LocalPreview
        );
        std::fs::remove_file(path).expect("remove test database");
    }

    #[test]
    fn native_controller_persists_and_revokes_companion_grants() {
        let path =
            std::env::temp_dir().join(format!("sdb-native-companion-{}.sqlite", Uuid::new_v4()));
        let grant = {
            let repository = SqliteRepository::open(&path).expect("repository");
            let mut state =
                NativeState::restore(repository, test_companion_identity()).expect("native state");
            let offer = state.companions.open(1_000).expect("pairing offer");
            state
                .pair_companion(
                    PairingRequest {
                        device_id: "projector-ipad".into(),
                        device_name: "Arcade iPad".into(),
                        code: offer.code,
                    },
                    2_000,
                )
                .expect("grant")
        };

        let repository = SqliteRepository::open(&path).expect("reopened repository");
        let mut state =
            NativeState::restore(repository, test_companion_identity()).expect("restored state");
        assert_eq!(state.companion_devices().len(), 1);
        assert!(state.companions.authenticate(&grant.token).is_some());
        state
            .revoke_companion("projector-ipad", 3_000)
            .expect("revoke");
        assert!(state.companions.authenticate(&grant.token).is_none());
        drop(state);

        let repository = SqliteRepository::open(&path).expect("reopen after revoke");
        let state = NativeState::restore(repository, test_companion_identity())
            .expect("restored revoked state");
        assert!(state.companion_devices().is_empty());
        std::fs::remove_file(path).expect("remove test database");
    }
}
