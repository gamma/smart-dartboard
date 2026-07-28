# Modulare Spielmodi

Spielmodi liegen unter `sdb_dartboard/games`. Der Registry-Loader entdeckt
automatisch jedes Python-Modul, das ein Objekt namens `GAME_MODE` exportiert.
Für einen neuen Modus müssen weder `app.py` noch die zentrale `GameEngine`
angepasst werden.

## Vertrag

Ein Spielmodul implementiert:

```python
class ExampleMode:
    metadata = GameMetadata(...)

    def initialize_player(self, player, options):
        ...

    def apply_throw(self, state, player, event):
        return ThrowOutcome(...)


GAME_MODE = ExampleMode()
```

`metadata` steuert gleichzeitig Backend und Oberfläche:

- eindeutiger `slug`
- Titel, Kurztext und Beschreibung
- Akzentfarben und visuelles Thema
- Spielergrenzen
- Point-and-click-Optionen
- Anleitungsschritte für Control und Projektor
- Soundthema

## Neues Spiel hinzufügen

1. `sdb_dartboard/games/mein_spiel.py` anlegen.
2. `GameMetadata`, Optionen und Anleitungen definieren.
3. Spieler in `initialize_player` initialisieren.
4. Würfe in `apply_throw` verarbeiten.
5. Ein `ThrowOutcome` mit Turnwert, Meldung und optionalem Gewinner liefern.
6. Ein Cover unter `web/static/assets/modes/mein_spiel.webp` ablegen. Der
   Dateiname wird automatisch aus dem Slug abgeleitet.
7. Regeltests in `tests/test_games.py` ergänzen.

## Verfügbare Zustandsdaten

`apply_throw` erhält:

- `state.players`
- `state.current_player_index`
- `state.darts_in_turn`
- `state.turn_score`
- `state.round_number`
- `state.options`
- `state.turn_start_values`

Das Event enthält bei einem Treffer typischerweise:

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

## Regeln

- Das Plugin verändert nur spielmodusspezifische Scores und Marks.
- Aufnahmewechsel, Hold, Undo, Persistenz und BLE bleiben Aufgabe des Cores.
- `ThrowOutcome.turn_value` bestimmt den angezeigten Aufnahmewert.
- `finished=True` beendet das Spiel.
- `winner_id` wird gesetzt, wenn nicht der gerade werfende Spieler gewinnt.
- `force_hold=True` beendet die Aufnahme sofort, beispielsweise bei Bust.
- Jede neue Regel benötigt Tests für Normalfall, Randfall, Sieg und Undo.
