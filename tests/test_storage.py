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

    def test_cooperative_win_awards_every_winner(self):
        ada = self.store.create_player("Ada")
        bob = self.store.create_player("Bob")
        session = self.store.start_session([ada["id"], bob["id"]])
        game_id = self.store.start_game(session["id"], "space_defender", {})
        self.store.finish_game(game_id, winner_ids=[ada["id"], bob["id"]])

        stats = {
            item["id"]: item
            for item in self.store.session_statistics(session["id"])
        }
        self.assertEqual(1, stats[ada["id"]]["wins"])
        self.assertEqual(1, stats[bob["id"]]["wins"])
        self.assertEqual(3, stats[ada["id"]]["session_points"])
        self.assertEqual(3, stats[bob["id"]]["session_points"])

    def test_database_health_probe(self):
        self.assertTrue(self.store.ping())

    def test_history_heatmap_and_replay(self):
        ada = self.store.create_player("Ada", "nova", "#ff00aa")
        session = self.store.start_session([ada["id"]])
        game_id = self.store.start_game(
            session["id"],
            "target_rush",
            {"rounds": 3},
            players=[ada],
            ruleset_version=2,
            app_version="test-suite",
            initial_state={"players": [{"id": ada["id"], "score": 0}]},
        )
        task = {
            "round_number": 1,
            "dart_in_turn": 1,
            "targets": [{"field": 20, "rings": ["triple"]}],
        }
        frame = {
            "round_number": 1,
            "players": [{"id": ada["id"], "name": "Ada", "score": 50}],
            "last_event": {"type": "hit", "field": 20, "ring": "triple"},
        }
        event_id = self.store.record_game_event(
            game_id,
            "throw",
            player_id=ada["id"],
            source="ble",
            payload={"type": "hit", "label": "T20", "field": 20, "ring": "triple"},
            task=task,
            frame=frame,
        )
        self.store.record_throw(
            game_id,
            7,
            ada["id"],
            {
                "type": "hit",
                "label": "T20",
                "score": 60,
                "field": 20,
                "ring": "triple",
                "multiplier": 3,
            },
            50,
            round_number=1,
            dart_in_turn=1,
            mode_points=50,
            outcome="success",
            source="ble",
            task=task,
            event_id=event_id,
        )
        self.store.finish_game(
            game_id,
            ada["id"],
            result_type="individual_win",
            finish_reason="target_complete",
            final_state=frame,
            final_scores={ada["id"]: 50},
        )

        self.assertNotIn("language", self.store.get_session(session["id"]))
        detail = self.store.game_detail(game_id)
        self.assertEqual(50, detail["players"][0]["final_score"])
        self.assertEqual("triple", detail["throws"][0]["ring"])
        self.assertEqual(task, detail["throws"][0]["task"])
        replay = self.store.game_replay(game_id)
        self.assertEqual("T20", replay["events"][0]["payload"]["label"])
        self.assertEqual(frame, replay["events"][0]["frame"])
        heatmap = self.store.heatmap(player_id=ada["id"])
        self.assertEqual(1, heatmap["total_darts"])
        self.assertEqual(1, heatmap["segments"][0]["successes"])
        recommendation = self.store.training_recommendations(ada["id"])
        self.assertEqual(20, recommendation["recommendations"][0]["field"])
        archive = self.store.export_data()
        self.assertEqual(2, archive["schema_version"])
        self.assertEqual("Ada", archive["players"][0]["name"])
        self.assertEqual(game_id, archive["games"][0]["detail"]["id"])

    def test_test_games_are_separate_but_can_be_requested(self):
        ada = self.store.create_player("Ada")
        session = self.store.start_session([ada["id"]])
        game_id = self.store.start_game(
            session["id"],
            "countup",
            {},
            players=[ada],
            environment="test",
        )
        self.store.record_throw(
            game_id,
            1,
            ada["id"],
            {
                "type": "hit",
                "label": "S20",
                "score": 20,
                "field": 20,
                "ring": "single_outer",
                "multiplier": 1,
            },
            20,
        )
        self.store.finish_game(game_id, ada["id"])

        production = self.store.statistics(completed_only=True)
        with_test = self.store.statistics(
            completed_only=True,
            include_nonproduction=True,
        )
        self.assertEqual(0, production[0]["games"])
        self.assertEqual(1, with_test[0]["games"])
        self.assertEqual(0, self.store.heatmap()["total_darts"])
        self.assertEqual(
            1,
            self.store.heatmap(include_nonproduction=True)["total_darts"],
        )


if __name__ == "__main__":
    unittest.main()
