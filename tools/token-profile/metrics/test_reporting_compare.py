"""The old-versus-Haiku delta, the low-n flag and the summary document."""

import unittest

from reporting.compare import delta, low_n, summary_doc


def _side(campaigns=5, packets=10, **metrics):
    return {"campaigns": campaigns, "packets": packets, **metrics}


def _row(pid, regime, first_pass=True, cycle=4.0):
    return {"id": pid, "regime": regime, "accepted": True, "first_pass": first_pass, "retries": [], "escalated": None, "cycle_hours": cycle}


def _records():
    return {
        "old-a": {
            "regime": "old",
            "cost": {"total_usd": 10.0, "by_model_tier": {"opus": 10.0}},
            "prs": {"count": 1, "merged": 1, "per_pr": [{"pr": 1, "usd": 4.0, "merged": True, "ci_first_pass": True, "ci_runs": 1}]},
            "accepted": {"packets": 2, "cost_per_accepted_usd": 5.0},
            "escaped_defects": {"count": 0},
            "overstep": {"recoverable_usd": {"low": 0.0, "high": 0.0}},
            "packet_rows": [_row("CM-01", "old"), _row("CM-02", "old", first_pass=False)],
        },
        "haiku-a": {
            "regime": "haiku",
            "cost": {"total_usd": 4.0, "by_model_tier": {"haiku": 4.0}},
            "prs": {"count": 1, "merged": 1, "per_pr": [{"pr": 2, "usd": 2.0, "merged": True, "ci_first_pass": True, "ci_runs": 1}]},
            "accepted": {"packets": 1, "cost_per_accepted_usd": 4.0},
            "escaped_defects": {"count": 0},
            "overstep": {"recoverable_usd": {"low": 0.0, "high": 0.0}},
            "packet_rows": [_row("CM-03", "haiku")],
        },
        "mixed-a": {"regime": "mixed", "cost": {"total_usd": 1.0}, "packet_rows": []},
    }


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


class SummaryDocTest(unittest.TestCase):
    def test_summary_doc_always_has_low_n(self):
        for records in ({}, _records()):
            comparison = summary_doc(records)["comparison"]
            self.assertIn("low_n", comparison)
            self.assertIs(comparison["low_n"], True)

    def test_summary_doc_schema_and_regimes(self):
        doc = summary_doc(_records())
        self.assertEqual(doc["schema"], "cimmeria-campaign-metrics/1")
        self.assertEqual(doc["regimes"]["old"]["campaigns"], 1)
        self.assertEqual(doc["regimes"]["old"]["total_usd"], 10.0)
        self.assertEqual(doc["regimes"]["haiku"]["total_usd"], 4.0)
        self.assertEqual(doc["mixed"], {"campaigns": 1, "names": ["mixed-a"]})
        comparison = doc["comparison"]
        self.assertEqual(comparison["n"], {"old": {"campaigns": 1, "packets": 2}, "haiku": {"campaigns": 1, "packets": 1}})
        self.assertEqual(comparison["delta"]["cost_per_accepted_usd"], -1.0)
        self.assertIn("negative delta is better", comparison["note"])
