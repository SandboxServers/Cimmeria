"""The memory rule line (C8) is in the 18 agents that get it, the copied memory prompt is gone, and the workflow doc holds its anchor."""
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
    "aoi-witness-broadcast",
    "bigworld-engine-advisor",
    "combat-systems-advisor",
    "database-persistence",
    "minigame-systems-advisor",
    "items-systems-advisor",
    "game-archaeology-specialist",
    "network-security-auth",
    "movement-teleport-advisor",
    "npc-ai-spawn-advisor",
    "rust-gameserver-dev",
    "social-systems-engineer",
    "server-authority-enforcer",
    "mission-systems-advisor",
)

# The 14 agents whose copied generic memory prompt became the C8 stub.
AGENTS_WITH_STUB = AGENTS_WITH_RULE[4:]

COPIED_HEADINGS = (
    "# Persistent Agent Memory",
    "## Types of memory",
    "## What NOT to save in memory",
    "## Before recommending from memory",
    "## Searching past context",
)

USER_PATHS = ("C:\\Users\\", "/home/")

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


def agent_files():
    """Return the paths of every agent file under .claude/agents."""
    return sorted(AGENTS.glob("*.md"))


class AgentMemoryPromptTest(unittest.TestCase):
    """Each agent that gets the rule carries it verbatim, and its anchor resolves."""

    def test_agents_carry_the_rule_line(self):
        for name in AGENTS_WITH_RULE:
            with self.subTest(agent=name):
                text = read_text(AGENTS / f"{name}.md")
                self.assertIn(RULE_LINE, text.splitlines())

    def test_stub_heading_names_its_own_folder(self):
        for name in AGENTS_WITH_STUB:
            with self.subTest(agent=name):
                lines = read_text(AGENTS / f"{name}.md").splitlines()
                self.assertIn("# Agent memory", lines)
                after = lines[lines.index("# Agent memory") + 1 :]
                following = next(line for line in after if line.strip())
                self.assertIn(f".claude/agent-memory/{name}/", following)

    def test_no_copied_memory_prompt(self):
        for path in agent_files():
            text = read_text(path)
            for heading in COPIED_HEADINGS:
                with self.subTest(agent=path.name, heading=heading):
                    self.assertNotIn(heading, text)

    def test_no_user_paths_in_agent_files(self):
        for path in agent_files():
            text = read_text(path)
            for marker in USER_PATHS:
                with self.subTest(agent=path.name, marker=marker):
                    self.assertNotIn(marker, text)

    def test_rule_anchor_resolves(self):
        text = read_text(WORKFLOW)
        self.assertIn("## Project memory", text)
        self.assertIn("INVALIDATED WHEN:", project_memory_section(text))

    def test_workflow_drops_the_stale_claim(self):
        self.assertNotIn("imports its index", read_text(WORKFLOW))


if __name__ == "__main__":
    unittest.main()
