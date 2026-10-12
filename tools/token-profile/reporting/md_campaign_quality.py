"""Campaign report quality tables: CI, escaped defects and escalation. Each returns a Markdown string with LF line ends."""

from report.render import num, pct, table, usd

from .md_common import cell, gap_of, section_gap


def ci_md(record):
    gap = section_gap(record,"ci")
    if gap:
        return gap + "\n"
    c = record["ci"]
    rates = [("First-attempt rate", "first_attempt_rate"), ("Overall rate", "overall_rate"),
             ("Code PRs first-attempt rate", "code_prs_first_attempt_rate"),
             ("Queue first-attempt rate", "queue_first_attempt_rate")]
    lines = [f"- {label}: {cell(c[key], gap_of(c, key), pct)}" for label, key in rates]
    lines.append(f"- PR CI runs, total: {cell(c['pr_runs_total'], fmt=num)}")
    return "\n".join(lines) + "\n"


def defects_md(record):
    gap = section_gap(record,"escaped_defects")
    if gap:
        return gap + "\n"
    d = record["escaped_defects"]
    lines = [
        f"- Escaped defects: {cell(d['count'], fmt=num)}",
        f"- Rate per merged PR: {cell(d['rate_per_merged_pr'], gap_of(d, 'rate_per_merged_pr'), num)}",
    ]
    if not d["items"]:
        return "\n".join(lines) + "\n"
    rows = [[cell(i["kind"]), cell(i["number"], fmt=str), cell(i["cites"], fmt=str)] for i in d["items"]]
    return "\n".join(lines) + "\n\n" + table(["kind", "citing item", "cites PR"], rows) + "\n"


def escalation_md(record):
    gap = section_gap(record,"escalation")
    if gap:
        return gap + "\n"
    e = record["escalation"]
    lines = [
        f"- Dispatched to Haiku: {cell(e['dispatched_to_haiku'], fmt=num)}",
        f"- Escalated: {cell(e['escalated'], fmt=num)}",
        f"- Escalation rate: {cell(e['rate'], gap_of(e, 'rate'), pct)}",
    ]
    if not e["items"]:
        return "\n".join(lines) + "\n"
    rows = [[cell(i["id"]), cell(i["to"]), cell(i["why"])] for i in e["items"]]
    return "\n".join(lines) + "\n\n" + table(["packet", "escalated to", "why"], rows, numeric=False) + "\n"


def stop_context_md(record):
    gap = section_gap(record,"stop_context")
    if gap:
        return gap + "\n"
    s = record["stop_context"]
    ctx = s["context_at_stop"]
    note = gap_of(s, "context_at_stop")
    lines = [
        f"- Workers: {cell(s['workers'], fmt=num)}",
        f"- Context at stop, p50 / p90 / max: {cell(ctx['p50'], note, num)} / {cell(ctx['p90'], note, num)} / "
        f"{cell(ctx['max'], note, num)}",
        f"- Share above 90k: {cell(s['share_above_90k'], gap_of(s, 'share_above_90k'), pct)}",
    ]
    stops = table(["stop", "count"], [[stop, num(n)] for stop, n in s["by_stop"].items()])
    errors = [[kind, num(n)] for kind, n in s["by_error_kind"].items() if n]
    error_md = table(["error kind", "count"], errors) if errors else "No error kinds recorded."
    return "\n".join(lines) + "\n\n" + stops + "\n\n" + error_md + "\n\n90k is the Haiku worker context cap.\n"


def cycle_md(record):
    gap = section_gap(record,"cycle_time")
    if gap:
        return gap + "\n"
    c = record["cycle_time"]
    hours = c["packet_hours"]
    note = gap_of(c, "packet_hours")
    _hours = lambda v: f"{v:,.1f}"
    lines = [
        f"- Campaign hours: {cell(c['campaign_hours'], gap_of(c, 'campaign_hours'), _hours)}",
        f"- Packet hours: p50 {cell(hours['p50'], note, _hours)}, p90 {cell(hours['p90'], note, _hours)} "
        f"over {cell(hours['n'], fmt=num)} packets",
    ]
    return "\n".join(lines) + "\n"


def overstep_md(record):
    o = record.get("overstep") or {}
    note = o.get("overstep_gap") or section_gap(record, "overstep")
    if note:
        return note + "\n"
    r = o["recoverable_usd"]
    r_note = gap_of(o, "recoverable_usd")
    # Older records predate the wasted-cost column, so the key may be absent: omit it rather than print 0.
    has_wasted = "wasted_usd" in o
    lines = [
        f"- Opus coding: {cell(o['opus_coding_usd'], fmt=usd)}",
        f"- Opus research: {cell(o['opus_research_usd'], fmt=usd)}",
        f"- Sonnet implementing: {cell(o['sonnet_implementing_usd'], fmt=usd)}",
        f"- Recoverable: {cell(r['low'], r_note, usd)} to {cell(r['high'], r_note, usd)}",
    ]
    if has_wasted:
        lines.append(f"- Wasted (repriced at intended tier): {cell(o['wasted_usd'], fmt=usd)}")
    if not o["items"]:
        return "\n".join(lines) + "\n"
    headers = ["rule", "USD", "tool", "path class", "model"]
    rows = [[cell(i["rule"]), cell(i["usd"], fmt=usd), cell(i["tool"]), cell(i["path_class"]), cell(i["model"])]
            for i in o["items"]]
    if has_wasted:
        headers.append("wasted USD")
        for row, i in zip(rows, o["items"]):
            row.append(cell(i.get("wasted_usd"), fmt=usd))
    return "\n".join(lines) + "\n\n" + table(headers, rows, numeric=False) + "\n"
