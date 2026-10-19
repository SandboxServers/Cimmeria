"""Campaign RESUME.md files fit the resume card (ai-harness-docs S6)."""
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
CARD_LINES = 40


def ledgers():
    """Return every campaign RESUME.md under docs/analysis."""
    return sorted((ROOT / "docs" / "analysis").glob("*/RESUME.md"))


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


if __name__ == "__main__":
    unittest.main()
