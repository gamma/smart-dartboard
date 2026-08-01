fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        let source = "gen/apple/Sources/sdb-native-m0/BoardTransportHost.mm";
        let header = "gen/apple/Sources/sdb-native-m0/BoardTransportHost.h";
        println!("cargo:rerun-if-changed={source}");
        println!("cargo:rerun-if-changed={header}");
        cc::Build::new()
            .cpp(true)
            .file(source)
            .flag("-fobjc-arc")
            .compile("sdb_apple_board_transport");
        println!("cargo:rustc-link-lib=framework=CoreBluetooth");
        println!("cargo:rustc-link-lib=framework=Foundation");
    }
    let attributes =
        tauri_build::Attributes::new().app_manifest(tauri_build::AppManifest::new().commands(&[
            "runtime_bootstrap",
            "runtime_dispatch",
            "runtime_query",
            "companion_pairing_open",
            "companion_devices",
            "companion_revoke",
        ]));
    tauri_build::try_build(attributes).expect("failed to build Tauri application");
}
