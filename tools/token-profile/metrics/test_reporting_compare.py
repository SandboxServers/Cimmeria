"""The old-versus-Haiku delta and the low-n flag."""

import unittest

from reporting.compare import delta, low_n


def _side(campaigns=5, packets=10, **metrics):
    return {"campaigns": campaigns, "packets": packets, **metrics}


class DeltaTest(unittest.TestCase):
    def test_delta_is_haiku_minus_old(self):
        old = _side(cost_per_pr={"p50": 20.0, "mean": 22.0})
        haiku = _side(cost_per_pr={"p50": 9.0, "mean": 10.0})
        d = delta(old, haiku)
        self.assertEqual(d["cost_per_pr_p50"], -11.0)
        self.assertEqual(d["cost_per_pr_mean"], -12.0)

    def test_delta_none_when_one_side_missing(self):
        old = _side(first_pass_rate=0.5, cost_per_pr={"p50": 4.0, "mean": 4.0})
        haiku = _side(first_pass_rate=None, cost_per_pr=None)
        d = delta(old, haiku)
        self.assertIsNone(d["first_pass_rate"])
        self.assertIsNone(d["cost_per_pr_p50"])
        self.assertIsNone(d["cost_per_pr_mean"])
        self.assertIsNone(d["retries_per_accepted"])


class LowNTest(unittest.TestCase):
    def test_low_n_thresholds(self):
        self.assertTrue(low_n(_side(campaigns=4, packets=20), _side(campaigns=5, packets=10)))
        self.assertFalse(low_n(_side(campaigns=5, packets=10), _side(campaigns=5, packets=10)))
        self.assertTrue(low_n(_side(campaigns=5, packets=9), _side(campaigns=5, packets=10)))
