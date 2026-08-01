use serde::Serialize;
use std::sync::Mutex;
use tauri::{Emitter, State};

#[cfg(target_os = "ios")]
use std::sync::OnceLock;
#[cfg(target_os = "ios")]
use std::sync::atomic::{AtomicU32, Ordering};
#[cfg(any(target_os = "ios", debug_assertions))]
use tauri::Manager;

#[cfg(target_os = "ios")]
static APP_HANDLE: OnceLock<tauri::AppHandle> = OnceLock::new();

#[cfg(target_os = "ios")]
static EXTERNAL_DISPLAY_COUNT: AtomicU32 = AtomicU32::new(0);

#[derive(Debug, Default)]
struct NativeState {
    counter: u64,
    revision: u64,
}

#[derive(Debug, Clone, Serialize)]
struct PublicState {
    runtime_instance_id: &'static str,
    revision: u64,
    counter: u64,
    external_display_count: u32,
}

impl NativeState {
    fn public(&self) -> PublicState {
        PublicState {
            runtime_instance_id: "native-m0",
            revision: self.revision,
            counter: self.counter,
            external_display_count: external_display_count(),
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
        state.counter += 1;
        state.revision += 1;
        state.public()
    };
    publish_to_external_projector(&public);
    app.emit("runtime-state", &public)
        .map_err(|error| error.to_string())?;
    Ok(public)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(Mutex::new(NativeState::default()))
        .setup(|app| {
            #[cfg(target_os = "ios")]
            {
                let _ = APP_HANDLE.set(app.handle().clone());
                if let Some(state) = app.try_state::<Mutex<NativeState>>() {
                    if let Ok(state) = state.lock() {
                        publish_to_external_projector(&state.public());
                    }
                }
            }
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
            runtime_dispatch
        ])
        .run(tauri::generate_context!())
        .expect("error while running Smart Dartboard native M0");
}
