from __future__ import annotations

from typing import Any, Dict

from .arcade import choose_targets, overlay_item, same_field, same_target, zone_id
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
        ],
        sound_theme="arcade",
    )

    def initialize_player(self, player: Any, options: Dict[str, Any]) -> None:
        player.score = 0
        player.marks = {}

    def initialize_state(self, state: Any, options: Dict[str, Any]) -> None:
        target = choose_targets(1, str(options.get("difficulty", "normal")))[0]
        state.mode_state = {"target": target, "combo": {}, "last_result": ""}
        state.message = f"Triff {target['label']}!"

    def _next_target(self, state: Any) -> Dict[str, Any]:
        current = state.mode_state.get("target", {})
        target = choose_targets(1, str(state.options.get("difficulty", "normal")), exclude=[zone_id(current)])[0]
        state.mode_state["target"] = target
        state.message = f"Triff {target['label']}!"
        return target

    def apply_throw(self, state: Any, player: Any, event: Dict[str, Any]) -> ThrowOutcome:
        target = state.mode_state.get("target") or choose_targets(1)[0]
        combo = int(state.mode_state.setdefault("combo", {}).get(player.id, 0))
        if event.get("type") == "miss":
            state.mode_state["combo"][player.id] = 0
            return ThrowOutcome(turn_value=0, message="Miss – Combo reset")
        if same_target(event, target):
            points = 50 + combo * 10
            player.score += points
            state.mode_state["combo"][player.id] = combo + 1
            old = target["label"]
            self._next_target(state)
            return ThrowOutcome(turn_value=points, message=f"Perfect {old}! +{points}")
        if same_field(event, target):
            player.score += 10
            state.mode_state["combo"][player.id] = 0
            return ThrowOutcome(turn_value=10, message=f"Almost {event.get('label')} +10")
        state.mode_state["combo"][player.id] = 0
        return ThrowOutcome(turn_value=0, message=f"Falsches Feld: {event.get('label', '')}")

    def get_overlay(self, state: Any) -> Dict[str, Any]:
        target = state.mode_state.get("target")
        combo = state.mode_state.get("combo", {}).get(state.current_player().id if state.current_player() else "", 0)
        return {
            "prompt": f"Triff {target['label']}!" if target else "Target Rush",
            "targets": [overlay_item(target, "cyan", "+50", True)] if target else [],
            "combo": {"count": combo, "bonus": combo * 10},
        }


GAME_MODE = TargetRushMode()
