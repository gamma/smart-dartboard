# Smart Dartboard – Technische Dokumentation

Stand: 2026-07-28

## 1. Projektziel

Ziel ist eine eigene Anwendung für ein elektronisches Bluetooth-Smart-Dartboard, die Treffer in Echtzeit ausliest und eigene Spiele sowie Beamer-Visualisierungen ermöglicht.

Das System soll zwei getrennte Oberflächen bereitstellen:

- **Projector UI**: reine Anzeige für Beamer/TV, z. B. Spielstand, aktueller Spieler, letzter Treffer, Animationen.
- **Control UI**: Bedienoberfläche für iPad, Handy, Mac oder zweiten Bildschirm, z. B. Spielauswahl, Spieler, Korrekturen, Einstellungen.

Langfristig soll die Anwendung lokal auf einem Raspberry Pi, Mini-PC oder Mac laufen und optional per Docker deploybar sein.

---

## 2. Systemarchitektur

```text
SDB-BT Dartboard
  ↓ Bluetooth Low Energy / GATT
BLE Client
  ↓
Low-Level Protocol Decoder
  ↓
Context Event Interpreter
  ↓
Game Engine / Central State
  ↓
FastAPI Backend
  ↓ REST + WebSocket
Web Frontend
  ├─ /control
  └─ /projector
```

### Hauptkomponenten

```text
app.py                         FastAPI Backend, REST, WebSocket
sdb_dartboard/client.py        BLE Client, verbindet SDB-BT und subscribed FFF1
sdb_dartboard/protocol.py      Paketdecoder für FFF1
sdb_dartboard/interpreter.py   Kontextlogik Miss/Button Release
sdb_dartboard/game.py          Game-State und einfache Spielregeln
sdb_dartboard/ws.py            WebSocket Manager
web/control.html               Steueroberfläche
web/projector.html             Beameroberfläche
web/static/app.js              Frontend-Logik
web/static/style.css           Styling
```

---

## 3. Bekannte Hardware / BLE-Profil

### Gerät

```text
Name: SDB-BT
Bluetooth: BLE / Bluetooth 4.0
Connectable: ja
Advertised Service UUID: FFF0
```

Aus LightBlue/iOS beobachtet:

```text
Peripheral UUID: E23BDC58-1310-A3F6-035E-2151CB175787
Local Name: SDB-BT
Advertisement Service UUID: FFF0
kCBAdvDataLeBluetoothDeviceAddress: 58 ce 97 0b 54 c8 01
```

Hinweis: Die iOS Peripheral UUID ist keine echte BLE-MAC-Adresse, sondern eine iOS-interne Kennung.

### Services und Characteristics

```text
Service FFF0
  FFF1: Notify, Read
  FFF2: Write, Write without Response
  FFF4: Notify, Read
  FFF5: Write, Write without Response

Service FE59
  8EC90003-F315-4F60-9FB8-838830DAEA50: Indicate, Write
```

### Einordnung

- `FFF1`: primärer Datenkanal für Treffer, Miss und Button-Events. Verstanden.
- `FFF2`: möglicher Steuerkanal App → Board. Aktuell nicht nötig, nicht blind beschreiben.
- `FFF4`: zweiter Notify/Read-Kanal. Bisher keine relevanten Daten beobachtet.
- `FFF5`: möglicher weiterer Steuerkanal. Aktuell nicht nötig, nicht blind beschreiben.
- `FE59`: sehr wahrscheinlich Firmware-/DFU-Service, vermutlich Nordic Buttonless Secure DFU. Nicht anfassen, außer Firmware-Update ist explizit Ziel.

---

## 4. FFF1-Paketformat

Alle beobachteten `FFF1`-Notifications haben 10 Bytes.

```text
[0..3]  Sequenzzähler, little endian
[4..5]  Eventtyp/Konstante, bisher immer 05 00
[6]     Ringcode oder Sondercode-Byte 1
[7]     Einerstelle des Feldes oder Sondercode-Byte 2
[8]     Zehnerstelle des Feldes oder Sondercode-Byte 3
[9]     Prüfsumme
```

Prüfsumme:

```text
checksum = (byte6 + byte7 + byte8) & 0xff
```

Beispiel:

```text
<4d000000 05000d00 020f>

4d 00 00 00   05 00   0d 00 02   0f
seq           type    code        checksum
```

Dekodiert:

```text
seq = 0x4d = 77
ring = 0x0d = Double
field = 0 + 10 * 2 = 20
checksum = 0x0d + 0x00 + 0x02 = 0x0f
result = D20, score 40
```

---

## 5. Treffer-Codierung

### Normale Felder

Für normale Zahlenfelder gilt:

```text
field = ones + 10 * tens
```

Dabei sind:

```text
ones = byte7
tens = byte8
```

### Ringcodes

| Ringcode | Bedeutung | Multiplikator | Label-Präfix |
|---:|---|---:|---|
| `0a` | Single innen | 1 | `S` |
| `0b` | Triple | 3 | `T` |
| `0c` | Single außen | 1 | `S` |
| `0d` | Double | 2 | `D` |

### Beispiele

```text
0c 01 00 -> S1 außen
0a 01 00 -> S1 innen
0b 01 00 -> T1
0d 01 00 -> D1
0c 00 01 -> S10 außen
0b 00 01 -> T10
0d 00 01 -> D10
0c 00 02 -> S20 außen
0a 00 02 -> S20 innen
0b 00 02 -> T20
0d 00 02 -> D20
```

### Bull

Bull ist als Sonderfall codiert:

```text
0c 00 0e -> Single Bull / 25
0d 00 0f -> Double Bull / 50
```

---

## 6. Button- und Miss-Codes

Beobachtete Sondercodes:

```text
00 00 ff -> Menü-/Next-Player-Button gedrückt
00 00 cc -> langer Buttondruck erkannt
00 00 ee -> Miss oder Button-Release, kontextabhängig
```

### Kontextproblem bei `00 00 ee`

`00 00 ee` wird in zwei Fällen gesendet:

1. Miss / kein Treffer
2. Loslassen des Menübuttons nach vorherigem Button-Press

Daher benötigt die Anwendung Kontextlogik.

Regel:

```text
Wenn vorher 00 00 ff kam und Button aktiv ist:
  00 00 ee = button release

Wenn kein Button aktiv ist:
  00 00 ee = miss
```

Diese Logik steckt in:

```text
sdb_dartboard/interpreter.py
```

---

## 7. Implementierter Decoder

Der Low-Level-Decoder in `sdb_dartboard/protocol.py` erzeugt aus Rohdaten zunächst strukturierte Events:

```json
{
  "type": "hit",
  "field": 20,
  "ring": "triple",
  "multiplier": 3,
  "label": "T20",
  "score": 60,
  "seq": 123
}
```

Für Button/Miss-Kontext wird danach `EventInterpreter` verwendet.

---

## 8. Web-App

### Routen

```text
GET /control      Steueroberfläche
GET /projector    Beameroberfläche
GET /api/state    aktueller Spielzustand
POST /api/new-game
POST /api/next-player
POST /api/event   Test-/Entwicklungsevent ohne BLE
WS /ws            Live-Updates
```

### Control UI

Gedacht für iPad/Handy/Mac.

Aktuell:

- Spiel starten
- Spieler eingeben
- Count Up / X01 wählen
- Nächster Spieler
- Testevents ohne Board
- Spielstand ansehen
- letztes Event anzeigen

### Projector UI

Gedacht für Beamer/TV.

Aktuell:

- aktueller Spieler
- großer Score
- Darts pro Aufnahme
- Turn Score
- letztes Event
- Verbindungsstatus

---

## 9. Aktuelle Game Engine

Datei:

```text
sdb_dartboard/game.py
```

Aktuell implementiert:

- Spieler
- aktueller Spieler
- Spielzustand
- Throw History
- Count Up rudimentär
- X01 rudimentär
- automatischer Spielerwechsel nach 3 Darts
- Miss zählt als Wurf
- Button Press löst `next_player` aus

Noch offen:

- Undo
- Wurfkorrektur
- Bust-Regeln sauber
- Double Out
- Master Out
- Checkout-Vorschläge
- Cricket
- Persistenz
- Spielkonfigurationen

---

## 10. Deployment

### Lokal ohne BLE

```bash
python3 -m venv .venv
source .venv/bin/activate
pip install -r requirements.txt
SDB_ENABLE_BLE=0 uvicorn app:app --host 0.0.0.0 --port 8000
```

### Lokal mit BLE

```bash
python3 -m venv .venv
source .venv/bin/activate
pip install -r requirements.txt
uvicorn app:app --host 0.0.0.0 --port 8000
```

### Docker

```bash
docker compose up --build
```

Docker nutzt:

```yaml
network_mode: host
privileged: true
volumes:
  - /var/run/dbus:/var/run/dbus
```

Das ist für BLE/BlueZ im Container der pragmatische Ansatz.

### Linux/BlueZ Troubleshooting

```bash
sudo systemctl status bluetooth
sudo systemctl restart bluetooth
bluetoothctl show
rfkill list
sudo rfkill unblock bluetooth
```

Wichtig: Das Dartboard kann wahrscheinlich nur eine Verbindung gleichzeitig halten. LightBlue/SDBplay trennen, bevor das Backend verbunden wird.

---

## 11. Hardware-Empfehlung

### Entwicklung

- Mac oder Linux-Laptop
- Vorteil: Debugging, BLE-Tests, zwei Bildschirme

### Minimal-Deployment

- Raspberry Pi Zero 2 W
- Gut als Headless-BLE-Backend
- Für Beamer-UI und Browser eher knapp

### Empfohlenes All-in-One-Deployment

- Raspberry Pi 4/5 oder Mini-PC
- Backend + Webserver + Browser/Kioskmodus + Beamer auf einem Gerät

Empfohlene Zielarchitektur:

```text
Dartboard → BLE → Pi 5/Mini-PC → HDMI Beamer /projector
                              → iPad/Mac Browser /control
```

---

## 12. Offene technische Punkte

1. Auf echter Linux-/Mac-Hardware mit BLE testen.
2. Prüfen, ob `connect → subscribe FFF1` ohne weitere Initialisierung zuverlässig funktioniert.
3. Reconnect-Verhalten testen.
4. Feste BLE-Adresse optional unterstützen.
5. Logging verbessern.
6. WebSocket-API stabilisieren.
7. Game Engine ausbauen.
8. UI/UX verbessern.
9. Optional PWA-Unterstützung für iPad-Control-UI.
10. Optional Kioskmodus für Projector-UI.

---

## 13. Nicht verwenden / Vorsicht

Nicht blind auf diese Characteristics schreiben:

```text
FFF2
FFF5
FE59 / 8EC90003-F315-4F60-9FB8-838830DAEA50
```

Insbesondere `FE59` ist wahrscheinlich DFU/Firmware-Update und kann das Gerät in einen Bootloader-Zustand versetzen.
