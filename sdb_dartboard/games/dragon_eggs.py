from __future__ import annotations

from typing import Any, Dict

from .arcade import choose_targets, finish_round_game, overlay_item, same_target, zone_id
from .base import GameMetadata, GameOption, InstructionStep, ThrowOutcome


class DragonEggsMode:
    metadata = GameMetadata(
        slug="dragon_eggs",
        title="Dragon Eggs",
        tagline="Eier bergen, Drachenfeuer vermeiden",
        description="Sammle goldene Eier. Jede rote Schuppe heizt den Drachen auf – die dritte entfacht sein Feuer.",
        accent="#f4a261",
        accent_secondary="#6ab04c",
        visual="dragon-eggs",
        icon="egg",
        options=[
            GameOption("rounds", "Runden", "choice", 5, [
                {"value": 5, "label": "5 Runden"},
                {"value": 8, "label": "8 Runden"},
            ]),
            GameOption("eggs", "Dracheneier", "choice", 4, [
                {"value": 3, "label": "3 Eier"},
                {"value": 4, "label": "4 Eier"},
                {"value": 6, "label": "6 Eier"},
            ]),
        ],
        instructions=[
            InstructionStep("Goldenes Ei", "Ein sichtbares Ei bringt einmal pro Runde +30 Punkte.", "egg"),
            InstructionStep("Rote Schuppe", "Eine Schuppe kostet 15 Punkte und füllt eine Flamme.", "danger"),
            InstructionStep("Drachenfeuer", "Die dritte Flamme verbrennt zusätzlich die Hälfte deiner positiven Punkte dieses Zugs.", "dragon"),
        ],
        sound_theme="arcade",
    )

    def initialize_player(self, player: Any, options: Dict[str, Any]) -> None:
        player.score = 0
        player.marks = {}

    def initialize_state(self, state: Any, options: Dict[str, Any]) -> None:
        state.mode_state = {
            "heat": {player.id: 0 for player in state.players},
            "turn_positive": {player.id: 0 for player in state.players},
            "collected": {player.id: [] for player in state.players},
            "layout_round": state.round_number,
        }
        self._shuffle(state)

    def _shuffle(self, state: Any) -> None:
        eggs = choose_targets(int(state.options.get("eggs", 4)), "normal")
        scales = choose_targets(8, "normal", exclude=[zone_id(item) for item in eggs])
        state.mode_state["eggs"] = eggs
        state.mode_state["scales"] = scales
        state.mode_state["collected"] = {player.id: [] for player in state.players}
        state.message = "Goldene Eier sammeln · rote Schuppen meiden!"

    def on_turn_start(self, state: Any, player: Any) -> None:
        state.mode_state["turn_positive"][player.id] = 0
        if int(state.mode_state.get("layout_round", 0)) != state.round_number:
            self._shuffle(state)
            state.mode_state["layout_round"] = state.round_number

    def apply_throw(self, state: Any, player: Any, event: Dict[str, Any]) -> ThrowOutcome:
        eggs = state.mode_state.get("eggs", [])
        scales = state.mode_state.get("scales", [])
        egg = next(
            (item for item in eggs if event.get("type") == "hit" and same_target(event, item)),
            None,
        )
        collected = state.mode_state.setdefault("collected", {}).setdefault(player.id, [])
        if egg and zone_id(egg) not in collected:
            points = 30
            collected.append(zone_id(egg))
            player.score += points
            state.mode_state["turn_positive"][player.id] += points
            event["effect"] = "dragon_egg"
            outcome = ThrowOutcome(points, "Ei geknackt! +30")
        elif egg:
            outcome = ThrowOutcome(0, "Dieses Ei ist schon leer")
        elif event.get("type") == "hit" and any(same_target(event, item) for item in scales):
            heat = int(state.mode_state["heat"].get(player.id, 0)) + 1
            points = -15
            event["effect"] = "dragon_scale"
            event["dragon_heat"] = heat
            message = f"Schuppe! -15 · Hitze {heat}/3"
            if heat >= 3:
                penalty = int(state.mode_state["turn_positive"].get(player.id, 0)) // 2
                points -= penalty
                heat = 0
                event["effect"] = "dragon_fire"
                event["dragon_fire_penalty"] = 15 + penalty
                event["dragon_heat"] = 0
                message = f"DRACHENFEUER! -{15 + penalty}"
            state.mode_state["heat"][player.id] = heat
            player.score += points
            outcome = ThrowOutcome(points, message)
        else:
            outcome = ThrowOutcome(0, "Kein Ei gefunden")
        return finish_round_game(state, outcome, "{winner} hütet den Drachenschatz!")

    def get_overlay(self, state: Any) -> Dict[str, Any]:
        current = state.current_player()
        player_id = current.id if current else ""
        heat = int(state.mode_state.get("heat", {}).get(player_id, 0))
        collected = set(state.mode_state.get("collected", {}).get(player_id, []))
        eggs = [
            item
            for item in state.mode_state.get("eggs", [])
            if zone_id(item) not in collected
        ]
        return {
            "prompt": "GOLDENE EIER SAMMELN · ROTE SCHUPPEN MEIDEN",
            "bonus": [
                overlay_item(item, "gold", "+30", False)
                | {"icon": "egg", "variant": "dragon-egg"}
                for item in eggs
            ],
            "danger": [
                overlay_item(item, "red", "HITZE", True)
                | {"icon": "dragon_scale", "variant": "dragon-scale"}
                for item in state.mode_state.get("scales", [])
            ],
            "visual_legend": [
                {"icon": "egg", "label": "GOLDENES EI", "value": "+30", "color": "#f4c95d"},
                {"icon": "dragon_scale", "label": "ROTE SCHUPPE", "value": "-15 · +1 HITZE", "color": "#f05d5e"},
            ],
            "panel": {
                "kind": "dragon_heat",
                "title": "DRACHEN-HITZE",
                "heat": heat,
                "headline": f"{heat}/3 FLAMMEN",
                "subline": "Noch eine Schuppe: Feuer!" if heat == 2 else "Die dritte Schuppe entfacht das Feuer",
                "progress": {"value": heat, "max": 3},
            },
        }


GAME_MODE = DragonEggsMode()
