"""The per-regime roll-up: pure-campaign cost metrics, row-split packet metrics, gaps and the mixed summary."""

import unittest

from reporting.rollup import mixed_summary, rollup


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


class RollupPacketTest(unittest.TestCase):
    def test_packet_metrics_split_mixed_by_row_regime(self):
        # the mixed campaign's two old rows count toward old, its two haiku rows toward haiku
        old = rollup(_records(), "old")
        self.assertEqual(old["packets"], 5)
        self.assertEqual(old["first_pass_rate"], 0.8)
        haiku = rollup(_records(), "haiku")
        self.assertEqual(haiku["packets"], 4)
        self.assertEqual(haiku["first_pass_rate"], 0.75)
        self.assertEqual(haiku["retries_per_accepted"], 0.5)

    def test_overstep_counts_mixed_post_switch_campaigns(self):
        records = {
            "haiku-a": {"regime": "haiku", "overstep": {"recoverable_usd": {"low": 1.0, "high": 2.0}}},
            "mixed-a": {"regime": "mixed", "overstep": {"recoverable_usd": {"low": 2.0, "high": 5.0}}},
            "old-a": {"regime": "old", "overstep": {"recoverable_usd": {"low": None, "high": None}}},
        }
        self.assertEqual(rollup(records, "haiku")["overstep_recoverable_usd"], {"low": 3.0, "high": 7.0})
        self.assertIsNone(rollup(records, "old")["overstep_recoverable_usd"])

    def test_zero_denominator_is_none_with_gap(self):
        records = {"x": {"regime": "haiku", "packet_rows": [_row("CM-10", "haiku", accepted=False, first_pass=True)]}}
        out = rollup(records, "haiku")
        self.assertIsNone(out["first_pass_rate"])
        self.assertIn("first_pass_rate_gap", out)
        self.assertIsNone(out["retries_per_accepted"])
        self.assertIn("retries_per_accepted_gap", out)

    def test_old_escalation_is_gap(self):
        old = rollup(_records(), "old")
        self.assertIsNone(old["escalation_rate"])
        self.assertEqual(old["escalation_rate_gap"], "n/a: nothing was dispatched to Haiku")
        self.assertEqual(rollup(_records(), "haiku")["escalation_rate"], 0.25)

    def test_mixed_summary(self):
        self.assertEqual(mixed_summary(_records()), {"campaigns": 1, "names": ["mixed-a"]})
