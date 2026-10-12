"""The GitHub and overstep sections of the campaign records, and the whole-report assemble (CM-08c)."""

import unittest
from pathlib import Path

from reporting import assemble, inputs

FIXTURES = Path(__file__).parent / "fixtures" / "inputs"


class GithubSectionTest(unittest.TestCase):
    def setUp(self):
        self.loaded = inputs.load_inputs(FIXTURES)

    def record(self, name):
        loaded = self.loaded
        return assemble.assemble_core(name, loaded["cost"], loaded["packets"], loaded["ledger"])

    def test_alpha_ci(self):
        # Alpha's overall CI is all green; a first-attempt rate built from `overall` would read 1.0.
        ci = self.record_with_github("alpha-old")["ci"]
        self.assertEqual(ci["first_attempt_rate"], 0.75)
        self.assertEqual(ci["overall_rate"], 1.0)
        self.assertEqual(ci["pr_runs_total"], 5)

    def test_beta_code_pr_rate(self):
        # 203 is not a code PR, so only 201 (true) and 202 (false) count.
        ci = self.record_with_github("beta-haiku")["ci"]
        self.assertEqual(ci["code_prs_first_attempt_rate"], 0.5)

    def test_per_pr_enriched(self):
        per_pr = {entry["pr"]: entry for entry in self.record_with_github("alpha-old")["prs"]["per_pr"]}
        self.assertFalse(per_pr[103]["ci_first_pass"])
        self.assertEqual(per_pr[103]["ci_runs"], 2)

    def test_alpha_escaped_defect(self):
        escaped = self.record_with_github("alpha-old")["escaped_defects"]
        self.assertEqual(escaped["count"], 1)
        self.assertEqual(escaped["rate_per_merged_pr"], 0.25)
        self.assertEqual(escaped["items"][0]["cites"], 102)

    def test_beta_escaped_defect_is_zero(self):
        escaped = self.record_with_github("beta-haiku")["escaped_defects"]
        self.assertEqual(escaped["count"], 0)
        self.assertEqual(escaped["rate_per_merged_pr"], 0.0)

    def test_no_ci_rows_is_gap(self):
        record = self.record("alpha-old")
        assemble.attach_github(record, {"prs": []})
        self.assertIsNone(record["ci"]["first_attempt_rate"])
        self.assertEqual(record["ci"]["first_attempt_rate_gap"], "no CI data")
        self.assertEqual(record["ci"]["pr_runs_total"], 0)
        self.assertEqual(record["escaped_defects"]["count"], 0)

    def record_with_github(self, name):
        record = self.record(name)
        assemble.attach_github(record, self.loaded["github"])
        return record


class OverstepSectionTest(unittest.TestCase):
    def setUp(self):
        self.loaded = inputs.load_inputs(FIXTURES)

    def record_with_overstep(self, name):
        loaded = self.loaded
        record = assemble.assemble_core(name, loaded["cost"], loaded["packets"], loaded["ledger"])
        assemble.attach_overstep(record, loaded["overstep"])
        return record

    def test_alpha_gap_beta_numbers(self):
        # Alpha is the old regime, so it has no overstep entry and its figures are gaps, not zeros.
        alpha = self.record_with_overstep("alpha-old")["overstep"]
        self.assertIn("overstep_gap", alpha)
        self.assertIsNone(alpha["recoverable_usd"]["low"])
        self.assertEqual(alpha["items"], [])
        beta = self.record_with_overstep("beta-haiku")["overstep"]
        self.assertEqual(beta["opus_coding_usd"], 6.0)
        self.assertEqual(beta["opus_research_usd"], 1.0)
        self.assertEqual(beta["sonnet_implementing_usd"], 0.0)
        self.assertEqual(beta["recoverable_usd"], {"low": 2.0, "high": 5.0})
        self.assertEqual(beta["items"][0]["rule"], "O1")

    def test_wasted_per_rule_and_total(self):
        # Beta's O1 and O2 wasted 4.5 and 0.5; the campaign total is their sum, and alpha (no rules) has none.
        beta = self.record_with_overstep("beta-haiku")["overstep"]
        self.assertEqual(beta["rules"]["O1"], {"count": 2, "usd": 6.0, "wasted_usd": 4.5})
        self.assertEqual(beta["rules"]["O2"]["wasted_usd"], 0.5)
        self.assertEqual(beta["wasted_usd"], 5.0)
        self.assertIsNone(self.record_with_overstep("alpha-old")["overstep"]["wasted_usd"])

    def test_haiku_without_entry_is_zeros(self):
        loaded = self.loaded
        record = assemble.assemble_core("beta-haiku", loaded["cost"], loaded["packets"], loaded["ledger"])
        record["regime"] = "haiku"
        assemble.attach_overstep(record, {"by_campaign": {}})
        self.assertEqual(record["overstep"]["opus_coding_usd"], 0.0)
        self.assertEqual(record["overstep"]["recoverable_usd"], {"low": 0.0, "high": 0.0})
        self.assertEqual(record["overstep"]["items"], [])
        self.assertNotIn("overstep_gap", record["overstep"])


class AssembleTest(unittest.TestCase):
    def test_assemble_two_campaigns_and_meta(self):
        result = assemble.assemble(inputs.load_inputs(FIXTURES))
        self.assertEqual(set(result["campaigns"]), {"alpha-old", "beta-haiku"})
        self.assertEqual(result["campaigns"]["alpha-old"]["regime"], "old")
        self.assertEqual(result["campaigns"]["beta-haiku"]["overstep"]["opus_coding_usd"], 6.0)
        self.assertEqual(result["campaigns"]["alpha-old"]["escaped_defects"]["count"], 1)
        self.assertEqual(result["meta"]["fixer_requests_total"], 7)
        self.assertEqual(result["meta"]["untagged_prs"], 2)
        self.assertEqual(result["meta"]["missing_inputs"], [])

    def test_assemble_empty_inputs(self):
        empty = {name: {} for name in ("cost", "ledger", "github", "packets", "overstep")}
        empty["missing"] = ["cost", "ledger", "github", "packets", "overstep"]
        result = assemble.assemble(empty)
        self.assertEqual(result["campaigns"], {})
        self.assertEqual(result["meta"]["missing_inputs"], empty["missing"])
        self.assertIsNone(result["meta"]["fixer_requests_total"])


if __name__ == "__main__":
    unittest.main()
