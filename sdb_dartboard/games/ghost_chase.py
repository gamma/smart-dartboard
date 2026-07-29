from __future__ import annotations

from typing import Any, Dict

from .arcade import choose_targets, finish_round_game, overlay_item, same_target, zone_id
from .base import GameMetadata, GameOption, InstructionStep, ThrowOutcome


class GhostChaseMode:
    metadata = GameMetadata(
        slug="ghost_chase",
        title="Ghost Chase",
        tagline="Fang den hüpfenden Geist",
        description="Triff den Geist für eine wachsende Dreier-Combo. Nach drei Fehlversuchen flieht er weiter.",
        accent="#72c9b9",
        accent_secondary="#f7d488",
        visual="ghost-chase",
        icon="ghost",
        options=[
            GameOption("rounds", "Runden", "choice", 5, [
                {"value": 5, "label": "5 Runden"},
                {"value": 8, "label": "8 Runden"},
            ]),
            GameOption("difficulty", "Geisterpfad", "choice", "normal", [
                {"value": "easy", "label": "Easy · Singles"},
                {"value": "normal", "label": "Normal · Alle Ringe"},
                {"value": "hard", "label": "Hard · Double/Triple/Bull"},
            ]),
        ],
        instructions=[
            InstructionStep("Geist treffen", "Triff das exakt markierte Segment.", "ghost"),
            InstructionStep("Combo jagen", "Treffer in einer Aufnahme zählen 40, 50 und 60.", "combo"),
            InstructionStep("Geist flieht", "Nach drei Fehlversuchen springt er auf ein neues Feld.", "dash"),
        ],
        sound_theme="arcade",
    )

    def initialize_player(self, player: Any, options: Dict[str, Any]) -> None:
        player.score = 0
        player.marks = {}

    def initialize_state(self, state: Any, options: Dict[str, Any]) -> None:
        state.mode_state = {
            "target": choose_targets(1, str(options.get("difficulty", "normal")))[0],
            "combo": {player.id: 0 for player in state.players},
            "escape": 0,
        }

    def _move(self, state: Any) -> None:
        old = state.mode_state["target"]
        state.mode_state["target"] = choose_targets(
            1,
            str(state.options.get("difficulty", "normal")),
            exclude=[zone_id(old)],
        )[0]

    def on_turn_start(self, state: Any, player: Any) -> None:
        state.mode_state["combo"][player.id] = 0

    def apply_throw(self, state: Any, player: Any, event: Dict[str, Any]) -> ThrowOutcome:
        target = state.mode_state["target"]
        combo = int(state.mode_state["combo"].get(player.id, 0))
        if event.get("type") == "hit" and same_target(event, target):
            points = 40 + min(combo, 2) * 10
            player.score += points
            state.mode_state["combo"][player.id] = combo + 1
            state.mode_state["escape"] = 0
            self._move(state)
            outcome = ThrowOutcome(points, f"GHOST CAUGHT! +{points}")
        else:
            state.mode_state["combo"][player.id] = 0
            escape = int(state.mode_state.get("escape", 0)) + 1
            state.mode_state["escape"] = escape
            message = "Der Geist bleibt"
            if escape >= 3:
                self._move(state)
                state.mode_state["escape"] = 0
                message = "WHOOSH! Der Geist ist geflohen"
            outcome = ThrowOutcome(0, message)
        return finish_round_game(state, outcome, "{winner} ist der beste Geisterjäger!")

    def get_overlay(self, state: Any) -> Dict[str, Any]:
        target = state.mode_state.get("target")
        current = state.current_player()
        combo = int(state.mode_state.get("combo", {}).get(current.id if current else "", 0))
        escape = int(state.mode_state.get("escape", 0))
        return {
            "prompt": f"Fang {target['label']}!" if target else "Fang den Geist!",
            "targets": [overlay_item(target, "cyan", "👻", True)] if target else [],
            "combo": {"count": combo, "bonus": combo * 10},
            "panel": {
                "title": "GHOST CHAIN",
                "headline": f"Combo ×{combo}",
                "subline": f"Fluchtladung {escape}/3",
                "progress": {"value": escape, "max": 3},
            },
        }


GAME_MODE = GhostChaseMode()
