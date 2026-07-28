# X01 Dynamic Checkout & Setup Advisor

Stand: 2026-07-28

## Ziel

Im X01-Modus zeigt die App dynamisch an, welcher Wurf bzw. welche Wurfsequenz gerade sinnvoll ist.

Es gibt zwei zentrale Fälle:

1. **Checkout**: Der aktuelle Score kann mit den verbleibenden Darts gefinished werden.
2. **Setup**: Der aktuelle Score kann nicht gefinished werden; die App zeigt eine sinnvolle Sequenz für alle verbleibenden Darts, um einen guten Rest zu stellen.

Die Empfehlung erscheint:

- im Control UI als Advisor-Panel
- im Projector UI groß sichtbar
- auf der Projektorscheibe als hervorgehobenes primäres Zielsegment

---

## Checkout-Tabelle

Für Double-Out nutzt der Advisor eine kuratierte Standard-Checkout-Tabelle statt rein algorithmischer Suche.

Beispiele:

```text
170 -> T20 · T20 · DBull
167 -> T20 · T19 · DBull
164 -> T20 · T18 · DBull
161 -> T20 · T17 · DBull
160 -> T20 · T20 · D20
100 -> T20 · D20
82  -> DBull · D16
40  -> D20
32  -> D16
2   -> D1
```

Warum Tabelle?

- algorithmisch gibt es viele gültige Wege
- viele gültige Wege sind praktisch schlecht
- etablierte Checkout-Wege sind Spielern vertraut
- Projector/Control sollen klare Ansagen machen, keine mathematischen Kuriositäten

Für Straight-Out oder Scores außerhalb der Tabelle bleibt ein algorithmischer Fallback vorhanden.

---

## Setup über alle verbleibenden Darts

Wenn kein Finish möglich ist, soll nicht nur „nächster sinnvoller Dart“ angezeigt werden, sondern ein Plan für **alle verbleibenden Darts der Aufnahme**.

Beispiel bei 171 Rest und 3 Darts:

```text
T20 · T20 · S11 lässt 40
nächster Turn: D20
```

Beispiel bei 169 Rest und 3 Darts:

```text
T20 · T20 · S9 lässt 40
nächster Turn: D20
```

Beispiel bei 165 Rest und 3 Darts:

```text
T20 · T20 · S5 lässt 40
nächster Turn: D20
```

Der Advisor versucht, mit allen verbleibenden Darts einen guten Rest zu stellen, bevorzugt:

1. klassische Doppel-Reste wie 40, 32, 36, 24, 16
2. Finish-fähige Scores für den nächsten Turn
3. hohe Triple als Scoring-Basis
4. sinnvolle Singles zum Stellen
5. Doubles beim Stellen möglichst nicht, außer es gibt keine gute Alternative

---

## Advice-Modell

Der Game-State enthält für X01 ein Feld `advice`.

### Checkout-Beispiel

```json
{
  "type": "x01_advice",
  "score": 170,
  "darts_left": 3,
  "out_rule": "double",
  "status": "checkout",
  "message": "Finish möglich",
  "primary": {"label": "T20", "field": 20, "ring": "triple", "multiplier": 3, "score": 60},
  "sequence": [
    {"label": "T20"},
    {"label": "T20"},
    {"label": "DBull"}
  ],
  "setup": null
}
```

### Setup-Beispiel

```json
{
  "type": "x01_advice",
  "score": 171,
  "darts_left": 3,
  "out_rule": "double",
  "status": "setup",
  "message": "Stellen: T20 · T20 · S11 lässt 40 – nächster Turn D20",
  "primary": {"label": "T20", "field": 20, "ring": "triple", "multiplier": 3, "score": 60},
  "sequence": [
    {"label": "T20"},
    {"label": "T20"},
    {"label": "S11"}
  ],
  "setup": {
    "leave": 40,
    "next_turn_checkout": [{"label": "D20"}]
  }
}
```

---

## Statuswerte

| Status | Bedeutung |
|---|---|
| `checkout` | Finish ist mit den verbleibenden Darts möglich |
| `setup` | Finish jetzt nicht möglich; Sequenz stellt einen guten Rest |
| `score_down` | kein klarer Setup, bester sicherer Scoring-Wurf |
| `none` | keine Empfehlung möglich oder nicht sinnvoll |

---

## UI-Verhalten

### Control UI

Während X01 im Playing-Screen:

- Advice Panel unter der Spielerüberschrift
- zeigt:
  - `Finish möglich`, `Clever stellen` oder `Runterspielen`
  - primäres Ziel, z. B. `T20`
  - komplette Sequenz für die verbleibenden Darts
  - Setup-Message inklusive Restscore und nächstem Turn

### Projector UI

Während X01 im Playing-Screen:

- großes Advice-Panel auf dem Projector
- primäres Ziel sehr groß, z. B. `T20`
- Sequenz oder Setup-Hinweis kleiner darunter
- das primäre Segment wird auf der Scheibe gold/amber hervorgehoben

### Hold-Zustand

Im Hold-Zustand wird kein neues Ziel als Advisor-Target gepulst. Hold dient dem Darts-Ziehen und bewusstem Weiterdrücken.

---

## Technische Umsetzung

Advisor-Modul:

```text
sdb_dartboard/games/x01_advisor.py
```

Wichtige Funktionen:

```text
checkout_sequence(score, darts_left, out_rule)
setup_plan(score, darts_left, out_rule)
x01_advice(score, darts_left, out_rule)
```

Integration:

```text
GameState.as_dict() -> advice
```

Frontend:

```text
web/static/app.js
  x01AdvicePanel(game)
  projectorAdvice(game)
  renderBoardEvent() markiert advice.primary als advice-target
```

CSS:

```text
.x01-advice
.projector-advice
.seg.advice-target
```

Tests:

```text
tests/test_x01_advisor.py
```

---

## Tests / erwartetes Verhalten

```text
40, 3 Darts, Double Out  -> checkout D20
170, 3 Darts, Double Out -> checkout T20 · T20 · DBull
171, 3 Darts, Double Out -> setup T20 · T20 · S11 lässt 40
1, Double Out            -> none
```

---

## Verbesserungspotenzial

- mehrere alternative Checkout-Wege anzeigen
- Skill-Level:
  - Anfänger: einfacher stellen, weniger Triple-lastig
  - Fortgeschritten: Standard-Tabelle
  - Profi: aggressivere Wege
- Spielerpräferenzen, z. B. Lieblingsdouble
- „sicher stellen“ vs. „maximal aggressiv“ als Option
- Voice/Sound-Ausgabe: „T20 stellen“
