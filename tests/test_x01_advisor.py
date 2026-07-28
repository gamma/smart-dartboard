import unittest

from sdb_dartboard.games.x01_advisor import x01_advice


class X01AdvisorTest(unittest.TestCase):
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
        self.assertEqual(advice["primary"]["label"], "T20")
        self.assertEqual(advice["setup"]["leave"], 111)


if __name__ == "__main__":
    unittest.main()
