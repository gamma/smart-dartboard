from __future__ import annotations

import random
from typing import Any, Dict, List

from .base import GameMetadata, GameOption, InstructionStep, ThrowOutcome

BOARD_ORDER = [20, 1, 18, 4, 13, 6, 10, 15, 2, 17, 3, 19, 7, 16, 8, 11, 14, 9, 12, 5]
PRESETS = {
    "explorer": {"mines": 3, "lives": 5},
    "classic": {"mines": 5, "lives": 3},
    "expert": {"mines": 7, "lives": 2},
}
ALL_RINGS = ["single_inner", "triple", "single_outer", "double"]


class DartSweeperMode:
    metadata = GameMetadata(
        slug="dart_sweeper",
        title="DartSweeper",
        tagline="Räumt gemeinsam das Minenfeld",
        description="Die 20 Zahlen werden zu Minesweeper-Feldern. Double, Triple und Bull decken zusätzliche sichere Zahlen auf.",
        accent="#5f8f71",
        accent_secondary="#e9c46a",
        visual="dart-sweeper",
        icon="mine",
        options=[
            GameOption("preset", "Schwierigkeit", "choice", "classic", [
                {"value": "explorer", "label": "Explorer · 3 Minen / 5 Leben", "description": "Wenige Minen und fünf gemeinsame Fehler – gut zum Kennenlernen.", "description_en": "Few mines and five shared mistakes—best for learning."},
                {"value": "classic", "label": "Classic · 5 Minen / 3 Leben", "description": "Ausgewogenes Minenfeld mit drei gemeinsamen Leben.", "description_en": "A balanced minefield with three shared lives."},
                {"value": "expert", "label": "Expert · 7 Minen / 2 Leben", "description": "Viele Minen und nur zwei gemeinsame Fehler.", "description_en": "Many mines and only two shared mistakes."},
            ]),
        ],
        instructions=[
            InstructionStep("Zahl aufdecken", "Singles decken genau die getroffene Zahl auf.", "reveal"),
            InstructionStep("Ring-Power", "Double deckt einen, Triple zwei sichere Nachbarn zusätzlich auf.", "power"),
            InstructionStep("Gemeinsam räumen", "Bull hilft beim Scannen. Räumt alle sicheren Zahlen vor dem letzten Leben.", "mine"),
        ],
        sound_theme="arcade",
    )

    def initialize_player(self, player: Any, options: Dict[str, Any]) -> None:
        player.score = 0
        player.marks = {}

    def initialize_state(self, state: Any, options: Dict[str, Any]) -> None:
        preset = PRESETS[str(options.get("preset", "classic"))]
        state.mode_state = {
            "seed": random.randint(0, 2**31 - 1),  # nosec B311
            "seeded": False,
            "mines": [],
            "revealed": {},
            "exploded": [],
            "lives": preset["lives"],
            "max_lives": preset["lives"],
            "mine_count": preset["mines"],
        }
        state.message = "Der erste Treffer ist garantiert sicher!"

    def _neighbors(self, field: int) -> List[int]:
        index = BOARD_ORDER.index(field)
        return [
            BOARD_ORDER[(index - 1) % 20],
            BOARD_ORDER[(index + 1) % 20],
            BOARD_ORDER[(index - 2) % 20],
            BOARD_ORDER[(index + 2) % 20],
        ]

    def _seed(self, state: Any, safe_field: int | None) -> None:
        excluded = set()
        if safe_field is not None:
            immediate = self._neighbors(safe_field)[:2]
            excluded = {safe_field, *immediate}
        available = [field for field in BOARD_ORDER if field not in excluded]
        rng = random.Random(int(state.mode_state["seed"]))  # nosec B311
        state.mode_state["mines"] = sorted(
            rng.sample(available, int(state.mode_state["mine_count"]))
        )
        state.mode_state["seeded"] = True

    def _count(self, state: Any, field: int) -> int:
        mines = set(state.mode_state["mines"])
        return sum(1 for neighbor in self._neighbors(field) if neighbor in mines)

    def _safe_covered(self, state: Any, fields: List[int] | None = None) -> List[int]:
        mines = set(state.mode_state["mines"])
        revealed = {int(field) for field in state.mode_state["revealed"]}
        candidates = fields or BOARD_ORDER
        return [
            field for field in candidates
            if field not in mines and field not in revealed
        ]

    def _reveal(self, state: Any, field: int) -> int:
        if str(field) in state.mode_state["revealed"]:
            return 0
        count = self._count(state, field)
        state.mode_state["revealed"][str(field)] = count
        return count

    def _finish_if_needed(
        self,
        state: Any,
        points: int,
        message: str,
    ) -> ThrowOutcome:
        safe_total = 20 - len(state.mode_state["mines"])
        revealed = len(state.mode_state["revealed"])
        if revealed >= safe_total:
            return ThrowOutcome(
                points,
                "MINENFELD GERÄUMT! Das Team gewinnt!",
                finished=True,
                winner_ids=[player.id for player in state.players],
                result_type="team_win",
            )
        if int(state.mode_state["lives"]) <= 0:
            return ThrowOutcome(
                points,
                "BOOM! Keine Leben mehr · Team-Niederlage",
                finished=True,
                result_type="challenge_loss",
            )
        return ThrowOutcome(points, message)

    def apply_throw(self, state: Any, player: Any, event: Dict[str, Any]) -> ThrowOutcome:
        is_hit = event.get("type") == "hit"
        field = int(event.get("field", 0) or 0)
        is_bull = is_hit and field == 25
        if not state.mode_state["seeded"]:
            self._seed(state, None if is_bull or not is_hit else field)

        if not is_hit:
            return self._finish_if_needed(state, 0, "MISS · Das Minenfeld bleibt verdeckt")

        if is_bull:
            amount = 2 if int(event.get("multiplier", 1)) == 2 else 1
            safe = self._safe_covered(state)
            rng = random.Random(  # nosec B311
                f"{state.mode_state['seed']}:{len(state.mode_state['revealed'])}:bull"
            )
            chosen = rng.sample(safe, min(amount, len(safe)))
            for target in chosen:
                self._reveal(state, target)
            points = len(chosen) * 5
            player.score += points
            return self._finish_if_needed(
                state,
                points,
                f"BULL-SCANNER! {len(chosen)} sichere Felder +{points}",
            )

        mines = set(state.mode_state["mines"])
        if field in mines:
            if field not in state.mode_state["exploded"]:
                state.mode_state["exploded"].append(field)
                state.mode_state["lives"] = max(0, int(state.mode_state["lives"]) - 1)
                event["effect"] = "mine_explosion"
                return self._finish_if_needed(
                    state,
                    0,
                    f"BOOM auf {field}! Noch {state.mode_state['lives']} Leben",
                )
            return self._finish_if_needed(state, 0, f"Mine {field} ist bereits bekannt")

        if str(field) in state.mode_state["revealed"]:
            return self._finish_if_needed(state, 0, f"{field} ist bereits aufgedeckt")

        count = self._reveal(state, field)
        points = 20 if count == 0 else 10
        bonus_count = 0
        multiplier = int(event.get("multiplier", 1))
        bonus_wanted = 2 if multiplier == 3 else 1 if multiplier == 2 else 0
        for neighbor in self._safe_covered(state, self._neighbors(field))[:bonus_wanted]:
            self._reveal(state, neighbor)
            bonus_count += 1
        points += bonus_count * 5
        player.score += points
        return self._finish_if_needed(
            state,
            points,
            f"{field} zeigt {count} · {bonus_count} Bonusfeld{'er' if bonus_count != 1 else ''} +{points}",
        )

    def get_overlay(self, state: Any) -> Dict[str, Any]:
        revealed = {
            int(field): int(count)
            for field, count in state.mode_state.get("revealed", {}).items()
        }
        exploded = set(state.mode_state.get("exploded", []))
        zones = []
        for field in BOARD_ORDER:
            if field in exploded:
                zones.append({
                    "field": field,
                    "rings": ALL_RINGS,
                    "role": "mine",
                    "label": "",
                    "icon": "mine",
                    "variant": "mine",
                    "color": "#e76f51",
                })
            elif field in revealed:
                count = revealed[field]
                color = "#70b77e" if count <= 1 else "#e9c46a" if count <= 2 else "#e76f51"
                zones.append({
                    "field": field,
                    "rings": ALL_RINGS,
                    "role": "revealed",
                    "color": color,
                    "label": str(count),
                })
            else:
                zones.append({
                    "field": field,
                    "rings": ALL_RINGS,
                    "role": "covered",
                    "label": "?",
                })
        safe_total = 20 - int(state.mode_state.get("mine_count", 0))
        safe_remaining = max(0, safe_total - len(revealed))
        lives = int(state.mode_state.get("lives", 0))
        maximum = int(state.mode_state.get("max_lives", lives))
        return {
            "prompt": "Single 1 Feld · Double +1 · Triple +2",
            "zones": zones,
            "panel": {
                "title": "DARTSWEEPER",
                "headline": f"{safe_remaining} sichere Felder übrig",
                "subline": "♥" * lives + "♡" * (maximum - lives),
                "progress": {"value": len(revealed), "max": safe_total},
                "stats": [
                    {"label": "MINEN", "value": state.mode_state.get("mine_count", 0)},
                    {"label": "GEFUNDEN", "value": len(exploded)},
                ],
            },
        }


GAME_MODE = DartSweeperMode()
