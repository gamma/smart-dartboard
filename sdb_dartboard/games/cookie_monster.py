from __future__ import annotations

from typing import Any, Dict, List

from .arcade import TARGET_POOL_NORMAL, finish_round_game, overlay_item
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
        tagline="Keksdose leer essen",
        description="Räume dein persönliches Cookie-Board ab, meide Schimmel und schalte erst dann die nächste Keksdose frei.",
        accent="#e9a23b",
        accent_secondary="#68b0ab",
        visual="cookie-monster",
        icon="cookie",
        options=[
            GameOption("difficulty", "Spielstufe", "choice", "easy", [
                {"value": "easy", "label": "Einfach · Snack Time", "description": "15 große Zahlenfelder; jeder Ring isst den Cookie. Bull gibt +30.", "description_en": "15 large number areas; any ring eats the cookie. Bull scores +30."},
                {"value": "normal", "label": "Mittel · Cookie Hunt", "description": "12 exakte Cookies mit Gold und Schimmel. Bull verdoppelt oder rettet den Zug.", "description_en": "12 exact cookies with gold and mold. Bull doubles or saves the visit."},
                {"value": "hard", "label": "Schwer · Sugar Rush", "description": "12 exakte, farbige Cookies; Serien aktivieren Sugar Rush. Bull verdoppelt oder rettet.", "description_en": "12 exact colored cookies; streaks trigger Sugar Rush. Bull doubles or saves."},
            ]),
            GameOption("rounds", "Runden", "choice", 5, [
                {"value": 5, "label": "5 Runden"},
                {"value": 8, "label": "8 Runden"},
            ]),
        ],
        instructions=[
            InstructionStep("Board leer essen", "Getroffene Cookies verschwinden für dich. Erst wenn alle weg sind, kommt ein neues Board.", "cookie"),
            InstructionStep("Schimmel meiden", "Schimmel kostet Punkte, muss aber nicht abgeräumt werden.", "danger"),
            InstructionStep("Bull ist Milch", "Easy gibt feste Bonuspunkte. Ab Mittel verdoppelt oder rettet Milch deinen Zug.", "milk"),
            InstructionStep("Stufe wählen", "Easy nutzt große Zahlenfelder; Mittel ergänzt Gold; Schwer ergänzt Farben und Sugar Rush.", "combo"),
        ],
        sound_theme="arcade",
        ruleset_version=2,
    )

    def initialize_player(self, player: Any, options: Dict[str, Any]) -> None:
        player.score = 0
        player.marks = {}

    def initialize_state(self, state: Any, options: Dict[str, Any]) -> None:
        state.mode_state = {
            "streak": {player.id: 0 for player in state.players},
            "sugar": {player.id: False for player in state.players},
            "wave": {player.id: 1 for player in state.players},
            "collected": {player.id: [] for player in state.players},
            "layouts": {},
            "last_effect": "",
            "effect_points": 0,
            "cookie_wave": 1,
        }
        self._layout(state, 1)

    def _layout(self, state: Any, wave: int) -> Dict[str, Dict[str, Any]]:
        layout_key = str(wave)
        layouts = state.mode_state.setdefault("layouts", {})
        if layout_key in layouts:
            return layouts[layout_key]
        difficulty = state.options.get("difficulty", "easy")
        if difficulty == "easy":
            available_fields = list(range(1, 21))
            chosen_fields = [
                available_fields.pop(state.random_index(len(available_fields)))
                for _ in range(15)
            ]
            kinds = ["blue"] * 12 + ["moldy"] * 3
            for index in range(len(kinds) - 1, 0, -1):
                swap_index = state.random_index(index + 1)
                kinds[index], kinds[swap_index] = kinds[swap_index], kinds[index]
            layout = {
                f"F{field}": {
                    "dart": {
                        "label": f"S{field}",
                        "field": field,
                        "ring": "single_outer",
                        "multiplier": 1,
                        "score": field,
                    },
                    "kind": kind,
                }
                for field, kind in zip(chosen_fields, kinds)
            }
            layouts[layout_key] = layout
            return layout

        pool = [dart for dart in TARGET_POOL_NORMAL if int(dart["field"]) != 25]
        chosen = [pool.pop(state.random_index(len(pool))) for _ in range(12)]
        kinds = (
            ["gold"] * 2 + ["blue"] * 7 + ["moldy"] * 3
            if difficulty == "normal"
            else ["gold"] * 2 + ["blue"] * 3 + ["green"] * 4 + ["moldy"] * 3
        )
        for index in range(len(kinds) - 1, 0, -1):
            swap_index = state.random_index(index + 1)
            kinds[index], kinds[swap_index] = kinds[swap_index], kinds[index]
        layout = {
            str(dart["label"]).upper(): {"dart": dart, "kind": kind}
            for dart, kind in zip(chosen, kinds)
        }
        layouts[layout_key] = layout
        return layout

    def on_turn_start(self, state: Any, player: Any) -> None:
        wave = int(state.mode_state.get("wave", {}).get(player.id, 1))
        self._layout(state, wave)

    def _reset_streak(self, state: Any, player: Any) -> None:
        state.mode_state["streak"][player.id] = 0

    def _cookie_overlay(
        self,
        dart: Dict[str, Any],
        kind: str,
        label: str,
        *,
        whole_field: bool = False,
    ) -> Dict[str, Any]:
        item = overlay_item(dart, COOKIE_COLORS[kind], label, kind == "moldy")
        item.update({
            "icon": "cookie_moldy" if kind == "moldy" else "cookie",
            "variant": kind,
            "match_field": whole_field,
        })
        return item

    def _current_board(
        self,
        state: Any,
        player: Any,
    ) -> tuple[int, Dict[str, Dict[str, Any]], set[str]]:
        wave = int(state.mode_state.get("wave", {}).get(player.id, 1))
        board = self._layout(state, wave)
        collected = set(
            state.mode_state.setdefault("collected", {}).setdefault(player.id, [])
        )
        return wave, board, collected

    def _event_cookie_id(self, state: Any, event: Dict[str, Any]) -> str:
        if state.options.get("difficulty", "easy") == "easy":
            return f"F{int(event.get('field', 0) or 0)}"
        return str(event.get("label", "")).upper()

    def apply_throw(self, state: Any, player: Any, event: Dict[str, Any]) -> ThrowOutcome:
        state.mode_state.update({"last_effect": "", "effect_points": 0})
        difficulty = state.options.get("difficulty", "easy")
        is_bull = event.get("type") == "hit" and int(event.get("field", 0)) == 25
        if is_bull:
            event["effect"] = "cookie_milk"
            if difficulty == "easy":
                player.score += 30
                event["cookie_points"] = 30
                state.mode_state.update({"last_effect": "cookie_milk", "effect_points": 30})
                outcome = ThrowOutcome(30, "MILCH! +30")
                return finish_round_game(
                    state, outcome, "{winner} gewinnt die Keksdose!"
                )
            current = int(state.turn_score)
            adjustment = current if current > 0 else -current if current < 0 else 0
            player.score += adjustment
            self._reset_streak(state, player)
            state.mode_state.update({"last_effect": "cookie_milk", "effect_points": adjustment})
            message = f"MILK! Turn gerettet {adjustment:+d}" if current < 0 else f"MILK! Turn verdoppelt +{adjustment}"
            outcome = ThrowOutcome(adjustment, message)
            return finish_round_game(state, outcome, "{winner} gewinnt die Keksdose!")

        wave, board, collected = self._current_board(state, player)
        hit_id = self._event_cookie_id(state, event)
        cookie = board.get(hit_id)
        if event.get("type") != "hit" or not cookie:
            self._reset_streak(state, player)
            outcome = ThrowOutcome(0, "Keine Krümel")
        elif hit_id in collected:
            self._reset_streak(state, player)
            outcome = ThrowOutcome(0, "Hier ist schon alles aufgegessen")
        elif cookie["kind"] == "moldy":
            self._reset_streak(state, player)
            penalty = {"easy": 20, "normal": 25, "hard": 30}.get(
                difficulty, 30
            )
            player.score -= penalty
            event["effect"] = "cookie_moldy"
            state.mode_state.update({"last_effect": "cookie_moldy", "effect_points": -penalty})
            outcome = ThrowOutcome(-penalty, f"SCHIMMEL! -{penalty}")
        else:
            base = (
                20
                if difficulty == "easy"
                else 50 if cookie["kind"] == "gold"
                else 20 if difficulty == "normal"
                else int(COOKIE_POINTS[cookie["kind"]])
            )
            charged = (
                difficulty == "hard"
                and bool(state.mode_state["sugar"].get(player.id, False))
            )
            points = base * (2 if charged else 1)
            state.mode_state["sugar"][player.id] = False
            streak = (
                int(state.mode_state["streak"].get(player.id, 0)) + 1
                if difficulty == "hard"
                else 0
            )
            if difficulty == "hard" and streak >= 3:
                state.mode_state["sugar"][player.id] = True
                streak = 0
            state.mode_state["streak"][player.id] = streak
            state.mode_state["collected"][player.id].append(hit_id)
            player.score += points
            event["cookie_points"] = points
            remaining = [
                cookie_id
                for cookie_id, item in board.items()
                if item["kind"] != "moldy"
                and cookie_id not in state.mode_state["collected"][player.id]
            ]
            if not remaining:
                next_wave = wave + 1
                state.mode_state["wave"][player.id] = next_wave
                state.mode_state["collected"][player.id] = []
                self._layout(state, next_wave)
                event["effect"] = "cookie_board_clear"
                event["cookie_wave"] = next_wave
                state.mode_state.update({"last_effect": "cookie_board_clear", "effect_points": points, "cookie_wave": next_wave})
                outcome = ThrowOutcome(
                    points,
                    f"BOARD GEPUTZT! +{points} · Neue Cookies!",
                )
            else:
                event["effect"] = "cookie_eaten"
                state.mode_state.update({"last_effect": "cookie_eaten", "effect_points": points, "cookie_wave": wave})
                suffix = (
                    " · SUGAR RUSH GELADEN!"
                    if state.mode_state["sugar"][player.id]
                    else ""
                )
                outcome = ThrowOutcome(
                    points,
                    f"{cookie['kind'].upper()} COOKIE +{points}{suffix}",
                )
        return finish_round_game(state, outcome, "{winner} gewinnt die Keksdose!")

    def get_overlay(self, state: Any) -> Dict[str, Any]:
        current = state.current_player()
        if not current:
            return {}
        difficulty = state.options.get("difficulty", "easy")
        wave, board, collected = self._current_board(state, current)
        groups: Dict[str, List[Dict[str, Any]]] = {
            "gold": [], "blue": [], "green": [], "moldy": []
        }
        for cookie_id, item in board.items():
            if cookie_id in collected:
                continue
            groups[item["kind"]].append(item["dart"])
        sugar = bool(state.mode_state.get("sugar", {}).get(current.id, False))
        streak = int(state.mode_state.get("streak", {}).get(current.id, 0))
        milk = [
            {
                **overlay_item(dart, "#8fd3ff", "MILCH" if index == 0 else "", True),
                "icon": "milk",
                "variant": "milk",
            }
            for index, dart in enumerate(MILK_BULLS)
        ]
        whole_field = difficulty == "easy"
        good_items = (
            [self._cookie_overlay(d, "gold", "+50") for d in groups["gold"]]
            + [
                self._cookie_overlay(
                    d,
                    "blue",
                    "+20" if difficulty != "hard" else "+25",
                    whole_field=whole_field,
                )
                for d in groups["blue"]
            ]
            + [self._cookie_overlay(d, "green", "+10") for d in groups["green"]]
        )
        mold_penalty = {"easy": 20, "normal": 25, "hard": 30}.get(
            difficulty, 30
        )
        danger = [
            self._cookie_overlay(
                d,
                "moldy",
                f"-{mold_penalty}",
                whole_field=whole_field,
            )
            for d in groups["moldy"]
        ]
        remaining = len(good_items)
        zones = []
        if whole_field:
            for item in good_items + danger:
                zones.append({
                    "field": item["field"],
                    "rings": [
                        "single_inner", "triple", "single_outer", "double",
                    ],
                    "role": "control",
                    "color": item["color"],
                })
        legend = [
            {
                "icon": "cookie",
                "color": COOKIE_COLORS["blue"],
                "label": "Cookie",
                "value": "+20" if difficulty != "hard" else "+25",
            },
        ]
        if difficulty != "easy":
            legend.insert(0, {
                "icon": "cookie",
                "color": COOKIE_COLORS["gold"],
                "label": "Gold-Cookie",
                "value": "+50",
            })
        if difficulty == "hard":
            legend.append({
                "icon": "cookie",
                "color": COOKIE_COLORS["green"],
                "label": "Grüner Cookie",
                "value": "+10",
            })
        legend.extend([
            {
                "icon": "cookie_moldy",
                "color": COOKIE_COLORS["moldy"],
                "label": "Schimmel",
                "value": f"-{mold_penalty}",
            },
            {
                "icon": "milk",
                "color": "#8fd3ff",
                "label": "Bull-Milch",
                "value": "+30" if difficulty == "easy" else "Turn ×2 / retten",
            },
        ])
        return {
            "prompt": "ALLE COOKIES ESSEN · SCHIMMEL MEIDEN · BULL = MILCH",
            "bonus": (
                [item for item in good_items if item["variant"] == "gold"]
                + milk
            ),
            "targets": [
                item for item in good_items if item["variant"] != "gold"
            ],
            "danger": danger,
            "zones": zones,
            "visual_legend": legend,
            "panel": {
                "title": f"COOKIE BOARD {wave}",
                "headline": f"{remaining} Cookies übrig",
                "subline": (
                    "SUGAR RUSH BEREIT · nächster Cookie doppelt"
                    if sugar
                    else f"Serie {streak}/3 · Board erst komplett leer essen"
                    if difficulty == "hard"
                    else "Board erst komplett leer essen"
                ),
                "progress": {
                    "value": len([
                        item for item in board.values()
                        if item["kind"] != "moldy"
                    ]) - remaining,
                    "max": len([
                        item for item in board.values()
                        if item["kind"] != "moldy"
                    ]),
                },
            },
        }


GAME_MODE = CookieMonsterMode()
