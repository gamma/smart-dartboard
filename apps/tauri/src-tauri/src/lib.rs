#[cfg(any(target_os = "ios", target_os = "macos"))]
use sdb_board::BoardFailureCode;
#[cfg(any(target_os = "ios", target_os = "macos", test))]
use sdb_board::{BoardIngress, BoardIngressOutcome};
use sdb_board::{BoardPhase, BoardStatus};
#[cfg(any(target_os = "ios", target_os = "macos"))]
use sdb_companion::{COMPANION_PROTOCOL_VERSION, CompanionFrame, ReplicaCursor, ReplicaDecision};
use sdb_companion::{
    CompanionRole, PairedDevice, PairingAuthority, PairingBootstrap, PairingGrant, PairingRequest,
};
use sdb_companion_transport::{
    SecretStore, TlsIdentity, certificate_sha256, load_identity, load_or_create_identity,
};
use sdb_contracts::{
    CommandEnvelope, DartEvent, DartSource, Envelope, MessageKind, Ring, RuntimeCommand,
};
use sdb_game_core::registered_game_metadata;
use sdb_runtime::{CommandResult, Runtime, RuntimeAction, RuntimeGameState, RuntimePublicSnapshot};
use sdb_storage::SqliteRepository;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};
#[cfg(any(target_os = "ios", target_os = "macos"))]
use tauri::async_runtime::JoinHandle;
use tauri::{Emitter, Manager, State};
#[cfg(any(target_os = "ios", target_os = "macos"))]
use tokio::sync::Mutex as AsyncMutex;
use tokio::sync::broadcast;
use uuid::Uuid;
#[cfg(any(target_os = "ios", target_os = "macos"))]
use zeroize::{Zeroize, Zeroizing};

const PROJECTOR_OUTPUT_PREFERENCE: &str = "projector.output";
#[cfg(any(target_os = "ios", target_os = "macos"))]
const COMPANION_HOST_ID_PREFERENCE: &str = "companion.host_id";
#[cfg(any(target_os = "ios", target_os = "macos"))]
const COMPANION_CLIENT_DEVICE_ID_PREFERENCE: &str = "companion.client_device_id";
#[cfg(any(target_os = "ios", target_os = "macos"))]
const COMPANION_CLIENT_HOST_ID_PREFERENCE: &str = "companion.client_host_id";
const APP_ROLE_PREFERENCE: &str = "app.role";

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
#[cfg(all(test, any(target_os = "ios", target_os = "macos")))]
static APPLE_NETWORK_TEST_LOCK: AsyncMutex<()> = AsyncMutex::const_new(());

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
    app_role: NativeAppRole,
}

type SharedNativeState = Arc<Mutex<NativeState>>;

#[derive(Debug, Clone)]
struct CompanionIdentity {
    host_id: String,
    certificate_sha256: String,
    available: bool,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum NativeAppRole {
    Controller,
    CompanionProjector,
}

impl NativeAppRole {
    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "controller" => Ok(Self::Controller),
            "companion_projector" => Ok(Self::CompanionProjector),
            _ => Err("unsupported app role".into()),
        }
    }

    const fn storage_value(self) -> &'static str {
        match self {
            Self::Controller => "controller",
            Self::CompanionProjector => "companion_projector",
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, Serialize)]
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

#[derive(Debug, Clone, Deserialize, PartialEq, Serialize)]
struct PublicState {
    app_role: NativeAppRole,
    runtime_instance_id: String,
    revision: u64,
    counter: i64,
    external_display_count: u32,
    board: BoardStatus,
    game: Option<RuntimeGameState>,
    projector_output: ProjectorOutput,
    companion_port: Option<u16>,
    companion_available: bool,
    companion_protocol_version: u16,
    test_events: bool,
    runtime: RuntimePublicSnapshot,
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

#[derive(Debug, Clone, Serialize)]
struct CompanionPairingTargetView {
    host_id: String,
    service_name: String,
    manual_fingerprint: String,
    expires_at_ms: u64,
}

#[derive(Debug, Clone, Serialize)]
struct CompanionClientView {
    host_id: String,
    service_name: String,
    paired: bool,
    phase: CompanionClientPhase,
    runtime_instance_id: Option<String>,
    revision: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum CompanionClientPhase {
    Discovering,
    Connecting,
    Connected,
    Reconnecting,
    PairingRequired,
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

fn manual_certificate_fingerprint(fingerprint: &str) -> Result<String, String> {
    if fingerprint.len() != 64
        || !fingerprint
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return Err("invalid TLS certificate fingerprint".into());
    }
    Ok(fingerprint
        .chars()
        .take(16)
        .collect::<Vec<_>>()
        .chunks(4)
        .map(|chunk| chunk.iter().collect::<String>().to_ascii_uppercase())
        .collect::<Vec<_>>()
        .join("-"))
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
        let app_role = repository
            .preference(APP_ROLE_PREFERENCE)
            .map_err(|error| error.to_string())?
            .map_or(Ok(NativeAppRole::Controller), |value| {
                NativeAppRole::parse(&value)
            })?;
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
        if runtime.snapshot().revision == 0 && app_role == NativeAppRole::Controller {
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
            board_status: if app_role == NativeAppRole::Controller {
                BoardStatus::unavailable()
            } else {
                BoardStatus::disabled()
            },
            companions: PairingAuthority::from_devices(companion_devices),
            projector_output,
            companion_identity,
            companion_states,
            companion_changes,
            app_role,
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
            RuntimeGameState::CountUp(state) => i64::from(state.players[0].score),
            RuntimeGameState::X01(state) => i64::from(state.players[0].score),
            RuntimeGameState::Registered(state) => state.players[0].score,
        });
        PublicState {
            app_role: self.app_role,
            runtime_instance_id: self.runtime.instance_id().into(),
            revision: self.runtime.snapshot().revision,
            counter,
            external_display_count: external_display_count(),
            board: self.board_status.clone(),
            game,
            projector_output: self.projector_output,
            companion_port: companion_port(),
            companion_available: self.companion_identity.available,
            companion_protocol_version: COMPANION_PROTOCOL_VERSION,
            test_events: cfg!(debug_assertions),
            runtime: self.runtime.public_snapshot(),
        }
    }

    fn select_app_role(&mut self, role: NativeAppRole) -> Result<PublicState, String> {
        self.runtime
            .repository_mut()
            .save_preference(APP_ROLE_PREFERENCE, role.storage_value())
            .map_err(|error| error.to_string())?;
        self.app_role = role;
        if role == NativeAppRole::CompanionProjector {
            self.board_status = BoardStatus::disabled();
        } else if self.board_status.phase == BoardPhase::Disabled {
            self.board_status = BoardStatus::unavailable();
        }
        Ok(self.public())
    }

    fn ingest_test_hit(&mut self) -> Result<PublicState, String> {
        self.require_controller()?;
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
        self.require_controller()?;
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
        self.require_controller()?;
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
        self.require_controller()?;
        self.runtime
            .repository_mut()
            .save_preference(PROJECTOR_OUTPUT_PREFERENCE, output.storage_value())
            .map_err(|error| error.to_string())?;
        self.projector_output = output;
        Ok(self.public())
    }

    #[cfg(any(target_os = "ios", target_os = "macos", test))]
    fn ingest_board_packet(&mut self, connection_id: &str, raw: &[u8]) -> Result<bool, String> {
        self.require_controller()?;
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

    fn require_controller(&self) -> Result<(), String> {
        if self.app_role == NativeAppRole::Controller {
            Ok(())
        } else {
            Err("command is unavailable in Companion projector mode".into())
        }
    }

    fn require_companion(&self) -> Result<(), String> {
        if self.app_role == NativeAppRole::CompanionProjector {
            Ok(())
        } else {
            Err("command is available only in Companion projector mode".into())
        }
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
    use super::{
        APP_HANDLE, CommandEnvelope, CommandResult, NativeState, SharedNativeState,
        publish_public_state, registered_game_metadata, runtime_v2_envelope,
        runtime_v2_projector_report_allowed, runtime_v2_projector_test_event_allowed,
    };
    use serde_json::{Value, json};
    use std::{
        ffi::{CStr, CString, c_char, c_void},
        sync::OnceLock,
    };
    use tauri::Manager;

    type ProjectorUpdate = unsafe extern "C" fn(*const c_char);
    static MODE_QUERY: OnceLock<Value> = OnceLock::new();

    #[repr(C)]
    pub struct ProjectorAsset {
        pub data: *mut u8,
        pub length: usize,
        pub mime: *mut c_char,
    }

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

    fn payload(state: &NativeState) -> Value {
        let modes = MODE_QUERY
            .get_or_init(|| json!(registered_game_metadata()))
            .clone();
        json!({
            "envelope": runtime_v2_envelope(state),
            "queries": {
                "/api/v2/modes": modes,
                "/api/v2/players": [],
                "/api/v2/statistics/players": [],
                "/api/v2/host": state.public(),
            }
        })
    }

    pub fn publish(state: &NativeState) {
        let Ok(json) = serde_json::to_string(&payload(state)) else {
            return;
        };
        let Ok(json) = CString::new(json) else {
            return;
        };
        if let Some(update) = projector_update() {
            unsafe { update(json.as_ptr()) };
        }
    }

    fn resolve_asset(path: &str) -> Option<tauri::Asset> {
        if path.len() > 512 || path.contains("..") || path.contains('\\') {
            return None;
        }
        let path = path.trim_start_matches('/');
        let path = if path.is_empty() {
            "projector.html"
        } else {
            path
        };
        APP_HANDLE.get()?.asset_resolver().get(path.to_owned())
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn sdb_projector_asset(
        path: *const c_char,
        output: *mut ProjectorAsset,
    ) -> bool {
        if path.is_null() || output.is_null() {
            return false;
        }
        let Ok(path) = (unsafe { CStr::from_ptr(path) }).to_str() else {
            return false;
        };
        let Some(asset) = resolve_asset(path) else {
            return false;
        };
        let Ok(mime) = CString::new(asset.mime_type) else {
            return false;
        };
        let bytes = asset.bytes.into_boxed_slice();
        let length = bytes.len();
        let data = Box::into_raw(bytes).cast::<u8>();
        unsafe {
            output.write(ProjectorAsset {
                data,
                length,
                mime: mime.into_raw(),
            });
        }
        true
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn sdb_projector_asset_free(asset: ProjectorAsset) {
        if !asset.data.is_null() {
            let slice = std::ptr::slice_from_raw_parts_mut(asset.data, asset.length);
            drop(unsafe { Box::from_raw(slice) });
        }
        if !asset.mime.is_null() {
            drop(unsafe { CString::from_raw(asset.mime) });
        }
    }

    fn dispatch_command(envelope: CommandEnvelope) -> Result<(CommandResult, Value), String> {
        let app = APP_HANDLE
            .get()
            .ok_or_else(|| "native app is not ready".to_owned())?;
        let state = app
            .try_state::<SharedNativeState>()
            .ok_or_else(|| "native runtime is not ready".to_owned())?;
        let (result, public, payload) = {
            let mut state = state.lock().map_err(|error| error.to_string())?;
            let allowed = runtime_v2_projector_report_allowed(&envelope.command)
                || (cfg!(debug_assertions)
                    && runtime_v2_projector_test_event_allowed(&envelope.command));
            if !allowed {
                return Err("external projector command is not allowed".into());
            }
            state.require_controller()?;
            let result = state
                .runtime
                .dispatch_envelope(envelope)
                .map_err(|error| error.message)?;
            (result, state.public(), payload(&state))
        };
        publish_public_state(app, &public);
        Ok((result, payload))
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn sdb_projector_command(command: *const c_char) -> *mut c_char {
        let response = (|| -> Result<Value, String> {
            if command.is_null() {
                return Err("missing external projector command".into());
            }
            let command = (unsafe { CStr::from_ptr(command) })
                .to_str()
                .map_err(|_| "external projector command is not UTF-8".to_owned())?;
            if command.len() > 64 * 1_024 {
                return Err("external projector command is too large".into());
            }
            let envelope: CommandEnvelope =
                serde_json::from_str(command).map_err(|error| error.to_string())?;
            let (result, payload) = dispatch_command(envelope)?;
            Ok(json!({"ok": true, "result": result, "payload": payload}))
        })()
        .unwrap_or_else(|error| json!({"ok": false, "error": error}));
        CString::new(response.to_string()).map_or(std::ptr::null_mut(), CString::into_raw)
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn sdb_projector_string_free(value: *mut c_char) {
        if !value.is_null() {
            drop(unsafe { CString::from_raw(value) });
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
        APP_HANDLE, COMPANION_PORT, CommandEnvelope, CommandResult, NativeState, PublicState,
        SharedNativeState, TlsIdentity, apple_bonjour, now_ms, publish_public_state,
        runtime_v2_projector_report_allowed,
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
            .route("/api/v2/companion/runtime/reports", post(report))
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

    async fn report(
        State(service): State<ServiceState>,
        headers: HeaderMap,
        Json(envelope): Json<CommandEnvelope>,
    ) -> Result<Json<CommandResult>, TransportError> {
        reject_browser_origin(&headers)?;
        let token = bearer_token(&headers)?;
        if !runtime_v2_projector_report_allowed(&envelope.command) {
            return Err(TransportError::forbidden("Companion report is not allowed"));
        }
        let (result, public) = {
            let mut state = service.native.lock().map_err(|_| TransportError::internal())?;
            authenticate(&state, token)?;
            state.require_controller().map_err(|_| TransportError::forbidden("host unavailable"))?;
            let result = state.runtime.dispatch_envelope(envelope)
                .map_err(|_| TransportError::bad_request("Companion report was rejected"))?;
            (result, state.public())
        };
        if let Some(app) = APP_HANDLE.get() {
            publish_public_state(app, &public);
        } else if let Ok(state) = service.native.lock() {
            let _ = state.companion_states.send(public);
        }
        Ok(Json(result))
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
        let mut heartbeat = tokio::time::interval_at(
            tokio::time::Instant::now() + Duration::from_secs(10),
            Duration::from_secs(10),
        );
        let mut awaiting_pong = false;
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
                incoming = socket.recv() => {
                    match incoming {
                        Some(Ok(Message::Pong(_))) => awaiting_pong = false,
                        Some(Ok(Message::Ping(payload))) => {
                            if socket.send(Message::Pong(payload)).await.is_err() {
                                break;
                            }
                        }
                        Some(Ok(Message::Close(_))) | Some(Err(_)) | None => break,
                        Some(Ok(Message::Text(_) | Message::Binary(_))) => {}
                    }
                }
                _ = heartbeat.tick() => {
                    if awaiting_pong
                        || socket.send(Message::Ping(now_ms().to_be_bytes().to_vec().into())).await.is_err()
                    {
                        break;
                    }
                    awaiting_pong = true;
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
        use crate::{APPLE_NETWORK_TEST_LOCK, CompanionIdentity};
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

            let sound_report = CommandEnvelope {
                protocol_version: sdb_contracts::PROTOCOL_VERSION,
                command_id: "sound-ready".into(),
                runtime_instance_id: frame.runtime_instance_id.clone(),
                expected_revision: Some(frame.revision),
                command: crate::RuntimeCommand::ReportSoundStatus {
                    status: sdb_contracts::SoundStatus::Ready,
                },
            };
            let response = app
                .clone()
                .oneshot(
                    Request::post("/api/v2/companion/runtime/reports")
                        .header(header::CONTENT_TYPE, "application/json")
                        .body(Body::from(serde_json::to_vec(&sound_report).expect("report")))
                        .expect("request"),
                )
                .await
                .expect("unauthenticated report response");
            assert_eq!(response.status(), StatusCode::FORBIDDEN);

            let response = app
                .clone()
                .oneshot(
                    Request::post("/api/v2/companion/runtime/reports")
                        .header(header::AUTHORIZATION, format!("Bearer {}", grant.token))
                        .header(header::CONTENT_TYPE, "application/json")
                        .body(Body::from(serde_json::to_vec(&sound_report).expect("report")))
                        .expect("request"),
                )
                .await
                .expect("authenticated report response");
            assert_eq!(response.status(), StatusCode::OK);

            let forbidden_report = CommandEnvelope {
                protocol_version: sdb_contracts::PROTOCOL_VERSION,
                command_id: "forbidden-reset".into(),
                runtime_instance_id: frame.runtime_instance_id.clone(),
                expected_revision: Some(frame.revision + 1),
                command: crate::RuntimeCommand::ResetCalibration,
            };
            let response = app
                .clone()
                .oneshot(
                    Request::post("/api/v2/companion/runtime/reports")
                        .header(header::AUTHORIZATION, format!("Bearer {}", grant.token))
                        .header(header::CONTENT_TYPE, "application/json")
                        .body(Body::from(
                            serde_json::to_vec(&forbidden_report).expect("forbidden report"),
                        ))
                        .expect("request"),
                )
                .await
                .expect("forbidden report response");
            assert_eq!(response.status(), StatusCode::FORBIDDEN);

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
            let _network_guard = APPLE_NETWORK_TEST_LOCK.lock().await;
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
mod native_companion_client {
    use super::{
        CommandEnvelope, CommandResult, CompanionFrame, DiscoveredCompanionHost, PairingGrant,
        PairingRequest, ReplicaCursor, ReplicaDecision, certificate_sha256, now_ms,
    };
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    use futures_util::{SinkExt, StreamExt};
    use serde::{Deserialize, Serialize};
    use std::sync::{Arc, Mutex as StdMutex};
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpStream,
        time::{Duration, timeout},
    };
    use tokio_rustls::{
        TlsConnector,
        rustls::{
            self, ClientConfig, DigitallySignedStruct, RootCertStore, SignatureScheme,
            client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier},
            pki_types::{CertificateDer, ServerName, UnixTime},
        },
    };
    use tokio_tungstenite::{
        Connector, client_async_tls_with_config,
        tungstenite::{
            Message,
            client::IntoClientRequest,
            http::{HeaderValue, header::AUTHORIZATION},
            protocol::WebSocketConfig,
        },
    };
    use zeroize::{Zeroize, Zeroizing};

    pub(super) const CLIENT_GRANT_KEY: &str = "companion.client.grant.v1";
    const PROBE_LIFETIME_MS: u64 = 5 * 60 * 1_000;
    const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
    const MAX_HTTP_RESPONSE_BYTES: u64 = 128 * 1_024;
    const MAX_WEBSOCKET_MESSAGE_BYTES: usize = 256 * 1_024;

    #[derive(Clone)]
    pub(super) struct ProbedTarget {
        pub host: DiscoveredCompanionHost,
        pub certificate_der: Vec<u8>,
        pub certificate_sha256: String,
        pub expires_at_ms: u64,
    }

    #[derive(Clone)]
    pub(super) struct ActiveGrant {
        pub host_id: String,
        pub certificate_der: Vec<u8>,
        pub token: Zeroizing<String>,
    }

    #[derive(Debug, PartialEq, Eq)]
    pub(super) enum SessionError {
        Authorization,
        Retry(String),
    }

    impl ActiveGrant {
        pub(super) fn is_usable(&self) -> bool {
            !self.host_id.is_empty() && !self.certificate_der.is_empty() && !self.token.is_empty()
        }
    }

    #[derive(Deserialize, Serialize)]
    pub(super) struct StoredGrant {
        pub protocol_version: u16,
        pub host_id: String,
        pub certificate_der_base64: String,
        pub device_id: String,
        pub token: String,
    }

    pub(super) fn decode_stored_grant(bytes: &[u8]) -> Result<ActiveGrant, String> {
        let mut stored: StoredGrant =
            serde_json::from_slice(bytes).map_err(|_| "stored Companion grant is malformed")?;
        if stored.protocol_version != sdb_companion::COMPANION_PROTOCOL_VERSION
            || uuid::Uuid::parse_str(&stored.host_id).is_err()
            || uuid::Uuid::parse_str(&stored.device_id).is_err()
            || stored.token.len() != 43
            || !stored
                .token
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
        {
            stored.token.zeroize();
            return Err("stored Companion grant is invalid".into());
        }
        let certificate_der = match STANDARD.decode(&stored.certificate_der_base64) {
            Ok(certificate) => certificate,
            Err(_) => {
                stored.token.zeroize();
                return Err("stored Companion certificate is malformed".into());
            }
        };
        if certificate_der.is_empty() || certificate_der.len() > 64 * 1_024 {
            stored.token.zeroize();
            return Err("stored Companion certificate is invalid".into());
        }
        Ok(ActiveGrant {
            host_id: stored.host_id,
            certificate_der,
            token: Zeroizing::new(stored.token),
        })
    }

    #[derive(Debug)]
    struct ProbeVerifier {
        certificate: Arc<StdMutex<Option<Vec<u8>>>>,
    }

    impl ServerCertVerifier for ProbeVerifier {
        fn verify_server_cert(
            &self,
            end_entity: &CertificateDer<'_>,
            _intermediates: &[CertificateDer<'_>],
            _server_name: &ServerName<'_>,
            _ocsp_response: &[u8],
            _now: UnixTime,
        ) -> Result<ServerCertVerified, rustls::Error> {
            if let Ok(mut certificate) = self.certificate.lock() {
                *certificate = Some(end_entity.as_ref().to_vec());
            }
            Ok(ServerCertVerified::assertion())
        }

        fn verify_tls12_signature(
            &self,
            _message: &[u8],
            _cert: &CertificateDer<'_>,
            _dss: &DigitallySignedStruct,
        ) -> Result<HandshakeSignatureValid, rustls::Error> {
            Ok(HandshakeSignatureValid::assertion())
        }

        fn verify_tls13_signature(
            &self,
            _message: &[u8],
            _cert: &CertificateDer<'_>,
            _dss: &DigitallySignedStruct,
        ) -> Result<HandshakeSignatureValid, rustls::Error> {
            Ok(HandshakeSignatureValid::assertion())
        }

        fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
            vec![SignatureScheme::ECDSA_NISTP256_SHA256]
        }
    }

    pub(super) async fn probe(host: DiscoveredCompanionHost) -> Result<ProbedTarget, String> {
        let certificate = Arc::new(StdMutex::new(None));
        let provider = Arc::new(rustls::crypto::ring::default_provider());
        let mut config = ClientConfig::builder_with_provider(provider)
            .with_safe_default_protocol_versions()
            .map_err(|_| "Companion TLS protocol setup failed")?
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(ProbeVerifier {
                certificate: certificate.clone(),
            }))
            .with_no_client_auth();
        config.alpn_protocols = vec![b"http/1.1".to_vec()];
        let tls = connect(&host, Arc::new(config)).await?;
        drop(tls);
        let certificate_der = certificate
            .lock()
            .map_err(|_| "Companion TLS probe state failed")?
            .take()
            .ok_or_else(|| "Companion did not provide a TLS certificate".to_owned())?;
        let fingerprint = certificate_sha256(&certificate_der);
        Ok(ProbedTarget {
            host,
            certificate_der,
            certificate_sha256: fingerprint,
            expires_at_ms: now_ms().saturating_add(PROBE_LIFETIME_MS),
        })
    }

    pub(super) async fn pair(
        target: &ProbedTarget,
        request: &PairingRequest,
    ) -> Result<PairingGrant, String> {
        if now_ms() >= target.expires_at_ms {
            return Err("TLS comparison expired; select the Controller again".into());
        }
        let body = serde_json::to_vec(request).map_err(|_| "pairing request encoding failed")?;
        let mut response = post_json(
            &target.host,
            &target.certificate_der,
            "/api/v2/companion/pairing",
            &body,
        )
        .await?;
        let grant =
            serde_json::from_slice(&response).map_err(|_| "invalid pairing response".to_owned());
        response.zeroize();
        grant
    }

    async fn post_json(
        host: &DiscoveredCompanionHost,
        certificate_der: &[u8],
        path: &str,
        body: &[u8],
    ) -> Result<Vec<u8>, String> {
        let mut tls = connect(host, verified_client_config(certificate_der)?).await?;
        let authority = format!("{}.local", host.host_id);
        let header = format!(
            "POST {path} HTTP/1.1\r\nHost: {authority}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        tls.write_all(header.as_bytes())
            .await
            .map_err(|_| "Companion request failed")?;
        tls.write_all(body)
            .await
            .map_err(|_| "Companion request failed")?;
        let mut response = Vec::new();
        tls.take(MAX_HTTP_RESPONSE_BYTES + 1)
            .read_to_end(&mut response)
            .await
            .map_err(|_| "Companion response failed")?;
        if response.len() as u64 > MAX_HTTP_RESPONSE_BYTES {
            return Err("Companion response exceeds size limit".into());
        }
        let parsed = parse_http_response(&response);
        response.zeroize();
        let (status, body) = parsed?;
        if status != 200 {
            return Err("pairing was rejected by the Controller".into());
        }
        Ok(body)
    }

    pub(super) async fn report(
        host: &DiscoveredCompanionHost,
        grant: &ActiveGrant,
        envelope: &CommandEnvelope,
    ) -> Result<CommandResult, String> {
        if host.host_id != grant.host_id
            || host.protocol_version != sdb_companion::COMPANION_PROTOCOL_VERSION
            || !host.tls
        {
            return Err("discovered Controller identity changed".into());
        }
        let body = serde_json::to_vec(envelope).map_err(|_| "Companion report is invalid")?;
        if body.len() > 64 * 1_024 {
            return Err("Companion report exceeds size limit".into());
        }
        let mut response = post_authenticated_json(
            host,
            grant,
            "/api/v2/companion/runtime/reports",
            &body,
        )
        .await?;
        let result = serde_json::from_slice(&response)
            .map_err(|_| "invalid Companion report response".to_owned());
        response.zeroize();
        result
    }

    async fn post_authenticated_json(
        host: &DiscoveredCompanionHost,
        grant: &ActiveGrant,
        path: &str,
        body: &[u8],
    ) -> Result<Vec<u8>, String> {
        let mut tls = connect(host, verified_client_config(&grant.certificate_der)?).await?;
        let authority = format!("{}.local", host.host_id);
        let mut header = Zeroizing::new(format!(
            "POST {path} HTTP/1.1\r\nHost: {authority}\r\nAuthorization: Bearer {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            grant.token.as_str(), body.len()
        ));
        tls.write_all(header.as_bytes())
            .await
            .map_err(|_| "Companion report failed")?;
        header.zeroize();
        tls.write_all(body)
            .await
            .map_err(|_| "Companion report failed")?;
        let mut response = Vec::new();
        tls.take(MAX_HTTP_RESPONSE_BYTES + 1)
            .read_to_end(&mut response)
            .await
            .map_err(|_| "Companion report response failed")?;
        if response.len() as u64 > MAX_HTTP_RESPONSE_BYTES {
            response.zeroize();
            return Err("Companion report response exceeds size limit".into());
        }
        let parsed = parse_http_response(&response);
        response.zeroize();
        let (status, mut body) = parsed?;
        if status != 200 {
            body.zeroize();
            return Err("Companion report was rejected by the Controller".into());
        }
        Ok(body)
    }

    pub(super) async fn replicate<F>(
        host: &DiscoveredCompanionHost,
        grant: &ActiveGrant,
        mut apply: F,
    ) -> Result<(), SessionError>
    where
        F: FnMut(super::PublicState) + Send,
    {
        if host.host_id != grant.host_id
            || host.protocol_version != sdb_companion::COMPANION_PROTOCOL_VERSION
            || !host.tls
        {
            return Err(SessionError::Retry(
                "discovered Controller identity changed".into(),
            ));
        }
        let bootstrap = get_bootstrap(host, grant).await?;
        let mut cursor = ReplicaCursor::default();
        apply_frame(&mut cursor, bootstrap, &mut apply)?;

        let tcp = connect_tcp(host).await.map_err(SessionError::Retry)?;
        let authority = format!("{}.local", host.host_id);
        let url = format!(
            "wss://{authority}:{}/api/v2/companion/runtime/events",
            host.port
        );
        let mut request = url
            .into_client_request()
            .map_err(|_| SessionError::Retry("invalid Companion WebSocket request".into()))?;
        let mut authorization = Zeroizing::new(format!("Bearer {}", grant.token.as_str()));
        let header = HeaderValue::from_str(&authorization)
            .map_err(|_| SessionError::Retry("invalid Companion authorization".into()))?;
        authorization.zeroize();
        request.headers_mut().insert(AUTHORIZATION, header);
        let config = WebSocketConfig::default()
            .read_buffer_size(16 * 1_024)
            .write_buffer_size(4 * 1_024)
            .max_write_buffer_size(32 * 1_024)
            .max_message_size(Some(MAX_WEBSOCKET_MESSAGE_BYTES))
            .max_frame_size(Some(MAX_WEBSOCKET_MESSAGE_BYTES));
        let connector = Connector::Rustls(
            verified_client_config(&grant.certificate_der).map_err(SessionError::Retry)?,
        );
        let (mut socket, _) = timeout(
            CONNECT_TIMEOUT,
            client_async_tls_with_config(request, tcp, Some(config), Some(connector)),
        )
        .await
        .map_err(|_| SessionError::Retry("Companion WebSocket timed out".into()))?
        .map_err(|error| match error {
            tokio_tungstenite::tungstenite::Error::Http(response)
                if matches!(response.status().as_u16(), 401 | 403) =>
            {
                SessionError::Authorization
            }
            _ => SessionError::Retry("Companion WebSocket failed".into()),
        })?;

        let mut first = true;
        while let Some(message) = socket.next().await {
            match message.map_err(|_| SessionError::Retry("Companion stream closed".into()))? {
                Message::Text(text) => {
                    let frame: CompanionFrame = serde_json::from_str(&text).map_err(|_| {
                        SessionError::Retry("invalid Companion stream frame".into())
                    })?;
                    if first && frame.kind != sdb_companion::CompanionFrameKind::Snapshot {
                        return Err(SessionError::Retry(
                            "Companion stream did not begin with a snapshot".into(),
                        ));
                    }
                    first = false;
                    apply_frame(&mut cursor, frame, &mut apply)?;
                }
                Message::Ping(payload) => socket
                    .send(Message::Pong(payload))
                    .await
                    .map_err(|_| SessionError::Retry("Companion stream closed".into()))?,
                Message::Close(_) => {
                    return Err(SessionError::Retry("Companion stream closed".into()));
                }
                Message::Binary(_) | Message::Pong(_) | Message::Frame(_) => {}
            }
        }
        Err(SessionError::Retry("Companion stream closed".into()))
    }

    fn apply_frame<F>(
        cursor: &mut ReplicaCursor,
        frame: CompanionFrame,
        apply: &mut F,
    ) -> Result<(), SessionError>
    where
        F: FnMut(super::PublicState),
    {
        let previous = (
            cursor.runtime_instance_id().map(str::to_owned),
            cursor.revision(),
        );
        let decision = cursor
            .accept(&frame)
            .map_err(|error| SessionError::Retry(error.to_string()))?;
        if decision == ReplicaDecision::Duplicate
            || previous
                == (
                    Some(frame.runtime_instance_id.clone()),
                    Some(frame.revision),
                )
        {
            return Ok(());
        }
        let state: super::PublicState = serde_json::from_value(frame.payload)
            .map_err(|_| SessionError::Retry("invalid Companion state payload".into()))?;
        if state.runtime_instance_id != frame.runtime_instance_id
            || state.revision != frame.revision
            || state.app_role != super::NativeAppRole::Controller
        {
            return Err(SessionError::Retry(
                "Companion state metadata does not match its frame".into(),
            ));
        }
        apply(state);
        Ok(())
    }

    async fn get_bootstrap(
        host: &DiscoveredCompanionHost,
        grant: &ActiveGrant,
    ) -> Result<CompanionFrame, SessionError> {
        let config = verified_client_config(&grant.certificate_der).map_err(SessionError::Retry)?;
        let mut tls = connect(host, config).await.map_err(SessionError::Retry)?;
        let authority = format!("{}.local", host.host_id);
        let mut request = Zeroizing::new(format!(
            "GET /api/v2/companion/runtime/bootstrap HTTP/1.1\r\nHost: {authority}\r\nAuthorization: Bearer {}\r\nConnection: close\r\n\r\n",
            grant.token.as_str()
        ));
        tls.write_all(request.as_bytes())
            .await
            .map_err(|_| SessionError::Retry("Companion bootstrap request failed".into()))?;
        request.zeroize();
        let mut response = Vec::new();
        tls.take(MAX_HTTP_RESPONSE_BYTES + 1)
            .read_to_end(&mut response)
            .await
            .map_err(|_| SessionError::Retry("Companion bootstrap response failed".into()))?;
        if response.len() as u64 > MAX_HTTP_RESPONSE_BYTES {
            response.zeroize();
            return Err(SessionError::Retry(
                "Companion bootstrap response exceeds size limit".into(),
            ));
        }
        let parsed = parse_http_response(&response).map_err(SessionError::Retry);
        response.zeroize();
        let (status, mut body) = parsed?;
        if matches!(status, 401 | 403) {
            body.zeroize();
            return Err(SessionError::Authorization);
        }
        if status != 200 {
            body.zeroize();
            return Err(SessionError::Retry("Companion bootstrap rejected".into()));
        }
        let frame = serde_json::from_slice(&body)
            .map_err(|_| SessionError::Retry("invalid Companion bootstrap frame".into()));
        body.zeroize();
        frame
    }

    fn verified_client_config(certificate_der: &[u8]) -> Result<Arc<ClientConfig>, String> {
        let mut roots = RootCertStore::empty();
        roots
            .add(CertificateDer::from(certificate_der.to_vec()))
            .map_err(|_| "invalid pinned Companion certificate")?;
        let provider = Arc::new(rustls::crypto::ring::default_provider());
        let mut config = ClientConfig::builder_with_provider(provider)
            .with_safe_default_protocol_versions()
            .map_err(|_| "Companion TLS protocol setup failed")?
            .with_root_certificates(roots)
            .with_no_client_auth();
        config.alpn_protocols = vec![b"http/1.1".to_vec()];
        Ok(Arc::new(config))
    }

    async fn connect(
        host: &DiscoveredCompanionHost,
        config: Arc<ClientConfig>,
    ) -> Result<tokio_rustls::client::TlsStream<TcpStream>, String> {
        let tcp = connect_tcp(host).await?;
        let server_name = ServerName::try_from(format!("{}.local", host.host_id))
            .map_err(|_| "invalid Companion TLS server name")?;
        timeout(
            CONNECT_TIMEOUT,
            TlsConnector::from(config).connect(server_name, tcp),
        )
        .await
        .map_err(|_| "Companion TLS handshake timed out")?
        .map_err(|_| "Companion TLS handshake failed".into())
    }

    async fn connect_tcp(host: &DiscoveredCompanionHost) -> Result<TcpStream, String> {
        timeout(
            CONNECT_TIMEOUT,
            TcpStream::connect((host.host_name.as_str(), host.port)),
        )
        .await
        .map_err(|_| "Companion connection timed out")?
        .map_err(|_| "Companion connection failed".into())
    }

    fn parse_http_response(response: &[u8]) -> Result<(u16, Vec<u8>), String> {
        let header_end = response
            .windows(4)
            .position(|window| window == b"\r\n\r\n")
            .map(|index| index + 4)
            .ok_or_else(|| "invalid Companion HTTP response".to_owned())?;
        let status_line_end = response
            .windows(2)
            .position(|window| window == b"\r\n")
            .ok_or_else(|| "invalid Companion HTTP response".to_owned())?;
        let status_line = std::str::from_utf8(&response[..status_line_end])
            .map_err(|_| "invalid Companion HTTP response")?;
        let status = status_line
            .split_ascii_whitespace()
            .nth(1)
            .and_then(|value| value.parse::<u16>().ok())
            .ok_or_else(|| "invalid Companion HTTP status".to_owned())?;
        Ok((status, response[header_end..].to_vec()))
    }

    #[cfg(test)]
    mod tests {
        use super::{StoredGrant, decode_stored_grant, parse_http_response};
        use base64::{Engine as _, engine::general_purpose::STANDARD};

        #[test]
        fn http_response_parser_accepts_only_complete_success_responses() {
            assert_eq!(
                parse_http_response(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\n{}")
                    .expect("response"),
                (200, b"{}".to_vec())
            );
            assert_eq!(
                parse_http_response(b"HTTP/1.1 403 Forbidden\r\nContent-Length: 0\r\n\r\n")
                    .expect("forbidden response"),
                (403, Vec::new())
            );
            assert!(parse_http_response(b"not-http").is_err());
        }

        #[test]
        fn stored_grant_requires_versioned_uuid_metadata_and_a_full_token() {
            let stored = StoredGrant {
                protocol_version: sdb_companion::COMPANION_PROTOCOL_VERSION,
                host_id: "991708fa-c4e7-419f-ad1d-c44f01891b03".into(),
                certificate_der_base64: STANDARD.encode([1, 2, 3]),
                device_id: "1645ae13-53d4-4ca4-8465-803f3adb3387".into(),
                token: "a".repeat(43),
            };
            let encoded = serde_json::to_vec(&stored).expect("encode");
            assert!(decode_stored_grant(&encoded).expect("grant").is_usable());

            let mut invalid = stored;
            invalid.token = "short".into();
            assert!(
                decode_stored_grant(&serde_json::to_vec(&invalid).expect("encode invalid"))
                    .is_err()
            );
        }
    }
}

#[cfg(any(target_os = "ios", target_os = "macos"))]
struct NativeCompanionService {
    transport: Option<Arc<native_companion_transport::NativeCompanionTransport>>,
    probed_target: AsyncMutex<Option<native_companion_client::ProbedTarget>>,
    active_grant: AsyncMutex<Option<native_companion_client::ActiveGrant>>,
    client_status: Arc<Mutex<Option<CompanionClientView>>>,
    client_frame: Arc<Mutex<Option<PublicState>>>,
    client_task: AsyncMutex<Option<JoinHandle<()>>>,
}

#[cfg(any(target_os = "ios", target_os = "macos"))]
fn companion_client_view(
    grant: &native_companion_client::ActiveGrant,
    service_name: impl Into<String>,
    phase: CompanionClientPhase,
) -> CompanionClientView {
    CompanionClientView {
        host_id: grant.host_id.clone(),
        service_name: service_name.into(),
        paired: phase != CompanionClientPhase::PairingRequired,
        phase,
        runtime_instance_id: None,
        revision: None,
    }
}

#[cfg(any(target_os = "ios", target_os = "macos"))]
fn publish_companion_client_status(
    app: &tauri::AppHandle,
    status: &Arc<Mutex<Option<CompanionClientView>>>,
    view: CompanionClientView,
) {
    if let Ok(mut current) = status.lock() {
        *current = Some(view.clone());
    }
    let _ = app.emit("companion-projector-status", view);
}

#[cfg(any(target_os = "ios", target_os = "macos"))]
fn companion_v2_envelope(state: &PublicState) -> Envelope<RuntimePublicSnapshot> {
    Envelope::new(
        state.runtime_instance_id.clone(),
        Uuid::new_v4().to_string(),
        state.revision,
        MessageKind::State,
        state.runtime.clone(),
    )
}

#[cfg(any(target_os = "ios", target_os = "macos"))]
fn clear_companion_client_frame(
    app: &tauri::AppHandle,
    frame: &Arc<Mutex<Option<PublicState>>>,
) {
    if let Ok(mut current) = frame.lock() {
        *current = None;
    }
    let _ = app.emit("companion-projector-v2-disconnected", ());
}

#[cfg(any(target_os = "ios", target_os = "macos"))]
async fn stop_companion_client(service: &NativeCompanionService) {
    if let Some(task) = service.client_task.lock().await.take() {
        task.abort();
    }
}

#[cfg(any(target_os = "ios", target_os = "macos"))]
async fn restart_companion_client(app: &tauri::AppHandle, service: &NativeCompanionService) {
    stop_companion_client(service).await;
    let grant = service.active_grant.lock().await.clone();
    let Some(grant) = grant.filter(native_companion_client::ActiveGrant::is_usable) else {
        if let Ok(mut status) = service.client_status.lock() {
            *status = None;
        }
        return;
    };
    let app = app.clone();
    let status = service.client_status.clone();
    let frame = service.client_frame.clone();
    let task = tauri::async_runtime::spawn(async move {
        companion_client_loop(app, status, frame, grant).await;
    });
    *service.client_task.lock().await = Some(task);
}

#[cfg(any(target_os = "ios", target_os = "macos"))]
async fn companion_client_loop(
    app: tauri::AppHandle,
    status: Arc<Mutex<Option<CompanionClientView>>>,
    frame: Arc<Mutex<Option<PublicState>>>,
    grant: native_companion_client::ActiveGrant,
) {
    let mut retry_seconds = 1_u64;
    loop {
        let host = apple_bonjour::browser_snapshot()
            .ok()
            .and_then(|hosts| hosts.into_iter().find(|host| host.host_id == grant.host_id));
        let Some(host) = host else {
            clear_companion_client_frame(&app, &frame);
            publish_companion_client_status(
                &app,
                &status,
                companion_client_view(
                    &grant,
                    "Gekoppelter Controller",
                    CompanionClientPhase::Discovering,
                ),
            );
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            continue;
        };
        publish_companion_client_status(
            &app,
            &status,
            companion_client_view(&grant, &host.service_name, CompanionClientPhase::Connecting),
        );
        clear_companion_client_frame(&app, &frame);
        let frame_app = app.clone();
        let frame_status = status.clone();
        let frame_cache = frame.clone();
        let frame_grant = grant.clone();
        let service_name = host.service_name.clone();
        let result = native_companion_client::replicate(&host, &grant, move |state| {
            let envelope = companion_v2_envelope(&state);
            let mut view =
                companion_client_view(&frame_grant, &service_name, CompanionClientPhase::Connected);
            view.runtime_instance_id = Some(state.runtime_instance_id.clone());
            view.revision = Some(state.revision);
            if let Ok(mut current) = frame_cache.lock() {
                *current = Some(state.clone());
            }
            publish_companion_client_status(&frame_app, &frame_status, view);
            let _ = frame_app.emit("companion-projector-frame", state);
            let _ = frame_app.emit("companion-projector-v2-state", envelope);
        })
        .await;
        match result {
            Err(native_companion_client::SessionError::Authorization) => {
                clear_companion_client_frame(&app, &frame);
                publish_companion_client_status(
                    &app,
                    &status,
                    companion_client_view(
                        &grant,
                        &host.service_name,
                        CompanionClientPhase::PairingRequired,
                    ),
                );
                return;
            }
            Err(native_companion_client::SessionError::Retry(error)) => {
                clear_companion_client_frame(&app, &frame);
                eprintln!("Companion projector reconnecting: {error}");
                publish_companion_client_status(
                    &app,
                    &status,
                    companion_client_view(
                        &grant,
                        &host.service_name,
                        CompanionClientPhase::Reconnecting,
                    ),
                );
            }
            Ok(()) => unreachable!("Companion replication runs until disconnect"),
        }
        tokio::time::sleep(std::time::Duration::from_secs(retry_seconds)).await;
        retry_seconds = (retry_seconds * 2).min(5);
    }
}

#[cfg(any(target_os = "ios", target_os = "macos"))]
#[allow(unsafe_code)]
mod apple_board {
    use super::{
        APP_HANDLE, BoardFailureCode, BoardPhase, BoardStatus, NativeAppRole, SharedNativeState,
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
            10 => BoardPhase::Disabled,
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
            if state.app_role == NativeAppRole::CompanionProjector {
                state.board_status = BoardStatus::disabled();
            } else {
                let phase = phase(phase_value);
                state.board_status = BoardStatus {
                    enabled: phase != BoardPhase::Disabled,
                    phase,
                    failure_code: failure(failure_value),
                    detail,
                    connection_id,
                };
            }
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
            if state.app_role == NativeAppRole::CompanionProjector {
                return;
            }
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

#[cfg(any(target_os = "ios", target_os = "macos"))]
#[allow(unsafe_code)]
mod apple_board_host {
    #[cfg(target_os = "ios")]
    use std::ffi::{CStr, c_void};

    type BoardHostAction = unsafe extern "C" fn();

    #[cfg(target_os = "macos")]
    #[link(name = "sdb_apple_board_transport", kind = "static")]
    unsafe extern "C" {
        fn sdb_install_board_transport_host();
        fn sdb_stop_board_transport_host();
    }

    #[cfg(target_os = "ios")]
    fn lookup(symbol: &CStr) -> Option<BoardHostAction> {
        let address = unsafe { libc::dlsym(libc::RTLD_DEFAULT, symbol.as_ptr()) };
        (!address.is_null())
            .then(|| unsafe { std::mem::transmute::<*mut c_void, BoardHostAction>(address) })
    }

    #[cfg(target_os = "macos")]
    const fn install_function() -> Option<BoardHostAction> {
        Some(sdb_install_board_transport_host)
    }

    #[cfg(target_os = "ios")]
    fn install_function() -> Option<BoardHostAction> {
        lookup(c"sdb_install_board_transport_host")
    }

    #[cfg(target_os = "macos")]
    const fn stop_function() -> Option<BoardHostAction> {
        Some(sdb_stop_board_transport_host)
    }

    #[cfg(target_os = "ios")]
    fn stop_function() -> Option<BoardHostAction> {
        lookup(c"sdb_stop_board_transport_host")
    }

    pub fn install() {
        if let Some(install) = install_function() {
            unsafe { install() };
        }
    }

    pub fn stop() {
        if let Some(stop) = stop_function() {
            unsafe { stop() };
        }
    }
}

#[cfg(target_os = "ios")]
fn publish_to_external_projector(state: &NativeState) {
    ios_display::publish(state);
}

#[cfg(not(target_os = "ios"))]
fn publish_to_external_projector(_state: &NativeState) {}

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

fn runtime_v2_envelope(state: &NativeState) -> Envelope<RuntimePublicSnapshot> {
    Envelope::new(
        state.runtime.instance_id(),
        Uuid::new_v4().to_string(),
        state.runtime.snapshot().revision,
        MessageKind::State,
        state.runtime.public_snapshot(),
    )
}

#[tauri::command]
fn runtime_v2_bootstrap(
    state: State<'_, SharedNativeState>,
) -> Result<Envelope<RuntimePublicSnapshot>, String> {
    state
        .lock()
        .map(|state| runtime_v2_envelope(&state))
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn runtime_v2_snapshot(
    state: State<'_, SharedNativeState>,
) -> Result<Envelope<RuntimePublicSnapshot>, String> {
    runtime_v2_bootstrap(state)
}

#[tauri::command]
fn runtime_v2_query(
    state: State<'_, SharedNativeState>,
    path: String,
) -> Result<serde_json::Value, String> {
    let state = state.lock().map_err(|error| error.to_string())?;
    runtime_v2_query_value(&state, &path)
}

fn runtime_v2_query_value(state: &NativeState, path: &str) -> Result<serde_json::Value, String> {
    if path.len() > 2_048 {
        return Err("native runtime query is too long".into());
    }
    let url = url::Url::parse(&format!("sdb://localhost{path}"))
        .map_err(|_| "invalid native runtime query".to_string())?;
    let route = url.path();
    if route == "/api/v2/modes" {
        return serde_json::to_value(registered_game_metadata()).map_err(|error| error.to_string());
    }
    let parameters = url.query_pairs().collect::<HashMap<_, _>>();
    let include_test = parameters.get("include_test").is_some_and(|value| value == "true");
    let repository = state.runtime.repository();
    let value = match route {
        "/api/v2/host" => serde_json::to_value(state.public()),
        "/api/v2/players" => {
            serde_json::to_value(repository.players().map_err(|error| error.to_string())?)
        }
        "/api/v2/statistics/players" => serde_json::to_value(
            repository
                .player_statistics_including_test(include_test)
                .map_err(|error| error.to_string())?,
        ),
        "/api/v2/statistics/modes" => serde_json::to_value(
            repository.mode_statistics(include_test).map_err(|error| error.to_string())?,
        ),
        "/api/v2/statistics/heatmap" => serde_json::to_value(
            repository.heatmap(
                parameters.get("player_id").map(AsRef::as_ref),
                parameters.get("session_id").map(AsRef::as_ref),
                parameters.get("game_type").map(AsRef::as_ref),
                include_test,
            ).map_err(|error| error.to_string())?,
        ),
        "/api/v2/history/sessions" => {
            let limit = parameters.get("limit").and_then(|value| value.parse().ok()).unwrap_or(50);
            serde_json::to_value(repository.sessions(limit).map_err(|error| error.to_string())?)
        }
        "/api/v2/data/export" => serde_json::to_value(
            repository.export_data().map_err(|error| error.to_string())?,
        ),
        _ if route.starts_with("/api/v2/history/sessions/") => {
            let id = route.trim_start_matches("/api/v2/history/sessions/");
            serde_json::to_value(repository.session_detail(id).map_err(|error| error.to_string())?
                .ok_or_else(|| "session not found".to_string())?)
        }
        _ if route.starts_with("/api/v2/history/games/") && route.ends_with("/replay") => {
            let id = route.trim_start_matches("/api/v2/history/games/")
                .trim_end_matches("/replay").trim_end_matches('/');
            serde_json::to_value(repository.game_replay(id).map_err(|error| error.to_string())?
                .ok_or_else(|| "game not found".to_string())?)
        }
        _ if route.starts_with("/api/v2/history/games/") => {
            let id = route.trim_start_matches("/api/v2/history/games/");
            serde_json::to_value(repository.game_detail(id).map_err(|error| error.to_string())?
                .ok_or_else(|| "game not found".to_string())?)
        }
        _ if route.starts_with("/api/v2/training/") && route.ends_with("/recommendations") => {
            let id = route.trim_start_matches("/api/v2/training/")
                .trim_end_matches("/recommendations").trim_end_matches('/');
            serde_json::to_value(repository.training_recommendations(id)
                .map_err(|error| error.to_string())?
                .ok_or_else(|| "player not found".to_string())?)
        }
        _ => return Err("unsupported native runtime query".into()),
    };
    value.map_err(|error| error.to_string())
}

#[tauri::command]
fn runtime_v2_dispatch(
    app: tauri::AppHandle,
    state: State<'_, SharedNativeState>,
    envelope: CommandEnvelope,
) -> Result<CommandResult, String> {
    let (result, public) = {
        let mut state = state.lock().map_err(|error| error.to_string())?;
        state.require_controller()?;
        let result = state
            .runtime
            .dispatch_envelope(envelope)
            .map_err(|error| error.message)?;
        (result, state.public())
    };
    publish_public_state(&app, &public);
    Ok(result)
}

#[tauri::command]
fn runtime_v2_report(
    app: tauri::AppHandle,
    state: State<'_, SharedNativeState>,
    envelope: CommandEnvelope,
) -> Result<CommandResult, String> {
    if !runtime_v2_projector_report_allowed(&envelope.command) {
        return Err("projector may only report geometry or sound status".into());
    }
    let (result, public) = {
        let mut state = state.lock().map_err(|error| error.to_string())?;
        state.require_controller()?;
        let result = state
            .runtime
            .dispatch_envelope(envelope)
            .map_err(|error| error.message)?;
        (result, state.public())
    };
    publish_public_state(&app, &public);
    Ok(result)
}

fn runtime_v2_projector_report_allowed(command: &RuntimeCommand) -> bool {
    matches!(
        command,
        RuntimeCommand::ReportProjectorGeometry { .. } | RuntimeCommand::ReportSoundStatus { .. }
    )
}

fn runtime_v2_projector_test_event_allowed(command: &RuntimeCommand) -> bool {
    matches!(
        command,
        RuntimeCommand::IngestDart {
            source: DartSource::ProjectorTest,
            ..
        }
    )
}

#[tauri::command]
fn runtime_v2_projector_test_event(
    app: tauri::AppHandle,
    state: State<'_, SharedNativeState>,
    envelope: CommandEnvelope,
) -> Result<CommandResult, String> {
    if !cfg!(debug_assertions) || !runtime_v2_projector_test_event_allowed(&envelope.command) {
        return Err("projector test events are disabled".into());
    }
    let (result, public) = {
        let mut state = state.lock().map_err(|error| error.to_string())?;
        state.require_controller()?;
        let result = state
            .runtime
            .dispatch_envelope(envelope)
            .map_err(|error| error.message)?;
        (result, state.public())
    };
    publish_public_state(&app, &public);
    Ok(result)
}

#[tauri::command]
fn companion_pairing_open(state: State<'_, SharedNativeState>) -> Result<PairingBootstrap, String> {
    let mut state = state.lock().map_err(|error| error.to_string())?;
    state.require_controller()?;
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
    let state = state.lock().map_err(|error| error.to_string())?;
    state.require_controller()?;
    Ok(state.companion_devices())
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
fn companion_discovery_start(state: State<'_, SharedNativeState>) -> Result<(), String> {
    let state = state.lock().map_err(|error| error.to_string())?;
    if state.app_role != NativeAppRole::CompanionProjector {
        return Err("select Companion projector mode before discovery".into());
    }
    drop(state);
    apple_bonjour::browser_start()
}

#[tauri::command]
#[cfg(not(any(target_os = "ios", target_os = "macos")))]
fn companion_discovery_start(_state: State<'_, SharedNativeState>) -> Result<(), String> {
    Err("native Companion discovery is not available on this platform".into())
}

#[tauri::command]
#[cfg(any(target_os = "ios", target_os = "macos"))]
fn companion_discovered_hosts(
    state: State<'_, SharedNativeState>,
) -> Result<Vec<DiscoveredCompanionHost>, String> {
    let state = state.lock().map_err(|error| error.to_string())?;
    if state.app_role != NativeAppRole::CompanionProjector {
        return Err("Companion discovery is not active in Controller mode".into());
    }
    drop(state);
    apple_bonjour::browser_snapshot()
}

#[tauri::command]
#[cfg(not(any(target_os = "ios", target_os = "macos")))]
fn companion_discovered_hosts(
    _state: State<'_, SharedNativeState>,
) -> Result<Vec<DiscoveredCompanionHost>, String> {
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
async fn companion_pairing_prepare(
    state: State<'_, SharedNativeState>,
    service: State<'_, NativeCompanionService>,
    host_id: String,
) -> Result<CompanionPairingTargetView, String> {
    {
        let state = state.lock().map_err(|error| error.to_string())?;
        if state.app_role != NativeAppRole::CompanionProjector {
            return Err("select Companion projector mode before pairing".into());
        }
    }
    let host = apple_bonjour::browser_snapshot()?
        .into_iter()
        .find(|host| host.host_id == host_id)
        .ok_or_else(|| "Controller is no longer available".to_owned())?;
    if host.protocol_version != COMPANION_PROTOCOL_VERSION {
        return Err("Controller uses an incompatible Companion protocol".into());
    }
    let target = native_companion_client::probe(host).await?;
    let view = CompanionPairingTargetView {
        host_id: target.host.host_id.clone(),
        service_name: target.host.service_name.clone(),
        manual_fingerprint: manual_certificate_fingerprint(&target.certificate_sha256)?,
        expires_at_ms: target.expires_at_ms,
    };
    *service.probed_target.lock().await = Some(target);
    Ok(view)
}

#[tauri::command]
#[cfg(not(any(target_os = "ios", target_os = "macos")))]
fn companion_pairing_prepare(
    _state: State<'_, SharedNativeState>,
    _host_id: String,
) -> Result<CompanionPairingTargetView, String> {
    Err("native Companion pairing is not available on this platform".into())
}

#[tauri::command]
#[cfg(any(target_os = "ios", target_os = "macos"))]
async fn companion_pairing_complete(
    app: tauri::AppHandle,
    state: State<'_, SharedNativeState>,
    service: State<'_, NativeCompanionService>,
    host_id: String,
    manual_fingerprint: String,
    code: String,
) -> Result<CompanionClientView, String> {
    use base64::{Engine as _, engine::general_purpose::STANDARD};

    if code.len() != 6 || !code.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err("enter the six-digit pairing code".into());
    }
    let existing_grant = apple_keychain::AppleKeychainStore
        .load(native_companion_client::CLIENT_GRANT_KEY)
        .map_err(|_| "secure storage is unavailable; pairing was not attempted".to_owned())?;
    let _existing_grant = existing_grant.map(Zeroizing::new);
    let target = service
        .probed_target
        .lock()
        .await
        .clone()
        .ok_or_else(|| "select and verify a Controller first".to_owned())?;
    if target.host.host_id != host_id
        || manual_certificate_fingerprint(&target.certificate_sha256)? != manual_fingerprint
    {
        return Err("TLS fingerprint confirmation does not match".into());
    }
    let (device_id, device_name) = {
        let mut state = state.lock().map_err(|error| error.to_string())?;
        if state.app_role != NativeAppRole::CompanionProjector {
            return Err("pairing is unavailable in Controller mode".into());
        }
        let device_id = match state
            .runtime
            .repository()
            .preference(COMPANION_CLIENT_DEVICE_ID_PREFERENCE)
            .map_err(|error| error.to_string())?
        {
            Some(device_id) => device_id,
            None => {
                let device_id = Uuid::new_v4().to_string();
                state
                    .runtime
                    .repository_mut()
                    .save_preference(COMPANION_CLIENT_DEVICE_ID_PREFERENCE, &device_id)
                    .map_err(|error| error.to_string())?;
                device_id
            }
        };
        let short_id = device_id.get(..8).unwrap_or(&device_id);
        let device_name = format!("Companion {short_id}");
        (device_id, device_name)
    };
    let request = PairingRequest {
        device_id: device_id.clone(),
        device_name,
        code,
    };
    let mut grant = native_companion_client::pair(&target, &request).await?;
    if grant.device_id != device_id || grant.role != CompanionRole::Projector {
        grant.token.zeroize();
        return Err("Controller returned an invalid Companion grant".into());
    }
    let mut stored = native_companion_client::StoredGrant {
        protocol_version: COMPANION_PROTOCOL_VERSION,
        host_id: target.host.host_id.clone(),
        certificate_der_base64: STANDARD.encode(&target.certificate_der),
        device_id,
        token: grant.token.clone(),
    };
    let mut blob = match serde_json::to_vec(&stored) {
        Ok(blob) => blob,
        Err(_) => {
            stored.token.zeroize();
            grant.token.zeroize();
            return Err("grant encoding failed".into());
        }
    };
    stored.token.zeroize();
    let save_result = apple_keychain::AppleKeychainStore
        .save(native_companion_client::CLIENT_GRANT_KEY, &blob)
        .map_err(|_| "secure grant storage failed".to_owned());
    blob.zeroize();
    if let Err(error) = save_result {
        grant.token.zeroize();
        return Err(error);
    }
    {
        let mut state = state.lock().map_err(|error| error.to_string())?;
        if let Err(error) = state
            .runtime
            .repository_mut()
            .save_preference(COMPANION_CLIENT_HOST_ID_PREFERENCE, &target.host.host_id)
        {
            eprintln!("Companion host preference could not be saved: {error}");
        }
    }
    let active_grant = native_companion_client::ActiveGrant {
        host_id: target.host.host_id.clone(),
        certificate_der: target.certificate_der,
        token: Zeroizing::new(grant.token),
    };
    let view = companion_client_view(
        &active_grant,
        &target.host.service_name,
        CompanionClientPhase::Discovering,
    );
    *service.active_grant.lock().await = Some(active_grant);
    *service.probed_target.lock().await = None;
    publish_companion_client_status(&app, &service.client_status, view.clone());
    restart_companion_client(&app, &service).await;
    Ok(view)
}

#[tauri::command]
#[cfg(not(any(target_os = "ios", target_os = "macos")))]
fn companion_pairing_complete(
    _app: tauri::AppHandle,
    _state: State<'_, SharedNativeState>,
    _host_id: String,
    _manual_fingerprint: String,
    _code: String,
) -> Result<CompanionClientView, String> {
    Err("native Companion pairing is not available on this platform".into())
}

#[tauri::command]
#[cfg(any(target_os = "ios", target_os = "macos"))]
async fn companion_client_status(
    state: State<'_, SharedNativeState>,
    service: State<'_, NativeCompanionService>,
) -> Result<Option<CompanionClientView>, String> {
    let is_companion = {
        let state = state.lock().map_err(|error| error.to_string())?;
        state.app_role == NativeAppRole::CompanionProjector
    };
    if !is_companion {
        return Ok(None);
    }
    service
        .client_status
        .lock()
        .map(|status| status.clone())
        .map_err(|error| error.to_string())
}

#[tauri::command]
#[cfg(not(any(target_os = "ios", target_os = "macos")))]
fn companion_client_status(
    _state: State<'_, SharedNativeState>,
) -> Result<Option<CompanionClientView>, String> {
    Ok(None)
}

#[tauri::command]
#[cfg(any(target_os = "ios", target_os = "macos"))]
fn companion_projector_v2_bootstrap(
    state: State<'_, SharedNativeState>,
    service: State<'_, NativeCompanionService>,
) -> Result<Envelope<RuntimePublicSnapshot>, String> {
    state.lock().map_err(|error| error.to_string())?.require_companion()?;
    service
        .client_frame
        .lock()
        .map_err(|error| error.to_string())?
        .as_ref()
        .map(companion_v2_envelope)
        .ok_or_else(|| "Companion snapshot is not connected".into())
}

#[tauri::command]
#[cfg(not(any(target_os = "ios", target_os = "macos")))]
fn companion_projector_v2_bootstrap(
    _state: State<'_, SharedNativeState>,
) -> Result<Envelope<RuntimePublicSnapshot>, String> {
    Err("native Companion projector is not available on this platform".into())
}

#[tauri::command]
fn companion_projector_v2_query(
    state: State<'_, SharedNativeState>,
    path: String,
) -> Result<serde_json::Value, String> {
    let state = state.lock().map_err(|error| error.to_string())?;
    companion_projector_v2_query_value(&state, &path)
}

fn companion_projector_v2_query_value(
    state: &NativeState,
    path: &str,
) -> Result<serde_json::Value, String> {
    if path.len() > 512 {
        return Err("Companion projector query is too long".into());
    }
    state.require_companion()?;
    match path {
        "/api/v2/modes" => {
            serde_json::to_value(registered_game_metadata()).map_err(|error| error.to_string())
        }
        "/api/v2/players" | "/api/v2/statistics/players" => Ok(serde_json::json!([])),
        "/api/v2/host" => Ok(serde_json::json!({
            "app_role": "companion_projector",
            "board": {"enabled": false, "phase": "disabled"},
            "test_events": false,
        })),
        _ => Err("unsupported Companion projector query".into()),
    }
}

#[tauri::command]
#[cfg(any(target_os = "ios", target_os = "macos"))]
async fn companion_projector_v2_report(
    state: State<'_, SharedNativeState>,
    service: State<'_, NativeCompanionService>,
    envelope: CommandEnvelope,
) -> Result<CommandResult, String> {
    {
        let state = state.lock().map_err(|error| error.to_string())?;
        state.require_companion()?;
    }
    if !runtime_v2_projector_report_allowed(&envelope.command) {
        return Err("Companion may only report geometry or sound status".into());
    }
    let grant = service.active_grant.lock().await.clone()
        .filter(native_companion_client::ActiveGrant::is_usable)
        .ok_or_else(|| "Companion is not paired".to_owned())?;
    let host = apple_bonjour::browser_snapshot()?
        .into_iter()
        .find(|host| host.host_id == grant.host_id)
        .ok_or_else(|| "paired Controller is not available".to_owned())?;
    native_companion_client::report(&host, &grant, &envelope).await
}

#[tauri::command]
#[cfg(not(any(target_os = "ios", target_os = "macos")))]
fn companion_projector_v2_report(
    _state: State<'_, SharedNativeState>,
    _envelope: CommandEnvelope,
) -> Result<CommandResult, String> {
    Err("native Companion projector is not available on this platform".into())
}

#[tauri::command]
#[cfg(any(target_os = "ios", target_os = "macos"))]
async fn app_role_select(
    app: tauri::AppHandle,
    state: State<'_, SharedNativeState>,
    service: State<'_, NativeCompanionService>,
    role: String,
) -> Result<PublicState, String> {
    let role = NativeAppRole::parse(&role)?;
    let shared_state = state.inner().clone();
    let transport = service.transport.clone();
    if role == NativeAppRole::CompanionProjector {
        apple_bonjour::browser_start()?;
        match shared_state
            .lock()
            .map_err(|error| error.to_string())?
            .select_app_role(role)
        {
            Ok(_) => {}
            Err(error) => {
                apple_bonjour::browser_stop();
                return Err(error);
            }
        };
        if let Some(transport) = transport {
            transport.stop().await;
        }
        apple_board_host::stop();
        restart_companion_client(&app, &service).await;
        let public = shared_state
            .lock()
            .map_err(|error| error.to_string())?
            .public();
        publish_public_state(&app, &public);
        return Ok(public);
    }

    let public = shared_state
        .lock()
        .map_err(|error| error.to_string())?
        .select_app_role(role)?;
    stop_companion_client(&service).await;
    apple_bonjour::browser_stop();
    apple_board_host::install();
    if public.projector_output == ProjectorOutput::Companion
        && let Some(transport) = transport
    {
        transport.start(shared_state).await?;
    }
    let public = state.lock().map_err(|error| error.to_string())?.public();
    publish_public_state(&app, &public);
    Ok(public)
}

#[tauri::command]
#[cfg(not(any(target_os = "ios", target_os = "macos")))]
fn app_role_select(
    app: tauri::AppHandle,
    state: State<'_, SharedNativeState>,
    role: String,
) -> Result<PublicState, String> {
    let role = NativeAppRole::parse(&role)?;
    if role == NativeAppRole::CompanionProjector {
        return Err("native Companion mode is not available on this platform".into());
    }
    let public = state
        .lock()
        .map_err(|error| error.to_string())?
        .select_app_role(role)?;
    publish_public_state(&app, &public);
    Ok(public)
}

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
    let mut runtime_message = None;
    if let Some(state) = app.try_state::<SharedNativeState>()
        && let Ok(state) = state.lock()
    {
        publish_to_external_projector(&state);
        let _ = state.companion_states.send(public.clone());
        runtime_message = Some(runtime_v2_envelope(&state));
    }
    let _ = app.emit("runtime-state", public);
    if let Some(message) = runtime_message {
        let _ = app.emit("runtime-v2-state", message);
    }
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
                && native_state.app_role == NativeAppRole::Controller
                && companion_transport.is_some();
            #[cfg(any(target_os = "ios", target_os = "macos"))]
            let app_role = native_state.app_role;
            #[cfg(any(target_os = "ios", target_os = "macos"))]
            let active_grant = match apple_keychain::AppleKeychainStore
                .load(native_companion_client::CLIENT_GRANT_KEY)
            {
                Ok(Some(bytes)) => {
                    let bytes = Zeroizing::new(bytes);
                    match native_companion_client::decode_stored_grant(&bytes) {
                        Ok(grant) => Some(grant),
                        Err(error) => {
                            eprintln!("Stored Companion grant is unavailable: {error}");
                            None
                        }
                    }
                }
                Ok(None) => None,
                Err(error) => {
                    eprintln!("Companion client secure storage is unavailable: {error}");
                    None
                }
            };
            #[cfg(any(target_os = "ios", target_os = "macos"))]
            if native_state.projector_output == ProjectorOutput::Companion
                && companion_transport.is_none()
            {
                native_state.projector_output = ProjectorOutput::ExternalDisplay;
            }
            let native_state = Arc::new(Mutex::new(native_state));
            app.manage(native_state.clone());
            #[cfg(any(target_os = "ios", target_os = "macos"))]
            let client_status = Arc::new(Mutex::new(active_grant.as_ref().map(|grant| {
                companion_client_view(
                    grant,
                    "Gekoppelter Controller",
                    CompanionClientPhase::Discovering,
                )
            })));
            #[cfg(any(target_os = "ios", target_os = "macos"))]
            app.manage(NativeCompanionService {
                transport: companion_transport.clone(),
                probed_target: AsyncMutex::new(None),
                active_grant: AsyncMutex::new(active_grant),
                client_status,
                client_frame: Arc::new(Mutex::new(None)),
                client_task: AsyncMutex::new(None),
            });
            #[cfg(any(target_os = "ios", target_os = "macos"))]
            let _ = APP_HANDLE.set(app.handle().clone());
            #[cfg(target_os = "ios")]
            {
                if let Some(state) = app.try_state::<SharedNativeState>() {
                    if let Ok(state) = state.lock() {
                        publish_to_external_projector(&state);
                    }
                }
            }
            #[cfg(any(target_os = "ios", target_os = "macos"))]
            match app_role {
                NativeAppRole::Controller => apple_board_host::install(),
                NativeAppRole::CompanionProjector => {
                    if let Err(error) = apple_bonjour::browser_start() {
                        eprintln!("Companion discovery startup failed: {error}");
                    } else {
                        let app_handle = app.handle().clone();
                        tauri::async_runtime::spawn(async move {
                            let service = app_handle.state::<NativeCompanionService>();
                            restart_companion_client(&app_handle, &service).await;
                        });
                    }
                }
            }
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
                tauri::WebviewUrl::App("projector.html".into()),
            )
            .title("Smart Dartboard · Projector")
            .inner_size(1280.0, 720.0)
            .build()?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            runtime_bootstrap,
            runtime_query,
            runtime_dispatch,
            runtime_v2_bootstrap,
            runtime_v2_snapshot,
            runtime_v2_query,
            runtime_v2_dispatch,
            runtime_v2_report,
            runtime_v2_projector_test_event,
            companion_pairing_open,
            companion_devices,
            companion_revoke,
            companion_discovery_start,
            companion_discovered_hosts,
            companion_discovery_stop,
            companion_pairing_prepare,
            companion_pairing_complete,
            companion_client_status,
            companion_projector_v2_bootstrap,
            companion_projector_v2_query,
            companion_projector_v2_report,
            app_role_select,
            projector_output_select
        ])
        .run(tauri::generate_context!())
        .expect("error while running Smart Dartboard native M0");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(any(target_os = "ios", target_os = "macos"))]
    #[derive(Default)]
    struct TestSecretStore(Mutex<Option<Vec<u8>>>);

    #[cfg(any(target_os = "ios", target_os = "macos"))]
    impl SecretStore for TestSecretStore {
        fn load(&self, _key: &str) -> Result<Option<Vec<u8>>, String> {
            Ok(self.0.lock().map_err(|error| error.to_string())?.clone())
        }

        fn save(&self, _key: &str, value: &[u8]) -> Result<(), String> {
            *self.0.lock().map_err(|error| error.to_string())? = Some(value.to_vec());
            Ok(())
        }
    }

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
    fn native_runtime_query_exposes_the_complete_read_model() {
        let state = NativeState::restore(
            SqliteRepository::in_memory().expect("repository"),
            test_companion_identity(),
        )
        .expect("native state");
        for path in [
            "/api/v2/history/sessions?limit=100",
            "/api/v2/statistics/players?include_test=true",
            "/api/v2/statistics/modes",
            "/api/v2/statistics/heatmap?player_id=test-player&include_test=true",
            "/api/v2/data/export",
        ] {
            assert!(runtime_v2_query_value(&state, path).is_ok(), "{path}");
        }
        let export = runtime_v2_query_value(&state, "/api/v2/data/export").expect("export");
        assert_eq!(export["schema_version"], 2);
        assert!(runtime_v2_query_value(&state, "/api/v2/history/games/missing").is_err());
        assert!(runtime_v2_query_value(&state, &"x".repeat(2_049)).is_err());
    }

    #[test]
    fn projector_report_bridge_cannot_mutate_game_or_setup_authority() {
        assert!(runtime_v2_projector_report_allowed(
            &RuntimeCommand::ReportProjectorGeometry {
                geometry: sdb_contracts::ProjectorGeometry {
                    width: 1_366,
                    height: 900,
                },
            }
        ));
        assert!(runtime_v2_projector_report_allowed(
            &RuntimeCommand::ReportSoundStatus {
                status: sdb_contracts::SoundStatus::Ready,
            }
        ));
        assert!(!runtime_v2_projector_report_allowed(
            &RuntimeCommand::ResetCalibration
        ));
        assert!(!runtime_v2_projector_report_allowed(
            &RuntimeCommand::SoundTest {
                effect_id: "forbidden".into(),
            }
        ));
        assert!(runtime_v2_projector_test_event_allowed(
            &RuntimeCommand::IngestDart {
                event: DartEvent::Hit {
                    seq: 1,
                    field: 20,
                    ring: Ring::Triple,
                    multiplier: 3,
                    label: "T20".into(),
                    score: 60,
                },
                source: DartSource::ProjectorTest,
            }
        ));
        assert!(!runtime_v2_projector_test_event_allowed(
            &RuntimeCommand::IngestDart {
                event: DartEvent::Hit {
                    seq: 1,
                    field: 20,
                    ring: Ring::Triple,
                    multiplier: 3,
                    label: "T20".into(),
                    score: 60,
                },
                source: DartSource::ManualCorrection,
            }
        ));
        assert!(!runtime_v2_projector_test_event_allowed(
            &RuntimeCommand::ResetCalibration
        ));
    }

    #[cfg(any(target_os = "ios", target_os = "macos"))]
    #[tokio::test]
    async fn companion_client_probes_then_pairs_over_verified_tls() {
        let _network_guard = APPLE_NETWORK_TEST_LOCK.lock().await;
        let host_id = "8dd5dc20-9010-4cce-9721-08e9931acc10";
        let identity =
            load_or_create_identity(&TestSecretStore::default(), host_id).expect("TLS identity");
        let state = Arc::new(Mutex::new(
            NativeState::restore(
                SqliteRepository::in_memory().expect("repository"),
                CompanionIdentity {
                    host_id: host_id.into(),
                    certificate_sha256: identity.certificate_sha256().into(),
                    available: true,
                },
            )
            .expect("native state"),
        ));
        let offer = state
            .lock()
            .expect("state")
            .companions
            .open(now_ms())
            .expect("pairing offer");
        let transport = native_companion_transport::NativeCompanionTransport::new(&identity)
            .expect("transport");
        let port = transport.start(state.clone()).await.expect("start");
        let target = native_companion_client::probe(DiscoveredCompanionHost {
            service_name: "Test Controller".into(),
            host_name: "localhost".into(),
            port,
            host_id: host_id.into(),
            protocol_version: COMPANION_PROTOCOL_VERSION,
            tls: true,
        })
        .await
        .expect("probe");
        assert_eq!(target.certificate_sha256, identity.certificate_sha256());
        let request = PairingRequest {
            device_id: "0d00e843-d495-44b3-9eb0-6f7e0ce304df".into(),
            device_name: "Test iPad".into(),
            code: offer.code,
        };
        let other_identity = load_or_create_identity(
            &TestSecretStore::default(),
            "4fb86f35-09d7-4e20-836f-016ad1fc21a5",
        )
        .expect("other TLS identity");
        let mut wrongly_pinned = target.clone();
        wrongly_pinned.certificate_der = other_identity.certificate_der().to_vec();
        assert!(
            native_companion_client::pair(&wrongly_pinned, &request)
                .await
                .is_err()
        );
        let grant = native_companion_client::pair(&target, &request)
            .await
            .expect("pair");
        assert_eq!(grant.role, CompanionRole::Projector);
        assert!(
            state
                .lock()
                .expect("state")
                .companions
                .authenticate(&grant.token)
                .is_some()
        );
        let active_grant = native_companion_client::ActiveGrant {
            host_id: host_id.into(),
            certificate_der: target.certificate_der.clone(),
            token: Zeroizing::new(grant.token),
        };
        let stream_host = target.host.clone();
        let stream_grant = active_grant.clone();
        let (frames, mut received) = tokio::sync::mpsc::unbounded_channel();
        let client = tokio::spawn(async move {
            native_companion_client::replicate(&stream_host, &stream_grant, |public| {
                frames.send(public).expect("frame receiver")
            })
            .await
        });
        let initial = tokio::time::timeout(std::time::Duration::from_secs(3), received.recv())
            .await
            .expect("bootstrap timeout")
            .expect("bootstrap frame");
        tokio::time::timeout(std::time::Duration::from_secs(3), async {
            loop {
                if state
                    .lock()
                    .expect("state")
                    .companion_states
                    .receiver_count()
                    > 0
                {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("WebSocket subscription timeout");
        let report = native_companion_client::report(
            &target.host,
            &active_grant,
            &CommandEnvelope {
                protocol_version: sdb_contracts::PROTOCOL_VERSION,
                command_id: "companion-sound-ready".into(),
                runtime_instance_id: initial.runtime_instance_id.clone(),
                expected_revision: Some(initial.revision),
                command: RuntimeCommand::ReportSoundStatus {
                    status: sdb_contracts::SoundStatus::Ready,
                },
            },
        )
        .await
        .expect("authenticated Companion report");
        assert_eq!(report.revision, initial.revision + 1);
        let reported = tokio::time::timeout(std::time::Duration::from_secs(3), received.recv())
            .await
            .expect("report state timeout")
            .expect("report state frame");
        assert_eq!(reported.runtime.settings.sound.status, sdb_contracts::SoundStatus::Ready);
        let updated = {
            let mut state = state.lock().expect("state");
            let public = state.ingest_test_hit().expect("test hit");
            state
                .companion_states
                .send(public.clone())
                .expect("publish state");
            public
        };
        let replicated = tokio::time::timeout(std::time::Duration::from_secs(3), received.recv())
            .await
            .expect("state timeout")
            .expect("state frame");
        assert_eq!(reported.revision + 1, replicated.revision);
        assert_eq!(replicated, updated);
        let product = companion_v2_envelope(&replicated);
        assert_eq!(product.runtime_instance_id, replicated.runtime_instance_id);
        assert_eq!(product.revision, replicated.revision);
        assert_eq!(product.payload, replicated.runtime);
        state
            .lock()
            .expect("state")
            .revoke_companion("0d00e843-d495-44b3-9eb0-6f7e0ce304df", now_ms())
            .expect("revoke");
        assert!(
            tokio::time::timeout(std::time::Duration::from_secs(3), client)
                .await
                .expect("stream close timeout")
                .expect("client task")
                .is_err()
        );
        assert_eq!(
            native_companion_client::replicate(&target.host, &active_grant, |_| {}).await,
            Err(native_companion_client::SessionError::Authorization)
        );
        transport.stop().await;
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
    fn companion_role_is_persisted_and_cannot_mutate_the_local_runtime() {
        let path = std::env::temp_dir().join(format!("sdb-native-role-{}.sqlite", Uuid::new_v4()));
        let revision = {
            let repository = SqliteRepository::open(&path).expect("repository");
            let mut state =
                NativeState::restore(repository, test_companion_identity()).expect("native state");
            let revision = state.public().revision;
            let public = state
                .select_app_role(NativeAppRole::CompanionProjector)
                .expect("select Companion role");
            assert_eq!(public.app_role, NativeAppRole::CompanionProjector);
            assert_eq!(public.board.phase, BoardPhase::Disabled);
            assert!(state.ingest_test_hit().is_err());
            assert!(state.ingest_board_packet("stale-board", &[0; 10]).is_err());
            assert!(
                companion_projector_v2_query_value(&state, "/api/v2/modes")
                    .expect("mode metadata")
                    .as_array()
                    .is_some_and(|modes| modes.len() == 24)
            );
            assert_eq!(
                companion_projector_v2_query_value(&state, "/api/v2/host")
                    .expect("read-only host")["test_events"],
                false
            );
            assert!(companion_projector_v2_query_value(&state, "/api/v2/data/export").is_err());
            revision
        };

        let repository = SqliteRepository::open(&path).expect("reopened repository");
        let state =
            NativeState::restore(repository, test_companion_identity()).expect("restored state");
        assert_eq!(state.app_role, NativeAppRole::CompanionProjector);
        assert_eq!(state.public().revision, revision);
        assert_eq!(state.public().board.phase, BoardPhase::Disabled);
        std::fs::remove_file(path).expect("remove test database");
    }

    #[test]
    fn fresh_companion_role_does_not_bootstrap_an_authoritative_game() {
        let mut repository = SqliteRepository::in_memory().expect("repository");
        repository
            .save_preference(APP_ROLE_PREFERENCE, "companion_projector")
            .expect("persist role");
        let state =
            NativeState::restore(repository, test_companion_identity()).expect("native state");
        assert_eq!(state.app_role, NativeAppRole::CompanionProjector);
        assert_eq!(state.runtime.snapshot().revision, 0);
        assert!(state.runtime.snapshot().game.is_none());
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
