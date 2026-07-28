# KI-Briefing: Smart-Dartboard-Projekt

Du arbeitest an einem Projekt zur Entwicklung einer eigenen Smart-Dartboard-Anwendung.

## Ziel

Eine Desktop-Anwendung soll sich per Bluetooth mit einem elektronischen Smart-Dartboard verbinden, Treffer in Echtzeit auslesen und eigene Dartspiele sowie Beamer-Projektionen ermöglichen.

## Hardware-Kontext

Der Nutzer besitzt ein kompatibles elektronisches Dartboard, das aktuell vermutlich mit der App **SDBplay 2** verwendet wird.

Recherchierte Informationen:

- App: SDBplay 2
- Entwickler/Plattform: Smart Dartboard / Winscend Tech Limited
- Android Package: `com.sdbplay2.sdb`
- iOS Bundle: `com.sdbplay2.sdb`
- Kompatible Geräte vermutlich: SDB C1, Kings Dart/Dartworld C1, Karella CB-Smart
- Handbuchdaten SDB/Kings Dart C1:
  - Bluetooth-Version: Bluetooth 4.0
  - rote LED blinkt: wartet auf Verbindung
  - blaue LED dauerhaft: verbunden
  - App verbindet automatisch nach Einschalten

## Technische Annahme

Das Board verwendet sehr wahrscheinlich Bluetooth Low Energy, BLE.

Das wichtigste technische Ziel ist das Reverse Engineering des BLE-GATT-Protokolls:

1. BLE-Gerät finden.
2. Services und Characteristics dumpen.
3. Notify-Characteristics identifizieren.
4. Rohdaten bei Treffern loggen.
5. Bytefolgen konkreten Dartsegmenten zuordnen.
6. Parser bauen.

## Gesuchte Events

Die Anwendung soll aus Rohdaten strukturierte Events machen, z. B.:

```json
{
  "type": "hit",
  "field": 20,
  "multiplier": 3,
  "label": "T20",
  "score": 60
}
```

## Empfohlener Start am Mac

Verwende Python und `bleak`.

```bash
python3 -m venv .venv
source .venv/bin/activate
pip install bleak
```

Dann erstelle drei Tools:

1. `scan_ble.py`
   - scannt BLE-Geräte
   - zeigt Name, Adresse, RSSI, Advertisement-Daten

2. `dump_gatt.py`
   - verbindet sich mit dem Dartboard
   - listet Services und Characteristics
   - zeigt Properties wie read/write/notify

3. `log_notify.py`
   - abonniert Notify-/Indicate-Characteristics
   - schreibt eingehende Daten mit Zeitstempel als Hex in eine Logdatei

## Wichtig

Noch wurde kein öffentlich dokumentiertes SDBplay-2-Protokoll gefunden. Daher nicht von bekannten UUIDs ausgehen, sondern zuerst scannen und dokumentieren.

## Nächster Erfolgspunkt

Der erste Meilenstein ist erreicht, wenn bei einem Wurf oder manuellen Druck auf ein Dartsegment ein Bytepaket im Notify-Logger erscheint.
