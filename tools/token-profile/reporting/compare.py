"""Old-versus-Haiku comparison: the delta between the two regimes and the low-n flag that goes with it."""

LOW_N_CAMPAIGNS = 5
LOW_N_PACKETS = 10
DELTA_METRICS = (
    "cost_per_pr_p50",
    "cost_per_pr_mean",
    "cost_per_accepted_usd",
    "first_pass_rate",
    "retries_per_accepted",
    "ci_first_attempt_rate",
    "escaped_defects_per_merged_pr",
    "packet_cycle_p50_hours",
)
_COST_PER_PR_KEYS = {"cost_per_pr_p50": "p50", "cost_per_pr_mean": "mean"}


def _value(metrics, name):
    """One delta metric from a roll-up; the two cost-per-PR figures live inside the cost_per_pr dict."""
    if name in _COST_PER_PR_KEYS:
        return (metrics.get("cost_per_pr") or {}).get(_COST_PER_PR_KEYS[name])
    return metrics.get(name)


def delta(old, haiku):
    """Haiku minus old for each delta metric, rounded to 4 places; None when either side is missing."""
    out = {}
    for name in DELTA_METRICS:
        old_value, haiku_value = _value(old, name), _value(haiku, name)
        out[name] = None if old_value is None or haiku_value is None else round(float(haiku_value) - float(old_value), 4)
    return out


def low_n(old, haiku):
    """True when either regime has too few campaigns or too few packets for its delta to be read."""
    return any(m["campaigns"] < LOW_N_CAMPAIGNS or m["packets"] < LOW_N_PACKETS for m in (old, haiku))
