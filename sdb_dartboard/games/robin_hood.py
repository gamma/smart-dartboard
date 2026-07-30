from __future__ import annotations

from typing import Any, Dict, List

from .arcade import choose_targets, finish_round_game, overlay_item, same_field, same_target
from .base import GameMetadata, GameOption, InstructionStep, ThrowOutcome


def event_dart(event: Dict[str, Any]) -> Dict[str, Any]:
    return {
        "label": str(event.get("label", "")),
        "field": int(event.get("field", 0)),
        "ring": str(event.get("ring", "")),
        "multiplier": int(event.get("multiplier", 1)),
        "score": int(event.get("score", 0)),
    }


class RobinHoodMode:
    metadata = GameMetadata(
        slug="robin_hood",
        title="Robin Hood Hunt",
        tagline="Spalte die Sheriff-Pfeile",
        description="Jage die drei Ziele des Vorgängers. Jeder eigene Treffer wird danach zum Ziel für den nächsten Spieler.",
        accent="#5aa469",
        accent_secondary="#f4b942",
        visual="robin-hood",
        icon="arrow",
        min_players=2,
        options=[
            GameOption("rounds", "Runden", "choice", 5, [
                {"value": 5, "label": "5 Runden"},
                {"value": 8, "label": "8 Runden"},
            ]),
            GameOption("matching", "Trefferregel", "choice", "exact", [
                {"value": "exact", "label": "Exact Ring"},
                {"value": "number", "label": "Same Number"},
            ]),
        ],
        instructions=[
            InstructionStep("Pfeile jagen", "Triff die Sheriff-Ziele. Doppelte Ziele zählen separat.", "target"),
            InstructionStep("Split-Punkte", "Ein Split gibt 30 Punkte plus den Wert des Sheriff-Pfeils.", "arrow"),
            InstructionStep("Ziele weitergeben", "Deine gültigen Treffer werden die Ziele des nächsten Spielers.", "shuffle"),
        ],
        sound_theme="arcade",
    )

    def initialize_player(self, player: Any, options: Dict[str, Any]) -> None:
        player.score = 0
        player.marks = {}

    def initialize_state(self, state: Any, options: Dict[str, Any]) -> None:
        targets = choose_targets(3, "normal")
        state.mode_state = {
            "sheriff_targets": targets,
            "remaining_targets": list(targets),
            "current_arrows": [],
            "splits": {player.id: 0 for player in state.players},
        }
        state.message = "Die Sheriff-Pfeile liegen bereit!"

    def on_turn_start(self, state: Any, player: Any) -> None:
        state.mode_state["remaining_targets"] = list(
            state.mode_state.get("sheriff_targets", [])
        )
        state.mode_state["current_arrows"] = []

    def apply_throw(self, state: Any, player: Any, event: Dict[str, Any]) -> ThrowOutcome:
        is_hit = event.get("type") == "hit"
        if is_hit:
            state.mode_state.setdefault("current_arrows", []).append(event_dart(event))
        remaining: List[Dict[str, Any]] = state.mode_state.setdefault(
            "remaining_targets", []
        )
        match_index = None
        if is_hit:
            matcher = same_target if state.options.get("matching") == "exact" else same_field
            match_index = next(
                (index for index, target in enumerate(remaining) if matcher(event, target)),
                None,
            )
        if match_index is None:
            outcome = ThrowOutcome(
                turn_value=0,
                message="Kein Sheriff-Pfeil gespalten",
            )
        else:
            target = remaining.pop(match_index)
            points = 30 + int(target.get("score", 0))
            player.score += points
            state.mode_state["splits"][player.id] += 1
            outcome = ThrowOutcome(
                turn_value=points,
                message=f"SPLIT! {target['label']} +{points}",
            )

        if state.darts_in_turn == 2:
            state.mode_state["sheriff_targets"] = list(
                state.mode_state.get("current_arrows", [])
            )
        return finish_round_game(
            state,
            outcome,
            "{winner} ist der beste Pfeilspalter!",
        )

    def on_turn_skipped(self, state: Any, player: Any) -> None:
        del player
        state.mode_state["sheriff_targets"] = list(
            state.mode_state.get("current_arrows", [])
        )

    def get_overlay(self, state: Any) -> Dict[str, Any]:
        targets = state.mode_state.get("remaining_targets", [])
        next_targets = state.mode_state.get("sheriff_targets", [])
        shown = next_targets if state.status == "hold" else targets
        return {
            "prompt": "Freie Runde – lege neue Pfeile!" if not shown else "Spalte: " + " · ".join(item["label"] for item in shown),
            "targets": [overlay_item(item, "green", "SPLIT", False) for item in shown],
            "panel": {
                "title": "SHERIFF-PFEILE",
                "headline": f"{len(targets)} noch offen",
                "rows": [
                    {"label": player.name, "value": f"{state.mode_state['splits'].get(player.id, 0)} Splits"}
                    for player in state.players
                ],
            },
        }


GAME_MODE = RobinHoodMode()
