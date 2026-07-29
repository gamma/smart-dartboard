from __future__ import annotations

import random
from typing import Any, Dict

from .arcade import choose_targets, finish_round_game, overlay_item, same_target
from .base import GameMetadata, GameOption, InstructionStep, ThrowOutcome


class SimonSaysMode:
    metadata = GameMetadata(
        slug="simon_says",
        title="Simon Says",
        tagline="Merken, treffen, erweitern",
        description="Der Projector zeigt eine Sequenz. Triff die Ziele in der richtigen Reihenfolge.",
        accent="#3dff91",
        accent_secondary="#9b5cff",
        visual="simon-says",
        icon="memory",
        options=[
            GameOption("rounds", "Runden", "choice", 5, [{"value":3,"label":"3 Runden"},{"value":5,"label":"5 Runden"},{"value":8,"label":"8 Runden"}]),
            GameOption("difficulty", "Ziele", "choice", "easy", [{"value":"easy","label":"Easy"},{"value":"normal","label":"Normal"},{"value":"hard","label":"Hard"}]),
        ],
        instructions=[
            InstructionStep("Sequenz merken", "Die leuchtenden Felder sind deine Reihenfolge.", "memory"),
            InstructionStep("Richtig treffen", "Jeder Treffer muss zum nächsten Sequenzziel passen.", "target"),
            InstructionStep("Sequenz wächst", "Erfolg macht die nächste Aufgabe länger.", "grow"),
        ],
        sound_theme="arcade",
    )

    def initialize_player(self, player: Any, options: Dict[str, Any]) -> None:
        player.score = 0; player.marks = {}

    def initialize_state(self, state: Any, options: Dict[str, Any]) -> None:
        seq = choose_targets(1, str(options.get("difficulty", "easy")))
        state.mode_state = {"sequence": seq, "position": 0}
        state.message = "Merke die Sequenz!"

    def _extend(self, state: Any) -> None:
        seq = list(state.mode_state.get("sequence", []))
        if len(seq) < 3:
            seq.extend(choose_targets(1, str(state.options.get("difficulty", "easy")), exclude=[item["label"] for item in seq]))
        else:
            seq = choose_targets(1, str(state.options.get("difficulty", "easy")))
        state.mode_state["sequence"] = seq
        state.mode_state["position"] = 0

    def apply_throw(self, state: Any, player: Any, event: Dict[str, Any]) -> ThrowOutcome:
        seq = state.mode_state.get("sequence", [])
        pos = int(state.mode_state.get("position", 0))
        target = seq[pos] if pos < len(seq) else None
        if event.get("type") != "hit" or not target or not same_target(event, target):
            state.mode_state["position"] = 0
            return finish_round_game(
                state,
                ThrowOutcome(
                    turn_value=0,
                    message="Falsches Feld – Sequenz reset",
                    force_hold=True,
                ),
                "{winner} gewinnt Simon Says!",
                darts_per_turn=1,
            )
        state.mode_state["position"] = pos + 1
        if pos + 1 >= len(seq):
            points = 25 * len(seq)
            player.score += points
            self._extend(state)
            return finish_round_game(
                state,
                ThrowOutcome(
                    turn_value=points,
                    message=f"Sequenz geschafft +{points}",
                    force_hold=True,
                ),
                "{winner} gewinnt Simon Says!",
                darts_per_turn=1,
            )
        return ThrowOutcome(turn_value=0, message=f"Weiter: {seq[pos+1]['label']}")

    def get_overlay(self, state: Any) -> Dict[str, Any]:
        seq = state.mode_state.get("sequence", [])
        pos = int(state.mode_state.get("position", 0))
        return {
            "prompt": " → ".join(item["label"] for item in seq) or "Simon Says",
            "targets": [overlay_item(item, "cyan" if index == pos else "green", str(index + 1), index == pos) for index, item in enumerate(seq)],
        }


GAME_MODE = SimonSaysMode()
