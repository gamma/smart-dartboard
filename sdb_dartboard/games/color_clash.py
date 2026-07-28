from __future__ import annotations

import random
from typing import Any, Dict, List

from .arcade import TARGET_POOL_NORMAL, overlay_item, same_target, zone_id
from .base import GameMetadata, GameOption, InstructionStep, ThrowOutcome

COLOR_SCORES = {"gold": 50, "cyan": 25, "green": 10, "red": -25}


class ColorClashMode:
    metadata = GameMetadata(
        slug="color_clash",
        title="Color Clash",
        tagline="Gold zählt, Rot tut weh",
        description="Das Board wird zur Arcade-Fläche: Farben bestimmen die Punkte, nicht der klassische Dartwert.",
        accent="#ffcf33",
        accent_secondary="#28e7ff",
        visual="color-clash",
        icon="palette",
        options=[
            GameOption("rounds", "Runden", "choice", 5, [{"value":3,"label":"3 Runden"},{"value":5,"label":"5 Runden"},{"value":8,"label":"8 Runden"}]),
            GameOption("shuffle", "Farbwechsel", "choice", "dart", [{"value":"dart","label":"Nach jedem Dart"},{"value":"turn","label":"Nach jeder Aufnahme"}]),
        ],
        instructions=[
            InstructionStep("Farben zählen", "Gold +50, Cyan +25, Grün +10, Rot -25.", "palette"),
            InstructionStep("Klassische Punkte egal", "Die Farbe des getroffenen Segments entscheidet.", "rules"),
            InstructionStep("Board mischt sich", "Je nach Variante wechseln Farben nach Dart oder Aufnahme.", "shuffle"),
        ],
        sound_theme="arcade",
    )

    def initialize_player(self, player: Any, options: Dict[str, Any]) -> None:
        player.score = 0
        player.marks = {}

    def initialize_state(self, state: Any, options: Dict[str, Any]) -> None:
        state.mode_state = {"colors": self._generate_colors()}
        state.message = "Gold zählt am meisten!"

    def _generate_colors(self) -> Dict[str, str]:
        pool = random.sample(TARGET_POOL_NORMAL, min(21, len(TARGET_POOL_NORMAL)))
        colors: Dict[str, str] = {}
        distribution = ["gold"] * 3 + ["cyan"] * 6 + ["green"] * 8 + ["red"] * 4
        random.shuffle(distribution)
        for dart, color in zip(pool, distribution):
            colors[zone_id(dart)] = color
        return colors

    def _refresh_if_needed(self, state: Any, force: bool = False) -> None:
        if force or state.options.get("shuffle", "dart") == "dart":
            state.mode_state["colors"] = self._generate_colors()

    def apply_throw(self, state: Any, player: Any, event: Dict[str, Any]) -> ThrowOutcome:
        if event.get("type") == "miss":
            self._refresh_if_needed(state)
            return ThrowOutcome(turn_value=0, message="Miss")
        colors = state.mode_state.get("colors", {})
        hit_id = zone_id({"label": event.get("label", ""), "field": event.get("field", 0), "ring": event.get("ring", "")})
        color = colors.get(hit_id)
        points = int(COLOR_SCORES.get(color or "", 0))
        player.score += points
        self._refresh_if_needed(state)
        label = event.get("label", "")
        if color:
            return ThrowOutcome(turn_value=points, message=f"{label}: {color} {points:+d}")
        return ThrowOutcome(turn_value=0, message=f"{label}: neutral")

    def get_overlay(self, state: Any) -> Dict[str, Any]:
        colors = state.mode_state.get("colors", {})
        by_color: Dict[str, List[Dict[str, Any]]] = {"gold": [], "cyan": [], "green": [], "red": []}
        for dart in TARGET_POOL_NORMAL:
            color = colors.get(zone_id(dart))
            if color in by_color:
                by_color[color].append(dart)
        return {
            "prompt": "Gold +50 · Cyan +25 · Grün +10 · Rot -25",
            "bonus": [overlay_item(d, "gold", "+50", False) for d in by_color["gold"]],
            "targets": [overlay_item(d, "cyan", "+25", False) for d in by_color["cyan"]] + [overlay_item(d, "green", "+10", False) for d in by_color["green"]],
            "danger": [overlay_item(d, "red", "-25", True) for d in by_color["red"]],
        }


GAME_MODE = ColorClashMode()
