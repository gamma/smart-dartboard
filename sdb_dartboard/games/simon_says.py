from __future__ import annotations

from typing import Any, Dict

from .arcade import finish_round_game
from .base import GameMetadata, GameOption, InstructionStep, ThrowOutcome

BOARD_ORDER = [20, 1, 18, 4, 13, 6, 10, 15, 2, 17, 3, 19, 7, 16, 8, 11, 14, 9, 12, 5]
NUMBER_RINGS = ["single_inner", "triple", "single_outer", "double"]
ZONE_COUNTS = {
    "very_easy": 4,
    "easy": 5,
    "normal": 10,
    "hard": 20,
}


class SimonSaysMode:
    metadata = GameMetadata(
        slug="simon_says",
        title="Simon Says",
        tagline="Merken, treffen, erweitern",
        description="Der Projector zeigt eine Sequenz. Triff die Ziele in der richtigen Reihenfolge.",
        accent="#3dff91",
        accent_secondary="#9b5cff",
        visual="simon-says",
        icon="memory",
        options=[
            GameOption("rounds", "Runden", "choice", 5, [{"value":3,"label":"3 Runden"},{"value":5,"label":"5 Runden"},{"value":8,"label":"8 Runden"}]),
            GameOption("difficulty", "Zielgröße", "choice", "easy", [
                {"value":"very_easy","label":"Sehr leicht · 4 Zonen","description":"Fünf benachbarte Zahlen bilden jeweils ein großes Zielgebiet.","description_en":"Five neighboring numbers form each large target zone."},
                {"value":"easy","label":"Leicht · 5 Zonen","description":"Vier benachbarte Zahlen bilden jeweils ein Zielgebiet.","description_en":"Four neighboring numbers form each target zone."},
                {"value":"normal","label":"Mittel · 10 Zonen","description":"Je zwei benachbarte Zahlen bilden ein Zielgebiet.","description_en":"Each pair of neighboring numbers forms one target zone."},
                {"value":"hard","label":"Schwer · 20 Zahlen","description":"Jede Zahl ist ein eigenes Ziel; der Ring bleibt egal.","description_en":"Every number is its own target; the ring still does not matter."},
            ]),
        ],
        instructions=[
            InstructionStep("Sequenz merken", "Die leuchtenden Zahlengruppen sind deine Reihenfolge.", "memory"),
            InstructionStep("Jeder Ring zählt", "Triff eine Zahl der aktuellen Gruppe. Single, Double und Triple sind gleich richtig.", "target"),
            InstructionStep("Bull ist Joker", "Single Bull und Double Bull erfüllen immer das nächste Ziel.", "joker"),
            InstructionStep("Sequenz wächst", "Die gemeinsame Aufgabe wächst über die ersten drei Runden.", "grow"),
            InstructionStep("Gleiche Chancen", "Alle spielen in einer Runde exakt dieselbe Sequenz.", "shuffle"),
        ],
        sound_theme="arcade",
        ruleset_version=2,
    )

    def initialize_player(self, player: Any, options: Dict[str, Any]) -> None:
        player.score = 0; player.marks = {}

    def initialize_state(self, state: Any, options: Dict[str, Any]) -> None:
        state.mode_state = {}
        self._generate_round_sequence(state)
        state.message = "Merke die Sequenz!"

    def _generate_round_sequence(self, state: Any) -> None:
        length = min(3, state.round_number)
        zone_count = ZONE_COUNTS.get(
            str(state.options.get("difficulty", "easy")),
            ZONE_COUNTS["easy"],
        )
        fields_per_zone = len(BOARD_ORDER) // zone_count
        zones = [
            {
                "zone": index + 1,
                "fields": BOARD_ORDER[
                    index * fields_per_zone:(index + 1) * fields_per_zone
                ],
            }
            for index in range(zone_count)
        ]
        available = list(zones)
        sequence = []
        for _ in range(length):
            sequence.append(available.pop(state.random_index(len(available))))
        state.mode_state["sequence"] = sequence
        state.mode_state["zone_count"] = zone_count
        state.mode_state["sequence_round"] = state.round_number
        state.mode_state["position"] = 0

    @staticmethod
    def _matches_target(event: Dict[str, Any], target: Dict[str, Any]) -> bool:
        if event.get("type") != "hit":
            return False
        if int(event.get("field", 0) or 0) == 25:
            return str(event.get("ring", "")) in {"single_bull", "double_bull"}
        return int(event.get("field", 0) or 0) in target.get("fields", [])

    @staticmethod
    def _target_label(target: Dict[str, Any], zone_count: int) -> str:
        fields = list(target.get("fields", []))
        if zone_count == 20 and fields:
            return str(fields[0])
        return f"Z{int(target.get('zone', 0))}"

    def on_turn_start(self, state: Any, player: Any) -> None:
        del player
        if int(state.mode_state.get("sequence_round", 0)) != state.round_number:
            self._generate_round_sequence(state)
        else:
            state.mode_state["position"] = 0

    def apply_throw(self, state: Any, player: Any, event: Dict[str, Any]) -> ThrowOutcome:
        seq = state.mode_state.get("sequence", [])
        pos = int(state.mode_state.get("position", 0))
        target = seq[pos] if pos < len(seq) else None
        if not target or not self._matches_target(event, target):
            state.mode_state["position"] = 0
            return finish_round_game(
                state,
                ThrowOutcome(
                    turn_value=0,
                    message="Falsches Feld – Sequenz reset",
                    force_hold=True,
                ),
                "{winner} gewinnt Simon Says!",
                darts_per_turn=1,
            )
        state.mode_state["position"] = pos + 1
        if pos + 1 >= len(seq):
            points = 25 * len(seq)
            player.score += points
            state.mode_state["position"] = 0
            return finish_round_game(
                state,
                ThrowOutcome(
                    turn_value=points,
                    message=f"Sequenz geschafft +{points}",
                    force_hold=True,
                ),
                "{winner} gewinnt Simon Says!",
            )
        zone_count = int(state.mode_state.get("zone_count", 5))
        return ThrowOutcome(
            turn_value=0,
            message=f"Weiter: {self._target_label(seq[pos + 1], zone_count)}",
        )

    def get_overlay(self, state: Any) -> Dict[str, Any]:
        seq = state.mode_state.get("sequence", [])
        pos = int(state.mode_state.get("position", 0))
        zone_count = int(state.mode_state.get("zone_count", 5))
        targets = []
        for step, target in enumerate(seq):
            fields = list(target.get("fields", []))
            label_field = fields[len(fields) // 2] if fields else 0
            for field in fields:
                for ring in NUMBER_RINGS:
                    targets.append({
                        "id": f"simon-{step + 1}-{ring}-{field}",
                        "field": field,
                        "ring": ring,
                        "color": "cyan" if step == pos else "green",
                        "label": str(step + 1)
                        if field == label_field and ring == "single_outer"
                        else "",
                        "pulse": step == pos,
                    })
        return {
            "prompt": " → ".join(
                self._target_label(item, zone_count) for item in seq
            ) or "Simon Says",
            "targets": targets,
            "bonus": [
                {
                    "id": "simon-joker-sbull",
                    "field": 25,
                    "ring": "single_bull",
                    "color": "gold",
                    "label": "JOKER",
                    "pulse": True,
                },
                {
                    "id": "simon-joker-dbull",
                    "field": 25,
                    "ring": "double_bull",
                    "color": "gold",
                    "label": "",
                    "pulse": True,
                },
            ],
        }


GAME_MODE = SimonSaysMode()
