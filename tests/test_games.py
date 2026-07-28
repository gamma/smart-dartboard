from __future__ import annotations

import unittest

from sdb_dartboard.game import GameEngine
from sdb_dartboard.games import registry


def hit(score: int, seq: int = 1, field: int = 20, multiplier: int = 1, label: str = "S20"):
    return {
        "type": "hit",
        "score": score,
        "seq": seq,
        "field": field,
        "multiplier": multiplier,
        "label": label,
    }


class GameRegistryTests(unittest.TestCase):
    def test_builtin_modes_are_discovered(self):
        self.assertEqual(
            {"countup", "cricket", "x01"},
            {mode.metadata.slug for mode in registry.all()},
        )
        for mode in registry.all():
            self.assertTrue(mode.metadata.instructions)
            self.assertTrue(mode.metadata.visual)


class GameEngineTests(unittest.TestCase):
    def test_countup_holds_after_three_darts(self):
        engine = GameEngine()
        engine.reset("countup", ["Ada", "Bob"])
        for seq in range(3):
            engine.handle_event(hit(60, seq, multiplier=3, label="T20"))
        self.assertEqual(180, engine.state.players[0].score)
        self.assertEqual("hold", engine.state.status)
        engine.continue_turn()
        self.assertEqual("Bob", engine.state.current_player().name)
        self.assertEqual(0, engine.state.darts_in_turn)

    def test_x01_bust_restores_complete_turn_and_holds(self):
        engine = GameEngine()
        engine.reset("x01", ["Ada", "Bob"], options={"start_score": 101})
        engine.handle_event(hit(60, 1, multiplier=3, label="T20"))
        engine.handle_event(hit(60, 2, multiplier=3, label="T20"))
        self.assertEqual(101, engine.state.players[0].score)
        self.assertEqual(60, engine.state.turn_score)
        self.assertEqual("hold", engine.state.status)
        self.assertTrue(engine.state.last_event["bust"])
        self.assertIn("Bust", engine.state.message)

    def test_double_out_requires_a_double(self):
        engine = GameEngine()
        engine.reset(
            "x01",
            ["Ada"],
            options={"start_score": 40, "out_rule": "double"},
        )
        engine.handle_event(hit(40, 1, field=20, multiplier=2, label="D20"))
        self.assertEqual("finished", engine.state.status)
        self.assertEqual(engine.state.players[0].id, engine.state.winner_id)

    def test_cricket_scores_overflow_only_against_open_opponent(self):
        engine = GameEngine()
        engine.reset("cricket", ["Ada", "Bob"])
        engine.handle_event(hit(60, 1, multiplier=3, label="T20"))
        engine.handle_event(hit(60, 2, multiplier=3, label="T20"))
        self.assertEqual(60, engine.state.players[0].score)
        self.assertEqual(3, engine.state.players[0].marks["20"])

    def test_undo_restores_finished_game(self):
        engine = GameEngine()
        engine.reset("x01", ["Ada"], options={"start_score": 40})
        engine.handle_event(hit(40, 1, multiplier=2, label="D20"))
        engine.undo()
        self.assertEqual("running", engine.state.status)
        self.assertEqual(40, engine.state.players[0].score)
        self.assertIsNone(engine.state.winner_id)


if __name__ == "__main__":
    unittest.main()
