"""Grouping shared by the sections."""

TOKEN_COLUMNS = ("input_tokens", "output_tokens", "thinking_tokens", "cache_read", "cache_write_5m", "cache_write_1h")

# The report's breakdowns, as (name, wreq column).
DIMENSIONS = (("model", "model"), ("scope", "scope"), ("agent_type", "agent_type"), ("trigger", "trigger_kind"))


def accumulate(rows, key_fn, fields):
    """Sum `fields` over rows grouped by key_fn(row), keeping first-seen order.

    Used after field validation, so two values that both became `<invalid>`
    land in one group instead of two rows with the same label.
    """
    out = {}
    for row in rows:
        key = key_fn(row)
        acc = out.setdefault(key, {f: 0 for f in fields})
        for f in fields:
            acc[f] += row[f] or 0
    return out


def gap_bucket(gap_s):
    if gap_s is None:
        return "first"
    if gap_s <= 300:
        return "<=5m"
    if gap_s <= 3600:
        return "5m-1h"
    return ">1h"


GAP_BUCKETS = ("first", "<=5m", "5m-1h", ">1h")
