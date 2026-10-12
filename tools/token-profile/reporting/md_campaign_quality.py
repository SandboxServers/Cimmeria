"""Campaign report quality tables: CI, escaped defects and escalation. Each returns a Markdown string with LF line ends."""

from report.render import num, pct, table

from .md_campaign_cost import _gap
from .md_common import cell, gap_of


def ci_md(record):
    gap = _gap(record, "ci")
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
    gap = _gap(record, "escaped_defects")
    if gap:
        return gap + "\n"
    d = record["escaped_defects"]
    lines = [
        f"- Escaped defects: {cell(d['count'], fmt=num)}",
        f"- Rate per merged PR: {cell(d['rate_per_merged_pr'], gap_of(d, 'rate_per_merged_pr'), num)}",
    ]
    if not d["items"]:
        return "\n".join(lines) + "\n"
    rows = [[cell(i["kind"]), cell(i["number"], fmt=num), cell(i["cites"], fmt=num)] for i in d["items"]]
    return "\n".join(lines) + "\n\n" + table(["kind", "citing item", "cites PR"], rows) + "\n"


def escalation_md(record):
    gap = _gap(record, "escalation")
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
