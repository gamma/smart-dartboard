from __future__ import annotations

from typing import Any, Dict

from .arcade import (
    choose_targets,
    finish_round_game,
    overlay_item,
    same_field,
    same_target,
)
from .base import GameMetadata, GameOption, InstructionStep, ThrowOutcome


class TargetRushMode:
    metadata = GameMetadata(
        slug="target_rush",
        title="Target Rush",
        tagline="Triff das leuchtende Ziel",
        description="Ein Arcade-Modus: Das Board zeigt ein Ziel. Exakt treffen gibt volle Punkte, gleiche Zahl gibt Almost-Punkte.",
        accent="#28e7ff",
        accent_secondary="#3dff91",
        visual="target-rush",
        icon="zap",
        options=[
            GameOption("rounds", "Runden", "choice", 5, [{"value":3,"label":"3 Runden"},{"value":5,"label":"5 Runden"},{"value":8,"label":"8 Runden"}]),
            GameOption("difficulty", "Ziele", "choice", "normal", [{"value":"easy","label":"Easy"},{"value":"normal","label":"Normal"},{"value":"hard","label":"Hard"}]),
        ],
        instructions=[
            InstructionStep("Ziel leuchtet", "Triff das cyan markierte Segment.", "target"),
            InstructionStep("Almost zählt", "Gleiche Zahl im falschen Ring gibt kleine Punkte.", "spark"),
            InstructionStep("Combo sammeln", "Exakte Treffer in Folge bringen Bonus.", "combo"),
            InstructionStep("Gleiche Chancen", "Alle spielen pro Runde dieselbe Folge aus drei Zielen.", "shuffle"),
        ],
        sound_theme="arcade",
    )

    def initialize_player(self, player: Any, options: Dict[str, Any]) -> None:
        player.score = 0
        player.marks = {}

    def initialize_state(self, state: Any, options: Dict[str, Any]) -> None:
        state.mode_state = {"combo": {}, "last_result": ""}
        self._generate_round_targets(state)

    def _generate_round_targets(self, state: Any) -> None:
        targets = choose_targets(
            3,
            str(state.options.get("difficulty", "normal")),
        )
        state.mode_state["target_round"] = state.round_number
        state.mode_state["round_targets"] = targets
        self._select_target(state, 0)

    def _select_target(self, state: Any, index: int) -> Dict[str, Any]:
        targets = state.mode_state["round_targets"]
        selected = max(0, min(index, len(targets) - 1))
        target = targets[selected]
        state.mode_state["target_index"] = selected
        state.mode_state["target"] = target
        state.message = f"Triff {target['label']}!"
        return target

    def on_turn_start(self, state: Any, player: Any) -> None:
        del player
        if int(state.mode_state.get("target_round", 0)) != state.round_number:
            self._generate_round_targets(state)
        else:
            self._select_target(state, 0)

    def apply_throw(self, state: Any, player: Any, event: Dict[str, Any]) -> ThrowOutcome:
        target = state.mode_state.get("target") or choose_targets(1)[0]
        combo = int(state.mode_state.setdefault("combo", {}).get(player.id, 0))
        if event.get("type") == "miss":
            state.mode_state["combo"][player.id] = 0
            outcome = ThrowOutcome(turn_value=0, message="Miss – Combo reset")
        elif same_target(event, target):
            points = 50 + combo * 10
            player.score += points
            state.mode_state["combo"][player.id] = combo + 1
            old = target["label"]
            outcome = ThrowOutcome(turn_value=points, message=f"Perfect {old}! +{points}")
        elif same_field(event, target):
            player.score += 10
            state.mode_state["combo"][player.id] = 0
            outcome = ThrowOutcome(turn_value=10, message=f"Almost {event.get('label')} +10")
        else:
            state.mode_state["combo"][player.id] = 0
            outcome = ThrowOutcome(turn_value=0, message=f"Falsches Feld: {event.get('label', '')}")
        if state.darts_in_turn + 1 < 3:
            self._select_target(state, state.darts_in_turn + 1)
        return finish_round_game(
            state, outcome, "{winner} gewinnt den Target Rush!"
        )

    def get_overlay(self, state: Any) -> Dict[str, Any]:
        target = state.mode_state.get("target")
        combo = state.mode_state.get("combo", {}).get(state.current_player().id if state.current_player() else "", 0)
        return {
            "prompt": f"Triff {target['label']}!" if target else "Target Rush",
            "targets": [overlay_item(target, "cyan", "+50", True)] if target else [],
            "combo": {"count": combo, "bonus": combo * 10},
        }


GAME_MODE = TargetRushMode()
