# Smart Dartboard

Eigene Web-Anwendung für ein Bluetooth-Smart-Dartboard vom Typ `SDB-BT` / SDBplay-kompatibel.

Ziel ist ein lokales System, das Treffer per Bluetooth Low Energy ausliest, daraus Spiel-Events erzeugt und zwei Web-Oberflächen bereitstellt:

- `/projector` – reine Beamer-/Anzeigeoberfläche
- `/control` – Steueroberfläche für iPad, Handy, Mac oder zweiten Bildschirm

## Aktueller Status

- BLE-Profil des Boards identifiziert.
- Treffer-Notify-Kanal gefunden: Service `FFF0`, Characteristic `FFF1`.
- Trefferprotokoll entschlüsselt.
- Python-Basisimplementierung vorhanden.
- FastAPI-Webserver mit REST/WebSocket vorhanden.
- Erste Control- und Projector-Weboberflächen vorhanden.
- Docker-/Raspberry-Pi-Deployment vorbereitet.

## Schnellstart lokal ohne Bluetooth

```bash
python3 -m venv .venv
source .venv/bin/activate
pip install -r requirements.txt
SDB_ENABLE_BLE=0 uvicorn app:app --host 0.0.0.0 --port 8000
```

Dann öffnen:

```text
http://localhost:8000/control
http://localhost:8000/projector
```

In `/control` können Testevents ohne Dartboard ausgelöst werden.

## Start mit Dartboard

```bash
python3 -m venv .venv
source .venv/bin/activate
pip install -r requirements.txt
uvicorn app:app --host 0.0.0.0 --port 8000
```

Das Backend sucht standardmäßig nach dem BLE-Gerät:

```text
SDB-BT
```

Optional kann ein fester Gerätename oder eine Adresse gesetzt werden:

```bash
SDB_DEVICE_NAME=SDB-BT uvicorn app:app --host 0.0.0.0 --port 8000
SDB_DEVICE_ADDRESS=<BLE_ADDRESS> uvicorn app:app --host 0.0.0.0 --port 8000
```

## Docker

```bash
docker compose up --build
```

Dann im lokalen Netzwerk öffnen:

```text
http://<host>:8000/control
http://<host>:8000/projector
```

Bluetooth im Container läuft über BlueZ/DBus des Hosts. Siehe `docker-compose.yml`.

## Wichtige Dateien

```text
app.py                         FastAPI Backend, REST API, WebSocket
sdb_dartboard/client.py        BLE Client für SDB-BT / FFF1 Notify
sdb_dartboard/protocol.py      Low-Level-Paketdecoder
sdb_dartboard/interpreter.py   Kontextlogik für Miss/Button-Release
sdb_dartboard/game.py          einfache Game Engine / State
sdb_dartboard/ws.py            WebSocket Connection Manager
web/control.html               Steueroberfläche
web/projector.html             Beameroberfläche
web/static/app.js              Frontend-Logik
web/static/style.css           Styling
scan_ble.py                    BLE Scanner
dump_gatt.py                   GATT-Dump
live_dartboard.py              Terminal-Live-Logger
Dockerfile                     Container Build
docker-compose.yml             Deployment mit Bluetooth-Zugriff
docs/TECHNICAL.md              vollständige technische Dokumentation
```

## Hardware-Empfehlung

Für Entwicklung: Mac oder Linux-Rechner.

Für Deployment:

- **Raspberry Pi Zero 2 W**: geeignet als kleines Headless-BLE-Backend.
- **Raspberry Pi 4/5 oder Mini-PC**: empfohlen, wenn Backend, Webserver und Beamer-Browser auf einem Gerät laufen sollen.

Empfohlene Zielarchitektur:

```text
Dartboard → BLE → Pi/Mini-PC Backend → WebSocket → /projector + /control
```

## Technische Dokumentation

Alle Erkenntnisse, Protokolldetails, Architekturentscheidungen und offenen Punkte stehen zentral in:

[docs/TECHNICAL.md](docs/TECHNICAL.md)
