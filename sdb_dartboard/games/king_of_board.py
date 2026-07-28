from __future__ import annotations

from typing import Any, Dict

from .arcade import zone_id
from .base import GameMetadata, GameOption, InstructionStep, ThrowOutcome


class KingOfBoardMode:
    metadata = GameMetadata(
        slug="king_of_board",
        title="King of the Board",
        tagline="Erobere die Scheibe",
        description="Jeder Treffer übernimmt ein Feld in deiner Farbe. Nach den Runden gewinnt die größte Herrschaft.",
        accent="#9b5cff",
        accent_secondary="#28e7ff",
        visual="king-board",
        icon="flag",
        options=[
            GameOption("rounds", "Runden", "choice", 5, [{"value":3,"label":"3 Runden"},{"value":5,"label":"5 Runden"},{"value":8,"label":"8 Runden"}]),
            GameOption("ownership", "Besitz", "choice", "segment", [{"value":"segment","label":"Segment genau"},{"value":"field","label":"Ganzes Zahlenfeld"}]),
        ],
        instructions=[
            InstructionStep("Felder erobern", "Treffer übernehmen Felder in deiner Farbe.", "capture"),
            InstructionStep("Zurückstehlen", "Triff gegnerische Felder, um sie zu übernehmen.", "steal"),
            InstructionStep("Mehrheit gewinnt", "Nach den Runden gewinnt die größte Board-Kontrolle.", "crown"),
        ],
        sound_theme="arcade",
    )

    def initialize_player(self, player: Any, options: Dict[str, Any]) -> None:
        player.score = 0
        player.marks = {}

    def initialize_state(self, state: Any, options: Dict[str, Any]) -> None:
        state.mode_state = {"owned": {}}
        state.message = "Erobere die Scheibe!"

    def _ownership_id(self, state: Any, event: Dict[str, Any]) -> str:
        field = int(event.get("field", 0) or 0)
        if field == 25:
            return "BULL"
        if state.options.get("ownership", "segment") == "field":
            return f"F{field}"
        label = str(event.get("label", ""))
        return label if label else f"F{field}"

    def _recalculate_scores(self, state: Any) -> None:
        counts = {player.id: 0 for player in state.players}
        for item in state.mode_state.get("owned", {}).values():
            owner = item.get("owner_id")
            if owner in counts:
                counts[owner] += 1
        for player in state.players:
            player.score = counts.get(player.id, 0)

    def apply_throw(self, state: Any, player: Any, event: Dict[str, Any]) -> ThrowOutcome:
        if event.get("type") == "miss":
            return ThrowOutcome(turn_value=0, message="Miss – kein Gebiet")
        owner_id = self._ownership_id(state, event)
        state.mode_state.setdefault("owned", {})[owner_id] = {
            "owner_id": player.id,
            "color": player.color,
            "label": owner_id,
            "field": int(event.get("field", 0) or 0),
            "ring": event.get("ring"),
        }
        self._recalculate_scores(state)
        is_last_dart = state.darts_in_turn == 2
        is_last_player = state.current_player_index == len(state.players) - 1
        if is_last_dart and is_last_player and state.round_number >= int(state.options.get("rounds", 5)):
            winner = max(state.players, key=lambda candidate: candidate.score)
            return ThrowOutcome(turn_value=1, message=f"{winner.name} regiert die Scheibe!", finished=True, winner_id=winner.id)
        return ThrowOutcome(turn_value=1, message=f"{player.name} erobert {event.get('label', '')}")

    def get_overlay(self, state: Any) -> Dict[str, Any]:
        owned = []
        for item in state.mode_state.get("owned", {}).values():
            field = item.get("field")
            ring = item.get("ring")
            if not field or not ring:
                continue
            owned.append({
                "id": item.get("label"),
                "field": field,
                "ring": ring,
                "color": item.get("color", "#28e7ff"),
                "owner_id": item.get("owner_id"),
            })
        return {"prompt": "Erobere die Scheibe!", "owned": owned}


GAME_MODE = KingOfBoardMode()
