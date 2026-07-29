from __future__ import annotations

import random
from typing import Any, Dict, List

from .arcade import (
    TARGET_POOL_BASIC,
    TARGET_POOL_HARD,
    TARGET_POOL_NORMAL,
    overlay_item,
    same_field,
    same_target,
    zone_id,
)
from .base import GameMetadata, GameOption, InstructionStep, ThrowOutcome


class MiniGolfMode:
    metadata = GameMetadata(
        slug="mini_golf",
        title="Mini Golf Darts",
        tagline="Neun Löcher auf der Scheibe",
        description="Alle spielen dasselbe Loch. Je früher du das Ziel triffst, desto weniger Schläge sammelst du.",
        accent="#74a57f",
        accent_secondary="#f2cc8f",
        visual="mini-golf",
        icon="flag",
        options=[
            GameOption("holes", "Löcher", "choice", 9, [
                {"value": 6, "label": "6 Löcher"},
                {"value": 9, "label": "9 Löcher"},
            ]),
            GameOption("difficulty", "Platz", "choice", "normal", [
                {"value": "easy", "label": "Easy · Zahl genügt"},
                {"value": "normal", "label": "Normal · Single/Double exakt"},
                {"value": "hard", "label": "Hard · Double/Triple/Bull"},
            ]),
        ],
        instructions=[
            InstructionStep("Gleiches Loch", "Jeder Spieler wirft auf dasselbe Ziel.", "flag"),
            InstructionStep("Wenige Schläge", "Treffer mit Dart 1, 2 oder 3 zählt entsprechend viele Schläge.", "golf"),
            InstructionStep("Niedrig gewinnt", "Kein Treffer zählt vier Schläge. Nach dem letzten Loch gewinnt der niedrigste Score.", "trophy"),
        ],
        sound_theme="arcade",
    )

    def initialize_player(self, player: Any, options: Dict[str, Any]) -> None:
        player.score = 0
        player.marks = {}

    def initialize_state(self, state: Any, options: Dict[str, Any]) -> None:
        state.options["rounds"] = int(options.get("holes", 9))
        state.mode_state = {"hole": 1, "used": []}
        self._new_hole(state)

    def _pool(self, state: Any) -> List[Dict[str, Any]]:
        difficulty = state.options.get("difficulty", "normal")
        if difficulty == "easy":
            return TARGET_POOL_BASIC
        if difficulty == "hard":
            return TARGET_POOL_HARD
        return [
            dart for dart in TARGET_POOL_NORMAL
            if dart["ring"] in {"single_outer", "double"}
        ]

    def _new_hole(self, state: Any) -> None:
        used = set(state.mode_state.get("used", []))
        available = [dart for dart in self._pool(state) if zone_id(dart) not in used]
        if not available:
            state.mode_state["used"] = []
            available = self._pool(state)
        target = random.choice(available)  # nosec B311
        state.mode_state["target"] = target
        state.mode_state.setdefault("used", []).append(zone_id(target))
        state.mode_state["hole"] = state.round_number
        state.message = f"Loch {state.round_number}: {target['label']}"

    def on_turn_start(self, state: Any, player: Any) -> None:
        if int(state.mode_state.get("hole", 0)) != state.round_number:
            self._new_hole(state)

    def _finish(self, state: Any, outcome: ThrowOutcome) -> ThrowOutcome:
        is_last = state.current_player_index == len(state.players) - 1
        end_turn = outcome.force_hold or state.darts_in_turn == 2
        if not (is_last and end_turn and state.round_number >= int(state.options["rounds"])):
            return outcome
        low = min(player.score for player in state.players)
        leaders = [player for player in state.players if player.score == low]
        outcome.finished = True
        outcome.force_hold = False
        if len(leaders) == 1:
            outcome.winner_id = leaders[0].id
            outcome.winner_ids = [leaders[0].id]
            outcome.result_type = "individual_win"
            outcome.message = f"{leaders[0].name} gewinnt den Platz mit {low} Schlägen!"
        else:
            outcome.result_type = "draw"
            outcome.message = "Unentschieden: " + " · ".join(player.name for player in leaders)
        return outcome

    def apply_throw(self, state: Any, player: Any, event: Dict[str, Any]) -> ThrowOutcome:
        target = state.mode_state["target"]
        matcher = same_field if state.options.get("difficulty") == "easy" else same_target
        if event.get("type") == "hit" and matcher(event, target):
            strokes = state.darts_in_turn + 1
            player.score += strokes
            label = {1: "BIRDIE", 2: "PAR", 3: "BOGEY"}[strokes]
            outcome = ThrowOutcome(strokes, f"{label}! {strokes} Schlag", force_hold=True)
        elif state.darts_in_turn == 2:
            player.score += 4
            outcome = ThrowOutcome(4, "DOUBLE BOGEY · 4 Schläge")
        else:
            outcome = ThrowOutcome(0, "Am Loch vorbei")
        return self._finish(state, outcome)

    def get_overlay(self, state: Any) -> Dict[str, Any]:
        target = state.mode_state.get("target")
        return {
            "prompt": f"Loch {state.round_number}: {target['label']}" if target else "Nächstes Loch",
            "targets": [overlay_item(target, "green", "⚑", True)] if target else [],
            "panel": {
                "title": f"LOCH {state.round_number}/{state.options.get('holes', 9)}",
                "headline": target["label"] if target else "–",
                "subline": "Birdie 1 · Par 2 · Bogey 3 · vorbei 4",
                "rows": [
                    {"label": player.name, "value": f"{player.score} Schläge"}
                    for player in sorted(state.players, key=lambda item: item.score)
                ],
            },
        }


GAME_MODE = MiniGolfMode()
