# X01 Dynamic Checkout & Setup Advisor

Stand: 2026-07-28

## Ziel

Im X01-Modus soll die App dynamisch anzeigen, welcher Wurf gerade sinnvoll ist.

Dabei gibt es zwei Fälle:

1. **Finish möglich**: Der aktuelle Score kann mit den verbleibenden Darts auf exakt 0 gebracht werden.
2. **Finish nicht möglich**: Die App empfiehlt einen Setup-Wurf, der einen guten Restscore stellt.

Die Anzeige soll sowohl auf dem Control-Screen als auch auf dem Projector erscheinen. Auf dem Projector soll das empfohlene Segment direkt auf der Scheibe hervorgehoben werden.

---

## Grundregeln

### Double Out

Bei `out_rule = double` muss der letzte Dart ein Double sein.

Ungültig:

```text
Rest 1
Rest 0 ohne Double
Überwerfen < 0
```

### Straight Out

Bei `out_rule = straight` kann jeder Treffer das Spiel beenden, solange exakt 0 erreicht wird.

---

## Advice-Modell

Der Game-State enthält für X01 ein Feld:

```json
{
  "advice": {
    "type": "x01_advice",
    "score": 171,
    "darts_left": 3,
    "out_rule": "double",
    "status": "setup",
    "message": "Stellen: T20 lässt 111 – nächster Turn T20 · T15 · D3",
    "primary": {"label": "T20", "field": 20, "ring": "triple", "multiplier": 3, "score": 60},
    "sequence": [],
    "setup": {
      "target": {"label": "T20", "field": 20, "ring": "triple", "multiplier": 3, "score": 60},
      "leave": 111,
      "remaining_checkout": [],
      "next_turn_checkout": [
        {"label": "T20"},
        {"label": "T15"},
        {"label": "D3"}
      ]
    }
  }
}
```

---

## Statuswerte

| Status | Bedeutung |
|---|---|
| `checkout` | Finish ist mit den verbleibenden Darts möglich |
| `setup` | Finish jetzt nicht möglich, empfohlener Wurf stellt sinnvoll |
| `score_down` | kein klarer Setup, bester sicherer Scoring-Wurf |
| `none` | keine Empfehlung möglich oder nicht sinnvoll |

---

## Beispiele

### 40 Rest, Double Out, 3 Darts

```text
D20
```

Status:

```text
checkout
```

### 170 Rest, Double Out, 3 Darts

```text
T20 → T20 → DBull
```

Status:

```text
checkout
```

### 171 Rest, Double Out, 3 Darts

171 kann nicht mit 3 Darts gefinished werden.

Empfehlung:

```text
T20 lässt 111
```

Nächster Turn kann dann z. B.:

```text
T20 → T15 → D3
```

Status:

```text
setup
```

### Rest 1, Double Out

Kein sinnvoller Finish möglich.

Status:

```text
none
```

---

## UI-Verhalten

### Control UI

Während X01 im Playing-Screen:

- Advice Panel unter der Spielerüberschrift
- zeigt:
  - Finish / Stellen / Nächster Wurf
  - primäres Ziel, z. B. `T20`
  - Sequenz, falls Finish möglich
  - Setup-Message, falls Finish nicht möglich

### Projector UI

Während X01 im Playing-Screen:

- großes Advice-Panel auf dem Projector
- primäres Ziel sehr groß, z. B. `T20`
- Sequenz oder Setup-Hinweis kleiner darunter
- das empfohlene Segment wird auf dem Board gold/amber hervorgehoben

### Hold-Zustand

Im Hold-Zustand wird keine neue Empfehlung als Ziel angezeigt. Hold dient dem Entfernen der Darts und bewusstem Weiterdrücken.

---

## Technische Umsetzung

Advisor-Modul:

```text
sdb_dartboard/games/x01_advisor.py
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

## Verbesserungspotenzial

Der aktuelle Advisor ist pragmatisch und spielbar, aber nicht perfekt professionell.

Mögliche spätere Verbesserungen:

- echte Profi-Checkout-Tabelle statt generischem Suchalgorithmus
- bevorzugte Wege, z. B. D16-Familie stärker gewichten
- unterschiedliche Skill-Level:
  - Anfänger: eher Singles stellen
  - Fortgeschritten: Standard-Checkouts
  - Profi: optimale Wege
- Vermeidung unpraktischer Doubles
- Spielerpräferenzen, z. B. Lieblingsdouble
- Anzeige alternativer Wege
- Voice/Sound: „T20 stellen“
