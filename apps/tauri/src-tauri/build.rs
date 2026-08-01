fn main() {
    let attributes =
        tauri_build::Attributes::new().app_manifest(tauri_build::AppManifest::new().commands(&[
            "runtime_bootstrap",
            "runtime_dispatch",
            "runtime_query",
        ]));
    tauri_build::try_build(attributes).expect("failed to build Tauri application");
}
