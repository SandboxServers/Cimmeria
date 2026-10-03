"""Layer 3: context pressure. Token counts of what each request carried; never priced."""

from ..stats import distribution, share
from .grouping import GAP_BUCKETS, gap_bucket

NOTE = ("Context pressure: how much each request re-read. These are token counts, not cost; "
        "the cost of carrying context is already inside the cost layer.")


def _by_key(rows, key, value):
    out = {}
    for r in rows:
        out.setdefault(r[key], []).append(r[value])
    return out


def build(db, sc):
    reqs = db.execute("SELECT session_id, COALESCE(agent_id, '') AS transcript, scope, agent_type, ts,"
                      " context_tokens, cache_write_5m + cache_write_1h AS cache_write, prev_gap_s"
                      " FROM wreq ORDER BY session_id, transcript, ts").fetchall()

    per_request = {"all": distribution([r["context_tokens"] for r in reqs])}
    for scope, values in _by_key(reqs, "scope", "context_tokens").items():
        per_request[scope] = distribution(values)

    transcripts = {}
    for r in reqs:
        t = transcripts.setdefault((r["session_id"], r["transcript"]), {
            "scope": r["scope"], "agent_type": r["agent_type"], "first": r["context_tokens"],
            "peak": 0, "requests": 0})
        t["peak"] = max(t["peak"], r["context_tokens"])
        t["requests"] += 1

    def per_transcript(field):
        out = {"all": distribution([t[field] for t in transcripts.values()])}
        for scope in ("main", "subagent"):
            vals = [t[field] for t in transcripts.values() if t["scope"] == scope]
            if vals:
                out[scope] = distribution(vals)
        return out

    # The first request of a transcript is the static context it paid before doing anything.
    # It is only the true first request when the window holds the transcript's start.
    first_by_type = {}
    for t in transcripts.values():
        first_by_type.setdefault(sc.label(t["agent_type"], "agent_type"), []).append(t["first"])

    gaps = {}
    for r in reqs:
        g = gaps.setdefault((r["scope"], gap_bucket(r["prev_gap_s"])), {"requests": 0, "cache_write": 0})
        g["requests"] += 1
        g["cache_write"] += r["cache_write"]
    idle_gap = {}
    for scope in ("main", "subagent"):
        total = sum(v["cache_write"] for (s, _), v in gaps.items() if s == scope)
        rows = []
        for bucket in GAP_BUCKETS:
            v = gaps.get((scope, bucket), {"requests": 0, "cache_write": 0})
            rows.append({"gap": bucket, **v, "write_share": share(v["cache_write"], total)})
        idle_gap[scope] = rows

    return {
        "layer": "context pressure",
        "note": NOTE,
        "per_request": per_request,
        "peak_per_transcript": per_transcript("peak"),
        "requests_per_transcript": per_transcript("requests"),
        "first_request_by_agent_type": {k: distribution(v) for k, v in sorted(first_by_type.items())},
        "cache_writes_by_idle_gap": idle_gap,
        "compactions": compactions(db, sc, len(transcripts)),
    }


def compactions(db, sc, transcripts):
    rows = db.execute(
        "SELECT c.trigger, c.pre_tokens, c.post_tokens, c.dropped_tokens, c.duration_ms,"
        " CASE WHEN c.agent_id IS NULL THEN 'main' ELSE 'subagent' END AS scope"
        " FROM compactions c WHERE c.ts >= (SELECT since FROM report_window)"
        " AND c.ts < (SELECT until FROM report_window)").fetchall()
    by_trigger = {}
    for r in rows:
        key = sc.label(r["trigger"], "compaction_trigger")
        by_trigger[key] = by_trigger.get(key, 0) + 1
    return {
        "count": len(rows),
        "transcripts_in_window": transcripts,
        "by_trigger": by_trigger,
        "by_scope": {s: sum(1 for r in rows if r["scope"] == s) for s in ("main", "subagent")},
        "pre_tokens": distribution([r["pre_tokens"] for r in rows]),
        "post_tokens": distribution([r["post_tokens"] for r in rows]),
        "dropped_tokens": distribution([r["dropped_tokens"] for r in rows]),
        "duration_ms": distribution([r["duration_ms"] for r in rows]),
    }
