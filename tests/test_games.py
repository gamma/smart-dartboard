from __future__ import annotations

import json
import unittest

from sdb_dartboard.game import GameEngine
from sdb_dartboard.games import registry
from sdb_dartboard.games.avoid_bomb import BOMB_POOL


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

    def test_skipping_every_player_finishes_all_fixed_round_games(self):
        round_modes = [
            mode
            for mode in registry.all()
            if any(option.key == "rounds" for option in mode.metadata.options)
        ]
        for mode in round_modes:
            with self.subTest(slug=mode.metadata.slug):
                player_count = max(2, mode.metadata.min_players)
                engine = GameEngine()
                engine.reset(
                    mode.metadata.slug,
                    [f"Player {index + 1}" for index in range(player_count)],
                )
                engine.state.round_number = int(engine.state.options["rounds"])
                for index in range(player_count):
                    engine.next_player()
                    expected = (
                        "finished"
                        if index == player_count - 1
                        else "running"
                    )
                    self.assertEqual(expected, engine.state.status)

    def test_king_of_board_counts_fully_skipped_rounds(self):
        engine = GameEngine()
        engine.reset(
            "king_of_board",
            ["Ada", "Bob"],
            options={"rounds": 3, "ownership": "segment"},
        )
        for expected_round in (2, 3):
            engine.next_player()
            engine.next_player()
            self.assertEqual("running", engine.state.status)
            self.assertEqual(expected_round, engine.state.round_number)
        engine.next_player()
        engine.next_player()
        self.assertEqual("finished", engine.state.status)
        self.assertEqual(3, engine.state.round_number)
        self.assertEqual([], engine.state.throws)

    def test_skipping_mini_golf_counts_as_four_strokes(self):
        engine = GameEngine()
        engine.reset(
            "mini_golf",
            ["Ada", "Bob"],
            options={"holes": 6, "difficulty": "easy"},
        )
        engine.state.round_number = 6
        engine.state.players[0].score = 1
        engine.next_player()
        self.assertEqual(5, engine.state.players[0].score)
        engine.next_player()
        self.assertEqual(4, engine.state.players[1].score)
        self.assertEqual("finished", engine.state.status)
        self.assertEqual(engine.state.players[1].id, engine.state.winner_id)

    def test_skipping_final_boss_turn_is_a_challenge_loss(self):
        engine = GameEngine()
        engine.reset(
            "boss_fight",
            ["Ada", "Bob"],
            options={"boss_hp": 600, "weak_points": 3, "rounds": 5},
        )
        engine.state.round_number = 5
        engine.next_player()
        engine.next_player()
        self.assertEqual("finished", engine.state.status)
        self.assertEqual("challenge_loss", engine.state.result_type)
        self.assertIn("Boss gewinnt", engine.state.message)

    def test_skipping_risk_it_discards_the_open_pot(self):
        engine = GameEngine()
        engine.reset("risk_it", ["Ada", "Bob"], options={"rounds": 3})
        player = engine.state.current_player()
        engine.state.mode_state["pot"][player.id] = 40
        engine.next_player()
        self.assertEqual(0, engine.state.mode_state["pot"][player.id])

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

    def test_x01_countdown_is_recorded_as_success_telemetry(self):
        engine = GameEngine()
        engine.reset("x01", ["Ada"], options={"start_score": 301})
        engine.handle_event(hit(60, 1, field=20, multiplier=3, label="T20"))
        self.assertEqual(241, engine.state.players[0].score)
        self.assertEqual(60, engine.state.throws[-1].mode_points)
        self.assertEqual("success", engine.state.throws[-1].outcome)

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

    def test_previous_turn_can_be_corrected_after_next_player_throws(self):
        engine = GameEngine()
        engine.reset("countup", ["Ada", "Bob"], options={"rounds": 5})
        for seq in range(1, 4):
            engine.handle_event(hit(20, seq))
        engine.continue_turn()
        engine.handle_event(hit(10, 4, field=10, label="S10"))

        turns = engine.editable_turns()
        previous = next(turn for turn in turns if not turn["current"])
        engine.correct_throw(
            previous["darts"][0]["action_id"],
            hit(60, 99, multiplier=3, label="T20"),
        )

        self.assertEqual([100, 10], [player.score for player in engine.state.players])
        self.assertEqual("Bob", engine.state.current_player().name)
        self.assertEqual(1, engine.state.darts_in_turn)
        self.assertEqual(
            ["T20", "S20", "S20", "S10"],
            [throw.label for throw in engine.state.throws],
        )

    def test_deleting_previous_dart_preserves_the_player_transition(self):
        engine = GameEngine()
        engine.reset("countup", ["Ada", "Bob"], options={"rounds": 5})
        for seq in range(1, 4):
            engine.handle_event(hit(20, seq))
        engine.continue_turn()
        engine.handle_event(hit(10, 4, field=10, label="S10"))
        previous = next(
            turn for turn in engine.editable_turns() if not turn["current"]
        )

        engine.delete_throw(previous["darts"][2]["action_id"])

        self.assertEqual([40, 10], [player.score for player in engine.state.players])
        self.assertEqual("Bob", engine.state.current_player().name)
        self.assertEqual(1, engine.state.darts_in_turn)

    def test_undo_after_player_change_restores_completed_turn_first(self):
        engine = GameEngine()
        engine.reset("countup", ["Ada", "Bob"], options={"rounds": 5})
        for seq in range(1, 4):
            engine.handle_event(hit(20, seq))
        engine.continue_turn()

        engine.undo()

        self.assertEqual("Ada", engine.state.current_player().name)
        self.assertEqual("hold", engine.state.status)
        self.assertEqual(3, engine.state.darts_in_turn)
        self.assertEqual("continue", engine.last_undo_action["kind"])

    def test_editable_timeline_survives_json_checkpoint_restore(self):
        engine = GameEngine()
        engine.reset("countup", ["Ada", "Bob"], options={"rounds": 5})
        for seq in range(1, 4):
            engine.handle_event(hit(20, seq))
        engine.continue_turn()
        restored = GameEngine()
        restored.import_state(json.loads(json.dumps(engine.export_state())))
        previous = next(
            turn for turn in restored.editable_turns() if not turn["current"]
        )

        restored.correct_throw(
            previous["darts"][0]["action_id"],
            hit(40, 99, multiplier=2, label="D20"),
        )

        self.assertEqual(80, restored.state.players[0].score)
        self.assertEqual("Bob", restored.state.current_player().name)

    def test_target_rush_exposes_overlay(self):
        engine = GameEngine()
        engine.reset("target_rush", ["Ada"], options={"difficulty": "easy"})
        state = engine.state.as_dict()
        self.assertEqual("target_rush", state["game_type"])
        self.assertTrue(state["overlay"]["targets"])
        self.assertIn("Triff", state["overlay"]["prompt"])
        target = state["overlay"]["targets"][0]
        engine.handle_event(
            {
                "type": "hit",
                "label": target["label"],
                "score": target["field"],
                "seq": 1,
                "field": target["field"],
                "ring": target["ring"],
                "multiplier": 1,
            }
        )
        self.assertEqual("success", engine.state.throws[-1].outcome)

    def test_avoid_bomb_exposes_danger_overlay(self):
        engine = GameEngine()
        engine.reset("avoid_bomb", ["Ada"], options={"bomb_count": 4})
        overlay = engine.state.as_dict()["overlay"]
        self.assertEqual(4, len(overlay["danger"]))
        self.assertTrue(all(item["icon"] == "mine" for item in overlay["danger"]))
        self.assertEqual("mine", overlay["visual_legend"][0]["icon"])

    def test_avoid_bomb_pool_contains_both_single_rings(self):
        ring_counts = {
            ring: sum(1 for bomb in BOMB_POOL if bomb["ring"] == ring)
            for ring in ("single_inner", "single_outer", "triple", "double")
        }
        self.assertEqual(
            {
                "single_inner": 20,
                "single_outer": 20,
                "triple": 20,
                "double": 20,
            },
            ring_counts,
        )

    def test_avoid_bomb_marks_exact_hit_for_full_explosion(self):
        engine = GameEngine()
        engine.reset("avoid_bomb", ["Ada"], options={"bomb_count": 4})
        engine.state.mode_state["bombs"] = [{
            "label": "T20",
            "field": 20,
            "ring": "triple",
            "multiplier": 3,
            "score": 60,
        }]

        event = hit(60, 1, field=20, multiplier=3, label="T20")
        event["ring"] = "triple"
        engine.handle_event(event)

        self.assertEqual("bomb_explosion", engine.state.last_event["effect"])
        self.assertEqual(-50, engine.state.players[0].score)

    def test_avoid_bomb_neighbor_hit_triggers_close_call_without_penalty(self):
        engine = GameEngine()
        engine.reset("avoid_bomb", ["Ada"], options={"bomb_count": 4})
        engine.state.mode_state["bombs"] = [{
            "label": "T20",
            "field": 20,
            "ring": "triple",
            "multiplier": 3,
            "score": 60,
        }]

        event = hit(3, 1, field=1, multiplier=3, label="T1")
        event["ring"] = "triple"
        engine.handle_event(event)

        self.assertEqual("bomb_near_miss", engine.state.last_event["effect"])
        self.assertEqual(20, engine.state.last_event["near_bomb"]["field"])
        self.assertEqual(3, engine.state.players[0].score)
        self.assertIn("DAS WAR KNAPP", engine.state.message)

    def test_avoid_bomb_radial_neighbor_also_triggers_close_call(self):
        engine = GameEngine()
        engine.reset("avoid_bomb", ["Ada"], options={"bomb_count": 4})
        engine.state.mode_state["bombs"] = [{
            "label": "T20",
            "field": 20,
            "ring": "triple",
            "multiplier": 3,
            "score": 60,
        }]

        event = hit(20, 1, field=20, label="S20")
        event["ring"] = "single_outer"
        engine.handle_event(event)

        self.assertEqual("bomb_near_miss", engine.state.last_event["effect"])
        self.assertEqual(20, engine.state.players[0].score)

    def test_avoid_bomb_bull_neighbor_triggers_close_call(self):
        engine = GameEngine()
        engine.reset("avoid_bomb", ["Ada"], options={"bomb_count": 4})
        engine.state.mode_state["bombs"] = [{
            "label": "DBull",
            "field": 25,
            "ring": "double_bull",
            "multiplier": 2,
            "score": 50,
        }]

        event = hit(25, 1, field=25, label="SBull")
        event["ring"] = "single_bull"
        engine.handle_event(event)

        self.assertEqual("bomb_near_miss", engine.state.last_event["effect"])
        self.assertEqual(25, engine.state.players[0].score)

    def test_avoid_bomb_adds_one_bomb_after_every_full_player_round(self):
        engine = GameEngine()
        engine.reset(
            "avoid_bomb",
            ["Ada", "Bob"],
            options={"bomb_count": 4, "bomb_growth": "steady", "hidden_bombs": "visible"},
        )
        initial_bombs = list(engine.state.mode_state["bombs"])

        for seq in range(3):
            engine.handle_event({"type": "miss", "score": 0, "seq": seq})
        engine.continue_turn()
        self.assertEqual(initial_bombs, engine.state.mode_state["bombs"])

        for seq in range(3, 6):
            engine.handle_event({"type": "miss", "score": 0, "seq": seq})
        engine.continue_turn()
        self.assertEqual(2, engine.state.round_number)
        self.assertEqual(5, len(engine.state.mode_state["bombs"]))
        self.assertEqual(initial_bombs, engine.state.mode_state["bombs"][:4])
        self.assertIn("neue Bombe", engine.state.message)

    def test_avoid_bomb_escalating_growth_adds_the_new_round_number(self):
        engine = GameEngine()
        engine.reset(
            "avoid_bomb",
            ["Ada"],
            options={"bomb_count": 4, "bomb_growth": "escalating", "hidden_bombs": "visible"},
        )
        for seq in range(3):
            engine.handle_event({"type": "miss", "score": 0, "seq": seq})
        engine.continue_turn()
        self.assertEqual(2, engine.state.round_number)
        self.assertEqual(6, len(engine.state.mode_state["bombs"]))
        self.assertIn("2 neue Bomben", engine.state.message)

    def test_avoid_bomb_hit_does_not_shuffle_existing_bombs(self):
        engine = GameEngine()
        engine.reset("avoid_bomb", ["Ada"], options={"bomb_count": 4})
        bombs = list(engine.state.mode_state["bombs"])
        bomb = bombs[0]
        engine.handle_event({
            "type": "hit",
            "seq": 1,
            "field": bomb["field"],
            "ring": bomb["ring"],
            "score": bomb["score"],
            "label": bomb["label"],
        })
        self.assertEqual(bombs, engine.state.mode_state["bombs"])

    def test_avoid_bomb_memory_hides_then_reveals_bombs(self):
        engine = GameEngine()
        engine.reset(
            "avoid_bomb",
            ["Ada"],
            options={
                "bomb_count": 4,
                "bomb_growth": "steady",
                "hidden_bombs": "memory",
            },
        )
        mode = registry.get("avoid_bomb")

        for round_number in (2, 3):
            engine.state.round_number = round_number
            mode.on_turn_start(engine.state, engine.state.current_player())

        hidden_ids = engine.state.mode_state["hidden_bomb_ids"]
        self.assertTrue(hidden_ids)
        overlay = engine.state.as_dict()["overlay"]
        self.assertEqual(len(engine.state.mode_state["bombs"]) - len(hidden_ids), len(overlay["danger"]))
        self.assertIn("versteckt", overlay["prompt"])

        engine.state.round_number = 4
        mode.on_turn_start(engine.state, engine.state.current_player())
        self.assertEqual([], engine.state.mode_state["hidden_bomb_ids"])
        self.assertEqual(
            len(engine.state.mode_state["bombs"]),
            len(engine.state.as_dict()["overlay"]["danger"]),
        )

    def test_hitting_hidden_bomb_reveals_it(self):
        engine = GameEngine()
        engine.reset("avoid_bomb", ["Ada"], options={"bomb_count": 4})
        bomb = engine.state.mode_state["bombs"][0]
        bomb_id = f"{bomb['ring']}:{bomb['field']}"
        engine.state.mode_state["hidden_bomb_ids"] = [bomb_id]

        event = hit(
            bomb["score"],
            field=bomb["field"],
            multiplier=bomb["multiplier"],
            label=bomb["label"],
        )
        event["ring"] = bomb["ring"]
        engine.handle_event(event)

        self.assertEqual("bomb_explosion", engine.state.last_event["effect"])
        self.assertNotIn(bomb_id, engine.state.mode_state["hidden_bomb_ids"])

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

    def test_risk_it_third_dart_creates_hot_pot_for_next_player(self):
        engine = GameEngine()
        engine.reset("risk_it", ["Ada", "Bob"], options={"rounds": 3})
        engine.handle_event(hit(20, 1, field=20, label="S20"))
        engine.handle_event(hit(18, 2, field=18, label="S18"))
        engine.handle_event(hit(27, 3, field=9, multiplier=3, label="T9"))

        ada = engine.state.players[0]
        self.assertEqual(0, ada.score)
        self.assertEqual(65, engine.state.mode_state["pot"][ada.id])
        self.assertEqual(
            {"owner_id": ada.id, "amount": 65, "field": 9, "label": "9"},
            engine.state.mode_state["hot_pot"],
        )
        self.assertEqual([], engine.state.as_dict()["overlay"]["actions"])

        engine.continue_turn()
        overlay = engine.state.as_dict()["overlay"]
        self.assertIn("TRIFF 9 MIT DART 1", overlay["prompt"])
        self.assertEqual(4, len(overlay["targets"]))

    def test_risk_it_next_players_first_dart_can_steal_hot_pot(self):
        engine = GameEngine()
        engine.reset("risk_it", ["Ada", "Bob"], options={"rounds": 3})
        for seq, score in enumerate((20, 20, 20), start=1):
            engine.handle_event(hit(score, seq, field=20, label="S20"))
        engine.continue_turn()

        engine.handle_event(hit(5, 4, field=20, label="S20"))

        ada, bob = engine.state.players
        self.assertEqual(0, ada.score)
        self.assertEqual(60, bob.score)
        self.assertEqual(5, engine.state.mode_state["pot"][bob.id])
        self.assertEqual(5, engine.state.turn_score)
        self.assertIsNone(engine.state.mode_state["hot_pot"])
        self.assertEqual("risk_steal", engine.state.last_event["effect"])
        self.assertEqual(60, engine.state.last_event["stolen_amount"])

    def test_risk_it_failed_heist_secures_owner_pot_and_expires(self):
        engine = GameEngine()
        engine.reset("risk_it", ["Ada", "Bob"], options={"rounds": 3})
        for seq in range(1, 4):
            engine.handle_event(hit(20, seq, field=20, label="S20"))
        engine.continue_turn()

        engine.handle_event(hit(19, 4, field=19, label="S19"))
        ada, bob = engine.state.players
        self.assertEqual(60, ada.score)
        self.assertEqual(19, engine.state.mode_state["pot"][bob.id])
        self.assertIsNone(engine.state.mode_state["hot_pot"])
        self.assertEqual("risk_secured", engine.state.last_event["effect"])

        engine.handle_event(hit(20, 5, field=20, label="S20"))
        self.assertEqual(60, ada.score)
        self.assertEqual(39, engine.state.mode_state["pot"][bob.id])

    def test_risk_it_skipped_heist_secures_hot_pot_for_owner(self):
        engine = GameEngine()
        engine.reset("risk_it", ["Ada", "Bob"], options={"rounds": 3})
        for seq in range(1, 4):
            engine.handle_event(hit(20, seq, field=20, label="S20"))
        engine.continue_turn()

        engine.next_player()

        self.assertEqual(60, engine.state.players[0].score)
        self.assertIsNone(engine.state.mode_state["hot_pot"])
        self.assertEqual("Ada", engine.state.current_player().name)

    def test_risk_it_solo_third_dart_banks_without_heist(self):
        engine = GameEngine()
        engine.reset("risk_it", ["Ada"], options={"rounds": 3})
        for seq in range(1, 4):
            engine.handle_event(hit(20, seq, field=20, label="S20"))

        self.assertEqual(60, engine.state.players[0].score)
        self.assertEqual(0, engine.state.mode_state["pot"][engine.state.players[0].id])
        self.assertIsNone(engine.state.mode_state["hot_pot"])
        self.assertEqual("hold", engine.state.status)

    def test_risk_it_final_hot_pot_gets_one_last_heist_dart(self):
        engine = GameEngine()
        engine.reset("risk_it", ["Ada", "Bob"], options={"rounds": 3})
        engine.state.round_number = 3
        for seq in range(1, 4):
            engine.handle_event(hit(1, seq, field=1, label="S1"))
        engine.continue_turn()
        engine.handle_event(hit(2, 4, field=2, label="S2"))
        engine.handle_event(hit(2, 5, field=2, label="S2"))
        engine.handle_event(hit(20, 6, field=20, label="S20"))

        self.assertEqual("hold", engine.state.status)
        self.assertTrue(engine.state.mode_state["final_heist"])
        engine.continue_turn()
        engine.handle_event(hit(1, 7, field=20, label="S20"))

        self.assertEqual("finished", engine.state.status)
        self.assertEqual(engine.state.players[0].id, engine.state.winner_id)
        self.assertEqual(27, engine.state.players[0].score)
        self.assertEqual(0, engine.state.players[1].score)

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

    def test_color_clash_round_layout_is_identical_for_every_player(self):
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
        self.assertEqual(initial, engine.state.mode_state["colors"])

        for seq in range(3, 6):
            engine.handle_event({"type": "miss", "score": 0, "seq": seq})
        engine.continue_turn()

        self.assertEqual(2, engine.state.round_number)
        self.assertNotEqual(initial, engine.state.mode_state["colors"])

    def test_color_clash_dart_layout_sequence_repeats_for_every_player(self):
        engine = GameEngine()
        engine.reset(
            "color_clash",
            ["Ada", "Bob"],
            options={"rounds": 3, "shuffle": "dart"},
        )
        layouts = [
            dict(layout) for layout in engine.state.mode_state["layouts"]
        ]

        self.assertEqual(layouts[0], engine.state.mode_state["colors"])
        engine.handle_event({"type": "miss", "score": 0, "seq": 1})
        self.assertEqual(layouts[1], engine.state.mode_state["colors"])
        engine.handle_event({"type": "miss", "score": 0, "seq": 2})
        self.assertEqual(layouts[2], engine.state.mode_state["colors"])
        engine.handle_event({"type": "miss", "score": 0, "seq": 3})
        engine.continue_turn()

        self.assertEqual(layouts[0], engine.state.mode_state["colors"])
        engine.handle_event({"type": "miss", "score": 0, "seq": 4})
        self.assertEqual(layouts[1], engine.state.mode_state["colors"])
        engine.handle_event({"type": "miss", "score": 0, "seq": 5})
        self.assertEqual(layouts[2], engine.state.mode_state["colors"])

    def test_easy_target_rush_keeps_one_number_for_the_whole_round(self):
        engine = GameEngine()
        engine.reset(
            "target_rush",
            ["Ada", "Bob"],
            options={"rounds": 3, "difficulty": "easy"},
        )
        targets = list(engine.state.mode_state["round_targets"])
        self.assertEqual(1, len(targets))
        target = targets[0]

        for seq in range(3):
            self.assertEqual(
                target["label"],
                engine.state.mode_state["target"]["label"],
            )
            engine.handle_event({"type": "miss", "score": 0, "seq": seq})
        engine.continue_turn()

        self.assertEqual(targets, engine.state.mode_state["round_targets"])
        self.assertEqual(target, engine.state.mode_state["target"])

    def test_easy_target_rush_accepts_every_ring_of_the_target_number(self):
        engine = GameEngine()
        engine.reset(
            "target_rush",
            ["Ada"],
            options={"rounds": 3, "difficulty": "easy"},
        )
        target = engine.state.mode_state["target"]

        engine.handle_event({
            "type": "hit",
            "seq": 240,
            "field": target["field"],
            "ring": "triple",
            "multiplier": 3,
            "score": target["field"] * 3,
            "label": f"T{target['field']}",
        })

        self.assertEqual(50, engine.state.players[0].score)
        self.assertEqual(target, engine.state.mode_state["target"])
        overlay = engine.state.as_dict()["overlay"]
        self.assertEqual(
            {"single_inner", "triple", "single_outer", "double"},
            {item["ring"] for item in overlay["targets"]},
        )

    def test_normal_target_rush_sequence_repeats_for_every_player(self):
        engine = GameEngine()
        engine.reset(
            "target_rush",
            ["Ada", "Bob"],
            options={"rounds": 3, "difficulty": "normal"},
        )
        targets = list(engine.state.mode_state["round_targets"])

        for seq in range(3):
            self.assertEqual(targets[seq], engine.state.mode_state["target"])
            engine.handle_event({"type": "miss", "score": 0, "seq": seq})
        engine.continue_turn()

        self.assertEqual(targets, engine.state.mode_state["round_targets"])
        self.assertEqual(targets[0], engine.state.mode_state["target"])

    def test_darts_bingo_uses_the_same_card_for_every_player(self):
        engine = GameEngine()
        engine.reset("darts_bingo", ["Ada", "Bob"])

        ada, bob = engine.state.players
        self.assertEqual(
            [
                (cell["task"], cell["label"])
                for cell in ada.marks.values()
            ],
            [
                (cell["task"], cell["label"])
                for cell in bob.marks.values()
            ],
        )
        self.assertIsNot(ada.marks, bob.marks)

    def test_darts_bingo_waits_for_equal_attempts_after_first_bingo(self):
        engine = GameEngine()
        engine.reset("darts_bingo", ["Ada", "Bob"])
        ada, bob = engine.state.players
        for player in (ada, bob):
            player.marks = {
                str(index): {
                    "task": "field_20",
                    "label": "Any 20",
                    "done": index not in (0, 1, 2),
                }
                for index in range(9)
            }

        event = hit(20, 70, field=20, label="S20")
        event["ring"] = "single_outer"
        engine.handle_event(event)
        self.assertEqual("hold", engine.state.status)
        self.assertEqual([ada.id], engine.state.mode_state["bingo_candidates"])

        engine.continue_turn()
        engine.handle_event(event | {"seq": 71})
        self.assertEqual("finished", engine.state.status)
        self.assertEqual("draw", engine.state.result_type)
        self.assertEqual({ada.id, bob.id}, set(engine.state.winner_ids))

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

    def test_simon_sequence_is_identical_for_every_player_in_a_round(self):
        engine = GameEngine()
        engine.reset(
            "simon_says",
            ["Ada", "Bob"],
            options={"rounds": 5, "difficulty": "easy"},
        )
        sequence = list(engine.state.mode_state["sequence"])
        target = dict(sequence[0])
        target_event = {
            "type": "hit",
            "seq": 220,
            "field": target["fields"][0],
            "ring": "single_outer",
            "multiplier": 1,
            "score": target["fields"][0],
            "label": f"S{target['fields'][0]}",
        }

        engine.handle_event(target_event)
        engine.continue_turn()
        self.assertEqual(sequence, engine.state.mode_state["sequence"])
        self.assertEqual(0, engine.state.mode_state["position"])

        engine.handle_event({**target_event, "seq": 221})
        engine.continue_turn()
        self.assertEqual(2, engine.state.round_number)
        self.assertEqual(2, len(engine.state.mode_state["sequence"]))

    def test_simon_difficulties_split_board_into_equal_contiguous_zones(self):
        expected = {
            "very_easy": (4, 5),
            "easy": (5, 4),
            "normal": (10, 2),
            "hard": (20, 1),
        }
        board_order = [20, 1, 18, 4, 13, 6, 10, 15, 2, 17, 3, 19, 7, 16, 8, 11, 14, 9, 12, 5]
        for difficulty, (zone_count, fields_per_zone) in expected.items():
            with self.subTest(difficulty=difficulty):
                engine = GameEngine()
                engine.reset(
                    "simon_says",
                    ["Ada"],
                    options={"rounds": 5, "difficulty": difficulty},
                )
                self.assertEqual(zone_count, engine.state.mode_state["zone_count"])
                target = engine.state.mode_state["sequence"][0]
                self.assertEqual(fields_per_zone, len(target["fields"]))
                start = (target["zone"] - 1) * fields_per_zone
                self.assertEqual(
                    board_order[start:start + fields_per_zone],
                    target["fields"],
                )

    def test_simon_accepts_any_ring_in_the_target_number_group(self):
        engine = GameEngine()
        engine.reset(
            "simon_says",
            ["Ada"],
            options={"rounds": 5, "difficulty": "easy"},
        )
        field = engine.state.mode_state["sequence"][0]["fields"][0]

        engine.handle_event({
            "type": "hit",
            "seq": 230,
            "field": field,
            "ring": "double",
            "multiplier": 2,
            "score": field * 2,
            "label": f"D{field}",
        })

        self.assertEqual(25, engine.state.players[0].score)
        self.assertIn("Sequenz geschafft", engine.state.message)

    def test_simon_bull_is_a_joker_for_every_target(self):
        engine = GameEngine()
        engine.reset(
            "simon_says",
            ["Ada"],
            options={"rounds": 5, "difficulty": "hard"},
        )

        engine.handle_event({
            "type": "hit",
            "seq": 231,
            "field": 25,
            "ring": "single_bull",
            "multiplier": 1,
            "score": 25,
            "label": "SBull",
        })

        self.assertEqual(25, engine.state.players[0].score)
        self.assertIn("Sequenz geschafft", engine.state.message)

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
