fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        let source = "gen/apple/Sources/sdb-native-m0/BoardTransportHost.mm";
        let header = "gen/apple/Sources/sdb-native-m0/BoardTransportHost.h";
        let keychain_source = "gen/apple/Sources/sdb-native-m0/CompanionKeychainHost.mm";
        let keychain_header = "gen/apple/Sources/sdb-native-m0/CompanionKeychainHost.h";
        let bonjour_source = "gen/apple/Sources/sdb-native-m0/CompanionBonjourHost.mm";
        let bonjour_header = "gen/apple/Sources/sdb-native-m0/CompanionBonjourHost.h";
        let lifecycle_source = "gen/apple/Sources/sdb-native-m0/AppLifecycleHost.mm";
        let lifecycle_header = "gen/apple/Sources/sdb-native-m0/AppLifecycleHost.h";
        println!("cargo:rerun-if-changed={source}");
        println!("cargo:rerun-if-changed={header}");
        println!("cargo:rerun-if-changed={keychain_source}");
        println!("cargo:rerun-if-changed={keychain_header}");
        println!("cargo:rerun-if-changed={bonjour_source}");
        println!("cargo:rerun-if-changed={bonjour_header}");
        println!("cargo:rerun-if-changed={lifecycle_source}");
        println!("cargo:rerun-if-changed={lifecycle_header}");
        cc::Build::new()
            .cpp(true)
            .file(source)
            .file(keychain_source)
            .file(bonjour_source)
            .file(lifecycle_source)
            .flag("-fobjc-arc")
            .compile("sdb_apple_board_transport");
        println!("cargo:rustc-link-lib=framework=CoreBluetooth");
        println!("cargo:rustc-link-lib=framework=Foundation");
        println!("cargo:rustc-link-lib=framework=AppKit");
        println!("cargo:rustc-link-lib=framework=Security");
    }
    let attributes =
        tauri_build::Attributes::new().app_manifest(tauri_build::AppManifest::new().commands(&[
            "runtime_query",
            "runtime_v2_import_data",
            "companion_pairing_open",
            "companion_devices",
            "companion_revoke",
            "companion_discovery_start",
            "companion_discovered_hosts",
            "companion_discovery_stop",
            "companion_pairing_prepare",
            "companion_pairing_complete",
            "companion_client_status",
            "app_role_select",
            "projector_output_select",
        ]));
    tauri_build::try_build(attributes).expect("failed to build Tauri application");
}
