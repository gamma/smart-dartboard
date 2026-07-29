from __future__ import annotations

from typing import Any, Dict

from .arcade import finish_round_game
from .base import GameMetadata, GameOption, InstructionStep, ThrowOutcome


class CandyCannonMode:
    metadata = GameMetadata(
        slug="candy_cannon",
        title="Candy Cannon",
        tagline="Laden, riskieren, feuern",
        description="Treffer laden deine Süßigkeitenkanone. Triff bei 8–10 Ladung ins Bull, bevor sie überhitzt.",
        accent="#e76f51",
        accent_secondary="#f4d35e",
        visual="candy-cannon",
        icon="cannon",
        min_players=2,
        options=[
            GameOption("rounds", "Runden", "choice", 5, [
                {"value": 5, "label": "5 Runden"},
                {"value": 8, "label": "8 Runden"},
            ]),
        ],
        instructions=[
            InstructionStep("Auf 8, 9 oder 10 laden", "Single zählt 1, Double 2, Triple 3 und Bull 4 Ladung.", "candy"),
            InstructionStep("BEREIT? Bull treffen", "Sobald BEREIT erscheint, feuert Single oder Double Bull: +50 für dich, −25 für den Führenden.", "cannon"),
            InstructionStep("11 ist zu viel", "Über 10 überhitzt die Kanone und deine Ladung fällt auf null.", "danger"),
        ],
        sound_theme="arcade",
    )

    def initialize_player(self, player: Any, options: Dict[str, Any]) -> None:
        player.score = 0
        player.marks = {}

    def initialize_state(self, state: Any, options: Dict[str, Any]) -> None:
        state.mode_state = {"charge": {player.id: 0 for player in state.players}}

    def _target(self, state: Any, player: Any) -> Any:
        opponents = [candidate for candidate in state.players if candidate.id != player.id]
        high = max(candidate.score for candidate in opponents)
        leaders = {candidate.id for candidate in opponents if candidate.score == high}
        start = state.current_player_index
        ordered = state.players[start + 1:] + state.players[:start]
        return next(candidate for candidate in ordered if candidate.id in leaders)

    def _fire(self, state: Any, player: Any, event: Dict[str, Any]) -> ThrowOutcome:
        target = self._target(state, player)
        previous_score = target.score
        player.score += 50
        target.score = max(0, target.score - 25)
        state.mode_state["charge"][player.id] = 0
        event["effect"] = "candy_fire"
        event["target_player_id"] = target.id
        event["target_score_loss"] = previous_score - target.score
        return ThrowOutcome(
            50,
            f"FIRE! {player.name} +50 · {target.name} -25",
        )

    def apply_throw(self, state: Any, player: Any, event: Dict[str, Any]) -> ThrowOutcome:
        if event.get("type") != "hit":
            outcome = ThrowOutcome(0, "MISS · Keine Ladung")
        else:
            charge = int(state.mode_state["charge"].get(player.id, 0))
            is_bull = int(event.get("field", 0)) == 25
            if is_bull and 8 <= charge <= 10:
                outcome = self._fire(state, player, event)
            else:
                addition = 4 if is_bull else int(event.get("multiplier", 1))
                charge += addition
                if charge > 10:
                    state.mode_state["charge"][player.id] = 0
                    event["effect"] = "candy_overheat"
                    outcome = ThrowOutcome(0, "OVERHEAT! Ladung verloren")
                else:
                    state.mode_state["charge"][player.id] = charge
                    outcome = ThrowOutcome(0, f"Kanone geladen: {charge}/10")
        return finish_round_game(state, outcome, "{winner} gewinnt die Candy Cannon!")

    def get_overlay(self, state: Any) -> Dict[str, Any]:
        player = state.current_player()
        charge = int(state.mode_state.get("charge", {}).get(player.id if player else "", 0))
        target = self._target(state, player) if player and len(state.players) > 1 else None
        ready = 8 <= charge <= 10
        targets = []
        if ready:
            targets = [
                {
                    "field": 25,
                    "ring": "single_bull",
                    "color": "#f4d35e",
                    "label": "FIRE",
                    "pulse": True,
                },
                {
                    "field": 25,
                    "ring": "double_bull",
                    "color": "#e76f51",
                    "label": "",
                    "pulse": True,
                },
            ]
        return {
            "prompt": "BEREIT · JETZT BULL TREFFEN!" if ready else "LADUNG AUF 8–10 STELLEN · DANN MIT BULL FEUERN",
            "targets": targets,
            "panel": {
                "title": "CANDY CANNON",
                "headline": f"BEREIT · {charge}/10" if ready else f"Ladung {charge}/10",
                "subline": (
                    f"BULL feuert auf {target.name}"
                    if ready and target
                    else "Über 10 überhitzt die Kanone"
                ),
                "progress": {"value": charge, "max": 10},
            },
        }


GAME_MODE = CandyCannonMode()
