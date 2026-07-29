from __future__ import annotations

import unittest

try:
    from pydantic import ValidationError
    from sdb_dartboard.validation import DartEventRequest, same_origin
except ModuleNotFoundError as exc:
    raise unittest.SkipTest("API validation tests require project dependencies") from exc


class BrowserBoundaryTests(unittest.TestCase):
    def test_same_origin_browser_and_native_clients_are_allowed(self):
        self.assertTrue(same_origin(None, "dartboard.local:8000"))
        self.assertTrue(
            same_origin(
                "http://dartboard.local:8000",
                "dartboard.local:8000",
            )
        )

    def test_cross_origin_browser_is_rejected(self):
        self.assertFalse(
            same_origin("https://evil.example", "dartboard.local:8000")
        )


class DartEventValidationTests(unittest.TestCase):
    def test_client_score_and_label_are_recomputed(self):
        event = DartEventRequest(
            type="hit",
            seq=7,
            field=20,
            ring="triple",
            multiplier=3,
            score=1,
            label="HACK",
        ).normalized()
        self.assertEqual(60, event["score"])
        self.assertEqual("T20", event["label"])

    def test_ring_multiplier_mismatch_is_rejected(self):
        with self.assertRaises(ValidationError):
            DartEventRequest(
                type="hit",
                field=20,
                ring="triple",
                multiplier=2,
            )

    def test_bull_geometry_is_rejected_for_number_ring(self):
        with self.assertRaises(ValidationError):
            DartEventRequest(type="hit", field=25, ring="single_outer")

    def test_miss_is_canonical(self):
        event = DartEventRequest(
            type="miss",
            seq=8,
            score=42,
            label="fake",
        ).normalized()
        self.assertEqual(
            {"type": "miss", "seq": 8, "label": "MISS", "score": 0},
            event,
        )


if __name__ == "__main__":
    unittest.main()
