"""Layer 1: raw token categories, reported independently and never priced."""

from ..stats import distribution
from .grouping import DIMENSIONS, TOKEN_COLUMNS, accumulate

NOTE = ("Raw token counts. thinking_tokens is part of output_tokens and is never added on top. "
        "Context is input plus cache read plus both cache writes.")


def build(db, sc):
    fields = ("requests",) + TOKEN_COLUMNS + ("context_tokens", "web_search", "web_fetch")
    select = ", ".join(f"SUM({c}) AS {c}" for c in fields[1:])
    totals = dict(db.execute(f"SELECT COUNT(*) AS requests, {select} FROM wreq").fetchone())
    totals = {k: v or 0 for k, v in totals.items()}

    by = {}
    for name, column in DIMENSIONS:
        rows = db.execute(f"SELECT {column} AS k, COUNT(*) AS requests, {select} FROM wreq"
                          f" GROUP BY {column} ORDER BY SUM(context_tokens) DESC").fetchall()
        groups = accumulate(rows, lambda r: sc.label(r["k"], name), fields)
        by[name] = [{"key": k, **v} for k, v in groups.items()]

    per_request = {}
    for column in TOKEN_COLUMNS + ("context_tokens",):
        per_request[column] = distribution([r[0] for r in db.execute(f"SELECT {column} FROM wreq")])

    return {"layer": "raw tokens", "note": NOTE, "totals": totals, "by": by, "per_request": per_request}
