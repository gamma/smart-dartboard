from __future__ import annotations

import random
from typing import Any, Dict, List, Tuple

from .base import GameMetadata, InstructionStep, ThrowOutcome

WIDTH = 5
HEIGHT = 8
SHAPES: Dict[str, List[List[Tuple[int, int]]]] = {
    "I": [
        [(0, 0), (1, 0), (2, 0), (3, 0)],
        [(0, 0), (0, 1), (0, 2), (0, 3)],
    ],
    "O": [[(0, 0), (1, 0), (0, 1), (1, 1)]],
    "T": [
        [(0, 0), (1, 0), (2, 0), (1, 1)],
        [(1, 0), (0, 1), (1, 1), (1, 2)],
        [(1, 0), (0, 1), (1, 1), (2, 1)],
        [(0, 0), (0, 1), (1, 1), (0, 2)],
    ],
    "L": [
        [(0, 0), (0, 1), (0, 2), (1, 2)],
        [(0, 0), (1, 0), (2, 0), (0, 1)],
        [(0, 0), (1, 0), (1, 1), (1, 2)],
        [(2, 0), (0, 1), (1, 1), (2, 1)],
    ],
    "S": [
        [(1, 0), (2, 0), (0, 1), (1, 1)],
        [(0, 0), (0, 1), (1, 1), (1, 2)],
    ],
}
LINE_POINTS = {1: 50, 2: 120, 3: 250, 4: 500}
CONTROL_ZONES = {
    "rotate_left": [12, 5, 20, 1, 18],
    "right": [4, 13, 6, 10, 15],
    "rotate_right": [2, 17, 3, 19, 7],
    "left": [16, 8, 11, 14, 9],
}


class BlockDropMode:
    metadata = GameMetadata(
        slug="block_drop",
        title="Block Drop Darts",
        tagline="Gemeinsam fünf Linien bauen",
        description="Darts steuern einen fröhlichen Block-Puzzler. Alle Spieler bauen gemeinsam am selben 5×8-Feld.",
        accent="#e07a5f",
        accent_secondary="#81b29a",
        visual="block-drop",
        icon="blocks",
        instructions=[
            InstructionStep("Vier große Flächen", "Die vier Farbbögen bewegen links/rechts oder drehen links/rechts.", "controls"),
            InstructionStep("Bull ist Drop", "Single Bull und Double Bull setzen den Stein sofort. Double Bull gibt +25.", "power"),
            InstructionStep("Gemeinsamer Takt", "Erst nachdem alle gespielt haben, fällt der Stein automatisch eine Zeile.", "round"),
            InstructionStep("Fünf Linien", "Löscht gemeinsam fünf Linien, bevor ein Stein oben herausragt.", "blocks"),
        ],
        sound_theme="arcade",
    )

    def initialize_player(self, player: Any, options: Dict[str, Any]) -> None:
        player.score = 0
        player.marks = {}

    def initialize_state(self, state: Any, options: Dict[str, Any]) -> None:
        state.mode_state = {
            "board": [[0 for _ in range(WIDTH)] for _ in range(HEIGHT)],
            "lines": 0,
            "seed": random.randint(0, 2**31 - 1),  # nosec B311
            "piece_index": 0,
            "gravity_round": 1,
        }
        self._spawn(state)

    def _shape(self, piece: Dict[str, Any]) -> List[Tuple[int, int]]:
        rotations = SHAPES[piece["kind"]]
        return rotations[int(piece["rotation"]) % len(rotations)]

    def _cells(
        self,
        piece: Dict[str, Any],
        *,
        x: int | None = None,
        y: int | None = None,
        rotation: int | None = None,
    ) -> List[Tuple[int, int]]:
        candidate = {
            **piece,
            "x": piece["x"] if x is None else x,
            "y": piece["y"] if y is None else y,
            "rotation": piece["rotation"] if rotation is None else rotation,
        }
        return [
            (candidate["x"] + dx, candidate["y"] + dy)
            for dx, dy in self._shape(candidate)
        ]

    def _valid(self, state: Any, cells: List[Tuple[int, int]]) -> bool:
        board = state.mode_state["board"]
        return all(
            0 <= x < WIDTH
            and 0 <= y < HEIGHT
            and not board[y][x]
            for x, y in cells
        )

    def _spawn(self, state: Any) -> bool:
        index = int(state.mode_state.get("piece_index", 0))
        rng = random.Random(f"{state.mode_state['seed']}:{index}")  # nosec B311
        kind = rng.choice(sorted(SHAPES))
        width = max(x for x, _ in SHAPES[kind][0]) + 1
        piece = {"kind": kind, "rotation": 0, "x": (WIDTH - width) // 2, "y": 0}
        state.mode_state["piece"] = piece
        state.mode_state["piece_index"] = index + 1
        return self._valid(state, self._cells(piece))

    def _move(self, state: Any, dx: int) -> None:
        piece = state.mode_state["piece"]
        x = int(piece["x"]) + dx
        if self._valid(state, self._cells(piece, x=x)):
            piece["x"] = x

    def _rotate(self, state: Any, direction: int) -> None:
        piece = state.mode_state["piece"]
        rotation = (
            int(piece["rotation"]) + direction
        ) % len(SHAPES[piece["kind"]])
        for kick in (0, -1, 1, -2, 2):
            if self._valid(
                state,
                self._cells(piece, x=int(piece["x"]) + kick, rotation=rotation),
            ):
                piece["x"] = int(piece["x"]) + kick
                piece["rotation"] = rotation
                return

    def _soft_drop(self, state: Any) -> bool:
        piece = state.mode_state["piece"]
        y = int(piece["y"]) + 1
        if self._valid(state, self._cells(piece, y=y)):
            piece["y"] = y
            return True
        return False

    def _hard_drop(self, state: Any) -> None:
        while self._soft_drop(state):
            pass

    def _lock(self, state: Any) -> Tuple[int, bool]:
        board = state.mode_state["board"]
        for x, y in self._cells(state.mode_state["piece"]):
            board[y][x] = 1
        remaining = [row for row in board if not all(row)]
        cleared = HEIGHT - len(remaining)
        state.mode_state["board"] = [
            [0 for _ in range(WIDTH)] for _ in range(cleared)
        ] + remaining
        state.mode_state["lines"] += cleared
        can_continue = self._spawn(state)
        return cleared, can_continue

    def _finish(self, state: Any, won: bool, message: str, points: int) -> ThrowOutcome:
        return ThrowOutcome(
            points,
            message,
            finished=True,
            winner_ids=[player.id for player in state.players] if won else [],
            result_type="team_win" if won else "challenge_loss",
        )

    def _award_team(self, state: Any, points: int) -> None:
        for member in state.players:
            member.score += points

    def _finish_state(self, state: Any, won: bool, message: str) -> None:
        state.status = "finished"
        state.winner_id = None
        state.winner_ids = [player.id for player in state.players] if won else []
        state.result_type = "team_win" if won else "challenge_loss"
        state.message = message

    def _lock_piece(
        self,
        state: Any,
        *,
        power_bonus: int = 0,
    ) -> Tuple[int, int, bool]:
        cleared, can_continue = self._lock(state)
        points = 10 + LINE_POINTS.get(cleared, 0) + power_bonus
        self._award_team(state, points)
        return cleared, points, can_continue

    def on_turn_start(self, state: Any, player: Any) -> None:
        del player
        gravity_round = int(state.mode_state.get("gravity_round", 1))
        if state.round_number <= gravity_round:
            return
        state.mode_state["gravity_round"] = state.round_number
        if self._soft_drop(state):
            state.message = f"Runde {state.round_number}: Stein fällt eine Zeile"
            return

        cleared, points, can_continue = self._lock_piece(state)
        lines = int(state.mode_state["lines"])
        if lines >= 5:
            self._finish_state(
                state,
                True,
                f"FÜNF LINIEN! Das Team gewinnt mit {lines} Linien",
            )
        elif not can_continue:
            self._finish_state(state, False, "BLOCK OUT! Das Feld ist voll")
        else:
            detail = f" · {cleared} Linie{'n' if cleared != 1 else ''}!" if cleared else ""
            state.message = f"Rundendrop setzt den Stein · +{points}{detail}"

    def apply_throw(self, state: Any, player: Any, event: Dict[str, Any]) -> ThrowOutcome:
        force_lock = False
        power_bonus = 0
        if event.get("type") != "hit":
            action = "MISS · keine Aktion"
        elif int(event.get("field", 0)) == 25:
            self._hard_drop(state)
            force_lock = True
            if event.get("ring") == "double_bull":
                power_bonus = 25
                action = "DOUBLE BULL · POWER DROP!"
            else:
                action = "SINGLE BULL · DROP!"
        else:
            field = int(event.get("field", 0))
            if field in CONTROL_ZONES["left"]:
                self._move(state, -1)
                action = "LINKS"
            elif field in CONTROL_ZONES["right"]:
                self._move(state, 1)
                action = "RECHTS"
            elif field in CONTROL_ZONES["rotate_left"]:
                self._rotate(state, -1)
                action = "LINKS DREHEN"
            else:
                self._rotate(state, 1)
                action = "RECHTS DREHEN"

        if not force_lock:
            return ThrowOutcome(0, action)

        cleared, points, can_continue = self._lock_piece(
            state,
            power_bonus=power_bonus,
        )
        lines = int(state.mode_state["lines"])
        if lines >= 5:
            return self._finish(state, True, f"FÜNF LINIEN! Das Team gewinnt mit {lines} Linien", points)
        if not can_continue:
            return self._finish(state, False, "BLOCK OUT! Das Feld ist voll", points)
        detail = f" · {cleared} Linie{'n' if cleared != 1 else ''}!" if cleared else ""
        return ThrowOutcome(points, f"{action} · +{points}{detail}", force_hold=True)

    def get_overlay(self, state: Any) -> Dict[str, Any]:
        board = [list(row) for row in state.mode_state.get("board", [])]
        for x, y in self._cells(state.mode_state["piece"]):
            if 0 <= y < HEIGHT and 0 <= x < WIDTH:
                board[y][x] = 2
        cells = []
        for row in board:
            for value in row:
                cells.append({
                    "value": "",
                    "state": "active" if value == 2 else "filled" if value == 1 else "",
                })
        control_colors = {
            "left": "#e9c46a",
            "rotate_left": "#a77bff",
            "rotate_right": "#f4a261",
            "right": "#81b29a",
        }
        zones = []
        for action, color in control_colors.items():
            for field in CONTROL_ZONES[action]:
                zones.append({
                    "field": field,
                    "rings": ["single_inner", "triple", "single_outer", "double"],
                    "role": "control",
                    "color": color,
                })
        zones.extend([
            {
                "field": 25,
                "rings": ["single_bull"],
                "role": "control",
                "color": "#28e7ff",
            },
            {
                "field": 25,
                "rings": ["double_bull"],
                "role": "control",
                "color": "#e76f51",
            },
        ])
        lines = int(state.mode_state.get("lines", 0))
        return {
            "prompt": "GELB ← · LILA ↶ · ORANGE ↷ · GRÜN → · BULL DROP",
            "zones": zones,
            "panel": {
                "title": "BLOCK DROP",
                "headline": f"{lines}/5 Linien",
                "subline": "Alle bauen gemeinsam",
                "progress": {"value": lines, "max": 5},
                "grid": {"columns": WIDTH, "cells": cells},
            },
        }


GAME_MODE = BlockDropMode()
