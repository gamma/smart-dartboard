from __future__ import annotations

from typing import Any, Dict

from .base import GameMetadata, InstructionStep, ThrowOutcome


class EightBallMode:
    metadata = GameMetadata(
        slug="eight_ball",
        title="8-Ball Darts",
        tagline="Räume deine Kugeln ab",
        description="Ein klares Duell für zwei Spieler: erst die eigenen Kugeln versenken, dann Double Bull als schwarze 8.",
        accent="#3d8b74",
        accent_secondary="#e9c46a",
        visual="eight-ball",
        icon="eight",
        min_players=2,
        max_players=2,
        instructions=[
            InstructionStep("Eigene Kugeln", "Spieler 1 räumt 1–7, Spieler 2 räumt 9–15.", "balls"),
            InstructionStep("Foul beendet", "Falsche Kugel, neutrales Feld oder Miss beendet die Aufnahme sofort.", "foul"),
            InstructionStep("Schwarze 8", "Sind deine Kugeln weg, gewinnst du mit Double Bull. Zu früh gewinnt der Gegner.", "eight"),
        ],
        sound_theme="club",
    )

    def initialize_player(self, player: Any, options: Dict[str, Any]) -> None:
        player.score = 0
        player.marks = {}

    def initialize_state(self, state: Any, options: Dict[str, Any]) -> None:
        state.mode_state = {
            "balls": {
                state.players[0].id: list(range(1, 8)),
                state.players[1].id: list(range(9, 16)),
            },
        }
        state.message = "Räumt eure Kugeln ab!"

    def _winner(self, winner: Any, message: str) -> ThrowOutcome:
        return ThrowOutcome(
            0,
            message,
            finished=True,
            winner_id=winner.id,
            winner_ids=[winner.id],
            result_type="individual_win",
        )

    def apply_throw(self, state: Any, player: Any, event: Dict[str, Any]) -> ThrowOutcome:
        opponent = next(item for item in state.players if item.id != player.id)
        balls = state.mode_state["balls"][player.id]
        is_double_bull = (
            event.get("type") == "hit"
            and int(event.get("field", 0)) == 25
            and int(event.get("multiplier", 1)) == 2
        )
        if is_double_bull:
            if balls:
                return self._winner(opponent, f"8-Ball zu früh! {opponent.name} gewinnt")
            return self._winner(player, f"BLACK 8! {player.name} gewinnt")

        is_single = str(event.get("ring", "")).startswith("single")
        field = int(event.get("field", 0) or 0)
        if event.get("type") == "hit" and is_single and field in balls:
            balls.remove(field)
            player.score += 20
            return ThrowOutcome(20, f"Kugel {field} versenkt! +20")
        return ThrowOutcome(0, "FOUL · Spielerwechsel", force_hold=True)

    def get_overlay(self, state: Any) -> Dict[str, Any]:
        player = state.current_player()
        balls = state.mode_state.get("balls", {}).get(player.id if player else "", [])
        targets = []
        if balls:
            for field in balls:
                targets.extend([
                    {"id": f"SI{field}", "field": field, "ring": "single_inner", "color": "green", "label": str(field), "pulse": False},
                    {"id": f"SO{field}", "field": field, "ring": "single_outer", "color": "green", "label": str(field), "pulse": False},
                ])
            prompt = "Versenke: " + " · ".join(str(field) for field in balls)
        else:
            targets = [{"id": "DBULL", "field": 25, "ring": "double_bull", "color": "gold", "label": "8", "pulse": True}]
            prompt = "BLACK 8 · DOUBLE BULL!"
        return {
            "prompt": prompt,
            "targets": targets,
            "panel": {
                "title": "8-BALL",
                "headline": "Schwarze 8" if not balls else f"{len(balls)} Kugeln übrig",
                "rows": [
                    {
                        "label": candidate.name,
                        "value": "8-BALL" if not state.mode_state["balls"][candidate.id] else " · ".join(str(item) for item in state.mode_state["balls"][candidate.id]),
                    }
                    for candidate in state.players
                ],
            },
        }


GAME_MODE = EightBallMode()
