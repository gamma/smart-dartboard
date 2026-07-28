from __future__ import annotations

from typing import Any, Dict

from .base import GameMetadata, InstructionStep, ThrowOutcome


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
        return ThrowOutcome(turn_value=score, message=f"{player.name}: {event.get('label', '')}")


GAME_MODE = CountUpMode()
