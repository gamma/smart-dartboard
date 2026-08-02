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
        self.assertEqual("dragon_fire", engine.state.last_event["effect"])
        self.assertIn("DRACHENFEUER", engine.state.message)

    def test_dragon_eggs_are_personal_collectibles_and_scales_are_visible(self):
        engine = GameEngine()
        engine.reset("dragon_eggs", ["Ada", "Bob"])
        player = engine.state.players[0]
        egg = engine.state.mode_state["eggs"][0]

        self.assertEqual(8, len(engine.state.mode_state["scales"]))
        overlay = engine.state.overlay()
        self.assertEqual("egg", overlay["bonus"][0]["icon"])
        self.assertEqual("dragon_scale", overlay["danger"][0]["icon"])

        engine.handle_event({"type": "hit", "seq": 1, **egg})
        self.assertEqual(30, player.score)
        self.assertEqual("dragon_egg", engine.state.last_event["effect"])
        self.assertNotIn(egg["label"], {item["id"] for item in engine.state.overlay()["bonus"]})

        engine.handle_event({"type": "hit", "seq": 2, **egg})
        self.assertEqual(30, player.score)
        self.assertIn("schon leer", engine.state.message)

    def test_dragon_layout_is_identical_for_every_player_in_a_round(self):
        engine = GameEngine()
        engine.reset("dragon_eggs", ["Ada", "Bob"])
        eggs = engine.state.mode_state["eggs"]
        scales = engine.state.mode_state["scales"]

        for seq in range(3):
            engine.handle_event(MISS | {"seq": seq})
        engine.continue_turn()

        self.assertIs(eggs, engine.state.mode_state["eggs"])
        self.assertIs(scales, engine.state.mode_state["scales"])

        for seq in range(3, 6):
            engine.handle_event(MISS | {"seq": seq})
        engine.continue_turn()

        self.assertEqual(2, engine.state.mode_state["layout_round"])
        self.assertIsNot(eggs, engine.state.mode_state["eggs"])
        self.assertIsNot(scales, engine.state.mode_state["scales"])

    def test_ghost_combo_and_escape(self):
        engine = GameEngine()
        engine.reset(
            "ghost_chase",
            ["Ada", "Bob"],
            options={"rounds": 5, "difficulty": "easy"},
        )
        path = list(engine.state.mode_state["path"])
        first = dict(path[0])
        engine.handle_event({"type": "hit", "seq": 1, **first})
        second = dict(path[1])
        engine.handle_event({"type": "hit", "seq": 2, **second})
        self.assertEqual(90, engine.state.players[0].score)
        engine.handle_event(MISS | {"seq": 3})
        engine.continue_turn()

        ghost_targets = engine.state.as_dict()["overlay"]["targets"]
        self.assertTrue(all(item["field"] == path[0]["field"] for item in ghost_targets))
        self.assertEqual(
            {"single_inner", "single_outer"},
            {item["ring"] for item in ghost_targets},
        )
        self.assertEqual(0, engine.state.mode_state["path_index"][engine.state.players[1].id])
        self.assertEqual(0, engine.state.mode_state["escape"][engine.state.players[1].id])

    def test_cookie_sugar_rush_and_milk_rescue(self):
        engine = GameEngine()
        engine.reset(
            "cookie_monster",
            ["Ada"],
            options={"difficulty": "hard"},
        )
        player = engine.state.players[0]
        board = engine.state.mode_state["layouts"]["1"]
        good = [
            item for item in board.values()
            if item["kind"] == "green"
        ][:3]
        for seq, item in enumerate(good):
            engine.handle_event({"type": "hit", "seq": seq, **item["dart"]})
        self.assertTrue(engine.state.mode_state["sugar"][player.id])
        engine.continue_turn()
        next_good = next(
            item for cookie_id, item in board.items()
            if item["kind"] == "green"
            and cookie_id not in engine.state.mode_state["collected"][player.id]
        )
        before = player.score
        engine.handle_event({"type": "hit", "seq": 4, **next_good["dart"]})
        self.assertEqual(before + 20, player.score)

        player.score = -30
        engine.state.turn_score = -30
        engine.handle_event(hit(25, "single_bull", 1, 5))
        self.assertEqual(0, player.score)

    def test_cookie_overlay_uses_cookie_props_and_marks_bull_as_milk(self):
        engine = GameEngine()
        engine.reset("cookie_monster", ["Ada"])

        overlay = engine.state.as_dict()["overlay"]
        cookie_items = overlay["bonus"] + overlay["targets"] + overlay["danger"]

        self.assertEqual(15, sum(
            item.get("icon") in {"cookie", "cookie_moldy"}
            for item in cookie_items
        ))
        self.assertEqual(
            {"single_bull", "double_bull"},
            {
                item["ring"] for item in overlay["bonus"]
                if item.get("icon") == "milk"
            },
        )
        self.assertEqual(3, len(overlay["visual_legend"]))
        self.assertEqual(15, len(overlay["zones"]))
        self.assertTrue(all(
            item.get("match_field")
            for item in cookie_items
            if item.get("icon") in {"cookie", "cookie_moldy"}
        ))

    def test_cookie_boards_are_personal_but_use_the_same_wave_layout(self):
        engine = GameEngine()
        engine.reset("cookie_monster", ["Ada", "Bob"])
        ada, bob = engine.state.players
        board = engine.state.mode_state["layouts"]["1"]
        cookie_id, cookie = next(
            (cookie_id, item)
            for cookie_id, item in board.items()
            if item["kind"] != "moldy"
        )

        engine.handle_event({"type": "hit", "seq": 1, **cookie["dart"]})
        self.assertIn(cookie_id, engine.state.mode_state["collected"][ada.id])
        self.assertNotIn(cookie_id, engine.state.mode_state["collected"][bob.id])
        for seq in range(2, 4):
            engine.handle_event(MISS | {"seq": seq})
        engine.continue_turn()
        bob_overlay_ids = {
            item["id"]
            for item in (
                engine.state.as_dict()["overlay"]["bonus"]
                + engine.state.as_dict()["overlay"]["targets"]
            )
            if item.get("icon") == "cookie"
        }
        self.assertIn(cookie["dart"]["label"], bob_overlay_ids)

    def test_cookie_board_only_refills_after_all_good_cookies_are_eaten(self):
        engine = GameEngine()
        engine.reset("cookie_monster", ["Ada"])
        player = engine.state.players[0]
        board = engine.state.mode_state["layouts"]["1"]
        good_ids = [
            cookie_id
            for cookie_id, item in board.items()
            if item["kind"] != "moldy"
        ]
        last_id = good_ids[-1]
        engine.state.mode_state["collected"][player.id] = good_ids[:-1]

        engine.handle_event({
            "type": "hit",
            "seq": 1,
            **board[last_id]["dart"],
        })

        self.assertEqual(2, engine.state.mode_state["wave"][player.id])
        self.assertEqual([], engine.state.mode_state["collected"][player.id])
        self.assertIn("2", engine.state.mode_state["layouts"])
        self.assertEqual(
            "cookie_board_clear",
            engine.state.last_event["effect"],
        )

    def test_cookie_easy_accepts_every_ring_of_a_cookie_number(self):
        engine = GameEngine()
        engine.reset("cookie_monster", ["Ada"])
        player = engine.state.players[0]
        cookie = next(
            item
            for item in engine.state.mode_state["layouts"]["1"].values()
            if item["kind"] == "blue"
        )
        field = cookie["dart"]["field"]

        engine.handle_event(hit(field, "triple", 3, 1))

        self.assertEqual(20, player.score)
        self.assertEqual("cookie_eaten", engine.state.last_event["effect"])

    def test_king_of_board_easy_double_takes_the_whole_number(self):
        engine = GameEngine()
        engine.reset(
            "king_of_board",
            ["Ada", "Bob"],
            options={"ownership": "area"},
        )

        engine.handle_event(hit(20, "double", 2, 1))

        self.assertEqual(4, engine.state.players[0].score)
        owned = engine.state.as_dict()["overlay"]["owned"]
        self.assertEqual(
            {"single_inner", "triple", "single_outer", "double"},
            {item["ring"] for item in owned if item["field"] == 20},
        )

    def test_king_of_board_easy_triple_takes_neighboring_numbers(self):
        engine = GameEngine()
        engine.reset(
            "king_of_board",
            ["Ada", "Bob"],
            options={"ownership": "area"},
        )

        engine.handle_event(hit(20, "triple", 3, 1))

        self.assertEqual(12, engine.state.players[0].score)
        owned = engine.state.as_dict()["overlay"]["owned"]
        self.assertEqual({5, 20, 1}, {item["field"] for item in owned})
        self.assertIn("5 · 20 · 1", engine.state.message)

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

    def test_space_defender_uses_seeded_targets_and_shared_team_score(self):
        players = [
            {"id": "ada", "name": "Ada"},
            {"id": "bob", "name": "Bob"},
        ]
        first = GameEngine()
        second = GameEngine()
        first.reset("space_defender", players, random_seed=42)
        second.reset("space_defender", players, random_seed=42)
        self.assertEqual(
            first.state.mode_state["ships"], second.state.mode_state["ships"]
        )
        ship = first.state.mode_state["ships"][0]
        target = ship["target"]
        first.handle_event(
            hit(
                int(target["field"]),
                str(target["ring"]),
                int(target["multiplier"]),
            )
        )
        self.assertEqual([10, 10], [player.score for player in first.state.players])
        self.assertEqual("space_destroy", first.state.mode_state["last_effect"])
        self.assertEqual(1, first.state.mode_state["destroyed"])

    def test_space_defender_old_state_can_spawn_the_next_wave(self):
        engine = GameEngine()
        engine.reset("space_defender", ["Ada", "Bob"], random_seed=42)
        for key in (
            "next_ship_id",
            "last_effect",
            "effect_points",
            "effect_damage",
            "destroyed",
        ):
            engine.state.mode_state.pop(key)
        engine.next_player()
        engine.next_player()
        self.assertEqual(2, engine.state.mode_state["wave"])
        self.assertEqual(7, engine.state.mode_state["next_ship_id"])

    def test_space_defender_skipped_team_rounds_still_advance_and_finish(self):
        engine = GameEngine()
        engine.reset("space_defender", ["Ada", "Bob"], options={"waves": 4})
        for expected_wave in (2, 3):
            engine.next_player()
            engine.next_player()
            self.assertEqual(expected_wave, engine.state.mode_state["wave"])
        engine.next_player()
        engine.next_player()
        self.assertEqual(4, engine.state.mode_state["wave"])
        self.assertEqual("finished", engine.state.status)
        self.assertEqual("challenge_loss", engine.state.result_type)

        engine = GameEngine()
        engine.reset("space_defender", ["Ada", "Bob"], options={"waves": 4})
        engine.state.mode_state.update({
            "wave": 4,
            "ships": engine.state.mode_state["ships"][:1],
            "cleanup": False,
        })
        engine.next_player()
        engine.next_player()
        self.assertTrue(engine.state.mode_state["cleanup"])
        self.assertEqual("running", engine.state.status)
        engine.next_player()
        engine.next_player()
        self.assertEqual("finished", engine.state.status)
        self.assertEqual("challenge_loss", engine.state.result_type)

    def test_candy_cannon_fire_and_overheat(self):
        engine = GameEngine()
        engine.reset("candy_cannon", ["Ada", "Bob"])
        ada, bob = engine.state.players
        bob.score = 100
        engine.state.mode_state["charge"][ada.id] = 8
        engine.handle_event(hit(25, "single_bull", 1, 4))
        self.assertEqual(50, ada.score)
        self.assertEqual(75, bob.score)
        self.assertEqual(0, engine.state.mode_state["charge"][ada.id])
        self.assertEqual("candy_fire", engine.state.last_event["effect"])
        self.assertEqual(bob.id, engine.state.last_event["target_player_id"])
        self.assertEqual(25, engine.state.last_event["target_score_loss"])
        engine.state.mode_state["charge"][ada.id] = 9
        engine.handle_event(hit(20, "triple", 3, 5))
        self.assertEqual(0, engine.state.mode_state["charge"][ada.id])
        self.assertEqual("candy_overheat", engine.state.last_event["effect"])

    def test_candy_cannon_bull_charges_until_fire_is_ready(self):
        engine = GameEngine()
        engine.reset("candy_cannon", ["Ada", "Bob"])
        ada = engine.state.players[0]

        engine.handle_event(hit(25, "double_bull", 2, 1))

        self.assertEqual(4, engine.state.mode_state["charge"][ada.id])
        self.assertEqual(0, ada.score)
        self.assertEqual([], engine.state.as_dict()["overlay"]["targets"])

    def test_candy_cannon_ready_overlay_marks_both_bulls_without_action(self):
        engine = GameEngine()
        engine.reset("candy_cannon", ["Ada", "Bob"])
        ada = engine.state.players[0]
        engine.state.mode_state["charge"][ada.id] = 8

        overlay = engine.state.as_dict()["overlay"]

        self.assertNotIn("actions", overlay)
        self.assertEqual(
            {"single_bull", "double_bull"},
            {target["ring"] for target in overlay["targets"]},
        )
        self.assertIn("BULL", overlay["prompt"])

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

        engine.handle_event(hit(25, "single_bull", 1, 1))
        self.assertEqual(piece_index + 1, engine.state.mode_state["piece_index"])
        self.assertEqual("running", engine.state.status)
        self.assertEqual(1, engine.state.darts_in_turn)

        double_engine = GameEngine()
        double_engine.reset("block_drop", ["Ada"])
        double_piece_index = double_engine.state.mode_state["piece_index"]
        double_engine.handle_event(hit(25, "double_bull", 2, 2))
        self.assertEqual(
            double_piece_index + 1,
            double_engine.state.mode_state["piece_index"],
        )
        self.assertEqual("running", double_engine.state.status)
        self.assertEqual(1, double_engine.state.darts_in_turn)

    def test_block_drop_difficulty_changes_drop_rings(self):
        easy = GameEngine()
        easy.reset("block_drop", ["Ada"], options={"difficulty": "easy"})
        easy_piece_index = easy.state.mode_state["piece_index"]
        easy.handle_event(hit(20, "triple", 3, 1))
        self.assertEqual(easy_piece_index + 1, easy.state.mode_state["piece_index"])

        normal = GameEngine()
        normal.reset("block_drop", ["Ada"], options={"difficulty": "normal"})
        normal_piece_index = normal.state.mode_state["piece_index"]
        normal.handle_event(hit(20, "triple", 3, 2))
        self.assertEqual(normal_piece_index, normal.state.mode_state["piece_index"])
        normal.handle_event(hit(20, "double", 2, 3))
        self.assertEqual(normal_piece_index + 1, normal.state.mode_state["piece_index"])

        hard = GameEngine()
        hard.reset("block_drop", ["Ada"], options={"difficulty": "hard"})
        hard_piece_index = hard.state.mode_state["piece_index"]
        hard.handle_event(hit(20, "double", 2, 4))
        self.assertEqual(hard_piece_index, hard.state.mode_state["piece_index"])
        hard.handle_event(hit(25, "single_bull", 1, 5))
        self.assertEqual(hard_piece_index + 1, hard.state.mode_state["piece_index"])

    def test_block_drop_action_pace_sinks_after_every_dart(self):
        engine = GameEngine()
        engine.reset("block_drop", ["Ada"], options={"pace": "action"})
        start_y = engine.state.mode_state["piece"]["y"]

        engine.handle_event({**MISS, "seq": 1})
        self.assertEqual(start_y + 1, engine.state.mode_state["piece"]["y"])
        self.assertIn("SINK", engine.state.message)
        overlay = engine.state.as_dict()["overlay"]
        self.assertEqual(10, overlay["panel"]["progress"]["max"])
        self.assertIn("NACH JEDEM DART", overlay["panel"]["subline"])

    def test_block_drop_can_end_turn_after_drop_by_option(self):
        engine = GameEngine()
        engine.reset("block_drop", ["Ada"], options={"drop_flow": "hold"})

        engine.handle_event(hit(25, "single_bull", 1, 1))

        self.assertEqual("hold", engine.state.status)
        self.assertEqual(1, engine.state.darts_in_turn)

    def test_block_drop_overlay_uses_four_contiguous_color_areas(self):
        engine = GameEngine()
        engine.reset("block_drop", ["Ada"])
        public_state = engine.state.as_dict()
        zones = public_state["overlay"]["zones"]
        normal = [zone for zone in zones if zone["field"] != 25]
        fields_by_color = {}
        for zone in normal:
            if zone["color"] == "#28e7ff":
                continue
            fields_by_color.setdefault(zone["color"], []).append(zone["field"])
        self.assertEqual(
            {
                "#a77bff": [12, 5, 20, 1, 18],
                "#81b29a": [4, 13, 6, 10, 15],
                "#f4a261": [2, 17, 3, 19, 7],
                "#e9c46a": [16, 8, 11, 14, 9],
            },
            fields_by_color,
        )
        drop_zones = [
            zone
            for zone in normal
            if zone["color"] == "#28e7ff"
        ]
        self.assertEqual(20, len(drop_zones))
        self.assertTrue(
            all(zone["rings"] == ["triple", "double"] for zone in drop_zones)
        )
        self.assertEqual(
            ["left", "rotate_left", "rotate_right", "right", "drop"],
            [
                item["icon"]
                for item in public_state["mode"]["control_legend"]
            ],
        )

    def test_block_drop_rotates_in_both_directions(self):
        engine = GameEngine()
        engine.reset("block_drop", ["Ada"])
        engine.state.mode_state["piece"] = {
            "kind": "T",
            "rotation": 0,
            "x": 1,
            "y": 0,
        }

        engine.handle_event(hit(20, seq=1))
        self.assertEqual(3, engine.state.mode_state["piece"]["rotation"])

        engine.handle_event(hit(3, seq=2))
        self.assertEqual(0, engine.state.mode_state["piece"]["rotation"])

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
        engine.state.mode_state.update({
            "seeded": True,
            "direct_hit_seen": True,
            "mines": [20, 1, 2, 3, 4],
        })
        lives = engine.state.mode_state["lives"]
        engine.handle_event(hit(20, "triple", 3, 1))
        self.assertEqual(lives - 1, engine.state.mode_state["lives"])
        self.assertIn(20, engine.state.mode_state["exploded"])
        self.assertEqual({}, engine.state.mode_state["revealed"])
        self.assertEqual("mine_explosion", engine.state.last_event["effect"])
        mine_zone = next(
            zone for zone in engine.state.as_dict()["overlay"]["zones"]
            if zone["field"] == 20
        )
        self.assertEqual("mine", mine_zone["icon"])

    def test_dart_sweeper_exploded_mine_only_costs_one_life(self):
        engine = GameEngine()
        engine.reset("dart_sweeper", ["Ada"], options={"preset": "classic"})
        engine.state.mode_state.update({
            "seeded": True,
            "direct_hit_seen": True,
            "mines": [20, 1, 2, 3, 4],
        })
        lives = engine.state.mode_state["lives"]
        engine.handle_event(hit(20, seq=1))
        engine.handle_event(hit(20, seq=2))
        self.assertEqual(lives - 1, engine.state.mode_state["lives"])

    def test_dart_sweeper_miss_does_not_consume_first_hit_safety(self):
        engine = GameEngine()
        engine.reset(
            "dart_sweeper", ["Ada"], options={"preset": "classic"}, random_seed=42
        )
        engine.handle_event(MISS)
        self.assertFalse(engine.state.mode_state["seeded"])
        engine.handle_event(hit(20, seq=1))
        protected = {20, 5, 1}
        self.assertTrue(protected.isdisjoint(engine.state.mode_state["mines"]))

    def test_dart_sweeper_bull_then_direct_hit_keeps_safe_halo(self):
        engine = GameEngine()
        engine.reset(
            "dart_sweeper", ["Ada"], options={"preset": "expert"}, random_seed=42
        )
        engine.handle_event(hit(25, "double_bull", 2, 1))
        engine.handle_event(hit(20, seq=2))
        protected = {20, 5, 1}
        self.assertTrue(protected.isdisjoint(engine.state.mode_state["mines"]))
        self.assertTrue(engine.state.mode_state["direct_hit_seen"])

    def test_dart_sweeper_awards_reveals_to_the_entire_team(self):
        engine = GameEngine()
        engine.reset(
            "dart_sweeper",
            [{"id": "ada", "name": "Ada"}, {"id": "bob", "name": "Bob"}],
            options={"preset": "classic"},
            random_seed=42,
        )
        engine.handle_event(hit(20, seq=1))
        self.assertEqual(
            engine.state.players[0].score, engine.state.players[1].score
        )
        self.assertGreater(engine.state.players[0].score, 0)


if __name__ == "__main__":
    unittest.main()
