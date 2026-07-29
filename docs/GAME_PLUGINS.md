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
- optionale grafische `control_legend` für Steuerflächen
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

Für feste Rundenzahlen sollte ein Arcade-Modus jeden Rückgabepfad aus
`apply_throw` über `finish_round_game(...)` führen. Dadurch beendet auch ein
Miss oder ein neutrales Feld die letzte Aufnahme korrekt. Controller-Aktionen,
die eine Aufnahme beenden, verwenden entsprechend
`finish_action_round_game(...)`.

Ein optionaler Hook `on_turn_start(state, player)` wird nach dem
Spielerwechsel ausgeführt. Kompetitive Zufallsbedingungen dürfen dort nur neu
erzeugt werden, wenn `state.round_number` gewechselt hat. Innerhalb einer Runde
müssen alle Spieler dasselbe Layout beziehungsweise dieselbe Aufgabenfolge
erhalten. Die verbindliche Regel und Ausnahmen stehen in
[`FAIRNESS.md`](FAIRNESS.md).

Eliminierungsmodi können zusätzlich
`is_player_active(state, player) -> bool` anbieten. Der Core überspringt
inaktive Spieler dann automatisch.

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
- Ein Einzelsieg setzt `winner_id`, `winner_ids=[winner_id]` und
  `result_type="individual_win"`.
- Ein Koop-Sieg setzt alle Teammitglieder in `winner_ids` und
  `result_type="team_win"`; `winner_id` bleibt leer.
- Niederlagen ohne Sieger verwenden `result_type="challenge_loss"`,
  Gleichstände `result_type="draw"`.
- `force_hold=True` beendet die Aufnahme sofort, beispielsweise bei Bust.
- `get_overlay(state)` darf neben Zielen ein deklaratives `panel` sowie
  mehrteilige `zones` liefern. Beide Ansichten rendern diese Daten ohne
  modusabhängige Core-Änderung.
- `GameMetadata.control_legend` rendert dieselbe vertikale Steuerungslegende in
  Anleitung und laufendem Spiel. Jeder Eintrag enthält `icon`, `color`,
  `label` sowie optional `secondary_color` und `detail`. Unterstützte
  Richtungssymbole sind `left`, `right`, `rotate_left`, `rotate_right` und
  `drop`; dargestellt wird immer Symbol → Farbe → Bezeichnung.
- Jede neue Regel benötigt Tests für Normalfall, Randfall, Sieg und Undo.
- Jeder kompetitive Zufallsmodus benötigt einen Mehrspieler-Test, der gleiche
  Bedingungen innerhalb der Runde und den Wechsel zur nächsten Runde prüft.
- Jeder Modus mit Cover folgt dem Basis-Prompt in
  `docs/ARTWORK_PROMPTS.md`; Bildtitel gehören in die UI, nicht in das Artwork.
