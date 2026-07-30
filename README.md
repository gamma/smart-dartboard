# Smart Dartboard

Lokale, touch-optimierte Spielhallen-Anwendung für ein Bluetooth-Dartboard vom
Typ `SDB-BT` / SDBplay-kompatibel.

[![Release](https://img.shields.io/github/v/release/gamma/smart-dartboard)](https://github.com/gamma/smart-dartboard/releases/latest)
[![CI](https://github.com/gamma/smart-dartboard/actions/workflows/ci.yml/badge.svg)](https://github.com/gamma/smart-dartboard/actions/workflows/ci.yml)
[![Security](https://github.com/gamma/smart-dartboard/actions/workflows/security.yml/badge.svg)](https://github.com/gamma/smart-dartboard/actions/workflows/security.yml)

Das System liest Treffer per Bluetooth Low Energy, speichert Spieler, Sessions,
Spiele und Würfe dauerhaft und synchronisiert zwei spezialisierte Oberflächen:

- `/control` – Point-and-click-Steuerung für Tablet, Handy oder Touchscreen
- `/projector` – kalibrierbare Projektion, Spielanleitungen, Effekte und Sound

## Projektstatus

`v0.0.1` ist das erste öffentlich installierbare Release. Anwendung,
Multi-Arch-Container und Deployment-Anleitung sind für lokale Test- und
Arcade-Installationen verfügbar. Die reale Hardwareintegration wird derzeit mit
einem King-Darts-SDB-BT-Board geprüft; weitere elektronisch auslesbare Boards
benötigen gegebenenfalls einen eigenen Decoder oder Adapter.

Zum Release:
[Smart Dartboard 0.0.1](https://github.com/gamma/smart-dartboard/releases/tag/v0.0.1)

## Funktionen

- dauerhafte Spielerprofile mit Farbe und Avatar
- mehrere Spiele pro Session
- Sessionwertung mit drei Punkten pro Sieg sowie dauerhafte Gesamtstatistiken
- schnelle Revanche nach Spielende per doppeltem Spielerwechsel-Tastendruck
- wertungsfreier Spielabbruch zurück zur Spielauswahl
- grafische Spielauswahl mit umschaltbarem Playful-Cartoon- und
  Classic-Neon-Artwork-Theme
- eigener Anleitungsbildschirm pro Spielmodus auf beiden Displays
- 24 automatisch entdeckte Spielmodule: Klassiker, Party-, Challenge- und
  Koop-Modi einschließlich Heart Chase, Space Defender, Block Drop und
  DartSweeper
- konfigurierbare Varianten wie Rundenzahl, 301/501/701 und Double Out
- persistenter Spielzustand mit Recovery und Undo nach einem Neustart
- serielle BLE-Verarbeitung und Schutz vor doppelten Notifications
- Drei-Dart-Hold für sicheres Entfernen der Darts
- große Anzeige der aktuellen drei Würfe mit Touch-Korrekturscheibe
- direkt anklickbare Projektorscheibe für Testtreffer ohne Hardware
- perspektivische Vierpunkt-Projektorkalibrierung mit automatischem,
  seitenverhältnisabhängigem Rund-und-mittig-Reset
- synthetisierte Treffer-, Miss-, Wechsel-, Countdown- und Siegersounds
- zentrale Projektor-Soundsteuerung samt Testton im Board-Setup
- Healthcheck und Docker-/Raspberry-Pi-Betrieb

## Schnellstart ohne Dartboard

```bash
python3 -m venv .venv
source .venv/bin/activate
pip install -r requirements.txt
SDB_ENABLE_BLE=0 SDB_ALLOW_TEST_EVENTS=1 \
  uvicorn app:app --host 0.0.0.0 --port 8000
```

Danach öffnen:

```text
http://localhost:8000/control
http://localhost:8000/projector
```

### Automatischer Neustart während der Entwicklung

Für die lokale Entwicklung muss der Server nur einmal gestartet werden:

```bash
./scripts/dev.sh
```

Das Skript deaktiviert BLE, erlaubt Testtreffer und beobachtet Python, HTML,
CSS, JavaScript sowie lokale Artwork-Dateien. Nach einer Änderung startet
Uvicorn automatisch neu; bereits geöffnete Controller- und Projektor-Seiten
laden sich danach ebenfalls automatisch neu.

Host und Port können bei Bedarf überschrieben werden:

```bash
SDB_HOST=127.0.0.1 SDB_PORT=8001 ./scripts/dev.sh
```

## Start mit Dartboard

```bash
source .venv/bin/activate
uvicorn app:app --host 0.0.0.0 --port 8000
```

Optionale Konfiguration:

```bash
SDB_DEVICE_NAME=SDB-BT
SDB_DEVICE_ADDRESS=<BLE_ADDRESS>
SDB_DATA_DIR=/pfad/zu/persistenten/daten
SDB_ALLOW_TEST_EVENTS=0
```

Der Boardstatus wird auf beiden Oberflächen angezeigt. `/api/health` meldet
Datenbank- und BLE-Status.

Für den Arcade-Betrieb sollte `SDB_DEVICE_ADDRESS` gesetzt und Port `8000`
ausschließlich im isolierten Dartboard-Netz erreichbar sein. Browsersteuerung
und WebSocket akzeptieren nur denselben Origin; native Wartungsskripte ohne
`Origin` bleiben im lokalen Netz nutzbar. Weitere Hinweise:
[Betrieb und Sicherheit](docs/OPERATIONS.md).

## Docker

Ein veröffentlichtes Image steht für AMD64 und ARM64 bereit:

```bash
docker pull ghcr.io/gamma/smart-dartboard:0.0.1
```

Für eine persistente Installation:

```bash
cp .env.example .env
# In .env: SDB_VERSION=0.0.1
mkdir -p data
docker compose -f compose.production.yml pull
docker compose -f compose.production.yml up -d
```

Für einen UI-Test ohne Linux-BlueZ, beispielsweise mit Docker Desktop:

```bash
SDB_ENABLE_BLE=0 SDB_ALLOW_TEST_EVENTS=1 \
  docker compose -f compose.production.yml up -d
```

Echte BLE-Hardware im Container benötigt einen Linux-Host mit laufendem BlueZ
und erreichbarem `/var/run/dbus`. Details und Fehlerdiagnose stehen unter
[Betrieb und Sicherheit](docs/OPERATIONS.md).

Für einen lokalen Build aus dem Quellcode:

```bash
docker compose up --build -d
docker compose ps
```

`./data` wird nach `/app/data` eingebunden. Spieler, Sessions und laufende
Spiele bleiben deshalb auch nach einem Container-Neubau erhalten.

## Tests

```bash
python3 -m unittest discover -s tests -v
node --check web/static/app.js
node --check website/app.js
docker compose config --quiet
bash website/build.sh
```

GitHub Actions führt diese Prüfungen bei Pull Requests und Pushes auf `main`
automatisch aus. Zusätzlich wird das Docker-Image gebaut und ohne BLE gegen
Healthcheck, Controller und Projektor getestet. Veröffentlichte GitHub Releases
erzeugen versionierte AMD64-/ARM64-Images in der GitHub Container Registry.

## Dokumentation

- [Technische Architektur](docs/TECHNICAL.md)
- [Betrieb und Kiosk-Setup](docs/OPERATIONS.md)
- [CI, Releases und Deployment](docs/DEPLOYMENT.md)
- [Neue Spielmodule entwickeln](docs/GAME_PLUGINS.md)
- [Party-Modi & visuelle Incentives](docs/PARTY_MODES.md)
- [Cartoon-, Challenge- und Minesweeper-Modi](docs/CARTOON_MODES.md)
- [X01 Checkout- und Setup-Advisor](docs/X01_ADVISOR.md)
- [Reproduzierbare Artwork-Prompts](docs/ARTWORK_PROMPTS.md)
- [Website und Publishing](docs/PUBLISHING.md)

## Lizenzierung

Der Softwarecode, die Konfiguration und die Dokumentation stehen unter der
[Apache License 2.0](LICENSE). Visuelle Assets verwenden einen getrennten,
provenienzbasierten Freigabeprozess in [ASSETS_LICENSE.md](ASSETS_LICENSE.md)
und sind, soweit dort ausdrücklich aufgeführt, unter CC BY-NC-SA 4.0
freigegeben.
Name und Markenauftritt bleiben gemäß [TRADEMARKS.md](TRADEMARKS.md)
vorbehalten. Hinweise zu Abhängigkeiten enthält
[THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).

## Datenfluss

```text
SDB-BT → BLE FFF1 → Decoder → Interpreter → Event Queue
       → Game Plugin → Session Controller → SQLite
       → REST/WebSocket → Control + Projector
```
