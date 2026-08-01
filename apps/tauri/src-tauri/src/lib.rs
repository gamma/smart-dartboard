use serde::Serialize;
use std::sync::Mutex;
use tauri::{Emitter, State};

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
}

impl NativeState {
    const fn public(&self) -> PublicState {
        PublicState {
            runtime_instance_id: "native-m0",
            revision: self.revision,
            counter: self.counter,
        }
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
    let public = {
        let mut state = state.lock().map_err(|error| error.to_string())?;
        state.counter += 1;
        state.revision += 1;
        state.public()
    };
    app.emit("runtime-state", &public)
        .map_err(|error| error.to_string())?;
    Ok(public)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(Mutex::new(NativeState::default()))
        .setup(|app| {
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
