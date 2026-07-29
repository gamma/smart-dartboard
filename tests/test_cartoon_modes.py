from __future__ import annotations

import unittest

from sdb_dartboard.game import GameEngine


def hit(
    field: int,
    ring: str = "single_outer",
    multiplier: int = 1,
    seq: int = 1,
):
    prefix = "T" if multiplier == 3 else "D" if multiplier == 2 else "S"
    if field == 25:
        label = "DBull" if multiplier == 2 else "SBull"
    else:
        label = f"{prefix}{field}"
    return {
        "type": "hit",
        "field": field,
        "ring": ring,
        "multiplier": multiplier,
        "score": field * multiplier,
        "label": label,
        "seq": seq,
    }


MISS = {"type": "miss", "score": 0, "label": "MISS", "seq": 999}


class CartoonModeTests(unittest.TestCase):
    def test_every_new_mode_exposes_instructions_and_panel(self):
        player_counts = {"heart_chase": 2, "robin_hood": 2, "candy_cannon": 2, "eight_ball": 2}
        for slug in (
            "heart_chase",
            "robin_hood",
            "dragon_eggs",
            "ghost_chase",
            "cookie_monster",
            "space_defender",
            "candy_cannon",
            "mini_golf",
            "eight_ball",
            "block_drop",
            "dart_sweeper",
        ):
            with self.subTest(slug=slug):
                engine = GameEngine()
                engine.reset(
                    slug,
                    [f"P{index}" for index in range(player_counts.get(slug, 1))],
                )
                self.assertTrue(engine.state.as_dict()["mode"]["instructions"])
                self.assertTrue(engine.state.as_dict()["overlay"]["panel"])

    def test_heart_chase_opens_then_eliminates_and_skips_player(self):
        engine = GameEngine()
        engine.reset("heart_chase", ["Ada", "Bob", "Cid"], options={"hearts": 2})
        for seq in range(3):
            engine.handle_event(hit(20, seq=seq))
        self.assertEqual(60, engine.state.mode_state["challenge_score"])
        engine.continue_turn()
        engine.state.mode_state["hearts"][engine.state.players[1].id] = 1
        for seq in range(3, 6):
            engine.handle_event(hit(1, seq=seq))
        self.assertEqual(0, engine.state.mode_state["hearts"][engine.state.players[1].id])
        engine.continue_turn()
        self.assertEqual("Cid", engine.state.current_player().name)

    def test_heart_chase_last_active_player_wins(self):
        engine = GameEngine()
        engine.reset("heart_chase", ["Ada", "Bob"], options={"hearts": 2})
        engine.state.mode_state.update({
            "opening_turn": False,
            "challenge_score": 100,
            "hearts": {
                engine.state.players[0].id: 1,
                engine.state.players[1].id: 1,
            },
        })
        for seq in range(3):
            engine.handle_event(MISS | {"seq": seq})
        self.assertEqual("finished", engine.state.status)
        self.assertEqual(engine.state.players[1].id, engine.state.winner_id)

    def test_robin_hood_splits_each_duplicate_arrow_once(self):
        engine = GameEngine()
        engine.reset("robin_hood", ["Ada", "Bob"])
        target = hit(20, "triple", 3)
        engine.state.mode_state["sheriff_targets"] = [dict(target), dict(target)]
        engine.state.mode_state["remaining_targets"] = [dict(target), dict(target)]
        engine.handle_event(target | {"seq": 1})
        self.assertEqual(90, engine.state.players[0].score)
        self.assertEqual(1, len(engine.state.mode_state["remaining_targets"]))
        engine.handle_event(target | {"seq": 2})
        self.assertEqual(180, engine.state.players[0].score)
        self.assertEqual(0, len(engine.state.mode_state["remaining_targets"]))

    def test_dragon_heat_persists_and_third_scale_penalizes_turn(self):
        engine = GameEngine()
        engine.reset("dragon_eggs", ["Ada"])
        player = engine.state.players[0]
        egg = engine.state.mode_state["eggs"][0]
        scale = engine.state.mode_state["scales"][0]
        engine.state.mode_state["heat"][player.id] = 2
        engine.handle_event({
            "type": "hit", "seq": 1, **egg,
        })
        engine.handle_event({
            "type": "hit", "seq": 2, **scale,
        })
        self.assertEqual(0, engine.state.mode_state["heat"][player.id])
        self.assertEqual(0, player.score)
        self.assertIn("DRAGON AWAKES", engine.state.message)

    def test_ghost_combo_and_escape(self):
        engine = GameEngine()
        engine.reset("ghost_chase", ["Ada"], options={"rounds": 5, "difficulty": "easy"})
        first = dict(engine.state.mode_state["target"])
        engine.handle_event({"type": "hit", "seq": 1, **first})
        second = dict(engine.state.mode_state["target"])
        engine.handle_event({"type": "hit", "seq": 2, **second})
        self.assertEqual(90, engine.state.players[0].score)
        old = dict(engine.state.mode_state["target"])
        engine.handle_event(MISS | {"seq": 3})
        engine.continue_turn()
        engine.handle_event(MISS | {"seq": 4})
        engine.handle_event(MISS | {"seq": 5})
        self.assertNotEqual(old["label"], engine.state.mode_state["target"]["label"])

    def test_cookie_sugar_rush_and_milk_rescue(self):
        engine = GameEngine()
        engine.reset("cookie_monster", ["Ada"])
        player = engine.state.players[0]
        good = next(
            item for item in engine.state.mode_state["cookies"].values()
            if item["kind"] == "green"
        )
        for seq in range(3):
            engine.handle_event({"type": "hit", "seq": seq, **good["dart"]})
        self.assertTrue(engine.state.mode_state["sugar"][player.id])
        engine.continue_turn()
        next_good = next(
            item for item in engine.state.mode_state["cookies"].values()
            if item["kind"] == "green"
        )
        before = player.score
        engine.handle_event({"type": "hit", "seq": 4, **next_good["dart"]})
        self.assertEqual(before + 20, player.score)

        player.score = -30
        engine.state.turn_score = -30
        engine.handle_event(hit(25, "single_bull", 1, 5))
        self.assertEqual(0, player.score)

    def test_space_defender_team_win_has_all_winners(self):
        engine = GameEngine()
        engine.reset("space_defender", ["Ada", "Bob"], options={"waves": 4})
        engine.state.mode_state.update({"ships": [], "wave": 4, "cleanup": True})
        engine.state.current_player_index = 1
        engine.state.darts_in_turn = 2
        engine.handle_event(MISS)
        self.assertEqual("team_win", engine.state.result_type)
        self.assertEqual(
            {player.id for player in engine.state.players},
            set(engine.state.winner_ids),
        )

    def test_candy_cannon_fire_and_overheat(self):
        engine = GameEngine()
        engine.reset("candy_cannon", ["Ada", "Bob"])
        ada, bob = engine.state.players
        bob.score = 100
        engine.state.mode_state["charge"][ada.id] = 8
        engine.handle_action("fire", {})
        self.assertEqual(50, ada.score)
        self.assertEqual(75, bob.score)
        self.assertEqual(0, engine.state.mode_state["charge"][ada.id])
        engine.state.mode_state["charge"][ada.id] = 9
        engine.handle_event(hit(25, "single_bull", 1, 5))
        self.assertEqual(0, engine.state.mode_state["charge"][ada.id])

    def test_mini_golf_same_hole_and_low_score_wins(self):
        engine = GameEngine()
        engine.reset("mini_golf", ["Ada", "Bob"], options={"holes": 6, "difficulty": "easy"})
        target = dict(engine.state.mode_state["target"])
        engine.state.round_number = 6
        engine.state.mode_state["hole"] = 6
        engine.handle_event({"type": "hit", "seq": 1, **target})
        self.assertEqual("hold", engine.state.status)
        engine.continue_turn()
        self.assertEqual(target["label"], engine.state.mode_state["target"]["label"])
        engine.state.darts_in_turn = 2
        engine.handle_event(MISS)
        self.assertEqual("finished", engine.state.status)
        self.assertEqual(engine.state.players[0].id, engine.state.winner_id)

    def test_eight_ball_foul_and_early_black_eight(self):
        engine = GameEngine()
        engine.reset("eight_ball", ["Ada", "Bob"])
        engine.handle_event(hit(1, "single_inner", 1, 1))
        self.assertNotIn(1, engine.state.mode_state["balls"][engine.state.players[0].id])
        engine.handle_event(hit(15, "single_outer", 1, 2))
        self.assertEqual("hold", engine.state.status)
        engine.continue_turn()
        engine.handle_event(hit(25, "double_bull", 2, 3))
        self.assertEqual(engine.state.players[0].id, engine.state.winner_id)

    def test_block_drop_team_win_and_undo(self):
        engine = GameEngine()
        engine.reset("block_drop", ["Ada", "Bob"])
        engine.state.mode_state["lines"] = 5
        engine.handle_event(hit(25, "double_bull", 2, 1))
        self.assertEqual("team_win", engine.state.result_type)
        self.assertEqual(
            engine.state.players[0].score,
            engine.state.players[1].score,
        )
        engine.undo()
        self.assertEqual("running", engine.state.status)
        self.assertEqual(5, engine.state.mode_state["lines"])

    def test_block_drop_gravity_runs_once_after_every_player(self):
        engine = GameEngine()
        engine.reset("block_drop", ["Ada", "Bob"])
        start_y = engine.state.mode_state["piece"]["y"]

        for seq in range(3):
            engine.handle_event({**MISS, "seq": seq})
        engine.continue_turn()
        self.assertEqual(start_y, engine.state.mode_state["piece"]["y"])

        for seq in range(3, 6):
            engine.handle_event({**MISS, "seq": seq})
        engine.continue_turn()
        self.assertEqual(2, engine.state.round_number)
        self.assertEqual(start_y + 1, engine.state.mode_state["piece"]["y"])
        self.assertIn("eine Zeile", engine.state.message)

    def test_block_drop_uses_bulls_for_drop_actions(self):
        engine = GameEngine()
        engine.reset("block_drop", ["Ada"])
        piece_index = engine.state.mode_state["piece_index"]
        start_y = engine.state.mode_state["piece"]["y"]

        engine.handle_event(hit(25, "single_bull", 1, 1))
        self.assertEqual(start_y + 1, engine.state.mode_state["piece"]["y"])
        self.assertEqual(piece_index, engine.state.mode_state["piece_index"])

        engine.handle_event(hit(25, "double_bull", 2, 2))
        self.assertEqual(piece_index + 1, engine.state.mode_state["piece_index"])

    def test_block_drop_overlay_uses_three_contiguous_color_areas(self):
        engine = GameEngine()
        engine.reset("block_drop", ["Ada"])
        zones = engine.state.as_dict()["overlay"]["zones"]
        normal = [zone for zone in zones if zone["field"] != 25]
        fields_by_color = {}
        for zone in normal:
            fields_by_color.setdefault(zone["color"], []).append(zone["field"])
        self.assertEqual(
            {
                "#e9c46a": [3, 19, 7, 16, 8, 11, 14],
                "#f4a261": [9, 12, 5, 20, 1, 18],
                "#81b29a": [4, 13, 6, 10, 15, 2, 17],
            },
            fields_by_color,
        )

    def test_dart_sweeper_first_hit_safe_and_triple_reveals_neighbors(self):
        engine = GameEngine()
        engine.reset("dart_sweeper", ["Ada"], options={"preset": "classic"})
        engine.handle_event(hit(20, "triple", 3, 1))
        self.assertNotIn(20, engine.state.mode_state["mines"])
        self.assertEqual(3, len(engine.state.mode_state["revealed"]))

    def test_dart_sweeper_double_reveals_one_safe_neighbor(self):
        engine = GameEngine()
        engine.reset("dart_sweeper", ["Ada"], options={"preset": "classic"})
        engine.handle_event(hit(20, "double", 2, 1))
        self.assertEqual(2, len(engine.state.mode_state["revealed"]))

    def test_dart_sweeper_multiplier_does_not_protect_direct_mine(self):
        engine = GameEngine()
        engine.reset("dart_sweeper", ["Ada"], options={"preset": "classic"})
        engine.state.mode_state.update({"seeded": True, "mines": [20, 1, 2, 3, 4]})
        lives = engine.state.mode_state["lives"]
        engine.handle_event(hit(20, "triple", 3, 1))
        self.assertEqual(lives - 1, engine.state.mode_state["lives"])
        self.assertIn(20, engine.state.mode_state["exploded"])
        self.assertEqual({}, engine.state.mode_state["revealed"])

    def test_dart_sweeper_exploded_mine_only_costs_one_life(self):
        engine = GameEngine()
        engine.reset("dart_sweeper", ["Ada"], options={"preset": "classic"})
        engine.state.mode_state.update({"seeded": True, "mines": [20, 1, 2, 3, 4]})
        lives = engine.state.mode_state["lives"]
        engine.handle_event(hit(20, seq=1))
        engine.handle_event(hit(20, seq=2))
        self.assertEqual(lives - 1, engine.state.mode_state["lives"])


if __name__ == "__main__":
    unittest.main()
