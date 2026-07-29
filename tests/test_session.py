from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

from sdb_dartboard.session import EventPipeline, SessionController


class SessionControllerTests(unittest.TestCase):
    def setUp(self):
        self.tempdir = tempfile.TemporaryDirectory()
        self.database = Path(self.tempdir.name) / "dartboard.db"
        self.controller = SessionController(self.database)

    def tearDown(self):
        self.controller.close()
        self.tempdir.cleanup()

    def _start_game(self):
        ada = self.controller.create_player("Ada", "nova", "#ff00aa")
        bob = self.controller.create_player("Bob", "comet", "#28e7ff")
        self.controller.start_session([ada["id"], bob["id"]])
        self.controller.prepare_game("countup", {})
        self.controller.start_game()
        self.controller.set_screen("playing")
        return ada, bob

    def test_screen_flow_and_throw_persistence(self):
        ada, _ = self._start_game()
        self.controller.process_event(
            {"type": "hit", "label": "T20", "score": 60, "seq": 7, "field": 20, "multiplier": 3}
        )
        state = self.controller.public_state()
        self.assertEqual("playing", state["screen"])
        self.assertEqual(60, state["game"]["players"][0]["score"])
        stats = {item["id"]: item for item in state["statistics"]}
        self.assertEqual(1, stats[ada["id"]]["darts"])

    def test_runtime_checkpoint_survives_restart_with_undo(self):
        self._start_game()
        self.controller.process_event(
            {"type": "hit", "label": "D20", "score": 40, "seq": 1, "field": 20, "multiplier": 2}
        )
        self.controller.close()
        self.controller = SessionController(self.database)
        self.assertEqual(40, self.controller.engine.state.players[0].score)
        self.assertEqual("playing", self.controller.screen)
        self.controller.undo()
        self.assertEqual(0, self.controller.engine.state.players[0].score)
        stats = self.controller.public_state()["statistics"]
        self.assertEqual(0, stats[0]["darts"])

    def test_finishing_session_keeps_summary_statistics(self):
        self._start_game()
        self.controller.abort_game()
        self.controller.end_session()
        state = self.controller.public_state()
        self.assertEqual("session_summary", state["screen"])
        self.assertEqual("finished", state["session"]["status"])

    def test_session_cannot_end_directly_from_running_game(self):
        self._start_game()
        with self.assertRaisesRegex(ValueError, "game selection"):
            self.controller.end_session()

    def test_calibration_reset_is_centered_and_square_for_projector(self):
        for width, height in ((1600, 900), (900, 1600), (1024, 768)):
            with self.subTest(width=width, height=height):
                self.controller.report_projector_geometry(width, height)
                calibration = self.controller.reset_calibration()
                corners = calibration["corners"]
                projected_width = (corners[1]["x"] - corners[0]["x"]) * width
                projected_height = (corners[3]["y"] - corners[0]["y"]) * height
                self.assertAlmostEqual(projected_width, projected_height)
                self.assertAlmostEqual(min(width, height) * 0.9, projected_width)
                self.assertAlmostEqual(
                    0.5, (corners[0]["x"] + corners[1]["x"]) / 2
                )
                self.assertAlmostEqual(
                    0.5, (corners[0]["y"] + corners[3]["y"]) / 2
                )
                self.assertEqual(1.0, calibration["scale"])
                self.assertEqual(0.0, calibration["offset_x"])
                self.assertEqual(0.0, calibration["offset_y"])

    def test_projector_sound_setting_and_status_are_persisted(self):
        self.controller.set_sound_enabled(True)
        self.assertEqual(
            {"enabled": True, "status": "starting"},
            self.controller.public_state()["sound"],
        )
        self.controller.report_sound_status("ready")
        self.controller.close()
        self.controller = SessionController(self.database)
        self.assertEqual(
            {"enabled": True, "status": "starting"},
            self.controller.public_state()["sound"],
        )
        self.controller.set_sound_enabled(False)
        self.assertEqual(
            {"enabled": False, "status": "disabled"},
            self.controller.public_state()["sound"],
        )

    def test_finished_game_cannot_be_returned_to_playing_screen(self):
        self._start_game()
        self.controller.engine.state.status = "finished"
        with self.assertRaises(ValueError):
            self.controller.set_screen("playing")

    def test_multiple_games_accumulate_in_the_same_session(self):
        player = self.controller.create_player("Ada", "nova", "#ff00aa")
        self.controller.start_session([player["id"]])
        for game_number in range(2):
            self.controller.prepare_game("countup", {"rounds": 5})
            self.controller.start_game()
            self.controller.engine.state.round_number = 5
            self.controller.set_screen("playing")
            for dart in range(3):
                self.controller.process_event(
                    {
                        "type": "hit",
                        "label": "S20",
                        "score": 20,
                        "seq": game_number * 3 + dart,
                        "field": 20,
                        "multiplier": 1,
                    }
                )
            self.assertEqual("game_result", self.controller.screen)
            if game_number == 0:
                self.controller.next_game()
        stats = self.controller.public_state()["statistics"][0]
        self.assertEqual(2, stats["games"])
        self.assertEqual(2, stats["wins"])
        self.assertEqual(6, stats["darts"])
        session_stats = self.controller.public_state()["session_statistics"][0]
        self.assertEqual(6, session_stats["session_points"])

    def test_draw_finishes_without_awarding_session_points(self):
        self._start_game()
        self.controller.selected_options = {"rounds": 5}
        self.controller.engine.state.options = {"rounds": 5}
        self.controller.engine.state.round_number = 5
        for seq in range(3):
            self.controller.process_event(
                {
                    "type": "hit",
                    "label": "S20",
                    "score": 20,
                    "seq": 700 + seq,
                    "field": 20,
                    "ring": "single_outer",
                    "multiplier": 1,
                }
            )
        self.controller.continue_turn()
        for seq in range(3):
            self.controller.process_event(
                {
                    "type": "hit",
                    "label": "S20",
                    "score": 20,
                    "seq": 710 + seq,
                    "field": 20,
                    "ring": "single_outer",
                    "multiplier": 1,
                }
            )
        state = self.controller.public_state()
        self.assertEqual("game_result", state["screen"])
        self.assertIsNone(state["game"]["winner_id"])
        self.assertTrue(
            all(player["session_points"] == 0 for player in state["session_statistics"])
        )

    def test_correction_rewrites_persisted_statistics(self):
        ada, _ = self._start_game()
        self.controller.process_event(
            {"type": "hit", "label": "T20", "score": 60, "seq": 1, "field": 20, "multiplier": 3}
        )
        self.controller.process_event(
            {"type": "hit", "label": "S20", "score": 20, "seq": 2, "field": 20, "multiplier": 1}
        )

        self.controller.correct_turn_throw(
            0,
            {"type": "hit", "label": "D20", "score": 40, "field": 20, "multiplier": 2},
        )

        self.assertEqual(60, self.controller.engine.state.players[0].score)
        stats = {
            item["id"]: item for item in self.controller.public_state()["statistics"]
        }
        self.assertEqual(60, stats[ada["id"]]["total_points"])
        self.assertEqual(2, stats[ada["id"]]["darts"])

    def test_finishing_game_via_plugin_action_updates_session_and_screen(self):
        player = self.controller.create_player("Ada", "robot", "#28e7ff")
        self.controller.start_session([player["id"]])
        self.controller.prepare_game("risk_it", {"rounds": 3})
        self.controller.start_game()
        self.controller.engine.state.round_number = 3
        self.controller.set_screen("playing")
        self.controller.process_event(
            {
                "type": "hit",
                "label": "T20",
                "score": 60,
                "seq": 501,
                "field": 20,
                "ring": "triple",
                "multiplier": 3,
            }
        )
        self.controller.game_action("bank")
        self.assertEqual("game_result", self.controller.screen)
        self.assertEqual("finished", self.controller.engine.state.status)
        self.assertEqual(
            "finished", self.controller.store.get_game(self.controller.game_id)["status"]
        )

    def test_aborted_game_returns_to_selection_and_is_not_counted(self):
        player = self.controller.create_player("Ada", "robot", "#28e7ff")
        self.controller.start_session([player["id"]])
        self.controller.prepare_game("countup", {"rounds": 5})
        self.controller.start_game()
        self.controller.engine.state.round_number = 5
        aborted_game_id = self.controller.game_id
        self.controller.set_screen("playing")
        self.controller.process_event(
            {
                "type": "hit",
                "label": "T20",
                "score": 60,
                "seq": 601,
                "field": 20,
                "ring": "triple",
                "multiplier": 3,
            }
        )

        self.controller.abort_game()

        state = self.controller.public_state()
        self.assertEqual("game_select", state["screen"])
        self.assertIsNone(state["game_id"])
        self.assertEqual("idle", state["game"]["status"])
        self.assertEqual("aborted", self.controller.store.get_game(aborted_game_id)["status"])
        self.assertEqual(0, state["session_statistics"][0]["games"])
        self.assertEqual(0, state["session_statistics"][0]["wins"])
        self.assertEqual(0, state["session_statistics"][0]["darts"])
        self.assertEqual(0, state["session_statistics"][0]["session_points"])
        self.assertEqual(0, state["statistics"][0]["darts"])


class EventPipelineTests(unittest.IsolatedAsyncioTestCase):
    async def asyncSetUp(self):
        self.tempdir = tempfile.TemporaryDirectory()
        self.controller = SessionController(Path(self.tempdir.name) / "dartboard.db")
        player = self.controller.create_player("Ada", "nova", "#ff00aa")
        self.controller.start_session([player["id"]])
        self.controller.prepare_game("countup", {})
        self.controller.start_game()
        self.controller.set_screen("playing")
        self.pipeline = EventPipeline(self.controller)

    async def asyncTearDown(self):
        self.controller.close()
        self.tempdir.cleanup()

    async def test_duplicate_ble_packet_is_ignored(self):
        event = {
            "type": "hit",
            "label": "T20",
            "score": 60,
            "seq": 42,
            "raw": "2a00000005000b000214",
            "field": 20,
            "multiplier": 3,
        }
        self.assertTrue(await self.pipeline.process(event, source="ble"))
        self.assertFalse(await self.pipeline.process(event, source="ble"))
        self.assertEqual(60, self.controller.engine.state.players[0].score)
        self.assertEqual(1, len(self.controller.engine.state.throws))

    async def test_test_events_are_not_deduplicated(self):
        event = {"type": "miss", "label": "MISS", "score": 0, "seq": 1}
        await self.pipeline.process(event, source="test")
        await self.pipeline.process(event, source="test")
        self.assertEqual(2, self.controller.engine.state.darts_in_turn)


if __name__ == "__main__":
    unittest.main()
