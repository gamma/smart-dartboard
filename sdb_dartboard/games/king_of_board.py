from __future__ import annotations

from typing import Any, Dict

from .arcade import finish_round_game
from .base import GameMetadata, GameOption, InstructionStep, ThrowOutcome

BOARD_ORDER = [20, 1, 18, 4, 13, 6, 10, 15, 2, 17, 3, 19, 7, 16, 8, 11, 14, 9, 12, 5]
NUMBER_RINGS = ["single_inner", "triple", "single_outer", "double"]


class KingOfBoardMode:
    metadata = GameMetadata(
        slug="king_of_board",
        title="King of the Board",
        tagline="Erobere die Scheibe",
        description="Jeder Treffer übernimmt ein Feld in deiner Farbe. Nach den Runden gewinnt die größte Herrschaft.",
        accent="#9b5cff",
        accent_secondary="#28e7ff",
        visual="king-board",
        icon="flag",
        options=[
            GameOption("rounds", "Runden", "choice", 5, [{"value":3,"label":"3 Runden"},{"value":5,"label":"5 Runden"},{"value":8,"label":"8 Runden"}]),
            GameOption("ownership", "Eroberung", "choice", "segment", [
                {"value": "area", "label": "Leicht · Double-Reihe, Triple-Nachbarn"},
                {"value": "segment", "label": "Klassisch · Segment genau"},
                {"value": "field", "label": "Sehr leicht · Ganzes Zahlenfeld"},
            ]),
        ],
        instructions=[
            InstructionStep("Felder erobern", "Treffer übernehmen Felder in deiner Farbe.", "capture"),
            InstructionStep("Leichte Ring-Power", "Double nimmt die ganze Zahl. Triple nimmt zusätzlich beide Nachbarzahlen.", "power"),
            InstructionStep("Zurückstehlen", "Triff gegnerische Felder, um sie zu übernehmen.", "steal"),
            InstructionStep("Mehrheit gewinnt", "Nach den Runden gewinnt die größte Board-Kontrolle.", "crown"),
        ],
        sound_theme="arcade",
    )

    def initialize_player(self, player: Any, options: Dict[str, Any]) -> None:
        player.score = 0
        player.marks = {}

    def initialize_state(self, state: Any, options: Dict[str, Any]) -> None:
        state.mode_state = {"owned": {}}
        state.message = "Erobere die Scheibe!"

    def _neighbor_fields(self, field: int) -> list[int]:
        index = BOARD_ORDER.index(field)
        return [
            BOARD_ORDER[(index - 1) % len(BOARD_ORDER)],
            field,
            BOARD_ORDER[(index + 1) % len(BOARD_ORDER)],
        ]

    def _capture_cells(
        self,
        state: Any,
        event: Dict[str, Any],
    ) -> list[tuple[int, str]]:
        field = int(event.get("field", 0) or 0)
        if field == 25:
            return [(field, str(event.get("ring") or "single_bull"))]
        ring = str(event.get("ring") or "single_outer")
        ownership = state.options.get("ownership", "segment")
        if ownership == "field":
            return [(field, owned_ring) for owned_ring in NUMBER_RINGS]
        if ownership == "area" and ring == "double":
            return [(field, owned_ring) for owned_ring in NUMBER_RINGS]
        if ownership == "area" and ring == "triple":
            return [
                (owned_field, owned_ring)
                for owned_field in self._neighbor_fields(field)
                for owned_ring in NUMBER_RINGS
            ]
        return [(field, ring)]

    def _capture_label(self, state: Any, event: Dict[str, Any]) -> str:
        field = int(event.get("field", 0) or 0)
        ring = str(event.get("ring") or "")
        if field == 25:
            return str(event.get("label") or "Bull")
        ownership = state.options.get("ownership", "segment")
        if ownership == "area" and ring == "triple":
            neighbors = self._neighbor_fields(field)
            return f"{neighbors[0]} · {field} · {neighbors[2]}"
        if ownership in {"area", "field"} and (
            ownership == "field" or ring == "double"
        ):
            return f"ganze {field}"
        return str(event.get("label") or field)

    def _recalculate_scores(self, state: Any) -> None:
        counts = {player.id: 0 for player in state.players}
        for item in state.mode_state.get("owned", {}).values():
            owner = item.get("owner_id")
            if owner in counts:
                counts[owner] += 1
        for player in state.players:
            player.score = counts.get(player.id, 0)

    def apply_throw(self, state: Any, player: Any, event: Dict[str, Any]) -> ThrowOutcome:
        if event.get("type") != "hit":
            outcome = ThrowOutcome(turn_value=0, message="Miss – kein Gebiet")
        else:
            cells = self._capture_cells(state, event)
            owned = state.mode_state.setdefault("owned", {})
            old_owners = {
                owned.get(f"{field}:{ring}", {}).get("owner_id")
                for field, ring in cells
            }
            before_score = player.score
            for field, ring in cells:
                cell_id = f"{field}:{ring}"
                owned[cell_id] = {
                    "owner_id": player.id,
                    "color": player.color,
                    "label": cell_id,
                    "field": field,
                    "ring": ring,
                }
            self._recalculate_scores(state)
            score_change = player.score - before_score
            action = (
                "hält"
                if old_owners == {player.id}
                else "übernimmt"
                if old_owners <= {None, player.id}
                else "erobert"
            )
            outcome = ThrowOutcome(
                turn_value=score_change,
                message=(
                    f"{player.name} {action} {self._capture_label(state, event)}"
                    f" · {len(cells)} Gebiet{'e' if len(cells) != 1 else ''}"
                ),
            )
        return finish_round_game(
            state, outcome, "{winner} regiert die Scheibe!"
        )

    def get_overlay(self, state: Any) -> Dict[str, Any]:
        owned = []
        for item in state.mode_state.get("owned", {}).values():
            field = item.get("field")
            ring = item.get("ring")
            if not field or not ring:
                continue
            owned.append({
                "id": str(item.get("label")),
                "field": field,
                "ring": ring,
                "color": item.get("color", "#28e7ff"),
                "owner_id": item.get("owner_id"),
            })
        prompt = (
            "SINGLE = SEGMENT · DOUBLE = GANZE ZAHL · TRIPLE = ZAHL + NACHBARN"
            if state.options.get("ownership") == "area"
            else "EROBERE DIE SCHEIBE!"
        )
        return {"prompt": prompt, "owned": owned}


GAME_MODE = KingOfBoardMode()
