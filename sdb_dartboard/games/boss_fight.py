from __future__ import annotations

from typing import Any, Dict

from .arcade import (
    choose_targets_for_state,
    overlay_item,
    result_message,
    same_target,
)
from .base import GameMetadata, GameOption, InstructionStep, ThrowOutcome


class BossFightMode:
    metadata = GameMetadata(
        slug="boss_fight",
        title="Boss Fight",
        tagline="Alle gegen den Boss",
        description="Gemeinsame Boss-Jagd mit Rundenlimit: Treffer verursachen Schaden, Schwachpunkte geben Bonusdamage. Bei Erfolg gewinnt das ganze Team.",
        accent="#ff4f79",
        accent_secondary="#9b5cff",
        visual="boss-fight",
        icon="monster",
        options=[
            GameOption("boss_hp", "Boss HP", "choice", 1000, [{"value":600,"label":"600 HP"},{"value":1000,"label":"1000 HP"},{"value":1500,"label":"1500 HP"}]),
            GameOption("weak_points", "Schwachpunkte", "choice", 3, [{"value":2,"label":"2"},{"value":3,"label":"3"},{"value":5,"label":"5"}]),
            GameOption("rounds", "Rundenlimit", "choice", 8, [{"value":5,"label":"5 Runden"},{"value":8,"label":"8 Runden"},{"value":12,"label":"12 Runden"}]),
        ],
        instructions=[
            InstructionStep("Schaden machen", "Jeder Treffer zieht Boss-HP ab.", "damage"),
            InstructionStep("Schwachpunkte", "Goldene Ziele verursachen doppelten Schaden.", "weak"),
            InstructionStep("Zeitlimit", "Besiegt den Boss innerhalb der gewählten Runden. Alle erhalten den Sieg; der meiste Schaden wird nur als MVP geehrt.", "coop"),
        ],
        sound_theme="arcade",
    )

    def initialize_player(self, player: Any, options: Dict[str, Any]) -> None:
        player.score = 0; player.marks = {}

    def initialize_state(self, state: Any, options: Dict[str, Any]) -> None:
        maximum = int(options.get("boss_hp", 1000))
        weak = choose_targets_for_state(
            state, int(options.get("weak_points", 3)), "normal"
        )
        state.mode_state = {
            "boss_hp": maximum,
            "max_hp": maximum,
            "weak": weak,
            "last_effect": "",
            "effect_damage": 0,
            "effect_weak": False,
            "effect_player_id": None,
        }
        state.message = "Boss Fight!"

    def _refresh_weak(self, state: Any) -> None:
        state.mode_state["weak"] = choose_targets_for_state(
            state, int(state.options.get("weak_points", 3)), "normal"
        )

    def apply_throw(self, state: Any, player: Any, event: Dict[str, Any]) -> ThrowOutcome:
        state.mode_state.update({
            "last_effect": "",
            "effect_damage": 0,
            "effect_weak": False,
            "effect_player_id": None,
        })
        is_hit = event.get("type") == "hit"
        weak = state.mode_state.get("weak", [])
        base = int(event.get("score", 0)) if is_hit else 0
        is_weak = is_hit and any(same_target(event, item) for item in weak)
        damage = base * (2 if is_weak else 1)
        if damage:
            player.score += damage
            state.mode_state["boss_hp"] = max(
                0, int(state.mode_state.get("boss_hp", 0)) - damage
            )
            if is_weak:
                self._refresh_weak(state)
            state.mode_state.update({
                "last_effect": "boss_weak" if is_weak else "boss_hit",
                "effect_damage": damage,
                "effect_weak": is_weak,
                "effect_player_id": player.id,
            })
        if state.mode_state["boss_hp"] <= 0:
            _, result = result_message(
                state.players, "Boss besiegt! MVP: {winner}"
            )
            state.mode_state["last_effect"] = "boss_defeated"
            return ThrowOutcome(
                turn_value=damage,
                message=result,
                finished=True,
                winner_ids=[candidate.id for candidate in state.players],
                result_type="team_win",
            )
        final_dart = state.darts_in_turn == 2
        final_player = state.current_player_index == len(state.players) - 1
        final_round = state.round_number >= int(state.options.get("rounds", 8))
        if final_dart and final_player and final_round:
            state.mode_state["last_effect"] = "boss_victory"
            return ThrowOutcome(
                turn_value=damage,
                message=f"Boss gewinnt mit {state.mode_state['boss_hp']} HP!",
                finished=True,
                result_type="challenge_loss",
            )
        message = (
            f"{event.get('label','')} macht {damage} Schaden"
            if is_hit
            else "Miss – kein Schaden"
        )
        return ThrowOutcome(turn_value=damage, message=message)

    def on_turn_skipped(self, state: Any, player: Any) -> None:
        del player
        final_player = state.current_player_index == len(state.players) - 1
        final_round = state.round_number >= int(state.options.get("rounds", 8))
        if final_player and final_round:
            state.status = "finished"
            state.winner_id = None
            state.winner_ids = []
            state.result_type = "challenge_loss"
            state.mode_state["last_effect"] = "boss_victory"
            state.message = (
                f"Boss gewinnt mit {state.mode_state['boss_hp']} HP!"
            )

    def get_overlay(self, state: Any) -> Dict[str, Any]:
        hp = int(state.mode_state.get("boss_hp", 0))
        weak = state.mode_state.get("weak", [])
        return {
            "prompt": f"BOSS HP {hp} – Gold = Schwachpunkt",
            "bonus": [overlay_item(item, "gold", "x2", True) for item in weak],
            "boss": {"hp": hp, "max_hp": int(state.options.get("boss_hp", 1000))},
        }


GAME_MODE = BossFightMode()
