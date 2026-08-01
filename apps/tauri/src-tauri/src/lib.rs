use sdb_board::BoardStatus;
#[cfg(any(target_os = "ios", target_os = "macos"))]
use sdb_board::{BoardFailureCode, BoardPhase};
#[cfg(any(target_os = "ios", target_os = "macos", test))]
use sdb_board::{BoardIngress, BoardIngressOutcome};
use sdb_companion::{
    CompanionRole, PairedDevice, PairingAuthority, PairingBootstrap, PairingGrant, PairingRequest,
};
use sdb_companion_transport::{
    SecretStore, TlsIdentity, load_identity, load_or_create_identity,
};
use sdb_contracts::{DartEvent, DartSource, Ring};
use sdb_runtime::{Runtime, RuntimeAction, RuntimeGameState};
use sdb_storage::SqliteRepository;
use serde::Serialize;
use std::{
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH},
};
use tauri::{Emitter, Manager, State};
use uuid::Uuid;

const PROJECTOR_OUTPUT_PREFERENCE: &str = "projector.output";
const COMPANION_HOST_ID_PREFERENCE: &str = "companion.host_id";

#[cfg(any(target_os = "ios", target_os = "macos"))]
use std::sync::OnceLock;
#[cfg(target_os = "ios")]
use std::sync::atomic::{AtomicU32, Ordering};
#[cfg(any(target_os = "ios", target_os = "macos"))]
static APP_HANDLE: OnceLock<tauri::AppHandle> = OnceLock::new();

#[cfg(target_os = "ios")]
static EXTERNAL_DISPLAY_COUNT: AtomicU32 = AtomicU32::new(0);

struct NativeState {
    runtime: Runtime<SqliteRepository>,
    next_dart_seq: u64,
    #[cfg(any(target_os = "ios", target_os = "macos", test))]
    board_ingress: BoardIngress,
    board_status: BoardStatus,
    companions: PairingAuthority,
    projector_output: ProjectorOutput,
    companion_identity: CompanionIdentity,
}

#[derive(Debug, Clone)]
struct CompanionIdentity {
    host_id: String,
    certificate_sha256: String,
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
}

#[derive(Debug, Clone, Serialize)]
struct CompanionDeviceView {
    device_id: String,
    device_name: String,
    role: CompanionRole,
    paired_at_ms: u64,
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
mod apple_board {
    use super::{
        APP_HANDLE, BoardFailureCode, BoardPhase, BoardStatus, NativeState, publish_public_state,
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
        let Some(state) = app.try_state::<std::sync::Mutex<NativeState>>() else {
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
        let Some(state) = app.try_state::<std::sync::Mutex<NativeState>>() else {
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
fn runtime_bootstrap(state: State<'_, Mutex<NativeState>>) -> Result<PublicState, String> {
    state
        .lock()
        .map(|state| state.public())
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn runtime_query(state: State<'_, Mutex<NativeState>>) -> Result<PublicState, String> {
    runtime_bootstrap(state)
}

#[tauri::command]
fn companion_pairing_open(
    state: State<'_, Mutex<NativeState>>,
) -> Result<PairingBootstrap, String> {
    let mut state = state.lock().map_err(|error| error.to_string())?;
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
    state: State<'_, Mutex<NativeState>>,
) -> Result<Vec<CompanionDeviceView>, String> {
    state
        .lock()
        .map(|state| state.companion_devices())
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn companion_revoke(
    state: State<'_, Mutex<NativeState>>,
    device_id: String,
) -> Result<Vec<CompanionDeviceView>, String> {
    let mut state = state.lock().map_err(|error| error.to_string())?;
    state.revoke_companion(&device_id, now_ms())?;
    Ok(state.companion_devices())
}

#[tauri::command]
fn projector_output_select(
    app: tauri::AppHandle,
    state: State<'_, Mutex<NativeState>>,
    output: String,
) -> Result<PublicState, String> {
    let public = state
        .lock()
        .map_err(|error| error.to_string())?
        .select_projector_output(ProjectorOutput::parse(&output)?)?;
    publish_public_state(&app, &public);
    Ok(public)
}

#[tauri::command]
fn runtime_dispatch(
    app: tauri::AppHandle,
    state: State<'_, Mutex<NativeState>>,
    action: String,
) -> Result<PublicState, String> {
    if action != "increment" {
        return Err("unsupported M0 action".into());
    }
    increment_runtime(&app, &state)
}

fn increment_runtime(
    app: &tauri::AppHandle,
    state: &Mutex<NativeState>,
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
    if let Some(identity) = load_identity(&apple_keychain::AppleKeychainStore)
        .map_err(|error| error.to_string())?
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
            let tls_identity =
                prepare_companion_identity(&mut repository).map_err(std::io::Error::other)?;
            #[cfg(any(target_os = "ios", target_os = "macos"))]
            let companion_identity = CompanionIdentity {
                host_id: tls_identity.host_id().into(),
                certificate_sha256: tls_identity.certificate_sha256().into(),
            };
            #[cfg(not(any(target_os = "ios", target_os = "macos")))]
            let companion_identity = CompanionIdentity {
                host_id: "unsupported-native-host".into(),
                certificate_sha256: "00".repeat(32),
            };
            let native_state = NativeState::restore(repository, companion_identity)
                .map_err(std::io::Error::other)?;
            app.manage(Mutex::new(native_state));
            #[cfg(any(target_os = "ios", target_os = "macos"))]
            app.manage(tls_identity);
            #[cfg(any(target_os = "ios", target_os = "macos"))]
            let _ = APP_HANDLE.set(app.handle().clone());
            #[cfg(target_os = "ios")]
            {
                if let Some(state) = app.try_state::<Mutex<NativeState>>() {
                    if let Ok(state) = state.lock() {
                        publish_to_external_projector(&state.public());
                    }
                }
            }
            #[cfg(target_os = "macos")]
            macos_board_host::install();
            #[cfg(debug_assertions)]
            if std::env::args().any(|argument| argument == "--m0-test-hit-after-start") {
                let app = app.handle().clone();
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_secs(3));
                    if let Some(state) = app.try_state::<Mutex<NativeState>>() {
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
        }
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
