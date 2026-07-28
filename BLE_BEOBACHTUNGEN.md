# BLE-Beobachtungen: SDB-BT Dartboard

Stand: 2026-07-28
Quelle: Screenshots aus BLE-Scanner-App auf iOS

## Gerät

- Anzeigename / Local Name: `SDB-BT`
- iOS Peripheral UUID: `E23BDC58-1310-A3F6-035E-2151CB175787`
- Connectable: Ja
- Advertised Service UUID: `FFF0`
- Advertisement-Feld `kCBAdvDataLeBluetoothDeviceAddress`:
  - Länge: 7 Bytes
  - Bytes: `58 ce 97 0b 54 c8 01`

Hinweis: Die iOS Peripheral UUID ist keine echte BLE-MAC-Adresse, sondern eine von iOS vergebene Identifier-UUID. Das Feld `kCBAdvDataLeBluetoothDeviceAddress` enthält wahrscheinlich eine Geräteadresse oder herstellerspezifische Adressinformation.

## Services und Characteristics

### Service `0xFFF0`

Wahrscheinlich proprietärer Hauptservice des Dartboards.

| Characteristic | Properties | Vermutete Bedeutung |
|---|---|---|
| `0xFFF1` | Notify, Read | möglicher Datenkanal vom Board zur App |
| `0xFFF2` | Write, Write without Response | möglicher Steuer-/Initialisierungskanal von App zum Board |
| `0xFFF4` | Notify, Read | weiterer möglicher Datenkanal vom Board zur App |
| `0xFFF5` | Write, Write without Response | weiterer möglicher Steuer-/Initialisierungskanal von App zum Board |

### Service `0xFE59`

| Characteristic | Properties | Vermutete Bedeutung |
|---|---|---|
| `8EC90003-F315-4F60-9FB8-838830DAEA50` | Indicate, Write | wahrscheinlich Firmware-/DFU-Funktion |

`0xFE59` ist sehr wahrscheinlich ein Device-Firmware-Update-Service, bekannt aus Nordic Secure DFU-Kontexten. Dieser Service ist für das Trefferprotokoll vermutlich nicht relevant und sollte zunächst nicht beschrieben werden.

## Interpretation

Die gefundenen UUIDs bestätigen sehr stark, dass das Board über BLE/GATT arbeitet.

Das relevante proprietäre Protokoll liegt höchstwahrscheinlich auf Service `FFF0`.

Die wahrscheinlichsten Kanäle sind:

- `FFF1` und/oder `FFF4`: Notifications mit Trefferdaten oder Statusdaten
- `FFF2` und/oder `FFF5`: Write-Kanäle für Initialisierung, Steuerbefehle, Heartbeat oder Spielmodus

## Nächste Tests

1. In der BLE-Scanner-App `FFF1` öffnen.
2. Notifications aktivieren/subscribe.
3. Dartsegment drücken oder werfen.
4. Prüfen, ob Hex-Daten im Log erscheinen.
5. Dasselbe mit `FFF4` wiederholen.
6. Read-Werte von `FFF1` und `FFF4` notieren, falls vorhanden.
7. Noch keine beliebigen Daten auf `FFF2`, `FFF5` oder `FE59` schreiben.

## Wichtig für Reverse Engineering

Falls bei `FFF1`/`FFF4` keine Daten erscheinen, muss wahrscheinlich eine Initialisierung auf `FFF2` oder `FFF5` geschrieben werden. Diese sollte idealerweise aus einem Android Bluetooth HCI Snoop Log der Original-App SDBplay 2 extrahiert werden.

## Protokoll-Hypothese

Mögliche Struktur:

```text
SDB-BT
  Service FFF0
    FFF1: notify/read  -> Board sendet Daten
    FFF2: write        -> App sendet Befehle
    FFF4: notify/read  -> Board sendet weitere Daten
    FFF5: write        -> App sendet weitere Befehle
```

Nächster Meilenstein: Rohdaten eines konkreten Treffers erfassen, z. B. `S20`, `T20`, `Bull`, `Miss`.
