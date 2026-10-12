"""List-price cost of this campaign's own sessions (CM-08i), kept beside the campaign rollup.

Every request of the given sessions counts as role `coordinator`. Only aggregates leave this module.
"""

from ingest.prices import CURRENT
from metrics import cost
from report.db import open_db

# The requests of the given sessions that the campaign's cost rollup does not already hold, so none is counted twice.
SELF_SQL = """
SELECT r.request_id, r.model, r.input_tokens, r.output_tokens, r.cache_read, r.cache_write_5m, r.cache_write_1h
FROM requests r
WHERE r.session_id IN ({marks})
  AND NOT EXISTS (SELECT 1 FROM pr_attribution pa LEFT JOIN prs p ON p.pr_number = pa.pr_number
                  WHERE pa.request_id = r.request_id AND (pa.campaign = ? OR p.campaign = ?))
"""


def _usage(row):
    """The row as `cost.request_usd` takes it: its model and token counts, at weight 1."""
    tokens = {"input": row["input_tokens"], "output": row["output_tokens"], "cache_read": row["cache_read"],
              "cache_write_5m": row["cache_write_5m"], "cache_write_1h": row["cache_write_1h"]}
    return {"model": row["model"], "tokens": tokens, "weight": 1.0}


def price_sessions(rows, price_table=CURRENT):
    """Price the requests in `rows` (dicts with the SELF_SQL columns). Unpriced rows are counted and left out
    of every sum."""
    out = {"requests": 0, "unpriced": 0, "total_usd": 0.0, "by_tier": {}, "by_role": {"coordinator": 0.0}}
    for row in rows:
        out["requests"] += 1
        usd = cost.request_usd(_usage(row), price_table)
        if usd is None:
            out["unpriced"] += 1
            continue
        tier = cost.tier_of(row["model"])
        out["by_tier"][tier] = out["by_tier"].get(tier, 0.0) + usd
        out["total_usd"] += usd
        out["by_role"]["coordinator"] += usd
    return out


def load_self_cost(db_path, session_ids, campaign="campaign-metrics"):
    """Price the requests of `session_ids` that the campaign's rollup does not hold. An empty id list returns
    zeros without opening the database."""
    session_ids = list(session_ids)
    if not session_ids:
        return {**price_sessions([]), "sessions": 0}
    sql = SELF_SQL.format(marks=",".join("?" for _ in session_ids))
    db = open_db(db_path)
    try:
        rows = [dict(row) for row in db.execute(sql, [*session_ids, campaign, campaign])]
    finally:
        db.close()
    return {**price_sessions(rows), "sessions": len(session_ids)}
