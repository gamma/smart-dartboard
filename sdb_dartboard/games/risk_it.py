from __future__ import annotations

from typing import Any, Dict, Optional

from .arcade import finish_action_round_game, finish_round_game, result_message
from .base import GameMetadata, GameOption, InstructionStep, ThrowOutcome


class RiskItMode:
    metadata = GameMetadata(
        slug="risk_it",
        title="Risk It",
        tagline="Banken oder den Hot Pot riskieren",
        description=(
            "Treffer füllen deinen Pot. Banke nach Dart 1 oder 2 – nach Dart 3 "
            "kann der nächste Spieler den Pot mit einem Treffer stehlen."
        ),
        accent="#ffb52b",
        accent_secondary="#ff4f79",
        visual="risk-it",
        icon="dice",
        options=[
            GameOption("rounds", "Runden", "choice", 5, [
                {"value": 3, "label": "3 Runden"},
                {"value": 5, "label": "5 Runden"},
                {"value": 8, "label": "8 Runden"},
            ]),
            GameOption("miss_loses", "Miss", "choice", "pot", [
                {"value": "pot", "label": "Pot verlieren", "description": "Ein Miss löscht den gesamten eigenen ungesicherten Pot.", "description_en": "A miss removes your entire unsecured pot."},
                {"value": "half", "label": "Pot halbieren", "description": "Ein Miss halbiert den eigenen ungesicherten Pot; bei Dart 3 wird die Hälfte gesichert.", "description_en": "A miss halves your unsecured pot; on dart 3, half is secured."},
            ]),
        ],
        instructions=[
            InstructionStep(
                "Pot füllen",
                "Jeder Treffer erhöht deinen ungesicherten Pot.",
                "pot",
            ),
            InstructionStep(
                "Nach Dart 1 oder 2 banken",
                "BANK sichert den Pot und beendet deinen Zug.",
                "bank",
            ),
            InstructionStep(
                "Dart 3 ist Risiko",
                "Ein Treffer macht seine Zahl zum leuchtenden Hot-Pot-Ziel.",
                "risk",
            ),
            InstructionStep(
                "Erster Dart kann stehlen",
                "Der nächste Spieler trifft die Zahl und stiehlt den Pot. Sonst wird er für dich gesichert.",
                "target",
            ),
        ],
        sound_theme="arcade",
        ruleset_version=2,
    )

    def initialize_player(self, player: Any, options: Dict[str, Any]) -> None:
        player.score = 0
        player.marks = {}

    def initialize_state(self, state: Any, options: Dict[str, Any]) -> None:
        state.mode_state = {
            "pot": {},
            "banked_last": 0,
            "hot_pot": None,
            "final_heist": False,
        }
        state.message = "Risk It: Pot füllen, banken oder Dart 3 riskieren!"

    def _pot(self, state: Any, player_id: str) -> int:
        return int(state.mode_state.setdefault("pot", {}).get(player_id, 0))

    def _set_pot(self, state: Any, player_id: str, value: int) -> None:
        state.mode_state.setdefault("pot", {})[player_id] = max(0, int(value))

    @staticmethod
    def _player(state: Any, player_id: str) -> Optional[Any]:
        return next((player for player in state.players if player.id == player_id), None)

    @staticmethod
    def _is_hit(event: Dict[str, Any]) -> bool:
        return event.get("type") == "hit"

    @staticmethod
    def _field(event: Dict[str, Any]) -> int:
        return int(event.get("field", 0) or 0)

    def _finish_outcome(self, state: Any, outcome: ThrowOutcome) -> ThrowOutcome:
        winner_id, message = result_message(
            state.players,
            "{winner} gewinnt Risk It!",
        )
        outcome.finished = True
        outcome.winner_id = winner_id
        outcome.winner_ids = [winner_id] if winner_id else []
        outcome.result_type = "individual_win" if winner_id else "draw"
        outcome.force_hold = False
        outcome.message = message
        state.mode_state["final_heist"] = False
        return outcome

    def _finish_state(self, state: Any) -> None:
        winner_id, message = result_message(
            state.players,
            "{winner} gewinnt Risk It!",
        )
        state.status = "finished"
        state.winner_id = winner_id
        state.winner_ids = [winner_id] if winner_id else []
        state.result_type = "individual_win" if winner_id else "draw"
        state.message = message
        state.mode_state["final_heist"] = False

    def _resolve_hot_pot(
        self,
        state: Any,
        attacker: Any,
        event: Optional[Dict[str, Any]],
    ) -> tuple[int, str]:
        hot = state.mode_state.get("hot_pot")
        if not hot:
            return 0, ""
        owner = self._player(state, str(hot.get("owner_id", "")))
        amount = int(hot.get("amount", 0))
        target_field = int(hot.get("field", 0))
        stolen = bool(
            owner
            and owner.id != attacker.id
            and event
            and self._is_hit(event)
            and self._field(event) == target_field
        )
        if owner:
            self._set_pot(state, owner.id, 0)
        state.mode_state["hot_pot"] = None
        if stolen:
            attacker.score += amount
            state.mode_state["banked_last"] = amount
            if event is not None:
                event.update({
                    "effect": "risk_steal",
                    "stolen_amount": amount,
                    "target_player_id": owner.id,
                })
            return amount, f"HEIST! {attacker.name} stiehlt {amount} von {owner.name}"
        if owner:
            owner.score += amount
            state.mode_state["banked_last"] = amount
            if event is not None:
                event.update({
                    "effect": "risk_secured",
                    "secured_amount": amount,
                    "target_player_id": owner.id,
                })
            return 0, f"SAFE! {owner.name} bankt {amount}"
        return 0, ""

    def _make_hot_pot(
        self,
        state: Any,
        player: Any,
        event: Dict[str, Any],
        amount: int,
    ) -> None:
        state.mode_state["hot_pot"] = {
            "owner_id": player.id,
            "amount": amount,
            "field": self._field(event),
            "label": "BULL" if self._field(event) == 25 else str(self._field(event)),
        }
        event.update({
            "effect": "risk_hot_pot",
            "hot_pot": amount,
            "hot_target": self._field(event),
        })

    def apply_throw(
        self,
        state: Any,
        player: Any,
        event: Dict[str, Any],
    ) -> ThrowOutcome:
        final_heist = bool(state.mode_state.get("final_heist"))
        stolen, hot_message = self._resolve_hot_pot(state, player, event)
        if final_heist:
            outcome = ThrowOutcome(stolen, hot_message or "Finaler Hot Pot gesichert")
            return self._finish_outcome(state, outcome)

        pot = self._pot(state, player.id)
        is_last_dart = state.darts_in_turn == 2
        if not self._is_hit(event):
            if state.options.get("miss_loses", "pot") == "half":
                new_pot = pot // 2
                if is_last_dart:
                    player.score += new_pot
                    state.mode_state["banked_last"] = new_pot
                    self._set_pot(state, player.id, 0)
                    message = f"Miss · halber Pot gesichert +{new_pot}"
                else:
                    self._set_pot(state, player.id, new_pot)
                    message = f"Miss · Pot halbiert auf {new_pot}"
                outcome = ThrowOutcome(stolen, self._join_messages(hot_message, message))
            else:
                self._set_pot(state, player.id, 0)
                outcome = ThrowOutcome(
                    stolen,
                    self._join_messages(hot_message, "Miss · eigener Pot verloren"),
                    force_hold=True,
                )
        else:
            score = int(event.get("score", 0))
            pot += score
            self._set_pot(state, player.id, pot)
            if is_last_dart:
                if len(state.players) == 1:
                    player.score += pot
                    state.mode_state["banked_last"] = pot
                    self._set_pot(state, player.id, 0)
                    message = f"Solo Auto-Bank +{pot}"
                else:
                    self._make_hot_pot(state, player, event, pot)
                    message = f"HOT POT {pot} · Ziel {state.mode_state['hot_pot']['label']}"
                    is_final_player = state.current_player_index == len(state.players) - 1
                    is_final_round = state.round_number >= int(state.options.get("rounds", 1))
                    if is_final_player and is_final_round:
                        state.mode_state["final_heist"] = True
            else:
                message = f"Pot {pot} · BANK oder weiter?"
            # The stolen pot is banked directly. Only this dart's own value
            # belongs in the current visit counter and throw telemetry.
            outcome = ThrowOutcome(score, self._join_messages(hot_message, message))

        if state.mode_state.get("final_heist"):
            return outcome
        return finish_round_game(state, outcome, "{winner} gewinnt Risk It!")

    @staticmethod
    def _join_messages(first: str, second: str) -> str:
        return " · ".join(part for part in (first, second) if part)

    def handle_action(
        self,
        state: Any,
        action: str,
        payload: Dict[str, Any],
    ) -> None:
        if action != "bank":
            raise ValueError(f"Unsupported action for Risk It: {action}")
        player = state.current_player()
        if not player or state.status != "running":
            return
        pot = self._pot(state, player.id)
        if state.darts_in_turn not in (1, 2) or pot <= 0:
            raise ValueError("The pot can only be banked after dart 1 or 2")
        player.score += pot
        state.mode_state["banked_last"] = pot
        self._set_pot(state, player.id, 0)
        state.status = "hold"
        state.message = f"{player.name} bankt +{pot}"
        finish_action_round_game(state, "{winner} gewinnt Risk It!")

    def on_turn_start(self, state: Any, player: Any) -> None:
        hot = state.mode_state.get("hot_pot")
        if hot and str(hot.get("owner_id")) == player.id:
            self._resolve_hot_pot(state, player, None)

    def on_turn_skipped(self, state: Any, player: Any) -> None:
        _, hot_message = self._resolve_hot_pot(state, player, None)
        own_pot = self._pot(state, player.id)
        self._set_pot(state, player.id, 0)
        state.message = self._join_messages(
            hot_message,
            f"{player.name} überspringt · Pot {own_pot} verloren",
        )
        if state.mode_state.get("final_heist"):
            self._finish_state(state)

    @staticmethod
    def _target_items(hot: Dict[str, Any]) -> list[Dict[str, Any]]:
        field = int(hot.get("field", 0))
        rings = (
            ["single_bull", "double_bull"]
            if field == 25
            else ["single_inner", "triple", "single_outer", "double"]
        )
        return [
            {
                "id": f"HEIST-{field}-{ring}",
                "field": field,
                "ring": ring,
                "color": "#ff4f79",
                "label": "STEAL" if index == 0 else "",
                "pulse": True,
            }
            for index, ring in enumerate(rings)
        ]

    def get_overlay(self, state: Any) -> Dict[str, Any]:
        player = state.current_player()
        pot = self._pot(state, player.id) if player else 0
        hot = state.mode_state.get("hot_pot")
        attack_open = bool(
            hot
            and player
            and str(hot.get("owner_id")) != player.id
            and state.darts_in_turn == 0
        )
        if attack_open:
            owner = self._player(state, str(hot.get("owner_id", "")))
            prompt = f"TRIFF {hot['label']} MIT DART 1 · STIEHL {hot['amount']}"
            panel = {
                "title": "HOT POT",
                "headline": f"{hot['amount']} PUNKTE",
                "subline": f"{player.name}: Triff {hot['label']} mit Dart 1",
                "stats": [
                    {"label": "BESITZER", "value": owner.name if owner else "—"},
                    {"label": "DIEBSTAHL-ZIEL", "value": hot["label"]},
                ],
            }
        elif hot:
            next_player = state.players[(state.current_player_index + 1) % len(state.players)]
            prompt = f"HOT POT {hot['amount']} · ZIEL {hot['label']}"
            panel = {
                "title": "HOT POT",
                "headline": f"{hot['amount']} PUNKTE",
                "subline": f"{next_player.name} kann mit Dart 1 auf {hot['label']} stehlen",
                "stats": [{"label": "DIEBSTAHL-ZIEL", "value": hot["label"]}],
            }
        else:
            decision = (
                "BANKEN ODER DART 3 RISKIEREN"
                if state.darts_in_turn == 2 and pot > 0
                else "BANKEN ODER WEITERWERFEN"
                if pot > 0
                else "TREFFER FÜLLEN DEINEN POT"
            )
            prompt = f"POT {pot} · {decision}"
            panel = {
                "title": "UNGESICHERTER POT",
                "headline": str(pot),
                "subline": decision,
            }
        can_bank = bool(
            player
            and state.status == "running"
            and state.darts_in_turn in (1, 2)
            and pot > 0
        )
        return {
            "prompt": prompt,
            "bonus": [],
            "targets": self._target_items(hot) if attack_open else [],
            "danger": [],
            "pot": pot,
            "hot_pot": hot,
            "panel": panel,
            "actions": (
                [{"id": "bank", "label": f"BANK +{pot}", "enabled": True}]
                if can_bank
                else []
            ),
        }


GAME_MODE = RiskItMode()
