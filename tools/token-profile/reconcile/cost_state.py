"""Reconcile the profiler against Claude Code's own cost-state records.

A cost-state record is the running total of one Claude Code process: every
request it made from its start (`process_start`) on, subagents included. A
resumed session's earlier requests belong to an earlier process whose record
was usually never written, so the profiler is compared over the same window:
the session's requests at or after `process_start`. What falls outside every
window is reported as uncovered, not compared.

Three things can differ, and each has its own check:

- **Prices.** The cost-state's own tokens, repriced with the profiler's table,
  must give the cost-state's USD. A miss means the price table is wrong.
- **Tokens the transcripts hold.** Output tokens come almost only from
  requests the transcripts record, so they must agree closely. A dedupe bug
  (keeping a streaming partial, or a copy counted twice) shows here first.
- **Requests the transcripts don't hold.** Claude Code makes requests that
  never reach a transcript but are in the cost-state: large cache reads,
  fresh input, little output. OTel's `query_source` is expected to name
  them (see otel.py). The profiler may fall short of the cost-state by up
  to `undercount`, and never exceed it by more than `overcount`.
"""

import json

from ingest.prices import normalize_model

from report.stats import distribution

# Tolerances, as shares of the cost-state value. Measured on 2026-10-03 over
# 94 sessions (2026-09-14 to 2026-10-03): price residual 0.03%, output gap
# 1.1%, profiler 10.2% under the cost-state; of 69 sessions with 20 or more
# requests none was over it (the closest was 0.002% under).
TOLERANCES = {
    "price_residual": 0.01,
    "output_gap": 0.03,
    "undercount": 0.15,
    "overcount": 0.01,
    "session_overcount": 0.02,
}
# Sessions with fewer requests are left out of the per-session check: one
# rounding cent is a large share of a 10-cent session.
MIN_SESSION_REQUESTS = 20

# Server tools are billed per use, not per token. Web search is $10 per 1,000
# searches (platform.claude.com pricing page, 2026-10-03); price_tables has no
# column for it, so it is applied here only.
WEB_SEARCH_USD = 0.01

TOKEN_KEYS = (("input", "inputTokens"), ("output", "outputTokens"), ("cache_read", "cacheReadInputTokens"),
              ("cache_write", "cacheCreationInputTokens"))


def _prices(db):
    return {r["model"]: dict(r) for r in db.execute("SELECT * FROM wprice")}


def _usd(row, t):
    return (t["input"] * row["input"] + t["output"] * row["output"] + t["cache_read"] * row["cache_read"]
            + t["w5"] * row["cache_write_5m"] + t["w1"] * row["cache_write_1h"]) / 1e6


def _profiler_by_model(db, session_id, start):
    out = {}
    for r in db.execute(
            "SELECT model, COUNT(*) AS n, SUM(input_tokens) AS input, SUM(output_tokens) AS output,"
            " SUM(cache_read) AS cache_read, SUM(cache_write_5m) AS w5, SUM(cache_write_1h) AS w1,"
            " SUM(web_search) AS web_search FROM requests WHERE session_id = ? AND ts >= ? GROUP BY model",
            (session_id, start or "")):
        m = normalize_model(r["model"])
        acc = out.setdefault(m, {k: 0 for k in ("n", "input", "output", "cache_read", "w5", "w1", "web_search")})
        for k in acc:
            acc[k] += r[k] or 0
    return out


def _cost_state_by_model(model_usage_json):
    out = {}
    for model, u in json.loads(model_usage_json or "{}").items():
        m = normalize_model(model)
        acc = out.setdefault(m, {"usd": 0.0, "web_search": 0, **{k: 0 for k, _ in TOKEN_KEYS}})
        acc["usd"] += u.get("costUSD") or 0
        acc["web_search"] += u.get("webSearchRequests") or 0
        for k, src in TOKEN_KEYS:
            acc[k] += u.get(src) or 0
    return out


def _share(part, whole):
    return part / whole if whole else 0.0


def build(db, sc, since=None, until=None):
    """The reconciliation as a dict of aggregates. Needs report.db.scope() to have run (for wprice)."""
    prices = _prices(db)
    lo, hi = since or "", until or "￿"
    rows = db.execute(
        "SELECT c.session_id, c.total_cost_usd, c.model_usage_json, c.process_start, s.first_ts"
        " FROM cost_states c JOIN sessions s USING (session_id)"
        " WHERE COALESCE(c.process_start, s.first_ts) >= ? AND COALESCE(c.process_start, s.first_ts) < ?",
        (lo, hi)).fetchall()

    models = {}
    sessions = []
    totals = {"cost_state_usd": 0.0, "profiler_usd": 0.0, "repriced_usd": 0.0, "unpriced_cost_state_usd": 0.0}
    for r in rows:
        pr = _profiler_by_model(db, r["session_id"], r["process_start"])
        cs = _cost_state_by_model(r["model_usage_json"])
        session_pr_usd = 0.0
        n = 0
        for m in set(pr) | set(cs):
            p = pr.get(m, {"n": 0, "input": 0, "output": 0, "cache_read": 0, "w5": 0, "w1": 0, "web_search": 0})
            c = cs.get(m, {"usd": 0.0, "web_search": 0, **{k: 0 for k, _ in TOKEN_KEYS}})
            price = prices.get(m)
            acc = models.setdefault(m, {"requests": 0, "cost_state_usd": 0.0, "profiler_usd": 0.0,
                                        "repriced_usd": 0.0, "priced": price is not None,
                                        "cost_state": {k: 0 for k, _ in TOKEN_KEYS},
                                        "profiler": {k: 0 for k, _ in TOKEN_KEYS}})
            acc["requests"] += p["n"]
            n += p["n"]
            acc["cost_state_usd"] += c["usd"]
            for k, _ in TOKEN_KEYS:
                acc["cost_state"][k] += c[k]
                acc["profiler"][k] += p["w5"] + p["w1"] if k == "cache_write" else p[k]
            if price is None:
                totals["unpriced_cost_state_usd"] += c["usd"]
                continue
            pusd = _usd(price, p) + p["web_search"] * WEB_SEARCH_USD
            session_pr_usd += pusd
            acc["profiler_usd"] += pusd
            # The cost-state doesn't split cache writes by TTL; use the profiler's split for the same
            # session and model, or 5m when the profiler saw no write.
            writes = p["w5"] + p["w1"]
            f1h = p["w1"] / writes if writes else 0.0
            repriced = _usd(price, {"input": c["input"], "output": c["output"], "cache_read": c["cache_read"],
                                    "w5": c["cache_write"] * (1 - f1h), "w1": c["cache_write"] * f1h})
            repriced += c["web_search"] * WEB_SEARCH_USD
            acc["repriced_usd"] += repriced
            totals["repriced_usd"] += repriced
        totals["cost_state_usd"] += r["total_cost_usd"]
        totals["profiler_usd"] += session_pr_usd
        sessions.append({"requests": n, "cost_state_usd": r["total_cost_usd"], "profiler_usd": session_pr_usd})

    uncovered = _uncovered(db, prices, lo, hi)
    priced_cs = totals["cost_state_usd"] - totals["unpriced_cost_state_usd"]
    out_cs = sum(m["cost_state"]["output"] for m in models.values())
    out_pr = sum(m["profiler"]["output"] for m in models.values())
    big = [s for s in sessions if s["requests"] >= MIN_SESSION_REQUESTS and s["cost_state_usd"] > 0]
    over = [s for s in big if s["profiler_usd"] - s["cost_state_usd"]
            > TOLERANCES["session_overcount"] * s["cost_state_usd"]]
    gap = _share(totals["profiler_usd"] - totals["cost_state_usd"], totals["cost_state_usd"])
    measures = {
        "price_residual": abs(_share(totals["repriced_usd"] - priced_cs, priced_cs)),
        "output_gap": abs(_share(out_pr - out_cs, out_cs)),
        "undercount": max(0.0, -gap),
        "overcount": max(0.0, gap),
        "session_overcount": len(over),
    }
    checks = [{"name": k, "value": v, "limit": 0 if k == "session_overcount" else TOLERANCES[k],
               "ok": v <= (0 if k == "session_overcount" else TOLERANCES[k])} for k, v in measures.items()]
    if not rows:
        checks = [{"name": "sessions", "value": 0, "limit": 1, "ok": False}]
    return {
        "source": "cost-state",
        "sessions": len(rows),
        "sessions_checked_individually": len(big),
        "min_session_requests": MIN_SESSION_REQUESTS,
        "totals": {**totals, "gap_share": gap},
        "uncovered": uncovered,
        "by_model": [{"model": sc.label(m, "model"), **v,
                      "gap": {k: _share(v["profiler"][k] - v["cost_state"][k], v["cost_state"][k])
                              for k, _ in TOKEN_KEYS}}
                     for m, v in sorted(models.items(), key=lambda kv: -kv[1]["cost_state_usd"])],
        "per_session_gap_share": distribution([_share(s["profiler_usd"] - s["cost_state_usd"], s["cost_state_usd"])
                                               for s in big]),
        "checks": checks,
        "ok": all(c["ok"] for c in checks),
    }


def _uncovered(db, prices, lo, hi):
    """Profiler spend in the window that no cost-state covers: sessions without one, and requests a
    session made before the process that wrote its cost-state started (a resumed session)."""
    out = {"requests": 0, "usd": 0.0, "sessions_without_cost_state": 0, "resumed_sessions": 0}
    rows = db.execute(
        "SELECT r.model, r.input_tokens AS input, r.output_tokens AS output, r.cache_read, r.cache_write_5m AS w5,"
        " r.cache_write_1h AS w1, r.web_search, c.session_id IS NULL AS no_cs, r.session_id"
        " FROM requests r LEFT JOIN cost_states c USING (session_id)"
        " WHERE r.ts >= ? AND r.ts < ? AND (c.session_id IS NULL OR r.ts < COALESCE(c.process_start, ''))",
        (lo, hi)).fetchall()
    no_cs, resumed = set(), set()
    for r in rows:
        price = prices.get(normalize_model(r["model"]))
        out["requests"] += 1
        if price is not None:
            out["usd"] += _usd(price, r) + r["web_search"] * WEB_SEARCH_USD
        (no_cs if r["no_cs"] else resumed).add(r["session_id"])
    out["sessions_without_cost_state"] = len(no_cs)
    out["resumed_sessions"] = len(resumed)
    return out
