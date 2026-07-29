from __future__ import annotations

import random
from typing import Any, Dict, List

from .arcade import DARTS, overlay_item
from .base import GameMetadata, GameOption, InstructionStep, ThrowOutcome

TASK_POOL = [
    {"id":"double", "label":"Any Double", "accept":lambda e: e.get("multiplier")==2},
    {"id":"triple", "label":"Any Triple", "accept":lambda e: e.get("multiplier")==3},
    {"id":"bull", "label":"Bull", "accept":lambda e: int(e.get("field",0) or 0)==25},
    {"id":"even", "label":"Even", "accept":lambda e: int(e.get("field",0) or 0)%2==0 and int(e.get("field",0) or 0)>0},
    {"id":"odd", "label":"Odd", "accept":lambda e: int(e.get("field",0) or 0)%2==1 and int(e.get("field",0) or 0)<25},
    {"id":"high", "label":"16+", "accept":lambda e: 16 <= int(e.get("field",0) or 0) <= 20},
    {"id":"low", "label":"1-5", "accept":lambda e: 1 <= int(e.get("field",0) or 0) <= 5},
]
for n in [20,19,18,17,16,15,10,5,1]:
    TASK_POOL.append({"id":f"field_{n}", "label":f"Any {n}", "field":n, "accept":lambda e,n=n: int(e.get("field",0) or 0)==n})


class DartsBingoMode:
    metadata = GameMetadata(
        slug="darts_bingo",
        title="Darts Bingo",
        tagline="Aufgaben markieren, Linie holen",
        description="Jeder Spieler hat eine 3x3 Bingo-Karte aus Dartaufgaben. Als Siegziel wählt ihr eine Linie oder die volle Karte.",
        accent="#ffcf33",
        accent_secondary="#9b5cff",
        visual="darts-bingo",
        icon="grid",
        options=[GameOption("points", "Sieg", "choice", "line", [{"value":"line","label":"Erste Linie"},{"value":"full","label":"Volle Karte"}])],
        instructions=[
            InstructionStep("Karte füllen", "Jeder Treffer kann eine Aufgabe markieren.", "grid"),
            InstructionStep("Siegziel beachten", "Je nach Auswahl zählt die erste Linie oder die volle Karte.", "line"),
            InstructionStep("Jeder hat eigene Karte", "Aufgaben sind pro Spieler individuell.", "cards"),
        ],
        sound_theme="arcade",
    )

    def initialize_player(self, player: Any, options: Dict[str, Any]) -> None:
        player.score = 0
        # Gameplay variety only; not used for a security decision.
        tasks = random.sample(TASK_POOL, 9)  # nosec B311
        player.marks = {str(i): {"task": tasks[i]["id"], "label": tasks[i]["label"], "done": False} for i in range(9)}

    def _task_by_id(self, task_id: str) -> Dict[str, Any]:
        return next(t for t in TASK_POOL if t["id"] == task_id)

    def _has_line(self, player: Any) -> bool:
        done = [bool(player.marks[str(i)]["done"]) for i in range(9)]
        lines = [(0,1,2),(3,4,5),(6,7,8),(0,3,6),(1,4,7),(2,5,8),(0,4,8),(2,4,6)]
        return any(all(done[i] for i in line) for line in lines)

    def apply_throw(self, state: Any, player: Any, event: Dict[str, Any]) -> ThrowOutcome:
        if event.get("type") != "hit":
            return ThrowOutcome(turn_value=0, message="Miss – kein Bingo")
        marked = []
        for idx, cell in player.marks.items():
            if cell["done"]:
                continue
            task = self._task_by_id(cell["task"])
            if task["accept"](event):
                cell["done"] = True
                marked.append(cell["label"])
                player.score += 1
        if not marked:
            return ThrowOutcome(turn_value=0, message="Keine Bingo-Aufgabe getroffen")
        full_card = all(cell["done"] for cell in player.marks.values())
        target_reached = (
            full_card
            if state.options.get("points", "line") == "full"
            else self._has_line(player)
        )
        if target_reached:
            return ThrowOutcome(turn_value=len(marked), message=f"{player.name} ruft BINGO!", finished=True, winner_id=player.id)
        return ThrowOutcome(turn_value=len(marked), message=f"Bingo markiert: {' · '.join(marked)}")

    def get_overlay(self, state: Any) -> Dict[str, Any]:
        player = state.current_player()
        if not player:
            return {"prompt":"Darts Bingo"}
        labels = [cell["label"] for cell in player.marks.values() if not cell["done"]]
        return {
            "prompt": "Bingo: " + " · ".join(labels[:4]),
            "card": [
                {
                    "index": int(index),
                    "label": cell["label"],
                    "done": bool(cell["done"]),
                }
                for index, cell in player.marks.items()
            ],
        }


GAME_MODE = DartsBingoMode()
