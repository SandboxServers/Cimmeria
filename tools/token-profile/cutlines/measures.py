"""Before-and-after measures across the ledger's cut lines.

Each measure compares the transcripts (or calls) on either side of the cut of
the packet that targeted it:

- **Static context** (TP-03): the context tokens of each transcript's first
  request, by agent type, for transcripts whose first request falls before or
  after the cut.
- **Lane output** (TP-02): result characters of foreground `lane.sh` calls.
  A background call's result is only its task notice, so those are counted
  apart.
- **Worker lifetime** (TP-00): requests and peak context per subagent
  transcript that started before or after the cut. Transcripts still running
  when the database was ingested are cut short, so the after side is a floor.
"""

from report.db import agent_type_sql
from report.stats import distribution

LANE = "bash tools/build-lane/lane.sh"


def _side(ts, cut):
    return "before" if ts < cut else "after"


def static_context(db, sc, since, cut):
    rows = db.execute(
        "SELECT * FROM (SELECT r.context_tokens, r.ts,"
        " ROW_NUMBER() OVER (PARTITION BY r.session_id, COALESCE(r.agent_id, '') ORDER BY r.ts, r.rowid) AS k,"
        " CASE WHEN r.agent_id IS NULL THEN CASE WHEN s.is_coordinator = 1 THEN 'main (coordinator)' ELSE 'main' END"
        "      ELSE " + agent_type_sql("a", "'unknown-agent'") + " END AS agent_type"
        " FROM requests r JOIN sessions s USING (session_id) LEFT JOIN agents a ON a.agent_id = r.agent_id)"
        " WHERE k = 1 AND ts >= ?", (since,)).fetchall()
    out = {}
    for r in rows:
        key = sc.label(r["agent_type"], "agent_type")
        out.setdefault(key, {"before": [], "after": []})[_side(r["ts"], cut)].append(r["context_tokens"])
    by_scope = {}
    for r in rows:
        scope = "main" if r["agent_type"].startswith("main") else "subagent"
        by_scope.setdefault(scope, {"before": [], "after": []})[_side(r["ts"], cut)].append(r["context_tokens"])
    return {"by_scope": {k: {s: distribution(v[s]) for s in v} for k, v in sorted(by_scope.items())},
            # Only the types seen after the cut; the baseline report has every type before it.
            "by_agent_type": {k: {s: distribution(v[s]) for s in v} for k, v in sorted(out.items()) if v["after"]}}


def lane_output(db, since, cut):
    rows = db.execute("SELECT ts, result_chars, task_id IS NOT NULL AS background FROM tool_calls"
                      " WHERE fingerprint = ? AND tool_name IN ('Bash', 'PowerShell') AND result_chars IS NOT NULL"
                      " AND ts >= ?", (LANE, since)).fetchall()
    out = {}
    for mode in ("foreground", "background"):
        sel = [r for r in rows if bool(r["background"]) == (mode == "background")]
        out[mode] = {s: distribution([r["result_chars"] for r in sel if _side(r["ts"], cut) == s])
                     for s in ("before", "after")}
    return out


def worker_lifetime(db, since, cut, cap=200):
    rows = db.execute("SELECT MIN(ts) AS first, COUNT(*) AS n, MAX(context_tokens) AS peak FROM requests"
                      " WHERE agent_id IS NOT NULL GROUP BY agent_id HAVING MIN(ts) >= ?", (since,)).fetchall()
    out = {}
    for s in ("before", "after"):
        sel = [r for r in rows if _side(r["first"], cut) == s]
        out[s] = {"requests": distribution([r["n"] for r in sel]), "peak_context": distribution([r["peak"] for r in sel]),
                  f"over_{cap}_requests": sum(1 for r in sel if r["n"] > cap)}
    return out


def build(db, sc, since, cuts):
    """cuts: {'TP-00': iso, 'TP-02': iso, 'TP-03': iso}."""
    last = db.execute("SELECT MAX(ts) FROM requests").fetchone()[0]
    return {
        "schema": "cimmeria-token-cutlines/1",
        "since": since,
        "last_request": last,
        "cuts": cuts,
        "static_context": {"cut": "TP-03", **static_context(db, sc, since, cuts["TP-03"])},
        "lane_output": {"cut": "TP-02", **lane_output(db, since, cuts["TP-02"])},
        "worker_lifetime": {"cut": "TP-00", **worker_lifetime(db, since, cuts["TP-00"])},
    }
