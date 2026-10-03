"""Cache-policy simulator: replay each transcript under a 5-minute and a 1-hour cache TTL.

For every request, the cacheable prefix is P = cache_read + cache_write_5m +
cache_write_1h (the context minus uncached input). Replayed under a TTL:

- warm (the gap since the previous request in the transcript is within the
  TTL, and no compaction came between): read min(P, previous P) from cache,
  write the rest;
- cold (first request, a compaction, or a gap past the TTL): read only the
  floor, write the rest. The floor is the smallest cache read the transcript
  actually got on its own cold requests (its first request, the one after a
  compaction, and any after a gap past the TTL it used): the part of the
  prefix that other sessions kept warm, such as the shared system prompt.

Input and output are the same under both policies and are left out. Each
transcript is also replayed under the TTL it actually used, and the report
gives that replay's ratio to the observed cache cost as its calibration, so a
reader can see how far the model is from what happened.

USD here is the same list-price estimate as the cost layer: a plan-usage
proxy, not a bill.
"""

from ..stats import share
from .grouping import GAP_BUCKETS, gap_bucket

TTL_S = {"5m": 300, "1h": 3600}


def replay(requests, price, floor):
    """Cache USD for one transcript under each TTL, plus the observed cost.

    requests: dicts in time order with cache_read, cache_write_5m,
    cache_write_1h, prev_gap_s and after_compaction.
    price: dict with cache_read, cache_write_5m, cache_write_1h (USD per MTok).
    """
    out = {"observed": 0.0, "5m": 0.0, "1h": 0.0}
    prev = None
    for r in requests:
        prefix = r["cache_read"] + r["cache_write_5m"] + r["cache_write_1h"]
        out["observed"] += (r["cache_read"] * price["cache_read"] + r["cache_write_5m"] * price["cache_write_5m"]
                            + r["cache_write_1h"] * price["cache_write_1h"]) / 1e6
        for policy, ttl in TTL_S.items():
            warm = prev is not None and not r["after_compaction"] and r["prev_gap_s"] is not None \
                and r["prev_gap_s"] <= ttl
            read = min(prefix, prev) if warm else min(prefix, floor)
            write = prefix - read
            out[policy] += (read * price["cache_read"] + write * price["cache_write_" + policy]) / 1e6
        prev = prefix
    return out


def observed_policy(requests):
    w5 = sum(r["cache_write_5m"] for r in requests)
    w1h = sum(r["cache_write_1h"] for r in requests)
    if not w5 and not w1h:
        return "none"
    return "1h" if w1h >= w5 else "5m"


def cold_floor(requests):
    """The smallest cache read on a request that was cold under the TTL the transcript used."""
    ttl = TTL_S.get(observed_policy(requests), TTL_S["5m"])
    cold = [r["cache_read"] for i, r in enumerate(requests)
            if i == 0 or r["after_compaction"] or r["prev_gap_s"] is None or r["prev_gap_s"] > ttl]
    return min(cold)


def build(db, sc):
    prices = {r["model"]: dict(r) for r in db.execute("SELECT * FROM wprice")}
    after = {r[0] for r in db.execute("SELECT request_after FROM compactions WHERE request_after IS NOT NULL")}
    rows = db.execute("SELECT request_id, session_id, COALESCE(agent_id, '') AS transcript, agent_type, model,"
                      " cache_read, cache_write_5m, cache_write_1h, prev_gap_s FROM wreq"
                      " ORDER BY session_id, transcript, ts").fetchall()
    transcripts = {}
    for r in rows:
        d = dict(r)
        d["after_compaction"] = r["request_id"] in after
        transcripts.setdefault((r["session_id"], r["transcript"]), []).append(d)

    groups, unpriced = {}, 0
    for reqs in transcripts.values():
        key = sc.label(reqs[0]["agent_type"], "agent_type")
        g = groups.setdefault(key, {"transcripts": 0, "requests": 0, "observed_usd": 0.0, "sim_5m_usd": 0.0,
                                    "sim_1h_usd": 0.0, "calibration_sim": 0.0, "calibration_obs": 0.0,
                                    "observed_policy": {}, "gaps": {b: 0 for b in GAP_BUCKETS}})
        g["transcripts"] += 1
        # Replay per model run, since price differs by model; a transcript rarely mixes models.
        runs = {}
        for r in reqs:
            runs.setdefault(r["model"], []).append(r)
            g["gaps"][gap_bucket(r["prev_gap_s"])] += 1
        for model, run in runs.items():
            if model not in prices:
                unpriced += len(run)
                continue
            res = replay(run, prices[model], cold_floor(run))
            g["requests"] += len(run)
            g["observed_usd"] += res["observed"]
            g["sim_5m_usd"] += res["5m"]
            g["sim_1h_usd"] += res["1h"]
            policy = observed_policy(run)
            g["observed_policy"][policy] = g["observed_policy"].get(policy, 0) + 1
            if policy in TTL_S:
                g["calibration_sim"] += res[policy]
                g["calibration_obs"] += res["observed"]

    out = []
    for key, g in sorted(groups.items(), key=lambda kv: -kv[1]["observed_usd"]):
        best = "1h" if g["sim_1h_usd"] < g["sim_5m_usd"] else "5m"
        out.append({
            "agent_type": key, "transcripts": g["transcripts"], "requests": g["requests"],
            "observed_policy": g["observed_policy"], "gaps": g["gaps"],
            "observed_usd": g["observed_usd"], "sim_5m_usd": g["sim_5m_usd"], "sim_1h_usd": g["sim_1h_usd"],
            "better_policy": best,
            "saving_vs_other_usd": abs(g["sim_5m_usd"] - g["sim_1h_usd"]),
            "calibration": share(g["calibration_sim"], g["calibration_obs"]) if g["calibration_obs"] else None,
        })
    return {
        "layer": "cache-policy simulation",
        "note": ("Cache read and write USD per agent type, replayed under 5m and 1h TTLs over the observed idle "
                 "gaps. calibration = replay under the observed TTL / observed cache USD (1.0 is exact). "
                 "Estimated list price, a plan-usage proxy."),
        "by_agent_type": out,
        "unpriced_requests": unpriced,
    }
