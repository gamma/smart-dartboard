# Party Game Modes & Visual Incentives Specification

Stand: 2026-07-28

Dieses Dokument spezifiziert die Party-/Arcade-Ausrichtung des Smart-Dartboard-Projekts. Ziel ist nicht klassisches Dart möglichst nüchtern abzubilden, sondern ein visuelles, zugängliches, reaktionsschnelles und beamer-taugliches Party-Erlebnis zu schaffen.

Die Modi sollen über die bestehende Architektur laufen:

```text
Dartboard BLE Events
  -> Event Interpreter
  -> Game Plugin / Mode Logic
  -> Shared Game State
  -> Board Overlay Model
  -> Projector UI + Control UI
```

---

## 1. Design-Prinzipien für Party-Modi

### 1.1 Sofort verständlich

Ein Modus muss in maximal 30 Sekunden erklärbar sein.

Gute Beispiele:

- „Triff das leuchtende Feld.“
- „Meide die roten Bomben.“
- „Sammle Farben, Gold gibt am meisten.“
- „Riskiere weiter oder sichere deine Punkte.“

Schlechte Beispiele:

- komplizierte Tabellen
- zu viele Sonderregeln
- lange Erklärtexte
- Regeln, die man erst nach mehreren Runden versteht

### 1.2 Visuelle Führung vor Text

Der Beamer soll Spieler leiten. Wenn ein Feld wichtig ist, muss es direkt auf dem Board sichtbar sein.

```text
Target = cyan / grün / pulsierend
Danger = rot / flackernd
Bonus = gold / leuchtend
Disabled = grau / abgedunkelt
Current hit = kurzer starker Pulse
```

### 1.3 Kurze Runden

Party-Modi sollten schnelle Erfolgserlebnisse erzeugen.

Empfehlung:

```text
1 Spiel: 3–8 Minuten
1 Runde pro Spieler: 3 Darts
Spezialrunden: 30–90 Sekunden
```

### 1.4 Comeback-Möglichkeiten

Gute Partyspiele lassen Rückstände aufholbar erscheinen.

Mechaniken:

- zufällige Bonusfelder
- Multiplikator in späteren Runden
- Risiko/Reward
- Powerups für zurückliegende Spieler
- Teamziele
- Jackpot-Felder

### 1.5 Zuschauerfreundlichkeit

Zuschauer müssen jederzeit erkennen:

- wer dran ist
- was das Ziel ist
- was gut/schlecht ist
- was gerade passiert ist
- wer führt

Der Projector ist primär Showfläche, nicht Admin-UI.

---

## 2. Technisches Overlay-Modell

Alle Party-Modi sollten ein gemeinsames Board-Overlay liefern. Die Projector-UI rendert dieses Overlay unabhängig vom konkreten Spielmodus.

### 2.1 Board Zone Identifier

Empfohlene IDs:

```text
S1..S20      Single, egal ob innen/außen falls nicht unterschieden
SI1..SI20    Single innen
SO1..SO20    Single außen
D1..D20      Double
T1..T20      Triple
SBULL        Single Bull / 25
DBULL        Double Bull / 50
BULL         beide Bull-Zonen, falls zusammen gemeint
MISS         Außen-/Miss-Zone
ANY_SINGLE
ANY_DOUBLE
ANY_TRIPLE
ANY_BULL
ANY_FIELD
```

Intern kann weiterhin mit `field`, `ring`, `multiplier` gearbeitet werden. Für UI und Game Specs sind Labels lesbarer.

### 2.2 Overlay State

Ein Game Plugin sollte optional dieses Modell bereitstellen:

```json
{
  "prompt": "Triff T20!",
  "sub_prompt": "Combo x2 aktiv",
  "targets": [
    {"id": "T20", "color": "cyan", "label": "+50", "pulse": true, "priority": 100}
  ],
  "danger": [
    {"id": "D1", "color": "red", "label": "BOMB", "pulse": true}
  ],
  "bonus": [
    {"id": "DBULL", "color": "gold", "label": "JACKPOT"}
  ],
  "disabled": [
    {"id": "T19", "color": "gray", "label": "LOCKED"}
  ],
  "owned": [
    {"id": "S20", "owner_id": "player-1", "color": "#00e5ff"}
  ],
  "timer": {
    "remaining_ms": 12000,
    "total_ms": 20000,
    "danger_at_ms": 5000
  },
  "combo": {
    "count": 2,
    "multiplier": 3
  },
  "effects": [
    {"type": "pulse", "zone": "T20"},
    {"type": "score_flyout", "text": "+50", "zone": "T20"}
  ]
}
```

### 2.3 Overlay Prioritäten

Wenn mehrere Zustände auf ein Segment fallen, gilt:

```text
hit effect > danger > target > bonus > owned > disabled > neutral
```

Beispiel: Wenn ein Bonusfeld gleichzeitig Bombe ist, soll die Bombe visuell gewinnen, außer der Spielmodus definiert ausdrücklich „risk bonus“.

### 2.4 Farbsystem

Empfohlene Standardfarben:

| Bedeutung | Farbe | Verhalten |
|---|---|---|
| Ziel | Cyan / Grün | sanft pulsierend |
| Bonus | Gold | glänzend / schimmernd |
| Gefahr | Rot / Magenta | flackernd, aggressiv |
| Treffer | Weiß/Cyan Flash | kurzer starker Pulse |
| Miss | Rot/Magenta Full-board Pulse | kurzer negativer Flash |
| Hold | Amber | stabil, gut lesbar |
| Disabled | Grau/Blau entsättigt | abgedunkelt |
| Spielerfarben | feste Palette | konsistent pro Spieler |

Spielerfarben:

```text
P1 cyan
P2 magenta
P3 amber
P4 green
P5 violet
P6 orange
```

---

## 3. Pflicht-Incentives / Visual Feedback

Diese visuellen Incentives sollten systemweit vorhanden sein, unabhängig vom Spielmodus.

### 3.1 Hit Pulse

Bei jedem gültigen Treffer:

- Segment leuchtet 500–900 ms stark auf
- Score-Flyout erscheint am Segment oder HUD
- kurzer Sound optional
- letzter Treffer wird groß angezeigt

Beispiel:

```text
T20 getroffen
-> T20 blinkt cyan/weiß
-> +60 fliegt aus Segment
-> HUD zeigt T20
```

### 3.2 Miss Feedback

Bei Miss:

- Board wird kurz rot/magenta überlagert
- HUD zeigt `MISS`
- optional kurzer negativer Sound
- kein langes Bestrafen; Partyfluss erhalten

### 3.3 Combo Meter

Mehrere erfolgreiche Aktionen hintereinander erhöhen Combo.

Beispiel:

```text
Hit target 1 -> Combo x1
Hit target 2 -> Combo x2
Hit target 3 -> Combo x3 + Bonus
Miss -> Combo reset
```

Visuell:

- Combo-Zähler rechts/oben
- zunehmender Glow
- bei Combo-Milestones kurzer Burst

### 3.4 Streak Rewards

Systemweite Awards:

```text
3 Treffer in Folge: Hot Hand
3 Ziele in Folge: Precision
Bull Treffer: Bullseye!
Triple Treffer auf Ziel: Perfect!
Comeback Treffer: Clutch!
```

Diese Awards müssen nicht direkt Spielpunkte ändern, können aber visuell motivieren.

### 3.5 Player Entrance

Wenn ein Spieler an der Reihe ist:

- Name groß
- Spielerfarbe pulst
- Board kurz in Spielerfarbe umranden
- optional Avatar/Emoji

### 3.6 Turn Hold Screen

Nach 3 Darts:

- `HOLD` / `Aufnahme beendet – Weiter drücken`
- Zusammenfassung der 3 Darts
- Turn Score
- Button-Hinweis: Board-Button oder Control-Button

Beispiel:

```text
Anna
T20 · S5 · MISS
Turn: 65
Weiter drücken
```

### 3.7 Winner Moment

Sieg darf nicht nüchtern sein.

Pflicht:

- Fullscreen Winner Overlay
- Konfetti/Particles
- Gewinnerfarbe
- letzter entscheidender Treffer
- Ranking/Scoreboard

### 3.8 Near Miss / Almost

Für Party-Modes mit Zielsegmenten kann ein falsches, aber nahes Segment als „Almost“ markiert werden.

Beispiel:

```text
Ziel T20, getroffen S20 -> Almost + kleine Punkte
Ziel D16, getroffen D8 -> Wrong double, no bonus
```

Muss pro Spielmodus definiert werden.

---

## 4. Game Mode Specs

## 4.1 Target Rush

### Kurzbeschreibung

Triff das aktuell leuchtende Ziel. Nach einem Treffer erscheint sofort ein neues Ziel.

### Zielgruppe

Alle Spieler, auch Anfänger. Perfekter erster Party-Modus.

### Regeln

- Pro Spieler 3 Darts pro Turn.
- Das Spiel zeigt ein aktives Zielsegment.
- Treffer auf exakt dieses Segment gibt volle Punkte.
- Treffer auf das richtige Zahlenfeld, aber falschen Ring, gibt kleine „Almost“-Punkte.
- Miss gibt 0 und bricht Combo.
- Nach Zieltreffer wird sofort ein neues Ziel generiert.

### Scoring

Empfohlen:

```text
Exact target: +50
Same field wrong ring: +10
Bull exact: +75
Miss: 0
Combo bonus: +10 pro zusätzlichem Treffer in Serie
```

Alternative für Anfänger:

```text
Any hit on target number: +25
Exact ring: +50
```

### Overlay

```json
{
  "prompt": "Triff T20!",
  "targets": [{"id": "T20", "color": "cyan", "label": "+50", "pulse": true}],
  "combo": {"count": 1, "multiplier": 1}
}
```

### Visual Incentives

- Ziel pulsiert cyan.
- Exact hit erzeugt weißen Flash und `PERFECT!`.
- Almost hit zeigt gelbes `ALMOST +10`.
- Combo x3 erzeugt kurzen Board-Ring-Pulse.

### Difficulty Settings

```text
Easy: nur Single/ganze Zahlenfelder
Normal: Singles + Doubles + Bull
Hard: Triples + Doubles + Bull
Chaos: Ziel wechselt auch nach Miss
```

### Offene Edge Cases

- Wenn Ziel `T20`, zählt `S20 innen` und `S20 außen` beide als same field.
- Bull kann als `SBULL` oder `DBULL` getrennt oder gemeinsam verwendet werden.

---

## 4.2 Avoid the Bomb

### Kurzbeschreibung

Sammle Punkte, aber vermeide rote Bombenfelder.

### Regeln

- Normale Treffer geben ihren Dartwert.
- Mehrere Bombensegmente sind rot markiert.
- Treffer auf Bombe löst Strafe aus.
- Bomben können pro Turn oder pro Dart wechseln.

### Scoring

Empfohlen:

```text
Normal hit: +score
Bomb hit: -50 und Turn endet optional
Miss: 0
Bull: +25/+50, außer Bull ist Bombe
```

Varianten:

```text
Soft Bomb: -25
Hard Bomb: -100
Sudden Bomb: aktueller Turn Score geht verloren
Party Bomb: alle anderen bekommen +20
```

### Overlay

```json
{
  "prompt": "Sammle Punkte – meide Rot!",
  "danger": [
    {"id": "D1", "color": "red", "label": "BOMB", "pulse": true},
    {"id": "T5", "color": "red", "label": "BOMB", "pulse": true}
  ]
}
```

### Visual Incentives

- Bomben flackern rot.
- Bombentreffer: Explosion, Screen Shake, roter Flash.
- Knapp vorbei: kleine Fun-Meldung `Lucky!`.
- Hohe Punkte ohne Bombe: `Clean Run`.

### Difficulty Settings

```text
Easy: 2 Bomben, wechseln pro Turn
Normal: 4 Bomben, wechseln pro Turn
Hard: 6 Bomben, wechseln nach jedem Dart
Hidden: Bomben werden nur 3 Sekunden gezeigt
```

---

## 4.3 Color Clash

### Kurzbeschreibung

Das Board ist farbig. Farben bestimmen Punkte, nicht klassische Dartwerte.

### Regeln

- Segmente werden zufällig eingefärbt.
- Jede Farbe hat Punktewert oder Strafe.
- Alle Spieler erhalten innerhalb einer Runde dieselben Farbchancen.
- Im Rundenmodus bleibt ein gemeinsames Layout für alle Aufnahmen bestehen.
- Im Dartmodus wird eine gemeinsame Folge aus drei Layouts erzeugt; jeder
  Spieler sieht diese Layouts bei Dart 1, 2 und 3 in derselben Reihenfolge.

### Standardfarben

```text
Gold: +50
Cyan: +25
Green: +10
Red: -25
Gray: 0 / blockiert
```

### Overlay

```json
{
  "prompt": "Gold zählt am meisten!",
  "bonus": [{"id": "T20", "color": "gold", "label": "+50"}],
  "targets": [{"id": "S5", "color": "green", "label": "+10"}],
  "danger": [{"id": "D1", "color": "red", "label": "-25"}],
  "disabled": [{"id": "T19", "color": "gray", "label": "0"}]
}
```

### Visual Incentives

- Board sieht wie Arcade-/Dancefloor aus.
- Goldfelder schimmern.
- Rote Felder flackern.
- Nach jedem Dart „reshuffle“-Animation.

### Varianten

```text
Stable: Farben bleiben einen Turn
Shuffle: Farben wechseln nach jedem Dart
Memory: Farben werden kurz gezeigt und dann versteckt
Team Color: Treffer auf eigene Farbe gibt Bonus
```

---

## 4.4 Risk It

### Kurzbeschreibung

Sammle Punkte in einem temporären Pot. Nach jedem Dart: sichern oder weiter riskieren.

### Regeln

- Treffer addiert zum Turn Pot.
- Spieler kann nach Dart 1 oder 2 bank drücken.
- Miss oder Gefahrenfeld verliert den Turn Pot.
- Nach 3 Darts wird automatisch gebankt, falls kein Bust.

### Scoring

```text
Hit: Pot += score
Bull: Pot *= 2 optional
Miss: Pot = 0, Turn endet
Bank: Score += Pot, Turn endet
```

### Control Requirement

Benötigt in `/control` und optional Boardbutton:

```text
BANK
CONTINUE
```

Board-Menübutton könnte im Hold/Risk-Kontext als `BANK` interpretiert werden.

### Visual Incentives

- Pot-Zahl groß und wachsend.
- Risk-Meter wird aggressiver.
- Bei Bank: Münzregen / Safe-Animation.
- Bei Bust: Pot zerplatzt.

---

## 4.5 Lightning Round

### Kurzbeschreibung

Schnelle Aufgaben, ein Dart pro Aufgabe.

### Regeln

- Spiel zeigt Aufgabe.
- Spieler hat einen Dart.
- Erfolg gibt Punkt(e), Fehler 0.
- Danach nächster Spieler oder nächste Aufgabe.

### Beispiel-Aufgaben

```text
Triff eine gerade Zahl
Triff etwas über 15
Triff ein Double
Triff ein Triple
Triff Bull
Triff nicht die 20
Triff ein rotes Segment
Triff ein Feld unter 10
```

### Visual Incentives

- Große Task-Karte.
- Countdown 5–10 Sekunden.
- Erfolg: grüner Stamp `SUCCESS`.
- Fehler: roter Stamp `FAIL`.

---

## 4.6 King of the Board

### Kurzbeschreibung

Spieler erobern Felder. Das Board färbt sich in Spielerfarben.

### Regeln

- Treffer auf Feld übernimmt dieses Feld.
- Singles übernehmen eine Zone oder das ganze Zahlenfeld, je nach Einstellung.
- Double/Triple können stärkere Eroberungen sein.
- Nach fester Rundenzahl gewinnt der Spieler mit größter Kontrolle oder den meisten Gebietspunkten.

### Scoring / Ownership

Option A:

```text
Jedes Segment einzeln besitzt Owner.
S20, D20, T20 getrennt.
```

Option B, einfacher:

```text
Jede Zahl 1–20 besitzt Owner.
Treffer auf beliebigen Ring übernimmt die Zahl.
Triple schützt für 2 gegnerische Treffer.
```

### Visual Incentives

- Board wird zu farbiger Landkarte.
- Capture-Animation: Farbe flutet Segment.
- Steal: Segment blitzt in alter und neuer Farbe.
- Endscreen zeigt Board als Territorienkarte.

---

## 4.7 Zombie Darts

### Kurzbeschreibung

Infizierte Felder breiten sich aus. Spieler reinigen sie durch Treffer.

### Regeln

- Einige Felder starten infiziert.
- Treffer auf infiziertes Feld gibt Punkte und reinigt es.
- Nach jedem Turn breitet sich Infektion aus.
- Modus kann kooperativ oder kompetitiv sein.

### Visual Incentives

- Infektion als grüner/lila Schleim.
- Reinigung mit hellem Sweep.
- Ausbreitung animiert von Feld zu Feld.
- Wenn Board fast voll ist: Alarm.

### Varianten

```text
Coop Survival: alle gegen Infektion
Versus Cleanup: wer reinigt am meisten
Boss Infection: zentrale Boss-HP sinkt pro Reinigung
```

---

## 4.8 Boss Fight

### Kurzbeschreibung

Alle Spieler kämpfen gegen einen Boss mit Lebenspunkten.

### Regeln

- Boss hat HP.
- Treffer verursachen Schaden.
- Schwachpunkte leuchten und geben Bonusdamage.
- Nach jeder Runde triggert der Boss eine Regeländerung.

### Boss Actions

```text
Blockiert ein Segment
Verwandelt ein Feld in Bombe
Halbiert Schaden für eine Runde
Fordert Bull als Shield Break
Greift führenden Spieler an
```

### Visual Incentives

- Boss-Grafik/Avatar neben Board.
- HP-Bar.
- Damage-Flyouts.
- Phase Change bei 75/50/25% HP.
- Final Hit Animation.

---

## 4.9 Simon Says Darts

### Kurzbeschreibung

Merke dir eine Zielsequenz und triff sie in Reihenfolge.

### Regeln

- Sequenz wird angezeigt oder vorgespielt.
- Spieler muss Ziele in Reihenfolge treffen.
- Jeder Erfolg verlängert Sequenz.
- Fehler beendet Runde oder setzt Sequenz zurück.

### Visual Incentives

- Segmente leuchten nacheinander.
- Memory-Modus: Ziele verschwinden.
- Fortschrittsleiste pro Sequenz.
- Fehler markiert falsches Segment rot.

---

## 4.10 Darts Bingo

### Kurzbeschreibung

Jeder Spieler hat eine Bingo-Karte mit Dartaufgaben.

### Regeln

- Karte 3x3 oder 4x4.
- Felder enthalten Ziele oder Bedingungen.
- Treffer erfüllt passende Karte.
- Bingo-Linie gewinnt oder gibt Bonus.

### Beispiel-Felder

```text
T20
D8
Bull
Any Triple
Even Number
Score > 40
Miss
Red Segment
```

### Visual Incentives

- Bingo-Karte auf Projector/Control.
- erfülltes Feld flippt um.
- Bingo-Linie leuchtet.

---

## 5. Priorisierte Implementierungsreihenfolge

### Phase 1: Gemeinsames Overlay-System

Zuerst technische Grundlage:

```text
GamePlugin.get_overlay(state)
GamePlugin.get_prompt(state)
Projector rendert targets/danger/bonus/disabled/owned
```

Ohne diese Grundlage müsste jeder Modus eigene UI-Logik bauen.

### Phase 2: Erste drei Party-Modi

1. `target_rush`
2. `avoid_bomb`
3. `color_clash`

Diese drei nutzen dieselbe Overlay-Mechanik und liefern schnell sichtbaren Mehrwert.

### Phase 3: Risiko und Timer

4. `risk_it`
5. `lightning_round`

Benötigen zusätzliche UI-Elemente:

- Timer
- Bank Button
- Task Cards

### Phase 4: Komplexe visuelle Modi

6. `king_of_the_board`
7. `zombie_darts`
8. `boss_fight`

Benötigen komplexeren persistenten Board-State.

---

## 6. Projector Requirements für Party-Modi

### Muss

- ohne Scrollen auf 16:9 funktionieren
- Segment-Overlays rendern
- Ziel/Gefahr/Bonus gleichzeitig darstellen
- letzter Treffer deutlich sichtbar
- Hold-Zustand deutlich sichtbar
- Turn-Zusammenfassung anzeigen
- keine kleinen Texte als primäre Information

### Sollte

- Fullscreen/Kiosk optimiert
- Animationen per CSS/SVG statt Video
- hohe Kontraste für Beamer
- Farben konfigurierbar
- reduzierte Darstellung für echte Projektion direkt auf Board

### Darf später

- Partikeleffekte
- Soundeffekte
- Spieler-Avatare
- Theme-Skins
- QR-Code zur Control UI

---

## 7. Control UI Requirements für Party-Modi

### Muss

- Touch-first
- kein Scrollen im laufenden Spiel, wenn möglich
- große Buttons für Weiter, Undo, Korrektur
- Spielmodus-Auswahl mit kurzen Erklärungen
- aktuelle Regel/Aufgabe anzeigen
- manuelle Korrektur des letzten Wurfs

### Sollte

- Party-Modi als Karten mit Bild/Icon
- Difficulty wählen
- Dauer/Runden wählen
- Teams unterstützen
- Bank-/Powerup-Aktionen kontextuell anzeigen

---

## 8. Game Plugin API Vorschlag

Langfristig sollten Party-Modi als Plugins abgebildet werden.

```python
class GamePlugin:
    id: str
    name: str
    category: str  # classic, training, party, coop

    def initial_state(self, players, settings): ...
    def handle_throw(self, state, event): ...
    def handle_action(self, state, action): ...
    def get_overlay(self, state): ...
    def get_prompt(self, state): ...
    def is_finished(self, state): ...
```

Actions aus der Control UI:

```text
continue
undo
bank
use_powerup
skip_task
start_timer
pause
```

---

## 9. Acceptance Criteria für erste Party-Version

Eine erste Party-Version gilt als erfolgreich, wenn:

1. Target Rush spielbar ist.
2. Projector zeigt Zielsegment vor dem Wurf.
3. Treffer auf Ziel erzeugt sichtbare Belohnung.
4. Miss erzeugt sichtbares negatives Feedback.
5. Nach 3 Darts geht Spiel in Hold.
6. Weiter funktioniert über Board und Control UI.
7. Control UI kann Modus starten und Schwierigkeit wählen.
8. Overlay-Datenmodell ist generisch genug für Avoid the Bomb und Color Clash.

---

## 10. Erste konkrete Spezifikation: `target_rush`

### Settings

```json
{
  "difficulty": "normal",
  "rounds": 5,
  "exact_points": 50,
  "almost_points": 10,
  "combo_bonus": 10,
  "allow_bull": true,
  "target_pool": "normal"
}
```

### State

```json
{
  "active_target": {"id": "T20", "field": 20, "ring": "triple", "label": "T20"},
  "combo_by_player": {"player-id": 2},
  "round": 1,
  "message": "Triff T20!"
}
```

### Throw Handling

```text
if miss:
  score += 0
  combo = 0
  message = "Miss"

if exact target:
  score += exact_points + combo * combo_bonus
  combo += 1
  generate new target

if same field wrong ring:
  score += almost_points
  combo = 0 or unchanged depending setting

else:
  score += 0
  combo = 0
```

### Overlay

```json
{
  "prompt": "Triff T20!",
  "targets": [{"id": "T20", "color": "cyan", "label": "+50", "pulse": true}],
  "combo": {"count": 2, "bonus": 20}
}
```

---

## 11. Erste konkrete Spezifikation: `avoid_bomb`

### Settings

```json
{
  "difficulty": "normal",
  "bomb_count": 4,
  "bomb_growth": "escalating",
  "penalty": -50,
  "turn_ends_on_bomb": false,
  "hidden_bombs": false
}
```

### State

```json
{
  "bombs": ["D1", "T5", "S20", "DBULL"],
  "bomb_round": 1,
  "message": "Meide Rot!"
}
```

### Throw Handling

```text
if hit bomb:
  score += penalty
  show explosion
  optionally end turn/hold
else if hit:
  score += dart score
else miss:
  score += 0

after every player has completed the round:
  keep all existing bombs
  if bomb_growth == "steady":
    add exactly one new bomb
  else:
    add as many bombs as the new round number
```

### Overlay

```json
{
  "prompt": "Runde 2: 5 Bomben – meide Rot!",
  "danger": [
    {"id": "D1", "color": "red", "label": "BOMB", "pulse": true},
    {"id": "T5", "color": "red", "label": "BOMB", "pulse": true}
  ]
}
```

---

## 12. Erste konkrete Spezifikation: `color_clash`

### Settings

```json
{
  "difficulty": "normal",
  "shuffle": "dart",
  "gold_count": 3,
  "cyan_count": 6,
  "green_count": 8,
  "red_count": 4
}
```

### Color Scores

```json
{
  "gold": 50,
  "cyan": 25,
  "green": 10,
  "red": -25,
  "gray": 0
}
```

### Throw Handling

```text
if hit colored zone:
  score += color value
if miss:
  score += 0
At round start:
  pre-generate one shared layout
  or three shared layouts for dart 1/2/3
For every player:
  replay the same round layout or three-layout sequence
After every player completed the round:
  generate the next shared layout set
```

### Overlay

```json
{
  "prompt": "Gold zählt am meisten!",
  "bonus": [{"id": "T20", "color": "gold", "label": "+50"}],
  "targets": [{"id": "S5", "color": "green", "label": "+10"}],
  "danger": [{"id": "D1", "color": "red", "label": "-25"}]
}
```

---

## 13. Implementierungsstand

Stand: 2026-07-28

Die erste Party-Version ist implementiert:

```text
target_rush
avoid_bomb
color_clash
risk_it
king_of_board
treasure_hunt
```

Zusätzlich wurde ein generisches Overlay-Feld im Game-State eingeführt:

```json
{
  "overlay": {
    "prompt": "Triff T20!",
    "targets": [],
    "danger": [],
    "bonus": [],
    "combo": {}
  }
}
```

Der Projector rendert diese Overlay-Zonen direkt auf der SVG-Dartboardscheibe:

- `targets` → cyan/grün
- `danger` → rot/flackernd
- `bonus` → gold/schimmernd

### Implementierte Dateien

```text
sdb_dartboard/games/arcade.py
sdb_dartboard/games/target_rush.py
sdb_dartboard/games/avoid_bomb.py
sdb_dartboard/games/color_clash.py
sdb_dartboard/games/risk_it.py
sdb_dartboard/games/king_of_board.py
sdb_dartboard/games/treasure_hunt.py
```

### Tests

Die Party-Modes sind in `tests/test_games.py` abgedeckt:

- Discovery der neuen Modes
- Target-Rush-Overlay
- Avoid-Bomb-Danger-Overlay
- Color-Clash-Farbwertung
- Risk-It-Bank-Action
- King-of-the-Board-Owned-Overlay
- Treasure-Hunt-Reveal-Mechanik

### Nächste sinnvolle Ergänzungen

1. Eigene Mode-Artworks für `target_rush`, `avoid_bomb`, `color_clash`.
2. Control-UI-Detailpanel für Party-Regeln während des Spiels.
3. Sound-Mapping pro Party-Event:
   - target exact
   - almost
   - bomb
   - color bonus
4. Timer-/Speed-Mechanik für Target Rush und Lightning Round.
5. Persistente Highscores pro Party-Modus.
