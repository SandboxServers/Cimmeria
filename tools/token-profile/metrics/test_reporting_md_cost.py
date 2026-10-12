"""The campaign Markdown report, part one (CM-08f): cells, cost, PR, accepted, first-pass and retry tables."""

import unittest

from reporting import md_campaign_cost, md_common


def haiku_record():
    return {
        "regime": "haiku",
        "stamp": {
            "generated": "2026-10-12T10:00:00Z",
            "window": {"since": "2026-10-10T20:02:54Z", "until": None},
            "price_table": "2026-10",
            "profiler_schema": "6",
            "unattributed_share": 0.1,
            "unmapped_share": 0.0,
        },
        "cost": {
            "total_usd": 30.0,
            "attributed_usd": 27.0,
            "unattributed_in_campaign_usd": 3.0,
            "by_model_tier": {"haiku": 24.0, "opus": 6.0},
            "by_role": {"implementer": 20.0, "reviewer": 10.0, "fix": 0.0},
        },
        "prs": {
            "count": 3,
            "merged": 2,
            "cost_per_pr": {"mean": 10.0, "p50": 9.0, "p90": 14.0},
            "per_pr": [
                {"pr": 101, "usd": 9.0, "merged": True, "ci_first_pass": True, "ci_runs": 1},
                {"pr": 102, "usd": 12.0, "merged": True, "ci_first_pass": False, "ci_runs": 3},
                {"pr": 103, "usd": 9.0, "merged": False, "ci_first_pass": None, "ci_runs": None},
            ],
        },
        "accepted": {"packets": 2, "cost_per_accepted_usd": 15.0},
        "first_pass": {
            "rate": 0.33,
            "eligible": 3,
            "packets": [{"id": "CM-08f", "implementer_sessions": 1, "reviewer_sessions": 1, "fix_commits": 1,
                         "first_pass": False}],
        },
        "retries": {"total": 2, "by_reason": {"ci": 2, "review": 0}},
    }


class StampCostPrTest(unittest.TestCase):
    def test_cost_md_numbers(self):
        out = md_campaign_cost.cost_md(haiku_record())
        self.assertIn("$30.00", out)
        self.assertIn("| haiku | $24.00 |", out)
        self.assertNotIn("| fix |", out)

    def test_prs_md_marks_ci(self):
        out = md_campaign_cost.prs_md(haiku_record())
        self.assertIn("| 102 | $12.00 | no | 3 |", out)
        self.assertIn("| 101 | $9.00 | yes | 1 |", out)
        self.assertIn("| 103 | $9.00 | - | - |", out)

    def test_stamp_md_states_the_floor(self):
        out = md_campaign_cost.stamp_md(haiku_record())
        self.assertIn("Dollars are list-price estimates of plan usage, a floor, never a bill.", out)
        self.assertIn("- Unattributed share: 10.0%", out)

    def test_empty_cost_renders_gap_not_table(self):
        record = haiku_record()
        record["cost"] = {}
        record["cost_gap"] = "no ledger rows"
        out = md_campaign_cost.cost_md(record)
        self.assertIn("no ledger rows", out)
        self.assertNotIn("|", out)

    def test_empty_cost_without_gap_says_no_data(self):
        record = haiku_record()
        record["cost"] = {}
        out = md_campaign_cost.cost_md(record)
        self.assertIn("no data", out)
        self.assertNotIn("|", out)

    def test_cost_per_pr_none_renders_without_error(self):
        record = haiku_record()
        record["prs"]["cost_per_pr"] = None
        out = md_campaign_cost.prs_md(record)
        self.assertIn("- Cost per PR: mean -, p50 -, p90 -", out)
        self.assertIn("| 102 | $12.00 | no | 3 |", out)


class AcceptedFirstPassRetriesTest(unittest.TestCase):
    def test_retries_none_sentence(self):
        record = haiku_record()
        record["retries"] = {"total": 0, "by_reason": {"ci": 0, "review": 0}}
        self.assertEqual(md_campaign_cost.retries_md(record), "No retries.\n")

    def test_retries_table_lists_non_zero_reasons(self):
        out = md_campaign_cost.retries_md(haiku_record())
        self.assertIn("- Retries: 2", out)
        self.assertIn("| ci | 2 |", out)
        self.assertNotIn("review", out)

    def test_first_pass_table(self):
        out = md_campaign_cost.first_pass_md(haiku_record())
        self.assertIn("- First-pass rate: 33.0%", out)
        self.assertIn("| CM-08f | 1 | 1 | 1 | no |", out)

    def test_missing_section_renders_gap_not_table(self):
        record = haiku_record()
        record["first_pass"] = {}
        record["first_pass_gap"] = "no eligible packets yet"
        out = md_campaign_cost.first_pass_md(record)
        self.assertIn("no eligible packets yet", out)
        self.assertNotIn("|", out)

    def test_no_zero_where_unknown(self):
        record = haiku_record()
        record["accepted"] = {"packets": 0, "cost_per_accepted_usd": None, "accepted_gap": "nothing accepted"}
        out = md_campaign_cost.accepted_md(record)
        self.assertNotIn("$0.00", out)
        self.assertIn("n/a (nothing accepted)", out)
        self.assertEqual(out.count("nothing accepted"), 1)

    def test_accepted_cost_renders_dollars(self):
        out = md_campaign_cost.accepted_md(haiku_record())
        self.assertIn("- Cost per accepted change: $15.00", out)

    def test_accepted_note_kept_beside_a_number(self):
        record = haiku_record()
        record["accepted"] = {"packets": 2, "cost_per_accepted_usd": 12.5,
                              "accepted_gap": "no packet table, used merged PRs"}
        out = md_campaign_cost.accepted_md(record)
        self.assertIn("$12.50", out)
        self.assertEqual(out.count("no packet table, used merged PRs"), 1)


class CellTest(unittest.TestCase):
    def test_cell_gap_beats_dash(self):
        self.assertEqual(md_common.cell(None, gap="no ledger"), "n/a (no ledger)")
        self.assertNotEqual(md_common.cell(None, gap="no ledger"), "-")

    def test_cell_dash_without_gap(self):
        self.assertEqual(md_common.cell(None), "-")

    def test_cell_formats_present_values(self):
        self.assertEqual(md_common.cell(3, fmt=str), "3")
        self.assertEqual(md_common.cell(0, gap="no ledger", fmt=str), "0")

    def test_gap_of_reads_the_sibling_note(self):
        self.assertEqual(md_common.gap_of({"accepted_gap": "none merged"}, "accepted"), "none merged")
        self.assertIsNone(md_common.gap_of({}, "accepted"))

    def test_section_md_wraps_body(self):
        self.assertEqual(md_common.section_md("Cost", "body"), "### Cost\n\nbody\n")


if __name__ == "__main__":
    unittest.main()
