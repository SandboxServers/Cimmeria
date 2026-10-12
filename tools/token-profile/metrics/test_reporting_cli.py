"""Own-session cost applied to the campaign (CM-08k): the apply step, then the `report` subcommand end to end."""

import copy
import unittest
from pathlib import Path

from reporting import self_apply
from reporting.inputs import load_inputs

FIXTURES = Path(__file__).resolve().parent / "fixtures" / "inputs"
SELF = {"requests": 3, "unpriced": 0, "total_usd": 5.0, "by_tier": {"opus": 4.0, "haiku": 1.0},
        "by_role": {"coordinator": 5.0}, "sessions": 2}


class ApplySelfCostTest(unittest.TestCase):
    def test_apply_adds_to_unattributed_and_counts_sessions(self):
        inputs = load_inputs(FIXTURES)
        before = inputs["cost"]["campaigns"]["alpha-old"]["cost"]
        out = self_apply.apply_self_cost(inputs, SELF, campaign="alpha-old")
        cost = out["cost"]["campaigns"]["alpha-old"]["cost"]
        self.assertAlmostEqual(cost["total_usd"], before["total_usd"] + 5.0)
        self.assertAlmostEqual(cost["unattributed_in_campaign_usd"], before["unattributed_in_campaign_usd"] + 5.0)
        self.assertAlmostEqual(cost["by_model_tier"]["opus"], before["by_model_tier"]["opus"] + 4.0)
        self.assertAlmostEqual(cost["by_role"]["coordinator"], before["by_role"]["coordinator"] + 5.0)
        self.assertEqual(cost["self_sessions"], 2)

    def test_absent_campaign_is_created_with_zero_pr_cost(self):
        inputs = load_inputs(FIXTURES)
        out = self_apply.apply_self_cost(inputs, SELF)
        record = out["cost"]["campaigns"]["campaign-metrics"]
        self.assertAlmostEqual(record["cost"]["total_usd"], 5.0)
        self.assertEqual(record["cost"]["self_sessions"], 2)
        self.assertEqual(record["prs"], {"count": 0, "merged": 0, "cost_per_pr": {"mean": None, "p50": None, "p90": None},
                                         "per_pr": []})
        self.assertEqual(record["accepted"], {"packets": 0, "cost_per_accepted_usd": None,
                                              "accepted_gap": "no packet table"})

    def test_apply_does_not_mutate_inputs(self):
        inputs = load_inputs(FIXTURES)
        snapshot = copy.deepcopy(inputs)
        self_apply.apply_self_cost(inputs, SELF)
        self.assertEqual(inputs, snapshot)


if __name__ == "__main__":
    unittest.main()
