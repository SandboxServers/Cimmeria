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

    def test_haiku_without_entry_is_zeros(self):
        loaded = self.loaded
        record = assemble.assemble_core("beta-haiku", loaded["cost"], loaded["packets"], loaded["ledger"])
        record["regime"] = "haiku"
        assemble.attach_overstep(record, {"by_campaign": {}})
        self.assertEqual(record["overstep"]["opus_coding_usd"], 0.0)
        self.assertEqual(record["overstep"]["recoverable_usd"], {"low": 0.0, "high": 0.0})
        self.assertEqual(record["overstep"]["items"], [])
        self.assertNotIn("overstep_gap", record["overstep"])


if __name__ == "__main__":
    unittest.main()
