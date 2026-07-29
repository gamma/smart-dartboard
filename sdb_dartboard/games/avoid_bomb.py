from __future__ import annotations

from typing import Any, Dict

from .arcade import choose_targets, finish_round_game, overlay_item, same_target
from .base import GameMetadata, GameOption, InstructionStep, ThrowOutcome


class AvoidBombMode:
    metadata = GameMetadata(
        slug="avoid_bomb",
        title="Avoid the Bomb",
        tagline="Sammle Punkte – meide Rot",
        description="Normale Treffer zählen, aber rote Bomben ziehen Punkte ab und sorgen für Party-Chaos.",
        accent="#ff4f79",
        accent_secondary="#ffb52b",
        visual="avoid-bomb",
        icon="bomb",
        options=[
            GameOption("rounds", "Runden", "choice", 5, [{"value":3,"label":"3 Runden"},{"value":5,"label":"5 Runden"},{"value":8,"label":"8 Runden"}]),
            GameOption("bomb_count", "Bomben", "choice", 4, [{"value":2,"label":"2 Bomben"},{"value":4,"label":"4 Bomben"},{"value":6,"label":"6 Bomben"}]),
            GameOption("penalty", "Strafe", "choice", -50, [{"value":-25,"label":"-25"},{"value":-50,"label":"-50"},{"value":-100,"label":"-100"}]),
        ],
        instructions=[
            InstructionStep("Rot ist gefährlich", "Rote Felder sind Bomben und kosten Punkte.", "danger"),
            InstructionStep("Alles andere zählt", "Normale Treffer geben ihren Dartwert.", "score"),
            InstructionStep("Schadenfreude", "Bombentreffer lösen eine Explosion aus.", "boom"),
        ],
        sound_theme="arcade",
    )

    def initialize_player(self, player: Any, options: Dict[str, Any]) -> None:
        player.score = 0
        player.marks = {}

    def initialize_state(self, state: Any, options: Dict[str, Any]) -> None:
        count = int(options.get("bomb_count", 4))
        bombs = choose_targets(count, "normal")
        state.mode_state = {"bombs": bombs}
        state.message = "Meide Rot!"

    def _refresh_bombs(self, state: Any) -> None:
        count = int(state.options.get("bomb_count", 4))
        state.mode_state["bombs"] = choose_targets(count, "normal")

    def apply_throw(self, state: Any, player: Any, event: Dict[str, Any]) -> ThrowOutcome:
        bombs = state.mode_state.get("bombs", [])
        if event.get("type") == "miss":
            outcome = ThrowOutcome(turn_value=0, message="Miss")
        elif any(same_target(event, bomb) for bomb in bombs):
            penalty = int(state.options.get("penalty", -50))
            player.score += penalty
            self._refresh_bombs(state)
            outcome = ThrowOutcome(turn_value=penalty, message=f"BOMB! {penalty}")
        else:
            score = int(event.get("score", 0))
            player.score += score
            outcome = ThrowOutcome(turn_value=score, message=f"Safe {event.get('label', '')} +{score}")
        return finish_round_game(
            state, outcome, "{winner} überlebt Avoid the Bomb!"
        )

    def get_overlay(self, state: Any) -> Dict[str, Any]:
        bombs = state.mode_state.get("bombs", [])
        return {
            "prompt": "Sammle Punkte – meide Rot!",
            "danger": [overlay_item(bomb, "red", "BOMB", True) for bomb in bombs],
        }


GAME_MODE = AvoidBombMode()
