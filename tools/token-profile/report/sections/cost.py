"""Layer 2: estimated list-price USD. A plan-usage proxy, never a bill."""

from ..stats import distribution, share
from .grouping import DIMENSIONS, accumulate

LABEL = ("Estimated list-price USD, priced per model from the stamped price table. "
         "The project is on a Max subscription, so this is a plan-usage proxy, not a bill (D-TP1).")

USD_PARTS = ("usd_input", "usd_output", "usd_cache_read", "usd_cache_write_5m", "usd_cache_write_1h")


def build(db, sc):
    parts = ", ".join(f"COALESCE(SUM({p}), 0) AS {p}" for p in USD_PARTS)
    totals = dict(db.execute(f"SELECT COALESCE(SUM(usd), 0) AS usd, {parts},"
                             " SUM(priced) AS priced_requests, COUNT(*) - SUM(priced) AS unpriced_requests"
                             " FROM wcost").fetchone())
    totals = {k: v or 0 for k, v in totals.items()}
    unpriced_models = [sc.label(r[0], "model") for r in db.execute(
        "SELECT DISTINCT model FROM wcost WHERE NOT priced ORDER BY model")]
    web = db.execute("SELECT COALESCE(SUM(web_search), 0), COALESCE(SUM(web_fetch), 0) FROM wreq").fetchone()

    by = {}
    fields = ("requests", "usd") + USD_PARTS
    for name, column in DIMENSIONS:
        rows = db.execute(f"SELECT {column} AS k, COUNT(*) AS requests, COALESCE(SUM(usd), 0) AS usd, {parts}"
                          f" FROM wcost GROUP BY {column} ORDER BY SUM(usd) DESC").fetchall()
        groups = accumulate(rows, lambda r: sc.label(r["k"], name), fields)
        by[name] = [{"key": k, **v, "share": share(v["usd"], totals["usd"])} for k, v in groups.items()]

    per_transcript = [r[0] for r in db.execute(
        "SELECT SUM(usd) FROM wcost WHERE priced GROUP BY session_id, COALESCE(agent_id, '')")]
    return {
        "layer": "estimated USD",
        "label": LABEL,
        "totals": totals,
        "unpriced_models": unpriced_models,
        "not_priced": {"web_search_requests": web[0], "web_fetch_requests": web[1]},
        "by": by,
        "per_request": distribution([r[0] for r in db.execute("SELECT usd FROM wcost WHERE priced")]),
        "per_transcript": distribution(per_transcript),
    }
