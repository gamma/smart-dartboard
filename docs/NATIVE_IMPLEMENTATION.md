# Native Implementierung

Stand: 2026-08-01

Dieses Dokument beschreibt den belegten Implementierungsstand des
Cross-Platform-Rewrites. Zielarchitektur und Abnahmekriterien stehen in
[CROSS_PLATFORM_ARCHITECTURE.md](CROSS_PLATFORM_ARCHITECTURE.md).

## Aktueller Stand

Umgesetzt und lokal verifiziert:

- Rust-Workspace mit Contracts, FFF1-Protokoll, CountUp-, X01-, Cricket-,
  8-Ball-, Heart-Chase-, Target-Rush- und Ghost-Chase-Kern, atomarer Runtime und
  SQLite-Repository,
- gemeinsame Golden Fixtures für Python und Rust; bei X01 einschließlich Bust,
  Double-out, Spielerwechsel, Wurfkorrektur, Löschen und Undo,
- statische, zur Buildzeit typsicher registrierte Spielmodi mit versionierten
  Metadaten, validierten Optionen, Anleitungen, Artwork-/Sound-Referenzen und
  Projector-Overlay. `GET /api/v2/modes` liefert derzeit CountUp, X01, Cricket,
  8-Ball, Heart Chase, Target Rush, Ghost Chase, Robin Hood und Candy Cannon.
  Cricket, 8-Ball, Heart Chase, Target Rush, Ghost Chase, Robin Hood und Candy
  Cannon laufen über
  denselben generischen Registry-Pfad
  und sind durch gemeinsame Python-/Rust-Golden-Fixtures belegt;
  Cricket zusätzlich durch den vollständigen SQLite-Recovery-Pfad,
- injizierter, plattformneutraler Zufallskontext für Registry-Modi. Reguläre
  Sessions leiten den Seed stabil aus der persistierten Spiel-ID ab; Seed und
  Cursor gehören zum Runtime-Snapshot. Damit reproduzieren Recovery, Undo und
  Wurfkorrektur dieselbe Arcade-Zielsequenz. Golden-Fixtures können einen Seed
  direkt vorgeben,
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
- Öffentliche Commands zum Korrigieren und Löschen über stabile Action-IDs für
  CountUp, X01 und alle Registry-Modi, derzeit Cricket, 8-Ball, Heart Chase,
  Target Rush, Ghost Chase, Robin Hood und Candy Cannon. Der gemeinsame State
  liefert die
  editierbaren
  Darts der aktuellen und vorherigen Aufnahme. Replay,
  Sessionpunkte, Gewinner, kanonische `throws` und append-only
  Korrekturevents werden atomar synchronisiert. Bereits mit den ersten
  CountUp-/Registry-Snapshotformaten gespeicherte Spiele bleiben lesbar und
  wechseln bei der nächsten Aktion auf die neue Timeline.
- `ContinueTurn` bestätigt ausschließlich eine bereits abgeschlossene Aufnahme;
  `NextPlayer` beendet dagegen einen laufenden Teilzug bewusst. Beide Grenzen
  sind in CountUp, X01 und Registry-Modi getrennt replaybar. Ein Skip bewahrt
  bereits geworfene Darts, führt modusspezifische Skip-Regeln aus und kann die
  letzte Aufnahme eines rundenbasierten Spiels regulär werten. SQLite führt ihn
  als eigenes digitales `next_player`-Ereignis.
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
  Der native Apple-Host stellt denselben Projector-Stream inzwischen über
  HTTPS/WSS mit der lokalen TLS-Identität bereit und veröffentlicht den nur bei
  gewähltem Companion-Ausgang aktiven Port per Bonjour. Der Projector-Client
  kann den Host auswählen, sicher koppeln und den laufenden Zustand über
  Bootstrap und WebSocket revisionsgenau wiedergeben. Details:
  [COMPANION_PROTOCOL.md](COMPANION_PROTOCOL.md).
- persistente lokale Apple-TLS-Identität für Companion-Pairing. Zertifikat und
  privater P-256-Schlüssel liegen als gebundener Datensatz im Keychain; SQLite
  speichert nur die nicht geheime Host-ID. Nach Verlust der App-Daten stellt
  die Keychain-Identität diese Host-ID wieder her, statt unbemerkt eine neue
  Identität zu erzeugen. Das temporäre Rust-Blob und der native
  `malloc`-Übergabepuffer mit Schlüsselmaterial werden beim Freigeben
  überschrieben. Der Pairing-Bootstrap verwendet den
  echten SHA-256-Zertifikat-Fingerprint. Identität und Fingerprint blieben bei
  wiederholtem Start in der macOS-App stabil. Der lokal getestete nicht
  signierte iOS-Simulator-Build erhielt keinen nutzbaren Keychain-Zugriff; die
  App startet in diesem Fall weiter, markiert Companion als nicht verfügbar und lässt die
  bevorzugte AirPlay-/HDMI-Ausgabe benutzbar. Keychain-Persistenz auf einem
  signierten realen iPhone/iPad ist noch abzunehmen.
- nativer, nur bei ausgewähltem Companion-Ausgang laufender HTTPS/WSS-Listener
  mit tatsächlichem TLS-Handshake-Test, Bootstrap-Snapshot, lückenlosem
  WebSocket-Stream, Origin-Ablehnung, Bearer-Authentisierung und sofortigem
  Schließen nach Widerruf. Der Apple-Host veröffentlicht Port, Host-ID,
  Protokollversion und TLS-Merkmal als `_sdb-darts._tcp` über Bonjour. Ein
  Fehler in Keychain oder TLS deaktiviert ausschließlich Companion und darf
  Controller, BLE, Runtime oder direkte Displayausgabe nicht verhindern.
- nativer Apple-Bonjour-Browser mit explizitem Lebenszyklus und einer
  größenbegrenzten JSON-FFI-Grenze. Er akzeptiert nur lokale DNS-Namen,
  UUID-Host-IDs und TLS-markierte Dienste. Der macOS-Test startet Listener und
  Advertiser, entdeckt den Dienst wirklich per DNS-SD und gleicht Host-ID,
  Protokollversion und dynamischen Port ab. Die Symbole sind auch im
  iOS-Simulator-Bundle exportiert.
- persistierte, point-and-click-fähige Rollenwahl in derselben Apple-App. Die
  Standardrolle `Dieses Gerät` behält Runtime, Board und direkte
  Projector-Ausgabe. `Companion-Projektor` startet bei einer frischen
  Installation kein lokales Spiel, lehnt mutierende Runtime-, Board-, Setup-
  und Host-Pairing-Pfade serverseitig ab, stoppt CoreBluetooth und den
  HTTPS/WSS-Host und zeigt ausschließlich die Discovery-Ansicht. Der Rückweg
  startet CoreBluetooth wieder. Persistenz und Autoritätsgrenze sind per Rust
  getestet; Layout und Neustart wurden im iPhone-17-Pro-Simulator geprüft.
- sicherer nativer Companion-Client für Apple: Ein im Bonjour-Browser
  ausgewählter Controller wird zunächst nur per TLS-Handshake geprüft, ohne
  Anmeldedaten zu senden. Erst nach sichtbarem Vergleich des kurzen
  SHA-256-Fingerprints und Eingabe des sechsstelligen Einmalcodes wird eine
  zweite, auf das bestätigte Zertifikat gepinnte TLS-Verbindung aufgebaut. Der
  zurückgegebene Projector-Grant wird auf Geräte-ID und Rolle geprüft und mit
  Zertifikat und Host-ID im Apple Keychain gespeichert; SQLite enthält nur die
  nicht geheime Geräte- und Hostpräferenz. Temporäre Antwort-, Speicher- und
  Tokenpuffer werden nach Gebrauch überschrieben. Ein Integrationstest koppelt
  gegen den echten nativen HTTPS-Host und belegt, dass ein falsches Pinning den
  Code nicht einlösen kann. Die point-and-click-fähige WebKit-UI wurde mit
  Playwright geprüft; der Kopplungsbutton bleibt bis zum bestätigten
  Fingerprint und vollständigen Code gesperrt.
- nativer Apple-Projector-Client mit authentisiertem Bootstrap-Snapshot und
  TLS-gepinntem WebSocket. Jeder Frame läuft durch den gemeinsamen
  `ReplicaCursor`; Duplikate werden ignoriert, Revisionslücken, Runtimewechsel
  ohne Snapshot und ungültige Payload-Metadaten erzwingen einen vollständigen
  Reconnect. Der Host prüft die Verbindung mit Heartbeats. Bei Unterbrechung
  blendet die UI den alten Spielstand aus und zeigt einen neutralen
  Reconnect-Zustand; nach Widerruf fordert sie neues Pairing. Ein realer
  In-Process-TLS-Test belegt Bootstrap, initialen WebSocket-Snapshot, den
  nächsten Runtime-Zustand, sofortiges Schließen nach Widerruf und die folgende
  Auth-Ablehnung. Derselbe UI-Lifecycle ist mit Playwright/WebKit geprüft.
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
- vollständige iPhone/iPad-zu-iPad-Hardwareabnahme des implementierten
  Companion-Pfads einschließlich App-Resume, realem WLAN-Verlust, Sound und
  Projector-Animationen,
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

Companion-UI mit WebKit:

```bash
npm --prefix apps/tauri run test:webkit
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
