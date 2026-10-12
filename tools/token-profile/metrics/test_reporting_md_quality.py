"""The campaign Markdown report, part two (CM-08g): CI, defect, escalation, stop, cycle and overstep tables."""

import unittest

from reporting import md_campaign, md_campaign_quality


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

    def test_defects_md_pr_numbers_are_plain(self):
        record = defect_record()
        record["escaped_defects"]["items"] = [{"kind": "pr", "number": 1643, "cites": 1102, "days_after_merge": 1.0}]
        out = md_campaign_quality.defects_md(record)
        self.assertIn("| pr | 1643 | 1102 |", out)
        self.assertNotIn("1,643", out)

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


def stop_record():
    return {
        "stop_context": {
            "workers": 5,
            "by_stop": {"end_turn": 2, "tool_use": 1, "error": 1, "no_record": 1},
            "by_error_kind": {"rate_limit": 1, "authentication_failed": 0, "other": 0},
            "context_at_stop": {"p50": 61000, "p90": 95000, "max": 98000},
            "share_above_90k": 0.4,
        },
    }


def overstep_record():
    return {
        "overstep": {
            "opus_coding_usd": 6.0,
            "opus_research_usd": 1.0,
            "sonnet_implementing_usd": 0.0,
            "recoverable_usd": {"low": 2.0, "high": 5.0},
            "items": [{"rule": "O1", "usd": 3.0, "tool": "Edit", "path_class": "code", "model": "claude-opus-5-5"}],
        },
    }


class StopCycleOverstepTest(unittest.TestCase):
    def test_stop_context_share_and_cap_sentence(self):
        out = md_campaign_quality.stop_context_md(stop_record())
        self.assertIn("40.0%", out)
        self.assertIn("61,000", out)
        self.assertIn("| error | 1 |", out)
        self.assertIn("| rate_limit | 1 |", out)
        self.assertIn("90k is the Haiku worker context cap.", out)

    def test_stop_context_gap_is_not_zero(self):
        record = stop_record()
        record["stop_context"]["context_at_stop"] = {"p50": None, "p90": None, "max": None}
        record["stop_context"]["context_at_stop_gap"] = "no context data"
        out = md_campaign_quality.stop_context_md(record)
        self.assertIn("n/a (no context data)", out)

    def test_cycle_md_numbers(self):
        record = {"cycle_time": {"campaign_hours": 24.0, "packet_hours": {"n": 4, "p50": 6.0, "p90": 9.0}}}
        out = md_campaign_quality.cycle_md(record)
        self.assertIn("Campaign hours: 24.0", out)
        self.assertIn("p50 6.0, p90 9.0 over 4 packets", out)

    def test_cycle_md_hours_one_decimal(self):
        record = {"cycle_time": {"campaign_hours": 36.7, "packet_hours": {"n": 4, "p50": 6.0, "p90": 9.0}}}
        out = md_campaign_quality.cycle_md(record)
        self.assertIn("Campaign hours: 36.7", out)

    def test_overstep_wasted_column_and_total(self):
        record = overstep_record()
        record["overstep"]["wasted_usd"] = 1.5
        record["overstep"]["items"][0]["wasted_usd"] = 1.5
        out = md_campaign_quality.overstep_md(record)
        self.assertIn("- Wasted (repriced at intended tier): $1.50", out)
        self.assertIn("| rule | USD | tool | path class | model | wasted USD |", out)
        self.assertIn("| O1 | $3.00 | Edit | code | claude-opus-5-5 | $1.50 |", out)

    def test_overstep_wasted_absent_is_omitted(self):
        out = md_campaign_quality.overstep_md(overstep_record())
        self.assertNotIn("Wasted", out)
        self.assertNotIn("wasted USD", out)

    def test_overstep_beta_numbers(self):
        out = md_campaign_quality.overstep_md(overstep_record())
        self.assertIn("Opus coding: $6.00", out)
        self.assertIn("Sonnet implementing: $0.00", out)
        self.assertIn("Recoverable: $2.00 to $5.00", out)
        self.assertIn("| O1 | $3.00 | Edit | code | claude-opus-5-5 |", out)

    def test_overstep_gap_hides_numbers(self):
        record = {"overstep": {"overstep_gap": "predates the regime switch; the rules apply to the Haiku regime",
                               "opus_coding_usd": None, "opus_research_usd": None, "sonnet_implementing_usd": None,
                               "recoverable_usd": {"low": None, "high": None}, "items": []}}
        out = md_campaign_quality.overstep_md(record)
        self.assertIn("predates the regime switch", out)
        self.assertNotIn("$", out)


def campaign_record():
    return {
        "campaign": "beta-haiku",
        "regime": "haiku",
        "stamp": {"generated": "2026-10-12T12:00:00Z", "window": {"since": "2026-09-21T00:00:00Z", "until": None},
                  "price_table": "2026-10-10", "profiler_schema": "6", "unattributed_share": 0.1,
                  "unmapped_share": 0.1},
        "cost": {"total_usd": 30.0, "attributed_usd": 27.0, "unattributed_in_campaign_usd": 3.0,
                 "by_model_tier": {"opus": 12.0, "haiku": 10.0}, "by_role": {"coordinator": 12.0, "reviewer": 0.0}},
        "prs": {"count": 3, "merged": 3, "cost_per_pr": {"mean": 9.0, "p50": 9.0, "p90": 9.8},
                "per_pr": [{"pr": 201, "usd": 9.0, "merged": True, "ci_first_pass": True, "ci_runs": 1}]},
        "accepted": {"packets": 3, "cost_per_accepted_usd": 10.0},
        "first_pass": {"rate": 0.33, "eligible": 3,
                       "packets": [{"id": "B-1", "implementer_sessions": 1, "reviewer_sessions": 1,
                                    "fix_commits": 0, "first_pass": True}]},
        "retries": {"total": 2, "by_reason": {"context-exhausted": 2, "ci-failure": 0}},
        "ci": {"first_attempt_rate": 0.67, "overall_rate": 1.0, "code_prs_first_attempt_rate": 0.5,
               "queue_first_attempt_rate": 1.0, "pr_runs_total": 4},
        "escaped_defects": {"count": 0, "rate_per_merged_pr": 0.0, "items": []},
        "escalation": {"dispatched_to_haiku": 3, "escalated": 1, "rate": 0.33,
                       "items": [{"id": "B-3", "to": "rust-gameserver-dev", "why": "context-exhausted"}]},
        "stop_context": {"workers": 5, "by_stop": {"end_turn": 2, "tool_use": 1, "error": 1, "no_record": 1},
                         "by_error_kind": {"rate_limit": 1, "authentication_failed": 0, "other": 0},
                         "context_at_stop": {"p50": 61000, "p90": 95000, "max": 98000}, "share_above_90k": 0.4},
        "cycle_time": {"campaign_hours": 12.0, "packet_hours": {"n": 3, "p50": 2.0, "p90": 3.0}},
        "overstep": {"opus_coding_usd": 6.0, "opus_research_usd": 1.0, "sonnet_implementing_usd": 0.0,
                     "recoverable_usd": {"low": 2.0, "high": 5.0}, "items": []},
    }


SECTION_TITLES = ["Stamp", "Cost", "PRs", "Accepted", "First pass", "Retries", "CI", "Escaped defects",
                  "Escalation", "Stop context", "Cycle time", "Overstep"]


class RenderCampaignTest(unittest.TestCase):
    def test_render_campaign_section_order(self):
        out = md_campaign.render_campaign(campaign_record())
        self.assertTrue(out.startswith("# beta-haiku\n\n> Regime: haiku\n"))
        positions = [out.index(f"### {title}\n") for title in SECTION_TITLES]
        self.assertEqual(positions, sorted(positions))
        self.assertEqual(out.count("\n### "), len(SECTION_TITLES))

    def test_render_campaign_ends_with_single_newline(self):
        out = md_campaign.render_campaign(campaign_record())
        self.assertTrue(out.endswith("\n"))
        self.assertFalse(out.endswith("\n\n"))

    def test_render_campaign_gap_section_has_no_table(self):
        record = campaign_record()
        record["first_pass"] = {}
        record["first_pass_gap"] = "no packets record for this campaign"
        out = md_campaign.render_campaign(record)
        self.assertIn("no packets record for this campaign", out)
        self.assertNotIn("| id |", out)


if __name__ == "__main__":
    unittest.main()
