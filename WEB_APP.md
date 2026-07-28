# Web-App Architektur

Die Anwendung ist als lokale Web-App mit zwei Oberflächen aufgebaut:

- `/projector`: reine Beamer-/Anzeigeoberfläche
- `/control`: Steueroberfläche für iPad, Handy, Mac oder zweiten Bildschirm

Das Backend hält den zentralen Spielzustand und verteilt Änderungen per WebSocket.

## Start lokal

```bash
python3 -m venv .venv
source .venv/bin/activate
pip install -r requirements.txt
uvicorn app:app --host 0.0.0.0 --port 8000
```

Dann öffnen:

```text
http://localhost:8000/control
http://localhost:8000/projector
```

Für Entwicklung ohne Bluetooth:

```bash
SDB_ENABLE_BLE=0 uvicorn app:app --host 0.0.0.0 --port 8000
```

Dann können Testevents über Buttons in `/control` ausgelöst werden.

## Docker

```bash
docker compose up --build
```

Dann im Netzwerk öffnen:

```text
http://<pi-hostname-oder-ip>:8000/control
http://<pi-hostname-oder-ip>:8000/projector
```

## Komponenten

```text
app.py                         FastAPI Backend, REST, WebSocket
sdb_dartboard/client.py        BLE Client, subscribes FFF1
sdb_dartboard/protocol.py      Raw Packet Decoder
sdb_dartboard/interpreter.py   Kontextlogik Miss/Button Release
sdb_dartboard/game.py          einfacher Spielzustand CountUp/X01
web/control.html               Steueroberfläche
web/projector.html             Beameroberfläche
web/static/app.js              Frontend-Logik + WebSocket
web/static/style.css           Styling
```

## Aktueller Funktionsstand

- BLE-Anbindung vorbereitet
- FFF1-Protokoll integriert
- Miss/Button-Kontextlogik integriert
- CountUp-Spielmodus rudimentär
- X01-Spielmodus rudimentär
- Spielerwechsel nach 3 Darts
- Button-Press löst Next Player aus
- WebSocket-Live-Updates
- Testevents ohne Board möglich

## Nächste Schritte

1. Auf Mac/Pi mit echtem Board testen.
2. Fehler bei BLE/BlueZ/Docker beheben.
3. Game Engine robuster machen:
   - Undo
   - Korrektur letzter Wurf
   - X01 Bust-Regeln
   - Double Out
   - Checkout-Hinweise
4. Control UI verbessern.
5. Projector UI visuell ausbauen.
6. WebSocket-API für externe Clients stabilisieren.
