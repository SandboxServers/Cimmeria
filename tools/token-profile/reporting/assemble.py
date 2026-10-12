"""Merges the cost, packets and ledger files for one campaign into its core report record (CM-08b)."""

import copy

from reporting.regime import campaign_regime, packet_regime

SECTIONS = ("first_pass", "retries", "escalation", "stop_context", "cycle_time")
STAMP_KEYS = ("generated", "window", "price_table", "profiler_schema", "unattributed_share")


def assemble_core(name, cost_doc, packets_doc, ledger_doc):
    """Return the core record of campaign `name`; inputs are copied, never mutated, and a missing packets record becomes gaps."""
    cost_campaign = cost_doc["campaigns"][name]
    packets_campaign = packets_doc["campaigns"].get(name)
    ledger_packets = {p["id"]: p for p in ledger_doc["campaigns"].get(name, {}).get("packets", [])}
    packets = packets_campaign.get("packets", []) if packets_campaign is not None else []

    out = {"campaign": name}
    if packets_campaign is None:
        out["regime"] = "old"
        out["regime_gap"] = "no packets record"
    else:
        out["regime"] = campaign_regime(packets)

    out["stamp"] = copy.deepcopy({key: cost_doc["stamp"][key] for key in STAMP_KEYS})
    out["stamp"]["unmapped_share"] = packets_doc["sessions"]["unmapped_share"]

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
