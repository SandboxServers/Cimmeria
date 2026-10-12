"""Campaign cost rollup (CM-03): list-price USD per campaign, role and PR, and the unattributed share.

USD is a list-price estimate (ingest/prices.py), not a bill: a subscription pays less, and extra turns a
cheaper model would need are not in it. Only aggregates leave this module; no id, branch, path or prompt.
"""

import re
from datetime import datetime, timezone

from ingest.prices import CURRENT, SOURCE, estimate_usd, normalize_model
from report.stats import distribution, share

TOKEN_KEYS = ("input", "output", "cache_read", "cache_write_5m", "cache_write_1h")
# Every campaign reports these roles, at 0.0 when it had no spend in them.
BASE_ROLES = ("coordinator", "implementer-haiku", "reviewer", "fixer", "other")

# requests -> pr_attribution rows: one row per (request, PR or campaign or unattributed), weights sum to 1.
# The agent's role key is custom_agent_type, else agent_type; a teammate's agent_type is its name, which is
# used only as a lookup key here and never shown.
ROWS_SQL = """
SELECT r.request_id, r.ts, r.model, r.agent_id, r.input_tokens, r.output_tokens, r.cache_read,
       r.cache_write_5m, r.cache_write_1h, a.custom_agent_type, a.agent_type, s.worktree,
       pa.pr_number, pa.method, pa.weight, pa.campaign AS row_campaign,
       p.campaign AS pr_campaign, p.state AS pr_state
FROM pr_attribution pa
JOIN requests r ON r.request_id = pa.request_id
JOIN sessions s ON s.session_id = r.session_id
LEFT JOIN agents a ON a.agent_id = r.agent_id
LEFT JOIN prs p ON p.pr_number = pa.pr_number
WHERE r.ts >= ? AND r.ts < ?
ORDER BY r.request_id, pa.pr_number
"""


def tier_of(model):
    """opus, sonnet, haiku, fable, or other, from the model's table key."""
    key = normalize_model(model)
    for tier in ("opus", "sonnet", "haiku", "fable"):
        if tier in key:
            return tier
    return "other"


def family_of(worktree):
    """The worktree family: the leading letters of its name ('lr-08a-fix2' is 'lr'); 'main' with no worktree."""
    if not worktree:
        return "main"
    name = re.split(r"[/\\]", worktree.rstrip("/\\"))[-1]
    found = re.match(r"[A-Za-z]+", name)
    return found.group(0).lower() if found else "other"


def campaign_of(row, ledger_pr_map):
    """The row's campaign: its own, then its PR's, then the ledger's map for a PR with no campaign of its own."""
    if row["row_campaign"]:
        return row["row_campaign"]
    if row["pr_campaign"]:
        return row["pr_campaign"]
    return (ledger_pr_map or {}).get(row["pr_number"])


def load_requests(conn, since, until, ledger_pr_map=None):
    """One dict per pr_attribution row in [since, until): the request's tokens and agent, and the row's PR,
    weight and campaign (None when untagged). `ledger_pr_map` ({pr: campaign}) tags a PR with no campaign."""
    # Timestamps are stored as 2026-09-21T00:00:00.000Z; the bounds come as ...00:00:00Z, so the Z is dropped
    # from both sides and the string comparison keeps the boundary instant inside the window.
    cur = conn.execute(ROWS_SQL, (since.rstrip("Z"), until.rstrip("Z")))
    names = [d[0] for d in cur.description]
    for values in cur:
        row = dict(zip(names, values))
        yield {"request_id": row["request_id"], "ts": row["ts"], "model": row["model"],
               "tokens": {"input": row["input_tokens"], "output": row["output_tokens"],
                          "cache_read": row["cache_read"], "cache_write_5m": row["cache_write_5m"],
                          "cache_write_1h": row["cache_write_1h"]},
               "is_coordinator": row["agent_id"] is None,
               "agent_key": row["custom_agent_type"] or row["agent_type"],
               "family": family_of(row["worktree"]),
               "pr": row["pr_number"], "pr_merged": row["pr_state"] == "MERGED",
               "method": row["method"], "weight": row["weight"],
               "campaign": campaign_of(row, ledger_pr_map)}


def request_usd(row, price_table=CURRENT):
    """The row's USD: its request's list-price estimate times the row's weight; None when the model is unpriced."""
    usd = estimate_usd(row["model"], row["tokens"], price_table)
    return None if usd is None else usd * row["weight"]


def _role(row, roles):
    if row["is_coordinator"]:
        return "coordinator"
    return roles.get(row["agent_key"], "other") if row["agent_key"] else "other"


def rollup(requests, price_table=CURRENT, roles=None, accepted=None):
    """Per campaign: the `cost` and `prs` parts of the contract, and `accepted` when the ledger gave a count.

    `roles` maps an agent key to a role (config.json's map); `accepted` is {campaign: packets}. Rows with no
    campaign are left out: they are reported by unattributed_share and the stamp, not in any campaign."""
    roles = roles or {}
    camps = {}
    unpriced = {}
    for row in requests:
        if not row["campaign"]:
            continue
        camp = camps.setdefault(row["campaign"], {"usd": 0.0, "tiers": {}, "roles": dict.fromkeys(BASE_ROLES, 0.0),
                                                  "attributed": 0.0, "no_pr": 0.0, "per_pr": {}, "prs": set(),
                                                  "merged": set()})
        usd = request_usd(row, price_table)
        if usd is None:
            unpriced[row["campaign"]] = unpriced.get(row["campaign"], 0) + 1
            continue
        tier, role = tier_of(row["model"]), _role(row, roles)
        camp["usd"] += usd
        camp["tiers"][tier] = camp["tiers"].get(tier, 0.0) + usd
        camp["roles"][role] = camp["roles"].get(role, 0.0) + usd
        if row["pr"] is None:
            camp["no_pr"] += usd
            continue
        camp["attributed"] += usd
        camp["prs"].add(row["pr"])
        camp["per_pr"][row["pr"]] = camp["per_pr"].get(row["pr"], 0.0) + usd
        if row["pr_merged"]:
            camp["merged"].add(row["pr"])
    return {name: _campaign_record(name, camp, unpriced.get(name, 0), (accepted or {}).get(name))
            for name, camp in sorted(camps.items())}


def _campaign_record(name, camp, unpriced, accepted):
    merged = camp["merged"]
    per_pr = [{"pr": pr, "usd": round(usd, 6), "merged": pr in merged} for pr, usd in sorted(camp["per_pr"].items())]
    merged_usd = sorted(camp["per_pr"][pr] for pr in merged)
    dist = distribution(merged_usd)
    cost = {"total_usd": round(camp["usd"], 6),
            "by_model_tier": {t: round(camp["tiers"].get(t, 0.0), 6) for t in ("opus", "sonnet", "haiku", "fable", "other")},
            "by_role": {r: round(v, 6) for r, v in sorted(camp["roles"].items())},
            "attributed_usd": round(camp["attributed"], 6), "unattributed_in_campaign_usd": round(camp["no_pr"], 6)}
    prs = {"count": len(camp["prs"]), "merged": len(merged),
           "cost_per_pr": {"mean": _r(dist["mean"]), "p50": _r(dist["p50"]), "p90": _r(dist["p90"])},
           "per_pr": per_pr}
    if not merged:
        prs["cost_per_pr_gap"] = "no merged PRs"
    record = {"campaign": name, "cost": cost, "prs": prs}
    if unpriced:
        cost["unpriced_rows"] = unpriced
        cost["unpriced_gap"] = "model has no price in the table for some rows; they are left out"
    if accepted:
        record["accepted"] = {"packets": accepted, "cost_per_accepted_usd": _r(camp["usd"] / accepted)}
    elif accepted == 0:
        # The ledger counted no accepted packets: there is no denominator, so no cost per packet.
        record["accepted"] = {"packets": 0, "cost_per_accepted_usd": None,
                              "accepted_gap": "the ledger counts no accepted packets"}
    elif merged:
        record["accepted"] = {"packets": 0, "cost_per_accepted_usd": _r(camp["usd"] / len(merged)),
                              "accepted_gap": "no packet table, used merged PRs"}
    else:
        record["accepted"] = {"packets": 0, "cost_per_accepted_usd": None,
                              "accepted_gap": "no packet table and no merged PRs"}
    return record


def _r(value):
    return None if value is None else round(value, 6)


def unattributed_share(conn, since, until, price_table=CURRENT):
    """The share of requests (by weight) and of list-price USD that is method 'unattributed', overall and per
    worktree family. Campaign tags are not needed here: every row counts, tagged or not."""
    # Each family: [weight, usd] over every row, and over the unattributed rows only.
    total, unattr = {}, {}
    requests_seen = set()
    for row in load_requests(conn, since, until):
        requests_seen.add(row["request_id"])
        usd = request_usd(row, price_table) or 0.0
        for bucket, hit in ((total, True), (unattr, row["method"] == "unattributed")):
            if hit:
                acc = bucket.setdefault(row["family"], [0.0, 0.0])
                acc[0] += row["weight"]
                acc[1] += usd
    all_weight = sum(v[0] for v in total.values())
    all_usd = sum(v[1] for v in total.values())
    by_family = {fam: {"requests": round(share(unattr.get(fam, [0.0, 0.0])[0], total[fam][0]), 4),
                       "usd": round(share(unattr.get(fam, [0.0, 0.0])[1], total[fam][1]), 4)}
                 for fam in sorted(total)}
    return {"requests": len(requests_seen),
            "by_request": round(share(sum(v[0] for v in unattr.values()), all_weight), 4),
            "by_usd": round(share(sum(v[1] for v in unattr.values()), all_usd), 4), "by_family": by_family}


def stamp(conn, since, until, ledger_pr_map=None, price_table=CURRENT):
    """What the numbers rest on: the window, the price table and its source, whether a ledger map was used,
    how many PRs are still untagged, and the unattributed share."""
    untagged = {row["pr"] for row in load_requests(conn, since, until, ledger_pr_map)
                if row["pr"] is not None and not row["campaign"]}
    schema = conn.execute("SELECT value FROM meta WHERE key = 'schema_version'").fetchone()[0]
    return {"generated": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
            "window": {"since": since, "until": until}, "price_table": price_table, "price_source": SOURCE,
            "profiler_schema": schema,
            "ledger": "applied" if ledger_pr_map else "not passed; PRs without a campaign stay untagged",
            "untagged_pr_count": len(untagged),
            "unattributed_share": unattributed_share(conn, since, until, price_table)}
