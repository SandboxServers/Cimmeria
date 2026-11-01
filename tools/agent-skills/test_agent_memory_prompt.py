"""The memory rule line (C8) is in the four agents that get it, and the workflow doc holds its anchor."""
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
AGENTS = ROOT / ".claude" / "agents"
WORKFLOW = ROOT / "docs" / "agents" / "development-workflow.md"

AGENTS_WITH_RULE = (
    "documentation-writer",
    "testing-validation-engineer",
    "architect",
    "hands-on",
)

RULE_LINE = (
    "**Memory:** a memory is a lead, not evidence; verify it against the code before you rely on it. "
    "Save methods and traps over states; a state claim carries `INVALIDATED WHEN: <condition>` and "
    "`checked: <date>`. Keep `MEMORY.md` under 200 lines. Rules: docs/agents/development-workflow.md "
    "§ Project memory."
)


def read_text(path):
    """Return the text of a file, read as UTF-8."""
    return path.read_text(encoding="utf-8")


def project_memory_section(text):
    """Return the text of the Project memory section, up to the next level-2 heading."""
    section = text.split("## Project memory", 1)[1]
    return section.split("\n## ", 1)[0]


class AgentMemoryPromptTest(unittest.TestCase):
    """Each agent that gets the rule carries it verbatim, and its anchor resolves."""

    def test_agents_carry_the_rule_line(self):
        for name in AGENTS_WITH_RULE:
            with self.subTest(agent=name):
                text = read_text(AGENTS / f"{name}.md")
                self.assertIn(RULE_LINE, text.splitlines())

    def test_rule_anchor_resolves(self):
        text = read_text(WORKFLOW)
        self.assertIn("## Project memory", text)
        self.assertIn("INVALIDATED WHEN:", project_memory_section(text))

    def test_workflow_drops_the_stale_claim(self):
        self.assertNotIn("imports its index", read_text(WORKFLOW))


if __name__ == "__main__":
    unittest.main()
