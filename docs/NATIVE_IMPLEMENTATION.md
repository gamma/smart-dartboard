# Native Implementierung

Stand: 2026-08-01

Dieses Dokument beschreibt den belegten Implementierungsstand des
Cross-Platform-Rewrites. Zielarchitektur und Abnahmekriterien stehen in
[CROSS_PLATFORM_ARCHITECTURE.md](CROSS_PLATFORM_ARCHITECTURE.md).

## Aktueller Stand

Umgesetzt und lokal verifiziert:

- Rust-Workspace mit Contracts, FFF1-Protokoll, CountUp- und X01-Kern,
  atomarer Runtime und SQLite-Repository,
- gemeinsame Golden Fixtures für Python und Rust; bei X01 einschließlich Bust,
  Double-out, Spielerwechsel, Wurfkorrektur, Löschen und Undo,
- Web-UI hinter `HostedRuntimeClient` und `TauriRuntimeClient`,
- macOS-Tauri-App mit Control- und Projector-Fenster,
- iOS-/iPadOS-Tauri-App für `aarch64-apple-ios-sim`,
- nativer Apple-DisplayHost mit eigener Projector-WKWebView auf `TVOut`,
- Live-State-Verteilung sowie Disconnect/Reconnect ohne Zustandsverlust,
- Apple-M0 verwendet für den Testtreffer keinen Demo-Zähler mehr: Ein
  kanonisches `T20`-Event läuft durch den gemeinsamen CountUp-Core und die
  revisionsgesicherte Runtime. Control und Projector zeigten im
  iPad-Pro-Simulator übereinstimmend Score 60 bei Revision 2.
- Rust-Headless-Server mit expliziter API v2, SQLite-Recovery, idempotenten
  Command-Envelopes, Snapshot-WebSocket und nicht privilegiertem
  Vorschaucontainer. Details: [RUST_SERVER_V2.md](RUST_SERVER_V2.md).

Noch nicht als produktionsreif nachgewiesen:

- echtes CoreBluetooth-Board auf macOS und iOS/iPadOS,
- reale AirPlay-, HDMI- und Audio-Hardware,
- External-Display-Scene-Accessory ab iOS/iPadOS 27,
- iPhone/iPad-zu-iPad-Companion mit Pairing,
- vollständige Portierung aller Spielmodi, Sessions und Statistiken,
- vollständige Docker-Parität zur Python-Anwendung.

## Lokale Befehle

Rust-Core:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

macOS-M0:

```bash
npm --prefix apps/tauri ci
npm --prefix apps/tauri run build
```

iOS-Simulator-Build:

```bash
npm --prefix apps/tauri run tauri ios build -- \
  --debug --target aarch64-sim --no-sign --ci
```

Der M0-Zwei-Display-Test kann in einem Debug-Build mit dem Simulator-Argument
`--m0-test-hit-after-start` reproduziert werden. Revision 1 ist dabei der
CountUp-Start, Revision 2 der automatische `T20`-Treffer. Der virtuelle externe
Ausgang wird über `simctl io ... screenConfig --display external power on|off`
verbunden beziehungsweise getrennt.

## Apple-DisplayHost

Die technische Entscheidung und ihre Versionsgrenzen stehen in
[ADR 0001](adr/0001-apple-external-display-host.md). Die Control UI zeigt den
aktuellen Status `nicht verbunden` oder die Zahl der aktiven AirPlay-/HDMI-
Displays. Das Projector-Fenster besitzt keinen schreibenden Runtime-Zugriff.

## Hardware-Abnahme

Ein Apple-Mobile-Ziel ist erst qualifiziert, wenn derselbe Test auf echter
Hardware folgende Ergebnisse liefert:

1. Board automatisch per CoreBluetooth verbinden.
2. Drei schnelle Treffer exakt einmal übernehmen.
3. Eigenständigen Projector über AirPlay testen.
4. Eigenständigen Projector über HDMI/USB-C testen.
5. Projector während eines Spiels trennen und erneut verbinden.
6. State, Revision, Kalibrierung und Soundausgabe nach Reconnect prüfen.
7. Background/Foreground, Rotation und eingehenden Anruf simulieren.
8. Einen signierbaren TestFlight-Build erzeugen.
