"""Per-regime roll-up of campaign records.

Cost and PR metrics use only pure campaigns (their own regime equals the
requested one); a mixed campaign's dollars belong to neither regime.
"""

from report.stats import distribution


def _sum(values):
    """Sum of the non-None values, or None when there are none."""
    vals = [v for v in values if v is not None]
    return sum(vals) if vals else None


def _ratio(part, whole, places=None):
    """part / whole, or None when either side is missing or the whole is zero."""
    if part is None or not whole:
        return None
    value = part / whole
    return round(value, places) if places is not None else value


def _share(flags, places=2):
    """Share of True among the flags that are not None, or None when there are none."""
    flags = [f for f in flags if f is not None]
    return round(sum(1 for f in flags if f) / len(flags), places) if flags else None


def _put(out, key, value, gap="no data"):
    """Store a metric; a metric with no denominator is None with a gap note, never 0."""
    out[key] = value
    if value is None:
        out[f"{key}_gap"] = gap


def rollup(records, regime):
    """Roll up the campaigns of one regime ("old" or "haiku") into one dict of metrics."""
    pure = [r for r in records.values() if r.get("regime") == regime]
    costs = [r.get("cost") or {} for r in pure]
    prs = [r.get("prs") or {} for r in pure]
    per_pr = [e for p in prs for e in (p.get("per_pr") or [])]

    total_usd = _sum(c.get("total_usd") for c in costs)
    merged_prs = _sum(p.get("merged") for p in prs)

    out = {"campaigns": len(pure)}
    _put(out, "prs", _sum(p.get("count") for p in prs))
    _put(out, "merged_prs", merged_prs)
    _put(out, "total_usd", total_usd)

    usd = distribution([e.get("usd") for e in per_pr if e.get("merged")])
    _put(out, "cost_per_pr", {"p50": usd["p50"], "mean": usd["mean"]} if usd["n"] else None)

    # The numerator and the denominator cover the same campaigns: those with a packet table.
    paid = [r for r in pure if ((r.get("accepted") or {}).get("packets") or 0) > 0]
    paid_usd = _sum((r.get("cost") or {}).get("total_usd") for r in paid)
    paid_packets = _sum((r.get("accepted") or {}).get("packets") for r in paid)
    _put(out, "cost_per_accepted_usd", _ratio(paid_usd, paid_packets), "no campaign has an accepted packet table")
    escaped = [r.get("escaped_defects") or {} for r in pure]
    _put(out, "escaped_defects_per_merged_pr", _ratio(_sum(e.get("count") for e in escaped), merged_prs, 2))
    _put(out, "ci_first_attempt_rate", _share(e.get("ci_first_pass") for e in per_pr))

    tiers = {}
    for c in costs:
        for tier, usd_tier in (c.get("by_model_tier") or {}).items():
            if usd_tier is not None:
                tiers[tier] = tiers.get(tier, 0.0) + usd_tier
    out["cost_share_by_tier"] = {t: round(u / total_usd, 2) for t, u in sorted(tiers.items())} if total_usd else {}

    # Overstep covers every campaign once: old ones for the old roll-up, haiku and mixed ones for the haiku roll-up.
    overstep_regimes = ("old",) if regime == "old" else ("haiku", "mixed")
    recoverable = [
        (r.get("overstep") or {}).get("recoverable_usd") or {}
        for r in records.values()
        if r.get("regime") in overstep_regimes
    ]
    low = _sum(x.get("low") for x in recoverable)
    high = _sum(x.get("high") for x in recoverable)
    _put(out, "overstep_recoverable_usd", {"low": low, "high": high} if (low, high) != (None, None) else None)

    # Packet metrics take every campaign's rows, mixed campaigns included, by the row's own regime.
    rows = [row for r in records.values() for row in (r.get("packet_rows") or []) if row.get("regime") == regime]
    accepted_rows = [row for row in rows if row.get("accepted")]
    cycles = distribution([row.get("cycle_hours") for row in rows])
    _put(out, "packets", len(accepted_rows))
    _put(out, "first_pass_rate", _share(row.get("first_pass") for row in accepted_rows))
    if regime == "haiku":
        retries = sum(len(row.get("retries") or []) for row in rows)
        _put(out, "retries_per_accepted", _ratio(retries, len(accepted_rows), 2))
        _put(out, "escalation_rate", _share(row.get("escalated") for row in rows))
    else:
        # Retries are only counted for Haiku packet workers, so the old regime's count would be an artifact.
        _put(out, "retries_per_accepted", None, "retries are counted for Haiku packet workers only")
        _put(out, "escalation_rate", None, "n/a: nothing was dispatched to Haiku")
    _put(out, "packet_cycle_p50_hours", cycles["p50"])
    return out


def mixed_summary(records):
    """The campaigns whose regime is mixed, by name: they count toward packets, not cost."""
    names = sorted(name for name, r in records.items() if r.get("regime") == "mixed")
    return {"campaigns": len(names), "names": names}
