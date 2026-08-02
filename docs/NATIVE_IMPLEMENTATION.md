# Native Implementierung

Stand: 2026-08-02

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
  8-Ball, Avoid the Bomb, Color Clash, Heart Chase, King of the Board, Target
  Rush, Ghost Chase, Risk It, Robin Hood, Candy Cannon, Lightning Round, Mini
  Golf, Simon Says, Treasure Hunt, Block Drop, Dragon Eggs, Cookie Monster und
  Space Defender, DartSweeper, Darts Bingo sowie Boss Fight V1.
  Cricket, 8-Ball, Avoid the Bomb, Color Clash, Heart Chase, King of the Board,
  Target Rush, Ghost Chase, Risk It, Robin Hood, Candy Cannon, Lightning Round,
  Mini Golf, Simon Says, Treasure Hunt, Block Drop, Dragon Eggs, Cookie Monster
  sowie Space Defender, DartSweeper, Darts Bingo und Boss Fight V1 laufen über
  denselben generischen Registry-Pfad und sind durch gemeinsame
  Python-/Rust-Golden-Fixtures belegt;
  Cricket zusätzlich durch den vollständigen SQLite-Recovery-Pfad,
- Avatar und frei gewählte Spielerfarbe werden aus der Session in native
  Registry-Spiele übernommen und bleiben im serialisierten Spielzustand
  erhalten. Alte Registry-Snapshots ohne diese Felder erhalten sichere
  Standardwerte. King of the Board verwendet dadurch auf der Scheibe exakt die
  gewählte Farbe des jeweiligen Spielers,
- injizierter, plattformneutraler Zufallskontext für Registry-Modi. Reguläre
  Sessions leiten den Seed stabil aus der persistierten Spiel-ID ab; Seed und
  Cursor gehören zum Runtime-Snapshot. Damit reproduzieren Recovery, Undo und
  Wurfkorrektur dieselbe Arcade-Zielsequenz. Golden-Fixtures können einen Seed
  direkt vorgeben,
- vorzeichenbehaftete Arcade-Scores in Registry-State, Dart-Replay und
  SQLite-Historie. Dadurch bleiben Modusregeln mit echten Minuspunkten über
  Snapshot, Korrektur, Recovery und API unverändert erhalten,
- semantischer `HostedRuntimeClient` für REST/WebSocket sowie eingeschränkter
  `TauriRuntimeClient` für native Commands und Events. Der Headless-Rust-Host
  liefert die bestehende Control-/Projector-Produkt-UI aus; der Experience-
  Adapter übersetzt deren Aktionen ohne modusspezifische Transportzweige in
  Runtime-v2-Commands und fällt bei einem Python-Host sauber auf dessen API
  zurück. Das native Control-Fenster und das macOS-Projector-Fenster laden
  dieselbe Produkt-UI und denselben Experience-Adapter wie Linux/Docker. Das
  separate Geräte-/Companion-Setup bleibt als schmale native Hostoberfläche
  erhalten. Board-, Reconnect- und Displaystatus fließen über einen getrennten
  read-only Host-Eventkanal live in die Produkt-UI,
- macOS-Tauri-App mit Control- und Projector-Fenster,
- iOS-/iPadOS-Tauri-App für `aarch64-apple-ios-sim`,
- nativer Apple-DisplayHost mit eigener Projector-WKWebView auf `TVOut`; sie
  lädt die gemeinsame `projector.html` samt Artworks aus dem eingebetteten
  Tauri-Asset-Resolver und verwendet denselben Runtime-v2-Adapter,
- Live-State-Verteilung sowie Disconnect/Reconnect ohne Zustandsverlust,
- Apple-M0 verwendet für den Testtreffer keinen Demo-Zähler mehr: Ein
  kanonisches `T20`-Event läuft durch den gemeinsamen CountUp-Core und die
  revisionsgesicherte Runtime. Control und Projector zeigten im
  iPad-Pro-Simulator übereinstimmend Score 60 bei Revision 2.
- Rust-Headless-Server mit expliziter API v2, SQLite-Recovery, idempotenten
  Command-Envelopes, Snapshot-WebSocket und nicht privilegiertem
  Vorschaucontainer. Details: [RUST_SERVER_V2.md](RUST_SERVER_V2.md).
- derselbe UI-Kernfluss wurde mit WebKit gegen den echten Rust-Host geprüft:
  Spieler `Ada` anlegen, Session starten, alle 24 Moduskarten laden, CountUp
  beginnen, im Projector T20 auslösen und im Control synchron Score 60 sehen.
  Testwürfe sind im Produktionsbetrieb verborgen und serverseitig gesperrt;
  der Testhost muss sie ausdrücklich freischalten,
- öffentliche Live-Snapshots enthalten ausschließlich den benötigten Spiel-,
  Session- und Setupzustand. Interne Initialzustände, Replay-Aktionen und Historien
  bleiben im Runtime-/Storage-Layer,
- Kalibrierung, Projector-Geometrie, Soundziel/-status, Artwork-Theme, Sprache
  und Korrektursperre gehören zum gemeinsamen, revisionsgesicherten Snapshot
  statt zu Browser-`localStorage`. Änderungen sind damit über Reload, Restart,
  Control und Projector konsistent. Die Kalibrieransicht wird als temporärer
  Display-Override synchronisiert, ohne den darunter laufenden Session-Screen
  zu überschreiben. Nach einem Crash wird eine Korrektursperre bewusst gelöst
  und ein aktivierter Soundausgang wieder in `starting` überführt,
- plattformweiter `PlatformEffect`-Vertrag für Sound- und visuelle Treffer-Cues.
  Stabile Effect-IDs, Zielrolle, erzeugende Revision und Lieferklasse werden
  zusammen mit Runtime, Journal, Historie und Statistik atomar in der
  SQLite-Outbox committed. Sound-Cues bleiben nach einem Crash innerhalb ihrer
  erzeugenden Revision bis zur bestätigten Ausführung wiederaufnehmbar;
  kurzlebige visuelle Cues werden weder aus einem Prozessneustart rekonstruiert
  noch beim späteren Anstecken, Reload oder Reconnect eines Displays wiederholt.
  Beide verfallen spätestens mit einer neueren Revision. Control,
  Browser-Projector, macOS-Projector,
  iOS-External-Display und Companion bestätigen ausschließlich die ihnen
  zugeordnete Rolle. Die UI dedupliziert über die Effect-ID und leitet einen
  alten `last_event` bei anderen Zustandsänderungen nicht erneut als Treffer
  weiter,
- versionierter Teamvertrag im gemeinsamen Core. Jeder Modus deklariert sein
  Format als `individual`, `cooperative` oder `teams`. Boss Fight, Space
  Defender, Block Drop Darts und DartSweeper bilden beim Auswählen automatisch
  genau ein Koop-Team aus allen Session-Spielern; dessen stabile Team-ID und
  Mitglieder laufen durch Session-, Game- und Public State. Ein Teamsieg zählt
  weiterhin atomar als Sieg und drei Sessionpunkte für jedes Mitglied. Explizite
  gegnerische Teamaufteilungen werden bereits als vollständige, disjunkte
  Spielerpartition validiert. In der gemeinsamen Control UI lassen sich zwei
  feste Teams bereits bei der Spielerauswahl point-and-click zusammenstellen;
  ein Klick auf einen Spieler verschiebt ihn zur Gegenseite, und leere Teams
  blockieren den Sessionstart. Die Konfiguration bleibt in der Modusauswahl
  sichtbar und kann von künftigen `teams`-Modi ohne neuen Transportpfad
  verwendet werden,
- das native Projector-Fenster und der iOS-External-Display-Host erhalten genau
  eine begrenzte Reporting-Bridge: Sie dürfen ihre Geometrie und ihren
  Soundstatus melden, aber weder Kalibrierung noch Spiel oder sonstiges Setup
  verändern. Debug-Builds besitzen zusätzlich einen dedizierten, auf
  `projector_test` beschränkten Testwurf-Command; Release-Builds lehnen ihn auch
  bei vorhandener Window-Berechtigung ab. Diese Autoritätsgrenze ist per
  Rust- und WebKit-Test belegt,
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
- einmaliger, wiederanlaufbarer Import bestehender Python-Schema-2-Datenbanken
  für den Linux-/Docker-Wechsel. Fehlt `runtime.sqlite`, wird `dartboard.db`
  vollständig validiert, per SQLite-Online-Backup unverändert gesichert und nur
  eine Arbeitskopie bis Schema 6 migriert. Profile, Historie, Würfe,
  Auditereignisse und Setup-Präferenzen bleiben erhalten; inkompatible laufende
  Altspiele werden in der Kopie ausdrücklich als unterbrochen markiert. Eine
  vollständige anonymisierte Python-v2-Fixture belegt Inhalt, Backup,
  Integrität, Settings-Übernahme und idempotenten Neustart,
- Transaktionale Historienprojektion für Profile, Sessions, Spiele, Würfe,
  Gewinner und Endstände. Nur beendete Produktionsspiele fließen in die neue
  Spielerstatistik ein; Undo macht Wurf und Sieg unwirksam, ohne das
  Auditereignis zu löschen. Die ersten read-only API-v2-Endpunkte liefern
  Profile, Sessionhistorie und Spielerstatistiken.
- Gemeinsamer Dart-Source-Contract für Board, Projektor-Test und manuelle
  Korrektur. Der Apple-Testtreffer ist als `projector_test` markiert; solche
  Spiele bleiben dauerhaft aus der normalen Statistik ausgeschlossen.
- Öffentliche Commands zum Korrigieren und Löschen über stabile Action-IDs für
  CountUp, X01 und alle Registry-Modi, derzeit Cricket, 8-Ball, Avoid the Bomb,
  Color Clash, Heart Chase, King of the Board, Target Rush, Ghost Chase, Risk
  It, Robin Hood, Candy Cannon, Lightning Round, Mini Golf, Simon Says,
  Treasure Hunt, Block Drop, Dragon Eggs, Cookie Monster, Space Defender,
  DartSweeper, Darts Bingo und Boss Fight V1. Der
  gemeinsame State liefert die editierbaren Darts der aktuellen und vorherigen
  Aufnahme. Replay,
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
- rotierende, redigierte JSONL-Diagnosen im nativen App-Datenverzeichnis unter
  `logs/`. Der Diagnoseexport ist nur aus den Control-Einstellungen aufrufbar
  und enthält weder Datenbank noch Spielernamen, Tokens oder BLE-Rohpakete.
- portabler JSON-Import über denselben Rust-Storage wie Linux/Docker. Nur das
  Control-Fenster besitzt den Tauri-Befehl; Projector und Companion haben keine
  Importberechtigung. Die 16-MiB-Grenze, vollständige Vorabvalidierung,
  Kollisionserkennung, Transaktion und Integritätsprüfung gelten auf Desktop
  und iOS identisch. Die gemeinsame Web-UI bietet Dateiauswahl und Bestätigung.
- expliziter iOS-App-Lifecycle: `applicationWillResignActive` wird über Tauri
  als Suspend verarbeitet, sperrt sofort weitere Dart-/Companion-Commands und
  stoppt BLE, Bonjour, Companion-Replikation sowie den lokalen TLS-Host
  serialisiert. Beim Resume starten ausschließlich die Adapter der gewählten
  Controller- oder Projector-Rolle neu. Runtime-Revision und laufendes Spiel
  bleiben dabei committed und unverändert. Doppelte Lifecycle-Events sind
  idempotent; Diagnoseereignisse machen Suspend und Resume nachvollziehbar.
- expliziter macOS-System-Lifecycle: Ein dünner AppKit-Host übersetzt
  `NSWorkspaceWillSleepNotification` und `NSWorkspaceDidWakeNotification` in
  denselben serialisierten Lifecycle-Pfad. Dadurch werden BLE, Bonjour,
  Companion-Replikation und lokaler TLS-Host vor dem Ruhezustand gestoppt und
  nach dem Aufwachen rollenabhängig neu gestartet, ohne Runtime-Revision oder
  Spielstand zu verändern. Der Release-App-Build belegt die native Verlinkung;
  echter System-Ruhezustand mit verbundener Scheibe bleibt Hardwareabnahme.

Noch nicht als produktionsreif nachgewiesen:

- echtes CoreBluetooth-Board auf macOS und iOS/iPadOS; beide Hosts sind
  implementiert, aber noch nicht mit der realen Scheibe qualifiziert,
- reale AirPlay-, HDMI- und Audio-Hardware,
- External-Display-Scene-Accessory ab iOS/iPadOS 27,
- vollständige iPhone/iPad-zu-iPad-Hardwareabnahme des implementierten
  Companion-Pfads einschließlich App-Resume, realem WLAN-Verlust, Sound und
  Projector-Animationen,
- das adaptive Boss Fight V2 als getrennte, ausdrücklich zurückgestellte
  Produktänderung; alle 24 heutigen Produktmodi sowie Historie, Replay,
  Heatmap, Modusstatistiken, Export und Trainingsempfehlungen sind im Rust-Pfad
  portiert,
- vollständige Docker-Bedien- und Hardwareparität zur Python-Anwendung; der
  Session-/Spielkernfluss, die statischen Oberflächen und der sichere
  Bestandsdatenimport sind bereits belegt,
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

Gemeinsame Produkt-UI gegen Runtime v2 mit WebKit:

```bash
npm --prefix apps/tauri run test:rust-ui
```

macOS-App:

```bash
npm --prefix apps/tauri ci
npm --prefix apps/tauri run build
npm --prefix apps/tauri run build:macos:app
```

`build` erzeugt weiterhin schnell nur das lokale Release-Binary.
`build:macos:app` erzeugt reproduzierbar ein bewusst unsigniertes
`Smart Dartboard.app`. CI archiviert dieses Bundle als kurzlebige Vorschau.
Es ist kein öffentlich verteilbares Release, solange Developer-ID-Signatur,
Notarisierung und Stapling fehlen.
macOS und iOS verwenden dabei bereits die dauerhafte Produktkennung
`de.gammaproduction.smart-dartboard`; die frühere Spike-Kennung mit `.m0`
wird nicht in künftige Installationen oder Store-Einträge übernommen.

iOS-Simulator-Build:

```bash
npm --prefix apps/tauri run ios:build:sim
npm --prefix apps/tauri run test:ios:lifecycle
```

Das Script entfernt vorab nur das generierte Simulator-Archiv und das bereits
exportierte `.app`-Bundle. Das ist nötig, weil Tauri 2.11.4 beim wiederholten
Simulator-Build ein vorhandenes Exportziel nicht selbst ersetzt.
Der zweite Befehl installiert das Bundle auf einem verfügbaren iPad-Simulator,
schickt die App durch Hintergrund und Vordergrund und belegt über die
redigierten Diagnoselogs, dass `app_suspended` und `app_resumed` dieselbe
Runtime-Revision behalten. Ein eigens gebooteter Simulator wird anschließend
wieder heruntergefahren.

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
Displays. Die Projector-WebViews besitzen keinen allgemeinen schreibenden
Runtime-Zugriff; nur Geometrie, Soundstatus und Debug-Testwürfe passieren die
explizit geprüfte Bridge.

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
