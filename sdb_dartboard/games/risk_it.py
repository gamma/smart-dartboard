from __future__ import annotations

from typing import Any, Dict

from .arcade import overlay_item
from .base import GameMetadata, GameOption, InstructionStep, ThrowOutcome


class RiskItMode:
    metadata = GameMetadata(
        slug="risk_it",
        title="Risk It",
        tagline="Sichern oder weiter zocken",
        description="Treffer landen im Pot. Banke rechtzeitig – ein Miss verliert alles im aktuellen Pot.",
        accent="#ffb52b",
        accent_secondary="#ff4f79",
        visual="risk-it",
        icon="dice",
        options=[
            GameOption("rounds", "Runden", "choice", 5, [{"value":3,"label":"3 Runden"},{"value":5,"label":"5 Runden"},{"value":8,"label":"8 Runden"}]),
            GameOption("miss_loses", "Miss", "choice", "pot", [{"value":"pot","label":"Pot verlieren"},{"value":"half","label":"Pot halbieren"}]),
        ],
        instructions=[
            InstructionStep("Pot sammeln", "Jeder Treffer erhöht deinen Aufnahme-Pot.", "pot"),
            InstructionStep("Bank drücken", "Sichere den Pot über den Control-Screen.", "bank"),
            InstructionStep("Miss tut weh", "Ein Miss verliert oder halbiert den Pot.", "risk"),
        ],
        sound_theme="arcade",
    )

    def initialize_player(self, player: Any, options: Dict[str, Any]) -> None:
        player.score = 0
        player.marks = {}

    def initialize_state(self, state: Any, options: Dict[str, Any]) -> None:
        state.mode_state = {"pot": {}, "banked_last": 0}
        state.message = "Risk it: Punkte sammeln oder banken!"

    def _pot(self, state: Any, player_id: str) -> int:
        return int(state.mode_state.setdefault("pot", {}).get(player_id, 0))

    def _set_pot(self, state: Any, player_id: str, value: int) -> None:
        state.mode_state.setdefault("pot", {})[player_id] = max(0, int(value))

    def apply_throw(self, state: Any, player: Any, event: Dict[str, Any]) -> ThrowOutcome:
        pot = self._pot(state, player.id)
        if event.get("type") == "miss":
            if state.options.get("miss_loses", "pot") == "half":
                new_pot = pot // 2
                self._set_pot(state, player.id, new_pot)
                return ThrowOutcome(turn_value=0, message=f"Miss – Pot halbiert auf {new_pot}")
            self._set_pot(state, player.id, 0)
            return ThrowOutcome(turn_value=0, message="Miss – Pot verloren", force_hold=True)
        score = int(event.get("score", 0))
        pot += score
        self._set_pot(state, player.id, pot)
        is_last_dart = state.darts_in_turn == 2
        if is_last_dart:
            player.score += pot
            state.mode_state["banked_last"] = pot
            self._set_pot(state, player.id, 0)
            return ThrowOutcome(turn_value=score, message=f"Auto-Bank +{pot}")
        return ThrowOutcome(turn_value=score, message=f"Pot {pot} – banken oder riskieren?")

    def handle_action(self, state: Any, action: str, payload: Dict[str, Any]) -> None:
        if action != "bank":
            raise ValueError(f"Unsupported action for Risk It: {action}")
        player = state.current_player()
        if not player or state.status != "running":
            return
        pot = self._pot(state, player.id)
        player.score += pot
        state.mode_state["banked_last"] = pot
        self._set_pot(state, player.id, 0)
        state.status = "hold"
        state.message = f"{player.name} bankt +{pot}"

    def get_overlay(self, state: Any) -> Dict[str, Any]:
        player = state.current_player()
        pot = self._pot(state, player.id) if player else 0
        return {
            "prompt": f"POT {pot} – BANK oder RISK?",
            "bonus": [],
            "targets": [],
            "danger": [],
            "pot": pot,
            "actions": [{"id": "bank", "label": f"BANK +{pot}", "enabled": pot > 0}],
        }


GAME_MODE = RiskItMode()
