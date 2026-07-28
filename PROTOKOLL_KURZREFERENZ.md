# SDB-BT FFF1 Protokoll Kurzreferenz

## BLE-Gerät

- Name: `SDB-BT`
- Service: `FFF0`
- Treffer-Notify-Characteristic: `FFF1`

## Paketformat

Alle bisher beobachteten Trefferpakete haben 10 Bytes:

```text
[0..3]  Sequenzzähler, little endian
[4..5]  Eventtyp/Konstante, bisher 05 00
[6]     Ringcode
[7]     Einerstelle des Feldes
[8]     Zehnerstelle des Feldes
[9]     Prüfsumme
```

Prüfsumme:

```text
checksum = (byte6 + byte7 + byte8) & 0xff
```

## Normale Felder

```text
field = byte7 + 10 * byte8
```

## Ringcodes

| Ringcode | Bedeutung | Multiplikator | Label-Präfix |
|---:|---|---:|---|
| `0a` | Single innen | 1 | `S` |
| `0b` | Triple | 3 | `T` |
| `0c` | Single außen | 1 | `S` |
| `0d` | Double | 2 | `D` |

## Beispiele

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

## Bull-Sondercodes

```text
0c 00 0e -> Single Bull / 25
0d 00 0f -> Double Bull / 50
```

## Button-/Menü-Sondercodes

```text
00 00 ff -> Menü-/Next-Player-Button gedrückt
00 00 ee -> Menü-/Next-Player-Button losgelassen
00 00 cc -> langer Buttondruck erkannt
```

Diese Codes folgen ebenfalls der Prüfsummenregel.

## Beispiel vollständiges Paket

```text
<4d000000 05000d00 020f>
```

Dekodiert:

```text
seq = 0x4d = 77
event_type = 0x0005
ring = 0x0d = Double
field = 0 + 10 * 2 = 20
checksum = 0x0d + 0x00 + 0x02 = 0x0f
result = D20, score 40
```
