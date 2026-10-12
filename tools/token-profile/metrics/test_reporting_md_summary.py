"""The summary Markdown: comparison and low-n warning, n/a rendering, campaign order, own-cost slot, caveats and links."""

import unittest

from reporting.md_summary import render_summary


def _regime(campaigns=5, packets=10, **metrics):
    base = {
        "campaigns": campaigns,
        "packets": packets,
        "prs": 0,
        "merged_prs": 0,
        "total_usd": None,
        "cost_per_pr": None,
        "cost_per_accepted_usd": None,
        "first_pass_rate": None,
        "retries_per_accepted": None,
        "ci_first_attempt_rate": None,
        "escaped_defects_per_merged_pr": None,
        "packet_cycle_p50_hours": None,
        "escalation_rate": None,
        "cost_share_by_tier": {},
        "overstep_recoverable_usd": None,
    }
    base.update(metrics)
    return base


def _summary(old, haiku, delta=None, low=False):
    return {
        "regimes": {"old": old, "haiku": haiku},
        "mixed": {"campaigns": 0, "names": []},
        "comparison": {
            "delta": delta or {},
            "n": {"old": {"campaigns": old["campaigns"], "packets": old["packets"]}, "haiku": {"campaigns": haiku["campaigns"], "packets": haiku["packets"]}},
            "low_n": low,
            "note": "haiku minus old",
        },
    }


def _meta(**overrides):
    meta = {
        "stamp": {"generated": "2026-10-12T12:00:00Z", "window": {"since": "2026-09-21", "until": "2026-10-13"}, "price_table": "2026-10-10", "unattributed_share": 0.16},
        "unmapped_share": 0.1,
        "untagged_prs": 2,
        "opus_subagents": 0,
        "fixer_requests_total": 7,
        "missing_inputs": [],
    }
    meta.update(overrides)
    return meta


def _records():
    return {
        "alpha-old": {
            "regime": "old",
            "cost": {"total_usd": 100.0},
            "prs": {"count": 4, "cost_per_pr": {"p50": 22.5}},
            "accepted": {"cost_per_accepted_usd": 25.0},
            "first_pass": {"rate": 0.75},
            "ci": {"first_attempt_rate": 0.75},
            "escaped_defects": {"count": 1},
        },
        "beta-haiku": {
            "regime": "haiku",
            "cost": {"total_usd": 30.0},
            "prs": {"count": 3, "cost_per_pr": {"p50": 9.0}},
            "accepted": {"cost_per_accepted_usd": 10.0},
            "first_pass": {"rate": 0.33},
            "ci": {"first_attempt_rate": 0.67},
            "escaped_defects": {"count": 0},
        },
    }


def _line(text, label):
    """The table row that starts with a metric label."""
    return next(line for line in text.splitlines() if line.startswith(f"| {label} |"))


class ComparisonTest(unittest.TestCase):
    def test_comparison_has_low_n_warning(self):
        old, haiku = _regime(), _regime()
        warned = render_summary({}, _summary(old, haiku, low=True), _meta())
        plain = render_summary({}, _summary(old, haiku, low=False), _meta())
        self.assertIn("**Small sample: fewer than 5 campaigns or 10 packets on a side", warned)
        self.assertNotIn("**Small sample", plain)

    def test_none_renders_na_not_zero(self):
        old = _regime(cost_per_accepted_usd=None)
        haiku = _regime(cost_per_accepted_usd=10.0)
        text = render_summary({}, _summary(old, haiku, delta={"cost_per_accepted_usd": None}), _meta())
        row = _line(text, "Cost per accepted change")
        self.assertIn("n/a", row)
        self.assertNotIn("$0", row)
        self.assertNotIn("+0", row)

    def test_delta_units_follow_the_metric(self):
        old = _regime(cost_per_pr={"p50": 20.0, "mean": 20.0}, first_pass_rate=0.5, escaped_defects_per_merged_pr=0.25, packet_cycle_p50_hours=4.0)
        haiku = _regime(cost_per_pr={"p50": 9.0, "mean": 9.0}, first_pass_rate=0.75, escaped_defects_per_merged_pr=1.25, packet_cycle_p50_hours=6.0)
        delta = {"cost_per_pr_p50": -11.0, "first_pass_rate": 0.25, "escaped_defects_per_merged_pr": 1.0, "packet_cycle_p50_hours": 2.0}
        text = render_summary({}, _summary(old, haiku, delta=delta), _meta())
        self.assertIn("-$11.00 (better)", _line(text, "Cost per merged PR, p50"))
        self.assertIn("+25.0 pp (better)", _line(text, "First-pass rate"))
        self.assertIn("+1.00 |", _line(text, "Escaped defects per merged PR"))
        self.assertNotIn("better", _line(text, "Escaped defects per merged PR"))
        self.assertIn("+2.0 h", _line(text, "Packet cycle p50"))

    def test_hours_show_one_decimal(self):
        old = _regime(packet_cycle_p50_hours=12.6)
        haiku = _regime(packet_cycle_p50_hours=12.6)
        text = render_summary({}, _summary(old, haiku), _meta())
        self.assertIn("12.6 h", _line(text, "Packet cycle p50"))

    def test_rollup_metrics_all_compared(self):
        old = _regime(total_usd=100.0, escalation_rate=None)
        haiku = _regime(total_usd=30.0, escalation_rate=0.1)
        text = render_summary({}, _summary(old, haiku), _meta())
        self.assertIn("$100.00", _line(text, "Total USD"))
        self.assertIn("$30.00", _line(text, "Total USD"))
        escalation = _line(text, "Escalation rate")
        self.assertIn("n/a", escalation)
        self.assertIn("10.0%", escalation)

    def test_campaigns_sorted_by_cost_desc(self):
        text = render_summary(_records(), _summary(_regime(), _regime()), _meta())
        self.assertLess(text.index("[alpha-old](alpha-old.md)"), text.index("[beta-haiku](beta-haiku.md)"))

    def test_empty_records_render(self):
        text = render_summary({}, _summary(_regime(), _regime()), _meta())
        self.assertIn("No campaigns.", text)


class SlotsAndCaveatsTest(unittest.TestCase):
    def test_own_cost_slot_empty_and_filled(self):
        empty = render_summary({}, _summary(_regime(), _regime()), _meta())
        self.assertIn("Filled at close-out (CM-10).", empty)
        filled = render_summary(
            {},
            _summary(_regime(), _regime()),
            _meta(),
            self_cost={"sessions": 2, "requests": 40, "total_usd": 12.5, "by_tier": {"opus": 12.5}, "by_role": {"coordinator": 12.5}},
        )
        self.assertNotIn("Filled at close-out", filled)
        self.assertIn("| coordinator | $12.50 |", filled)
        self.assertIn("Total $12.50 over 40 requests in 2 sessions.", filled)
        self.assertIn("Measured at close-out; excludes the close-out session's own tail.", filled)

    def test_caveats_list_mixed_names_and_missing_inputs(self):
        summary = _summary(_regime(), _regime())
        summary["mixed"] = {"campaigns": 1, "names": ["mixed-a"]}
        text = render_summary({}, summary, _meta(missing_inputs=["overstep"]))
        self.assertIn("- Mixed campaigns are excluded from the cost comparison: mixed-a.", text)
        self.assertIn("- Missing inputs: overstep.", text)
        self.assertIn("- The overstep range covers every post-switch campaign, mixed ones included; cost comparisons use pure campaigns only.", text)
        self.assertIn("- Untagged PRs: 2.", text)
        self.assertIn("- Unattributed share of spend: 16.0%; unmapped session share: 10.0%.", text)

    def test_links_are_relative_md(self):
        text = render_summary(_records(), _summary(_regime(), _regime()), _meta())
        self.assertIn("[alpha-old](alpha-old.md)", text)
        self.assertIn("(../README.md#findings-for-the-report)", text)
        self.assertNotIn("http", text)


if __name__ == "__main__":
    unittest.main()
