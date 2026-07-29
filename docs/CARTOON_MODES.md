# Cartoon & Challenge Game Modes

Stand: 2026-07-29

Dieses Dokument enthält sowohl die verbindlichen V1-Regeln als auch ältere
Ideenskizzen. Bei Widersprüchen gilt ausschließlich die folgende
V1-Entscheidungstabelle. Varianten in den ausführlichen Ideenskizzen darunter
sind nicht Bestandteil von V1.

## Verbindliche V1-Entscheidungen

Alle Modi sind eigenständige Game-Plugins. Ein Koop-Sieg gibt jedem Spieler der
aktuellen Session einen Sieg und drei Sessionpunkte; ein MVP ist rein visuell.
Das gilt ebenfalls für den bereits vorhandenen Koop-Modus `boss_fight`.

| Plugin | Verbindliche Kurzregel |
| --- | --- |
| `heart_chase` | 2–8 Spieler, 2/3/5 Herzen. Drei Darts müssen die aktuelle Turn-Punktzahl strikt übertreffen; sonst geht ein Herz verloren. Die tatsächlich erzielte Punktzahl wird immer die nächste Messlatte. Letzter aktiver Spieler gewinnt. Keine V1-Varianten. |
| `robin_hood` | Drei zufällige Sheriff-Ziele starten das Spiel. Jedes Ziel ist ein eigener Pfeil und kann genau einmal gesplittet werden; Duplikate bleiben getrennt. Standard trifft das exakte Segment, Easy dieselbe Zahl. Fünf Runden, höchste Punktzahl gewinnt. Authentic Robin Hood bleibt zurückgestellt. |
| `dragon_eggs` | Sichtbare Eier geben +30, sichtbare Schuppen −15 und ein persönliches Heat. Beim dritten Heat wird zusätzlich die Hälfte der positiven Punkte des aktuellen Turns abgezogen, danach Heat auf null. Ziele wechseln je Turn. 5/8 Runden, höchste Punktzahl. |
| `ghost_chase` | Exaktes wechselndes Ziel. Treffer 1/2/3 eines Turns geben 40/50/60. Ein erfolgloser Turn setzt Combo zurück und erhöht Escape; nach drei Fehlschlägen zieht der Geist um, wird aber nicht schwerer. Easy/Normal/Hard, 5/8 Runden. |
| `cookie_monster` | Pro Turn: 2 goldene Cookies +50, 3 blaue +25, 4 grüne +10, 3 schimmelige −30. Cookie- und Milch-Props liegen direkt auf den Zielsegmenten; dieselbe grafische Legende erscheint auf Controller und Projektor. Bull-Milch verdoppelt einen positiven Turn oder neutralisiert einen negativen. Drei gute Cookies laden Sugar Rush; der nächste gute Cookie zählt doppelt. 5/8 Runden. |
| `space_defender` | Koop: exakte Schiffe, Ringmultiplikator entspricht Schaden, Bull trifft alle. Bei zehn aktiven Schiffen verliert das Team. Nach der letzten Welle gibt es genau eine Aufräumrunde. Erfolgreiches Team: +3 für alle. |
| `candy_cannon` | Persönliche Ladung bleibt über Turns: Single +1, Double +2, Triple +3, Bull +4. Bei 8–10 wird Bull zum Abzug: Der nächste SBull- oder DBull-Treffer feuert automatisch, gibt +50 und zieht dem führenden Gegner 25 ab (Minimum null; Gleichstand nach Turn-Reihenfolge). Andere Treffer können weiter überladen; über 10 setzt die Ladung auf null. Kein Control-Button. 5/8 Runden, mindestens zwei Spieler. |
| `mini_golf` | Alle spielen dasselbe Loch. Easy: Zahl genügt; Normal: exaktes Single/Double; Hard: exaktes Double/Triple/Bull. Treffer mit Dart 1/2/3 zählt 1/2/3 Schläge, kompletter Fehlschlag 4. 6/9 Löcher, niedrigster Score. |
| `eight_ball` | Exakt zwei Spieler. Spieler 1 räumt Singles 1–7, Spieler 2 Singles 9–15. Richtig +20 und bis maximal drei Darts weiterspielen; falsches Feld oder Miss beendet den Turn. Double Bull gewinnt erst nach Abräumen, zu frühes Double Bull schenkt dem Gegner den Sieg. |
| `block_drop` | Koop ohne Timer auf 5×8 Raster. Vier zusammenhängende Farbbögen steuern links/rechts sowie links/rechts drehen. SBull und DBull setzen den Stein sofort; DBull gibt zusätzlich +25. Standardmäßig darf der Spieler mit verbleibenden Darts am neuen Stein weiterwerfen; optional beendet ein Drop den Zug. Miss macht nichts. Erst nachdem alle Spieler dran waren, fällt der Stein automatisch eine Zeile. Fünf Linien gewinnen, Top-out verliert; Blockpunkte zählen für alle. |
| `dart_sweeper` | Koop auf den 20 Zahlenfeldern. Single deckt das direkte Feld auf; Double zusätzlich einen, Triple zwei sichere Nachbarn. Ein direkter Minentreffer explodiert unabhängig vom Ring. SBull scannt ein, DBull zwei sichere Felder. Kein Flood Reveal. Presets: 3/5/7 Minen und 5/3/2 Leben. Erster Direkttreffer plus unmittelbare Nachbarn sind minenfrei. |

## Gemeinsame Prinzipien

- Eine Regel muss in wenigen Sekunden verständlich sein.
- Ein Turn bleibt grundsätzlich bei bis zu drei Darts und endet danach im bekannten Hold-Zustand.
- Der Projector führt die Spieler visuell: Ziel = Cyan/Grün, Bonus = Gold, Gefahr = Rot/Magenta.
- Jeder Modus benötigt ein klares, großes Feedback nach jedem Dart.
- Wo physische Ereignisse vom Board nicht automatisch erkannt werden können, braucht es eine faire Control-UI-Bestätigung.

---

## 1. Heart Chase / Herzjagd

### Pitch

Jeder Spieler versucht, die Punktzahl des vorherigen Spielers zu **übertreffen**. Wer es nicht schafft, verliert ein Herz — aber seine niedrigere Punktzahl wird sofort zur neuen Messlatte.

Der Modus ist einfach, erzeugt permanente Comebacks und funktioniert mit Anfänger- und Profi-Gruppen.

### Grundeinstellung

```text
Spieler: 2–8
Herzen pro Spieler: 3 (Option: 2 / 3 / 5)
Darts pro Turn: 3
Vergleich: strikt größer als die aktuelle Jagdpunktzahl
```

### Ablauf

1. Der erste Spieler eröffnet mit drei Darts.
2. Seine Turn-Punktzahl wird zur **Jagdpunktzahl**.
3. Der nächste Spieler muss diese Punktzahl mit seiner Aufnahme strikt übertreffen.
4. Bei Erfolg:
   - kein Herzverlust
   - seine Punktzahl wird die neue Jagdpunktzahl
   - sie darf höher sein und die Herausforderung steigern
5. Bei Gleichstand oder Misserfolg:
   - Spieler verliert ein Herz
   - **seine tatsächliche Turn-Punktzahl** wird trotzdem zur neuen Jagdpunktzahl
   - der nächste Spieler muss nur noch diese neue, meist niedrigere Zahl schlagen
6. Wer keine Herzen mehr hat, scheidet aus.
7. Der letzte Spieler mit mindestens einem Herz gewinnt.

### Wichtige Beispiele

```text
Anna eröffnet: 63
Ben macht: 65       -> Erfolg, neue Jagdpunktzahl 65
Clara macht: 60     -> verliert 1 Herz, neue Jagdpunktzahl trotzdem 60
David macht: 60     -> Gleichstand reicht nicht, verliert 1 Herz, neue Jagdpunktzahl 60
Anna macht: 81      -> Erfolg, neue Jagdpunktzahl 81
```

### Edge Cases

```text
Erster Spieler macht 0:
  Nächster Spieler braucht mindestens 1.

Spieler macht 0 und verliert Herz:
  Neue Jagdpunktzahl ist 0.

Letztes Herz verloren:
  Der aktuelle Turn wird noch als Jagdpunktzahl gespeichert,
  der Spieler ist danach jedoch ausgeschieden.
```

### Optionale Varianten

#### Mercy Heart

Ein Spieler kann einmal pro Spiel einen Herzverlust abwehren, wenn er einen Bull trifft.

#### Heartbreak

Miss in allen drei Darts führt unabhängig von Jagdpunktzahl zu zusätzlichem Herzverlust.

#### Golden Heart

Ein zufällig ausgewähltes Turn hat doppelte Herzen-Strafe, aber ein Bull heilt ein Herz.

#### Team Hearts

Teams teilen sich einen Herzvorrat, z. B. fünf Herzen pro Team.

### Projector UI

Muss anzeigen:

```text
AKTUELLE JAGD: 65
BEN MUSS SCHLAGEN
BEN: 45 / 65
♥ ♥ ♡
```

Nach Erfolg:

```text
CHASE BEATEN!
65 → 81
```

Nach Misserfolg:

```text
HEART LOST
Neue Jagd: 60
```

### Overlay

Kein bestimmtes Zielsegment nötig. Der zentrale visuelle Fokus ist ein großes Herz-/Score-Rennen.

```json
{
  "prompt": "Schlag 65!",
  "hearts": {"current": 2, "max": 3},
  "challenge_score": 65,
  "turn_score": 45
}
```

### Mode State

```json
{
  "challenge_score": 65,
  "hearts": {"player-id-a": 3, "player-id-b": 2},
  "eliminated": ["player-id-c"],
  "opening_turn": false
}
```

---

## 2. Robin Hood Hunt

### Begriff und Hardware-Grenze

Ein echter **Robin Hood** im Darts bedeutet, dass ein geworfener Dart im Flight, Shaft oder Barrel eines bereits steckenden Darts landet. Das elektronische Board kann dieses physische Ereignis nicht zuverlässig erkennen: Der zweite Dart registriert häufig keinen normalen Segmentkontakt oder wirkt wie ein Miss.

Daher wird der Modus in zwei klar getrennten Varianten angeboten:

1. **Robin Hood Hunt – digital**: Treffer auf dieselben Zielsegmente als digitale Annäherung.
2. **Authentic Robin Hood – manuell**: Ein echter Dart-im-Dart-Treffer wird über die Control UI als `ROBIN!` bestätigt.

Die allgemein bekannte Robin-Hood-Definition stammt aus dem normalen Dartsport; sie ist kein vom Board geliefertes BLE-Event.

### 2.1 Robin Hood Hunt – digital

#### Pitch

Der vorherige Spieler hinterlässt bis zu drei „Sheriff-Arrows“ auf den Segmenten, die er getroffen hat. Der nächste Spieler jagt exakt diese Segmente. Wer einen Sheriff-Pfeil „spaltet“, stiehlt Punkte.

#### Ablauf

1. Spieler A wirft drei Darts.
2. Seine gültigen Treffer werden als Sheriff-Ziele auf dem Projector markiert.
3. Spieler B bekommt drei Darts und versucht, exakt dieselben Segmenttypen zu treffen:

```text
T20 muss wieder T20 sein
D16 muss wieder D16 sein
S5 darf ein Single 5 sein; innen/außen optional gleich behandeln
```

4. Treffer auf Sheriff-Ziel:

```text
+30 Split-Punkte
+ ursprünglicher Dartwert als Bonus
```

5. Treffer auf ein anderes Feld:

```text
0 Split-Punkte
```

6. Nach dem Turn werden die Treffer von Spieler B zu den neuen Sheriff-Zielen für Spieler C.

#### Varianten

```text
Exact Ring:
  T20 nur mit T20, D16 nur mit D16.

Same Number:
  Jede 20 trifft ein Sheriff-Ziel T20/S20/D20.

Bounty:
  Triple-Sheriff-Pfeile sind +60, Doubles +40, Singles +25.
```

#### Projector

- Sheriff-Ziele: violett/blau mit Pfeil-Icon
- Treffer: goldener Split-Effekt
- nicht passender Treffer: neutral
- nächste Ziele werden nach Hold sichtbar

### 2.2 Authentic Robin Hood – manuell

#### Regeln

- Klassische Punkteregel des gerade laufenden Modus bleibt bestehen.
- Wenn ein Dart physisch in einen bereits steckenden Dart eindringt, drückt ein Bediener in der Control UI `ROBIN!`.
- Der aktuelle Spieler erhält:

```text
+100 Robin-Bonus
Robin Token
```

- Drei Robin Tokens können optional ein verlorenes Herz in Herzjagd heilen oder einmalig einen Bombentreffer neutralisieren.

#### Fairness

Die manuelle Bestätigung muss innerhalb eines kurzen Fensters erfolgen, z. B. bevor der nächste Dart registriert wird.

### Mode State für digitale Variante

```json
{
  "sheriff_targets": ["T20", "D16", "S5"],
  "split_count": {"player-id": 2},
  "round": 3
}
```

---

## 3. Dragon Eggs

### Pitch

Ein Drache bewacht Eier auf dem Board. Spieler sammeln Eier, aber wenn zu viele rote Drachenschuppen getroffen werden, schlüpft der Drache und klaut Punkte.

### Regeln

- 3–6 Eier werden als goldene Felder versteckt.
- Treffer auf Ei:

```text
+30 Punkte
Ei eingesammelt
```

- rote Schuppenfelder sind sichtbar:

```text
-15 Punkte
Heat Meter +1
```

- Bei drei Heat-Punkten:

```text
DRAGON AWAKES
aktueller Turn Score halbiert
Heat Meter reset
```

- Nach einer Aufnahme erscheinen neue Eier.

### Projector

```text
Gold = Ei
Rot = Schuppe
Heat Meter = 0 / 3
```

### Cartoon Incentives

- Ei knackt auf
- kleiner Drache fliegt über Board
- Heat Meter raucht bei 2/3

---

## 4. Ghost Chase

### Pitch

Ein freundliches Geistchen wandert über das Board. Triff sein aktuelles Feld, bevor es verschwindet.

### Regeln

- Ein Segment ist als Geist markiert.
- Exakter Treffer:

```text
+40
Geist springt auf neues Segment
Combo +1
```

- falscher Treffer:

```text
0
Geist bleibt
```

- Miss:

```text
Geist bekommt eine Fluchtladung
```

- Bei drei Fluchtladungen springt der Geist auf einen schwierigeren Double-/Triple-Ring.

### Varianten

```text
Easy: Single-Ziele
Normal: beliebige Ringe
Hard: Double/Triple/Bull
```

### Projector

- weiß/türkis glühender Geist
- Bewegungs-Spur beim Sprung
- Combo als „Ghost Chain“

---

## 5. Cookie Monster

### Pitch

Das Board ist eine Keksdose. Gute Cookies geben Punkte, verdorbene Cookies kosten Punkte, Milch-Bull rettet den Turn.

### Regeln

```text
Gold Cookie: +50
Blue Cookie: +25
Green Cookie: +10
Moldy Cookie: -30
Milk / Bull: verdoppelt aktuellen Turn Score
```

- Farben werden nach jeder Aufnahme gemischt.
- Bei drei guten Cookies in Folge: `SUGAR RUSH`, nächster guter Cookie zählt doppelt.

### Warum sinnvoll

Dies ist eine kindlichere, humorvolle Variante von Color Clash mit klarerem Thema und einer Bull-Sondermechanik.

---

## 6. Alien Invasion

### Pitch

Aliens landen auf Segmenten. Spieler verteidigen gemeinsam die Erde.

### Regeln

- Jede Runde landen 3–5 Aliens als grüne/lila Ziele.
- Treffer auf Alien:

```text
+20 Team Damage
Alien entfernt
```

- nicht entfernte Aliens bleiben und vermehren sich nach dem Turn.
- Bei zehn aktiven Aliens verliert das Team.
- Bull ist ein Laserstrahl und entfernt alle Aliens eines Rings.

### Varianten

```text
Coop Survival
Competitive: bester Alien Hunter gewinnt
Boss Alien: großer HP-Gegner nach jeder dritten Runde
```

### Projector

- kleine UFOs auf Segmenten
- Laser bei Bull
- Invasion Meter

---

## 7. Candy Cannon

### Pitch

Treffer füllen eine Süßigkeitenkanone. Wer den richtigen Moment abpasst, feuert sie auf Gegner oder sammelt einen Jackpot.

### Regeln

- Jeder gültige Treffer lädt die Cannon um seinen Multiplikator:

```text
Single +1
Double +2
Triple +3
Bull +4
```

- Bei Ladung 8–10 wird Bull zum Abzug. Der nächste Single- oder Double-Bull-
  Treffer feuert automatisch.
- Fire:

```text
selbst: +50
Gegner mit höchstem Score: -25
```

- Wer über 10 lädt, überhitzt:

```text
Ladung auf 0
keine Belohnung
```

Auf dem Board werden beide Bull-Ringe bei Feuerbereitschaft deutlich als
`FIRE` markiert. Ein separater Control-Button ist nicht erforderlich.

---

## 8. Mini Golf Darts

### Pitch

Jede Runde ist ein Dartloch. Der Projector zeigt ein Ziel; möglichst wenige Darts zum Treffen gewinnen das Loch.

### Regeln

- Ein Zielsegment ist das Loch.
- Jeder Spieler wirft bis zu drei Darts.
- Treffer beendet sein Loch.
- Wertung:

```text
1 Dart = Birdie / 1 Schlag
2 Darts = Par / 2 Schläge
3 Darts = Bogey / 3 Schläge
kein Treffer = +4 Schläge
```

- Nach 9 Löchern gewinnt der niedrigste Gesamtscore.

### Projector

- Ziel als Golf-Loch/Fahne
- Schlagzähler
- Birdie/Par/Bogey Cartoon-Stamps

---

## 9. Mode-Priorisierung

### Sehr leicht auf bestehender Infrastruktur

```text
Heart Chase
Robin Hood Hunt (digital)
Ghost Chase
Dragon Eggs
Cookie Monster
```

### Benötigt zusätzliche Mode-Actions

```text
Risk It                 -> bereits vorhanden: bank
Authentic Robin Hood    -> robin_confirm
```

### Benötigt Timer oder größere neue Infrastruktur

```text
Time Attack
voller Lightning Timer
bewegte Snake
```

### Empfohlene Implementierungsreihenfolge

1. Heart Chase
2. Robin Hood Hunt (digital)
3. Ghost Chase
4. Dragon Eggs
5. Mini Golf Darts
6. Alien Invasion
7. Candy Cannon
8. Authentic Robin Hood Confirmation

---

## 10. Acceptance Criteria

### Heart Chase

- Herzen pro Spieler sichtbar
- Jagdpunktzahl sichtbar
- striktes Übertreffen, Gleichstand ist Misserfolg
- fehlgeschlagene Punktzahl wird nächste Jagdpunktzahl
- Spieler scheiden bei 0 Herzen aus

### Robin Hood Hunt

- vorheriger Turn erzeugt Sheriff-Ziele
- nächster Spieler kann Ziele digital „splitten“
- Treffer auf Ziel bringt Split-Punkte
- Ziele wechseln turnweise
- optionaler manueller Robin-Bonus ist klar getrennt

### Cartoon Modes allgemein

- Mode liefert Projector Prompt
- Mode nutzt Overlay für Ziel/Gefahr/Bonus
- Mode hat eine klare Punkte-/Siegbedingung
- Mode hat mindestens einen visuellen Moment: Capture, Reveal, Combo, Explosion oder Gewinnanimation

---

## Quellenhinweis Robin Hood

Die reale Darts-Bedeutung von „Robin Hood“ ist ein Dart, der im Dart/Flight eines bereits steckenden Darts landet. Als Hintergrundquellen wurden u. a. konsultiert:

- https://darthelp.com/articles/what-is-a-robin-hood-in-darts/
- https://robinhooddarts.com/pages/how-to-play

Die hier beschriebene digitale `Robin Hood Hunt`-Variante ist eine eigene Spieladaption für ein elektronisches Smartboard und keine offizielle Turnierregel.

---

## 11. Space Defender / Raumschiffe abschießen

### Pitch

Kleine Raumschiffe fliegen oder landen auf Dartsegmenten. Die Spieler verteidigen die Erde, indem sie die markierten Zonen treffen.

Der Modus ist ideal für Projektion: Die Scheibe wird zur Sternenkarte, Treffer sind Laserschüsse und die Gegner bewegen sich von Runde zu Runde.

### Basisregeln

- Pro Runde erscheinen 3–6 Raumschiffe auf Segmenten.
- Jedes Schiff besitzt HP:

```text
Scout: 1 HP
Fighter: 2 HP
Cruiser: 3 HP
Boss-UFO: 5 HP
```

- Treffer auf ein Schiff verursacht Schaden entsprechend dem Ring:

```text
Single: 1 Schaden
Double: 2 Schaden
Triple: 3 Schaden
Bull: 4 Schaden / Flächenlaser
```

- Zerstörte Schiffe geben Punkte:

```text
Scout: +10
Fighter: +25
Cruiser: +50
Boss-UFO: +100
```

- Nicht zerstörte Schiffe bleiben auf dem Board.
- Nach jeder vollständigen Spielerrunde erscheint eine neue Welle.
- Wenn zu viele Schiffe aktiv sind, verliert die Gruppe oder erhält eine Strafwelle.

### Coop-Variante

Alle Spieler teilen sich einen Team-Score und müssen eine festgelegte Zahl Wellen überleben.

```text
Welle 1: 3 Scouts
Welle 2: 4 Scouts + 1 Fighter
Welle 3: 2 Fighter + 1 Cruiser
Welle 4: Boss-UFO
```

### Competitive-Variante

- Jeder zerstörte Gegner zählt nur für den Spieler, der den letzten Schaden verursacht.
- Wer am meisten Alien-Score hat, gewinnt.
- Optional: Assist-Punkte für vorherigen Schaden.

### Spezialmechaniken

#### Laser Bull

Ein Treffer auf Bull schadet allen Schiffen auf dem gleichen Ring:

```text
Single Bull: 1 Schaden an allen Schiffen
Double Bull: 2 Schaden an allen Schiffen
```

#### Shielded Ship

Ein geschütztes Schiff kann nur durch Double oder Triple beschädigt werden.

#### Warp

Wenn ein Spieler ein Schiff verfehlt, darf es auf ein neues Segment warpen.

#### Overcharge

Drei Triple-Treffer in einer Aufnahme laden einen Superlaser:

```text
nächster Treffer +2 Schaden
```

### Projector UI

- Raumschiffe als kleine animierte Sprites auf Segmenten.
- HP-Balken direkt neben/über dem Segment.
- Treffer: Laserstrahl vom Boardrand zum Segment.
- Zerstörung: Explosion, Partikel, Score-Flyout.
- Oben: aktuelle Welle, aktive Gegner, Team-/Spielerscore.

### Overlay-Modell

```json
{
  "prompt": "WELLE 3 – Verteidigt die Erde!",
  "enemies": [
    {"id": "ship-1", "zone": "T20", "hp": 2, "max_hp": 2, "type": "fighter"},
    {"id": "ship-2", "zone": "D16", "hp": 1, "max_hp": 3, "type": "cruiser"}
  ],
  "boss": null,
  "wave": 3
}
```

### Warum technisch sinnvoll

Der Modus nutzt die vorhandene Overlay-/Mode-State-Infrastruktur, benötigt aber zusätzlich mehrere Einheiten pro Segment bzw. einen `enemies`-Overlay-Renderer.

---

## 12. Tetris Darts

### Pitch

Tetris-Steine fallen auf ein Raster rund um oder über der Dartboard-Projektion. Treffer auf bestimmte Segmente drehen, bewegen oder droppen den aktuell fallenden Stein.

Es ist kein präzises Tetris-Ersatzspiel, sondern ein Dart-Arcade-Hybrid: Darts steuern den Stein, das Board entscheidet über Risiko und Geschick.

### Empfehlung: vereinfachtes 5x8-Raster

Ein klassisches 10x20-Tetrisfeld wäre auf Beamer und beim Werfen unnötig komplex. Besser:

```text
5 Spalten
8 Reihen
kleine, klar erkennbare Blöcke
```

### Steuerung über Dartsegmente

Feste, leicht merkbare Steuerzonen:

```text
S1–S5      -> Stein nach links
S6–S10     -> Stein nach rechts
S11–S15    -> Stein drehen
S16–S20    -> Hard Drop
Bull       -> Power Drop / sofortige Sonderaktion
Miss       -> Stein fällt eine Reihe
```

Alternative für Einsteiger: Projektor hebt jeweils nur drei große Aktionszonen hervor:

```text
LINKS
DREHEN
RECHTS
DROP
```

### Turn-Ablauf

1. Ein Tetris-Stein erscheint oben im Raster.
2. Spieler hat bis zu drei Darts, um ihn zu steuern.
3. Jeder Dart löst eine Aktion aus.
4. Nach drei Darts oder Hard Drop landet der Stein.
5. Volle Linien werden gelöscht.
6. Nächster Spieler erhält den nächsten Stein.

### Scoring

```text
Stein sauber platziert: +10
Eine Linie: +50
Zwei Linien: +120
Drei Linien: +250
Vier Linien / Mini Tetris: +500
Bull Power Drop: +25 Bonus
```

### Fehler / Cartoon-Regeln

```text
Miss: Stein rutscht unkontrolliert eine Reihe nach unten
Treffer auf falsche Steuerzone: Aktion passiert trotzdem, aber ohne Bonus
Stein über Oberkante: Spieler verliert ein Herz oder -100 Punkte
```

### Varianten

#### Team Tetris

Alle Spieler bauen an einem gemeinsamen Feld. Ziel: möglichst viele Linien vor Game Over.

#### Battle Tetris

Gelöschte Linien senden „Müllzeilen“ an den nächsten Spieler.

#### Chaos Tetris

Jeder Treffer kann zufällig drehen, spiegeln oder drop auslösen.

### Projector UI

- Das Raster sollte neben oder leicht über dem Board liegen, nicht die reale Scheibe vollständig verdecken.
- Aktueller Stein groß und klar erkennbar.
- Steuersegmente zeigen entsprechende Icons/Pfeile.
- Linienclear: starke horizontale Lichtanimation.

### Hardware-Hinweis

Dieser Modus nutzt nicht die natürliche Punktwertung des Boards. Er interpretiert Feldgruppen als Eingabesteuerung. Das ist absolut möglich, aber eher ein „Dart Controller“-Spiel als klassisches Dart.

---

## 13. Billiard / 8-Ball Darts

### Pitch

Die Dartboard-Segmente sind Taschen bzw. Stoßzonen eines virtuellen Billardtischs. Jeder Treffer spielt einen virtuellen Stoß: Kugeln wandern, fallen in Taschen oder verursachen Fouls.

Die beste Variante ist kein physikalisch perfekter Simulator, sondern ein klarer, zugänglicher Arcade-Billardmodus.

### Variante A: Pocket Hunt – empfohlen

#### Grundidee

Der Projector zeigt einen Billardtisch mit nummerierten Kugeln und sechs Taschen. Jede Kugel besitzt ein zugeordnetes Dartziel.

Beispiel:

```text
Kugel 1 -> S1
Kugel 2 -> S2
...
Kugel 8 -> DBull
```

Der aktuelle Spieler muss seine erlaubte Kugel treffen, um sie virtuell zu versenken.

#### Regeln

1. Zu Beginn werden Kugeln auf Ziele verteilt.
2. Ein Spieler erhält einen Kugeltyp oder eine Zielreihenfolge.
3. Treffer auf das richtige Ziel:

```text
Kugel versenkt
+ Punkte
Spieler bleibt am Tisch / darf weiter
```

4. Treffer auf falsche Kugel:

```text
Foul
nächster Spieler
```

5. Die schwarze 8 darf erst gespielt werden, wenn die eigenen Kugeln weg sind.
6. Wer die 8 korrekt versenkt, gewinnt.

### Variante B: Straight Pool Darts

Einfacher Party-Modus:

- Jede Zahl 1–15 ist eine Billardkugel.
- Treffer auf noch vorhandene Zahl versenkt die Kugel.
- Single/Double/Triple ändern nur Punkte, nicht die Gültigkeit.
- Bull ist die schwarze 8 / Jackpot-Kugel.
- Höchste Punktzahl nach leerem Tisch gewinnt.

### Variante C: Trick Shot

Der Projector zeigt einen virtuellen Winkel-/Bandenpfeil.

- Der Spieler muss ein markiertes Zielsegment treffen.
- Richtiger Treffer löst einen virtuellen Trickshot aus.
- Schwierige Ziele geben mehr Punkte.

### Scoring für Pocket Hunt

```text
Eigene Kugel versenkt: +20
Combo / am Tisch bleiben: +10
Falsche Kugel: -10 und Turn Ende
8-Ball korrekt: +100 / Sieg
8-Ball zu früh: sofortige Niederlage oder -100
```

### Projector UI

- Virtueller Billardtisch als Hauptgrafik.
- Verbleibende Kugeln klar sichtbar.
- Aktuelle Zielkugel pulsiert.
- Bei Treffer rollt Kugel animiert in eine Tasche.
- Foul: rote Kreide-/Scratch-Animation.

### Warum dieser Modus gut ist

- leichte, bekannte Metapher
- sehr gut für Teamplay
- keine Echtzeit-Timer nötig
- kann vollständig aus Boardtreffern abgeleitet werden
- visuell dank Kugelanimationen sehr stark

---

## 14. Priorisierung dieser drei Modi

| Modus | Aufwand | Visual Impact | Party-Faktor | Empfehlung |
|---|---:|---:|---:|---|
| Space Defender | Mittel | Sehr hoch | Sehr hoch | Als nächstes nach bestehenden Arcade-Overlays |
| Tetris Darts | Hoch | Sehr hoch | Hoch | Nach Timer-/Action-Framework |
| Billiard Pocket Hunt | Mittel | Hoch | Hoch | Gute nächste Wahl ohne Timer |

### Empfohlene Reihenfolge

```text
1. Space Defender / Alien Invasion
2. Billiard Pocket Hunt
3. Tetris Darts
```

Space Defender kann viel von `Boss Fight` und `Alien Invasion` wiederverwenden. Billiard Pocket Hunt baut sauber auf Mode-State, Ziel-Overlay und Turn-Hold auf. Tetris braucht ein eigenes Raster-/Action-Modell und sollte deshalb danach kommen.

---

## 15. Ergänzende Cartoon-Ideen in Kurzform

### Pinball Panic

Die Scheibe wird zum Flipperautomaten. Bestimmte Felder sind Bumper, Rampen, Multiball oder Drain.

```text
Triple = Ramp
Double = Bumper
Bull = Multiball
Miss = Drain / Ball verloren
```

### Pirate Plunder

Segmente sind Inseln. Spieler sammeln Kartenstücke, Gold und Schiffs-Upgrades; rote Krakenfelder stehlen Loot.

### Dino Dash

Ein Dinosaurier rennt über nummerierte Segmente. Richtige Treffer lassen ihn springen, falsche Treffer landen ihn im Sumpf.

### Wizard Duel

Treffer laden Zauber:

```text
Singles = Mana
Doubles = Schild
Triples = Feuerball
Bull = Ultimate Spell
```

Spieler greifen sich gegenseitig mit cartoonigen Zaubern an.

### Monster Kitchen

Spieler sammeln Zutaten auf Segmenten und kochen verrückte Monstergerichte. Falsche Zutaten erzeugen Schleim und Minuspunkte.

---

## 16. DartSweeper / Minesweeper Darts

### Pitch

Ein echter Minesweeper-Modus auf der Dartscheibe: Alle Felder sind zu Beginn abgedeckt. Bomben sind unsichtbar. Wer ein Feld trifft, deckt es auf und erhält die Anzahl der Bomben in den angrenzenden Dartboard-Feldern.

Der Projector verwandelt die Scheibe in ein dunkles Minenfeld. Jedes sichere Feld wird dauerhaft sichtbar; Bomben bleiben bis zur Explosion oder bis zum Spielende verborgen.

### Warum der Modus besonders gut passt

- Die Projektion kann Informationen zeigen, die physisch nicht auf der Scheibe liegen.
- Jeder Dart ist eine bewusste Entscheidung: neues Feld aufdecken oder sicheres Wissen nutzen.
- Das Spiel funktioniert ohne Timer und ist spannend für Zuschauer.
- Die Regeln sind aus Minesweeper sofort verständlich.

---

### Board-Graph statt rechteckigem Raster

Eine Dartscheibe ist kein Rechteck. Für Minesweeper wird sie als **Graph aus Zonen** modelliert.

Empfohlene spielbare Zonen:

```text
20 Zahlenfelder × 4 Ringe = 80 Zonen
+ Single Bull
+ Double Bull
= 82 Zonen
```

Die vier Ringe sind:

```text
SI  = Single innen
T   = Triple
SO  = Single außen
D   = Double
```

Beispiele:

```text
SI20
T20
SO20
D20
SBULL
DBULL
```

### Nachbarschaftsmodell

Jede normale Zone hat Nachbarn:

1. gleiche Ringzone der linken Nachbarzahl
2. gleiche Ringzone der rechten Nachbarzahl
3. direkt innerer Ring derselben Zahl, falls vorhanden
4. direkt äußerer Ring derselben Zahl, falls vorhanden
5. optional diagonale Nachbarn, abhängig vom Schwierigkeitsgrad

Beispiel für `T20`:

```text
Ringnachbarn: T5, T1
Innen/Außen: SI20, SO20
Optional diagonal: SI5, SI1, SO5, SO1
```

Wichtig: Die Reihenfolge der Dartboard-Zahlen ist die physische Reihenfolge:

```text
20, 1, 18, 4, 13, 6, 10, 15, 2, 17,
3, 19, 7, 16, 8, 11, 14, 9, 12, 5
```

Für Bull:

```text
SBULL Nachbarn: alle SI-Felder oder nur eine logische Bull-Nachbarschaft
DBULL Nachbar: SBULL
```

### Empfohlene Varianten der Nachbarschaft

#### Arcade / Easy

```text
Nur direkte Nachbarn:
- linker/rechter Ringnachbar
- innerer/äußerer Ring derselben Zahl
```

Dadurch liegen Anzeigezahlen meist zwischen 0 und 4.

#### Classic / Normal

```text
direkte Nachbarn + diagonale Ringnachbarn
```

Dadurch entstehen klassische Minesweeper-artige Zahlen von 0 bis 8.

#### Chaos / Hard

```text
Classic-Nachbarschaft
+ Bomben können nach jedem vollständigen Turn um ein Nachbarfeld wandern
```

---

## Spielziel

### Coop-Variante – empfohlen

Alle Spieler versuchen gemeinsam, alle sicheren Felder aufzudecken, bevor eine festgelegte Anzahl Leben verloren ist.

```text
Team-Leben: 3
Ziel: alle sicheren Felder aufdecken
Bombentreffer: -1 Leben
0 Leben: Game Over
```

### Competitive-Variante

- Jedes sichere, neu aufgedeckte Feld gibt Punkte.
- Bombentreffer kostet Punkte oder Herz.
- Gewonnen hat nach leerem Feldsatz der höchste Score.

### Solo-Variante

Ein Spieler versucht, mit möglichst wenigen Bombenfehlern das Feld zu räumen.

---

## Grundablauf

1. Spiel startet mit verdeckter Scheibe.
2. Spieler trifft ein Dartsegment.
3. Wenn es noch nicht aufgedeckt ist:
   - Sicheres Feld: Zahl wird angezeigt.
   - Bombe: Explosion, Lebenverlust/Strafe.
4. Ein bereits aufgedecktes Feld gibt keinen normalen neuen Aufdeckpunkt.
5. Nach drei Darts: Hold, dann bewusster Spielerwechsel.
6. Spiel endet bei:

```text
alle sicheren Felder aufgedeckt -> Sieg
oder
keine Team-Leben / kein Spielerleben mehr -> Niederlage
```

---

## Score- und Risikoregeln

### Empfohlene Coop-Regeln

```text
Neues sicheres Feld: +10 Team Score
0er-Feld: +20, weil besonders wertvoll
Bombentreffer: -1 Team-Leben
bereits aufgedecktes Feld: 0
```

### Empfohlene Competitive-Regeln

```text
Neues sicheres Feld: +10
0er-Feld: +25
Bombe: -30 und ein Herz weniger
bereits aufgedecktes Feld: 0
```

### Automatisches Flood Reveal – optional

Im klassischen Minesweeper deckt ein 0er-Feld angrenzende sichere Felder automatisch auf.

Für DartSweeper gibt es zwei Optionen:

#### True Minesweeper

```text
0er-Feld löst rekursives Auto-Reveal benachbarter sicherer Zonen aus.
```

Vorteil: echtes Minesweeper-Gefühl.

#### Dart-Arcade

```text
0er-Feld zeigt nur eine 0, gibt aber Bonuspunkte.
Keine automatische Aufdeckung.
```

Vorteil: mehr Darts bleiben relevant.

Empfehlung: Beide als Option anbieten, Standard `Dart-Arcade`.

---

## Bombenverteilung

### Difficulty Presets

| Preset | Bomben | Leben | Nachbarschaft | Empfehlung |
|---|---:|---:|---|---|
| Explorer | 6 | 5 | direkt | Familien/Anfänger |
| Classic | 10 | 3 | inklusive Diagonalen | Standard |
| Expert | 14 | 2 | inklusive Diagonalen | erfahrene Spieler |
| Chaos | 10 | 3 | wandernde Bomben | Party |

### Fairness-Regel: erster Dart sicher

Der erste tatsächlich getroffene Bereich darf nie eine Bombe sein.

Umsetzung:

1. Bomben erst nach dem ersten Dart generieren, oder
2. falls das erste Feld Bombe wäre, Bombe an ein anderes Feld verschieben.

Optional werden auch direkte Nachbarn des ersten Felds bombenfrei gehalten. Das erzeugt einen angenehmen Einstieg wie bei digitalem Minesweeper.

---

## Projector UI

### Verdeckter Zustand

Jede Zone erhält ein dunkles, leicht strukturiertes Overlay:

```text
unaufgedeckt = dunkelblau/grau, kleines ? oder Rastertextur
```

Die reale Segmentstruktur bleibt sichtbar genug, um werfen zu können.

### Sicheres Feld

Nach Treffer:

```text
0 = türkis/grün
1 = blau
2 = grün
3 = amber
4+ = rot/magenta
```

Die Zahl wird groß und zentriert im Segment dargestellt.

### Bombe

```text
roter Flash
Mine/Explosion-Icon
Board-Pulse
Herzverlust sichtbar
```

Bomben bleiben nach Explosion sichtbar, damit die Gruppe daraus lernt.

### Hold

Nach drei Darts:

```text
FELD GESICHERT
X / Y sichere Felder
♥ ♥ ♡
Weiter drücken
```

---

## Control UI

Muss anzeigen:

```text
verbleibende sichere Felder
Bomben / bekannte Bomben
Team-Leben oder Spielerherzen
aktueller Spieler
letzte aufgedeckte Zahl
```

Zusätzliche Bedienaktionen:

```text
Flag setzen / Flag entfernen
Hinweis kaufen (optional)
Modus pausieren
```

### Flag-Mechanik – optional

Da das Board keine separate „Markieren“-Eingabe pro Segment hat, funktioniert Flaggen über Control UI:

1. Bediener wählt `FLAG MODE`.
2. Nächster Dart markiert die getroffene Zone als vermutete Bombe.
3. Flaggen zählen nicht als Aufdeckung.
4. Richtige Flaggen können am Ende Bonus geben.

Für eine erste Version kann Flagging weggelassen werden.

---

## Mode State

```json
{
  "seeded": true,
  "bombs": ["T20", "SO5", "D16"],
  "revealed": {
    "SI20": {"count": 2, "revealed_by": "player-id"},
    "D1": {"count": 0, "revealed_by": "player-id"}
  },
  "exploded": ["D16"],
  "flags": ["T20"],
  "lives": 3,
  "safe_remaining": 69,
  "variant": "arcade"
}
```

## Overlay-Modell

Das bestehende Overlay muss für diesen Modus um Zelleninformationen ergänzt werden:

```json
{
  "prompt": "Räume das Minenfeld!",
  "covered": ["T20", "SO20", "D20"],
  "revealed": [
    {"id": "SI20", "count": 2, "color": "green"},
    {"id": "D1", "count": 0, "color": "cyan"}
  ],
  "mines": ["D16"],
  "flags": ["T20"],
  "lives": 3,
  "safe_remaining": 69
}
```

### Neue Projector-Renderer-Anforderungen

- `covered` muss Segment abdunkeln.
- `revealed` muss Zahl in Segment zeichnen.
- `mines` zeigt Bombe erst nach Explosion oder Game End.
- `flags` zeigt kleines Flaggen-Icon.
- bei `count == 0` optional Flood-Reveal-Animation.

---

## Ereignisbeispiele

### Sicheres Feld

```text
Spieler trifft T20
T20 war sicher
Nachbarn: 2 Bomben

Projector:
T20 zeigt große "2"
+10
```

### Bombe

```text
Spieler trifft D16
D16 war Bombe

Projector:
BOOM!
-1 Herz
D16 bleibt als Mine sichtbar
```

### Spielgewinn

```text
alle sicheren Felder aufgedeckt

Projector:
MINENFELD GERÄUMT!
Konfetti / Team Score / verbleibende Herzen
```

---

## Technische Machbarkeit

Der Modus ist sehr gut machbar, benötigt aber mehr Board-Overlay-Logik als Target Rush oder Color Clash:

```text
- Dartboard-Graph / Nachbarfunktion
- persistente verdeckte/aufgedeckte Zonen
- Zahl-Rendering in SVG-Segmenten
- optionaler Flood-Fill
- optionales Flag-Action-System
```

Er sollte deshalb nach den einfachen Party-Modi, aber vor oder parallel zu Tetris Darts umgesetzt werden.

### Empfohlene Umsetzungsschritte

1. `dart_sweeper.py` als Plugin.
2. Board-Graph mit direkter Nachbarschaft implementieren.
3. verdeckte/revealed Bomben im `mode_state`.
4. Projector-SVG um Segmentzahl-Labels erweitern.
5. Coop-Leben und first-click-safe Regel.
6. später Flagging und Flood-Reveal.
