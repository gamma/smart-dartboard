# Smart Dartboard

Lokale, touch-optimierte Spielhallen-Anwendung für ein Bluetooth-Dartboard vom
Typ `SDB-BT` / SDBplay-kompatibel.

Das System liest Treffer per Bluetooth Low Energy, speichert Spieler, Sessions,
Spiele und Würfe dauerhaft und synchronisiert zwei spezialisierte Oberflächen:

- `/control` – Point-and-click-Steuerung für Tablet, Handy oder Touchscreen
- `/projector` – kalibrierbare Projektion, Spielanleitungen, Effekte und Sound

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

```bash
docker compose up --build -d
docker compose ps
```

`./data` wird nach `/app/data` eingebunden. Spieler, Sessions und laufende
Spiele bleiben deshalb auch nach einem Container-Neubau erhalten.

## Tests

```bash
python3 -m unittest discover -v
node --check web/static/app.js
docker compose config --quiet
```

## Dokumentation

- [Technische Architektur](docs/TECHNICAL.md)
- [Betrieb und Kiosk-Setup](docs/OPERATIONS.md)
- [Neue Spielmodule entwickeln](docs/GAME_PLUGINS.md)
- [Party-Modi & visuelle Incentives](docs/PARTY_MODES.md)
- [Cartoon-, Challenge- und Minesweeper-Modi](docs/CARTOON_MODES.md)
- [X01 Checkout- und Setup-Advisor](docs/X01_ADVISOR.md)
- [Reproduzierbare Artwork-Prompts](docs/ARTWORK_PROMPTS.md)

## Datenfluss

```text
SDB-BT → BLE FFF1 → Decoder → Interpreter → Event Queue
       → Game Plugin → Session Controller → SQLite
       → REST/WebSocket → Control + Projector
```
