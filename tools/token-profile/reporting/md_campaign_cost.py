"""Campaign report tables for the stamp, cost and PRs. Each returns a Markdown string with LF line ends."""

from report.render import num, pct, table, usd

from .md_common import cell, gap_of


def _gap(record, name):
    """The `<name>_gap` note when the section is empty, else None."""
    return None if record.get(name) else record.get(name + "_gap") or "no data"


def _yes_no(ok):
    return "yes" if ok else "no"


def stamp_md(record):
    gap = _gap(record, "stamp")
    if gap:
        return gap + "\n"
    s = record["stamp"]
    w = s["window"]
    lines = [
        f"- Regime: {cell(record['regime'])}",
        f"- Generated: {cell(s['generated'])}",
        f"- Window: {w['since'] or 'start'} to {w['until'] or 'now'}",
        f"- Price table: {cell(s['price_table'])}",
        f"- Profiler schema: {cell(s['profiler_schema'])}",
        f"- Unattributed share: {cell(s['unattributed_share'], fmt=pct)}",
        f"- Unmapped share: {cell(s['unmapped_share'], fmt=pct)}",
    ]
    return "\n".join(lines) + "\n\nDollars are list-price estimates of plan usage, a floor, never a bill.\n"


def cost_md(record):
    gap = _gap(record, "cost")
    if gap:
        return gap + "\n"
    c = record["cost"]
    parts = [
        table(["", "USD"], [
            ["total", usd(c["total_usd"])],
            ["attributed", usd(c["attributed_usd"])],
            ["unattributed in campaign", usd(c["unattributed_in_campaign_usd"])],
        ]),
        "By model tier:\n\n" + table(["model tier", "USD"], [[tier, usd(v)] for tier, v in c["by_model_tier"].items()]),
        "By role:\n\n" + table(["role", "USD"], [[role, usd(v)] for role, v in c["by_role"].items() if v]),
    ]
    return "\n\n".join(parts) + "\n"


def prs_md(record):
    gap = _gap(record, "prs")
    if gap:
        return gap + "\n"
    p = record["prs"]
    cost = p.get("cost_per_pr") or {}
    note = gap_of(p, "cost_per_pr")
    lines = [
        f"- PRs: {num(p['count'])}, merged {num(p['merged'])}",
        f"- Cost per PR: mean {cell(cost.get('mean'), note, usd)}, p50 {cell(cost.get('p50'), note, usd)}, "
        f"p90 {cell(cost.get('p90'), note, usd)}",
    ]
    rows = [[cell(r["pr"]), cell(r["usd"], fmt=usd), cell(r["ci_first_pass"], fmt=_yes_no), cell(r["ci_runs"], fmt=num)]
            for r in p["per_pr"]]
    return "\n".join(lines) + "\n\n" + table(["PR", "USD", "CI first pass", "CI runs"], rows) + "\n"


def accepted_md(record):
    gap = _gap(record, "accepted")
    if gap:
        return gap + "\n"
    a = record["accepted"]
    note = gap_of(a, "accepted")
    lines = [
        f"- Accepted packets: {num(a['packets'])}",
        f"- Cost per accepted change: {cell(a['cost_per_accepted_usd'], note, usd)}",
    ]
    return "\n".join(lines) + "\n"


def first_pass_md(record):
    gap = _gap(record, "first_pass")
    if gap:
        return gap + "\n"
    f = record["first_pass"]
    lines = [
        f"- First-pass rate: {cell(f['rate'], fmt=pct)}",
        f"- Eligible packets: {num(f['eligible'])}",
    ]
    rows = [[cell(p["id"]), cell(p["implementer_sessions"], fmt=num), cell(p["reviewer_sessions"], fmt=num),
             cell(p["fix_commits"], fmt=num), cell(p["first_pass"], fmt=_yes_no)] for p in f["packets"]]
    return "\n".join(lines) + "\n\n" + table(["id", "implementer sessions", "reviewer sessions", "fix commits",
                                             "first pass"], rows) + "\n"


def retries_md(record):
    gap = _gap(record, "retries")
    if gap:
        return gap + "\n"
    r = record["retries"]
    if r["total"] == 0:
        return "No retries.\n"
    reasons = [[reason, num(n)] for reason, n in r["by_reason"].items() if n]
    return f"- Retries: {cell(r['total'], fmt=num)}\n\n" + table(["reason", "count"], reasons) + "\n"
