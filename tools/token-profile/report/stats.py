"""Distributions for the reports: p50/p75/p90/p95/p99/max, never a mean alone."""

PERCENTILES = (50, 75, 90, 95, 99)


def percentile(sorted_values, p):
    """Linear interpolation between closest ranks (numpy's default, type 7)."""
    if not sorted_values:
        return None
    if len(sorted_values) == 1:
        return sorted_values[0]
    pos = (len(sorted_values) - 1) * p / 100.0
    lo = int(pos)
    hi = min(lo + 1, len(sorted_values) - 1)
    return sorted_values[lo] + (sorted_values[hi] - sorted_values[lo]) * (pos - lo)


def distribution(values):
    """n, sum, mean and the standard percentiles of a list of numbers.

    Every report distribution goes through here, so none of them can show a
    mean without the tail next to it.
    """
    vals = sorted(v for v in values if v is not None)
    out = {"n": len(vals), "sum": sum(vals), "mean": (sum(vals) / len(vals)) if vals else None}
    for p in PERCENTILES:
        out[f"p{p}"] = percentile(vals, p)
    out["max"] = vals[-1] if vals else None
    return out


def share(part, whole):
    return (part / whole) if whole else 0.0


def top_share(values, fraction):
    """Share of the total held by the largest `fraction` of the values (at least one)."""
    vals = sorted((v for v in values if v), reverse=True)
    if not vals:
        return 0.0
    k = max(1, int(round(len(vals) * fraction)))
    return share(sum(vals[:k]), sum(vals))
