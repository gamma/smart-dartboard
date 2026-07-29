from __future__ import annotations

import random
from typing import Any, Dict, List

from .arcade import TARGET_POOL_NORMAL, finish_round_game, overlay_item, zone_id
from .base import GameMetadata, GameOption, InstructionStep, ThrowOutcome

COOKIE_POINTS = {"gold": 50, "blue": 25, "green": 10, "moldy": -30}
COOKIE_COLORS = {
    "gold": "#ffcf33",
    "blue": "#55c7dc",
    "green": "#69c98f",
    "moldy": "#9dac76",
}
MILK_BULLS = [
    {"label": "SBull", "field": 25, "ring": "single_bull"},
    {"label": "DBull", "field": 25, "ring": "double_bull"},
]


class CookieMonsterMode:
    metadata = GameMetadata(
        slug="cookie_monster",
        title="Cookie Monster",
        tagline="Naschen, retten, Sugar Rush",
        description="Triff die sichtbaren Cookies auf der Scheibe, meide Schimmel und rette deinen Turn mit Bull-Milch.",
        accent="#e9a23b",
        accent_secondary="#68b0ab",
        visual="cookie-monster",
        icon="cookie",
        options=[
            GameOption("rounds", "Runden", "choice", 5, [
                {"value": 5, "label": "5 Runden"},
                {"value": 8, "label": "8 Runden"},
            ]),
        ],
        instructions=[
            InstructionStep("Cookie-Feld treffen", "Nur Segmente mit einem sichtbaren Cookie geben Cookie-Punkte.", "cookie"),
            InstructionStep("Cookie-Farbe zählt", "Gold +50, Blau +25, Grün +10 und Schimmel -30.", "score"),
            InstructionStep("Bull ist Milch", "Bull verdoppelt einen positiven Turn oder rettet einen negativen.", "milk"),
            InstructionStep("Drei gute laden Rush", "Der nächste gute Cookie zählt danach doppelt.", "combo"),
        ],
        sound_theme="arcade",
    )

    def initialize_player(self, player: Any, options: Dict[str, Any]) -> None:
        player.score = 0
        player.marks = {}

    def initialize_state(self, state: Any, options: Dict[str, Any]) -> None:
        state.mode_state = {
            "streak": {player.id: 0 for player in state.players},
            "sugar": {player.id: False for player in state.players},
        }
        self._shuffle(state)

    def _shuffle(self, state: Any) -> None:
        pool = [dart for dart in TARGET_POOL_NORMAL if int(dart["field"]) != 25]
        chosen = random.sample(pool, 12)  # nosec B311
        kinds = ["gold"] * 2 + ["blue"] * 3 + ["green"] * 4 + ["moldy"] * 3
        random.shuffle(kinds)  # nosec B311
        state.mode_state["cookies"] = {
            zone_id(dart): {"dart": dart, "kind": kind}
            for dart, kind in zip(chosen, kinds)
        }

    def on_turn_start(self, state: Any, player: Any) -> None:
        self._shuffle(state)

    def _reset_streak(self, state: Any, player: Any) -> None:
        state.mode_state["streak"][player.id] = 0

    def _cookie_overlay(
        self,
        dart: Dict[str, Any],
        kind: str,
        label: str,
    ) -> Dict[str, Any]:
        item = overlay_item(dart, COOKIE_COLORS[kind], label, kind == "moldy")
        item.update({
            "icon": "cookie_moldy" if kind == "moldy" else "cookie",
            "variant": kind,
        })
        return item

    def apply_throw(self, state: Any, player: Any, event: Dict[str, Any]) -> ThrowOutcome:
        is_bull = event.get("type") == "hit" and int(event.get("field", 0)) == 25
        if is_bull:
            current = int(state.turn_score)
            adjustment = current if current > 0 else -current if current < 0 else 0
            player.score += adjustment
            self._reset_streak(state, player)
            message = f"MILK! Turn gerettet {adjustment:+d}" if current < 0 else f"MILK! Turn verdoppelt +{adjustment}"
            outcome = ThrowOutcome(adjustment, message)
            return finish_round_game(state, outcome, "{winner} gewinnt die Keksdose!")

        hit_id = str(event.get("label", "")).upper()
        if hit_id == "SBULL":
            hit_id = "SBULL"
        elif hit_id == "DBULL":
            hit_id = "DBULL"
        cookie = state.mode_state.get("cookies", {}).get(hit_id)
        if event.get("type") != "hit" or not cookie:
            self._reset_streak(state, player)
            outcome = ThrowOutcome(0, "Keine Krümel")
        elif cookie["kind"] == "moldy":
            self._reset_streak(state, player)
            player.score -= 30
            outcome = ThrowOutcome(-30, "MOLDY COOKIE! -30")
        else:
            base = int(COOKIE_POINTS[cookie["kind"]])
            charged = bool(state.mode_state["sugar"].get(player.id, False))
            points = base * (2 if charged else 1)
            state.mode_state["sugar"][player.id] = False
            streak = int(state.mode_state["streak"].get(player.id, 0)) + 1
            if streak >= 3:
                state.mode_state["sugar"][player.id] = True
                streak = 0
            state.mode_state["streak"][player.id] = streak
            player.score += points
            suffix = " · SUGAR RUSH GELADEN!" if state.mode_state["sugar"][player.id] else ""
            outcome = ThrowOutcome(points, f"{cookie['kind'].upper()} COOKIE +{points}{suffix}")
        return finish_round_game(state, outcome, "{winner} gewinnt die Keksdose!")

    def get_overlay(self, state: Any) -> Dict[str, Any]:
        groups: Dict[str, List[Dict[str, Any]]] = {
            "gold": [], "blue": [], "green": [], "moldy": []
        }
        for item in state.mode_state.get("cookies", {}).values():
            groups[item["kind"]].append(item["dart"])
        current = state.current_player()
        sugar = bool(state.mode_state.get("sugar", {}).get(current.id if current else "", False))
        streak = int(state.mode_state.get("streak", {}).get(current.id if current else "", 0))
        milk = [
            {
                **overlay_item(dart, "#8fd3ff", "MILCH" if index == 0 else "", True),
                "icon": "milk",
                "variant": "milk",
            }
            for index, dart in enumerate(MILK_BULLS)
        ]
        return {
            "prompt": "COOKIE-FELDER TREFFEN · BULL = MILCH",
            "bonus": [self._cookie_overlay(d, "gold", "+50") for d in groups["gold"]] + milk,
            "targets": [self._cookie_overlay(d, "blue", "+25") for d in groups["blue"]]
                + [self._cookie_overlay(d, "green", "+10") for d in groups["green"]],
            "danger": [self._cookie_overlay(d, "moldy", "-30") for d in groups["moldy"]],
            "visual_legend": [
                {"icon": "cookie", "color": COOKIE_COLORS["gold"], "label": "Gold-Cookie", "value": "+50"},
                {"icon": "cookie", "color": COOKIE_COLORS["blue"], "label": "Blauer Cookie", "value": "+25"},
                {"icon": "cookie", "color": COOKIE_COLORS["green"], "label": "Grüner Cookie", "value": "+10"},
                {"icon": "cookie_moldy", "color": COOKIE_COLORS["moldy"], "label": "Schimmel", "value": "-30"},
                {"icon": "milk", "color": "#8fd3ff", "label": "Bull-Milch", "value": "Turn ×2 / retten"},
            ],
            "panel": {
                "title": "COOKIE COMBO",
                "headline": "SUGAR RUSH BEREIT!" if sugar else f"Serie {streak}/3",
                "subline": "Der nächste gute Cookie zählt doppelt" if sugar else "Drei gute Cookies laden den Rush",
                "progress": {"value": 3 if sugar else streak, "max": 3},
            },
        }


GAME_MODE = CookieMonsterMode()
