"""Build one PR's stats comment: a short human table and the cimmeria-pr-stats/1 block.

The database answers through report.sections.prs.pr_records(); gh answers the
quality fields and the diff. The whole block goes through the TP-01b scrubber,
and the rendered comment through its gate, before anything is printed or posted.
Fields are only ever added; a breaking change bumps the schema version.
"""

import json
import re
import sqlite3
from datetime import datetime

from report import db as dbmod
from report.sections.prs import pr_records

from .github import MARKER

SCHEMA = "cimmeria-pr-stats/1"
# No #fragment: the gate allows a public URL only without query or fragment.
README = "https://github.com/SandboxServers/Cimmeria/blob/main/tools/token-profile/README.md"
LABEL = "Estimated list-price USD, a plan-usage proxy, not a bill (D-TP1)."
# Keys added after the plan's, in this order (fields are only ever appended).
APPENDED = ("campaign", "cost_state")
COST_STATE_REASON = ("Claude Code's cost-state total is per process, and a process spans several PRs,"
                     " so it is not split by PR")
_BLOCK = re.compile(r"```json\n(\{.*?\})\n```", re.S)


class NoData(Exception):
    """The database has no request attributed to this PR."""


def open_scoped(db_path, price_table=None):
    """(db, price table version), with the report views in place over all requests."""
    db = dbmod.open_db(db_path)
    try:
        return db, dbmod.scope(db, price_table=price_table)
    except Exception:
        db.close()
        raise


def _version_key(v):
    return tuple(int(p) if p.isdigit() else -1 for p in re.split(r"[.-]", v))


def db_fields(db, sc, pr, price_table):
    """The block's database-backed fields, the version stamp first. Raises NoData."""
    rec = pr_records(db, sc, [pr])[0]
    if not rec["requests"]:
        raise NoData(pr)
    profiler = db.execute("SELECT profiler_commit FROM profiler_runs WHERE status = 'ok'"
                          " ORDER BY run_id DESC LIMIT 1").fetchone()
    versions = [r[0] for r in db.execute(
        "SELECT DISTINCT r.cc_version FROM pr_attribution a JOIN requests r ON r.request_id = a.request_id"
        " WHERE a.pr_number = ? AND r.cc_version IS NOT NULL", (pr,))]
    claude_code = max(versions, key=_version_key) if versions else None
    campaign = _campaign(db, pr)
    return {
        "schema": SCHEMA,
        "pr": pr,
        # Twelve characters identify a commit; forty look like a secret to the gate.
        "profiler": sc.label(profiler[0][:12] if profiler and profiler[0] else None, "profiler_commit"),
        "claude_code": sc.label(claude_code, "cc_version"),
        "price_table": sc.label(price_table, "price_table"),
        **{k: v for k, v in rec.items() if k != "pr"},
        "campaign": sc.label(campaign, "campaign"),
        "cost_state": cost_state(db, pr),
    }


def _campaign(db, pr):
    try:
        row = db.execute("SELECT campaign FROM prs WHERE pr_number = ?", (pr,)).fetchone()
    except sqlite3.OperationalError:        # schema before version 4
        return None
    return row[0] if row else None


def cost_state(db, pr):
    """D-TP6 for one PR: no per-PR cost-state number (there is none), but how far the profiler fell short of
    Claude Code's own total over the sessions this PR's spend came from, and how much of the spend they cover."""
    out = {"usd": None, "reason": COST_STATE_REASON, "sessions": 0, "covered_share": 0.0, "sessions_gap_share": None}
    try:
        rows = db.execute(
            "SELECT r.session_id, SUM(a.weight * COALESCE(c.usd, 0)),"
            " SUM(CASE WHEN cs.session_id IS NOT NULL AND r.ts >= COALESCE(cs.process_start, '')"
            "     THEN a.weight * COALESCE(c.usd, 0) ELSE 0 END)"
            " FROM pr_attribution a JOIN requests r USING (request_id) JOIN rcost c USING (request_id)"
            " LEFT JOIN cost_states cs ON cs.session_id = r.session_id"
            " WHERE a.pr_number = ? GROUP BY r.session_id", (pr,)).fetchall()
        spend = sum(r[1] or 0 for r in rows)
        covered = sum(r[2] or 0 for r in rows)
        sessions = [r[0] for r in rows if r[2]]
        cs_total = prof_total = 0.0
        for s in sessions:
            total, start = db.execute("SELECT total_cost_usd, process_start FROM cost_states WHERE session_id = ?",
                                      (s,)).fetchone()
            cs_total += total or 0
            prof_total += db.execute("SELECT COALESCE(SUM(c.usd), 0) FROM requests r JOIN rcost c USING (request_id)"
                                     " WHERE r.session_id = ? AND r.ts >= ?", (s, start or "")).fetchone()[0]
    except sqlite3.OperationalError:        # schema before version 3: no process_start
        return out
    out.update(sessions=len(sessions), covered_share=round(covered / spend, 4) if spend else 0.0,
               sessions_gap_share=round((prof_total - cs_total) / cs_total, 4) if cs_total else None)
    return out


def assemble(fields, quality, gh_pr):
    """Add the gh-backed fields. The plan's key order is kept: quality goes before diff."""
    block = {k: v for k, v in fields.items() if k != "diff" and k not in APPENDED}
    if gh_pr.get("mergedAt") and not block["window"].get("merged"):
        block["window"] = {**block["window"], "merged": gh_pr["mergedAt"]}
    block["quality"] = quality
    if gh_pr.get("additions") is not None:
        block["diff"] = {"additions": gh_pr["additions"], "deletions": gh_pr.get("deletions"),
                         "files": gh_pr.get("changedFiles")}
    else:
        block["diff"] = fields.get("diff") or {}
    for key in APPENDED:
        if key in fields:
            block[key] = fields[key]
    return block


def cost_state_sentence(cs):
    if not cs or not cs.get("sessions") or cs.get("sessions_gap_share") is None:
        return "No Claude Code cost-state record covers this PR's sessions, so there is no total to compare."
    gap = cs["sessions_gap_share"]
    return (f"Claude Code's own cost-state total is per process and is not split by PR; over the {cs['sessions']}"
            f" sessions this PR's spend came from ({cs['covered_share'] * 100:.0f}% of it), the profiler was"
            f" {abs(gap) * 100:.1f}% {'under' if gap < 0 else 'over'} it, so read the USD as a floor.")


def _ts(s):
    return datetime.fromisoformat(s.replace("Z", "+00:00")) if s else None


def wall_clock_hours(window):
    first, merged = _ts(window.get("first")), _ts(window.get("merged"))
    if not first or not merged:
        return None
    return max((merged - first).total_seconds(), 0) / 3600


def _num(x):
    return "-" if x is None else f"{round(x):,}"


def render(block):
    """The comment body: marker, table, one line of context, the JSON block."""
    q, a = block["quality"], block["attribution"]
    hours = wall_clock_hours(block["window"])
    ci = (f"{q['ci_fail_rounds']} failed of {q['ci_rounds']}" if "ci_rounds" in q
          else f"{q['ci_fail_rounds']} failed")
    row = [f"${block['usd_est']:,.2f}", _num(block["requests"]), _num(block["human_prompts"]),
           _num(sum(v["count"] for v in block["agents"].values())), _num(block["context"]["peak"]),
           "-" if hours is None else f"{hours:,.1f} h", ci]
    fixes = ", ".join(f"#{n}" for n in q["followup_fix_prs"]) or "none"
    lines = [
        MARKER,
        "**Token profile** for this PR. " + LABEL,
        "",
        "| est. USD (list) | requests | human prompts | agents | peak ctx | wall clock | CI rounds |",
        "|---:|---:|---:|---:|---:|---:|---:|",
        "| " + " | ".join(row) + " |",
        "",
        f"Attribution: {a['method'] or '-'}, confidence {a['confidence']:.2f}; "
        f"{a['unattributed_share'] * 100:.1f}% of the same sessions' spend over this PR's window is unattributed. "
        f"Review rounds: {q['review_rounds']}. Follow-up fix PRs: {fixes}. "
        f"Reverted: {'yes' if q['reverted'] else 'no'}."
        + (f" Campaign: {block['campaign']}." if block.get("campaign") else ""),
        "",
        cost_state_sentence(block.get("cost_state")),
        "",
        f"Profiler {block['profiler'] or '-'}, Claude Code {block['claude_code'] or '-'}, "
        f"price table {block['price_table'] or '-'}. [How this is measured]({README})",
        "",
        "```json",
        json.dumps(block, ensure_ascii=False, separators=(",", ":")),
        "```",
        "",
    ]
    return "\n".join(lines)


def parse(body):
    """The cimmeria-pr-stats/1 block in a comment body, or None. For the aggregation (TP-11)."""
    if not body or not body.startswith(MARKER):
        return None
    m = _BLOCK.search(body)
    if not m:
        return None
    block = json.loads(m.group(1))
    return block if block.get("schema", "").startswith("cimmeria-pr-stats/") else None


def comment(db, sc, pr, price_table, gh, before_gh=None):
    """(body, block) for one PR, scrubbed and through the gate. Raises NoData or scrub.PrivacyError.

    The database is read first, so a PR with no data costs no gh call, and
    `before_gh` (the backfill's rate limiter) runs only when gh is about to be called.
    """
    fields = db_fields(db, sc, pr, price_table)
    if before_gh:
        before_gh()
    quality, gh_pr = gh.quality(pr)
    block = sc.obj(assemble(fields, quality, gh_pr))
    body = render(block)
    sc.assert_clean(body, f"PR {pr} stats comment")
    return body, block
