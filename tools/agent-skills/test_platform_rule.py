"""Agents with a shell pick it from the Platform line (ai-harness-docs D-AH53, C17)."""
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
AGENTS = ROOT / ".claude" / "agents"
SHELL_AGENTS = ("packet-coder", "packet-reviewer", "architect", "ops")
BLOCKED = "platform <p> is not supported (Windows, Linux and WSL only)"


def split_agent(name):
    """Return the frontmatter and the body of one agent file."""
    text = (AGENTS / f"{name}.md").read_text(encoding="utf-8")
    _, front, body = text.split("---\n", 2)
    return front, body


def tools_of(front):
    """Return the tool names on the frontmatter tools line."""
    for line in front.splitlines():
        if line.startswith("tools:"):
            return [t.strip() for t in line[len("tools:"):].split(",")]
    return []


class PlatformRuleTest(unittest.TestCase):
    """The agents that run a shell grant both shells and name the Platform line."""

    def test_shell_agents_grant_both_shells(self):
        for name in SHELL_AGENTS:
            front, body = split_agent(name)
            with self.subTest(agent=name):
                tools = tools_of(front)
                self.assertIn("PowerShell", tools)
                self.assertIn("Bash", tools)
                for phrase in ("Platform:", "win32", "linux", BLOCKED):
                    self.assertIn(phrase, body)

    def test_packet_coder_names_both_lanes_and_the_host_rule(self):
        _, body = split_agent("packet-coder")
        for phrase in (
            "tools\\build-lane\\lane.ps1",
            "bash tools/build-lane/lane.sh",
            "Host:",
            "does not match platform",
        ):
            with self.subTest(phrase=phrase):
                self.assertIn(phrase, body)

    def test_domain_agents_name_both_lanes(self):
        for name in ("game-archaeology-specialist", "social-systems-engineer"):
            _, body = split_agent(name)
            with self.subTest(agent=name):
                self.assertIn("tools/build-lane/lane.ps1", body)
                self.assertIn("tools/build-lane/lane.sh", body)


if __name__ == "__main__":
    unittest.main()
