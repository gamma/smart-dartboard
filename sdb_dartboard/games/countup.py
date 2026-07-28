from __future__ import annotations

from typing import Any, Dict

from .base import GameMetadata, GameOption, InstructionStep, ThrowOutcome


class CountUpMode:
    metadata = GameMetadata(
        slug="countup",
        title="Count Up",
        tagline="Jeder Punkt zählt",
        description="Sammelt über mehrere Aufnahmen so viele Punkte wie möglich.",
        accent="#28e7ff",
        accent_secondary="#176dff",
        visual="neon-orbit",
        icon="target",
        options=[
            GameOption(
                "rounds",
                "Runden",
                "choice",
                8,
                [
                    {"value": 5, "label": "5 Runden"},
                    {"value": 8, "label": "8 Runden"},
                    {"value": 10, "label": "10 Runden"},
                ],
            ),
        ],
        instructions=[
            InstructionStep("Punkte sammeln", "Jeder Treffer wird direkt zu deinem Konto addiert.", "score"),
            InstructionStep("Drei Darts", "Eine Aufnahme besteht aus drei Würfen.", "darts"),
            InstructionStep("Vorne gewinnt", "Nach der gewählten Rundenzahl gewinnt der höchste Score.", "trophy"),
        ],
    )

    def initialize_player(self, player: Any, options: Dict[str, Any]) -> None:
        player.score = 0
        player.marks = {}

    def apply_throw(self, state: Any, player: Any, event: Dict[str, Any]) -> ThrowOutcome:
        score = int(event.get("score", 0))
        player.score += score
        is_last_dart = state.darts_in_turn == 2
        is_last_player = state.current_player_index == len(state.players) - 1
        if is_last_dart and is_last_player and state.round_number >= int(state.options.get("rounds", 8)):
            winner = max(state.players, key=lambda candidate: candidate.score)
            return ThrowOutcome(
                turn_value=score,
                message=f"{winner.name} gewinnt!",
                finished=True,
                winner_id=winner.id,
            )
        return ThrowOutcome(turn_value=score, message=f"{player.name}: {event.get('label', '')}")


GAME_MODE = CountUpMode()
