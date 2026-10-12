"""The per-regime roll-up: pure-campaign cost metrics, row-split packet metrics, gaps and the mixed summary."""

import unittest

from reporting.rollup import rollup


def _row(pid, regime, first_pass=None, accepted=True, retries=(), escalated=None, cycle=None):
    return {
        "id": pid,
        "regime": regime,
        "accepted": accepted,
        "first_pass": first_pass,
        "retries": list(retries),
        "escalated": escalated,
        "cycle_hours": cycle,
    }


def _records():
    return {
        "old-a": {
            "regime": "old",
            "cost": {"total_usd": 10.0, "by_model_tier": {"opus": 8.0, "sonnet": 2.0}},
            "prs": {
                "count": 3,
                "merged": 2,
                "per_pr": [
                    {"pr": 1, "usd": 3.0, "merged": True, "ci_first_pass": True, "ci_runs": 1},
                    {"pr": 2, "usd": 5.0, "merged": True, "ci_first_pass": False, "ci_runs": 2},
                    {"pr": 3, "usd": 2.0, "merged": False, "ci_first_pass": True, "ci_runs": 1},
                ],
            },
            "accepted": {"packets": 2, "cost_per_accepted_usd": 5.0},
            "escaped_defects": {"count": 1},
            "overstep": {"recoverable_usd": {"low": 1.0, "high": 2.0}},
            "packet_rows": [
                _row("CM-01", "old", first_pass=True, cycle=4.0),
                _row("CM-02", "old", first_pass=False, retries=["ci"], cycle=8.0),
            ],
        },
        "old-b": {
            "regime": "old",
            "cost": {"total_usd": 6.0, "by_model_tier": {"opus": 6.0}},
            "prs": {
                "count": 2,
                "merged": 1,
                "per_pr": [
                    {"pr": 4, "usd": 4.0, "merged": True, "ci_first_pass": True, "ci_runs": 1},
                    {"pr": 5, "usd": 50.0, "merged": False, "ci_first_pass": None, "ci_runs": 0},
                ],
            },
            "accepted": {"packets": 1, "cost_per_accepted_usd": 6.0},
            "escaped_defects": {"count": 0},
            "overstep": {"recoverable_usd": {"low": None, "high": 3.0}},
            "packet_rows": [_row("CM-03", "old", first_pass=True, cycle=6.0)],
        },
        "haiku-a": {
            "regime": "haiku",
            "cost": {"total_usd": 4.0, "by_model_tier": {"haiku": 3.0, "sonnet": 1.0}},
            "prs": {
                "count": 2,
                "merged": 2,
                "per_pr": [
                    {"pr": 6, "usd": 1.0, "merged": True, "ci_first_pass": True, "ci_runs": 1},
                    {"pr": 7, "usd": 3.0, "merged": True, "ci_first_pass": True, "ci_runs": 1},
                ],
            },
            "accepted": {"packets": 2, "cost_per_accepted_usd": 2.0},
            "escaped_defects": {"count": 1},
            "overstep": {"recoverable_usd": {"low": 0.5, "high": 0.5}},
            "packet_rows": [
                _row("CM-04", "haiku", first_pass=True, escalated=False, cycle=2.0),
                _row("CM-05", "haiku", first_pass=False, retries=["review", "ci"], escalated=True, cycle=3.0),
            ],
        },
        "mixed-a": {
            "regime": "mixed",
            "cost": {"total_usd": 100.0, "by_model_tier": {"opus": 60.0, "haiku": 40.0}},
            "prs": {
                "count": 1,
                "merged": 1,
                "per_pr": [{"pr": 8, "usd": 100.0, "merged": True, "ci_first_pass": False, "ci_runs": 2}],
            },
            "accepted": {"packets": 4, "cost_per_accepted_usd": 25.0},
            "escaped_defects": {"count": 0},
            "overstep": {"recoverable_usd": {"low": 9.0, "high": 9.0}},
            "packet_rows": [
                _row("CM-06", "old", first_pass=True, cycle=5.0),
                _row("CM-07", "old", first_pass=True, cycle=5.0),
                _row("CM-08", "haiku", first_pass=True, escalated=False, cycle=1.0),
                _row("CM-09", "haiku", first_pass=True, escalated=False, cycle=1.0),
            ],
        },
    }


class RollupCostTest(unittest.TestCase):
    def test_cost_metrics_use_pure_campaigns_only(self):
        old = rollup(_records(), "old")
        self.assertEqual(old["campaigns"], 2)
        self.assertEqual(old["total_usd"], 16.0)
        self.assertEqual(old["cost_share_by_tier"], {"opus": 0.88, "sonnet": 0.12})
        self.assertEqual(old["cost_per_accepted_usd"], 16.0 / 3)
        self.assertEqual(old["overstep_recoverable_usd"], {"low": 1.0, "high": 5.0})

        haiku = rollup(_records(), "haiku")
        self.assertEqual(haiku["campaigns"], 1)
        self.assertEqual(haiku["total_usd"], 4.0)
        self.assertEqual(haiku["cost_share_by_tier"], {"haiku": 0.75, "sonnet": 0.25})

    def test_cost_per_pr_only_merged(self):
        old = rollup(_records(), "old")
        # merged old PRs cost 3, 5 and 4; the unmerged 2 and 50 are ignored
        self.assertEqual(old["cost_per_pr"]["mean"], 4.0)
        self.assertEqual(old["cost_per_pr"]["p50"], 4.0)
        self.assertEqual(old["merged_prs"], 3)
        self.assertEqual(old["escaped_defects_per_merged_pr"], 0.33)
