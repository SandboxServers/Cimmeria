import unittest

from .stats import distribution, percentile, top_share


class StatsTest(unittest.TestCase):
    def test_percentiles_interpolate_between_ranks(self):
        vals = list(range(1, 101))  # 1..100
        self.assertAlmostEqual(percentile(vals, 50), 50.5)
        self.assertAlmostEqual(percentile(vals, 90), 90.1)
        self.assertAlmostEqual(percentile(vals, 99), 99.01)
        self.assertEqual(percentile([7], 99), 7)
        self.assertIsNone(percentile([], 50))

    def test_distribution_has_the_tail_not_only_a_mean(self):
        d = distribution([1, 1, 1, 1, 100, None])
        self.assertEqual(set(d), {"n", "sum", "mean", "p50", "p75", "p90", "p95", "p99", "max"})
        self.assertEqual(d["n"], 5)
        self.assertEqual(d["max"], 100)
        self.assertEqual(d["p50"], 1)
        self.assertEqual(distribution([])["max"], None)

    def test_top_share(self):
        self.assertAlmostEqual(top_share([90] + [1] * 10, 0.10), 90 / 100)
        self.assertEqual(top_share([], 0.1), 0.0)


if __name__ == "__main__":
    unittest.main()
