# Portabilität für Android und ein eigenständiges Spieltablet

Stand: 2026-08-01

Status: **Unterstützende Plattformanalyse, nicht normativ.** Die verbindliche
Zielarchitektur, Verträge, Sicherheitsregeln und Meilensteine stehen in
[CROSS_PLATFORM_ARCHITECTURE.md](CROSS_PLATFORM_ARCHITECTURE.md). Capacitor,
Chaquopy und der eingebettete Python-Core sind ausschließlich optionale,
isolierte Spike-Alternativen und nicht die gewählte Produktarchitektur.

## 1. Produktziel

Das bevorzugte Bedienmodell ist nicht dauerhaft ein verteilter Serveraufbau,
sondern ein möglichst eigenständiges Tablet:

```text
Tablet starten
  ↓
Smart-Dartboard einschalten
  ↓
automatisch verbinden
  ↓
Spiel auswählen und spielen
```

Netzwerk, Benutzerkonto, Terminal, Docker und manuelle IP-Adressen dürfen für
diesen normalen Ablauf nicht erforderlich sein. Das System muss vollständig
lokal funktionieren und nach der ersten Einrichtung sowohl die Boardverbindung
als auch Spieler, Statistiken, Optionen und Projektorkalibrierung behalten.

Android ist für dieses Ziel technisch gut geeignet. Linux/Docker bleibt der
erste Produktionspfad; macOS und iPadOS haben bei den nativen Apps Vorrang.
Android folgt mit demselben plattformneutralen Kern. Es erlaubt eine native
BLE-Verbindung, einen kontrollierbaren Foreground-Service,
Kiosk-/Lock-Task-Betrieb, lokale Dateispeicherung und eine eigene Darstellung
auf einem externen Display.

Die Apple-Bewertung und die gemeinsame langfristige Plattformstrategie stehen
ergänzend in [APPLE_PORTABILITY.md](APPLE_PORTABILITY.md).

## 2. Android-spezifische Python-Zwischenlösung

Falls unabhängig vom Apple-Rewrite ein schneller Android-Prototyp benötigt
wird, ist diese Architektur möglich:

```text
Android Tablet
  ├─ Capacitor WebView
  │    ├─ bestehende Control UI
  │    └─ bestehende Projector UI
  ├─ Kotlin Platform Plugin
  │    ├─ BLE / GATT / FFF1 Notifications
  │    ├─ Board Discovery und Reconnect
  │    ├─ Lifecycle und Foreground Service
  │    ├─ External Display / Presentation
  │    └─ Keep Awake, Haptik und Diagnose
  ├─ eingebetteter Python-Core über Chaquopy
  │    ├─ Protocol Decoder und Interpreter
  │    ├─ Game Engine und 24 Spielmodi
  │    ├─ Session Controller
  │    └─ SQLite-Statistik
  └─ lokaler App-Speicher
```

FastAPI, Uvicorn und Bleak werden in dieser App-Variante nicht gestartet. Die
Weboberfläche spricht über einen Capacitor-Plugin-Vertrag direkt mit einem
kleinen nativen Runtime-Adapter. Dieser ruft den eingebetteten Python-Spielkern
auf und liefert dessen öffentlichen Zustand als JSON zurück.

Damit wird das riskante Muster „lokalen Webserver innerhalb einer mobilen App
betreiben“ vermieden, während die heute getestete Python-Regellogik zunächst
erhalten bleibt.

Der eingebettete Core sollte dafür eine sehr kleine Fassade erhalten:

```text
MobileRuntime.bootstrap()                 → vollständiger öffentlicher Zustand
MobileRuntime.dispatch(action, payload)   → neuer Zustand
MobileRuntime.ingest_packet(raw_bytes)    → akzeptiertes Event + neuer Zustand
MobileRuntime.resume()                    → Zustand nach App-Rückkehr
MobileRuntime.close()                     → sauberer SQLite-Abschluss
```

Kotlin serialisiert alle Aufrufe auf genau einer Runtime-Queue und verteilt
Zustandsänderungen als Capacitor-Events an Control- und Projector-WebView. So
bleibt der Mobile-Vertrag fachlich nahe am heutigen REST-/WebSocket-Vertrag,
ohne einen HTTP-Server zu benötigen.

## 3. Warum Android für das Ein-Gerät-Ziel günstiger ist

### 3.1 BLE

Android stellt den vollständigen BLE-Central-Workflow nativ bereit:

- nach `SDB-BT` und optional Service `FFF0` scannen,
- GATT-Verbindung aufbauen,
- Characteristic `FFF1` abonnieren,
- Notifications empfangen,
- Verbindung und Services nach einem Abbruch wiederherstellen.

Ab Android 12 werden `BLUETOOTH_SCAN` und `BLUETOOTH_CONNECT` als
Laufzeitberechtigungen benötigt. Wenn Scan-Ergebnisse nicht zur
Standortbestimmung verwendet werden, kann die App dies im Manifest erklären
und auf neueren Android-Versionen ohne Standortfreigabe arbeiten.

Der
[Companion Device Manager](https://developer.android.com/develop/connectivity/bluetooth/companion-device-pairing)
kann die einmalige Gerätezuordnung vereinfachen. Er führt die Auswahl eines
nahen Geräts im Auftrag der App aus, erzeugt aber nicht selbst die eigentliche
GATT-Verbindung. Die App bleibt für Connect, Subscribe und Reconnect
verantwortlich.

Für eine laufende Verbindung empfiehlt Android entweder einen
`CompanionDeviceService` oder einen Foreground Service vom Typ
`connectedDevice`. Android dokumentiert dieses Modell ausdrücklich für das
langfristige Empfangen von BLE-Notifications:
[BLE im Hintergrund](https://developer.android.com/develop/connectivity/bluetooth/ble/background).

Im aktiven Spiel bleibt die App ohnehin sichtbar und der Bildschirm an. Der
Service ist vor allem für kurze App-Wechsel, Systemdialoge und einen robusten
Reconnect nützlich. Er darf nicht als Vorwand für unnötigen Dauerbetrieb
verwendet werden.

### 3.2 Kiosk- und Arcade-Betrieb

Android unterstützt einen dedizierten Lock-Task-Modus. Auf einem vollständig
verwalteten Arcade-Tablet kann dadurch verhindert werden, dass Gesten,
Benachrichtigungen oder ein versehentlicher Home-Tastendruck das laufende Spiel
verlassen. Die offizielle Beschreibung steht unter
[Lock Task Mode](https://developer.android.com/work/dpc/dedicated-devices/lock-task-mode).

Für private Installationen reicht zunächst ein normaler Vollbildmodus mit
`keep-awake`. Der echte Lock-Task-Modus sollte als optionales
„Dediziertes Gerät“-Setup angeboten werden, weil seine vollständige
Absicherung eine Device-Owner-/DPC-Konfiguration benötigt.

### 3.3 Externer Projektor

Android kann auf einem angeschlossenen HDMI- oder geeigneten drahtlosen Display
über die native `Presentation`-API andere Inhalte als auf dem Tablet anzeigen.
Damit ist folgender Ein-Gerät-Aufbau möglich:

```text
Tabletdisplay:     /control
USB-C/HDMI-Ausgang: /projector
```

Androids
[`Presentation`](https://developer.android.com/reference/android/app/Presentation)
ist genau für eine eigene Ansicht auf einem zweiten Display vorgesehen. Für
Capacitor wäre dafür ein kleines Kotlin-Plugin erforderlich, das beim Anstecken
eines Displays eine zweite WebView beziehungsweise Projector-Activity auf dem
ermittelten Display öffnet und Displaywechsel behandelt.

Nicht jedes Android-Tablet mit USB-C unterstützt DisplayPort Alternate Mode.
Das ist ein entscheidendes Beschaffungskriterium. Ein günstiger USB-C-Port kann
nur Laden und USB 2.0 beherrschen. Für den Arcade-Einsatz ist eine kabelgebundene
HDMI-Verbindung einer drahtlosen Projektion vorzuziehen.

## 4. „Starten, Board einschalten, geht“ als Abnahmekriterium

### 4.1 Einmalige Einrichtung

Beim ersten Start darf Folgendes nötig sein:

1. Bluetooth-/Nearby-Devices-Berechtigung erlauben.
2. Gefundenes `SDB-BT` aus einer kurzen Liste auswählen.
3. Optional Projektor anschließen und einmal kalibrieren.
4. Optional das Gerät in den Arcade-/Kioskmodus setzen.

Danach speichert die App keine flüchtige Scanposition, sondern eine
Gerätezuordnung und die bekannten GATT-Merkmale. Falls die Boardadresse nicht
stabil ist, muss weiterhin über Name und Service-UUID gefunden werden.

### 4.2 Jeder weitere Start

Der Normalablauf muss vollständig automatisch sein:

```text
App wird sichtbar
  ↓
Bluetoothstatus prüfen
  ├─ aus: verständlichen Systemdialog zum Einschalten anbieten
  └─ an: bekanntes Board suchen
           ↓
      verbinden und FFF1 abonnieren
           ↓
      Start-/Spielerauswahl freigeben
```

Die Reihenfolge muss egal sein:

- App zuerst, Board danach: Die App bleibt in einer sparsamen Suchschleife.
- Board zuerst, App danach: Die App verbindet unmittelbar beim Start.
- Board verliert Strom: Das Spiel pausiert sichtbar und verbindet erneut.
- Tablet schläft oder rotiert: Zustand und laufendes Spiel bleiben erhalten.
- App-Prozess wird beendet: Der letzte konsistente Zustand wird aus SQLite
  wiederhergestellt.

### 4.3 Sichtbare Zustände

Die Oberfläche benötigt nur wenige, verständliche Hardwarezustände:

- **Board wird gesucht**
- **Board wird verbunden**
- **Board bereit**
- **Board getrennt – Spiel pausiert**
- **Board wird bereits von einem anderen Gerät verwendet**
- **Bluetooth ist ausgeschaltet**
- **Berechtigung fehlt**

Rohe GATT-Fehler, MAC-Adressen oder Exception-Texte gehören in die Diagnose,
nicht in den normalen Spielbildschirm. Nach einem erfolglosen automatischen
Versuch muss es genau eine große Aktion geben: **Erneut verbinden**. Eine
zweite, weniger prominente Aktion öffnet die Boardauswahl.

## 5. Technische Umsetzungsoptionen

### Option A: Capacitor plus Chaquopy und bestehender Python-Core

**Schneller Android-spezifischer Weg, aber nicht die generische Empfehlung.**

Capacitor verpackt die bestehende Weboberfläche. Ein eigenes Kotlin-Plugin
liefert BLE, Lifecycle und External Display. Chaquopy bettet Python in die
Android-App ein.

Der Python-Anteil sollte nur diese vorhandenen Komponenten übernehmen:

- `protocol.py`,
- `interpreter.py`,
- `game.py` und `games/`,
- `session.py`,
- `storage.py`,
- soweit nötig plattformneutrale Validation.

Nicht in die Android-Runtime gehören:

- `app.py` als FastAPI-Server,
- Uvicorn,
- Bleak,
- serverbezogene Same-Origin- und WebSocket-Schichten.

Vorteile:

- höchste Wiederverwendung der getesteten Spielregeln,
- alle vorhandenen Modi stehen früh zur Verfügung,
- SQLite und Recovery können weitgehend erhalten bleiben,
- macOS-/Linux-Host und Android-App verwenden zunächst denselben Python-Core,
- ein realistischer Hardwareprototyp ist vor einer großen Sprachmigration
  möglich.

Risiken:

- JavaScript, Kotlin und Python erzeugen drei Debugging-Ebenen,
- Python-Aufrufe müssen strikt seriell und außerhalb des UI-Threads erfolgen,
- Python-Lifecycle und Datenbank müssen bei Activity-Neuerstellung stabil
  bleiben,
- App-Größe und Startzeit steigen,
- jede Python-Abhängigkeit muss auf Android verfügbar sein,
- die heutige dynamische Modulerkennung muss im gepackten Python-Bundle geprüft
  werden,
- diese Lösung hilft einer späteren eigenständigen iOS-App nur begrenzt.

Chaquopy kann reine Python-Pakete und eine Auswahl nativer Pakete einbinden.
Die aktuelle Spiellogik verwendet überwiegend Python-Standardbibliothek. Durch
das Weglassen von FastAPI, Pydantic, Uvicorn und Bleak wird das
Abhängigkeitsrisiko deutlich reduziert. Vor einer Festlegung muss ein
Minimalprojekt mit `GameEngine`, `SessionController` und SQLite auf einem
echten Tablet gebaut werden. Siehe
[Chaquopy-Dokumentation](https://chaquo.com/chaquopy/documentation/) und
[Python auf Android](https://docs.python.org/3/using/android.html).

Der heutige `GameRegistry` sucht Modi über `pkgutil.iter_modules`. Falls diese
Suche in der gepackten Python-Umgebung nicht zuverlässig arbeitet, sollte der
Build eine statische Registry aus den vorhandenen Modulen generieren. Das
ändert die Modularität im Repository nicht: Neue Modi bleiben eigenständige
Module, werden aber beim signierten App-Build explizit in ein Manifest
aufgenommen.

### Option B: Capacitor plus gemeinsamer TypeScript-Spielkern

**Mögliche Web-first-Variante, aber nicht die gewählte Gesamtarchitektur.**

Die Spielregeln, Sessions und der lokale Zustand werden nach TypeScript
portiert. Capacitor stellt BLE und Speicher auf Android und iOS bereit; Browser
und Desktop verwenden passende Adapter. Dieses Modell entspricht am ehesten
der beobachtbaren Silent-Shark-Architektur.

Vorteile:

- ein Laufzeitkern für Web, Android und iOS,
- keine eingebettete Python-Runtime auf Tablets,
- direkte Integration mit der vorhandenen Web-UI,
- weniger Bridge-Aufrufe,
- einfacher für App Stores und Mobile-Lifecycle.

Nachteile:

- großer einmaliger Portierungsaufwand,
- Risiko semantischer Abweichungen bei 24 Spielmodi,
- SQLite, Undo, Zufall und Replay müssen exakt reproduziert werden,
- Python- und TypeScript-Kern existieren während der Migration parallel.

Diese Option sollte erst nach einem funktionierenden BLE-/Display-Prototyp
begonnen werden. Andernfalls würden gleichzeitig Hardware, Packaging und alle
Spielregeln neu gebaut.

### Option C: native Kotlin-/Compose-App

Eine vollständig native Android-App wäre technisch robust, würde aber die
gesamte touch- und projektoroptimierte Weboberfläche neu implementieren. Für
die gewünschte gemeinsame Codebasis mit iOS, Web und macOS ist dies derzeit
nicht sinnvoll.

### Option D: lokaler FastAPI-Server im Tablet

Python, Uvicorn und FastAPI könnten theoretisch innerhalb einer Android-App
gestartet und aus einer WebView über `localhost` angesprochen werden. Diese
Variante wirkt zunächst wie maximale Wiederverwendung, bringt aber unnötige
Probleme mit Ports, Serverstart, App-Suspendierung, Security Policy und
Prozessüberwachung. Die direkte Plugin-Brücke ist einfacher und robuster.

## 6. Capacitor und BLE

Capacitor ist eine Web-first Native Runtime für Android und iOS. Die bestehende
HTML/CSS/JavaScript-Oberfläche kann darin weiterlaufen und über Plugin-Aufrufe
native Funktionen verwenden:
[Capacitor-Dokumentation](https://capacitorjs.com/docs).

Das Community-Plugin
[`@capacitor-community/bluetooth-le`](https://github.com/capacitor-community/bluetooth-le)
unterstützt Android und iOS als BLE-Central und bietet Scan, Connect, Service
Discovery, Notifications, Read und Write. Es ist ein sinnvoller Ausgangspunkt
für einen Prototyp.

Für den Arcade-Produktionsbetrieb sollte trotzdem geprüft werden, ob ein
kleines projektspezifisches Plugin langfristig besser ist. Smart Dartboard
benötigt nur ein enges Profil:

- ein Gerätename,
- ein Service,
- eine primäre Notify-Characteristic,
- definierte Reconnect-Regeln,
- klare Statusereignisse.

Ein eigener, kleiner Adapter vermeidet unnötige API-Fläche und erlaubt eine
gezielte Hardware-Testmatrix. Decoder und Regeln bleiben davon getrennt.

## 7. Projektor, Sound und Tablet-Hardware

Für die Beschaffung eines geeigneten Tablets sind wichtiger als reine
CPU-Benchmarks:

- zuverlässiges BLE,
- USB-C mit DisplayPort Alternate Mode oder ein dokumentierter HDMI-Ausgang,
- gleichzeitiges Laden und HDMI über ein geeignetes Dock,
- Querformatbetrieb ohne Überhitzung,
- ausreichend heller und großer Touchscreen,
- mindestens mehrere Jahre Sicherheitsupdates,
- stabiler WebView-/GPU-Treiber für CSS, SVG, Canvas und Web Audio,
- deaktivierbare Akkuoptimierung für die App,
- Ethernet am Dock als optionale Wartungsschnittstelle,
- physisch sichere Strom- und Displayverbindungen.

Vor dem Kauf einer Geräteflotte muss ein Exemplar mit genau dem vorgesehenen
Dock, Netzteil und Projektor getestet werden. Besonders zu prüfen sind:

- Control UI auf dem Tablet und Projector UI gleichzeitig,
- 60-fps-Animationen bei voller Projektorauflösung,
- Touchlatenz während laufender Projektoranimationen,
- Audioausgabe über HDMI sowie optional über Tabletlautsprecher,
- BLE-Reconnect bei an- und abgestecktem Dock,
- Laden unter Dauerlast,
- Wiederanlauf nach Tablet- und Board-Neustart.

Sound sollte logisch der Projector UI zugeordnet bleiben. Falls Web Audio in
einer sekundären WebView auf einzelnen Geräten unzuverlässig ist, kann ein
kleiner nativer Sounddienst dieselben Cue-Definitionen abspielen.

## 8. Persistenz und App-Lifecycle

Der heutige SQLite-Store ist eine gute Grundlage. In der Tablet-App gilt:

- Datenbank nur im privaten App-Datenverzeichnis öffnen,
- genau eine serialisierte Schreibqueue verwenden,
- vor Activity-Neuerstellung und Displaywechsel nicht schließen,
- bei Prozessende auf Transaktionsgrenzen vertrauen,
- Export über Androids Storage Access Framework anbieten,
- regelmäßige lokale Backups oder einen expliziten Export ermöglichen,
- laufende Spiele nach App-Neustart eindeutig als fortsetzbar anzeigen.

Die Python-Runtime, der BLE-Service und die sichtbare Activity dürfen nicht je
eine eigene Kopie des Spielzustands besitzen. Es gibt genau eine Runtime als
Quelle der Wahrheit. Control und Projector rendern nur deren öffentlichen
Zustand.

## 9. Sicherheit und Store-Verteilung

Eine vollständig lokale Tablet-App benötigt keinen offenen HTTP-Port und kein
CORS. Das reduziert die Angriffsfläche gegenüber dem heutigen LAN-Server.

Zu beachten sind dennoch:

- nur das erwartete BLE-Serviceprofil akzeptieren,
- eingehende Pakete weiterhin vollständig validieren,
- keine unkontrollierten Python- oder JavaScript-Spielmodule nachladen,
- Exporte nur nach sichtbarer Benutzeraktion erzeugen,
- Diagnoseinformationen ohne unnötige personenbezogene Daten halten,
- Release-APK/AAB signieren und Schlüssel sicher verwahren,
- Abhängigkeiten und Android-SDK regelmäßig aktualisieren,
- Kiosk-Administrationsfunktionen mit einer geschützten Geste oder PIN
  verlassen können.

Für private Arcade-Geräte kann zunächst ein signiertes APK verteilt werden.
Für Google Play wird ein Android App Bundle verwendet. Neue ausführbare
Spielmodi sollten in beiden Fällen als signiertes App-Update erscheinen;
Artworks und deklarative Inhalte können getrennt behandelt werden.

## 10. Steam und Desktop als spätere Stufe

Steam ist technisch interessant, aber für das primäre Tablet-Ziel nachrangig.
Ein späterer Steam-Build profitiert dennoch von derselben Vorbereitung:

- Web-UI und optional TypeScript-Spielkern bleiben gemeinsam,
- Desktop-Hülle über Tauri oder Electron,
- native BLE-/BlueZ-/CoreBluetooth-Adapter,
- Steam-Achievements nur als zusätzlicher Plattformdienst,
- Tastatur/Maus und Touch bleiben alternative Eingaben,
- lokale Offlinefunktion bleibt erhalten.

Der Steam-Build sollte nicht die Architektur des Tablets diktieren. Zuerst muss
das reale Board mit einem Android-Tablet zuverlässig funktionieren.

## 11. Empfohlener Prototyp

Ein Android-M0-Spike prüft bewusst nur das technische Risiko und ist keine
Produktmigration. Er läuft früh parallel zum Contract-/Fixture-Aufbau, damit
BLE, externe Anzeige und App-Lifecycle bewiesen sind, bevor alle Spielregeln
portiert werden. Der normative Spike verwendet einen minimalen Rust-Zustand und
die vorgesehene Tauri-/Kotlin-Grenze:

1. Minimale Tauri-App mit Control-WebView und Rust-Zählerzustand erzeugen.
2. Auf einem echten Android-Tablet BLE-Berechtigung anfordern.
3. `SDB-BT` finden, verbinden und `FFF1` abonnieren.
4. Ein Rohpaket exakt einmal in die Rust-Runtime übernehmen.
5. Zweite Projector-WebView über HDMI-`Presentation` öffnen.
6. Zustand gleichzeitig und revisionsgeordnet in beide Views verteilen.
7. HDMI trennen, wieder verbinden und den aktuellen Snapshot herstellen.
8. Activity neu erstellen sowie App in Hintergrund und Vordergrund bewegen.
9. Sicherstellen, dass nie eine zweite autoritative Runtime entsteht.
10. Einen installierbaren signierten Debugbuild erzeugen.

Ein separater Capacitor-/Chaquopy-Versuch ist nur ein Rückfall- oder
Vergleichsspike. Falls er ausdrücklich beschlossen wird, gilt für ihn:

1. Minimale Capacitor-App mit bestehender Control UI erzeugen.
2. Auf einem echten Android-Tablet BLE-Berechtigung anfordern.
3. `SDB-BT` finden, verbinden und `FFF1` abonnieren.
4. Rohpakete gegen die vorhandenen Python-Fixtures prüfen.
5. Chaquopy einbinden und den bestehenden Decoder/Interpreter ausführen.
6. `GameEngine`, `SessionController` und SQLite ohne FastAPI starten.
7. CountUp und X01 vollständig über eine Plugin-Brücke bedienen.
8. App und Board in beiden Reihenfolgen starten und Reconnect testen.
9. Zweite Projector-WebView über HDMI-`Presentation` öffnen.
10. Tablet neu starten und laufendes Spiel wiederherstellen.

Erst nach erfolgreichem M0 und dokumentierter Hüllenentscheidung werden
Packaging, vollständige UI-Einbettung und Spielmodi für Android übernommen.

## 12. Test- und Abnahmematrix

Mindestens folgende Fälle werden auf realer Hardware geprüft:

| Fall | Erwartung |
| --- | --- |
| App läuft, Board wird später eingeschaltet | automatische Verbindung ohne Menü |
| Board läuft, App wird später gestartet | automatische Verbindung |
| Board wird während eines Spiels ausgeschaltet | sichtbare Pause, kein Wurfverlust, Reconnect |
| andere App hält die BLE-Verbindung | klare Meldung und erneuter Versuch |
| Bluetooth ist aus | eine verständliche Einschaltaktion |
| Berechtigung wurde abgelehnt | Link zu App-Einstellungen |
| Tablet-App wird kurz in den Hintergrund gelegt | Verbindung bleibt oder wird transparent erneuert |
| Prozess wird vom System beendet | konsistente Wiederherstellung aus SQLite |
| HDMI wird im Spiel getrennt und wieder verbunden | Projector UI kehrt mit korrektem Zustand zurück |
| Tablet wird gleichzeitig geladen und projiziert | kein Entladen oder thermisches Drosseln |
| 100 reale Würfe einschließlich schneller Folgen | keine Duplikate oder verlorenen Treffer |
| Board-Taste und Dartziehen | korrekter Turn-/Hold-Zustand |

Zusätzlich bleiben alle bestehenden Python-Regeltests aktiv. Neue Android-Tests
decken BLE-Adapter, Plugin-Vertrag, Lifecycle und externe Displays ab.

## 13. Entscheidung

Android bleibt für das Ziel „ein Tablet, App starten, Board einschalten,
spielen“ sehr gut geeignet. Es folgt nach dem Linux-Docker-Referenzpfad und den
priorisierten Apple-Apps als weiterer Adapter desselben Rust-Kerns.

Der risikoärmste Weg lautet:

```text
gemeinsamer Kern:
plattformneutraler Rust-Core, zuerst im Linux-Docker-Server bewiesen

Android-App:
Tauri-Web-UI + derselbe Rust-Core + Kotlin BLE/Display

optionaler isolierter Android-Versuch:
Capacitor UI + Kotlin BLE/Display + Chaquopy Python-Core
```

Der bestehende macOS-/Linux-/Pi-Host bleibt parallel erhalten. Er ist weiterhin
für feste Installationen, mehrere Controller und netzwerkbasierte Projektoren
sinnvoll. Das Android-Tablet wird eine zusätzliche lokale Runtime, nicht ein
Ersatz, der den bewährten Hostbetrieb sofort entfernt.

## 14. Externe Referenzen

- [Android Bluetooth-Berechtigungen](https://developer.android.com/develop/connectivity/bluetooth/bt-permissions)
- [Android BLE im Hintergrund](https://developer.android.com/develop/connectivity/bluetooth/ble/background)
- [Android Companion Device Pairing](https://developer.android.com/develop/connectivity/bluetooth/companion-device-pairing)
- [Android Foreground-Service-Typen](https://developer.android.com/develop/background-work/services/fgs/service-types)
- [Android Lock Task Mode](https://developer.android.com/work/dpc/dedicated-devices/lock-task-mode)
- [Android Presentation API](https://developer.android.com/reference/android/app/Presentation)
- [Capacitor](https://capacitorjs.com/docs)
- [Capacitor Community BLE](https://github.com/capacitor-community/bluetooth-le)
- [Chaquopy](https://chaquo.com/chaquopy/documentation/)
- [Python auf Android](https://docs.python.org/3/using/android.html)
