from __future__ import annotations

from typing import Any, Dict, List
from uuid import uuid4

from .arcade import choose_targets, overlay_item, same_target, zone_id
from .base import GameMetadata, GameOption, InstructionStep, ThrowOutcome

SHIP_STATS = {
    "scout": {"hp": 1, "points": 10},
    "fighter": {"hp": 2, "points": 25},
    "cruiser": {"hp": 3, "points": 50},
    "boss": {"hp": 5, "points": 100},
}


class SpaceDefenderMode:
    metadata = GameMetadata(
        slug="space_defender",
        title="Space Defender",
        tagline="Gemeinsam die Wellen stoppen",
        description="Ein fröhliches Koop-Weltraumabenteuer: Trefft die Raumschiffe, bevor die Invasion zehn Gegner erreicht.",
        accent="#4f9d69",
        accent_secondary="#f2c14e",
        visual="space-defender",
        icon="rocket",
        min_players=1,
        options=[
            GameOption("waves", "Wellen", "choice", 4, [
                {"value": 4, "label": "4 Wellen"},
                {"value": 6, "label": "6 Wellen"},
            ]),
        ],
        instructions=[
            InstructionStep("Schiffe treffen", "Triff das exakte Segment. Single, Double und Triple machen 1, 2 oder 3 Schaden.", "rocket"),
            InstructionStep("Bull-Laser", "Bull trifft alle aktiven Schiffe gleichzeitig.", "laser"),
            InstructionStep("Erde retten", "Nach der letzten Welle räumt ihr gemeinsam die restlichen Schiffe ab.", "earth"),
        ],
        sound_theme="arcade",
        ruleset_version=2,
    )

    def initialize_player(self, player: Any, options: Dict[str, Any]) -> None:
        player.score = 0
        player.marks = {}

    def initialize_state(self, state: Any, options: Dict[str, Any]) -> None:
        state.mode_state = {"ships": [], "wave": 1, "cleanup": False}
        self._spawn_wave(state, 1)

    def _wave_types(self, state: Any, wave: int) -> List[str]:
        count = min(6, max(3, 2 + len(state.players) // 2))
        final_wave = wave >= int(state.options.get("waves", 4))
        if final_wave:
            return ["boss"] + ["scout"] * max(2, count - 1)
        if wave == 1:
            return ["scout"] * count
        if wave % 3 == 0:
            return ["cruiser"] + ["fighter"] * 2 + ["scout"] * max(0, count - 3)
        return ["fighter"] * 2 + ["scout"] * max(1, count - 2)

    def _spawn_wave(self, state: Any, wave: int) -> None:
        existing = [zone_id(ship["target"]) for ship in state.mode_state["ships"]]
        types = self._wave_types(state, wave)
        targets = choose_targets(len(types), "normal", exclude=existing)
        for ship_type, target in zip(types, targets):
            stats = SHIP_STATS[ship_type]
            state.mode_state["ships"].append({
                "id": str(uuid4()),
                "type": ship_type,
                "target": target,
                "hp": stats["hp"],
                "max_hp": stats["hp"],
                "points": stats["points"],
            })
        state.mode_state["wave"] = wave
        state.message = f"Welle {wave} ist gelandet!"

    def _damage_ship(self, player: Any, ship: Dict[str, Any], damage: int) -> int:
        ship["hp"] = max(0, int(ship["hp"]) - damage)
        if ship["hp"] > 0:
            return 0
        points = int(ship["points"])
        player.score += points
        return points

    def _team_result(
        self,
        state: Any,
        won: bool,
        message: str,
        points: int = 0,
    ) -> ThrowOutcome:
        winner_ids = [player.id for player in state.players] if won else []
        return ThrowOutcome(
            turn_value=points,
            message=message,
            finished=True,
            winner_ids=winner_ids,
            result_type="team_win" if won else "challenge_loss",
        )

    def _finish_team_round(
        self,
        state: Any,
        points: int = 0,
    ) -> ThrowOutcome | None:
        wave = int(state.mode_state.get("wave", 1))
        maximum = int(state.options.get("waves", 4))
        if wave >= maximum:
            if not state.mode_state["ships"]:
                return self._team_result(
                    state,
                    True,
                    "ERDE GERETTET! Das Team gewinnt!",
                    points,
                )
            if state.mode_state.get("cleanup"):
                return self._team_result(
                    state,
                    False,
                    "Die Flotte entkommt · Team-Niederlage",
                    points,
                )
            state.mode_state["cleanup"] = True
            state.message = "LETZTE AUFRÄUMRUNDE!"
            return None

        self._spawn_wave(state, wave + 1)
        if len(state.mode_state["ships"]) >= 10:
            return self._team_result(
                state,
                False,
                "INVASION! Zehn Schiffe haben die Erde erreicht",
                points,
            )
        return None

    @staticmethod
    def _apply_terminal_outcome(state: Any, outcome: ThrowOutcome) -> None:
        state.status = "finished"
        state.winner_id = outcome.winner_id
        state.winner_ids = list(outcome.winner_ids)
        state.result_type = outcome.result_type
        state.message = outcome.message

    def apply_throw(self, state: Any, player: Any, event: Dict[str, Any]) -> ThrowOutcome:
        ships = state.mode_state.get("ships", [])
        destroyed_points = 0
        if event.get("type") == "hit" and int(event.get("field", 0)) == 25:
            damage = 2 if int(event.get("multiplier", 1)) == 2 else 1
            for ship in ships:
                destroyed_points += self._damage_ship(player, ship, damage)
            message = f"FLÄCHENLASER! {damage} Schaden an allen"
        elif event.get("type") == "hit":
            ship = next((item for item in ships if same_target(event, item["target"])), None)
            if ship:
                damage = int(event.get("multiplier", 1))
                destroyed_points = self._damage_ship(player, ship, damage)
                message = f"{ship['type'].upper()} getroffen · {damage} Schaden"
            else:
                message = "Laser geht vorbei"
        else:
            message = "Laser geht vorbei"
        state.mode_state["ships"] = [ship for ship in ships if int(ship["hp"]) > 0]

        end_of_team_round = (
            state.darts_in_turn == 2
            and state.current_player_index == len(state.players) - 1
        )
        if end_of_team_round:
            terminal = self._finish_team_round(state, destroyed_points)
            if terminal:
                return terminal
            message = state.message

        return ThrowOutcome(destroyed_points, message)

    def on_turn_skipped(self, state: Any, player: Any) -> None:
        del player
        if state.current_player_index != len(state.players) - 1:
            return
        terminal = self._finish_team_round(state)
        if terminal:
            self._apply_terminal_outcome(state, terminal)

    def get_overlay(self, state: Any) -> Dict[str, Any]:
        ships = state.mode_state.get("ships", [])
        wave = int(state.mode_state.get("wave", 1))
        return {
            "prompt": "Aufräumrunde!" if state.mode_state.get("cleanup") else f"Welle {wave} verteidigen!",
            "targets": [
                overlay_item(ship["target"], "green", f"{ship['hp']} HP", True)
                for ship in ships
            ],
            "panel": {
                "title": "SPACE DEFENDER",
                "headline": f"Welle {wave} · {len(ships)} Schiffe",
                "subline": "Bei 10 aktiven Schiffen ist die Erde verloren",
                "progress": {"value": len(ships), "max": 10},
                "rows": [
                    {
                        "label": f"{ship['type'].title()} · {ship['target']['label']}",
                        "value": f"{ship['hp']}/{ship['max_hp']} HP",
                    }
                    for ship in ships[:6]
                ],
            },
        }


GAME_MODE = SpaceDefenderMode()
