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
            {
                "avoid_bomb",
                "block_drop",
                "boss_fight",
                "candy_cannon",
                "color_clash",
                "cookie_monster",
                "countup",
                "cricket",
                "dart_sweeper",
                "darts_bingo",
                "dragon_eggs",
                "eight_ball",
                "ghost_chase",
                "heart_chase",
                "king_of_board",
                "lightning_round",
                "mini_golf",
                "risk_it",
                "robin_hood",
                "simon_says",
                "space_defender",
                "target_rush",
                "treasure_hunt",
                "x01",
            },
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
        engine.reset("countup", ["Ada", "Bob"], options={"rounds": 5})
        engine.state.round_number = 5
        for seq in range(3):
            engine.handle_event(hit(20, seq))
        engine.continue_turn()
        for seq in range(3, 6):
            engine.handle_event(hit(10, seq))
        self.assertEqual("finished", engine.state.status)
        self.assertEqual(engine.state.players[0].id, engine.state.winner_id)

    def test_fixed_round_tie_has_no_arbitrary_winner(self):
        engine = GameEngine()
        engine.reset("countup", ["Ada", "Bob"], options={"rounds": 5})
        engine.state.round_number = 5
        for seq in range(3):
            engine.handle_event(hit(20, seq))
        engine.continue_turn()
        for seq in range(3, 6):
            engine.handle_event(hit(20, seq))
        self.assertEqual("finished", engine.state.status)
        self.assertIsNone(engine.state.winner_id)
        self.assertIn("Unentschieden", engine.state.message)

    def test_x01_bust_restores_complete_turn_and_holds(self):
        engine = GameEngine()
        engine.reset("x01", ["Ada", "Bob"], options={"start_score": 301})
        engine.state.players[0].score = 101
        engine.state.turn_start_values[engine.state.players[0].id] = 101
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
            options={"start_score": 301, "out_rule": "double"},
        )
        engine.state.players[0].score = 40
        engine.state.turn_start_values[engine.state.players[0].id] = 40
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

    def test_cricket_projector_overlay_shows_remaining_targets(self):
        engine = GameEngine()
        engine.reset("cricket", ["Ada", "Bob"])
        overlay = engine.state.as_dict()["overlay"]
        self.assertEqual(7, len(overlay["cricket"]["remaining"]))
        self.assertEqual(26, len(overlay["targets"]))
        self.assertEqual(3, overlay["cricket"]["remaining"][0]["needed"])

        event = hit(60, 77, multiplier=3, label="T20")
        event["ring"] = "triple"
        engine.handle_event(event)

        overlay = engine.state.as_dict()["overlay"]
        self.assertNotIn(20, [item["field"] for item in overlay["targets"]])
        self.assertNotIn(
            "20", [item["label"] for item in overlay["cricket"]["remaining"]]
        )

    def test_undo_restores_finished_game(self):
        engine = GameEngine()
        engine.reset("x01", ["Ada"], options={"start_score": 301})
        engine.state.players[0].score = 40
        engine.state.turn_start_values[engine.state.players[0].id] = 40
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
        engine.reset("x01", ["Ada"], options={"start_score": 301})
        engine.state.players[0].score = 101
        engine.state.turn_start_values[engine.state.players[0].id] = 101
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

    def test_risk_it_half_miss_auto_banks_on_third_dart(self):
        engine = GameEngine()
        engine.reset("risk_it", ["Ada"], options={"miss_loses": "half"})
        engine.handle_event(hit(60, 1, multiplier=3, label="T20"))
        engine.handle_event(hit(40, 2, multiplier=2, label="D20"))
        engine.handle_event({"type": "miss", "score": 0, "seq": 3})
        self.assertEqual(50, engine.state.players[0].score)
        self.assertEqual(0, engine.state.mode_state["pot"][engine.state.players[0].id])

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

    def test_fixed_round_arcade_modes_finish_even_on_miss(self):
        for slug in (
            "target_rush",
            "avoid_bomb",
            "color_clash",
            "king_of_board",
            "risk_it",
            "treasure_hunt",
        ):
            with self.subTest(slug=slug):
                engine = GameEngine()
                engine.reset(slug, ["Ada"], options={"rounds": 3})
                engine.state.round_number = 3
                engine.state.darts_in_turn = 2
                engine.handle_event(
                    {"type": "miss", "score": 0, "seq": 100, "label": "MISS"}
                )
                self.assertEqual("finished", engine.state.status)
                self.assertIsNotNone(engine.state.winner_id)

    def test_color_clash_shuffle_turn_refreshes_for_next_player(self):
        engine = GameEngine()
        engine.reset(
            "color_clash",
            ["Ada", "Bob"],
            options={"rounds": 3, "shuffle": "turn"},
        )
        initial = dict(engine.state.mode_state["colors"])
        for seq in range(3):
            engine.handle_event({"type": "miss", "score": 0, "seq": seq})
        engine.continue_turn()
        self.assertNotEqual(initial, engine.state.mode_state["colors"])

    def test_darts_bingo_full_card_does_not_finish_on_line(self):
        engine = GameEngine()
        engine.reset("darts_bingo", ["Ada"], options={"points": "full"})
        player = engine.state.players[0]
        for index in range(9):
            player.marks[str(index)] = {
                "task": f"field_{20 if index == 2 else 19}",
                "label": "Any 20" if index == 2 else "Any 19",
                "done": index in (0, 1),
            }
        event = hit(20, 80, field=20, label="S20")
        event["ring"] = "single_outer"
        engine.handle_event(event)
        self.assertEqual("running", engine.state.status)
        self.assertEqual(3, sum(cell["done"] for cell in player.marks.values()))
        self.assertEqual(9, len(engine.state.as_dict()["overlay"]["card"]))

    def test_darts_bingo_marks_every_matching_open_task(self):
        engine = GameEngine()
        engine.reset("darts_bingo", ["Ada"])
        player = engine.state.players[0]
        tasks = ("double", "even", "field_20")
        for index, task in enumerate(tasks):
            label = {"double": "Any Double", "even": "Even", "field_20": "Any 20"}[task]
            player.marks[str(index)] = {"task": task, "label": label, "done": False}
        completed_before = sum(cell["done"] for cell in player.marks.values())
        event = hit(40, 81, field=20, multiplier=2, label="D20")
        event["ring"] = "double"
        engine.handle_event(event)
        self.assertTrue(all(player.marks[str(index)]["done"] for index in range(3)))
        completed_after = sum(cell["done"] for cell in player.marks.values())
        self.assertEqual(completed_after - completed_before, player.score)

    def test_one_attempt_modes_finish_on_final_failed_attempt(self):
        for slug in ("lightning_round", "simon_says"):
            with self.subTest(slug=slug):
                engine = GameEngine()
                rounds = 5 if slug == "lightning_round" else 3
                engine.reset(slug, ["Ada"], options={"rounds": rounds})
                engine.state.round_number = rounds
                engine.handle_event(
                    {"type": "miss", "score": 0, "seq": 200, "label": "MISS"}
                )
                self.assertEqual("finished", engine.state.status)

    def test_lightning_uses_same_task_for_every_player_in_round(self):
        engine = GameEngine()
        engine.reset("lightning_round", ["Ada", "Bob"], options={"rounds": 5})
        first_task = engine.state.mode_state["task_id"]
        engine.handle_event({"type": "miss", "score": 0, "seq": 210})
        self.assertEqual(first_task, engine.state.mode_state["task_id"])
        engine.continue_turn()
        engine.handle_event({"type": "miss", "score": 0, "seq": 211})
        self.assertNotEqual(first_task, engine.state.mode_state["task_id"])

    def test_risk_it_bank_can_finish_final_round(self):
        engine = GameEngine()
        engine.reset("risk_it", ["Ada"], options={"rounds": 3})
        engine.state.round_number = 3
        engine.handle_event(hit(60, 300, multiplier=3, label="T20"))
        engine.handle_action("bank", {})
        self.assertEqual("finished", engine.state.status)
        self.assertEqual(engine.state.players[0].id, engine.state.winner_id)

    def test_boss_fight_finishes_when_hp_reaches_zero(self):
        engine = GameEngine()
        engine.reset(
            "boss_fight",
            ["Ada", "Bob"],
            options={"boss_hp": 600, "weak_points": 3},
        )
        engine.state.mode_state["boss_hp"] = 60
        engine.handle_event(hit(60, 400, multiplier=3, label="T20"))
        self.assertEqual("finished", engine.state.status)
        self.assertEqual(0, engine.state.mode_state["boss_hp"])
        self.assertEqual("team_win", engine.state.result_type)
        self.assertEqual(
            {player.id for player in engine.state.players},
            set(engine.state.winner_ids),
        )

    def test_boss_fight_can_be_lost_at_round_limit(self):
        engine = GameEngine()
        engine.reset(
            "boss_fight",
            ["Ada"],
            options={"boss_hp": 600, "weak_points": 3, "rounds": 5},
        )
        engine.state.round_number = 5
        engine.state.darts_in_turn = 2
        engine.handle_event({"type": "miss", "score": 0, "seq": 401})
        self.assertEqual("finished", engine.state.status)
        self.assertIsNone(engine.state.winner_id)
        self.assertIn("Boss gewinnt", engine.state.message)

    def test_invalid_plugin_option_is_rejected(self):
        engine = GameEngine()
        with self.assertRaisesRegex(ValueError, "Unknown options"):
            engine.reset("avoid_bomb", ["Ada"], options={"surprise": 999})

    def test_plugin_option_outside_declared_choices_is_rejected(self):
        engine = GameEngine()
        with self.assertRaisesRegex(ValueError, "choose one of"):
            engine.reset("avoid_bomb", ["Ada"], options={"rounds": 999})


if __name__ == "__main__":
    unittest.main()
