# Smart-Dartboard-Projekt

Ziel: Entwicklung einer eigenen Computer-Anwendung, die ein Bluetooth-Smart-Dartboard direkt anbindet, Treffer in Echtzeit ausliest und eigene Spielmodi sowie Beamer-Projektionen ermöglicht.

## Aktueller Stand

- SDBplay 2 wurde als Referenz-App identifiziert.
- Das Dartboard nutzt sehr wahrscheinlich Bluetooth Low Energy, BLE.
- Öffentlich wurde bisher keine vollständige Protokolldokumentation gefunden.
- Die wichtigste nächste Aufgabe ist Reverse Engineering der BLE-Kommunikation.

## Wichtige Dateien

- `smart-dartboard-technische-dokumentation.md` enthält die aktuelle technische Projektdokumentation.
- `BLE_BEOBACHTUNGEN.md` enthält reale BLE-Daten aus dem iOS-BLE-Scanner.

## Reale BLE-Erkenntnisse

Das Dartboard wurde als BLE-Gerät erkannt:

- Gerätename: `SDB-BT`
- Advertised Service UUID: `FFF0`
- Hauptservice: `FFF0`
- Characteristics:
  - `FFF1`: Notify, Read
  - `FFF2`: Write, Write without Response
  - `FFF4`: Notify, Read
  - `FFF5`: Write, Write without Response
- Weiterer Service: `FE59`, vermutlich Firmware-/DFU-Service

Damit ist bestätigt: Das Board arbeitet über BLE/GATT. Das Trefferprotokoll liegt sehr wahrscheinlich auf Service `FFF0`.

## Nächste technische Schritte am Mac

1. Python installieren oder prüfen:

```bash
python3 --version
```

2. Virtuelle Umgebung anlegen:

```bash
python3 -m venv .venv
source .venv/bin/activate
pip install bleak
```

3. BLE-Scanner schreiben/ausführen.
4. Dartboard einschalten.
5. Gerätename, Services und Characteristics erfassen.
6. Notify-Characteristics abonnieren.
7. Treffer auslösen und Rohdaten loggen.
8. Bytefolgen den Segmenten zuordnen.

## Empfohlene Architektur

```text
Dartboard
  ↓ Bluetooth LE
Python BLE Backend
  ↓ WebSocket/Event API
Frontend / Beamer UI
  ↓
Eigene Spiele, Animationen, Projektion
```

## Übergabe an andere KI / Codex

Eine andere KI sollte zuerst diese Dateien lesen:

1. `README.md`
2. `smart-dartboard-technische-dokumentation.md`
3. `KI_BRIEFING.md`

Danach sollte sie mit dem BLE-Prototyping beginnen.
