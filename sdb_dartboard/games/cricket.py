from __future__ import annotations

from typing import Any, Dict

from .base import GameMetadata, InstructionStep, ThrowOutcome

CRICKET_TARGETS = [20, 19, 18, 17, 16, 15, 25]


class CricketMode:
    metadata = GameMetadata(
        slug="cricket",
        title="Cricket",
        tagline="Schließen und punkten",
        description="Schließe 15 bis 20 und Bull, während du offene Felder deiner Gegner punktest.",
        accent="#3dff91",
        accent_secondary="#11a56a",
        visual="clubhouse",
        icon="shield",
        instructions=[
            InstructionStep("Ziele treffen", "Nur 15, 16, 17, 18, 19, 20 und Bull zählen.", "targets"),
            InstructionStep("Dreimal schließen", "Single zählt eins, Double zwei und Triple drei Marks.", "marks"),
            InstructionStep("Offen punkten", "Weitere Treffer punkten, solange ein Gegner das Feld offen hat.", "lock"),
        ],
        sound_theme="club",
    )

    def initialize_player(self, player: Any, options: Dict[str, Any]) -> None:
        player.score = 0
        player.marks = {str(target): 0 for target in CRICKET_TARGETS}

    def apply_throw(self, state: Any, player: Any, event: Dict[str, Any]) -> ThrowOutcome:
        field = int(event.get("field", 0) or 0)
        multiplier = int(event.get("multiplier", 0) or 0)
        if field not in CRICKET_TARGETS or multiplier <= 0:
            return ThrowOutcome(turn_value=0, message=f"{player.name}: kein Cricket-Ziel")
        key = str(field)
        before = player.marks.get(key, 0)
        after = min(3, before + multiplier)
        overflow = max(0, before + multiplier - 3)
        player.marks[key] = after
        scored = 0
        if overflow and any(
            opponent.id != player.id and opponent.marks.get(key, 0) < 3
            for opponent in state.players
        ):
            scored = overflow * field
            player.score += scored
        closed_all = all(player.marks.get(str(target), 0) >= 3 for target in CRICKET_TARGETS)
        leading = player.score >= max(
            (opponent.score for opponent in state.players if opponent.id != player.id),
            default=0,
        )
        if closed_all and leading:
            return ThrowOutcome(turn_value=scored, message=f"{player.name} gewinnt!", finished=True)
        return ThrowOutcome(turn_value=scored, message=f"{player.name}: {event.get('label', '')}")

    def get_overlay(self, state: Any) -> Dict[str, Any]:
        player = state.current_player()
        if not player:
            return {"prompt": "Cricket"}
        remaining = []
        targets = []
        for field in CRICKET_TARGETS:
            marks = min(3, int(player.marks.get(str(field), 0)))
            needed = 3 - marks
            if needed <= 0:
                continue
            remaining.append(
                {
                    "field": field,
                    "label": "BULL" if field == 25 else str(field),
                    "marks": marks,
                    "needed": needed,
                }
            )
            rings = (
                ["single_bull", "double_bull"]
                if field == 25
                else ["single_inner", "triple", "single_outer", "double"]
            )
            targets.extend(
                {
                    "id": f"cricket-{field}-{ring}",
                    "field": field,
                    "ring": ring,
                    "color": "green",
                    "label": "",
                    "pulse": False,
                }
                for ring in rings
            )
        return {
            "prompt": "Offene Cricket-Ziele",
            "targets": targets,
            "cricket": {"remaining": remaining},
        }


GAME_MODE = CricketMode()
