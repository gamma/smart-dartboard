from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

from sdb_dartboard.game import GameEngine
from sdb_dartboard.session import SessionController
from sdb_dartboard.protocol import decode_packet, normalize_hex


FIXTURE = (
    Path(__file__).parents[1] / "fixtures" / "packets" / "fff1_decoder_v1.json"
)
COUNTUP_FIXTURE = (
    Path(__file__).parents[1] / "fixtures" / "games" / "countup_v1.json"
)
X01_FIXTURE = Path(__file__).parents[1] / "fixtures" / "games" / "x01_v1.json"
CRICKET_FIXTURE = (
    Path(__file__).parents[1] / "fixtures" / "games" / "cricket_v1.json"
)
EIGHT_BALL_FIXTURE = (
    Path(__file__).parents[1] / "fixtures" / "games" / "eight_ball_v1.json"
)
HEART_CHASE_FIXTURE = (
    Path(__file__).parents[1] / "fixtures" / "games" / "heart_chase_v1.json"
)
TARGET_RUSH_FIXTURE = (
    Path(__file__).parents[1] / "fixtures" / "games" / "target_rush_v2.json"
)
GHOST_CHASE_FIXTURE = (
    Path(__file__).parents[1] / "fixtures" / "games" / "ghost_chase_v2.json"
)
ROBIN_HOOD_FIXTURE = (
    Path(__file__).parents[1] / "fixtures" / "games" / "robin_hood_v2.json"
)
CANDY_CANNON_FIXTURE = (
    Path(__file__).parents[1] / "fixtures" / "games" / "candy_cannon_v1.json"
)
LIGHTNING_ROUND_FIXTURE = (
    Path(__file__).parents[1] / "fixtures" / "games" / "lightning_round_v2.json"
)
MINI_GOLF_FIXTURE = (
    Path(__file__).parents[1] / "fixtures" / "games" / "mini_golf_v2.json"
)
SIMON_SAYS_FIXTURE = (
    Path(__file__).parents[1] / "fixtures" / "games" / "simon_says_v2.json"
)
TREASURE_HUNT_FIXTURE = (
    Path(__file__).parents[1] / "fixtures" / "games" / "treasure_hunt_v1.json"
)
SESSION_FIXTURE = (
    Path(__file__).parents[1] / "fixtures" / "sessions" / "session_v1.json"
)


class CrossPlatformProtocolFixtureTests(unittest.TestCase):
    def test_python_decoder_matches_shared_rust_fixture(self) -> None:
        fixture = json.loads(FIXTURE.read_text(encoding="utf-8"))
        self.assertEqual(fixture["fixture_schema_version"], 1)
        for case in fixture["cases"]:
            with self.subTest(case=case["name"]):
                self.assertEqual(
                    decode_packet(normalize_hex(case["hex"])),
                    case["expected"],
                )

    def test_python_countup_matches_shared_rust_fixture(self) -> None:
        fixture = json.loads(COUNTUP_FIXTURE.read_text(encoding="utf-8"))
        self.assertEqual(fixture["fixture_schema_version"], 1)
        engine = GameEngine()
        engine.reset(
            "countup",
            fixture["players"],
            options=fixture["options"],
        )
        for step in fixture["steps"]:
            command = step["command"]
            if command["type"] == "dart":
                engine.handle_event(dict(command["event"]))
            elif command["type"] == "continue":
                engine.continue_turn()
            elif command["type"] == "correct":
                engine.correct_throw(
                    int(command["action_id"]),
                    dict(command["event"]),
                )
            elif command["type"] == "delete":
                engine.delete_throw(int(command["action_id"]))
            elif command["type"] == "undo":
                engine.undo()
            else:
                self.fail(f"Unsupported fixture command: {command['type']}")
            state = engine.state
            actual = {
                "scores": [player.score for player in state.players],
                "current_player_index": state.current_player_index,
                "darts_in_turn": state.darts_in_turn,
                "turn_score": state.turn_score,
                "round_number": state.round_number,
                "status": state.status,
                "winner_id": state.winner_id,
                "result_type": state.result_type,
            }
            self.assertEqual(actual, step["expected"])

    def test_python_x01_matches_shared_rust_fixture(self) -> None:
        fixture = json.loads(X01_FIXTURE.read_text(encoding="utf-8"))
        self.assertEqual(fixture["fixture_schema_version"], 1)
        for case in fixture["cases"]:
            with self.subTest(case=case["name"]):
                engine = GameEngine()
                engine.reset("x01", case["players"], options=case["options"])
                player = engine.state.current_player()
                player.score = int(case["setup_current_score"])
                engine.state.turn_start_values[player.id] = player.score

                for step in case["steps"]:
                    command = step["command"]
                    if command["type"] == "dart":
                        engine.handle_event(dict(command["event"]))
                    elif command["type"] == "continue":
                        engine.continue_turn()
                    elif command["type"] == "correct":
                        engine.correct_throw(
                            int(command["action_id"]),
                            dict(command["event"]),
                        )
                    elif command["type"] == "delete":
                        engine.delete_throw(int(command["action_id"]))
                    elif command["type"] == "undo":
                        engine.undo()
                    else:
                        self.fail(f"Unsupported fixture command: {command['type']}")

                    state = engine.state
                    actual = {
                        "scores": [player.score for player in state.players],
                        "current_player_index": state.current_player_index,
                        "darts_in_turn": state.darts_in_turn,
                        "turn_score": state.turn_score,
                        "round_number": state.round_number,
                        "status": state.status,
                        "winner_id": state.winner_id,
                        "result_type": state.result_type,
                        "bust": bool((state.last_event or {}).get("bust", False)),
                        "labels": [throw.label for throw in state.throws],
                        "seqs": [throw.seq for throw in state.throws],
                    }
                    self.assertEqual(actual, step["expected"])

    def test_python_cricket_matches_shared_rust_fixture(self) -> None:
        fixture = json.loads(CRICKET_FIXTURE.read_text(encoding="utf-8"))
        self.assertEqual(fixture["fixture_schema_version"], 1)
        self.assertEqual(fixture["ruleset_version"], 1)
        for case in fixture["cases"]:
            with self.subTest(case=case["name"]):
                engine = GameEngine()
                engine.reset("cricket", case["players"], options=case["options"])
                for step in case["steps"]:
                    command = step["command"]
                    if command["type"] == "dart":
                        engine.handle_event(dict(command["event"]))
                    elif command["type"] == "continue":
                        engine.continue_turn()
                    elif command["type"] == "correct":
                        engine.correct_throw(
                            int(command["action_id"]),
                            dict(command["event"]),
                        )
                    elif command["type"] == "delete":
                        engine.delete_throw(int(command["action_id"]))
                    elif command["type"] == "undo":
                        engine.undo()
                    else:
                        self.fail(f"Unsupported fixture command: {command['type']}")
                    state = engine.state
                    remaining = (state.overlay() or {}).get("cricket", {}).get(
                        "remaining", []
                    )
                    actual = {
                        "scores": [player.score for player in state.players],
                        "marks_20": [
                            player.marks.get("20", 0) for player in state.players
                        ],
                        "current_player_index": state.current_player_index,
                        "darts_in_turn": state.darts_in_turn,
                        "turn_score": state.turn_score,
                        "round_number": state.round_number,
                        "status": state.status,
                        "winner_id": state.winner_id,
                        "result_type": state.result_type,
                        "remaining_fields": [item["field"] for item in remaining],
                    }
                    self.assertEqual(actual, step["expected"])

    def test_python_eight_ball_matches_shared_rust_fixture(self) -> None:
        fixture = json.loads(EIGHT_BALL_FIXTURE.read_text(encoding="utf-8"))
        self.assertEqual(fixture["fixture_schema_version"], 1)
        self.assertEqual(fixture["ruleset_version"], 1)
        engine = GameEngine()
        engine.reset("eight_ball", fixture["players"], options=fixture["options"])
        for step in fixture["steps"]:
            command = step["command"]
            if command["type"] == "dart":
                engine.handle_event(dict(command["event"]))
            elif command["type"] == "continue":
                engine.continue_turn()
            elif command["type"] == "correct":
                engine.correct_throw(
                    int(command["action_id"]),
                    dict(command["event"]),
                )
            else:
                self.fail(f"Unsupported fixture command: {command['type']}")
            state = engine.state
            player_id = state.players[state.current_player_index].id
            actual = {
                "scores": [player.score for player in state.players],
                "current_player_index": state.current_player_index,
                "darts_in_turn": state.darts_in_turn,
                "turn_score": state.turn_score,
                "round_number": state.round_number,
                "status": state.status,
                "winner_id": state.winner_id,
                "result_type": state.result_type,
                "remaining_balls": state.mode_state["balls"][player_id],
            }
            self.assertEqual(actual, step["expected"])

    def test_python_heart_chase_matches_shared_rust_fixture(self) -> None:
        fixture = json.loads(HEART_CHASE_FIXTURE.read_text(encoding="utf-8"))
        self.assertEqual(fixture["fixture_schema_version"], 1)
        self.assertEqual(fixture["ruleset_version"], 1)
        engine = GameEngine()
        engine.reset("heart_chase", fixture["players"], options=fixture["options"])
        for step in fixture["steps"]:
            command = step["command"]
            if command["type"] == "dart":
                engine.handle_event(dict(command["event"]))
            elif command["type"] == "continue":
                engine.continue_turn()
            else:
                self.fail(f"Unsupported fixture command: {command['type']}")
            state = engine.state
            actual = {
                "scores": [player.score for player in state.players],
                "hearts": [
                    state.mode_state["hearts"][player.id]
                    for player in state.players
                ],
                "challenge_score": state.mode_state["challenge_score"],
                "current_player_index": state.current_player_index,
                "darts_in_turn": state.darts_in_turn,
                "turn_score": state.turn_score,
                "round_number": state.round_number,
                "status": state.status,
                "winner_id": state.winner_id,
                "result_type": state.result_type,
            }
            self.assertEqual(actual, step["expected"])

    def test_python_target_rush_matches_shared_rust_fixture(self) -> None:
        fixture = json.loads(TARGET_RUSH_FIXTURE.read_text(encoding="utf-8"))
        self.assertEqual(fixture["fixture_schema_version"], 1)
        self.assertEqual(fixture["ruleset_version"], 2)
        engine = GameEngine()
        engine.reset(
            "target_rush",
            fixture["players"],
            options=fixture["options"],
            random_seed=fixture["random_seed"],
        )
        for step in fixture["steps"]:
            command = step["command"]
            if command["type"] == "dart":
                engine.handle_event(dict(command["event"]))
            elif command["type"] == "continue":
                engine.continue_turn()
            elif command["type"] == "correct":
                engine.correct_throw(
                    int(command["action_id"]),
                    dict(command["event"]),
                )
            else:
                self.fail(f"Unsupported fixture command: {command['type']}")
            state = engine.state
            actual = {
                "scores": [player.score for player in state.players],
                "combos": [
                    state.mode_state["combo"].get(player.id, 0)
                    for player in state.players
                ],
                "round_targets": [
                    target["label"]
                    for target in state.mode_state["round_targets"]
                ],
                "active_target": state.mode_state["target"]["label"],
                "current_player_index": state.current_player_index,
                "darts_in_turn": state.darts_in_turn,
                "turn_score": state.turn_score,
                "round_number": state.round_number,
                "status": state.status,
                "winner_id": state.winner_id,
                "result_type": state.result_type,
                "random_cursor": state.random_cursor,
            }
            self.assertEqual(actual, step["expected"])

    def test_python_ghost_chase_matches_shared_rust_fixture(self) -> None:
        fixture = json.loads(GHOST_CHASE_FIXTURE.read_text(encoding="utf-8"))
        self.assertEqual(fixture["fixture_schema_version"], 1)
        self.assertEqual(fixture["ruleset_version"], 2)
        engine = GameEngine()
        engine.reset(
            "ghost_chase",
            fixture["players"],
            options=fixture["options"],
            random_seed=fixture["random_seed"],
        )
        for step in fixture["steps"]:
            command = step["command"]
            if command["type"] == "dart":
                engine.handle_event(dict(command["event"]))
            elif command["type"] == "continue":
                engine.continue_turn()
            elif command["type"] == "correct":
                engine.correct_throw(
                    int(command["action_id"]),
                    dict(command["event"]),
                )
            else:
                self.fail(f"Unsupported fixture command: {command['type']}")
            state = engine.state
            def values(name: str) -> list[int]:
                return [
                    state.mode_state[name][player.id]
                    for player in state.players
                ]
            actual = {
                "scores": [player.score for player in state.players],
                "combos": values("combo"),
                "escapes": values("escape"),
                "path_indices": values("path_index"),
                "path": [target["label"] for target in state.mode_state["path"]],
                "active_target": state.overlay()["targets"][0]["id"],
                "current_player_index": state.current_player_index,
                "darts_in_turn": state.darts_in_turn,
                "turn_score": state.turn_score,
                "round_number": state.round_number,
                "status": state.status,
                "winner_id": state.winner_id,
                "result_type": state.result_type,
                "random_cursor": state.random_cursor,
            }
            self.assertEqual(actual, step["expected"])

    def test_python_robin_hood_matches_shared_rust_fixture(self) -> None:
        fixture = json.loads(ROBIN_HOOD_FIXTURE.read_text(encoding="utf-8"))
        self.assertEqual(fixture["fixture_schema_version"], 1)
        self.assertEqual(fixture["ruleset_version"], 2)
        engine = GameEngine()
        engine.reset(
            "robin_hood",
            fixture["players"],
            options=fixture["options"],
            random_seed=fixture["random_seed"],
        )
        for step in fixture["steps"]:
            command = step["command"]
            if command["type"] == "dart":
                engine.handle_event(dict(command["event"]))
            elif command["type"] == "continue":
                engine.continue_turn()
            elif command["type"] == "correct":
                engine.correct_throw(
                    int(command["action_id"]),
                    dict(command["event"]),
                )
            else:
                self.fail(f"Unsupported fixture command: {command['type']}")
            state = engine.state
            labels = lambda name: [
                target["label"] for target in state.mode_state[name]
            ]
            shown_targets = (
                labels("sheriff_targets")
                if state.status == "hold"
                else labels("remaining_targets")
            )
            actual = {
                "scores": [player.score for player in state.players],
                "splits": [
                    state.mode_state["splits"].get(player.id, 0)
                    for player in state.players
                ],
                "sheriff_targets": labels("sheriff_targets"),
                "remaining_targets": labels("remaining_targets"),
                "current_arrows": labels("current_arrows"),
                "shown_targets": shown_targets,
                "current_player_index": state.current_player_index,
                "darts_in_turn": state.darts_in_turn,
                "turn_score": state.turn_score,
                "round_number": state.round_number,
                "status": state.status,
                "winner_id": state.winner_id,
                "result_type": state.result_type,
                "random_cursor": state.random_cursor,
            }
            self.assertEqual(actual, step["expected"])

    def test_python_candy_cannon_matches_shared_rust_fixture(self) -> None:
        fixture = json.loads(CANDY_CANNON_FIXTURE.read_text(encoding="utf-8"))
        self.assertEqual(fixture["fixture_schema_version"], 1)
        self.assertEqual(fixture["ruleset_version"], 1)
        engine = GameEngine()
        engine.reset("candy_cannon", fixture["players"], options=fixture["options"])
        for step in fixture["steps"]:
            command = step["command"]
            if command["type"] == "dart":
                engine.handle_event(dict(command["event"]))
            elif command["type"] == "continue":
                engine.continue_turn()
            elif command["type"] == "correct":
                engine.correct_throw(int(command["action_id"]), dict(command["event"]))
            else:
                self.fail(f"Unsupported fixture command: {command['type']}")
            state = engine.state
            actual = {
                "scores": [player.score for player in state.players],
                "charges": [
                    state.mode_state["charge"].get(player.id, 0)
                    for player in state.players
                ],
                "current_player_index": state.current_player_index,
                "darts_in_turn": state.darts_in_turn,
                "turn_score": state.turn_score,
                "round_number": state.round_number,
                "status": state.status,
                "winner_id": state.winner_id,
                "result_type": state.result_type,
            }
            self.assertEqual(actual, step["expected"])

    def test_python_lightning_round_matches_shared_rust_fixture(self) -> None:
        fixture = json.loads(LIGHTNING_ROUND_FIXTURE.read_text(encoding="utf-8"))
        self.assertEqual(fixture["fixture_schema_version"], 1)
        self.assertEqual(fixture["ruleset_version"], 2)
        engine = GameEngine()
        engine.reset("lightning_round", fixture["players"], options=fixture["options"], random_seed=fixture["random_seed"])
        for step in fixture["steps"]:
            command = step["command"]
            if command["type"] == "dart":
                engine.handle_event(dict(command["event"]))
            elif command["type"] == "continue":
                engine.continue_turn()
            elif command["type"] == "correct":
                engine.correct_throw(int(command["action_id"]), dict(command["event"]))
            else:
                self.fail(f"Unsupported fixture command: {command['type']}")
            state = engine.state
            actual = {
                "scores": [player.score for player in state.players],
                "task_id": state.mode_state["task_id"],
                "current_player_index": state.current_player_index,
                "darts_in_turn": state.darts_in_turn,
                "turn_score": state.turn_score,
                "round_number": state.round_number,
                "status": state.status,
                "winner_id": state.winner_id,
                "result_type": state.result_type,
                "random_cursor": state.random_cursor,
            }
            self.assertEqual(actual, step["expected"])

    def test_python_mini_golf_matches_shared_rust_fixture(self) -> None:
        fixture = json.loads(MINI_GOLF_FIXTURE.read_text(encoding="utf-8"))
        self.assertEqual(fixture["fixture_schema_version"], 1)
        self.assertEqual(fixture["ruleset_version"], 2)
        engine = GameEngine()
        engine.reset("mini_golf", fixture["players"], options=fixture["options"], random_seed=fixture["random_seed"])
        for step in fixture["steps"]:
            command = step["command"]
            if command["type"] == "dart": engine.handle_event(dict(command["event"]))
            elif command["type"] == "continue": engine.continue_turn()
            elif command["type"] == "correct": engine.correct_throw(int(command["action_id"]), dict(command["event"]))
            else: self.fail(f"Unsupported fixture command: {command['type']}")
            state = engine.state
            actual = {"scores":[player.score for player in state.players],"target":state.mode_state["target"]["label"],"used":state.mode_state["used"],"current_player_index":state.current_player_index,"darts_in_turn":state.darts_in_turn,"turn_score":state.turn_score,"round_number":state.round_number,"status":state.status,"winner_id":state.winner_id,"result_type":state.result_type,"random_cursor":state.random_cursor}
            self.assertEqual(actual, step["expected"])

    def test_python_simon_says_matches_shared_rust_fixture(self) -> None:
        fixture = json.loads(SIMON_SAYS_FIXTURE.read_text(encoding="utf-8"))
        self.assertEqual(fixture["fixture_schema_version"], 1)
        self.assertEqual(fixture["ruleset_version"], 2)
        engine = GameEngine()
        engine.reset(
            "simon_says",
            fixture["players"],
            options=fixture["options"],
            random_seed=fixture["random_seed"],
        )
        for step in fixture["steps"]:
            command = step["command"]
            if command["type"] == "dart":
                engine.handle_event(dict(command["event"]))
            elif command["type"] == "continue":
                engine.continue_turn()
            elif command["type"] == "correct":
                engine.correct_throw(
                    int(command["action_id"]), dict(command["event"])
                )
            else:
                self.fail(f"Unsupported fixture command: {command['type']}")
            state = engine.state
            actual = {
                "scores": [player.score for player in state.players],
                "sequence": state.mode_state["sequence"],
                "position": state.mode_state["position"],
                "current_player_index": state.current_player_index,
                "darts_in_turn": state.darts_in_turn,
                "turn_score": state.turn_score,
                "round_number": state.round_number,
                "status": state.status,
                "winner_id": state.winner_id,
                "result_type": state.result_type,
                "random_cursor": state.random_cursor,
            }
            self.assertEqual(actual, step["expected"])

    def test_python_treasure_hunt_matches_shared_rust_fixture(self) -> None:
        fixture = json.loads(TREASURE_HUNT_FIXTURE.read_text(encoding="utf-8"))
        self.assertEqual(fixture["fixture_schema_version"], 1)
        self.assertEqual(fixture["ruleset_version"], 1)
        engine = GameEngine()
        engine.reset(
            "treasure_hunt",
            fixture["players"],
            options=fixture["options"],
            random_seed=fixture["random_seed"],
        )
        for step in fixture["steps"]:
            command = step["command"]
            if command["type"] == "dart":
                engine.handle_event(dict(command["event"]))
            elif command["type"] == "continue":
                engine.continue_turn()
            elif command["type"] == "correct":
                engine.correct_throw(
                    int(command["action_id"]), dict(command["event"])
                )
            else:
                self.fail(f"Unsupported fixture command: {command['type']}")
            state = engine.state
            revealed = {
                key: {
                    "reward": item["reward"],
                    "revealed_by": item["revealed_by"],
                }
                for key, item in sorted(state.mode_state["revealed"].items())
            }
            actual = {
                "scores": [player.score for player in state.players],
                "revealed": revealed,
                "current_player_index": state.current_player_index,
                "darts_in_turn": state.darts_in_turn,
                "turn_score": state.turn_score,
                "round_number": state.round_number,
                "status": state.status,
                "winner_id": state.winner_id,
                "result_type": state.result_type,
                "random_cursor": state.random_cursor,
            }
            self.assertEqual(actual, step["expected"])

    def test_python_session_flow_matches_shared_rust_fixture(self) -> None:
        fixture = json.loads(SESSION_FIXTURE.read_text(encoding="utf-8"))
        self.assertEqual(fixture["fixture_schema_version"], 1)
        for case in fixture["cases"]:
            with self.subTest(case=case["name"]), tempfile.TemporaryDirectory() as temp:
                controller = SessionController(Path(temp) / "session.sqlite")
                try:
                    actual_ids = {}
                    symbolic_ids = {}
                    for player in case["players"]:
                        created = controller.create_player(
                            player["name"], player["avatar"], player["color"]
                        )
                        actual_ids[player["id"]] = created["id"]
                        symbolic_ids[created["id"]] = player["id"]

                    player_order = [player["id"] for player in case["players"]]
                    for step in case["steps"]:
                        command = step["command"]
                        command_type = command["type"]
                        if command_type == "start_session":
                            controller.start_session(
                                [actual_ids[player_id] for player_id in player_order]
                            )
                        elif command_type == "prepare":
                            controller.prepare_game(
                                command["game_type"], command["options"]
                            )
                        elif command_type == "start_game":
                            controller.start_game()
                        elif command_type == "playing":
                            controller.set_screen("playing")
                        elif command_type == "complete":
                            winners = [
                                actual_ids[player_id]
                                for player_id in command["winner_ids"]
                            ]
                            controller.engine.state.status = "finished"
                            controller.engine.state.winner_ids = winners
                            controller.engine.state.winner_id = (
                                winners[0] if len(winners) == 1 else None
                            )
                            controller.engine.state.result_type = command["result_type"]
                            controller.engine.state.message = "fixture result"
                            controller._finish_game_if_needed()
                            controller._persist()
                        elif command_type == "next_game":
                            controller.next_game()
                        elif command_type == "abort":
                            controller.abort_game()
                        elif command_type == "end_session":
                            controller.end_session()
                        elif command_type == "rematch":
                            self.assertFalse(controller.rematch_button())
                            self.assertTrue(controller.rematch_button())
                        else:
                            self.fail(f"Unsupported fixture command: {command_type}")

                        state = controller.public_state()
                        standings = {
                            symbolic_ids[item["id"]]: item
                            for item in state["session_statistics"]
                        }
                        actual = {
                            "screen": state["screen"],
                            "session_status": (
                                state["session"]["status"]
                                if state["session"]
                                else None
                            ),
                            "selected_mode": state["selected_mode"],
                            "starter_id": symbolic_ids.get(
                                state["starter"]["player_id"]
                            ),
                            "starter_selection": state["starter"]["selection"],
                            "game_active": state["game_id"] is not None,
                            "lineup": (
                                [
                                    symbolic_ids[player["id"]]
                                    for player in state["game"]["players"]
                                ]
                                if state["game_id"] is not None
                                else []
                            ),
                            "standings": [
                                {
                                    "id": player_id,
                                    "games": standings[player_id]["games"],
                                    "wins": standings[player_id]["wins"],
                                    "session_points": standings[player_id][
                                        "session_points"
                                    ],
                                }
                                for player_id in player_order
                            ],
                        }
                        self.assertEqual(actual, step["expected"])
                finally:
                    controller.close()


if __name__ == "__main__":
    unittest.main()
