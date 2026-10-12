"""The campaign Markdown report, part two (CM-08g): CI, defect, escalation, stop, cycle and overstep tables."""

import unittest

from reporting import md_campaign_quality


def ci_record():
    return {
        "ci": {
            "first_attempt_rate": 0.75,
            "overall_rate": 1.0,
            "code_prs_first_attempt_rate": 0.5,
            "queue_first_attempt_rate": 1.0,
            "pr_runs_total": 5,
        },
    }


def defect_record():
    return {
        "escaped_defects": {
            "count": 1,
            "rate_per_merged_pr": 0.25,
            "items": [{"kind": "pr", "number": 900, "cites": 102, "days_after_merge": 2.0}],
        },
    }


def escalation_record():
    return {
        "escalation": {
            "dispatched_to_haiku": 3,
            "escalated": 1,
            "rate": 0.33,
            "items": [{"id": "B-3", "to": "rust-gameserver-dev", "why": "context-exhausted"}],
        },
    }


class CiDefectEscalationTest(unittest.TestCase):
    def test_ci_md_rates(self):
        out = md_campaign_quality.ci_md(ci_record())
        self.assertIn("75.0%", out)
        self.assertIn("100.0%", out)
        self.assertIn("PR CI runs, total: 5", out)

    def test_ci_md_none_is_dash_not_zero(self):
        record = ci_record()
        record["ci"]["queue_first_attempt_rate"] = None
        out = md_campaign_quality.ci_md(record)
        self.assertIn("Queue first-attempt rate: -", out)
        self.assertNotIn("Queue first-attempt rate: 0", out)

    def test_ci_md_missing_section_is_gap(self):
        out = md_campaign_quality.ci_md({"ci": {}, "ci_gap": "no CI data"})
        self.assertIn("no CI data", out)
        self.assertNotIn("|", out)

    def test_defects_md_lists_citation(self):
        out = md_campaign_quality.defects_md(defect_record())
        self.assertIn("| pr | 900 | 102 |", out)
        self.assertIn("0.25", out)

    def test_defects_md_missing_section_is_gap(self):
        out = md_campaign_quality.defects_md({"escaped_defects": {}, "escaped_defects_gap": "no defect data"})
        self.assertIn("no defect data", out)
        self.assertNotIn("|", out)

    def test_escalation_md_item(self):
        out = md_campaign_quality.escalation_md(escalation_record())
        self.assertIn("| B-3 | rust-gameserver-dev | context-exhausted |", out)
        self.assertIn("33.0%", out)

    def test_escalation_rate_gap_is_not_zero(self):
        record = escalation_record()
        record["escalation"].update(rate=None, rate_gap="n/a: nothing was dispatched to Haiku")
        out = md_campaign_quality.escalation_md(record)
        self.assertIn("n/a: nothing was dispatched to Haiku", out)
        self.assertNotIn("0.0%", out)


if __name__ == "__main__":
    unittest.main()
