"""Regime labels for packets and campaigns: a packet is "haiku" when a Haiku implementer ran, else "old"."""

HAIKU_SHARE = 0.7


def packet_regime(packet_record):
    """"haiku" when the packet had at least one implementer session, else "old" (D-CM1: the implementer that ran)."""
    return "haiku" if packet_record.get("implementer_sessions", 0) > 0 else "old"


def campaign_regime(packet_records):
    """"haiku" when at least 70% of the packets are haiku, "old" when none are, "mixed" otherwise; no packets is "old"."""
    if not packet_records:
        return "old"
    haiku = sum(1 for p in packet_records if packet_regime(p) == "haiku")
    if haiku == 0:
        return "old"
    if haiku >= HAIKU_SHARE * len(packet_records):
        return "haiku"
    return "mixed"
