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
- deterministischer Session-Core mit gemeinsamer Python-/Rust-Fixture für
  Screenfluss, Starterrotation, Rematch, Abbruch, Draw sowie Einzel- und
  Koop-Wertung mit drei Punkten pro gewonnenem Spiel.
- Spiel und Session werden in einem Runtime-Snapshot atomar committed. Undo
  eines Siegtreffers öffnet das Ergebnis wieder und entfernt auch die zuvor
  vergebene Sessionwertung; ein Commitfehler lässt beide Zustände unverändert.
- SQLite-Schema 4 mit fortlaufender Migration, Downgrade-Sperre,
  Post-Migration-Integritätsprüfung und append-only Runtime-Journal. Ein
  injizierter Fehler beim Journal-Insert rollt Snapshot und Deduplizierung mit
  zurück.
- Transaktionale Historienprojektion für Profile, Sessions, Spiele, Würfe,
  Gewinner und Endstände. Nur beendete Produktionsspiele fließen in die neue
  Spielerstatistik ein; Undo macht Wurf und Sieg unwirksam, ohne das
  Auditereignis zu löschen. Die ersten read-only API-v2-Endpunkte liefern
  Profile, Sessionhistorie und Spielerstatistiken.
- Gemeinsamer Dart-Source-Contract für Board, Projektor-Test und manuelle
  Korrektur. Der Apple-Testtreffer ist als `projector_test` markiert; solche
  Spiele bleiben dauerhaft aus der normalen Statistik ausgeschlossen.
- Öffentliche X01-Commands zum Korrigieren und Löschen über stabile Action-IDs.
  Der gemeinsame State liefert die editierbaren Darts der aktuellen und
  vorherigen Aufnahme. Replay, Sessionpunkte, Gewinner, kanonische `throws`
  und append-only Korrekturevents werden atomar synchronisiert.
- API-v2-Details für Sessions und Spiele sowie ein vollständiges Replay-Envelope
  mit Initialzustand, Finalzustand, Frames, unwirksam gemachten Originalevents
  und Korrekturverkettung. Unbekannte IDs liefern stabil `not_found`/HTTP 404.
- Transportneutraler Rust-Board-Ingress für rohe FFF1-Pakete mit gemeinsamer
  Interpretation, begrenzter Deduplizierung und stabilen Fehlerzuständen. Ein
  unprivilegierter Linux-Bleak-Sidecar liefert nur Notifications und Status
  über token-authentisierte interne Endpunkte; der reale Containervertrag ist
  ohne Hardware per D20-Rohpaket verifiziert.
- iOS-/iPadOS-CoreBluetooth-Host mit State Restoration, gespeicherter
  Peripheral-ID, Namens-/Service-Scan-Fallback und den Phasen Berechtigung,
  Scan, Verbindung, Serviceprüfung, Subscription, Ready und Reconnect. Der
  Adapter liefert FFF1 unverändert an denselben Rust-Ingress. Simulator-Bundle
  und FFI linken erfolgreich; ein nativer Test verarbeitet D20 genau einmal,
  und die Control UI zeigt im BLE-losen Simulator verständlich `nicht
  verfügbar`.

Noch nicht als produktionsreif nachgewiesen:

- echtes CoreBluetooth-Board auf macOS und iOS/iPadOS; der iOS-Adapter ist
  implementiert, aber noch nicht mit der realen Scheibe qualifiziert, der
  macOS-Host ist noch nicht verdrahtet,
- reale AirPlay-, HDMI- und Audio-Hardware,
- External-Display-Scene-Accessory ab iOS/iPadOS 27,
- iPhone/iPad-zu-iPad-Companion mit Pairing,
- vollständige Portierung aller Spielmodi sowie Heatmap, Modusstatistiken,
  Export und Trainingsempfehlungen,
- vollständige Docker-Parität zur Python-Anwendung,
- reale Linux-BlueZ-/Board-Abnahme des neuen Sidecars.

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
npm --prefix apps/tauri run ios:build:sim
```

Das Script entfernt vorab nur das generierte Simulator-Archiv und das bereits
exportierte `.app`-Bundle. Das ist nötig, weil Tauri 2.11.4 beim wiederholten
Simulator-Build ein vorhandenes Exportziel nicht selbst ersetzt.

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

## Apple-BoardTransport

`BoardTransportHost.mm` besitzt als dünner Plattformadapter ausschließlich
CoreBluetooth-Discovery, Verbindung, State Restoration und FFF1-Subscription.
Decoder, Button-/Miss-Interpretation, Deduplizierung und Runtime-Dispatch liegen
im gemeinsamen Rust-`sdb-board`-Crate. Damit kann derselbe Byte-Fixture-Test auf
Linux, macOS, iPhone und iPad gelten.

Die App verlangt `NSBluetoothAlwaysUsageDescription` und deklariert
`bluetooth-central` für Preservation/Restoration. Eine bekannte iOS-Peripheral-
UUID wird in `NSUserDefaults` gespeichert; ist sie nicht mehr abrufbar oder
scheitert die Verbindung, scannt der Host erneut nach Name `SDB-BT` oder Service
`FFF0`. Die UI zeigt den stabilen Boardstatus, aber keine rohen Backendfehler als
Spielmeldung.

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
