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
- SQLite-Schema 6 mit fortlaufender Migration, Downgrade-Sperre,
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
- gemeinsamer macOS-/iOS-/iPadOS-CoreBluetooth-Host mit State Restoration,
  gespeicherter Peripheral-ID, Namens-/Service-Scan-Fallback und den Phasen
  Berechtigung, Scan, Verbindung, Serviceprüfung, Subscription, Ready und
  Reconnect. Der Adapter liefert FFF1 unverändert an denselben Rust-Ingress. iOS-Simulator-
  Bundle und macOS-Release-Binary linken und starten erfolgreich; ein nativer
  Test verarbeitet D20 genau einmal, und die Control UI zeigt im BLE-losen
  Simulator verständlich `nicht verfügbar`.
- dieselbe Schema-6-SQLite-Persistenz wie der Headless-Host im nativen
  App-Datenverzeichnis. Jede Prozessausführung erhält eine neue Runtime-ID,
  stellt aber ausschließlich den letzten atomar committed Snapshot wieder her.
  Im iPad-Simulator blieben Score 60 und Revision 2 nach vollständigem
  Terminieren und erneutem Start erhalten; die Runtime-ID wechselte dabei wie
  vorgesehen.
- plattformneutraler Companion-Kern für fünf Minuten gültige sechsstellige
  Einmalcodes, Versuchslimit, 256-Bit-Projector-Tokens, Hashspeicherung,
  Widerruf und strikt revisionsgenaue Snapshot-Replikation. Der Headless-Host
  bietet den authentisierten Pairing-, Bootstrap- und WebSocket-Transport an
  und schließt aktive Verbindungen beim Widerruf oder einer Revisionslücke.
  Native Discovery, TLS/Keychain, eingehender Transport und die UI der
  Projector-Rolle sind noch offen; Details:
  [COMPANION_PROTOCOL.md](COMPANION_PROTOCOL.md).
- natives Board-Setup für den Controller: Pairing-Fenster öffnen, gruppierten
  Einmalcode mit Live-Countdown anzeigen, persistierte Projector-Geräte ohne
  Token-Hash auflisten und Grants widerrufen. Nur das Control-Fenster besitzt
  diese Tauri-Rechte; das Einlösen bleibt der noch anzuschließenden nativen
  Netzwerkgrenze vorbehalten. Der iPad-Viewport wurde mit WebKit visuell
  geprüft.
- persistente Projector-Auswahl im Board-Setup mit `AirPlay / HDMI` als
  Standard, `Companion-iPad` und lokaler Vorschau. Der Wechsel schreibt nur die
  Hostpräferenz in SQLite-Schema 6, erzeugt keine Runtime-Revision und lässt
  Session, Spiel und BLE-Verbindung unberührt. Nicht ausgewählte lokale
  Projector-Ausgänge werden neutral ausgeblendet.

Noch nicht als produktionsreif nachgewiesen:

- echtes CoreBluetooth-Board auf macOS und iOS/iPadOS; beide Hosts sind
  implementiert, aber noch nicht mit der realen Scheibe qualifiziert,
- reale AirPlay-, HDMI- und Audio-Hardware,
- External-Display-Scene-Accessory ab iOS/iPadOS 27,
- iPhone/iPad-zu-iPad-Companion mit Discovery, eingehendem Pairing-Transport,
  Keychain und Projector-Client,
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

Der gleiche Startparameter dient außerdem als Persistenz-Smoke-Test: App einmal
mit `--m0-test-hit-after-start` starten, nach Revision 2 vollständig beenden und
ohne Parameter neu öffnen. Score und Revision müssen erhalten bleiben, die
angezeigte Runtime-ID muss wechseln.

## Apple-DisplayHost

Die technische Entscheidung und ihre Versionsgrenzen stehen in
[ADR 0001](adr/0001-apple-external-display-host.md). Die Control UI zeigt den
aktuellen Status `nicht verbunden` oder die Zahl der aktiven AirPlay-/HDMI-
Displays. Das Projector-Fenster besitzt keinen schreibenden Runtime-Zugriff.

## Apple-BoardTransport

`BoardTransportHost.mm` wird für macOS, iOS und iPadOS als derselbe dünne
Plattformadapter gebaut und besitzt ausschließlich CoreBluetooth-Discovery,
Verbindung, State Restoration und FFF1-Subscription. Decoder, Button-/Miss-
Interpretation, Deduplizierung und Runtime-Dispatch liegen im gemeinsamen
Rust-`sdb-board`-Crate. Damit gilt derselbe Byte-Fixture-Test auf Linux, macOS,
iPhone und iPad.

Die Apple-Apps verlangen `NSBluetoothAlwaysUsageDescription`; iOS/iPadOS
deklariert zusätzlich `bluetooth-central` für Preservation/Restoration, und das
macOS-Bundle besitzt das Bluetooth-Geräte-Entitlement. Eine bekannte Apple-
Peripheral-UUID wird in `NSUserDefaults` gespeichert; ist sie nicht mehr
abrufbar oder scheitert die Verbindung, scannt der Host erneut nach Name
`SDB-BT` oder Service `FFF0`. Die UI zeigt den stabilen Boardstatus, aber keine
rohen Backendfehler als Spielmeldung.

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
