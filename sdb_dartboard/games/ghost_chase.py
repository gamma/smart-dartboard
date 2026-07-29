from __future__ import annotations

from typing import Any, Dict

from .arcade import choose_targets, finish_round_game, overlay_item, same_target
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
            InstructionStep("Gleicher Pfad", "Alle jagen pro Runde dieselbe Folge von Geisterzielen.", "shuffle"),
        ],
        sound_theme="arcade",
    )

    def initialize_player(self, player: Any, options: Dict[str, Any]) -> None:
        player.score = 0
        player.marks = {}

    def initialize_state(self, state: Any, options: Dict[str, Any]) -> None:
        state.mode_state = {
            "combo": {player.id: 0 for player in state.players},
            "escape": {player.id: 0 for player in state.players},
            "path_index": {player.id: 0 for player in state.players},
        }
        self._generate_round_path(state)

    def _generate_round_path(self, state: Any) -> None:
        state.mode_state["path"] = choose_targets(
            4,
            str(state.options.get("difficulty", "normal")),
        )
        state.mode_state["path_round"] = state.round_number

    def _target(self, state: Any, player: Any) -> Dict[str, Any]:
        path = state.mode_state.get("path", [])
        index = int(state.mode_state.get("path_index", {}).get(player.id, 0))
        return path[min(index, len(path) - 1)]

    def on_turn_start(self, state: Any, player: Any) -> None:
        if int(state.mode_state.get("path_round", 0)) != state.round_number:
            self._generate_round_path(state)
        state.mode_state["combo"][player.id] = 0
        state.mode_state["escape"][player.id] = 0
        state.mode_state["path_index"][player.id] = 0

    def apply_throw(self, state: Any, player: Any, event: Dict[str, Any]) -> ThrowOutcome:
        target = self._target(state, player)
        combo = int(state.mode_state["combo"].get(player.id, 0))
        if event.get("type") == "hit" and same_target(event, target):
            points = 40 + min(combo, 2) * 10
            player.score += points
            state.mode_state["combo"][player.id] = combo + 1
            state.mode_state["escape"][player.id] = 0
            state.mode_state["path_index"][player.id] += 1
            outcome = ThrowOutcome(points, f"GHOST CAUGHT! +{points}")
        else:
            state.mode_state["combo"][player.id] = 0
            escape = int(state.mode_state["escape"].get(player.id, 0)) + 1
            state.mode_state["escape"][player.id] = escape
            message = "Der Geist bleibt"
            if escape >= 3:
                state.mode_state["path_index"][player.id] += 1
                state.mode_state["escape"][player.id] = 0
                message = "WHOOSH! Der Geist ist geflohen"
            outcome = ThrowOutcome(0, message)
        return finish_round_game(state, outcome, "{winner} ist der beste Geisterjäger!")

    def get_overlay(self, state: Any) -> Dict[str, Any]:
        current = state.current_player()
        target = self._target(state, current) if current else None
        combo = int(state.mode_state.get("combo", {}).get(current.id if current else "", 0))
        escape = int(state.mode_state.get("escape", {}).get(current.id if current else "", 0))
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
