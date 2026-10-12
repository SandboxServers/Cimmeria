"""Merges the cost, packets and ledger files for one campaign into its core report record (CM-08b)."""

import copy

from .regime import campaign_regime, packet_regime

SECTIONS = ("first_pass", "retries", "escalation", "stop_context", "cycle_time")
STAMP_KEYS = ("generated", "window", "price_table", "profiler_schema", "unattributed_share")


def assemble_core(name, cost_doc, packets_doc, ledger_doc):
    """Return the core record of campaign `name`; inputs are copied, never mutated, and a missing packets record becomes gaps."""
    cost_campaign = cost_doc["campaigns"][name]
    packets_campaign = packets_doc.get("campaigns", {}).get(name)
    ledger_packets = {p["id"]: p for p in ledger_doc.get("campaigns", {}).get(name, {}).get("packets", [])}
    packets = packets_campaign.get("packets", []) if packets_campaign is not None else []

    out = {"campaign": name}
    if packets_campaign is None:
        out["regime"] = "old"
        out["regime_gap"] = "no packets record"
    else:
        out["regime"] = campaign_regime(packets)

    stamp = cost_doc.get("stamp") or {}
    out["stamp"] = copy.deepcopy({key: stamp.get(key) for key in STAMP_KEYS})
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
