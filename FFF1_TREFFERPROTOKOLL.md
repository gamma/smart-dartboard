# SDB-BT FFF1 Trefferprotokoll: erste Messreihe

Stand: 2026-07-28
Quelle: LightBlue Notify-Log, Characteristic `FFF1`

## Messreihe

Vom Nutzer angegebene Reihenfolge:

1. S20 außen
2. S20 innen
3. Double 20
4. Triple 20
5. 25 / Single Bull
6. 50 / Bullseye
7. 1 außen

Im Log sichtbar sind 8 Notifications. Eine davon ist wahrscheinlich eine doppelte Auslösung bei Bullseye oder ein zusätzlicher Kontakt.

## Rohdaten

| Nr. | Zeit | Treffer laut Reihenfolge | FFF1 Notify Raw | Bytes |
|---:|---|---|---|---|
| 1 | 15:38:06.684 | S20 außen | `<1c000000 05000c00 020e>` | `1c 00 00 00 05 00 0c 00 02 0e` |
| 2 | 15:38:09.174 | S20 innen | `<1d000000 05000a00 020c>` | `1d 00 00 00 05 00 0a 00 02 0c` |
| 3 | 15:38:10.792 | Double 20 | `<1e000000 05000d00 020f>` | `1e 00 00 00 05 00 0d 00 02 0f` |
| 4 | 15:38:11.783 | Triple 20 | `<1f000000 05000b00 020d>` | `1f 00 00 00 05 00 0b 00 02 0d` |
| 5 | 15:38:13.102 | 25 / Single Bull | `<20000000 05000c00 0e1a>` | `20 00 00 00 05 00 0c 00 0e 1a` |
| 6 | 15:38:14.062 | 50 / Bullseye | `<21000000 05000d00 0f1c>` | `21 00 00 00 05 00 0d 00 0f 1c` |
| 7 | 15:38:14.185 | vermutlich doppelte Bullseye-Auslösung | `<22000000 05000d00 0f1c>` | `22 00 00 00 05 00 0d 00 0f 1c` |
| 8 | 15:38:15.382 | 1 außen | `<23000000 05000c01 000d>` | `23 00 00 00 05 00 0c 01 00 0d` |

## Erste Struktur-Hypothese

Jedes Notify-Paket ist 10 Bytes lang:

```text
[0..3]  sequence counter, little endian
[4..5]  event/type constant? bisher immer 05 00
[6..8]  Treffer-/Sensorcode, 3 Bytes
[9]     Prüfsumme / einfache Summe
```

Beispiel:

```text
1c 00 00 00 05 00 0c 00 02 0e
^^          ^^ ^^ ^^^^^^^^ ^^
Counter     Typ  Treffer   Checksum
```

## Prüfsummen-Hypothese

Das letzte Byte scheint die Summe der drei Treffer-Code-Bytes `[6] + [7] + [8]` modulo 256 zu sein.

Beispiele:

- `0c 00 02` → `0c + 00 + 02 = 0e`
- `0a 00 02` → `0c`
- `0d 00 02` → `0f`
- `0b 00 02` → `0d`
- `0c 00 0e` → `1a`
- `0d 00 0f` → `1c`
- `0c 01 00` → `0d`

Das passt für alle bisher beobachteten Pakete.

## Treffer-Code-Mapping bisher

| Treffer | Code-Bytes `[6..8]` | Checksum | Interpretation |
|---|---|---:|---|
| S20 außen | `0c 00 02` | `0e` | äußerer Single-Ring, Sektorcode 20? |
| S20 innen | `0a 00 02` | `0c` | innerer Single-Ring, gleicher Sektorcode |
| D20 | `0d 00 02` | `0f` | Double-Ring, gleicher Sektorcode |
| T20 | `0b 00 02` | `0d` | Triple-Ring, gleicher Sektorcode |
| 25 | `0c 00 0e` | `1a` | Single Bull |
| 50 | `0d 00 0f` | `1c` | Bullseye |
| 1 außen | `0c 01 00` | `0d` | äußerer Single-Ring, Sektorcode 1? |

## Ringcode-Hypothese

Für die 20 sieht es sehr deutlich so aus:

| Ring | Erstes Code-Byte |
|---|---:|
| Single innen | `0a` |
| Triple | `0b` |
| Single außen | `0c` |
| Double | `0d` |

Für `S20 außen` und `S1 außen` beginnt der Code jeweils mit `0c`, was diese Hypothese stützt.

## Noch offen

Die Segment-/Sektorcodierung ist noch nicht vollständig verstanden.

Auffällig:

- Alle 20er-Treffer enden in `... 00 02`.
- `1 außen` ist `0c 01 00`.
- Bull-Codes sind Sonderfälle `0c 00 0e` und `0d 00 0f`.

Möglicherweise sind die letzten zwei Code-Bytes keine einfache Zahl 1–20, sondern eine Matrix-/Sensorposition.

## Nächste empfohlene Messung

Um das Mapping schnell zu knacken, bitte nur Außen-Single-Felder messen, langsam und in Reihenfolge:

```text
S1 außen
S2 außen
S3 außen
S4 außen
S5 außen
...
S20 außen
```

Alternativ zuerst diese Werte:

```text
S1 außen
S2 außen
S3 außen
S4 außen
S5 außen
S10 außen
S15 außen
S20 außen
```

Wichtig: Pro Feld am besten nur ein eindeutiger Druck, dann Log/Screenshot schicken.

Danach messen:

```text
S20 innen
T20
D20
S1 innen
T1
D1
```

Damit kann geprüft werden, ob Ringcode und Sektorcode unabhängig sind.

---

# Ergänzung: Single-außen-Messreihe S1 bis S20

Quelle: LightBlue Notify-Log, Nutzerangabe: Reihenfolge `1` bis `20`, jeweils Single außen.

## Gültige Trefferpakete

Zwischen den gültigen Treffern tauchen zusätzliche Pakete wie `00 00 ee ee` oder `00 00 ff ff` auf. Diese werden vorerst als Sonder-/Fehler-/Release-/Zwischenzustände behandelt und nicht als reguläre Treffer gewertet.

Die gültigen Single-außen-Treffer folgen einem sehr klaren Muster:

| Dartfeld | FFF1 Raw | Treffer-Code `[6..8]` | Checksum |
|---:|---|---|---:|
| S1 außen | `<24000000 05000c01 000d>` | `0c 01 00` | `0d` |
| S2 außen | `<25000000 05000c02 000e>` | `0c 02 00` | `0e` |
| S3 außen | `<26000000 05000c03 000f>` | `0c 03 00` | `0f` |
| S4 außen | `<27000000 05000c04 0010>` | `0c 04 00` | `10` |
| S5 außen | `<28000000 05000c05 0011>` | `0c 05 00` | `11` |
| S6 außen | `<2c000000 05000c06 0012>` | `0c 06 00` | `12` |
| S7 außen | `<2e000000 05000c07 0013>` | `0c 07 00` | `13` |
| S8 außen | `<2f000000 05000c08 0014>` | `0c 08 00` | `14` |
| S9 außen | `<31000000 05000c09 0015>` | `0c 09 00` | `15` |
| S10 außen | `<32000000 05000c00 010d>` | `0c 00 01` | `0d` |
| S11 außen | `<33000000 05000c01 010e>` | `0c 01 01` | `0e` |
| S12 außen | `<35000000 05000c02 010f>` | `0c 02 01` | `0f` |
| S13 außen | `<36000000 05000c03 0110>` | `0c 03 01` | `10` |
| S14 außen | `<38000000 05000c04 0111>` | `0c 04 01` | `11` |
| S15 außen | `<39000000 05000c05 0112>` | `0c 05 01` | `12` |
| S16 außen | `<3a000000 05000c06 0113>` | `0c 06 01` | `13` |
| S17 außen | `<3b000000 05000c07 0114>` | `0c 07 01` | `14` |
| S18 außen | `<3d000000 05000c08 0115>` | `0c 08 01` | `15` |
| S19 außen | `<3e000000 05000c09 0116>` | `0c 09 01` | `16` |
| S20 außen | `<40000000 05000c00 020e>` | `0c 00 02` | `0e` |

## Segmentcodierung erkannt

Für reguläre Zahlenfelder besteht der Treffer-Code aus:

```text
[ring_code] [ones_digit] [tens_digit]
```

Beispiele:

```text
S1 außen  -> 0c 01 00
S9 außen  -> 0c 09 00
S10 außen -> 0c 00 01
S11 außen -> 0c 01 01
S20 außen -> 0c 00 02
```

Damit gilt für normale Felder:

```text
field = byte7 + 10 * byte8
```

Falls `field == 0`, ist es kein normales Zahlenfeld oder ein Sondercode.

## Ringcode bestätigt für Single außen

Für Single außen ist der Ringcode:

```text
0c = Single außen
```

Aus der ersten Messreihe ist wahrscheinlich:

```text
0a = Single innen
0b = Triple
0c = Single außen
0d = Double
```

## Sonderpakete

Beobachtet:

```text
<29000000 05000000 eeee> -> Code `00 00 ee`, Checksum `ee`
<30000000 05000000 ffff> -> Code `00 00 ff`, Checksum `ff`
```

Diese Pakete passen formal ebenfalls zur einfachen Prüfsummenregel:

```text
00 + 00 + ee = ee
00 + 00 + ff = ff
```

Mögliche Bedeutung:

- Fehlkontakt
- Loslassen/Release
- ungültiger Treffer
- Miss/kein Wert
- Board-interner Zwischenstatus

Diese Codes sollten im Parser zunächst ignoriert oder als Sonderereignis `unknown`/`invalid` protokolliert werden.

## Decoder-Hypothese Version 1

```python
def decode_sdb_packet(data: bytes):
    if len(data) != 10:
        return None

    seq = int.from_bytes(data[0:4], "little")
    event_type = int.from_bytes(data[4:6], "little")
    ring = data[6]
    ones = data[7]
    tens = data[8]
    checksum = data[9]

    if ((ring + ones + tens) & 0xff) != checksum:
        return {"type": "checksum_error", "seq": seq, "raw": data.hex()}

    # bekannte Sondercodes
    if ring == 0x00 and ones == 0x00 and tens in (0xee, 0xff):
        return {"type": "special", "code": f"00 00 {tens:02x}", "seq": seq}

    # Bull-Sondercodes aus erster Messreihe
    if (ring, ones, tens) == (0x0c, 0x00, 0x0e):
        return {"type": "hit", "field": 25, "multiplier": 1, "label": "SBull", "score": 25, "seq": seq}
    if (ring, ones, tens) == (0x0d, 0x00, 0x0f):
        return {"type": "hit", "field": 25, "multiplier": 2, "label": "DBull", "score": 50, "seq": seq}

    field = ones + 10 * tens

    ring_map = {
        0x0a: ("single_inner", 1, "S"),
        0x0b: ("triple", 3, "T"),
        0x0c: ("single_outer", 1, "S"),
        0x0d: ("double", 2, "D"),
    }

    if ring in ring_map and 1 <= field <= 20:
        ring_name, multiplier, prefix = ring_map[ring]
        return {
            "type": "hit",
            "seq": seq,
            "field": field,
            "ring": ring_name,
            "multiplier": multiplier,
            "label": f"{prefix}{field}",
            "score": field * multiplier,
        }

    return {"type": "unknown", "seq": seq, "code": f"{ring:02x} {ones:02x} {tens:02x}", "raw": data.hex()}
```

## Nächster Test

Zur endgültigen Bestätigung bitte nur wenige Kontrolltreffer messen:

```text
S1 innen
T1
D1
S10 innen
T10
D10
S20 innen
T20
D20
```

Erwartung nach aktueller Hypothese:

```text
S1 innen  -> 0a 01 00
T1        -> 0b 01 00
D1        -> 0d 01 00
S10 innen -> 0a 00 01
T10       -> 0b 00 01
D10       -> 0d 00 01
S20 innen -> 0a 00 02
T20       -> 0b 00 02
D20       -> 0d 00 02
```

---

# Ergänzung: Kontrollmessung Innen/Triple/Double

Quelle: LightBlue Notify-Log, Kontrollmessung nach vorhergesagtem Muster.

## Rohdaten

| Treffer | FFF1 Raw | Treffer-Code `[6..8]` | Ergebnis |
|---|---|---|---|
| S1 innen | `<46000000 05000a01 000b>` | `0a 01 00` | bestätigt |
| T1 | `<47000000 05000b01 000c>` | `0b 01 00` | bestätigt |
| D1 | `<48000000 05000d01 000e>` | `0d 01 00` | bestätigt |
| T10 | `<49000000 05000b00 010c>` | `0b 00 01` | bestätigt |
| D10 | `<4a000000 05000d00 010e>` | `0d 00 01` | bestätigt |
| S20 innen | `<4b000000 05000a00 020c>` | `0a 00 02` | bestätigt |
| T20 | `<4c000000 05000b00 020d>` | `0b 00 02` | bestätigt |
| D20 | `<4d000000 05000d00 020f>` | `0d 00 02` | bestätigt |

Hinweis: In der sichtbaren Messung fehlt vermutlich nur noch `S10 innen`, erwartet wäre:

```text
S10 innen -> 0a 00 01, Checksum 0b
```

## Ringcode final bestätigt

Die Ringcodes sind damit für normale Zahlenfelder bestätigt:

| Ringcode | Bedeutung | Multiplikator |
|---:|---|---:|
| `0a` | Single innen | 1 |
| `0b` | Triple | 3 |
| `0c` | Single außen | 1 |
| `0d` | Double | 2 |

## Feldcodierung final bestätigt

Für normale Zahlenfelder gilt:

```text
field = ones + 10 * tens
```

Dabei sind `ones` und `tens` die Bytes 7 und 8 des Pakets:

```text
[ring] [ones] [tens]
```

Beispiele:

```text
0a 01 00 -> S1 innen
0b 01 00 -> T1
0d 01 00 -> D1
0b 00 01 -> T10
0d 00 01 -> D10
0a 00 02 -> S20 innen
0b 00 02 -> T20
0d 00 02 -> D20
```

## Aktueller Status

Das Basis-Trefferprotokoll für normale Dartfelder ist damit ausreichend verstanden:

```text
Bytes 0..3 : Sequenzzähler, little endian
Bytes 4..5 : Eventtyp/Konstante, bisher 05 00
Bytes 6    : Ringcode
Byte 7     : Einerstelle des Feldes
Byte 8     : Zehnerstelle des Feldes
Byte 9     : Prüfsumme = Byte6 + Byte7 + Byte8 mod 256
```

Für Bull gelten Sondercodes:

```text
25 -> 0c 00 0e
50 -> 0d 00 0f
```

Damit kann ein erster funktionsfähiger Parser implementiert werden.

---

# Ergänzung: Sondercodes / Menübutton

Quelle: LightBlue Notify-Log, Nutzerangabe:

1. Menübutton / Spielerwechsel kurz drücken und loslassen
2. Danach Menübutton lange drücken und loslassen

## Rohdaten

| Nr. | Zeit | Aktion | FFF1 Raw | Code `[6..8]` | Interpretation |
|---:|---|---|---|---|---|
| 1 | 15:54:22.522 | kurzer Druck beginnt | `<4e000000 05000000 ffff>` | `00 00 ff` | Button Press / Menübutton gedrückt |
| 2 | 15:54:22.730 | kurzer Druck loslassen | `<4f000000 05000000 eeee>` | `00 00 ee` | Button Release / losgelassen |
| 3 | 15:54:27.380 | langer Druck beginnt | `<50000000 05000000 ffff>` | `00 00 ff` | Button Press / Menübutton gedrückt |
| 4 | 15:54:28.339 | langer Druck erkannt | `<51000000 05000000 cccc>` | `00 00 cc` | Long Press / langer Menübutton-Druck |
| 5 | 15:54:28.580 | langer Druck loslassen | `<52000000 05000000 eeee>` | `00 00 ee` | Button Release / losgelassen |

## Sondercode-Mapping

Damit sind die zuvor unbekannten Sondercodes wahrscheinlich:

| Code `[6..8]` | Bedeutung |
|---|---|
| `00 00 ff` | Menü-/Next-Player-Button gedrückt |
| `00 00 ee` | Menü-/Next-Player-Button losgelassen |
| `00 00 cc` | langer Buttondruck erkannt |

Auch diese Sondercodes folgen der bekannten Prüfsummenregel:

```text
00 + 00 + ff = ff
00 + 00 + ee = ee
00 + 00 + cc = cc
```

## Parser-Erweiterung

Der Parser sollte diese Codes als Button-Events behandeln:

```python
if (ring, ones, tens) == (0x00, 0x00, 0xff):
    return {"type": "button", "button": "menu", "action": "press", "seq": seq}

if (ring, ones, tens) == (0x00, 0x00, 0xee):
    return {"type": "button", "button": "menu", "action": "release", "seq": seq}

if (ring, ones, tens) == (0x00, 0x00, 0xcc):
    return {"type": "button", "button": "menu", "action": "long_press", "seq": seq}
```

## Bedeutung für die eigene Anwendung

Der Menübutton kann später sinnvoll genutzt werden für:

- nächster Spieler
- Wurf bestätigen
- Runde beenden
- Spiel pausieren
- Menü öffnen
- langer Druck: Reset / Zurück / Spiel abbrechen

Für die Spiel-Engine ist wichtig, zwischen `press`, `long_press` und `release` zu unterscheiden.

---

# Ergänzung: Miss-Code

Quelle: LightBlue Characteristic-Ansicht, Nutzerangabe: Miss 3x.

## Rohdaten

| Nr. | Zeit | Aktion | FFF1 Raw | Code `[6..8]` | Interpretation |
|---:|---|---|---|---|---|
| 1 | 15:59:40.677 | Miss | `0x5300000005000000EEEE` | `00 00 ee` | Miss / kein Treffer |
| 2 | 15:59:47.651 | Miss | `0x5400000005000000EEEE` | `00 00 ee` | Miss / kein Treffer |
| 3 | 15:59:50.634 | Miss | `0x5500000005000000EEEE` | `00 00 ee` | Miss / kein Treffer |

## Wichtige Korrektur

Der Code `00 00 ee` tritt beim Loslassen des Menübuttons auf, aber auch bei einem Miss.

Daher ist `00 00 ee` nicht eindeutig nur Button-Release. Besseres Mapping:

| Code `[6..8]` | Bedeutung |
|---|---|
| `00 00 ff` | Menü-/Next-Player-Button gedrückt |
| `00 00 cc` | langer Buttondruck erkannt |
| `00 00 ee` | neutrales Ereignis: Release / Miss / kein Treffer |

Für die Spiel-Engine kann `00 00 ee` abhängig vom Kontext interpretiert werden:

- Wenn vorher `00 00 ff` kam: Button Release
- Wenn kein Button-Kontext aktiv ist: Miss

## Parser-Kontextlogik

Ein zustandsloser Parser kann `00 00 ee` nur als neutrales Ereignis ausgeben:

```json
{
  "type": "neutral",
  "code": "00 00 ee"
}
```

Ein zustandsbehafteter Parser kann unterscheiden:

```text
Wenn button_down == true und Code == ee -> button release
Wenn button_down == false und Code == ee -> miss
```

## Bedeutung für Spielregeln

Miss kann als eigener Wurf gezählt werden. Beim Button-Release darf dagegen kein Wurf gezählt werden.

Daher muss die Anwendung Kontextzustand speichern.

---

# Ergänzung: Einordnung der übrigen BLE-Services/Characteristics

## Übersicht

Aus LightBlue bekannt:

```text
Device: SDB-BT
Service FFF0
  FFF1: Notify, Read
  FFF2: Write, Write without Response
  FFF4: Notify, Read
  FFF5: Write, Write without Response

Service FE59
  8EC90003-F315-4F60-9FB8-838830DAEA50: Indicate, Write
```

## FFF1

Status: verstanden.

- Liefert Treffer-, Miss- und Button-Events.
- Für eigene Anwendung primärer Datenkanal.
- Subscribe reicht offenbar aus, um Events zu empfangen.

## FFF2

Properties:

```text
Write, Write without Response
```

Wahrscheinliche Bedeutung:

- Steuerkanal passend zu `FFF1`.
- Möglicherweise für Initialisierung, Spielmodus, LEDs, Sound, Heartbeat oder Kommandos von App zu Board.

Aktueller Stand:

- Für Treffererkennung offenbar nicht nötig.
- Nicht blind beschreiben.

Sichere Behandlung:

- Erst dokumentieren.
- Später nur gezielt testen, wenn bekannt ist, welche Befehle sicher sind.
- Idealerweise Writes der Original-App per Android HCI Snoop Log mitschneiden.

## FFF4

Properties:

```text
Notify, Read
```

Mögliche Bedeutung:

- Zweiter Datenkanal.
- Statusdaten, Batterie, Heartbeat, Konfiguration, alternative Events oder Legacy-Kanal.

Sicherer nächster Test:

- `FFF4` subscriben.
- Kein Write nötig.
- Prüfen, ob bei Treffern, Button, Miss oder nach einiger Zeit Daten kommen.
- Read-Wert dokumentieren.

## FFF5

Properties:

```text
Write, Write without Response
```

Wahrscheinliche Bedeutung:

- Steuerkanal passend zu `FFF4`.
- Möglicherweise zweiter UART-/Command-Kanal, Konfigurationskanal oder interner Service.

Aktueller Stand:

- Für Treffererkennung nicht nötig.
- Nicht blind beschreiben.

## FE59

`FE59` ist sehr wahrscheinlich ein Firmware-Update-/DFU-Service, vermutlich Nordic Buttonless Secure DFU.

Characteristic:

```text
8EC90003-F315-4F60-9FB8-838830DAEA50
Properties: Indicate, Write
```

Einordnung:

- Nicht relevant für normale Treffererkennung.
- Dient vermutlich dazu, das Board in einen Firmware-Update-Modus zu versetzen.
- Auf diesen Service sollte nicht geschrieben werden, außer man will bewusst Firmware-Update/DFU analysieren.

Risiko:

- Ein falscher Write könnte das Gerät in einen Bootloader-/DFU-Zustand versetzen.
- Daher vorerst ignorieren.

## Priorisierung

Für die eigene Anwendung reicht zunächst:

```text
FFF0 / FFF1 Notify
```

Optional später untersuchen:

```text
FFF4 Notify/Read
```

Nur mit Vorsicht/reverse-engineering untersuchen:

```text
FFF2 Write
FFF5 Write
```

Nicht anfassen ohne Grund:

```text
FE59 / 8EC90003-F315-4F60-9FB8-838830DAEA50
```
