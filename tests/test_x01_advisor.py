import unittest

from sdb_dartboard.games.x01_advisor import x01_advice


class X01AdvisorTest(unittest.TestCase):
    def test_advice_stays_hidden_above_setup_range(self):
        for score in (301, 201):
            with self.subTest(score=score):
                advice = x01_advice(score, 3, "double")
                self.assertEqual(advice["status"], "none")
                self.assertIsNone(advice["primary"])

    def test_setup_advice_starts_at_200(self):
        advice = x01_advice(200, 3, "double")
        self.assertEqual(advice["status"], "setup")
        self.assertIsNotNone(advice["primary"])

    def test_checkout_40_double_out(self):
        advice = x01_advice(40, 3, "double")
        self.assertEqual(advice["status"], "checkout")
        self.assertEqual(advice["primary"]["label"], "D20")

    def test_checkout_170_double_out(self):
        advice = x01_advice(170, 3, "double")
        self.assertEqual(advice["status"], "checkout")
        self.assertEqual([dart["label"] for dart in advice["sequence"]], ["T20", "T20", "DBull"])

    def test_no_double_checkout_leave_one(self):
        self.assertEqual(x01_advice(2, 1, "double")["primary"]["label"], "D1")
        self.assertEqual(x01_advice(1, 3, "double")["status"], "none")

    def test_setup_when_no_finish_possible(self):
        advice = x01_advice(171, 3, "double")
        self.assertEqual(advice["status"], "setup")
        self.assertEqual([dart["label"] for dart in advice["sequence"]], ["T20", "T20", "S11"])
        self.assertEqual(advice["setup"]["leave"], 40)

    def test_standard_table_is_preferred(self):
        advice = x01_advice(82, 2, "double")
        self.assertEqual([dart["label"] for dart in advice["sequence"]], ["DBull", "D16"])


if __name__ == "__main__":
    unittest.main()
