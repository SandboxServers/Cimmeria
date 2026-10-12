"""Adds the own-session cost (CM-08i) to the campaign's record in cost.json, so the campaign's totals and splits
include the coordinator's own sessions. The inputs are never mutated."""

import copy

TIERS = ("opus", "sonnet", "haiku", "fable", "other")
EMPTY_PRS = {"count": 0, "merged": 0, "cost_per_pr": {"mean": None, "p50": None, "p90": None}, "per_pr": []}
EMPTY_ACCEPTED = {"packets": 0, "cost_per_accepted_usd": None, "accepted_gap": "no packet table"}


def _empty_campaign(campaign):
    """A campaign record with zero cost, for a campaign that cost.json does not list."""
    cost = {"total_usd": 0.0, "by_model_tier": {tier: 0.0 for tier in TIERS}, "by_role": {},
            "attributed_usd": 0.0, "unattributed_in_campaign_usd": 0.0}
    # Mixed, not old: the coordinator's own spend belongs to neither regime, so it stays out of the old-model column.
    return {"campaign": campaign, "regime": "mixed", "cost": cost, "prs": copy.deepcopy(EMPTY_PRS),
            "accepted": copy.deepcopy(EMPTY_ACCEPTED)}


def apply_self_cost(inputs, self_cost, campaign="campaign-metrics"):
    """Return a copy of `inputs` with `self_cost` (from `reporting.self_cost.load_self_cost`) added to `campaign`:
    to its total and to its unattributed cost (own sessions carry no PR), to its model tiers and its coordinator role,
    and its session count. A campaign that cost.json lacks is created with zero cost first."""
    out = copy.deepcopy(inputs)
    campaigns = out["cost"].setdefault("campaigns", {})
    if campaign not in campaigns:
        campaigns[campaign] = _empty_campaign(campaign)
    cost = campaigns[campaign]["cost"]
    cost["total_usd"] += self_cost["total_usd"]
    cost["unattributed_in_campaign_usd"] += self_cost["total_usd"]
    for tier, usd in self_cost["by_tier"].items():
        cost["by_model_tier"][tier] = cost["by_model_tier"].get(tier, 0.0) + usd
    cost["by_role"]["coordinator"] = cost["by_role"].get("coordinator", 0.0) + self_cost["by_role"]["coordinator"]
    cost["self_sessions"] = self_cost["sessions"]
    return out
