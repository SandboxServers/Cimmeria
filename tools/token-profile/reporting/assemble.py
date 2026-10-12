"""Merges the input files into one report record per campaign: the core merge (CM-08b), then GitHub and overstep (CM-08c)."""

import copy

from .regime import campaign_regime, packet_regime

SECTIONS = ("first_pass", "retries", "escalation", "stop_context", "cycle_time")
STAMP_KEYS = ("generated", "window", "price_table", "profiler_schema", "unattributed_share")


def scalar_share(stamp):
    """Return a copy of `stamp` with the unattributed share as a number (CM-10 real shape).

    cost.json carries a dict of shares by family, request and dollars: the dollar share becomes `unattributed_share`,
    the request share `unattributed_request_share`, and `by_family` is dropped. A number or a missing value passes through.
    """
    if stamp is None:
        return None
    out = copy.deepcopy(stamp)
    share = out.get("unattributed_share")
    if isinstance(share, dict):
        out["unattributed_share"] = share.get("by_usd")
        out["unattributed_request_share"] = share.get("by_request")
    return out


def assemble_core(name, cost_doc, packets_doc, ledger_doc):
    """Return the core record of campaign `name`; inputs are copied, never mutated, and a missing packets record becomes gaps."""
    cost_campaign = cost_doc["campaigns"][name]
    packets_campaign = packets_doc.get("campaigns", {}).get(name)
    ledger_packets = {p["id"]: p for p in ledger_doc.get("campaigns", {}).get(name, {}).get("packets", [])}
    packets = packets_campaign.get("packets", []) if packets_campaign is not None else []

    out = {"campaign": name}
    if packets_campaign is None:
        # A cost record may already carry its regime (a mixed campaign created by the own-session cost).
        out["regime"] = cost_campaign.get("regime", "old")
        if "regime" not in cost_campaign:
            out["regime_gap"] = "no packets record"
    else:
        out["regime"] = campaign_regime(packets)

    stamp = cost_doc.get("stamp") or {}
    out["stamp"] = scalar_share({key: stamp.get(key) for key in STAMP_KEYS})
    out["stamp"]["unmapped_share"] = (packets_doc.get("sessions") or {}).get("unmapped_share")

    for key in ("cost", "prs", "accepted"):
        out[key] = copy.deepcopy(cost_campaign[key])

    for section in SECTIONS:
        if packets_campaign is not None and section in packets_campaign:
            out[section] = copy.deepcopy(packets_campaign[section])
        else:
            out[section] = {}
            out[f"{section}_gap"] = "no packets record for this campaign"

    out["packet_rows"] = [
        {
            "id": p["id"],
            "regime": packet_regime(p),
            "accepted": p.get("accepted"),
            "first_pass": p.get("first_pass"),
            "retries": copy.deepcopy(p.get("retries")),
            "escalated": p.get("escalated"),
            "cycle_hours": p.get("cycle_hours"),
            "implementer": ledger_packets.get(p["id"], {}).get("implementer"),
        }
        for p in packets
    ]
    return out


def _share(flags):
    """Share of True among the flags that are not None, rounded to 2 places, or None when there are none."""
    flags = [f for f in flags if f is not None]
    return round(sum(1 for f in flags if f) / len(flags), 2) if flags else None


def _put(out, key, value, gap):
    """Store a rate; a rate with no rows is None with a gap note, never 0."""
    out[key] = value
    if value is None:
        out[f"{key}_gap"] = gap


def attach_github(record, github_doc):
    """Add the CI, queue and escaped-defect figures of campaign `record` from the GitHub collector output (CM-08c)."""
    name = record["campaign"]
    ci_rows = [
        row
        for row in github_doc.get("prs") or []
        if row.get("campaign") == name and (row.get("ci") or {}).get("first_attempt") is not None
    ]

    ci = {}
    _put(ci, "first_attempt_rate", _share(r["ci"]["first_attempt"] for r in ci_rows), "no CI data")
    _put(ci, "overall_rate", _share(r["ci"]["overall"] for r in ci_rows), "no CI data")
    _put(
        ci,
        "code_prs_first_attempt_rate",
        _share(r["ci"]["first_attempt"] for r in ci_rows if r.get("code_pr")),
        "no CI data",
    )
    _put(ci, "queue_first_attempt_rate", _share((r.get("queue") or {}).get("first_attempt") for r in ci_rows), "no CI data")
    ci["pr_runs_total"] = sum(r["ci"].get("runs") or 0 for r in ci_rows)
    record["ci"] = ci

    ci_by_pr = {row["number"]: row.get("ci") or {} for row in github_doc.get("prs") or []}
    for entry in record["prs"]["per_pr"]:
        pr_ci = ci_by_pr.get(entry["pr"], {})
        entry["ci_first_pass"] = pr_ci.get("first_attempt")
        entry["ci_runs"] = pr_ci.get("runs")

    merged = record["prs"].get("merged")
    defects = (github_doc.get("escaped_defects") or {}).get(name, [])
    escaped = {"count": len(defects)}
    _put(
        escaped,
        "rate_per_merged_pr",
        len(defects) / merged if merged else None,
        "no merged PRs",
    )
    escaped["items"] = [
        {key: item.get(key) for key in ("kind", "number", "cites", "days_after_merge")} for item in defects
    ]
    record["escaped_defects"] = escaped


def _usd(rules, rule):
    """The dollars of overstep rule `rule`; a rule that did not fire counts as 0.0."""
    return (rules.get(rule) or {}).get("usd", 0.0)


def attach_overstep(record, overstep_doc):
    """Add the overstep dollars and recoverable range of campaign `record` from the overstep output (CM-08c)."""
    entry = (overstep_doc.get("by_campaign") or {}).get(record["campaign"])
    if entry is None:
        # The rules apply to the Haiku regime only: an old campaign has no figures, any other has zeros.
        if record["regime"] == "old":
            record["overstep"] = {
                "opus_coding_usd": None,
                "opus_research_usd": None,
                "sonnet_implementing_usd": None,
                "wasted_usd": None,
                "recoverable_usd": {"low": None, "high": None},
                "rules": {},
                "items": [],
                "overstep_gap": "predates the regime switch; the rules apply to the Haiku regime",
            }
        else:
            record["overstep"] = {
                "opus_coding_usd": 0.0,
                "opus_research_usd": 0.0,
                "sonnet_implementing_usd": 0.0,
                "wasted_usd": 0.0,
                "recoverable_usd": {"low": 0.0, "high": 0.0},
                "rules": {},
                "items": [],
            }
        return

    rules = entry.get("rules") or {}
    record["overstep"] = {
        "opus_coding_usd": _usd(rules, "O1"),
        "opus_research_usd": _usd(rules, "O2"),
        "sonnet_implementing_usd": _usd(rules, "O3"),
        # Wasted cost is the headline figure: the campaign total is the rounded sum over its rules, None without rules.
        "wasted_usd": round(sum(body.get("wasted_usd") or 0.0 for body in rules.values()), 2) if rules else None,
        "recoverable_usd": {"low": entry.get("recoverable_low"), "high": entry.get("recoverable_high")},
        "rules": {
            rule: {"count": body.get("count"), "usd": body.get("usd"), "wasted_usd": body.get("wasted_usd")}
            for rule, body in rules.items()
        },
        "items": copy.deepcopy(entry.get("items") or []),
    }


def assemble(inputs):
    """Return {"campaigns": {name: record}, "meta": {...}} for every campaign with spend; an empty input gives no campaigns."""
    cost_doc = inputs.get("cost") or {}
    ledger_doc = inputs.get("ledger") or {}
    github_doc = inputs.get("github") or {}
    packets_doc = inputs.get("packets") or {}
    overstep_doc = inputs.get("overstep") or {}
    cost_campaigns = cost_doc.get("campaigns") or {}

    campaigns = {}
    for name in cost_campaigns:
        record = assemble_core(name, cost_doc, packets_doc, ledger_doc)
        attach_github(record, github_doc)
        attach_overstep(record, overstep_doc)
        campaigns[name] = record

    fixer_requests = overstep_doc.get("fixer_requests")
    ledger_campaigns = ledger_doc.get("campaigns") or {}
    meta = {
        "stamp": scalar_share(cost_doc.get("stamp")),
        "unmapped_share": (packets_doc.get("sessions") or {}).get("unmapped_share"),
        "untagged_prs": github_doc.get("untagged_prs"),
        "opus_subagents": overstep_doc.get("opus_subagents"),
        "fixer_requests_total": sum(fixer_requests.values()) if fixer_requests is not None else None,
        "missing_inputs": inputs.get("missing"),
        "skipped_ledger_campaigns": len(set(ledger_campaigns) - set(cost_campaigns)),
    }
    return {"campaigns": campaigns, "meta": meta}
