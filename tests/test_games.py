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
            {"countup", "cricket", "x01", "target_rush", "avoid_bomb", "color_clash", "risk_it", "king_of_board", "treasure_hunt"},
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

    def test_countup_finishes_after_configured_rounds(self):
        engine = GameEngine()
        engine.reset("countup", ["Ada", "Bob"], options={"rounds": 1})
        for seq in range(3):
            engine.handle_event(hit(20, seq))
        engine.continue_turn()
        for seq in range(3, 6):
            engine.handle_event(hit(10, seq))
        self.assertEqual("finished", engine.state.status)
        self.assertEqual(engine.state.players[0].id, engine.state.winner_id)

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

    def test_throw_correction_replays_the_current_turn(self):
        engine = GameEngine()
        engine.reset("countup", ["Ada"])
        engine.handle_event(hit(20, 1, label="S20"))
        engine.handle_event(hit(60, 2, multiplier=3, label="T20"))
        engine.handle_event({"type": "miss", "score": 0, "seq": 3, "label": "MISS"})

        engine.correct_turn_throw(0, hit(40, 99, multiplier=2, label="D20"))

        self.assertEqual(100, engine.state.players[0].score)
        self.assertEqual("hold", engine.state.status)
        self.assertEqual(["D20", "T20", "MISS"], [throw.label for throw in engine.state.throws])
        self.assertEqual(1, engine.state.throws[0].seq)
        self.assertTrue(engine.state.throws[0].raw["corrected"])

    def test_x01_correction_recalculates_later_throws(self):
        engine = GameEngine()
        engine.reset("x01", ["Ada"], options={"start_score": 101})
        engine.handle_event(hit(20, 1, label="S20"))
        engine.handle_event(hit(20, 2, label="S20"))
        self.assertEqual(61, engine.state.players[0].score)

        engine.correct_turn_throw(0, hit(60, 99, multiplier=3, label="T20"))

        self.assertEqual(21, engine.state.players[0].score)
        self.assertEqual(80, engine.state.turn_score)

    def test_target_rush_exposes_overlay(self):
        engine = GameEngine()
        engine.reset("target_rush", ["Ada"], options={"difficulty": "easy"})
        state = engine.state.as_dict()
        self.assertEqual("target_rush", state["game_type"])
        self.assertTrue(state["overlay"]["targets"])
        self.assertIn("Triff", state["overlay"]["prompt"])

    def test_avoid_bomb_exposes_danger_overlay(self):
        engine = GameEngine()
        engine.reset("avoid_bomb", ["Ada"], options={"bomb_count": 2})
        overlay = engine.state.as_dict()["overlay"]
        self.assertEqual(2, len(overlay["danger"]))

    def test_color_clash_scores_colored_segments(self):
        engine = GameEngine()
        engine.reset("color_clash", ["Ada"], options={"shuffle": "turn"})
        colors = engine.state.mode_state["colors"]
        first_id, color = next(iter(colors.items()))
        label = "DBull" if first_id == "DBULL" else "SBull" if first_id == "SBULL" else first_id
        if label.startswith("T"):
            field, multiplier, score, ring = int(label[1:]), 3, int(label[1:]) * 3, "triple"
        elif label.startswith("D") and label != "DBull":
            field, multiplier, score, ring = int(label[1:]), 2, int(label[1:]) * 2, "double"
        elif label == "DBull":
            field, multiplier, score, ring = 25, 2, 50, "double_bull"
        elif label == "SBull":
            field, multiplier, score, ring = 25, 1, 25, "single_bull"
        else:
            field, multiplier, score, ring = int(label[1:]), 1, int(label[1:]), "single_outer"
        event = hit(score, 99, field=field, multiplier=multiplier, label=label)
        event["ring"] = ring
        engine.handle_event(event)
        self.assertEqual({"gold": 50, "cyan": 25, "green": 10, "red": -25}[color], engine.state.players[0].score)
    def test_risk_it_banks_pot_via_action(self):
        engine = GameEngine()
        engine.reset("risk_it", ["Ada"])
        engine.handle_event(hit(60, 1, multiplier=3, label="T20"))
        self.assertEqual(0, engine.state.players[0].score)
        self.assertEqual(60, engine.state.mode_state["pot"][engine.state.players[0].id])
        engine.handle_action("bank", {})
        self.assertEqual(60, engine.state.players[0].score)
        self.assertEqual("hold", engine.state.status)

    def test_king_of_board_tracks_owned_segments(self):
        engine = GameEngine()
        engine.reset("king_of_board", ["Ada", "Bob"])
        event = hit(60, 1, multiplier=3, label="T20")
        event["ring"] = "triple"
        engine.handle_event(event)
        self.assertEqual(1, engine.state.players[0].score)
        overlay = engine.state.as_dict()["overlay"]
        self.assertEqual(1, len(overlay["owned"]))
    def test_treasure_hunt_reveals_reward(self):
        engine = GameEngine()
        engine.reset("treasure_hunt", ["Ada"], options={"traps": 3})
        key, item = next(iter(engine.state.mode_state["hidden"].items()))
        target = item["dart"]
        event = hit(target["score"], 123, field=target["field"], multiplier=target["multiplier"], label=target["label"])
        event["ring"] = target["ring"]
        engine.handle_event(event)
        self.assertTrue(engine.state.mode_state["revealed"])
        self.assertIsNotNone(engine.state.as_dict()["overlay"])


if __name__ == "__main__":
    unittest.main()
