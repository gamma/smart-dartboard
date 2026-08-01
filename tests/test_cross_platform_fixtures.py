from __future__ import annotations

import json
import unittest
from pathlib import Path

from sdb_dartboard.game import GameEngine
from sdb_dartboard.protocol import decode_packet, normalize_hex


FIXTURE = (
    Path(__file__).parents[1] / "fixtures" / "packets" / "fff1_decoder_v1.json"
)
COUNTUP_FIXTURE = (
    Path(__file__).parents[1] / "fixtures" / "games" / "countup_v1.json"
)
X01_FIXTURE = Path(__file__).parents[1] / "fixtures" / "games" / "x01_v1.json"


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


if __name__ == "__main__":
    unittest.main()
