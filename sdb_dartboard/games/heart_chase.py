from __future__ import annotations

from typing import Any, Dict

from .base import GameMetadata, GameOption, InstructionStep, ThrowOutcome


class HeartChaseMode:
    metadata = GameMetadata(
        slug="heart_chase",
        title="Heart Chase",
        tagline="Schlag die Jagdpunktzahl",
        description="Übertriff die letzte Aufnahme. Wer scheitert, verliert ein Herz und setzt trotzdem die neue Jagd.",
        accent="#ef476f",
        accent_secondary="#ffd166",
        visual="heart-chase",
        icon="heart",
        min_players=2,
        options=[
            GameOption("hearts", "Herzen", "choice", 3, [
                {"value": 2, "label": "2 Herzen"},
                {"value": 3, "label": "3 Herzen"},
                {"value": 5, "label": "5 Herzen"},
            ]),
        ],
        instructions=[
            InstructionStep("Jagd eröffnen", "Der erste Spieler legt mit drei Darts die Jagdpunktzahl vor.", "target"),
            InstructionStep("Strikt übertreffen", "Gleichstand reicht nicht. Bei Misserfolg verlierst du ein Herz.", "heart"),
            InstructionStep("Letztes Herz gewinnt", "Ausgeschiedene Spieler werden automatisch übersprungen.", "trophy"),
        ],
        sound_theme="arcade",
    )

    def initialize_player(self, player: Any, options: Dict[str, Any]) -> None:
        player.score = 0
        player.marks = {}

    def initialize_state(self, state: Any, options: Dict[str, Any]) -> None:
        hearts = int(options.get("hearts", 3))
        state.mode_state = {
            "challenge_score": 0,
            "hearts": {player.id: hearts for player in state.players},
            "opening_turn": True,
        }
        state.message = "Eröffnet die Jagd!"

    def is_player_active(self, state: Any, player: Any) -> bool:
        return int(state.mode_state.get("hearts", {}).get(player.id, 0)) > 0

    def apply_throw(self, state: Any, player: Any, event: Dict[str, Any]) -> ThrowOutcome:
        value = int(event.get("score", 0)) if event.get("type") == "hit" else 0
        player.score += value
        turn_total = state.turn_score + value
        if state.darts_in_turn < 2:
            return ThrowOutcome(
                turn_value=value,
                message=f"{turn_total} · Jagd {state.mode_state['challenge_score']}",
            )

        previous_challenge = int(state.mode_state.get("challenge_score", 0))
        opening = bool(state.mode_state.get("opening_turn", False))
        state.mode_state["challenge_score"] = turn_total
        state.mode_state["opening_turn"] = False
        if opening:
            return ThrowOutcome(
                turn_value=value,
                message=f"Jagd eröffnet: {turn_total}",
            )

        if turn_total > previous_challenge:
            return ThrowOutcome(
                turn_value=value,
                message=f"CHASE BEATEN! {previous_challenge} → {turn_total}",
            )

        hearts = state.mode_state["hearts"]
        hearts[player.id] = max(0, int(hearts.get(player.id, 0)) - 1)
        active = [
            candidate
            for candidate in state.players
            if int(hearts.get(candidate.id, 0)) > 0
        ]
        if len(active) == 1:
            winner = active[0]
            return ThrowOutcome(
                turn_value=value,
                message=f"{winner.name} gewinnt die Herzjagd!",
                finished=True,
                winner_id=winner.id,
                winner_ids=[winner.id],
                result_type="individual_win",
            )
        return ThrowOutcome(
            turn_value=value,
            message=f"HEART LOST · Neue Jagd: {turn_total}",
        )

    def get_overlay(self, state: Any) -> Dict[str, Any]:
        current = state.current_player()
        hearts = state.mode_state.get("hearts", {})
        maximum = int(state.options.get("hearts", 3))
        rows = []
        for player in state.players:
            remaining = int(hearts.get(player.id, 0))
            rows.append({
                "label": player.name,
                "value": "♥" * remaining + "♡" * (maximum - remaining),
                "state": "danger" if remaining == 0 else "",
            })
        challenge = int(state.mode_state.get("challenge_score", 0))
        return {
            "prompt": "Jagd eröffnen!" if state.mode_state.get("opening_turn") else f"Schlag {challenge}!",
            "panel": {
                "title": "HERZJAGD",
                "headline": f"Aktuelle Jagd: {challenge}",
                "subline": f"{current.name} muss strikt mehr werfen" if current and not state.mode_state.get("opening_turn") else "Drei Darts legen die erste Jagd fest",
                "rows": rows,
            },
        }


GAME_MODE = HeartChaseMode()
