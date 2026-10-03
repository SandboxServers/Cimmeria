"""Cost per merged PR: the outcome-normalized metric.

A PR's numbers are the weighted sums of its pr_attribution rows
(attribution.md), over its whole history, not only what falls in the window:
the window picks which PRs merged, not which of their requests count.
Unattributed spend is reported next to the totals, never spread onto PRs.

pr_records() returns the fields of the cimmeria-pr-stats/1 block that the
database can answer. TP-10's pr_stats.py adds the version stamp and the
quality fields (CI and review rounds) on top.
"""

from ..db import agent_type_sql
from ..stats import distribution, percentile, share, top_share
from .cost import LABEL
from .grouping import TOKEN_COLUMNS

EVENT_KINDS = ("background_completion", "monitor_event", "idle_notification", "teammate_message",
               "agent_message", "cross_session_message", "scheduled_task")
TOKEN_KEYS = {"input_tokens": "input", "output_tokens": "output", "thinking_tokens": "thinking",
              "cache_read": "cache_read", "cache_write_5m": "cache_write_5m", "cache_write_1h": "cache_write_1h"}


def build(db, sc, top=20):
    merged = [r[0] for r in db.execute(
        "SELECT pr_number FROM prs WHERE state = 'MERGED' AND merged_at >= (SELECT since FROM report_window)"
        " AND merged_at < (SELECT until FROM report_window) ORDER BY pr_number")]
    records = pr_records(db, sc, merged)
    usd = [r["usd_est"] for r in records]

    coverage = {k: 0.0 for k in ("merged_in_window", "other_prs", "unattributed")}
    merged_set = set(merged)
    for pr, spend in db.execute("SELECT a.pr_number, SUM(a.weight * COALESCE(c.usd, 0)) FROM wcost c"
                                " JOIN pr_attribution a ON a.request_id = c.request_id GROUP BY a.pr_number"):
        key = "unattributed" if pr is None else "merged_in_window" if pr in merged_set else "other_prs"
        coverage[key] += spend or 0.0
    window_usd = sum(coverage.values())
    method_mix = {sc.label(m, "method"): {"usd": u or 0.0, "share": share(u or 0.0, window_usd)}
                  for m, u in db.execute("SELECT a.method, SUM(a.weight * COALESCE(c.usd, 0)) FROM wcost c"
                                         " JOIN pr_attribution a ON a.request_id = c.request_id"
                                         " GROUP BY a.method ORDER BY 2 DESC")}

    return {
        "layer": "cost per merged PR",
        "label": LABEL,
        "merged_prs": len(records),
        "per_pr": {
            "usd_est": distribution(usd),
            "requests": distribution([r["requests"] for r in records]),
            "peak_context": distribution([r["context"]["peak"] for r in records]),
            "unattributed_share": distribution([r["attribution"]["unattributed_share"] for r in records]),
        },
        "top_10pct_share": top_share(usd, 0.10),
        "window_spend": {"usd": window_usd, **{k: {"usd": v, "share": share(v, window_usd)}
                                               for k, v in coverage.items()}},
        "method_mix": method_mix,
        "top": sorted(records, key=lambda r: -r["usd_est"])[:top],
    }


def pr_records(db, sc, pr_numbers):
    """One cimmeria-pr-stats/1-shaped dict per PR, in the order given."""
    db.execute("DROP TABLE IF EXISTS temp.report_prs")
    db.execute("CREATE TEMP TABLE report_prs (pr_number INTEGER PRIMARY KEY)")
    db.executemany("INSERT INTO report_prs VALUES (?)", [(n,) for n in pr_numbers])
    db.execute("DROP TABLE IF EXISTS temp.report_pa")
    db.execute("""
        CREATE TEMP TABLE report_pa AS
        SELECT a.pr_number, a.method, a.weight, a.confidence, r.request_id, r.session_id, r.agent_id, r.ts,
               r.model, r.trigger_id, r.context_tokens, r.prev_gap_s, r.input_tokens, r.output_tokens,
               r.thinking_tokens, r.cache_read, r.cache_write_5m, r.cache_write_1h,
               COALESCE(c.usd, 0) AS usd, t.kind AS trigger_kind,
               """ + agent_type_sql("ag") + """ AS agent_type
        FROM pr_attribution a
        JOIN report_prs p ON p.pr_number = a.pr_number
        JOIN requests r ON r.request_id = a.request_id
        JOIN rcost c ON c.request_id = a.request_id
        JOIN triggers t ON t.trigger_id = r.trigger_id
        LEFT JOIN agents ag ON ag.agent_id = r.agent_id""")

    tok = ", ".join(f"SUM(weight * {c}) AS {c}" for c in TOKEN_COLUMNS)
    base = {r["pr_number"]: r for r in db.execute(
        f"SELECT pr_number, SUM(weight) AS requests, SUM(weight * usd) AS usd, {tok},"
        " SUM(weight * confidence) / SUM(weight) AS confidence, MIN(ts) AS first_ts, MAX(context_tokens) AS peak,"
        " SUM(CASE WHEN prev_gap_s > 300 THEN weight * (cache_write_5m + cache_write_1h) ELSE 0 END) AS idle_writes,"
        " SUM(weight * (cache_write_5m + cache_write_1h)) AS writes"
        " FROM report_pa GROUP BY pr_number")}
    prs = {r["pr_number"]: r for r in db.execute(
        "SELECT p.* FROM prs p JOIN report_prs USING (pr_number)")}

    def grouped(sql):
        out = {}
        for row in db.execute(sql):
            out.setdefault(row[0], []).append(row)
        return out

    by_model = grouped("SELECT pr_number, model, SUM(weight * usd) FROM report_pa GROUP BY 1, 2 ORDER BY 3 DESC")
    methods = grouped("SELECT pr_number, method, SUM(weight * usd), SUM(weight) FROM report_pa GROUP BY 1, 2")
    triggers = grouped("SELECT DISTINCT pr_number, trigger_id, trigger_kind FROM report_pa")
    agents = grouped("SELECT pr_number, agent_type, COUNT(DISTINCT agent_id), SUM(weight), SUM(weight * usd)"
                     " FROM report_pa WHERE agent_id IS NOT NULL GROUP BY 1, 2 ORDER BY 5 DESC")
    contexts = grouped("SELECT pr_number, context_tokens FROM report_pa")
    compactions = dict(db.execute(
        "SELECT pa.pr_number, COUNT(DISTINCT c.boundary_uuid) FROM compactions c"
        " JOIN report_pa pa ON pa.request_id = c.request_after GROUP BY 1").fetchall())
    tools = grouped("SELECT pa.pr_number, tc.tool_name, tc.fingerprint, SUM(pa.weight * COALESCE(tc.result_chars, 0)),"
                    " SUM(pa.weight * COALESCE(tc.exposure_chars, 0))"
                    " FROM tool_calls tc JOIN report_pa pa ON pa.request_id = tc.request_id GROUP BY 1, 2, 3")
    unattributed = dict(db.execute("""
        WITH win AS (
            SELECT pa.pr_number, MIN(pa.ts) AS first_ts,
                   COALESCE(p.merged_at, MAX(pa.ts)) AS end_ts
            FROM report_pa pa JOIN prs p ON p.pr_number = pa.pr_number GROUP BY pa.pr_number),
        sess AS (SELECT DISTINCT pr_number, session_id FROM report_pa)
        SELECT s.pr_number, SUM(u.weight * COALESCE(c.usd, 0))
        FROM sess s
        JOIN win w ON w.pr_number = s.pr_number
        JOIN requests r ON r.session_id = s.session_id AND r.ts >= w.first_ts AND r.ts <= w.end_ts
        JOIN pr_attribution u ON u.request_id = r.request_id AND u.pr_number IS NULL
        JOIN rcost c ON c.request_id = r.request_id
        GROUP BY s.pr_number""").fetchall())

    records = []
    for n in pr_numbers:
        b, p = base.get(n), prs.get(n)
        usd = (b["usd"] if b else 0.0) or 0.0
        un = unattributed.get(n) or 0.0
        mix = methods.get(n, [])
        mix_total = sum(m[2] for m in mix) or sum(m[3] for m in mix)
        method = None
        if mix:
            idx = 2 if sum(m[2] for m in mix) else 3
            best = max(mix, key=lambda m: m[idx])
            method = best[1] if share(best[idx], mix_total) > 0.5 else "split"
        kinds = [t[2] for t in triggers.get(n, [])]
        fps = {}
        for _, tool, fp, chars, exposure in tools.get(n, []):
            key = (sc.label(tool, "tool_name"), sc.fingerprint(tool, fp))
            acc = fps.setdefault(key, [0.0, 0.0])
            acc[0] += chars or 0
            acc[1] += exposure or 0
        tool_chars = {}
        for (tool, _), (chars, _) in fps.items():
            tool_chars[tool] = tool_chars.get(tool, 0.0) + chars
        top_exposure = sorted(fps.items(), key=lambda kv: -kv[1][1])[:5]
        records.append({
            "pr": n,
            "window": {"first": b["first_ts"] if b else None, "merged": p["merged_at"] if p else None},
            "attribution": {"method": sc.label(method, "method"),
                            "confidence": round(b["confidence"], 3) if b else 0.0,
                            "unattributed_share": round(share(un, un + usd), 4)},
            "usd_est": round(usd, 4),
            "by_model": {sc.label(m[1], "model"): round(m[2] or 0.0, 4) for m in by_model.get(n, [])},
            "tokens": {TOKEN_KEYS[c]: round(b[c] or 0) if b else 0 for c in TOKEN_COLUMNS},
            "requests": round(b["requests"], 2) if b else 0,
            "turns": len(kinds),
            "human_prompts": kinds.count("human_prompt"),
            "event_triggers": {k: kinds.count(k) for k in EVENT_KINDS if kinds.count(k)},
            "agents": {sc.label(a[1] or "unknown-agent", "agent_type"):
                       {"count": a[2], "requests": round(a[3], 2), "usd_est": round(a[4] or 0.0, 4)}
                       for a in agents.get(n, [])},
            "context": {"peak": b["peak"] if b else 0,
                        "p50": percentile(sorted(c[1] for c in contexts.get(n, [])), 50) or 0,
                        "compactions": compactions.get(n, 0),
                        "idle_gap_write_share": round(share(b["idle_writes"], b["writes"]), 4) if b else 0.0},
            "tools": {"bash_chars": round(tool_chars.get("Bash", 0)), "read_chars": round(tool_chars.get("Read", 0)),
                      "top_exposure": [{"tool": k[0], "fingerprint": k[1], "exposure_chars": round(v[1])}
                                       for k, v in top_exposure if v[1]]},
            "diff": {"additions": p["additions"], "deletions": p["deletions"], "files": p["changed_files"]}
            if p else {},
        })
    return records
