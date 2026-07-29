from __future__ import annotations

from typing import Any, Dict

from .arcade import finish_round_game
from .base import GameMetadata, GameOption, InstructionStep, ThrowOutcome


class CandyCannonMode:
    metadata = GameMetadata(
        slug="candy_cannon",
        title="Candy Cannon",
        tagline="Laden, riskieren, feuern",
        description="Treffer laden deine Süßigkeitenkanone. Feuere zwischen 8 und 10, bevor sie überhitzt.",
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
            InstructionStep("Kanone laden", "Single +1, Double +2, Triple +3 und Bull +4 Ladung.", "candy"),
            InstructionStep("Bei 8–10 feuern", "FIRE gibt dir 50 und nimmt dem führenden Gegner 25 Punkte.", "cannon"),
            InstructionStep("Nicht überladen", "Über 10 fällt deine Ladung ohne Belohnung auf null.", "danger"),
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

    def apply_throw(self, state: Any, player: Any, event: Dict[str, Any]) -> ThrowOutcome:
        if event.get("type") != "hit":
            outcome = ThrowOutcome(0, "MISS · Keine Ladung")
        else:
            addition = 4 if int(event.get("field", 0)) == 25 else int(event.get("multiplier", 1))
            charge = int(state.mode_state["charge"].get(player.id, 0)) + addition
            if charge > 10:
                state.mode_state["charge"][player.id] = 0
                outcome = ThrowOutcome(0, "OVERHEAT! Ladung verloren")
            else:
                state.mode_state["charge"][player.id] = charge
                outcome = ThrowOutcome(0, f"Kanone geladen: {charge}/10")
        return finish_round_game(state, outcome, "{winner} gewinnt die Candy Cannon!")

    def handle_action(self, state: Any, action: str, payload: Dict[str, Any]) -> None:
        if action != "fire":
            raise ValueError(f"Unknown Candy Cannon action: {action}")
        if state.status != "running":
            raise ValueError("FIRE is only available during an active turn")
        player = state.current_player()
        if not player:
            raise ValueError("No active player")
        charge = int(state.mode_state["charge"].get(player.id, 0))
        if not 8 <= charge <= 10:
            raise ValueError("FIRE needs a charge between 8 and 10")
        target = self._target(state, player)
        player.score += 50
        target.score = max(0, target.score - 25)
        state.mode_state["charge"][player.id] = 0
        state.message = f"FIRE! {player.name} +50 · {target.name} -25"

    def get_overlay(self, state: Any) -> Dict[str, Any]:
        player = state.current_player()
        charge = int(state.mode_state.get("charge", {}).get(player.id if player else "", 0))
        target = self._target(state, player) if player and len(state.players) > 1 else None
        return {
            "prompt": "FIRE drücken!" if 8 <= charge <= 10 else "Kanone auf 8–10 laden",
            "actions": [{
                "id": "fire",
                "label": f"FIRE AUF {target.name.upper()}" if target else "FIRE",
                "enabled": 8 <= charge <= 10 and state.status == "running",
            }],
            "panel": {
                "title": "CANDY CANNON",
                "headline": f"Ladung {charge}/10",
                "subline": f"Ziel: {target.name}" if 8 <= charge <= 10 and target else "Über 10 überhitzt die Kanone",
                "progress": {"value": charge, "max": 10},
            },
        }


GAME_MODE = CandyCannonMode()
