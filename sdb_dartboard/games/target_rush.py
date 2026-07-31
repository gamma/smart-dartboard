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

NUMBER_RINGS = ["single_inner", "triple", "single_outer", "double"]


class TargetRushMode:
    metadata = GameMetadata(
        slug="target_rush",
        title="Target Rush",
        tagline="Triff das leuchtende Ziel",
        description="Das Board zeigt ein Ziel. Easy nimmt die ganze Zahl, Normal und Hard verlangen das genaue Segment.",
        accent="#28e7ff",
        accent_secondary="#3dff91",
        visual="target-rush",
        icon="zap",
        options=[
            GameOption("rounds", "Runden", "choice", 5, [{"value":3,"label":"3 Runden"},{"value":5,"label":"5 Runden"},{"value":8,"label":"8 Runden"}]),
            GameOption("difficulty", "Ziele", "choice", "normal", [
                {"value":"easy","label":"Easy · ganze Zahl"},
                {"value":"normal","label":"Normal · exaktes Segment"},
                {"value":"hard","label":"Hard · Double/Triple"},
            ]),
        ],
        instructions=[
            InstructionStep("Easy: ganze Zahl", "Alle vier Ringe der Zielzahl zählen voll. Das Ziel bleibt für die ganze Runde stehen.", "target"),
            InstructionStep("Normal und Hard", "Triff das exakte Segment. Die richtige Zahl im falschen Ring gibt Almost-Punkte.", "spark"),
            InstructionStep("Combo sammeln", "Exakte Treffer in Folge bringen Bonus.", "combo"),
            InstructionStep("Gleiche Chancen", "Easy gibt allen dasselbe Rundenziel. Normal und Hard geben dieselbe Folge aus drei Zielen.", "shuffle"),
        ],
        sound_theme="arcade",
        ruleset_version=2,
    )

    def initialize_player(self, player: Any, options: Dict[str, Any]) -> None:
        player.score = 0
        player.marks = {}

    def initialize_state(self, state: Any, options: Dict[str, Any]) -> None:
        state.mode_state = {"combo": {}, "last_result": ""}
        self._generate_round_targets(state)

    def _generate_round_targets(self, state: Any) -> None:
        difficulty = str(state.options.get("difficulty", "normal"))
        targets = choose_targets(
            1 if difficulty == "easy" else 3,
            difficulty,
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
        state.message = (
            f"Triff die {target['field']}!"
            if state.options.get("difficulty") == "easy"
            else f"Triff {target['label']}!"
        )
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
        elif (
            state.options.get("difficulty") == "easy"
            and same_field(event, target)
        ) or same_target(event, target):
            points = 50 + combo * 10
            player.score += points
            state.mode_state["combo"][player.id] = combo + 1
            old = (
                str(target["field"])
                if state.options.get("difficulty") == "easy"
                else target["label"]
            )
            outcome = ThrowOutcome(turn_value=points, message=f"Perfect {old}! +{points}")
        elif same_field(event, target):
            player.score += 10
            state.mode_state["combo"][player.id] = 0
            outcome = ThrowOutcome(turn_value=10, message=f"Almost {event.get('label')} +10")
        else:
            state.mode_state["combo"][player.id] = 0
            outcome = ThrowOutcome(turn_value=0, message=f"Falsches Feld: {event.get('label', '')}")
        if (
            state.options.get("difficulty") != "easy"
            and state.darts_in_turn + 1 < 3
        ):
            self._select_target(state, state.darts_in_turn + 1)
        return finish_round_game(
            state, outcome, "{winner} gewinnt den Target Rush!"
        )

    def get_overlay(self, state: Any) -> Dict[str, Any]:
        target = state.mode_state.get("target")
        combo = state.mode_state.get("combo", {}).get(state.current_player().id if state.current_player() else "", 0)
        easy = state.options.get("difficulty") == "easy"
        targets = []
        if target:
            if easy:
                targets = [
                    {
                        **overlay_item(target, "cyan", "+50" if ring == "single_outer" else "", True),
                        "id": f"target-rush-{ring}-{target['field']}",
                        "ring": ring,
                    }
                    for ring in NUMBER_RINGS
                ]
            else:
                targets = [overlay_item(target, "cyan", "+50", True)]
        return {
            "prompt": (
                f"Triff die {target['field']}!"
                if target and easy
                else f"Triff {target['label']}!"
                if target
                else "Target Rush"
            ),
            "targets": targets,
            "combo": {"count": combo, "bonus": combo * 10},
        }


GAME_MODE = TargetRushMode()
