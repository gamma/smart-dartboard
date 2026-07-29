from __future__ import annotations

from typing import Any, Dict

from .arcade import choose_targets, finish_round_game, overlay_item, same_target, zone_id
from .base import GameMetadata, GameOption, InstructionStep, ThrowOutcome


class DragonEggsMode:
    metadata = GameMetadata(
        slug="dragon_eggs",
        title="Dragon Eggs",
        tagline="Sammle Eier, meide Schuppen",
        description="Goldene Dracheneier geben Punkte. Rote Schuppen heizen den persönlichen Heat-Meter auf.",
        accent="#f4a261",
        accent_secondary="#6ab04c",
        visual="dragon-eggs",
        icon="egg",
        options=[
            GameOption("rounds", "Runden", "choice", 5, [
                {"value": 5, "label": "5 Runden"},
                {"value": 8, "label": "8 Runden"},
            ]),
            GameOption("eggs", "Dracheneier", "choice", 4, [
                {"value": 3, "label": "3 Eier"},
                {"value": 4, "label": "4 Eier"},
                {"value": 6, "label": "6 Eier"},
            ]),
        ],
        instructions=[
            InstructionStep("Eier sammeln", "Jedes sichtbare goldene Ei gibt 30 Punkte.", "egg"),
            InstructionStep("Schuppen meiden", "Rote Schuppen kosten 15 Punkte und erhöhen dein Heat.", "danger"),
            InstructionStep("Drache erwacht", "Bei Heat 3 verlierst du die Hälfte deiner positiven Turn-Punkte.", "dragon"),
        ],
        sound_theme="arcade",
    )

    def initialize_player(self, player: Any, options: Dict[str, Any]) -> None:
        player.score = 0
        player.marks = {}

    def initialize_state(self, state: Any, options: Dict[str, Any]) -> None:
        state.mode_state = {
            "heat": {player.id: 0 for player in state.players},
            "turn_positive": {player.id: 0 for player in state.players},
        }
        self._shuffle(state)

    def _shuffle(self, state: Any) -> None:
        eggs = choose_targets(int(state.options.get("eggs", 4)), "normal")
        scales = choose_targets(4, "normal", exclude=[zone_id(item) for item in eggs])
        state.mode_state["eggs"] = eggs
        state.mode_state["scales"] = scales
        state.message = "Sammelt die goldenen Eier!"

    def on_turn_start(self, state: Any, player: Any) -> None:
        state.mode_state["turn_positive"][player.id] = 0
        self._shuffle(state)

    def apply_throw(self, state: Any, player: Any, event: Dict[str, Any]) -> ThrowOutcome:
        eggs = state.mode_state.get("eggs", [])
        scales = state.mode_state.get("scales", [])
        if event.get("type") == "hit" and any(same_target(event, item) for item in eggs):
            points = 30
            player.score += points
            state.mode_state["turn_positive"][player.id] += points
            outcome = ThrowOutcome(points, "Ei eingesammelt! +30")
        elif event.get("type") == "hit" and any(same_target(event, item) for item in scales):
            heat = int(state.mode_state["heat"].get(player.id, 0)) + 1
            points = -15
            message = "Drachenschuppe! -15"
            if heat >= 3:
                penalty = int(state.mode_state["turn_positive"].get(player.id, 0)) // 2
                points -= penalty
                heat = 0
                message = f"DRAGON AWAKES! -{15 + penalty}"
            state.mode_state["heat"][player.id] = heat
            player.score += points
            outcome = ThrowOutcome(points, message)
        else:
            outcome = ThrowOutcome(0, "Kein Ei gefunden")
        return finish_round_game(state, outcome, "{winner} hütet den Drachenschatz!")

    def get_overlay(self, state: Any) -> Dict[str, Any]:
        current = state.current_player()
        heat = int(state.mode_state.get("heat", {}).get(current.id if current else "", 0))
        return {
            "prompt": "Gold sammeln · Rot meiden",
            "bonus": [overlay_item(item, "gold", "+30", False) for item in state.mode_state.get("eggs", [])],
            "danger": [overlay_item(item, "red", "-15", True) for item in state.mode_state.get("scales", [])],
            "panel": {
                "title": "DRACHEN-HITZE",
                "headline": "🔥" * heat + "○" * (3 - heat),
                "subline": "Bei drei Schuppen erwacht der Drache",
                "progress": {"value": heat, "max": 3},
            },
        }


GAME_MODE = DragonEggsMode()
