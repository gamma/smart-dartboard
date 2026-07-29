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

## 8. Session- und Screen-Architektur

`SessionController` ist die zentrale, persistente Zustandsquelle. Beide
Oberflächen folgen derselben Screen-State-Machine:

```text
attract → players → game_select → instructions → countdown
        → playing → game_result → game_select
        → session_summary → attract
```

SQLite speichert Spieler, Sessionteilnehmer, Spiele, Würfe, Statistiken,
Runtime-Checkpoints und Projektorkalibrierung. Nach jedem relevanten Event wird
ein vollständiger Checkpoint inklusive Undo-Snapshots geschrieben.

### Wichtige Routen

```text
GET  /control
GET  /projector
GET  /api/bootstrap
GET  /api/health
POST /api/players
POST /api/session/start
POST /api/session/end
POST /api/game/prepare
POST /api/game/start
POST /api/game/live
POST /api/game/next
POST /api/continue
POST /api/next-player
POST /api/undo
POST /api/throw/correct
POST /api/calibration
WS   /ws
```

`POST /api/event` ist ausschließlich für lokale Entwicklung gedacht und bei
aktiviertem BLE standardmäßig gesperrt.

---

## 9. Eventpipeline und Game Engine

BLE-Callbacks legen dekodierte Ereignisse in eine Queue. Dadurch werden Treffer
streng seriell verarbeitet. Die `EventPipeline` verwirft identische
Sequenznummer-/Rohdatenkombinationen innerhalb eines begrenzten Fensters.

Spielregeln liegen als automatisch entdeckte Plugins unter
`sdb_dartboard/games`. Aktuell implementiert:

- Count Up mit 5, 8 oder 10 Runden und automatischer Siegerermittlung
- X01 mit 301/501/701, Straight Out und Double Out
- vollständiger Aufnahme-Bust mit Rücksetzung auf den Startscore der Aufnahme
- Cricket mit Marks, Overflow-Scoring und Gewinnerprüfung
- Miss als regulärer Wurf
- Drei-Dart-Hold, bewusster Spielerwechsel und Undo
- Korrektur eines aktuellen Wurfs mit Neuberechnung der restlichen Aufnahme

Der Plugin-Vertrag und das Hinzufügen neuer Modi sind in
`docs/GAME_PLUGINS.md` dokumentiert.

---

## 10. Projektor

Die spielrelevante Dartboard-Ebene verwendet ein festes
`1000 × 1000`-Koordinatensystem. Eine aus vier Zielpunkten berechnete
Homographie wird als CSS-`matrix3d` angewendet. So werden Position, Skalierung,
Rotation, Keystone und Perspektive softwareseitig korrigiert.

Atmosphärische 3D-Hintergründe bleiben von dieser präzisen Ebene getrennt.
Während des Spiels wird das Artwork des aktiven Themes abgedunkelt und weich
als Kulisse dargestellt. Es wird so ausgerichtet, dass die im Cover enthaltene
Scheibe hinter der kalibrierten SVG-Scheibe verschwindet. Eine mit derselben
Homographie transformierte dunkle Schutzzone hält die echte Scheibe ruhig und
kontrastreich.

Modusspezifische, hochauflösende 3D-Props werden als freigestellte Alpha-WebP-
Sprites aus `web/static/assets/effects/` geladen und bewegen sich ausschließlich
außerhalb der Scheibe: beispielsweise Stoffherzen, Eier, Cookies, Bonbons,
Blöcke, Billardkugeln und Geister-Wisps. Nur Lichtspuren, Konfetti, Blitze und
Sternenflug entstehen direkt per CSS, damit diese Partikeleffekte flüssig
animierbar bleiben. Treffer und Miss lösen kurze Reaktionen aus; das Ergebnis
friert die Kulisse ein. Bei
`prefers-reduced-motion: reduce` bleiben alle Ambient-Animationen automatisch
statisch. Treffersegmente selbst werden weiterhin ausschließlich auf der
SVG-Geometrie hervorgehoben.

Die Sound-Engine nutzt Web Audio und erzeugt getrennte Cues für Treffer,
Double/Triple, Miss, Spielerwechsel, Countdown, Sieg und Boardfehler. Im
Kioskmodus sollte Browser-Autoplay freigeschaltet werden.

---

## 11. Deployment

Empfohlen ist ein Raspberry Pi 4/5 oder Mini-PC mit lokalem Projektor-Browser
und einem Tablet im selben Netzwerk.

```text
Dartboard → BLE → Pi/Mini-PC → HDMI → /projector
                       └─────→ WLAN → /control
```

Docker verwendet Host-Netzwerk, System-DBus und aktuell `privileged: true`, um
BlueZ zuverlässig zu erreichen. `./data` wird persistent nach `/app/data`
eingebunden. Details stehen in `docs/OPERATIONS.md`.

---

## 12. Verbleibende Erweiterungen

1. Langzeittest mit realem Board und kontrollierten Reconnect-Szenarien.
2. Wurf manuell ersetzen statt nur Undo.
3. X01 Legs, Sets, Master Out und Checkout-Vorschläge.
4. Cricket Cut-Throat und No-Score.
5. PWA-/Homescreen-Paket für das Control-Tablet.
6. Optional mehrere benannte Kalibrierungsprofile.

---

## 13. Nicht verwenden / Vorsicht

Nicht blind auf diese Characteristics schreiben:

```text
FFF2
FFF5
FE59 / 8EC90003-F315-4F60-9FB8-838830DAEA50
```

Insbesondere `FE59` ist wahrscheinlich DFU/Firmware-Update und kann das Gerät in einen Bootloader-Zustand versetzen.
