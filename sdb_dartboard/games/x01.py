from __future__ import annotations

from typing import Any, Dict

from .base import GameMetadata, GameOption, InstructionStep, ThrowOutcome


class X01Mode:
    metadata = GameMetadata(
        slug="x01",
        title="X01",
        tagline="Runter auf exakt Null",
        description="Der Turnierklassiker: 301, 501 oder 701 Punkte präzise herunterspielen.",
        accent="#ffb52b",
        accent_secondary="#ff3d5f",
        visual="championship",
        icon="crown",
        options=[
            GameOption(
                "start_score",
                "Startpunktzahl",
                "choice",
                501,
                [{"value": 301, "label": "301"}, {"value": 501, "label": "501"}, {"value": 701, "label": "701"}],
            ),
            GameOption(
                "out_rule",
                "Checkout",
                "choice",
                "straight",
                [
                    {"value": "straight", "label": "Straight Out", "description": "Jeder Treffer darf das Spiel exakt auf null beenden.", "description_en": "Any hit may finish the game exactly on zero."},
                    {"value": "double", "label": "Double Out", "description": "Der letzte Treffer muss ein Double oder Double Bull sein.", "description_en": "The final hit must be a Double or Double Bull."},
                ],
            ),
        ],
        instructions=[
            InstructionStep("Herunterspielen", "Jeder Treffer wird von deinem Restscore abgezogen.", "subtract"),
            InstructionStep("Exakt Null", "Du musst deinen Score genau auf null bringen. Bei Double Out muss der letzte Dart ein Double sein.", "zero"),
            InstructionStep("Bust", "Überwirfst du dich, wird die komplette Aufnahme zurückgesetzt.", "bust"),
        ],
        sound_theme="championship",
    )

    def initialize_player(self, player: Any, options: Dict[str, Any]) -> None:
        player.score = int(options.get("start_score", 501))
        player.marks = {}

    def apply_throw(self, state: Any, player: Any, event: Dict[str, Any]) -> ThrowOutcome:
        score = int(event.get("score", 0))
        new_score = player.score - score
        out_rule = state.options.get("out_rule", "straight")
        checkout_invalid = (
            out_rule == "double"
            and (new_score == 1 or (new_score == 0 and int(event.get("multiplier", 0)) != 2))
        )
        if new_score < 0 or checkout_invalid:
            player.score = int(state.turn_start_values.get(player.id, player.score))
            event["bust"] = True
            return ThrowOutcome(
                turn_value=0,
                message="Bust – Aufnahme wird zurückgesetzt",
                bust=True,
                force_hold=True,
            )
        player.score = new_score
        if new_score == 0:
            return ThrowOutcome(
                turn_value=score,
                message=f"{player.name} gewinnt!",
                finished=True,
                winner_id=player.id,
            )
        return ThrowOutcome(turn_value=score, message=f"{player.name}: {event.get('label', '')}")


GAME_MODE = X01Mode()
