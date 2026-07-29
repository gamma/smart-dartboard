from __future__ import annotations

import random
from typing import Any, Dict

from .arcade import (
    TARGET_POOL_NORMAL,
    finish_round_game,
    overlay_item,
    same_target,
    zone_id,
)
from .base import GameMetadata, GameOption, InstructionStep, ThrowOutcome

REWARDS = {
    "gold": {"points": 75, "color": "gold", "label": "+75"},
    "silver": {"points": 35, "color": "cyan", "label": "+35"},
    "coin": {"points": 10, "color": "green", "label": "+10"},
    "trap": {"points": -40, "color": "red", "label": "TRAP"},
}


class TreasureHuntMode:
    metadata = GameMetadata(
        slug="treasure_hunt",
        title="Treasure Hunt",
        tagline="Finde Schätze, meide Fallen",
        description="Das Board ist eine Schatzkarte. Treffer decken versteckte Münzen, Gold und Fallen auf.",
        accent="#ffcf33",
        accent_secondary="#3dff91",
        visual="treasure-hunt",
        icon="gem",
        options=[
            GameOption("rounds", "Runden", "choice", 5, [{"value":3,"label":"3 Runden"},{"value":5,"label":"5 Runden"},{"value":8,"label":"8 Runden"}]),
            GameOption("traps", "Fallen", "choice", 5, [{"value":3,"label":"3 Fallen"},{"value":5,"label":"5 Fallen"},{"value":8,"label":"8 Fallen"}]),
        ],
        instructions=[
            InstructionStep("Schätze versteckt", "Treffer decken geheime Inhalte auf.", "gem"),
            InstructionStep("Gold lohnt sich", "Gold und Silber bringen große Punkte.", "coins"),
            InstructionStep("Fallen meiden", "Rote Fallen kosten Punkte.", "trap"),
        ],
        sound_theme="arcade",
    )

    def initialize_player(self, player: Any, options: Dict[str, Any]) -> None:
        player.score = 0
        player.marks = {}

    def initialize_state(self, state: Any, options: Dict[str, Any]) -> None:
        pool = random.sample(TARGET_POOL_NORMAL, min(36, len(TARGET_POOL_NORMAL)))
        trap_count = int(options.get("traps", 5))
        reward_types = ["gold"] * 4 + ["silver"] * 8 + ["coin"] * 16 + ["trap"] * trap_count
        random.shuffle(reward_types)
        hidden = {}
        for dart, reward in zip(pool, reward_types):
            hidden[zone_id(dart)] = {"dart": dart, "reward": reward, "revealed_by": None}
        state.mode_state = {"hidden": hidden, "revealed": {}}
        state.message = "Finde die Schätze!"

    def apply_throw(self, state: Any, player: Any, event: Dict[str, Any]) -> ThrowOutcome:
        if event.get("type") == "miss":
            outcome = ThrowOutcome(turn_value=0, message="Miss – kein Fund")
        else:
            hidden = state.mode_state.get("hidden", {})
            found_key = None
            found = None
            for key, item in hidden.items():
                if same_target(event, item["dart"]):
                    found_key, found = key, item
                    break
            if not found:
                outcome = ThrowOutcome(turn_value=0, message=f"{event.get('label', '')}: leer")
            elif found.get("revealed_by"):
                outcome = ThrowOutcome(turn_value=0, message=f"{event.get('label', '')}: bereits gefunden")
            else:
                reward = REWARDS[found["reward"]]
                points = int(reward["points"])
                player.score += points
                found["revealed_by"] = player.id
                state.mode_state.setdefault("revealed", {})[found_key] = found
                outcome = ThrowOutcome(
                    turn_value=points,
                    message=f"{event.get('label', '')}: {reward['label']}",
                )
        return finish_round_game(
            state, outcome, "{winner} findet den größten Schatz!"
        )

    def get_overlay(self, state: Any) -> Dict[str, Any]:
        revealed = state.mode_state.get("revealed", {})
        bonus = []
        targets = []
        danger = []
        for item in revealed.values():
            reward = REWARDS[item["reward"]]
            entry = overlay_item(item["dart"], reward["color"], reward["label"], False)
            if item["reward"] == "trap":
                danger.append(entry)
            elif item["reward"] == "gold":
                bonus.append(entry)
            else:
                targets.append(entry)
        return {
            "prompt": "Treffer decken Schätze auf!",
            "bonus": bonus,
            "targets": targets,
            "danger": danger,
        }


GAME_MODE = TreasureHuntMode()
