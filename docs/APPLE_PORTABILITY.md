# Portabilität für iOS und macOS

Stand: 2026-08-01

Status: **Unterstützende Plattformanalyse, nicht normativ.** Die verbindliche
Zielarchitektur, Verträge, Sicherheitsregeln und Meilensteine stehen in
[CROSS_PLATFORM_ARCHITECTURE.md](CROSS_PLATFORM_ARCHITECTURE.md). Die hier
beschriebenen Alternativen sind verworfene Optionen oder begrenzte
Machbarkeitstests, sofern die zentrale Spezifikation sie nicht ausdrücklich
übernimmt.

## 1. Ergebnis und Priorität

Smart Dartboard ist bereits gut für einen nativen macOS-Host vorbereitet. Die
vorhandene Control- und Projector-Oberfläche ist außerdem ohne grundlegenden
Neuaufbau auf iPhone und iPad nutzbar. Nicht direkt portabel ist dagegen die
Kombination aus Python-Spielkern, FastAPI, Bleak und einem dauerhaft laufenden
lokalen Server auf iOS.

Linux/Docker bleibt der erste Installations- und Produktionspfad. iPadOS und
macOS sind wichtige native Folgeziele, dürfen aber keine Apple-spezifische
Neuarchitektur des Spielkerns erzwingen. Android und Windows sollen denselben
Kern später ebenfalls verwenden.

Für einen vollständigen Rewrite wird deshalb ein plattformneutraler Rust-Core
mit einem Headless-Linux-Server und späteren Tauri-Hüllen empfohlen. Die
vorhandenen Weboberflächen, Artworks und Animationen werden weiterverwendet.
Der übergreifende Plan steht in
[CROSS_PLATFORM_ARCHITECTURE.md](CROSS_PLATFORM_ARCHITECTURE.md).

Bis zur vollständigen Parität des neuen Cross-Platform-Kerns bleibt diese
bereits funktionierende Hostarchitektur der produktive Referenzpfad:

```text
SDB-BT Dartboard
  ↓ BLE
macOS-, Linux- oder Raspberry-Pi-Host
  ├─ Python-Spielkern und SQLite
  ├─ FastAPI / WebSocket
  ├─ Projector UI und Sound
  └─ Control UI
       ↑ LAN
       └─ iPhone / iPad als Browser, PWA oder Capacitor-App
```

Damit bleiben aktuelle Installationen während des Rewrite stabil. Die neue
Zielarchitektur verwendet denselben Spielzustand, dieselbe Statistik und
dieselben Regeln im Linux-Server sowie in den nativen Apps. Nur Bluetooth,
Dateisystempfade, Haptik, Displays und App-Lifecycle werden adaptiert. Silent
Shark ist für die Wiederverwendung der Weboberfläche eine interessante
Referenz.

## 2. Heutiger Projektstand

```text
SDB-BT
  ↓ FFF1 Notifications
Bleak Client
  ↓
Protocol Decoder + Event Interpreter
  ↓
Python Game Engine + automatisch entdeckte Game Plugins
  ↓
Session Controller + SQLite Event Store
  ↓
FastAPI REST + WebSocket
  ↓
gemeinsame HTML/CSS/JavaScript-Oberfläche
  ├─ /control
  └─ /projector
```

Die Trennung ist grundsätzlich günstig:

- `sdb_dartboard/client.py` kapselt die BLE-Verbindung.
- `protocol.py` und `interpreter.py` enthalten wenig plattformspezifische
  Logik.
- Spielregeln liegen in eigenständigen Python-Modulen.
- `storage.py` hält Statistik und Wiederherstellung lokal in SQLite.
- Die Weboberfläche bündelt REST-Aufrufe in `api()` und `getJson()` und empfängt
  Zustände über eine zentrale WebSocket-Verbindung.
- Artworks, Theme-Packs, Dartboard-SVG, Animationen und Web Audio sind auf
  WebKit grundsätzlich weiterverwendbar.

Die aktuelle Portabilität je Baustein:

| Baustein | macOS | iOS/iPadOS | Bemerkung |
| --- | --- | --- | --- |
| HTML/CSS/JavaScript und Assets | sehr gut | sehr gut | WebKit-tauglich; Touch bereits vorgesehen |
| Python-Spielmodi und Sessions | direkt | nur eingebettetes Python oder Portierung | logisch plattformneutral, sprachlich an Python gebunden |
| FastAPI und WebSocket-Server | direkt | technisch möglich, aber ungeeignet als dauerhafter App-Kern | iOS kann Apps suspendieren oder beenden |
| SQLite | direkt | direkt über native Bibliothek oder Plugin | Datenpfade und App-Lifecycle unterscheiden sich |
| Bleak | CoreBluetooth-Backend vorhanden | nicht als bestehende Integration nutzbar | iOS benötigt einen nativen CoreBluetooth-Adapter |
| Projector und Sound | sehr gut | eingeschränkt | macOS ist für Kiosk, Audio und externes Display zuverlässiger |
| Docker | Linux-Deployment | nicht verfügbar | Docker Desktop reicht macOS-BLE nicht an Linux weiter |

## 3. Referenzanalyse: Silent Shark

### 3.1 Öffentlich verifizierbarer Stand

Die öffentliche [Silent-Shark-Website](https://silentshark.app/) nennt am
1. August 2026:

- eine direkt spielbare Browser-Demo,
- eine für den 12. August angekündigte Steam-Vollversion für Windows, macOS
  und Linux,
- separat geplante Versionen für iOS, iPadOS und Android.

Ein öffentlicher App-Store-Eintrag für Silent Shark war zu diesem Zeitpunkt
weder über die Apple-Suche noch über die öffentliche iTunes Search API
auffindbar. Eine TestFlight-, private oder noch nicht veröffentlichte Fassung
kann dadurch nicht ausgeschlossen werden. Die Aussage zur iOS-Verfügbarkeit
sollte daher nicht als verifizierter öffentlicher Release-Status dokumentiert
werden.

### 3.2 Beobachtbare technische Architektur

Die ausgelieferte Browser-Demo unter
[`https://silentshark.app/alpha/`](https://silentshark.app/alpha/) enthält eine
statisch ausgelieferte, mit Vite gebaute Webanwendung. Im kompilierten Bundle
sind unter anderem folgende Eigenschaften direkt erkennbar:

- React mit `createRoot` als UI-Runtime,
- Three.js und Canvas für die grafische Simulation,
- Capacitor als native Mobile-Brücke,
- Capacitor-Plugins für `Filesystem`, `Haptics`, `SystemBars` und HTTP,
- Touch-, Safari- und iOS-spezifische Anpassungen,
- ein Speicherdienst mit mehreren Adaptern,
- eine separate globale Desktop-Brücke namens `silentSharkDesktop`,
- Desktop-Funktionen für persistenten Speicher, Fensterzustand, Diagnose,
  Beenden der Anwendung und Steam-Achievements,
- ein Web-Fallback auf `localStorage`.

Die Speicherauswahl folgt im ausgelieferten Code sinngemäß diesem Muster:

```text
Desktop-Bridge Storage
  sonst Capacitor/native Storage
  sonst Browser localStorage
```

Die Desktop-Brücke sieht nach einem Electron-artigen Preload-/IPC-Modell aus.
Aus einem minimierten Browser-Bundle allein lässt sich jedoch nicht
beweissicher bestimmen, ob die veröffentlichte Desktop-Hülle tatsächlich
Electron oder eine eigene vergleichbare Integration verwendet. Diese
Beobachtung ist deshalb ausdrücklich eine technische Inferenz und keine
Herstellerangabe.

Das HTML der untersuchten Browser-Demo bindet weder ein Web App Manifest noch
einen Service Worker ein; auch die üblichen Manifest- und Service-Worker-Pfade
lieferten keinen Treffer. Sie ist damit eine Web-App, aber in dieser öffentlich
ausgelieferten Form keine vollständig installierbare Offline-PWA.

### 3.3 Was Silent Shark tatsächlich demonstriert

Silent Shark verwendet nicht eine identische Binärdatei auf allen Plattformen.
Wiederverwendet werden vielmehr Web-UI, Simulation und Datenmodell. Native
Hüllen liefern pro Plattform die fehlenden Fähigkeiten:

```text
                gemeinsamer React/JavaScript-Spielkern
                              │
             ┌────────────────┼────────────────┐
             │                │                │
         Browser        Mobile Shell      Desktop Shell
      localStorage      Capacitor         Desktop Bridge
                        Filesystem         Filesystem
                        Haptics            Window/Steam
                        System Bars        Diagnostics
```

Das ist für Smart Dartboard ein relevantes Muster. Es zeigt:

1. Eine aufwendige Spieloberfläche mit Audio, Canvas und 3D kann als Web-Code
   über Browser, iOS und macOS geteilt werden.
2. Plattformfunktionen sollten hinter kleinen Adapter-Schnittstellen liegen.
3. Browser-, Mobile- und Desktop-Edition dürfen unterschiedliche Hüllen und
   Fähigkeiten besitzen, ohne dass die eigentliche Anwendung dupliziert wird.
4. Persistenz muss migrationsfähig sein, wenn Daten zunächst in
   `localStorage` und später in einem nativen Speicher liegen.
5. Editionen und Build-Kanäle sollten explizit modelliert werden.

Der wesentliche Unterschied zu Smart Dartboard ist der Ort des Spielkerns:
Silent Sharks Simulation ist im JavaScript-Bundle erkennbar. Smart Dartboards
autoritative Regeln, Sessions und Statistiken laufen in Python. Ein
Silent-Shark-ähnliches, vollständig eigenständiges iOS-Paket setzt daher
entweder einen TypeScript-Spielkern oder eine wesentlich komplexere
Python-Einbettung voraus.

## 4. Sinnvolle Zieloptionen

### Option A: Host plus Web-/Mobile-Controller

**Stabiler bestehender Betrieb und Rückfallpfad während des Rewrite.**

- macOS, Linux oder Raspberry Pi betreiben BLE, Python und SQLite.
- `/projector` läuft auf dem fest verbundenen Projektorrechner.
- `/control` läuft in Safari oder als PWA.
- Eine optionale Capacitor-App verpackt dieselbe Control UI und ergänzt
  Bonjour, QR-Pairing, Haptik und sichere Tokenablage.

Wiederverwendung: ungefähr 90 bis 95 Prozent des heutigen Codes.

Vorteile:

- geringstes Betriebsrisiko,
- keine doppelte Statistik,
- iOS-Suspendierung beendet kein Spiel,
- bestehende Python-Spielplugins bleiben unverändert,
- Projektor, Sound und Kalibrierung bleiben auf einem dauerhaften Host,
- Raspberry Pi und Linux bleiben gleichberechtigte Ziele.

Grenzen:

- iPhone oder iPad können ohne Host nicht direkt spielen,
- LAN-Erkennung und Pairing müssen ergänzt werden,
- die App ist ein Controller und nicht die vollständige Spielkonsole.

### Option B: Gemeinsamer TypeScript-Kern nach Silent-Shark-Muster

**Mögliche Web-first-Variante, aber nicht mehr die generische Hauptentscheidung.**

- Spielzustand und Spielregeln werden schrittweise nach TypeScript portiert.
- Der reine Core läuft gebündelt in JavaScriptCore und in Browser-Tests.
- Control und Projector bleiben WKWebViews, besitzen aber keinen eigenen Core.
- Swift stellt auf iPadOS und macOS CoreBluetooth, SQLite und Lifecycle bereit.
- Eine gemeinsame Swift-Runtime verteilt Zustände an alle Fenster und Scenes.
- Browser und spätere Plattformen erhalten eigene Adapter desselben Kerns.

Für eine primär web- und mobileorientierte Anwendung wäre diese Lösung gut.
Mehrere native Fenster, Headless-Linux, Windows und verschiedene
JavaScript-Runtimes machen sie für das nun gewünschte Gesamtziel aber weniger
einheitlich als einen Rust-Core.

### Option B2: Plattformneutraler Rust-Core plus Tauri

**Gewählte langfristige Zielarchitektur.**

- ein Fachkern für Linux, macOS, Windows, iPadOS und Android,
- Headless-Server für Docker ohne Tauri-Abhängigkeit,
- Tauri-Hüllen für native Desktop- und Mobile-Apps,
- dieselbe bestehende Web-UI auf allen interaktiven Zielen,
- native Adapter ausschließlich für BLE, Displays, Lifecycle und Packaging,
- statisch gebündelte, modulare Spielmodi.

Die vollständige Architektur steht in
[CROSS_PLATFORM_ARCHITECTURE.md](CROSS_PLATFORM_ARCHITECTURE.md).

Beim gewählten Rust-Ansatz ist der gemeinsame Nenner nicht die Hülle, sondern:

- Rust-Spielkern,
- Zustandsmodell und Ereignisvertrag,
- Web-UI,
- Artworks, Sound und Animationen,
- sprachunabhängige Regeltests und Replay-Fixtures.

Der Aufwand ist erheblich: 24 Modi, Sessionwertung, Undo, Recovery,
Statistiken und Migrationen müssen semantisch identisch bleiben. Die vorhandene
Testsuite ist dafür wertvoll, sollte aber um JSON-Golden-Fixtures ergänzt
werden, die Python- und Rust-Ergebnisse vergleichen.

### Option C: Python in iOS einbetten

CPython kann über ein XCFramework in iOS-Anwendungen eingebettet werden. Diese
Option bewahrt mehr Python-Code, bringt im aktuellen Projekt aber hohe Risiken:

- `pydantic-core` und optionale Uvicorn-Abhängigkeiten enthalten native
  Komponenten und benötigen iOS-Wheels,
- Bleak muss trotzdem durch eine Swift-CoreBluetooth-Brücke ersetzt werden,
- ein lokaler FastAPI-Server ist auf iOS eine zusätzliche Fehlerquelle,
- App-Lifecycle, Packaging, Signierung und Debugging werden komplex,
- dynamisch nachgeladene Python-Spielplugins kollidieren mit App-Store-Regeln.

Diese Variante eignet sich als begrenzter Machbarkeitstest, nicht als derzeit
empfohlene Produktarchitektur.

### Option D: ausschließlich native SwiftUI-App

SwiftUI und CoreBluetooth wären für Apple-Plattformen robust. Die vorhandene
UI, alle Animationen und ein großer Teil der Logik müssten jedoch neu erstellt
werden. Gleichzeitig würden Browser, Linux und Raspberry Pi ihre gemeinsame
Codebasis verlieren. Diese Option ist nur sinnvoll, wenn Apple-Plattformen
später das alleinige Produktziel werden.

## 5. Empfohlene gemeinsame Schnittstellen

Vor einer nativen Hülle sollte die Anwendung folgende Dienste explizit
abstrahieren:

```text
BoardTransport
  ├─ BleakHostTransport
  ├─ RemoteWebSocketTransport
  └─ CoreBluetoothTransport

StateTransport
  ├─ FastApiTransport
  └─ LocalCoreTransport

Storage
  ├─ HostSqliteStorage
  ├─ CapacitorStorage
  └─ BrowserStorage

PlatformServices
  ├─ Audio
  ├─ Haptics
  ├─ Display / Kiosk
  ├─ Discovery / Pairing
  └─ Lifecycle
```

Die bestehende UI setzt aktuell relative REST-URLs und denselben Origin voraus.
Als erster Vorbereitungsschritt sollte der Transport aus `app.js` in ein
kleines Modul ausgelagert werden. Die UI darf dann nicht wissen, ob ein Befehl
per HTTP, nativer Bridge oder lokalem TypeScript-Kern ausgeführt wird.

## 6. macOS-spezifische Anforderungen

- signiertes und notarisiertes `.app`-/DMG-Paket,
- Bluetooth-Nutzungsbeschreibung und korrekte Berechtigungen,
- Anwendungsdaten unter `Application Support` statt im App-Bundle,
- kontrollierter Start und Stopp des lokalen Servers,
- verständliche BLE-Reconnect- und Board-Besitzanzeige,
- Vollbild-/Kioskmodus für den Projektor,
- Displayprofil und Kalibrierung pro Projektor beziehungsweise Auflösung,
- Verhindern von Ruhezustand und Bildschirmschoner während einer Session,
- feste Audioausgabe und vorhersehbares Verhalten bei Gerätewechsel,
- Logs, Diagnoseexport und Wiederherstellung nach einem Absturz,
- Universal Build für Apple Silicon und möglichst Intel, falls alte
  Arcade-Macs unterstützt werden sollen.

Für den Arcade-Betrieb ist zunächst ein direkt vertriebenes, notarisiertes DMG
einfacher als der Mac App Store. Der Mac App Store bringt zusätzliche
Sandbox-, Update- und Plugin-Einschränkungen.

## 7. iOS-/iPadOS-spezifische Anforderungen

### Bluetooth

Safari bietet kein Web Bluetooth. Eine direkte Board-Verbindung benötigt
CoreBluetooth in Swift oder in einem Capacitor-/Tauri-Plugin. Der bestehende
Python-Decoder und Interpreter können anhand gemeinsamer Byte-Fixtures
nachimplementiert und gegeneinander getestet werden.

iOS kann Apps im Hintergrund suspendieren oder beenden. Der
`bluetooth-central`-Modus erlaubt das Aufwachen für BLE-Ereignisse, ersetzt aber
keinen dauerhaft laufenden Arcade-Server. State Preservation und Restoration
sind für eine direkte iOS-Verbindung zwingend einzuplanen.

### Lokales Netzwerk

Eine Controller-App benötigt:

- `NSLocalNetworkUsageDescription`,
- deklarierte Bonjour-Service-Typen,
- Retry nach dem ersten Berechtigungsdialog,
- ATS-Konfiguration für lokale HTTP-Verbindungen oder lokales TLS,
- IPv6-Tauglichkeit,
- einen manuellen Fallback per QR-Code oder Hostname.

Die bestehende Same-Origin-Prüfung darf nicht durch global offenes CORS ersetzt
werden. Native Controller sollten über einen QR-Code oder kurzen Code gepaart
werden und ein widerrufbares Token erhalten.

### Audio und Anzeige

- Audio mit Ton benötigt in WebKit normalerweise zunächst eine
  Benutzerinteraktion.
- Der Projektorhost sollte weiterhin der primäre Soundausgang sein.
- Safe Areas, Rotation, Dynamic Type und unterschiedliche iPad-Größen müssen
  geprüft werden.
- Eine iPad-App mit externem Bildschirm ist möglich, aber deutlich anfälliger
  für Lifecycle-, AirPlay- und Displaywechsel als ein fester macOS-Host.

Der bevorzugte mobile Arcade-Modus heißt **Direkte Ausgabe**: Ein einzelnes
unterstütztes iPhone oder iPad besitzt Board, Runtime, SQLite und Control UI und
gibt eine separate Projector UI über AirPlay oder HDMI aus. Der Controller muss
also kein iPad sein. Nach dem App-Start und Einschalten eines bekannten Boards
soll ohne erneutes Pairing oder manuelle Navigation gespielt werden können.
System-Mirroring der Control UI reicht dafür nicht aus; ein AirPlay-Ziel ohne
erweiterte externe Anzeige führt kontrolliert zum HDMI- oder Companion-Fallback.
Bis iOS/iPadOS 26 wird die nichtinteraktive externe Scene über
`windowExternalDisplayNonInteractive` angebunden; ab Version 27 muss die App
dafür ein External-Display-Scene-Accessory registrieren. Beide Implementierungen
liegen hinter demselben `DisplayHost`-Vertrag.

Als Fallback und zweiter Produktmodus dient **Companion-Projektor**. Sein
prominenter Produktfall ist ein iPhone als handlicher Controller und ein iPad
als kabelloser Projector; auch ein iPad darf Controller sein:

```text
iPhone/iPad: Controller + BLE + Runtime + SQLite
                    ↓ lokales, gepaartes Netzwerk
iPad:          Projector UI + Projector-Sound
```

Das Companion-iPad ist kein zweiter Host. Es entdeckt die Controller-Runtime
über Bonjour, wird per QR- oder Einmalcode mit der Rolle `projector` gepaart und
bezieht nach jeder Unterbrechung zuerst einen vollständigen Snapshot. Es darf
weder BLE übernehmen noch Commands für Spiel oder Setup senden. Die
Controller-Runtime bleibt bei Display- oder Netzwerkverlust spielfähig.

AirPlay/HDMI, Companion und lokale Vorschau werden im Board-Setup gewählt. Die
App merkt sich den zuletzt erfolgreichen Ausgabepfad, stellt ihn beim nächsten
Arcade-Start nach Möglichkeit automatisch wieder her und zeigt aktiven Pfad,
Audioziel sowie Verbindungsstatus ausschließlich auf der Control UI. Der
Wechsel des Ausgabepfads darf keine neue Runtime oder Session erzeugen.

Für AirPlay, HDMI und Companion werden getrennte Hardwarematrizen geführt, weil
Displayausgabe, Audio-Routing und Laden über einen Adapter modellabhängig sind.
Die App zeigt den aktiven Ausgabepfad und bietet bei fehlender erweiterter
Anzeige einen klaren Wechsel zum Companion-Modus an.

### App Store und Spielplugins

Die heutigen Python-Modi werden dynamisch entdeckt. In einer iOS-App dürfen
keine neuen ausführbaren Python- oder JavaScript-Module nachgeladen werden, die
die Funktion der App verändern. Neue Modi sollten deshalb entweder:

- mit einem regulären App-Update ausgeliefert werden oder
- aus einer begrenzten, deklarativen Regelbeschreibung bestehen, die nur
  bereits eingebaute und geprüfte Aktionen kombiniert.

Theme-Artworks und Sounds sind als Daten einfacher zu verteilen als Code. Ein
komplett neuer Spielablauf sollte dennoch über einen App-Release kommen.

## 8. Daten und Synchronisation

Der Host bleibt zunächst die alleinige Quelle der Wahrheit. Spieler, Sessions,
Würfe und Statistiken dürfen nicht gleichzeitig in voneinander unabhängigen
iOS- und macOS-Datenbanken fortgeschrieben werden.

Für eine spätere eigenständige App werden benötigt:

- versionierte Datenbankmigrationen,
- Import und Export derselben JSON-Struktur,
- eindeutige Geräte-, Session-, Spiel- und Event-IDs,
- Konfliktregeln für spätere Synchronisation,
- getrennte Kalibrierungsprofile pro Display,
- sichere Migration von Browserdaten zu nativem Speicher,
- deterministische Zeit-, Zufalls- und UUID-Abstraktionen für Tests.

Silent Sharks Storage-Adapter und Migration aus `localStorage` sind hierfür ein
sinnvolles Vorbild.

Der erste portable Datentransfer ist implementiert: Export und bestätigter
Import verwenden auf Linux, macOS und iOS denselben versionierten JSON-Vertrag
und denselben Rust/SQLite-Code. Der Import ist atomar, größenbegrenzt und nur
für die Controller-Rolle freigegeben. Er ist bewusst noch keine bidirektionale
Synchronisation; ID-Kollisionen werden abgelehnt statt automatisch aufgelöst.

Auch der iOS-Lifecycle besitzt nun eine explizite Grenze. Beim Wechsel in den
Hintergrund bleiben Session und Spiel atomar erhalten, während BLE und lokale
Netzwerkrollen gestoppt werden; nach dem Vordergrundwechsel werden sie passend
zur gespeicherten App-Rolle neu verbunden. Das ist im iPad-Simulator als echte
Suspend/Resume-Sequenz geprüft. Verhalten mit realer Scheibe, iOS-
Hintergrundzeitlimits und WLAN-/AirPlay-Unterbrechungen ist weiterhin ein
Hardware-Gate, keine aus dem Simulator abgeleitete Supportaussage.

Zusätzlich wird die native Hülle als vollständiges, bewusst unsigniertes
ARM64-Gerätearchive gebaut und in CI kurzzeitig bereitgestellt. Damit sind
iPhone-/iPad-Gerätecompilation und Packaging belegt. Das Archive ist ohne
Development Team und Provisioning weder installierbar noch ein TestFlight-
oder App-Store-Nachweis; diese Aussagen bleiben bis zum Lauf auf echter
Hardware ausdrücklich offen.

macOS verwendet für Systemruhe und Aufwachen denselben Zustandsübergang. Ein
kleiner AppKit-Adapter beobachtet die beiden `NSWorkspace`-Benachrichtigungen;
Rust serialisiert danach Adapterabbau und Wiederanlauf genauso wie auf iOS.
Der Adapter ist im signaturfreien App-Bundle kompiliert und verlinkt. Ein
echter Sleep/Wake-Lauf mit verbundenem Board ist weiterhin Teil der
Hardwarequalifikation und wird nicht durch einen reinen Build als bestanden
gewertet.

Im normalen Arcade-Betrieb verhindert die App den automatischen Ruhezustand
bereits pro Session statt global: macOS hält Display und System mit einer
`NSProcessInfo`-Aktivität wach, iOS/iPadOS deaktiviert den Idle-Timer. Die
Sperre folgt dem autoritativen Sessionstatus und ist nur in der aktiven
Controller-Rolle gesetzt. Nach Sessionende, Suspend, Rollenwechsel oder
App-Ende wird sie freigegeben. Manuell ausgelöster Systemruhezustand bleibt
möglich und läuft weiterhin durch den oben beschriebenen Lifecycle-Pfad.

Der macOS-Displaypfad reagiert außerdem auf
`NSApplicationDidChangeScreenParametersNotification`. Die App unterscheidet
den Monitor des Control-Fensters von eigenständigen erweiterten Desktops,
wählt bei mehreren Ausgängen deterministisch den größten, legt den Projector
rahmenlos darüber und reagiert ohne Neustart auf An- und Abstecken. Reines
Spiegeln zählt nicht als Projector-Ausgang. Ohne zweiten Desktop bleibt die
Projector-WebView im Modus `AirPlay / HDMI` verborgen; die bewusst gewählte
lokale Vorschau erscheint dagegen zentriert auf dem Control-Screen. Auswahl-
und Randfalllogik sowie App-Bundle sind automatisiert geprüft, reale AirPlay-
und HDMI-Hardware noch nicht.

Der normale native Erststart enthält keinen M0-Demozustand mehr: Controller
beginnen auf dem Startbildschirm ohne Spieler oder laufendes Spiel. Der
automatische CountUp-/T20-Ablauf existiert nur hinter dem expliziten
Debug-Schalter für Simulator- und Displaytests und ist in Release-Builds
gesperrt.

## 9. Empfohlene Umsetzungsschritte

1. Mit einem minimalen M0-Spike CoreBluetooth, zwei WebViews sowie eigenständige
   Projector-Ausgabe über HDMI und AirPlay auf echter Hardware beweisen; dabei
   sowohl die Scene-Rolle bis iOS/iPadOS 26 als auch den Scene-Accessory-Pfad ab
   Version 27 berücksichtigen.
2. Companion-Spike mit iPhone/iPad als autoritativem Controller und iPad als
   gepaartem, read-only Projector beweisen.
3. Ergebnis und Rückfallpfad für die native Hülle als ADR dokumentieren.
4. Bestehende Python-Zustände im versionierten Golden-Fixture-Format sichern.
5. Rust-Workspace mit Contracts, Protocol, Core und Fixtures aufbauen.
6. CountUp und X01 mit exakter Python-/Rust-Parität portieren.
7. Atomare Runtime, Session, SQLite, Undo, Recovery, Statistik und Replay
   portieren.
8. Alle weiteren Modi einzeln mit Golden Fixtures übernehmen.
9. Gemeinsamen Runtime-Client sowie Rust-REST-/WebSocket-Server anbinden.
10. Linux-Docker-Image gegen die reale Hardware qualifizieren.
11. macOS- und anschließend iOS-/iPadOS-Hülle produktiv anbinden.
12. TestFlight, notarisiertes macOS-Paket und spätere Store-Pakete erstellen.

Details, Branch-Regeln und Meilensteine stehen in
[CROSS_PLATFORM_ARCHITECTURE.md](CROSS_PLATFORM_ARCHITECTURE.md).

## 10. Entscheidung

Die Silent-Shark-Analyse bestätigt, dass die Weboberfläche über Plattformen
geteilt werden kann. Der Spielkern soll jedoch allgemeiner als Silent Sharks
erkennbare Desktop-/Mobile-Brücken aufgestellt werden.

Sie präzisiert jedoch die langfristige Richtung. Falls eine echte
plattformgleiche Anwendung gewünscht ist, sollte nicht versucht werden, den
kompletten FastAPI-Prozess unverändert in jede App zu verpacken. Stattdessen
sollten Web-UI und Rust-Spielkern gemeinsam genutzt werden, während jede
Plattform nur BLE, Displays, Lifecycle und Packaging adaptiert.

Die empfohlenen Produktstufen sind daher:

```text
M0:       nativer BLE-/External-Display-Machbarkeitstest ohne Spielportierung
Stufe 1:  Rust-Core mit Parität zur Python-Version
Stufe 2:  Headless-Server und Linux-Docker-Produktionspfad
Stufe 3:  macOS-App mit direktem CoreBluetooth
Stufe 4:  iOS-/iPadOS-App mit direktem BLE, AirPlay/HDMI und Companion
Stufe 5:  Android, Windows und optional Steam
```

Der bestehende Python-/FastAPI-Host bleibt währenddessen als funktionsfähige
Referenz und Vergleichsorakel erhalten. Er wird im Linux-Betrieb erst ersetzt,
wenn der neue Container Parität und reale Hardwarestabilität bewiesen hat.

## 11. Externe Referenzen

- [Silent Shark – Produktseite und Plattformstatus](https://silentshark.app/)
- [Silent Shark – öffentliche Browser-Demo](https://silentshark.app/alpha/)
- [Capacitor – Web-first Native Runtime](https://capacitorjs.com/docs)
- [Apple Core Bluetooth](https://developer.apple.com/documentation/corebluetooth/)
- [Apple: UIKit Scene Lifecycle und externe Displays](https://developer.apple.com/documentation/uikit/transitioning-to-the-uikit-scene-based-life-cycle)
- [Apple: nichtinteraktive External-Display-Scene](https://developer.apple.com/documentation/uikit/uiscenesession/role-swift.struct/windowexternaldisplaynoninteractive)
- [Apple: Bonjour-Discovery mit NWBrowser](https://developer.apple.com/documentation/network/nwbrowser)
- [Apple: Core Bluetooth im Hintergrund](https://developer.apple.com/library/archive/documentation/NetworkingInternetWeb/Conceptual/CoreBluetooth_concepts/CoreBluetoothBackgroundProcessingForIOSApps/PerformingTasksWhileYourAppIsInTheBackground.html)
- [Apple: Local Network Privacy](https://developer.apple.com/documentation/technotes/tn3179-understanding-local-network-privacy)
- [Apple: App Transport Security](https://developer.apple.com/documentation/security/preventing-insecure-network-connections)
- [Apple App Review Guidelines](https://developer.apple.com/app-store/review/guidelines/)
- [Python Apple Support](https://github.com/beeware/Python-Apple-support)
- [Briefcase Platform Support](https://briefcase.beeware.org/en/stable/about/faq/)
- [Tauri 2 – Plattformvoraussetzungen](https://v2.tauri.app/start/prerequisites/)
