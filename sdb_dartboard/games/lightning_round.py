from __future__ import annotations

import random
from typing import Any, Dict, List

from .arcade import DARTS, finish_round_game, number_overlay_items, overlay_item
from .base import GameMetadata, GameOption, InstructionStep, ThrowOutcome

TASKS = [
    {"id":"any_double","prompt":"Triff ein Double", "accept":lambda e: e.get("multiplier")==2, "zones":[d for d in DARTS if d["multiplier"]==2]},
    {"id":"any_triple","prompt":"Triff ein Triple", "accept":lambda e: e.get("multiplier")==3, "zones":[d for d in DARTS if d["multiplier"]==3]},
    {"id":"over_15","prompt":"Triff eine Zahl über 15", "accept":lambda e: int(e.get("field",0) or 0)>15 and int(e.get("field",0) or 0)<=20, "zones":[d for d in DARTS if 15 < d["field"] <= 20]},
    {"id":"under_10","prompt":"Triff eine Zahl unter 10", "accept":lambda e: 1 <= int(e.get("field",0) or 0)<10, "zones":[d for d in DARTS if 1 <= d["field"] < 10]},
    {"id":"bull","prompt":"Triff Bull", "accept":lambda e: int(e.get("field",0) or 0)==25, "zones":[d for d in DARTS if d["field"]==25]},
    {"id":"even","prompt":"Triff eine gerade Zahl", "accept":lambda e: int(e.get("field",0) or 0) in range(2,21,2), "zones":[d for d in DARTS if d["field"] in range(2,21,2)]},
]


def task_overlay(task: Dict[str, Any]) -> List[Dict[str, Any]]:
    if task["id"] in {"over_15", "under_10", "even"}:
        fields = sorted({int(dart["field"]) for dart in task["zones"]})
        return [
            item
            for field in fields
            for item in number_overlay_items(field, "cyan", "OK", False)
        ]
    return [overlay_item(dart, "cyan", "OK", False) for dart in task["zones"]]


class LightningRoundMode:
    metadata = GameMetadata(
        slug="lightning_round",
        title="Lightning Round",
        tagline="Eine Aufgabe, ein Dart",
        description="Schnelle Mini-Challenges: Löse die angezeigte Aufgabe mit deinem nächsten Dart.",
        accent="#28e7ff",
        accent_secondary="#ffcf33",
        visual="lightning-round",
        icon="bolt",
        options=[GameOption("rounds", "Runden", "choice", 8, [{"value":5,"label":"5 Runden"},{"value":8,"label":"8 Runden"},{"value":12,"label":"12 Runden"}])],
        instructions=[
            InstructionStep("Aufgabe lesen", "Der Projector zeigt die Challenge.", "task"),
            InstructionStep("Ein Dart", "Jeder Spieler hat genau einen Dart pro Aufgabe.", "dart"),
            InstructionStep("Erfolg punktet", "Erfolg gibt +25, Fehler gibt 0.", "success"),
        ],
        sound_theme="arcade",
        ruleset_version=2,
    )

    def initialize_player(self, player: Any, options: Dict[str, Any]) -> None:
        player.score = 0; player.marks = {}

    def initialize_state(self, state: Any, options: Dict[str, Any]) -> None:
        # Gameplay variety only; not used for a security decision.
        task = random.choice(TASKS)  # nosec B311
        state.mode_state = {"task_id": task["id"]}
        state.message = task["prompt"]

    def _task(self, state: Any) -> Dict[str, Any]:
        task_id = state.mode_state.get("task_id")
        return next((t for t in TASKS if t["id"] == task_id), TASKS[0])

    def _next_task(self, state: Any) -> None:
        current = state.mode_state.get("task_id")
        pool = [t for t in TASKS if t["id"] != current]
        task = random.choice(pool)  # nosec B311
        state.mode_state["task_id"] = task["id"]
        state.message = task["prompt"]

    def apply_throw(self, state: Any, player: Any, event: Dict[str, Any]) -> ThrowOutcome:
        task = self._task(state)
        success = event.get("type") == "hit" and bool(task["accept"](event))
        points = 25 if success else 0
        player.score += points
        msg = "SUCCESS +25" if success else "FAIL"
        # Every player receives the same challenge in a round.
        if state.current_player_index == len(state.players) - 1:
            self._next_task(state)
        # one dart only: force hold after each throw
        outcome = ThrowOutcome(turn_value=points, message=msg, force_hold=True)
        return finish_round_game(
            state,
            outcome,
            "{winner} gewinnt Lightning!",
            darts_per_turn=1,
        )

    def on_turn_skipped(self, state: Any, player: Any) -> None:
        del player
        is_last_player = (
            state.current_player_index == len(state.players) - 1
        )
        final_round = state.round_number >= int(
            state.options.get("rounds", 8)
        )
        if is_last_player and not final_round:
            self._next_task(state)

    def get_overlay(self, state: Any) -> Dict[str, Any]:
        task = self._task(state)
        return {"prompt": task["prompt"], "targets": task_overlay(task)}


GAME_MODE = LightningRoundMode()
