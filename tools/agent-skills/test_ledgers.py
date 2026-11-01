"""Campaign RESUME.md files fit the resume card (ai-harness-docs S6); ID prefixes stay in one campaign."""
import re
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
CARD_LINES = 40
DECISION_ROW = re.compile(r"^\|\s*\**(D-[A-Z]{1,4})\d+[a-z]?\b")
PACKET_ROW = re.compile(r"^\|\s*\**([A-Z]{2,4})-\d{2,3}[a-z]?\b")
# Tokens in a ledger's first cell that are not packet IDs (none in the 2026-10-31 scan).
NOT_PACKETS = frozenset()
# frozen 2026-10-31; never add a new campaign here
GRANDFATHERED = {
    "CM": frozenset({"campaign-metrics", "custom-debug-map"}),
    "CS": frozenset({"castle-continuation", "class-start-v6", "four-zone-planning", "four-zone-review"}),
    "D-AU": frozenset({"ability-mechanics", "launcher-adopt-ux"}),
    "D-CA": frozenset({"castle-continuation", "castle-rebuild"}),
    "D-H": frozenset({"harset-continuation", "harset-rebuild"}),
    "D-LC": frozenset({"lab-cli", "lane-classes"}),
    "D-LR": frozenset({"lab-record", "lab-uat-reliability"}),
    "D-LX": frozenset({"lab-cli-rust", "launcher-consolidation"}),
    "D-PP": frozenset({"launcher-linux-play-proton", "persistent-player-data"}),
    "D-UB": frozenset({"union-beta-e1-continuation", "union-zone-planning"}),
    "D-US": frozenset({"union-sgc-continuation", "union-zone-planning"}),
    "D-UD": frozenset({"union-sgc-continuation", "union-zone-planning"}),
    "DK": frozenset({"dakara-e1-rebuild", "union-zone-planning"}),
    "FX": frozenset({"lab-fixtures", "lab-record"}),
    "GD": frozenset({"lab-golden", "lab-record"}),
    "HA": frozenset({"four-zone-planning", "four-zone-review", "harset-continuation"}),
    "LC": frozenset({"lab-cli", "lane-classes"}),
    "LP": frozenset({"lab-parallel-clients", "launcher-cache-presync"}),
    "LR": frozenset({"lab-record", "lab-uat-reliability"}),
    "LW": frozenset({"lab-record", "lab-watch"}),
    "LX": frozenset({"lab-cli-rust", "launcher-consolidation"}),
    "PP": frozenset({"launcher-linux-play-proton", "persistent-player-data"}),
    "PU": frozenset({"four-zone-review", "harset-continuation"}),
    "SV": frozenset({"lab-record", "lab-spec-vocab"}),
    "UB": frozenset({"union-beta-e1-continuation", "union-zone-planning"}),
    "US": frozenset({"union-sgc-continuation", "union-zone-planning"}),
}


def ledgers():
    """Return every campaign RESUME.md under docs/analysis."""
    return sorted((ROOT / "docs" / "analysis").glob("*/RESUME.md"))


def prefix_owners():
    """Map each decision or packet ID prefix to the campaign folders whose top-level ledgers use it."""
    owners = {}
    for f in sorted((ROOT / "docs" / "analysis").glob("*/*.md")):
        text = f.read_text(encoding="utf-8", errors="replace")
        for line in text.splitlines():
            for pattern in (DECISION_ROW, PACKET_ROW):
                m = pattern.match(line)
                if m:
                    owners.setdefault(m.group(1), set()).add(f.parent.name)
    return owners


class LedgerTest(unittest.TestCase):
    """Every campaign ledger fits the card that resume_card.py injects."""

    def test_finds_enough_ledgers(self):
        self.assertGreaterEqual(len(ledgers()), 5)

    def test_every_resume_fits_the_card(self):
        for f in ledgers():
            lines = len(f.read_text(encoding="utf-8").splitlines())
            with self.subTest(file=str(f)):
                self.assertLessEqual(
                    lines,
                    CARD_LINES,
                    f"{f} has {lines} lines; resume_card.py injects only the first 40 "
                    "(campaign-packet skill § 5)",
                )

    def test_prefixes_belong_to_one_campaign(self):
        bad = []
        for prefix, folders in sorted(prefix_owners().items()):
            if prefix in NOT_PACKETS or len(folders) < 2:
                continue
            allowed = GRANDFATHERED.get(prefix)
            if allowed is not None and folders <= allowed:
                continue
            bad.append(f"{prefix} is used by {', '.join(sorted(folders))}")
        self.assertFalse(
            bad,
            "an ID prefix belongs to more than one campaign folder (campaign-packet skill § 1): "
            + "; ".join(bad),
        )


if __name__ == "__main__":
    unittest.main()
