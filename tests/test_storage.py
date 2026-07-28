from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

from sdb_dartboard.storage import DartboardStore


class StorageTests(unittest.TestCase):
    def setUp(self):
        self.tempdir = tempfile.TemporaryDirectory()
        self.store = DartboardStore(Path(self.tempdir.name) / "dartboard.db")

    def tearDown(self):
        self.store.close()
        self.tempdir.cleanup()

    def test_session_game_throw_and_statistics_are_persisted(self):
        ada = self.store.create_player("Ada", "nova", "#ff00aa")
        bob = self.store.create_player("Bob")
        session = self.store.start_session([ada["id"], bob["id"]])
        game_id = self.store.start_game(session["id"], "countup", {})
        self.store.record_throw(
            game_id,
            1,
            ada["id"],
            {"type": "hit", "label": "T20", "score": 60},
            60,
        )
        self.store.finish_game(game_id, ada["id"])

        active = self.store.active_session()
        self.assertEqual(["Ada", "Bob"], [player["name"] for player in active["players"]])
        stats = {item["name"]: item for item in self.store.statistics()}
        self.assertEqual(1, stats["Ada"]["games"])
        self.assertEqual(1, stats["Ada"]["wins"])
        self.assertEqual(180, stats["Ada"]["three_dart_average"])
        self.assertEqual(0, stats["Bob"]["wins"])

    def test_unknown_session_player_is_rejected(self):
        with self.assertRaises(ValueError):
            self.store.start_session(["missing"])


if __name__ == "__main__":
    unittest.main()
