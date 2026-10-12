"""Summary Markdown: the stamp, the old-versus-Haiku comparison, the campaign table, overstep, caveats, sizing and own cost."""

from report.render import num, pct, table, usd

from .compare import LOW_N_CAMPAIGNS, LOW_N_PACKETS

NA = "n/a"
SIZING = "Packet sizing against the 90k Haiku context cap is summarized in the [README findings](../README.md#findings-for-the-report)."
LOW_N_WARNING = (
    f"**Small sample: fewer than {LOW_N_CAMPAIGNS} campaigns or {LOW_N_PACKETS} packets on a side; "
    "read the deltas as directional only.**"
)
# (label, delta key, kind, better): kind picks the value and delta units; better is the good direction, None when there is none.
COMPARISON = (
    ("Cost per merged PR, p50", "cost_per_pr_p50", "usd", "lower"),
    ("Cost per merged PR, mean", "cost_per_pr_mean", "usd", "lower"),
    ("Cost per accepted change", "cost_per_accepted_usd", "usd", "lower"),
    ("First-pass rate", "first_pass_rate", "rate", "higher"),
    ("Retries per accepted packet", "retries_per_accepted", "num", "lower"),
    ("CI first-attempt rate", "ci_first_attempt_rate", "rate", "higher"),
    ("Escaped defects per merged PR", "escaped_defects_per_merged_pr", "num", None),
    ("Packet cycle p50", "packet_cycle_p50_hours", "hours", None),
    ("Pure campaigns", "campaigns", "num", None),
    ("Merged PRs", "merged_prs", "num", None),
    ("Total USD", "total_usd", "usd", None),
    ("Escalation rate", "escalation_rate", "rate", None),
)


def _get(doc, *path):
    """Walk nested dicts; None as soon as a key is missing or a level is not a dict."""
    for key in path:
        if not isinstance(doc, dict):
            return None
        doc = doc.get(key)
    return doc


def _shown(value, kind):
    """A value in its own unit; None renders n/a, never 0."""
    if value is None:
        return NA
    if kind == "usd":
        return usd(value)
    if kind == "rate":
        return pct(value)
    if kind == "hours":
        return f"{value:,.1f} h"
    if kind == "text":
        return str(value)
    return num(value)


def _plural(value, noun):
    """A count with its noun, singular for one: "1 request", "2 requests"; n/a when the count is missing."""
    if value is None:
        return NA
    return f"{num(value)} {noun}" if value == 1 else f"{num(value)} {noun}s"


def _regime_value(regime, key):
    """One comparison metric from a roll-up; the two cost-per-PR figures live inside cost_per_pr."""
    if key.startswith("cost_per_pr_"):
        return _get(regime, "cost_per_pr", key[len("cost_per_pr_"):])
    return _get(regime, key)


def _delta_text(value, kind, better):
    """Haiku minus old in the metric's unit, with a better or worse label where the metric has a direction."""
    if value is None:
        return NA
    if kind == "usd":
        text = ("-" if value < 0 else "+") + f"${abs(value):,.2f}"
    elif kind == "rate":
        text = f"{value * 100:+.1f} pp"
    elif kind == "hours":
        text = f"{value:+,.1f} h"
    else:
        text = f"{value:+,.2f}"
    if better is None or value == 0:
        return text
    improved = (value < 0) == (better == "lower")
    return f"{text} ({'better' if improved else 'worse'})"


def _section(title, blocks):
    """One Markdown section: a level-two heading over blocks separated by blank lines."""
    return f"## {title}\n\n" + "\n\n".join(blocks) + "\n"


def _stamp_text(meta):
    """The cost stamp: when the numbers were made, the window, the price table and the price basis."""
    stamp = meta.get("stamp") or {}
    window = stamp.get("window") or {}
    return "\n".join(
        [
            f"- Generated: {_shown(stamp.get('generated'), 'text')}",
            f"- Window: {_shown(window.get('since'), 'text')} to {_shown(window.get('until'), 'text')}",
            f"- Price table: {_shown(stamp.get('price_table'), 'text')}",
            "- Dollars are a list-price estimate, a floor.",
        ]
    )


def _sample(side):
    """Campaign and packet counts for one side of the comparison."""
    side = side or {}
    return f"{_shown(side.get('campaigns'), 'num')} campaigns and {_shown(side.get('packets'), 'num')} packets"


def _comparison_blocks(summary, low):
    """The comparison table with its low-n warning, sample counts and note."""
    comparison = _get(summary, "comparison") or {}
    old, haiku = _get(summary, "regimes", "old"), _get(summary, "regimes", "haiku")
    rows = []
    for label, key, kind, better in COMPARISON:
        rows.append(
            [
                label,
                _shown(_regime_value(old, key), kind),
                _shown(_regime_value(haiku, key), kind),
                _delta_text(_get(comparison, "delta", key), kind, better),
            ]
        )
    blocks = [LOW_N_WARNING] if low else []
    blocks.append(table(["metric", "old", "Haiku", "delta"], rows))
    sample = comparison.get("n") or {}
    blocks.append(f"Sample: old {_sample(sample.get('old'))}; Haiku {_sample(sample.get('haiku'))}.")
    blocks.append(f"Note: {_shown(comparison.get('note'), 'text')}.")
    return blocks


def _campaign_table(records):
    """One row per campaign, largest spend first, each name linked to its own report."""
    if not records:
        return "No campaigns."
    head = ["campaign", "regime", "USD", "PRs", "$/PR p50", "$/accepted", "first pass", "CI first", "escaped"]
    ordered = sorted(records.items(), key=lambda item: _get(item[1], "cost", "total_usd") or 0, reverse=True)
    rows = []
    for name, record in ordered:
        rows.append(
            [
                f"[{name}]({name}.md)",
                _shown(_get(record, "regime"), "text"),
                _shown(_get(record, "cost", "total_usd"), "usd"),
                _shown(_get(record, "prs", "count"), "num"),
                _shown(_get(record, "prs", "cost_per_pr", "p50"), "usd"),
                _shown(_get(record, "accepted", "cost_per_accepted_usd"), "usd"),
                _shown(_get(record, "first_pass", "rate"), "rate"),
                _shown(_get(record, "ci", "first_attempt_rate"), "rate"),
                _shown(_get(record, "escaped_defects", "count"), "num"),
            ]
        )
    return table(head, rows)


def _recoverable_text(regime):
    """The recoverable overstep range for one regime, or its gap note when the regime has none."""
    recoverable = _get(regime, "overstep_recoverable_usd")
    if recoverable is None:
        gap = _get(regime, "overstep_recoverable_usd_gap")
        return f"n/a ({gap})" if gap else NA
    return f"{_shown(recoverable.get('low'), 'usd')} to {_shown(recoverable.get('high'), 'usd')}"


def _overstep_text(summary, meta):
    """The recoverable range per regime and the fixer request total."""
    lines = [
        f"- Old regime, recoverable overstep: {_recoverable_text(_get(summary, 'regimes', 'old'))}.",
        f"- Haiku regime, recoverable overstep: {_recoverable_text(_get(summary, 'regimes', 'haiku'))}.",
        f"- Fixer requests: {_shown(meta.get('fixer_requests_total'), 'num')}.",
    ]
    return "\n".join(lines)


def _caveat_text(summary, meta, low):
    """Bullets on what the numbers do not cover: unattributed and unmapped spend, floors, mixed and missing inputs."""
    stamp = meta.get("stamp") or {}
    names = _get(summary, "mixed", "names") or []
    missing = meta.get("missing_inputs") or []
    lines = [
        f"- Unattributed share of spend: {_shown(stamp.get('unattributed_share'), 'rate')}; "
        f"unmapped session share: {_shown(meta.get('unmapped_share'), 'rate')}.",
        "- USD is a floor: list-price estimates of plan usage, not a bill.",
        f"- Mixed campaigns are excluded from the cost comparison: {', '.join(names) or 'none'}.",
        f"- Untagged PRs: {_shown(meta.get('untagged_prs'), 'num')}.",
        f"- Missing inputs: {', '.join(missing) or 'none'}.",
        f"- Small samples: {'yes, see the warning in Comparison' if low else 'no'}.",
    ]
    return "\n".join(lines)


def _own_cost_blocks(self_cost):
    """The own-cost slot: a placeholder until close-out, then the priced sessions by role."""
    if self_cost is None:
        return ["Filled at close-out (CM-10)."]
    rows = [[role, _shown(value, "usd")] for role, value in sorted((self_cost.get("by_role") or {}).items())]
    return [
        table(["role", "USD"], rows),
        f"Total {_shown(self_cost.get('total_usd'), 'usd')} over {_plural(self_cost.get('requests'), 'request')} "
        f"in {_plural(self_cost.get('sessions'), 'session')}.",
        "Measured at close-out; excludes the close-out session's own tail.",
    ]


def render_summary(records, summary, meta, self_cost=None):
    """The campaign-metrics summary as Markdown; a missing number renders n/a, and the own-cost slot waits for close-out."""
    meta = meta or {}
    low = bool(_get(summary, "comparison", "low_n"))
    sections = [
        _section("Stamp", [_stamp_text(meta)]),
        _section("Comparison", _comparison_blocks(summary, low)),
        _section("Campaigns", [_campaign_table(records)]),
        _section("Overstep", [_overstep_text(summary, meta)]),
        _section("Caveats", [_caveat_text(summary, meta, low)]),
        _section("Packet sizing vs the 90k Haiku cap", [SIZING]),
        _section("Cost of this campaign", _own_cost_blocks(self_cost)),
    ]
    return "# Campaign metrics summary\n\n" + "\n".join(sections)
