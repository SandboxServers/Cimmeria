"""Reconcile the profiler against Claude Code's OTel `claude_code.api_request` events.

The events come from a file, never from the network: an OTLP JSON export
(`resourceLogs`), a SigNoz log search result saved as JSON, or JSON Lines of
flat attribute objects. Each event is one API request with `session.id`,
`request_id`, `model`, `query_source`, the token counts and `cost_usd`.

Joined on `request_id`, the two sources answer different questions:

- **Matched requests** must carry the same tokens. OTel has one cache-write
  count; the profiler's 5m and 1h writes are summed to compare.
- **OTel-only requests** are what the transcripts never record. They are
  grouped by `query_source` and model, which says what the cost-state
  undercount is made of.
- **Profiler-only requests** inside the time span OTel covers for a session
  are telemetry losses (export failures, a batch not flushed at exit).
"""

import json
from pathlib import Path

from ingest.prices import normalize_model

EVENT = "claude_code.api_request"

TOLERANCES = {
    # Matched requests whose token counts differ in any category, as a share of matched requests.
    "matched_token_mismatch": 0.005,
    # |OTel cost_usd - profiler USD| over matched requests, as a share of OTel cost_usd.
    "matched_cost_residual": 0.01,
    # Profiler USD inside OTel's covered span with no event, as a share of profiler USD there.
    "profiler_only": 0.02,
}

_ATTR_MAPS = ("attributes", "attributes_string", "attributes_number", "attributes_int64", "attributes_float64",
              "attributes_bool", "resources_string", "resource", "resources", "resource_attributes")
_NUMERIC = ("input_tokens", "output_tokens", "cache_read_tokens", "cache_creation_tokens", "cost_usd")


def _otlp_value(v):
    if not isinstance(v, dict):
        return v
    for k in ("stringValue", "doubleValue", "boolValue"):
        if k in v:
            return v[k]
    if "intValue" in v:
        return int(v["intValue"])
    return None


def _otlp_attrs(items):
    return {a["key"]: _otlp_value(a.get("value")) for a in items or () if isinstance(a, dict) and "key" in a}


def _flatten(rec):
    """One event as a flat dict, whatever envelope it came in."""
    flat = {k: v for k, v in rec.items() if not isinstance(v, (dict, list))}
    for key in _ATTR_MAPS:
        v = rec.get(key)
        if isinstance(v, dict):
            flat.update({k: x for k, x in v.items() if not isinstance(x, (dict, list))})
        elif isinstance(v, list):
            flat.update(_otlp_attrs(v))
    data = rec.get("data")
    if isinstance(data, dict):
        flat.update(_flatten(data))
    return flat


def _from_otlp(doc):
    for rl in doc.get("resourceLogs") or ():
        res = _otlp_attrs((rl.get("resource") or {}).get("attributes"))
        for sl in rl.get("scopeLogs") or ():
            for lr in sl.get("logRecords") or ():
                body = _otlp_value(lr.get("body"))
                ts = lr.get("timeUnixNano")
                yield {**res, **_otlp_attrs(lr.get("attributes")), "body": body,
                       "timestamp": int(ts) // 1_000_000 if ts else None}


def _walk(doc):
    """Every dict in a JSON document that looks like a log row."""
    if isinstance(doc, list):
        for x in doc:
            yield from _walk(x)
    elif isinstance(doc, dict):
        if "resourceLogs" in doc:
            yield from _from_otlp(doc)
            return
        flat = _flatten(doc)
        if flat.get("body") == EVENT or flat.get("event.name") in (EVENT, "api_request"):
            yield flat
            return
        for v in doc.values():
            if isinstance(v, (dict, list)):
                yield from _walk(v)


def _iso(ts):
    """Event time as ISO-8601 UTC, from epoch ms/ns or an ISO string."""
    from datetime import datetime, timezone
    if isinstance(ts, str) and not ts.isdigit():
        return ts if ts.endswith("Z") else ts.replace("+00:00", "Z")
    try:
        n = int(ts)
    except (TypeError, ValueError):
        return None
    if n > 10 ** 15:
        n //= 1_000_000
    return datetime.fromtimestamp(n / 1000, timezone.utc).strftime("%Y-%m-%dT%H:%M:%S.%f")[:-3] + "Z"


def load(path):
    """The api_request events in a file, normalized. JSON or JSON Lines."""
    text = Path(path).read_text(encoding="utf-8")
    try:
        docs = [json.loads(text)]
    except ValueError:
        docs = [json.loads(line) for line in text.splitlines() if line.strip()]
    events = []
    for flat in (e for d in docs for e in _walk(d)):
        if flat.get("body") not in (EVENT, None) and flat.get("event.name") not in (EVENT, "api_request"):
            continue
        ev = {"session_id": flat.get("session.id") or flat.get("session_id"),
              "request_id": flat.get("request_id") or flat.get("request.id"),
              "model": normalize_model(str(flat.get("model") or "")),
              "query_source": flat.get("query_source") or "unknown",
              "ts": _iso(flat.get("event.timestamp") or flat.get("timestamp"))}
        for k in _NUMERIC:
            try:
                ev[k] = float(flat.get(k) or 0)
            except (TypeError, ValueError):
                ev[k] = 0.0
        events.append(ev)
    return events


def _share(part, whole):
    return part / whole if whole else 0.0


def compare(db, sc, events, cost_state_usd_by_session=None):
    """Aggregates only. Needs report.db.scope() to have run (for rcost)."""
    by_id = {}
    no_id = 0
    for e in events:
        if e["request_id"]:
            by_id[e["request_id"]] = e
        else:
            no_id += 1
    prof = {}
    span = {}
    for e in events:
        if e["session_id"] and e["ts"]:
            lo, hi = span.get(e["session_id"], (e["ts"], e["ts"]))
            span[e["session_id"]] = (min(lo, e["ts"]), max(hi, e["ts"]))
    for sid, (lo, hi) in span.items():
        for r in db.execute(
                "SELECT r.request_id, r.session_id, r.input_tokens, r.output_tokens, r.cache_read,"
                " r.cache_write_5m + r.cache_write_1h AS cache_write, c.usd"
                " FROM requests r JOIN rcost c USING (request_id) WHERE r.session_id = ? AND r.ts >= ? AND r.ts <= ?",
                (sid, lo, hi)):
            prof[r["request_id"]] = dict(r)
    # Requests OTel names that fall just outside a session's span still match.
    missing = [rid for rid in by_id if rid not in prof]
    for i in range(0, len(missing), 500):
        chunk = missing[i:i + 500]
        for r in db.execute(
                "SELECT r.request_id, r.session_id, r.input_tokens, r.output_tokens, r.cache_read,"
                " r.cache_write_5m + r.cache_write_1h AS cache_write, c.usd"
                f" FROM requests r JOIN rcost c USING (request_id) WHERE r.request_id IN ({','.join('?' * len(chunk))})",
                chunk):
            prof[r["request_id"]] = dict(r)

    matched = [(by_id[rid], p) for rid, p in prof.items() if rid in by_id]
    mismatched = [1 for e, p in matched
                  if (e["input_tokens"], e["output_tokens"], e["cache_read_tokens"], e["cache_creation_tokens"])
                  != (p["input_tokens"], p["output_tokens"], p["cache_read"], p["cache_write"])]
    otel_matched_usd = sum(e["cost_usd"] for e, _ in matched)
    prof_matched_usd = sum(p["usd"] or 0 for _, p in matched)

    otel_only = {}
    for rid, e in by_id.items():
        if rid in prof:
            continue
        key = (sc.label(e["query_source"], "query_source"), sc.label(e["model"], "model"))
        acc = otel_only.setdefault(key, {"requests": 0, "cost_usd": 0.0, "input_tokens": 0, "output_tokens": 0,
                                         "cache_read_tokens": 0, "cache_creation_tokens": 0})
        acc["requests"] += 1
        for k in ("cost_usd", "input_tokens", "output_tokens", "cache_read_tokens", "cache_creation_tokens"):
            acc[k] += e[k]
    prof_only = [p for rid, p in prof.items() if rid not in by_id]
    prof_span_usd = sum(p["usd"] or 0 for p in prof.values())
    prof_only_usd = sum(p["usd"] or 0 for p in prof_only)

    sessions = {}
    for e in events:
        if e["session_id"]:
            sessions[e["session_id"]] = sessions.get(e["session_id"], 0.0) + e["cost_usd"]
    vs_cost_state = None
    if cost_state_usd_by_session:
        both = [s for s in sessions if s in cost_state_usd_by_session]
        cs = sum(cost_state_usd_by_session[s] for s in both)
        ot = sum(sessions[s] for s in both)
        vs_cost_state = {"sessions": len(both), "otel_usd": ot, "cost_state_usd": cs, "gap_share": _share(ot - cs, cs)}

    measures = {
        "matched_token_mismatch": _share(len(mismatched), len(matched)),
        "matched_cost_residual": abs(_share(prof_matched_usd - otel_matched_usd, otel_matched_usd)),
        "profiler_only": _share(prof_only_usd, prof_span_usd),
    }
    checks = [{"name": k, "value": v, "limit": TOLERANCES[k], "ok": v <= TOLERANCES[k]} for k, v in measures.items()]
    if not matched:
        checks.append({"name": "matched_requests", "value": 0, "limit": 1, "ok": False})
    return {
        "source": "otel",
        "events": len(events),
        "events_without_request_id": no_id,
        "sessions": len(sessions),
        "matched": {"requests": len(matched), "token_mismatches": len(mismatched),
                    "otel_cost_usd": otel_matched_usd, "profiler_usd": prof_matched_usd},
        "otel_only": [{"query_source": k[0], "model": k[1], **v}
                      for k, v in sorted(otel_only.items(), key=lambda kv: -kv[1]["cost_usd"])],
        "otel_only_cost_usd": sum(v["cost_usd"] for v in otel_only.values()),
        "otel_cost_usd": sum(e["cost_usd"] for e in events),
        "profiler_only": {"requests": len(prof_only), "usd": prof_only_usd, "span_usd": prof_span_usd},
        "vs_cost_state": vs_cost_state,
        "checks": checks,
        "ok": all(c["ok"] for c in checks),
    }
