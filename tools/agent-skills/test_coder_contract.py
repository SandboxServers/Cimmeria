"""The Haiku coder contract (C4) is in the packet-coder and orchestrator agent files."""
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
AGENTS = ROOT / ".claude" / "agents"


def read_agent(name):
    """Return the text of one agent file."""
    return (AGENTS / f"{name}.md").read_text(encoding="utf-8")


def line_starting(text, prefix):
    """Return the first line of the text that starts with the prefix."""
    for line in text.splitlines():
        if line.startswith(prefix):
            return line
    return ""


class CoderContractTest(unittest.TestCase):
    """A packet-coder writes code and commits; the orchestrator routes its one line."""

    def test_packet_coder_names_the_contract(self):
        text = read_agent("packet-coder")
        for phrase in (
            "Context cap reached",
            "reason=context",
            "base=<sha>",
            "Rejected <note-id>:",
            "Deviation: <what> - <why>",
            "not_run=",
            "--allow-empty",
            "(part N)",
            "<commit-sha|-> -",
        ):
            with self.subTest(phrase=phrase):
                self.assertIn(phrase, text)

    def test_packet_coder_has_no_worknote_or_report_rules(self):
        text = read_agent("packet-coder")
        for phrase in (
            "worknotes/",
            "worknote first",
            "<report-path>",
            "in your report",
            "write your report",
            "Report, compactly",
        ):
            with self.subTest(phrase=phrase):
                self.assertNotIn(phrase, text)

    def test_orchestrator_coder_loop_resumes_from_the_branch(self):
        line = line_starting(read_agent("orchestrator"), "- Coder loop")
        self.assertIn("base=<sha>", line)
        self.assertIn("no usable line", line)
        self.assertNotIn("worknote path", line)

    def test_orchestrator_brief_names_no_report_path(self):
        self.assertIn("names no report path", read_agent("orchestrator"))


if __name__ == "__main__":
    unittest.main()