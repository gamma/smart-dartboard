# Build- und Plattformmatrix

Die maschinenlesbare Quelle ist
[`PLATFORM_MATRIX.json`](PLATFORM_MATRIX.json). Sie fixiert die Toolchains und
trennt drei Aussagen bewusst voneinander:

- **Build**: Der Quellcode kompiliert für das Ziel.
- **Installation**: Das erzeugte Artefakt wurde auf dem Ziel gestartet.
- **Hardware**: Board, Display, Audio, Reconnect und Langzeitbetrieb wurden mit
  einer benannten Gerätekombination abgenommen.

Nur `hardware: qualified` darf in Website oder Release Notes als
„unterstützt“ bezeichnet werden. Ein grüner Simulator- oder Containerbuild ist
kein Ersatz dafür.

## Festgelegte Toolchains

| Baustein | Version |
| --- | --- |
| Rust / Edition / MSRV | 1.97.1 / 2024 / 1.97.1 |
| Node.js / npm | 26.x / 11.x |
| Python-Bestandspfad | 3.14.x |
| Tauri CLI / Rust-Runtime | 2.11.4 / 2.11.5 |
| Playwright | 1.62.1 |
| rusqlite / gebündeltes SQLite | 0.38.0 / 3.51.1 |
| Apple-Buildbaseline | Xcode 26.6 |

`Cargo.lock` und beide npm-Lockfiles bleiben committed. Rust-Container werden
mit `--locked`, npm-Abhängigkeiten in CI mit `npm ci` gebaut.

## Produktziele

| Reihenfolge | Ziel | Mindeststand | Nachweis | Hardware |
| --- | --- | --- | --- | --- |
| 1 | Linux/Docker, AMD64 + ARM64 | Debian 12 Bookworm im Container | Build und Start ohne BLE | offen |
| 2 | macOS ARM64 | macOS 13 | unsigniertes `.app`-Bundle in CI und lokaler Bundle-Start | offen |
| 3 | iOS/iPadOS | iOS/iPadOS 14 | unsigniertes ARM64-Gerätearchive gebaut; Simulatorbundle installiert, gestartet und Suspend/Resume geprüft | offen |
| 4 | Android ARM64 | minSdk 31, targetSdk 36 | noch nicht implementiert | offen |
| 5 | Windows x64/ARM64 | Windows 10 1809 | noch nicht implementiert | offen |

Das macOS-`.app` wird reproduzierbar ohne Signatur gebaut, lokal direkt
gestartet und in CI sieben Tage als ZIP-Artefakt bereitgestellt. Das beweist
Packaging und Start, aber noch keine Verteilung: Developer-ID-Signatur,
Notarisierung, Stapling und Installation auf einem zweiten Mac bleiben offen.
Während einer aktiven Session verhindert die native App automatischen Display-
und Systemruhezustand; iOS/iPadOS verwendet für denselben Sessionzeitraum den
Idle-Timer. Reale Langzeit- und Akkutests bleiben Teil der Hardwareabnahme.

Die Linux-BLE-Freigabe verlangt einen Host mit BlueZ und erreichbarem
System-D-Bus. Docker Desktop auf macOS oder Windows qualifiziert diesen Pfad
nicht. Für Apple bleiben Signierung, echte CoreBluetooth-Scheibe,
AirPlay/HDMI, Audio, Reconnect und der dokumentierte Soak-Test offen.
Das iOS-`.xcarchive` belegt Geräte-Compilation und Packaging, ist ohne
Development Team, Provisioning und Signatur aber weder installierbare IPA noch
TestFlight-Release.

## Hardwarestatus

Es gibt noch **keine vollständig qualifizierte Kombination** aus Tablet/Mac,
Dartboard, Dock, Projektor und Audioausgang. Das Kings-Dart-/SDB-BT-Board
`B0F6Y5ZJ3F` ist das reale Entwicklungsboard aus dem Bestandspfad; für den
Rust-Core steht seine Hardware-Abnahme noch aus.

Die Matrix wird mit folgendem Befehl gegen Toolchain-, Lockfile-, Xcode-,
Docker- und Workflow-Konfiguration geprüft:

```bash
node scripts/verify-platform-matrix.mjs
```
