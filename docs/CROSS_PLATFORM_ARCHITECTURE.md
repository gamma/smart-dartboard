# Plattformübergreifende Zielarchitektur

Stand: 2026-08-01

## 1. Priorität und Ziel

Linux mit Docker bleibt der erste Installations-, Produktions- und
Releasepfad. Die Anwendung muss dort weiterhin lokal, offline und mit realem
BLE-Dartboard funktionieren.

Gleichzeitig soll der Kern so gebaut sein, dass später keine erneute fachliche
Portierung für iPadOS, macOS, Android oder Windows notwendig ist:

```text
ein Spielkern
  ├─ Linux/Docker-Server
  ├─ macOS-App
  ├─ Windows-App
  ├─ iPadOS-App
  └─ Android-App
```

Plattformspezifisch bleiben nur Fähigkeiten, die das Betriebssystem selbst
vorgibt:

- Bluetooth,
- App-Lifecycle und Hintergrundbetrieb,
- Fenster und externe Displays,
- sichere Verzeichnisse und Dateidialoge,
- Kiosk-/Keep-Awake-Funktionen,
- Installation, Signierung und Updates.

Spielregeln, Sessionlogik, Undo, Recovery, Statistik, Datenmodell und
öffentlicher Zustand dürfen nicht pro Plattform neu implementiert werden.

## 2. Entscheidung

Für einen vollständigen, langfristig generischen Rewrite wird ein
plattformneutraler Rust-Core empfohlen. Die bestehende Weboberfläche bleibt
erhalten. Tauri 2 dient später als native Hülle für Desktop und Mobile; ein
separates Headless-Server-Binary bleibt die Linux-/Docker-Referenz.

```text
                         gemeinsamer Rust-Core
                    Regeln · Session · Statistik
                              · SQLite
                                  │
       ┌───────────────┬──────────┼──────────┬───────────────┐
       │               │          │          │               │
 Linux/Docker       macOS      Windows    iPadOS          Android
 Headless Server    Tauri       Tauri      Tauri            Tauri
 BlueZ Adapter      Apple BLE   WinRT BLE   Swift BLE     Kotlin BLE
 REST/WebSocket     Fenster     Fenster     Scenes        Presentation
       │               │          │          │               │
       └──────────── gemeinsame HTML/CSS/JS-Oberfläche ──────┘
```

Tauri 2 unterstützt Desktop- und Mobile-Ziele und erlaubt eigene native
Mobile-Plugins. Der Core darf dennoch nicht von Tauri abhängen. Dadurch bleibt
er auch in Tests, im Headless-Server und in zukünftigen Integrationen nutzbar.
Siehe [Tauri-Plattformvoraussetzungen](https://v2.tauri.app/start/prerequisites/)
und
[Mobile Plugin Development](https://v2.tauri.app/develop/plugins/develop-mobile/).

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
- bestehende Python- und neue Rust-Implementierung laufen während der
  Migration parallel.

Dieser große Rewrite wird einmal durchgeführt, damit nicht später für jede
Plattform ein weiterer großer Rewrite entsteht.

## 4. Warum nicht eine plattformspezifische Hauptarchitektur

### Python überall

Python ist für Linux und macOS sehr gut, auf iOS und Android aber nur als
eingebettete Runtime verfügbar. BLE, Lifecycle und Packaging benötigen dort
trotzdem native Brücken. Binäre Abhängigkeiten erhöhen das Risiko. Python
bleibt die Referenz während der Portierung, ist aber nicht der generische
Endzustand.

### TypeScript als alleiniger Kern

TypeScript passt gut zur vorhandenen UI und ist für Regeln angenehm. Für eine
autoritative Runtime außerhalb einer WebView wären jedoch unterschiedliche
Hosts nötig: Node im Server, JavaScriptCore auf Apple und eine weitere Engine
auf Android. Mehrere Fenster und externe Projector-WebViews müssten ihren
Zustand über zusätzliche Runtime-Brücken teilen. Das ist möglich, aber weniger
einheitlich als ein nativer Rust-Core.

### SwiftUI oder Kotlin Multiplatform als Hauptkern

SwiftUI priorisiert Apple zu stark. Kotlin Multiplatform ist für Android und
Apple interessant, aber weniger natürlich für den Linux-Headless-Server,
Windows und die bestehende Weboberfläche. Beide wären gute Plattformhüllen,
nicht der generische Produktkern.

### Electron

Electron ist für Desktop brauchbar, bietet aber keinen iOS-/Android-Pfad und
ist für den Headless-Docker-Server unnötig schwer. Ein späterer Steam-Build kann
Tauri verwenden.

## 5. Workspace-Struktur

Vorgeschlagene Struktur auf einem separaten Rewrite-Branch:

```text
crates/
  contracts/          versionierte Actions, Events und Public State
  protocol/           FFF1-Decoder und Event Interpreter
  game-core/          Spieler, Turns, Undo, Recovery, Registry
  game-modes/         alle klassischen und Arcade-Modi
  statistics/         Sessionwertung, Heatmap, Replay, Empfehlungen
  storage/            SQLite-Schema, Migrationen, Repository-Vertrag
  runtime/            serialisierte autoritative Runtime und Effect Queue
  server/             REST, WebSocket, Static Assets und Healthcheck
  board-api/          plattformneutraler BoardTransport-Vertrag

platforms/
  linux/              BlueZ/BLE und Docker-Entry-Point
  apple/              CoreBluetooth und iPad/macOS-Displayintegration
  windows/            WinRT BLE und Fensterintegration
  android/            Kotlin BLE und External Display

apps/
  tauri/              gemeinsame Desktop-/Mobile-Hülle

web/
  control.html
  projector.html
  static/             vorhandene UI, Assets, Themes und Sounds

fixtures/
  packets/
  games/
  sessions/
  replays/

legacy/
  python/             zunächst unveränderte Referenzimplementierung
```

Am Anfang werden die vorhandenen Dateien nicht nach `legacy/` verschoben. Die
neue Struktur entsteht daneben. Umbenennungen folgen erst, wenn der neue
Linux-Server produktiv gleichwertig ist.

## 6. Kernverträge

Der Fachkern arbeitet als deterministische Zustandsmaschine:

```rust
pub trait GameMode {
    fn metadata(&self) -> &GameMetadata;
    fn initialize(&self, player: &mut Player, options: &GameOptions);
    fn apply_throw(
        &self,
        state: &mut GameState,
        event: &DartEvent,
    ) -> ThrowOutcome;
}

pub struct Transition {
    pub state: PublicState,
    pub accepted: bool,
    pub event: Option<PublicEvent>,
    pub effects: Vec<PlatformEffect>,
}
```

Der Core kennt nicht:

- BlueZ, CoreBluetooth oder WinRT,
- HTTP oder WebSocket,
- DOM oder WebView,
- konkrete Fenster und Displays,
- App-Verzeichnisse,
- Sound-APIs,
- Tauri-Kommandos.

Er erzeugt nur Zustände und deklarative Effekte. Server oder App-Hülle führen
diese aus.

## 7. Plattformadapter

### `BoardTransport`

```text
scan → connect → discover → subscribe → events → disconnect/reconnect
```

Implementierungen:

- Linux: zunächst bestehender Bleak-Gateway oder neuer BlueZ-Adapter,
- macOS/iPadOS: CoreBluetooth,
- Windows: Windows Runtime Bluetooth LE,
- Android: Android BluetoothGatt über Kotlin-Plugin,
- Tests: Replay- und Fake-Board-Adapter.

Die App speichert eine bekannte Gerätekennung nur zur Beschleunigung. Wenn sie
nicht mehr gültig ist, sucht sie wieder nach Name `SDB-BT` und Service `FFF0`.

### `DisplayHost`

- Linux/Browser: getrennte URLs `/control` und `/projector`,
- macOS/Windows: zwei Tauri-Fenster und auswählbares Vollbilddisplay,
- iPadOS: interaktive Hauptscene plus externe nichtinteraktive Display-Scene,
- Android: Haupt-Activity plus `Presentation` auf externem Display.

### `Storage`

Alle nativen Ziele verwenden dasselbe versionierte SQLite-Schema. Unterschiede
bestehen nur im vom Betriebssystem vorgegebenen Speicherpfad und in Export-
beziehungsweise Dateidialogen.

## 8. Linux/Docker bleibt erster Meilenstein

Der Rewrite wird nicht zuerst als iPad- oder Desktop-App gebaut. Seine erste
vollständige Zielplattform ist Linux/Docker:

```text
SDB-BT
  ↓ BlueZ oder bestehender BLE-Gateway
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
- Healthcheck,
- Betrieb ohne `privileged`,
- nur erforderlicher D-Bus-Zugriff für BlueZ,
- Umgebungsvariablen für Boardname, Adresse, Port und Testmodus,
- vollständiger Offlinebetrieb,
- Recovery nach Prozess- und Hostneustart,
- bestehende REST-/WebSocket-Oberfläche oder klar versionierte Migration,
- reale Hardwaretests vor Ablösung des Python-Containers.

Erst wenn dieser Pfad mindestens die heutige Funktionalität und Stabilität
erreicht, beginnen produktive native Releases.

## 9. Reihenfolge der Zielplattformen

```text
1. Linux/Docker, Headless-Server, reales Board
2. macOS, weil CoreBluetooth und Projektorfenster gut testbar sind
3. iPadOS mit direktem BLE und externer Projector-Scene
4. Android mit Kotlin-BLE und Presentation
5. Windows mit WinRT-BLE
6. optional Steam-Distribution des Desktop-Builds
```

Diese Reihenfolge ist eine Releasepriorität, keine unterschiedliche
Kernarchitektur. Alle Ziele verwenden denselben Rust-Core.

## 10. „Starten, Board einschalten, geht“

Der Verbindungsablauf ist Teil des gemeinsamen Produktvertrags:

```text
Runtime startet
  ↓
BoardTransport prüft Berechtigung und Funkstatus
  ↓
bekanntes Board verbinden oder nach SDB-BT/FFF0 suchen
  ↓
FFF1 abonnieren
  ↓
Runtime meldet BOARD_READY
```

Die Einschaltreihenfolge ist egal. Nach der ersten Einrichtung erfolgen Scan,
Connect, Subscribe und Reconnect automatisch. Nur diese Zustände erscheinen in
der normalen UI:

- Board wird gesucht,
- Board wird verbunden,
- Board bereit,
- Board getrennt – Spiel pausiert,
- Bluetooth oder Berechtigung fehlt,
- Board wird vermutlich von einem anderen Gerät verwendet.

Rohe Backendfehler bleiben in Diagnose und Logs. Ein laufendes Spiel nimmt
keine weiteren Würfe an, solange die Hardwareverbindung unklar ist.

## 11. Spielmodi und Modularität

Spielmodi bleiben eigenständige Module mit einem gemeinsamen Trait. Eine
Build-Time-Registry entdeckt beziehungsweise registriert sie. Das Hinzufügen
eines Modus verändert weder Server, Plattformadapter noch UI-Core.

Native Store-Builds laden keinen ausführbaren Code nach. Neue Modi werden in
signierten Releases gebündelt. Metadaten, Anleitungen, Optionen, Overlay-Daten,
Sounds und Artworkreferenzen bleiben deklarativ und werden von derselben UI
gerendert.

## 12. Parität statt Big Bang

Der Rewrite entsteht auf einem langlebigen Branch, aber in überprüfbaren
vertikalen Schnitten. Die Python-Version bleibt das Vergleichsorakel.

Vorgeschlagener Branch:

```text
rewrite/cross-platform-core
```

Portierreihenfolge:

1. Verträge und JSON-Schemas aus dem heutigen öffentlichen Zustand ableiten.
2. Reale BLE-Pakete, Spielabläufe und Replays als Golden Fixtures sichern.
3. Protokolldecoder und Interpreter nach Rust portieren.
4. Basisruntime mit Spielern, Turns, Hold, Undo und Recovery portieren.
5. CountUp und X01 als erste End-to-End-Modi portieren.
6. SQLite, Sessions, Statistik, Heatmap und Replay portieren.
7. verbleibende Modi einzeln mit Paritätstests übernehmen.
8. REST/WebSocket-Server und vorhandene UI verbinden.
9. Linux-Docker-Image mit realer Hardware qualifizieren.
10. erst danach Tauri-Hüllen und native Adapter ausliefern.

Jede Golden Fixture läuft gegen Python und Rust. Verglichen werden mindestens:

- akzeptierte und verworfene Events,
- aktueller Spieler, Runde und Dartnummer,
- Scores, Marks und Moduszustand,
- Gewinner und Ergebnisart,
- Undo-/Recovery-Snapshot,
- persistierte Telemetrie,
- öffentliches Overlay und Anweisungen.

## 13. Branch- und Release-Regeln

- `main` bleibt bis zur Parität die produktive Python-/Docker-Version.
- Fehlerkorrekturen landen zuerst auf `main` und werden als Fixture und
  Rust-Fix in den Rewrite übernommen.
- Der Rewrite löscht keine funktionierende Produktionsstrecke.
- Plattformcode darf keine Spielregeln duplizieren.
- Kein nativer Release vor erfolgreichem Linux-Paritäts- und Hardwaretest.
- Kein Merge allein aufgrund struktureller Vollständigkeit.
- Der Wechsel des Docker-Images benötigt Migrationstest mit einer Kopie einer
  realen Datenbank.

## 14. Meilensteine

```text
A  Contracts + Golden Fixtures
B  Rust Protocol + Core + CountUp/X01
C  Sessions + SQLite + Recovery
D  alle Spielmodi mit Parität
E  Rust REST/WebSocket-Server mit bestehender UI
F  Linux-Docker-Image und Hardware-Langzeittest
G  macOS-App
H  iPadOS-App mit externem Projector
I  Android- und Windows-Adapter
J  optional Steam
```

## 15. Ergebnis

Die aktuelle Linux-/Docker-Anwendung bleibt das Produkt und wird nicht durch
eine frühe Apple- oder Mobile-Architektur verdrängt. Der Rewrite wird zuerst
als kompatibler Linux-Server bewiesen.

Der einmalige Wechsel zu einem plattformneutralen Rust-Core verhindert danach
große fachliche Rewrites für iPadOS, macOS, Android und Windows. Die
unvermeidlichen Plattformunterschiede werden auf kleine, testbare Adapter für
BLE, Displays, Lifecycle und Packaging begrenzt.
