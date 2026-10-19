"""Agent names in the harness docs match the agent files (ai-harness-docs D-AH42, D-AH47)."""
import re
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
AGENTS = ROOT / ".claude" / "agents"
NAME = re.compile(r"`(packet-[a-z-]+)`")
LINK = re.compile(r"\]\(([^)\s#]+\.md)(?:#[^)]*)?\)")
GEN = re.compile(r"(?s)<!-- gen:[a-z-]+ -->.*?<!-- /gen:[a-z-]+ -->")


def scanned():
    """Return the agent, skill and harness doc files that may name an agent."""
    files = sorted(AGENTS.glob("*.md")) + sorted((ROOT / ".claude" / "skills").glob("*/SKILL.md"))
    files += [ROOT / n for n in ("AGENTS.md", "README.md", "CLAUDE.md")]
    files += sorted((ROOT / "docs" / "agents").glob("*.md"))
    return [f for f in files if f.is_file()]


class AgentRefsTest(unittest.TestCase):
    """Every backticked or linked agent in these files has an agent file."""

    def test_scans_enough_files(self):
        self.assertGreaterEqual(len(scanned()), 20)

    def test_backticked_agent_names_exist(self):
        for f in scanned():
            text = f.read_text(encoding="utf-8")
            for name in NAME.findall(text):
                with self.subTest(file=str(f), name=name):
                    self.assertTrue((AGENTS / f"{name}.md").is_file())

    def test_agent_links_exist(self):
        for f in scanned():
            text = f.read_text(encoding="utf-8")
            for link in LINK.findall(text):
                if "://" in link:
                    continue
                target = (f.parent / link).resolve()
                # Only packet-* agent links: other links into the folder (TESTING.md, file.md) are not agents.
                if target.parent == AGENTS.resolve() and target.stem.startswith("packet-"):
                    with self.subTest(file=str(f), link=link):
                        self.assertTrue(target.is_file())

    def test_retired_review_fixer_is_gone(self):
        for f in scanned():
            text = GEN.sub("", f.read_text(encoding="utf-8")).lower()
            with self.subTest(file=str(f)):
                self.assertNotIn(
                    "review-fixer",
                    text,
                    f"{f} names the retired packet-review-fixer (ai-harness-docs D-AH42); "
                    "describe the packet-reviewer loop instead",
                )


if __name__ == "__main__":
    unittest.main()
