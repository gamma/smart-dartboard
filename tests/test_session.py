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
        self.controller.end_session()
        state = self.controller.public_state()
        self.assertEqual("session_summary", state["screen"])
        self.assertEqual("finished", state["session"]["status"])


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
