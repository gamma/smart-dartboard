# Plattformübergreifende Zielarchitektur

Stand: 2026-08-01

Status: **Normative Spezifikation für den Cross-Platform-Rewrite**

Dieses Dokument ist für die neue Runtime verbindlich. Die Dokumente
`APPLE_PORTABILITY.md` und `ANDROID_PORTABILITY.md` liefern Analysen,
Alternativen und Plattformdetails, dürfen diese Spezifikation aber nicht
überschreiben. Abweichungen benötigen eine dokumentierte Architecture Decision
Record (ADR) mit Begründung, Folgen und Rückfallpfad.

## 1. Priorität und Produktziel

Linux mit Docker bleibt der erste Installations-, Produktions- und Releasepfad.
Die Anwendung muss dort weiterhin lokal, offline und mit realem BLE-Dartboard
funktionieren.

Gleichzeitig wird der Kern so gebaut, dass später keine erneute fachliche
Portierung für iOS/iPadOS, macOS, Android oder Windows notwendig ist:

```text
ein Spielkern
  ├─ Linux/Docker-Server
  ├─ macOS-App
  ├─ Windows-App
  ├─ iOS-/iPadOS-App
  └─ Android-App
```

Plattformspezifisch bleiben ausschließlich Fähigkeiten, die das Betriebssystem
selbst vorgibt:

- Bluetooth,
- App-Lifecycle und Hintergrundbetrieb,
- Fenster und externe Displays,
- sichere Verzeichnisse und Dateidialoge,
- Kiosk-/Keep-Awake-Funktionen,
- Installation, Signierung und Updates.

Spielregeln, Sessionlogik, Undo, Recovery, Statistik, Datenmodell und
öffentlicher Zustand dürfen nicht pro Plattform neu implementiert werden.

Der normale Produktablauf lautet auf allen eigenständigen Geräten:

```text
App starten → Board einschalten → automatisch verbinden → spielen
```

Nach der einmaligen Einrichtung sind dafür weder Terminal, Benutzerkonto,
Internetverbindung noch manuelle IP-Adresse erforderlich.

## 2. Architekturentscheidung und Grenzen

Der langfristige gemeinsame Fachkern wird in plattformneutralem Rust gebaut.
Die bestehende Weboberfläche bleibt erhalten. Tauri 2 ist die bevorzugte native
Hülle für Desktop und Mobile; ein separates Headless-Server-Binary bleibt die
Linux-/Docker-Referenz.

```text
                         gemeinsamer Rust-Core
                Regeln · Session · Statistik · Runtime
                              · SQLite
                                  │
       ┌───────────────┬──────────┼──────────┬───────────────┐
       │               │          │          │               │
 Linux/Docker       macOS      Windows    iPadOS          Android
 Headless Server    Tauri       Tauri      Tauri            Tauri
 BLE Gateway        Apple BLE   WinRT BLE   Swift BLE     Kotlin BLE
 REST/WebSocket     Fenster     Fenster     Scenes        Presentation
       │               │          │          │               │
       └──────────── gemeinsame HTML/CSS/JS-Oberfläche ──────┘
```

Tauri 2 unterstützt Desktop- und Mobile-Ziele und erlaubt eigene native
Mobile-Plugins. Der Core darf dennoch weder von Tauri noch von einer WebView
abhängen. Dadurch bleibt er in Tests, im Headless-Server und bei einem späteren
Wechsel der nativen Hülle nutzbar. Siehe
[Tauri-Plattformvoraussetzungen](https://v2.tauri.app/start/prerequisites/) und
[Mobile Plugin Development](https://v2.tauri.app/develop/plugins/develop-mobile/).

### 2.1 Verbindliche und vorläufige Entscheidungen

Verbindlich sind:

- Rust als Sprache des gemeinsamen Fachkerns,
- eine serialisierte autoritative Runtime,
- SQLite als lokaler persistenter Store,
- dieselben versionierten Contracts für Server und native Apps,
- die bestehende Web-UI hinter einem austauschbaren Runtime-Transport,
- Linux/Docker als erster produktiver Releasepfad.

Tauri als native Hülle ist die bevorzugte, aber durch Milestone M0 noch zu
beweisende Entscheidung. Scheitert Tauri an externer Anzeige, WebView-Lifecycle
oder Store-Anforderungen, bleibt der Rust-Core erhalten und wird über eine
dünne Swift-/Kotlin- beziehungsweise andere native Hülle eingebunden. Ein
Scheitern der Hülle darf keinen erneuten Rewrite der Regeln auslösen.

### 2.2 Produkt-Topologien

Die Runtime unterstützt drei bewusst getrennte Topologien:

```text
Hosted Mode
  Board → Headless Runtime → SQLite → REST/WebSocket
                                    ├─ Control-Browser
                                    └─ Projector-Browser

Standalone Mode
  Board → lokale Runtime → SQLite → native IPC/Event-Transport
                                  ├─ Control-WebView
                                  └─ Projector-WebView

Companion Mode
  Board → lokale Runtime im Controller-Gerät → SQLite
                         ├─ IPC → lokale Control-WebView
                         └─ gepaarter Netzwerktransport → Projector-Gerät
```

Der Hosted Mode kann mehrere gepaarte Netzwerkclients bedienen. Im Standalone
Mode öffnet die App keinen HTTP-Port. Der Companion Mode ist ein gezielt
freigeschalteter lokaler Ein-Host-Betrieb: Das Controller-Gerät bleibt alleinige
Runtime-, BLE- und Datenbankinstanz; das zweite Gerät ist nur Projector-Client.
Alle drei Topologien verwenden dieselben Commands, Events, Zustände und
fachlichen Tests.

### 2.3 Bevorzugte iOS-/iPadOS-Anzeigemodi

Die Apple-App bietet zwei klar benannte Produktmodi. **Direkte Ausgabe** ist der
bevorzugte Arcade-Pfad: Ein beliebiges unterstütztes iPhone oder iPad ist
Controller, BLE-Host, Runtime und Datenbank. Es verbindet das Board direkt per
CoreBluetooth und gibt eine eigenständige Projector-Ansicht über AirPlay oder
einen kabelgebundenen HDMI-/USB-C-Displayadapter aus. Das interne Display bleibt
dabei immer die interaktive Control UI. Das externe Display zeigt nicht deren
Spiegelung, sondern ausschließlich die nichtinteraktive Projector-Rolle.

Der normale Start öffnet direkt diesen Controller-Modus. Nach dem Einschalten
des bekannten Boards verbindet sich die App automatisch; eine verfügbare,
zuletzt verwendete externe Ausgabe wird wiederhergestellt. Die Arcade-Kette
lautet damit im Idealfall: App starten, Board einschalten, spielen. AirPlay und
HDMI sind lediglich zwei Adapter desselben `DisplayHost` und dürfen weder eine
zweite Runtime noch unterschiedliche Spielregeln erzeugen.

Dieser Pfad ist der Standard und benötigt kein Companion-Pairing:

```text
iPhone oder iPad                         AirPlay oder HDMI/USB-C
Controller + BLE + Runtime + SQLite  ─────→  Projector + Sound
```

Die App zeigt auf dem internen Display immer die vollständige Bedienoberfläche.
Beim Verbinden eines externen Displays erscheint dort automatisch die
Projector-Ansicht. Ein laufendes Spiel, die Boardverbindung und die Session
bleiben beim Verbinden, Wechseln oder Trennen des Ausgabegeräts erhalten. Der
Nutzer muss dafür weder eine Adresse eingeben noch einen Server starten.

**Companion-Projektor** ist der zweite Produktmodus und benötigt zwei Geräte:

```text
iPhone oder iPad                              iPad
Controller + BLE + Runtime + SQLite  ──────→  Projector + Sound
```

Beide Geräte verwenden dieselbe App; es gibt keine gesonderte zweite
Companion-Anwendung. Beim Start wird die Rolle `Dieses Gerät` oder
`Companion-Projektor` gewählt; ein bereits eingerichtetes Gerät merkt sich die
Auswahl. Der normale Controller-Start verlangt kein Companion-Pairing und
bevorzugt ein bereits verfügbares AirPlay-/HDMI-Ziel. Der Controller wird im
Companion-Modus über Bonjour gefunden und per QR-Code oder
kurzem Einmalcode gepaart. QR bindet dabei den Fingerprint der lokalen
TLS-Identität; bei manueller Eingabe wird derselbe kurze Fingerprint auf beiden
Geräten bestätigt. Bonjour allein ist ausdrücklich kein Vertrauensanker. Es
gibt niemals zwei autoritative Runtimes für dasselbe laufende Spiel.

Der typische Companion-Fall ist ausdrücklich **iPhone als Controller und iPad
als Projector**. Auch ein iPad darf Controller sein, damit die Topologie nicht
an ein bestimmtes Apple-Gerätemodell gekoppelt wird. Das Projector-iPad zeigt
nach der einmaligen Kopplung beim nächsten Start nur eine große, einfache
Hostauswahl beziehungsweise verbindet sich automatisch mit dem zuletzt
verwendeten verfügbaren Controller. Es bietet keine lokale Spiel- oder
Boardsteuerung an.

Die Ausgabeauswahl liegt im Board-Setup und bietet `AirPlay/externes Display`,
`Companion-iPad` und `Vorschau auf diesem Gerät`. Die App stellt den zuletzt
erfolgreichen Pfad beim nächsten Arcade-Start automatisch wieder her und zeigt
Verbindung, Audioziel und Fallback eindeutig auf dem Controller. Ein AirPlay-
Empfänger, der nur Systemspiegelung zulässt, erfüllt den Projector-Modus nicht;
in diesem Fall verweist die App auf HDMI oder das Companion-iPad.

Reihenfolge der Produktpräferenz:

1. iPhone/iPad als Controller mit AirPlay oder HDMI als Projector,
2. iPhone/iPad als Controller mit iPad als Companion-Projector,
3. vorübergehende Projector-Vorschau auf demselben Display als Fallback.

Für den point-and-click-fähigen Arcade-Betrieb gelten zusätzlich folgende
UX-Kriterien:

- Ohne eingerichteten Companion startet die App immer als Controller und sucht
  das bekannte Board automatisch.
- AirPlay oder HDMI wird durch Anschließen beziehungsweise Auswählen des
  Ausgabegeräts aktiv; eine zusätzliche Netzwerk- oder Serverkonfiguration ist
  unzulässig.
- Ein bereits gekoppeltes Companion-iPad kann sich nach App-Neustart ohne neuen
  Einmalcode verbinden. Ein neuer Code ist nur für erstmaliges Pairing oder
  bewusstes Neu-Pairing erforderlich.
- Der Controller zeigt jederzeit knapp an, ob Projector und Sound lokal,
  über AirPlay/HDMI oder auf einem Companion-iPad ausgegeben werden.
- Fällt die Projector-Ausgabe aus, läuft das Spiel auf dem Controller weiter
  und die UI bietet den nächsten verfügbaren Ausgabepfad mit einem Klick an.

Für die Abnahme müssen mindestens diese Kombinationen denselben laufenden
Spielstand und dieselbe Projector-Darstellung liefern:

| Controller | Projector | Modus |
|---|---|---|
| iPhone | AirPlay-Empfänger | direkte Ausgabe |
| iPhone | HDMI-/USB-C-Adapter | direkte Ausgabe |
| iPad | AirPlay-Empfänger | direkte Ausgabe |
| iPad | HDMI-/USB-C-Adapter | direkte Ausgabe |
| iPhone oder iPad | zweites iPad | Companion-Projektor |

„Direkte Ausgabe“ gilt nur dann als verfügbar, wenn iOS/iPadOS eine separate
externe Anzeige bereitstellt. Reine Bildschirmspiegelung, bei der die Control UI
auf dem Projektor erscheint, ist kein zulässiger Arcade-Modus.

Der Spielbetrieb muss beim Trennen eines externen Displays oder Companion-
Geräts auf dem Controller weiterlaufen. Nach Wiederverbindung erhält der
Projector zuerst einen vollständigen Snapshot und danach nur lückenlose
Revisionen. Ein Wechsel zwischen AirPlay, HDMI und Companion verändert weder
die autoritative Runtime noch Session, Spielstand oder Board-Verbindung.

## 3. Warum Rust für den gemeinsamen Kern

### Vorteile

- derselbe kompilierte Fachcode auf Linux, macOS, Windows, iOS und Android,
- kein eingebetteter Python-, Node- oder JavaScript-Server auf Mobilgeräten,
- eine autoritative Runtime für mehrere Control-/Projector-Fenster,
- gute FFI-Grenzen zu Swift und Kotlin,
- Headless-Binary für Docker ohne GUI-Framework,
- statisch gebündelte Spielmodi entsprechen den Store-Vorgaben,
- kontrollierbarer Speicher- und Threading-Lifecycle,
- SQLite kann auf allen Zielplattformen dasselbe Schema verwenden,
- Regeln lassen sich als reine, deterministische Zustandsübergänge testen.

### Kosten

- einmalige vollständige Portierung des Python-Spielkerns,
- höhere Einstiegshürde für neue Regeln als bei Python oder TypeScript,
- mehr Build-Ziele und Cross-Compilation,
- native BLE- und Displayadapter müssen dennoch je Plattform gebaut werden,
- bestehende Python- und neue Rust-Implementierung laufen während der Migration
  parallel.

Dieser Rewrite wird einmal durchgeführt, damit nicht später für jede Plattform
ein weiterer fachlicher Rewrite entsteht.

## 4. Verworfene Hauptarchitekturen

### Python überall

Python ist für Linux und macOS geeignet, auf iOS und Android aber nur als
eingebettete Runtime verfügbar. BLE, Lifecycle und Packaging benötigen dort
trotzdem native Brücken. Python bleibt das Vergleichsorakel während der
Portierung, ist aber nicht der generische Endzustand.

### TypeScript als alleiniger Kern

TypeScript passt zur vorhandenen UI. Für eine autoritative Runtime außerhalb
einer WebView wären jedoch unterschiedliche Hosts nötig. Mehrere Fenster und
externe Projector-WebViews müssten ihren Zustand über zusätzliche
Runtime-Brücken teilen. Das ist möglich, aber weniger einheitlich als ein
nativer Rust-Core.

### SwiftUI oder Kotlin Multiplatform als Hauptkern

SwiftUI priorisiert Apple zu stark. Kotlin Multiplatform ist für Android und
Apple interessant, aber weniger natürlich für den Linux-Headless-Server,
Windows und die bestehende Weboberfläche. Beide bleiben mögliche Hüllen, nicht
der Produktkern.

### Electron

Electron ist für Desktop brauchbar, bietet aber keinen iOS-/Android-Pfad und
ist für den Headless-Docker-Server unnötig schwer. Ein späterer Steam-Build kann
die dann qualifizierte Desktop-Hülle verwenden.

## 5. Workspace-Struktur

```text
crates/
  contracts/          versionierte Commands, Events, Effects und Public State
  protocol/           FFF1-Decoder und Event Interpreter
  game-core/          Spieler, Turns, Undo, Recovery, Registry
  game-modes/         alle klassischen und Arcade-Modi
  statistics/         Sessionwertung, Heatmap, Replay, Empfehlungen
  storage/            Schema, Migrationen, Repository und Outbox
  runtime/            autoritativer Actor und Command-Verarbeitung
  server/             REST, WebSocket, Assets und Healthcheck
  board-api/          plattformneutraler BoardTransport-Vertrag

platforms/
  linux/              BLE-Gateway/BlueZ und Docker-Entry-Point
  apple/              CoreBluetooth und Display-Scenes
  windows/            WinRT BLE und Fensterintegration
  android/            Kotlin BLE und External Display

apps/
  tauri/              gemeinsame Desktop-/Mobile-Hülle

web/
  control.html
  projector.html
  static/
  runtime-client/     Hosted-, Tauri- und Test-Transport

fixtures/
  packets/
  games/
  sessions/
  replays/
  databases/

legacy/
  python/             zunächst unveränderte Referenzimplementierung
```

Die vorhandenen Dateien werden anfangs nicht nach `legacy/` verschoben. Die
neue Struktur entsteht daneben. Umbenennungen folgen erst, wenn der Rust-Server
produktiv gleichwertig ist.

## 6. Versionierte Contracts

`contracts` ist die einzige öffentliche Sprache zwischen UI, Server, nativer
Hülle und Runtime. JSON Schema wird aus den Rust-Typen erzeugt und in CI gegen
Beispiele sowie die TypeScript-Typen geprüft.

Jede Nachricht enthält mindestens:

```json
{
  "protocol_version": 1,
  "runtime_instance_id": "uuid",
  "message_id": "uuid",
  "revision": 1842,
  "kind": "state|command|event|error",
  "payload": {}
}
```

Regeln:

- `protocol_version` wird bei inkompatiblen Änderungen erhöht.
- `runtime_instance_id` ändert sich mit jeder neuen Runtime-Instanz.
- `message_id` beziehungsweise `command_id` ermöglicht Deduplizierung.
- `revision` steigt nach jedem committed State-Transition monoton.
- Ein neuer Subscriber erhält zuerst einen vollständigen Snapshot.
- Ein Client mit Lücke oder falscher Runtime-Instanz fordert einen Snapshot an.
- Unbekannte additive Felder werden toleriert; unbekannte Command-Arten nicht.
- Fehler besitzen stabilen Code, verständliche Meldung und optionale Details.
- Lokalisierte Texte sind keine fachlichen Vergleichswerte oder Fehlercodes.

Vor Milestone B wird entweder die heutige REST-/WebSocket-API als `v1`
inventarisiert oder ein explizites `v2` mit UI-Migration beschlossen. Dokumentiert
werden alle Routen, Schemas, Fehler, WebSocket-Envelopes, Exportformate,
Testmodusgrenzen und Healthcheck-Felder. „Bestehend oder neu“ ist kein gültiges
Abnahmekriterium.

## 7. Deterministischer Fachkern und Spielmodi

Der Fachkern ist eine deterministische Zustandsmaschine. Zeit, Zufall und IDs
werden ausschließlich über einen injizierten Kontext bezogen.

```rust
pub trait GameMode {
    fn metadata(&self) -> &GameMetadata;
    fn option_schema(&self) -> &OptionSchema;
    fn initialize(&self, ctx: &mut GameContext, options: &GameOptions);
    fn handle_throw(&self, ctx: &mut GameContext, event: &DartEvent);
    fn handle_action(&self, ctx: &mut GameContext, action: &GameAction);
    fn on_turn_started(&self, ctx: &mut GameContext);
    fn on_turn_ended(&self, ctx: &mut GameContext);
    fn on_round_ended(&self, ctx: &mut GameContext);
    fn public_view(&self, ctx: &GameContext) -> ModePublicState;
    fn result(&self, ctx: &GameContext) -> Option<GameResult>;
}

pub struct DeterministicServices {
    pub clock: Box<dyn Clock>,
    pub ids: Box<dyn IdGenerator>,
    pub random: Box<dyn RandomSource>,
}

pub struct Transition {
    pub accepted: bool,
    pub next_state: RuntimeState,
    pub public_events: Vec<PublicEvent>,
    pub effects: Vec<PlatformEffect>,
    pub persistence: PersistenceDelta,
}
```

Der Core kennt nicht:

- BlueZ, CoreBluetooth, BluetoothGatt oder WinRT,
- HTTP, WebSocket oder Tauri IPC,
- DOM oder WebView,
- konkrete Fenster, Displays oder App-Verzeichnisse,
- Sound-, Haptik- oder Betriebssystem-APIs.

Spielmodi bleiben eigenständige Module in einer statischen Build-Time-Registry.
Neue Modi verändern weder Server noch Plattformadapter oder UI-Transport.
Native Store-Builds laden keinen ausführbaren Code nach. Metadaten,
Anleitungen, Optionen, Overlay-Daten, Sounds und Artworkreferenzen sind
deklarativ, validiert und versioniert.

Ein Modus definiert zusätzlich:

- stabile Modus- und Ruleset-Version,
- validierbares Optionsschema mit Defaults,
- unterstützte Spieler-/Teamkonfiguration,
- Verhalten für Turn, Runde, Hold, Undo, Korrektur und Abbruch,
- Ergebnis- und Statistikauswertung,
- vollständigen öffentlichen Moduszustand für Control und Projector,
- referenzierte Assets und lokalisierte Anleitungen.

## 8. Autoritative Runtime und Persistenz

Pro Prozess existiert genau eine autoritative Runtime. Alle Commands,
BLE-Pakete, Lifecycle-Ereignisse und Timer werden über eine begrenzte,
serialisierte Actor-Queue verarbeitet. UI, BLE-Adapter und Display-Hosts dürfen
keine eigene schreibbare Kopie des Spielzustands besitzen.

### 8.1 Atomare Verarbeitung

Ein akzeptiertes Ereignis gilt erst nach erfolgreichem SQLite-Commit als
wirksam:

```text
Input mit command_id
  → Berechtigung, Version und Revision prüfen
  → Core-Transition und next_revision berechnen
  → Event + Snapshot/Delta + next_revision + Statistik + Effect-Outbox
      in genau einer SQLite-Transaktion schreiben
  → COMMIT
  → committed Revision und neuen State publizieren
  → committed Effects ausführen
```

Bei fehlgeschlagenem Commit werden weder State noch Effects publiziert. Ein
wiederholter `command_id` liefert das gespeicherte Ergebnis und erzeugt keinen
zweiten Wurf. Effects sind mindestens einmal ausführbar und müssen entweder
idempotent sein oder eine Effect-ID deduplizieren. Nicht ausgeführte committed
Effects dürfen nach einem Crash wiederaufgenommen werden; rein kosmetische
Animationen können ausdrücklich als verwerfbar markiert werden.

### 8.2 Recovery

- Nach Start wird ausschließlich ein committed Zustand geladen.
- Ein laufendes Spiel wird eindeutig als fortsetzbar oder abgebrochen markiert.
- Unvollständige SQLite-Transaktionen werden niemals fachlich rekonstruiert.
- Event-Journal und Snapshot besitzen Ruleset- und Schema-Versionen.
- Replay desselben Journals muss denselben fachlichen Zustand erzeugen.
- Korrektur und Undo werden als nachvollziehbare Events gespeichert und dürfen
  die Historie nicht still überschreiben.

### 8.3 SQLite und Migrationen

Die Runtime ist alleinige Eigentümerin der Datenbank. WebViews erhalten keinen
direkten SQL-Zugriff. Alle Plattformen verwenden dasselbe versionierte Schema.

Jeder produktive Upgradepfad benötigt:

- explizite, fortlaufende Migrationen,
- Backup und WAL-Checkpoint vor einer destruktiven Migration,
- Transaktion oder dokumentierte Wiederanlaufstrategie je Migration,
- Integritätsprüfung nach Migration,
- definiertes Verhalten für Downgrade und inkompatible Datenbanken,
- versionierte JSON-Importe und -Exporte,
- Tests mit anonymisierten Kopien realer Datenbanken,
- dokumentierte private App- beziehungsweise `/data`-Pfade.

Der portable Archivvertrag `schema_version: 2` ist inzwischen im gemeinsamen
Rust-Repository umgesetzt: Linux/Docker nutzen einen Same-Origin-POST, Native
einen ausschließlich dem Control-Fenster erlaubten Tauri-Befehl. Beide Grenzen
begrenzen auf 16 MiB und führen denselben validierten, atomaren Import aus.
Kollisionen überschreiben nie lokale Historie; nicht fortsetzbare aktive
Archivdatensätze werden explizit als unterbrochen übernommen.

Ein fehlgeschlagenes Upgrade darf die einzige Nutzerdatenbank nicht
überschreiben. Der vorherige Stand bleibt als wiederherstellbare Kopie erhalten.

## 9. Runtime-Client und Zustandsverteilung

Die Web-UI verwendet nur diesen abstrakten Vertrag:

```text
RuntimeClient.bootstrap()                 → vollständiger StateEnvelope
RuntimeClient.dispatch(command)           → CommandResult
RuntimeClient.subscribe(callback)         → unsubscribe
RuntimeClient.request_snapshot()          → StateEnvelope
RuntimeClient.close()
```

Implementierungen:

- `HostedRuntimeClient`: REST und WebSocket,
- `TauriRuntimeClient`: eingeschränkte Tauri-Commands und Events,
- `TestRuntimeClient`: In-Memory/Fake.

Die UI darf außerhalb dieses Moduls weder relative API-URLs noch WebSocket-URLs
oder Tauri-Aufrufe kennen. Control und Projector rendern ausschließlich
vollständige oder revisionsgeordnete öffentliche Zustände.

Beim Reload, Resume, Displaywechsel oder Verbindungsverlust gilt:

1. Runtime-Instanz und letzte Revision vergleichen.
2. Bei jeder Lücke vollständigen Snapshot laden.
3. Keine UI-Aktion lokal als erfolgreich annehmen, bevor sie bestätigt wurde.
4. Retry nur mit derselben `command_id` ausführen.
5. Veraltete oder nicht autorisierte Commands mit stabilem Fehlercode ablehnen.

## 10. Plattformadapter

### 10.1 `BoardTransport`

Der Boardadapter liefert rohe Pakete plus Empfangsmetadaten. FFF1-Decodierung,
Interpretation und fachliche Deduplizierung liegen im gemeinsamen Core.

```text
Unavailable
  → PermissionRequired | BluetoothOff
  → Scanning
  → Connecting
  → Discovering
  → Subscribing
  → Ready
  → Reconnecting
  → Ready | Unavailable
```

Der Vertrag definiert typisierte Status- und Fehlercodes sowie:

- Cancellation und Timeout jeder Operation,
- bestätigte Notification-Subscription vor `Ready`,
- begrenzte Paketqueue ohne stilles Verwerfen,
- exponentiellen Reconnect-Backoff mit Jitter und Obergrenze,
- Verhalten bei App-Suspendierung und Adapterwechsel,
- wechselnde Geräteadressen und ungültige gespeicherte Kennungen,
- Sequenz-Wrap, Board-Neustart, doppelte und schnelle Notifications,
- Besitz durch ein anderes Gerät,
- Diagnosemetadaten ohne Anzeige roher Backendfehler in der Spiel-UI.

Implementierungen:

- Linux erster Release: bestehender Bleak-Gateway als Baseline; ein direkter
  BlueZ-Adapter darf ihn später nach demselben Hardwaretest ersetzen,
- macOS/iPadOS: CoreBluetooth,
- Windows: Windows Runtime Bluetooth LE,
- Android: BluetoothGatt über Kotlin-Plugin,
- Tests: Replay-, Fault-Injection- und Fake-Board-Adapter.

Eine bekannte Gerätekennung beschleunigt den Start, ist aber nicht die einzige
Identität. Ist sie ungültig, sucht der Adapter erneut nach Name `SDB-BT` und
Service `FFF0`.

### 10.2 `DisplayHost`

- Linux/Browser: getrennte URLs `/control` und `/projector`,
- macOS/Windows: zwei Fenster und auswählbares Vollbilddisplay,
- iOS/iPadOS bis einschließlich Version 26: interaktive Hauptscene plus
  `windowExternalDisplayNonInteractive`-Scene für HDMI und AirPlay,
- iOS/iPadOS ab Version 27: registriertes nichtinteraktives External-Display-
  Scene-Accessory; der Vertrag bleibt identisch,
- iOS/iPadOS Companion: lokale Control-WebView auf dem Runtime-Gerät und
  gepaarte, nichtinteraktive Projector-WebView auf einem zweiten iPad,
- Android: Haupt-Activity plus `Presentation` auf externem Display.

Der Displayvertrag umfasst:

- stabile Rollen `control` und `projector`,
- Erkennung, Auswahl, Verbindung und Trennung externer Displays,
- Wiederherstellung der Projector-WebView aus dem aktuellen Snapshot,
- Displayprofil pro stabiler Identität, Auflösung und Orientierung,
- definierte Fallbackdarstellung ohne externes Display,
- klare Anzeige auf dem Controller, ob HDMI/AirPlay oder ein Companion aktiv
  ist,
- Keep-Awake und Kiosk nur während relevanter Betriebszustände,
- keine Spielzustandskopie im Displayadapter.

AirPlay und HDMI sind unterschiedliche Hardware-Abnahmepfade, obwohl sie
denselben Displayvertrag implementieren. Reines System-Mirroring ist kein
Produktmodus. Unterstützt ein konkretes Gerät oder ein Adapter keine erweiterte
Ausgabe, erklärt die App den Fallback und bietet Companion-Pairing an.

Der Companion-Transport verwendet die versionierten Runtime-Contracts über
eine lokale, authentisierte und verschlüsselte Verbindung. Bonjour dient nur
der Discovery. Pairing-Tokens liegen im Keychain, sind widerrufbar und binden
die Rolle `projector`; ein Companion darf keine Spiel-, Setup- oder BLE-Commands
dispatchen. Bei Revisionslücken, App-Resume oder Hostwechsel ist ein vollständiger
Snapshot Pflicht. Ein automatischer Hostwechsel während eines laufenden Spiels
ist nicht zulässig.

Der normative Pairing-, Rollen- und Revisionsvertrag einschließlich Threat
Model und Abnahmestand steht in
[COMPANION_PROTOCOL.md](COMPANION_PROTOCOL.md).

### 10.3 Audio und Effekte

Sound ist ein deklarativer `PlatformEffect` und wird logisch der
Projector-Rolle zugeordnet. Der Host entscheidet über Web Audio oder einen
nativen Audiodienst. Festgelegt werden:

- einmaliges Audio-Unlock nach Benutzerinteraktion,
- gewünschtes Ausgabegerät und Verhalten bei Gerätewechsel,
- Deduplizierung über Effect-ID,
- Lautstärke, Testton und globales Enable ausschließlich im Setup,
- stummer, aber spielfähiger Fallback bei Audiofehlern.

## 11. Native Machbarkeit vor dem Rewrite: M0

M0 ist ein technischer Spike, kein vorgezogener nativer Produktrelease. Er
verhindert, dass die größten Plattformrisiken erst nach vollständiger
Core-Portierung sichtbar werden.

Der Spike verwendet nur einen minimalen Rust-Zustand, einen simulierten Wurf
und höchstens ein reales BLE-Paket. Er portiert keine Spielmodi.

### iOS-/iPadOS-Abnahme

- Control-WebView auf dem internen Display eines iPhones und eines iPads,
- native nichtinteraktive Projector-Scene über HDMI auf echter Hardware,
- dieselbe Projector-Scene über AirPlay auf echter Hardware,
- eigenständiger Projector-Inhalt statt bloßer Bildschirmspiegelung,
- beide Views zeigen denselben Rust-Zustand,
- Projector abziehen und erneut verbinden ohne Zustandsverlust,
- ein CoreBluetooth-Paket erreicht exakt einmal die Runtime,
- Background/Foreground und Rotation ohne zweite Runtime,
- iPhone/iPad als Runtime-Controller mit iPad als gepaartem Projector,
- Companion-Discovery, Pairing, Rollenbegrenzung und Widerruf funktionieren,
- Companion-Verbindungsverlust stoppt weder Runtime noch laufendes Spiel,
- Companion-Reconnect beginnt mit Snapshot und setzt revisionsgenau fort,
- signierbarer TestFlight-Debugbuild.

### Android-Abnahme

- Control-WebView und native `Presentation` auf externem Display,
- eigenständiger Projector-Inhalt statt bloßer Bildschirmspiegelung,
- ein BluetoothGatt-Paket erreicht exakt einmal die Runtime,
- Display- und Activity-Neuerstellung ohne zweite Runtime,
- installierbarer signierter Debugbuild.

### M0-Entscheidung

- Sind beide Spikes erfolgreich, bleibt Tauri die native Standardhülle.
- Benötigt nur Display oder BLE nativen Code, wird ein enges Swift-/Kotlin-
  Plugin gebaut.
- Kann Tauri die erforderlichen Scenes/WebViews nicht zuverlässig hosten,
  bleibt Rust verbindlich und die betroffene Plattform erhält eine dünne native
  Hülle.
- Ergebnisse, unterstützte Versionen und bekannte Einschränkungen werden als
  ADR festgehalten.

## 12. Linux/Docker als erster Produktmeilenstein

```text
SDB-BT
  ↓ Bleak-Gateway, später optional direkter BlueZ-Adapter
Rust Runtime
  ↓
SQLite
  ↓
REST/WebSocket
  ↓
bestehende Control + Projector UI
```

Der neue Server muss denselben betrieblichen Vertrag erfüllen:

- Multi-Arch-Image für AMD64 und ARM64,
- persistentes `/data`-Volume,
- Healthcheck mit Runtime-, DB- und Adapterstatus,
- Betrieb ohne `privileged`,
- nur erforderlicher D-Bus-Zugriff für BlueZ/Bleak,
- Umgebungsvariablen für Boardname, Adresse, Port und Testmodus,
- vollständiger Offlinebetrieb,
- Recovery nach Prozess- und Hostneustart,
- eingefrorene API v1 oder dokumentierte versionierte Migration,
- reale Hardwaretests vor Ablösung des Python-Containers.

Erst wenn dieser Pfad mindestens die heutige Funktionalität und Stabilität
erreicht, beginnen produktive native Releases. M0 bleibt davon unberührt, weil
es nur die technische Machbarkeit prüft.

## 13. „Starten, Board einschalten, geht“

```text
Runtime startet
  ↓
BoardTransport prüft Berechtigung und Funkstatus
  ↓
bekanntes Board verbinden oder nach SDB-BT/FFF0 suchen
  ↓
FFF1 abonnieren und Subscription bestätigen
  ↓
Runtime meldet BOARD_READY
```

Die Einschaltreihenfolge ist egal. Nach der ersten Einrichtung erfolgen Scan,
Connect, Subscribe und Reconnect automatisch. Die normale UI zeigt nur:

- Board wird gesucht,
- Board wird verbunden,
- Board bereit,
- Board getrennt – Spiel pausiert,
- Bluetooth ist ausgeschaltet,
- Berechtigung fehlt,
- Board wird vermutlich von einem anderen Gerät verwendet.

Rohe Fehler bleiben in Diagnose und Logs. Ein laufendes Spiel nimmt keine
weiteren Würfe an, solange die Verbindung unklar ist.

Verbindliche Qualitätsziele für die erste Hardwarequalifizierung:

- null verlorene oder doppelte Treffer in 1.000 realen Testwürfen,
- drei schnelle Darts werden in richtiger Reihenfolge verarbeitet,
- Board-first und App-first verbinden ohne Menüeingriff,
- Reconnect nach Stromverlust ohne doppelte Wertung,
- Prozessabbruch an jeder Persistenzgrenze stellt den letzten committed Zustand
  wieder her,
- mindestens acht Stunden aktiver Soak-Test und 72 Stunden Bereitschaftstest,
- Projector hält auf der Zielhardware 60 fps für vorgesehene Animationen oder
  dokumentiert einen qualifizierten 30-fps-Fallback,
- konkrete Zeitbudgets für Cold Start, Connect und Reconnect werden nach M0 auf
  realer Zielhardware als Baseline festgeschrieben und in CI/HIL überwacht.

## 14. Golden Fixtures und Tests

Jede Golden Fixture läuft gegen Python und Rust. Das Fixture-Format ist
versioniert und enthält mindestens:

```text
fixture_schema_version
ruleset_version
seed
fixed_clock
deterministic_id_sequence
initial_state
raw_packets und/oder commands
expected_transitions
expected_public_states
expected_events
expected_persistence_delta
expected_final_snapshot
```

Verglichen werden:

- akzeptierte und verworfene Events,
- aktueller Spieler, Runde und Dartnummer,
- Scores, Marks und vollständiger Moduszustand,
- Gewinner, Ergebnisart und Statistikauswirkung,
- Undo-, Korrektur- und Recovery-Snapshot,
- persistierte Telemetrie und Journalreihenfolge,
- öffentliches Overlay, Anweisungen und deklarative Effects.

Kanonische JSON-Serialisierung definiert Feld-, Map-, Zahlen- und
Nullbehandlung. Uhrzeiten, UUIDs und Zufall stammen aus der Fixture. Lokalisierte
Texte werden über Schlüssel statt gerenderten Wortlaut verglichen.

Die Testpyramide umfasst:

- Unit- und Property-Tests des Fachkerns,
- Fuzzing von Decoder und Interpreter,
- Golden- und Differential-Tests Python gegen Rust,
- API-/IPC-Contract-Tests gegen dieselben Fälle,
- Datenbankmigrations-, Backup- und Crash-Injection-Tests,
- WebKit- und Chromium-UI-Tests,
- native Lifecycle-, Display- und Permission-Tests,
- reale Hardwaretests und Soak-Tests als Release-Gate.

## 15. Sicherheit und Datenschutz

Hosted und Standalone Mode besitzen getrennte Threat Models.

### Hosted Mode

- Browsermutationen bleiben same-origin oder benötigen ein widerrufbares,
  gepaartes Token.
- Kein global offenes CORS.
- Remote Control wird über QR-/Kurzcode gepaart.
- Testevents sind nur bei explizitem Testmodus und entsprechender Berechtigung
  verfügbar.
- Server bindet standardmäßig nur an die dokumentierte lokale Schnittstelle.

### Standalone/Tauri

Tauri-Capabilities werden pro Window-Label und Plattform explizit aktiviert:

| Rolle | Erlaubte Fähigkeiten |
| --- | --- |
| `projector` | Zustand empfangen, freigegebene Audio-/Displayeffekte |
| `control` | normale Spiel-, Session- und Navigationscommands |
| `settings` | BLE-Auswahl, Kalibrierung, Export und Diagnose |
| `test` | simulierte Treffer ausschließlich bei aktivem Testmodus |

Die Projector-WebView erhält keinen schreibenden Spielzugriff. Keine WebView
erhält direkten SQL-, Shell- oder uneingeschränkten Dateisystemzugriff.
Privilegierte Fenstererzeugung bleibt auf die native Hülle beschränkt. Remote
Webinhalte laufen nie in einer privilegierten WebView.

Für alle Distributionen gelten außerdem:

- restriktive Content Security Policy,
- vollständig validierte BLE-Pakete und Importe,
- signierte Builds und Updates,
- getrennte und geschützte Signing-/Updater-Schlüssel,
- Lockfiles, Dependency- und Lizenzprüfung, SBOM und bekannte
  Schwachstellenprüfung,
- strukturierte Logs ohne unnötige personenbezogene Daten,
- Diagnoseexport nur nach sichtbarer Benutzeraktion.

Siehe [Tauri Capabilities](https://v2.tauri.app/security/capabilities/) und
[Runtime Authority](https://v2.tauri.app/security/runtime-authority/).

## 16. Observability und Support

Die Runtime schreibt strukturierte, rotierende Logs mit:

- App-, Contract-, Ruleset- und Schema-Version,
- Runtime-Instanz und State-Revision,
- anonymisierbarer Session-/Game-/Event-ID,
- Boardzustand und typisiertem Adapterfehler,
- Migration, Recovery und Effect-Ausführung,
- Plattform-, OS- und Adapterversion.

BLE-Rohdaten, Spielernamen und Tokens werden standardmäßig nicht vollständig
protokolliert. Ein Diagnoseexport enthält Health-Snapshot, relevante redigierte
Logs, Versionen und Konfiguration, aber keine Datenbank, sofern dies nicht
separat bestätigt wird.

## 17. Toolchain- und Plattformmatrix

Vor Milestone B wird eine versionierte Buildmatrix committed. Sie fixiert:

- Rust Edition, MSRV und Cargo-Lockfile-Regel,
- Tauri-, Node- und Frontend-Buildversion,
- SQLite-Version und aktivierte Features,
- minimale macOS-, iPadOS-, Android- und Windows-Version,
- Android min/target SDK,
- unterstützte Linuxdistributionen und BlueZ-/D-Bus-Voraussetzungen,
- AMD64, ARM64, Apple Silicon und gegebenenfalls Intel,
- qualifizierte Tablets, Docks, Projektoren und Audioausgänge.

Ein Ziel gilt erst als unterstützt, wenn Build, Installation und relevante
Hardwaretests in dieser Matrix dokumentiert sind.

Die versionierte Quelle liegt in
[`PLATFORM_MATRIX.json`](PLATFORM_MATRIX.json); die lesbare Einordnung und der
lokale Drift-Check stehen in [`PLATFORM_MATRIX.md`](PLATFORM_MATRIX.md).

## 18. Branch-, CI- und Release-Regeln

- `main` bleibt bis zur Parität die produktive Python-/Docker-Version.
- Fehlerkorrekturen landen zuerst auf `main` und erhalten dort eine Fixture.
- Jeder übernommene Fix referenziert Python-Commit, Fixture und Rust-Commit.
- `main` wird mindestens wöchentlich in den Rewrite-Branch integriert; Konflikte
  werden nicht bis zum Ende gesammelt.
- Neue Modi während des Rewrites benötigen sofort versionierte Fixtures.
- Der Rewrite löscht keine funktionierende Produktionsstrecke.
- Plattformcode darf keine Spielregeln duplizieren.
- Kein nativer Produktrelease vor Linux-Parität und Hardwaretest.
- Kein Merge allein aufgrund struktureller Vollständigkeit.
- Der Docker-Wechsel benötigt Migrationstests mit mehreren anonymisierten
  Datenbankkopien, Backup-/Rollbacktest und reale Hardwarequalifizierung.

CI prüft pro relevantem Commit:

- Format, Lints, Unit-, Property- und Contract-Tests,
- Python-/Rust-Differential-Fixtures,
- Web-UI in mindestens Chromium und WebKit,
- DB-Migrationen vorwärts und Fehlerwiederherstellung,
- Linux AMD64/ARM64 Containerbuild,
- kompilierbare native Zielartefakte sobald der jeweilige Adapter beginnt,
- Dependency-, Lizenz- und Security-Audit.

Hardware-in-the-loop und signierte Paketbuilds dürfen als geschützte
Release-Workflows laufen, bleiben aber Pflicht vor Veröffentlichung.

## 19. Reihenfolge und Meilensteine

Die technische Risikoprüfung und die Produkt-Releasereihenfolge sind getrennt:

```text
M0 Native Feasibility: iOS/iPadOS + Android BLE/External Display Spikes
A  Contracts, API-Inventar, Buildmatrix und Golden-Fixture-Format
B  Rust Protocol + Core + CountUp/X01
C  atomare Runtime + Sessions + SQLite + Recovery
D  alle Spielmodi mit Parität
E  RuntimeClient + Rust REST/WebSocket-Server + bestehende UI
F  Linux-Docker-Image und Hardware-Langzeittest
G  macOS-App
H  iOS-/iPadOS-App mit AirPlay/HDMI und Companion-Projector
I  Android-App
J  Windows-App
K  optional Steam
```

M0 läuft vor oder parallel zu A und portiert keine Produktlogik. Die
Releasereihenfolge bleibt:

1. Linux/Docker,
2. macOS,
3. iOS/iPadOS,
4. Android,
5. Windows,
6. optional Steam.

Jeder Meilenstein benötigt dokumentierte Entry-/Exit-Kriterien, grüne CI und
die zu ihm gehörenden Fixtures. Ein Meilenstein ist nicht abgeschlossen, wenn
nur Strukturen oder Traits existieren.

## 20. Ergebnis

Die aktuelle Linux-/Docker-Anwendung bleibt das Produkt und wird nicht durch
eine frühe Apple- oder Mobile-Architektur verdrängt. Der neue Kern wird zuerst
als kompatibler Linux-Server produktiv bewiesen.

Der frühe M0-Spike prüft ausschließlich, ob BLE, WebViews und externe Displays
mit der vorgesehenen nativen Hülle zuverlässig zusammenspielen. Dadurch wird
die wichtigste Portabilitätsannahme bewiesen, bevor alle Regeln portiert sind.

Der gemeinsame Rust-Core, versionierte Contracts, atomare Persistenz und ein
austauschbarer Runtime-Client verhindern danach fachliche Plattform-Rewrites.
Unvermeidliche Unterschiede bleiben in kleinen, testbaren Adaptern für BLE,
Displays, Lifecycle, Audio und Packaging.

Für iOS ist dieser Lifecycle-Adapter inzwischen konkret umgesetzt: Suspend
sperrt neue Eingaben vor dem asynchronen Adapterabbau, stoppt BLE/Bonjour und
Companion-Verbindungen geordnet und verändert den committed Spielzustand
nicht. Resume startet rollenabhängig neu. Ein CI-Smoke-Test installiert die App
auf einem iPad-Simulator und weist Suspend/Resume bei identischer
Runtime-Revision in den redigierten Diagnoselogs nach. Reale
Hintergrundzeitlimits und Reconnect-Verhalten bleiben Teil der Hardwareabnahme.
