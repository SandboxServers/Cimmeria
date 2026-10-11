"""Tests for tools/agent-skills/sync.py."""

import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import sync  # noqa: E402

SKILL = "---\nname: demo\ndescription: Demo skill.\n---\n\n# Demo\n"


class SyncTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.repo = Path(self.tmp.name)
        src = self.repo / ".claude/skills/demo"
        src.mkdir(parents=True)
        (src / "SKILL.md").write_text(SKILL, encoding="utf-8")
        (src / "extra.md").write_text("more\n", encoding="utf-8")

    def tearDown(self):
        self.tmp.cleanup()

    def run_sync(self, *args):
        return sync.main([*args, "--repo", str(self.repo)])

    def test_marker_follows_frontmatter(self):
        self.assertEqual(self.run_sync(), 0)
        text = (self.repo / ".agents/skills/demo/SKILL.md").read_text(encoding="utf-8")
        lines = text.split("\n")
        self.assertEqual(lines[0], "---")
        self.assertEqual(lines[3], "---")
        self.assertIn("by tools/agent-skills/sync.py", lines[4])
        self.assertEqual((self.repo / ".agents/skills/demo/extra.md").read_text(), "more\n")

    def test_check_detects_stale_then_passes(self):
        self.assertEqual(self.run_sync("--check"), 1)
        self.run_sync()
        self.assertEqual(self.run_sync("--check"), 0)
        (self.repo / ".claude/skills/demo/extra.md").write_text("changed\n", encoding="utf-8")
        self.assertEqual(self.run_sync("--check"), 1)

    def test_orphan_removed_but_hand_written_kept(self):
        self.run_sync()
        hand = self.repo / ".agents/skills/hand"
        hand.mkdir(parents=True)
        (hand / "SKILL.md").write_text(SKILL.replace("demo", "hand"), encoding="utf-8")
        for path in (self.repo / ".claude/skills/demo").iterdir():
            path.unlink()
        (self.repo / ".claude/skills/demo").rmdir()
        self.assertEqual(self.run_sync("--check"), 1)
        self.run_sync()
        self.assertFalse((self.repo / ".agents/skills/demo").exists())
        self.assertTrue((hand / "SKILL.md").is_file())
        self.assertEqual(self.run_sync("--check"), 0)

    def test_refuses_to_overwrite_hand_written_same_name(self):
        hand = self.repo / ".agents/skills/demo"
        hand.mkdir(parents=True)
        (hand / "SKILL.md").write_text(SKILL, encoding="utf-8")
        with self.assertRaises(SystemExit) as raised:
            self.run_sync()
        self.assertIn("hand-written", str(raised.exception))
        self.assertEqual((hand / "SKILL.md").read_text(encoding="utf-8"), SKILL)

    def test_links_leaving_the_skill_are_repointed(self):
        body = (
            "[agent](../../agents/lab-driver.md) [doc](../../../docs/x.md#part) "
            "[local](extra.md) [web](https://example.org/a) [here](#top)\n"
        )
        (self.repo / ".claude/skills/demo/SKILL.md").write_text(SKILL + body, encoding="utf-8")
        self.run_sync()
        text = (self.repo / ".agents/skills/demo/SKILL.md").read_text(encoding="utf-8")
        self.assertIn("[agent](../../../.claude/agents/lab-driver.md)", text)
        # Same depth on both sides, so repo-level links come out unchanged.
        self.assertIn("[doc](../../../docs/x.md#part)", text)
        self.assertIn("[local](extra.md)", text)
        self.assertIn("[web](https://example.org/a)", text)
        self.assertIn("[here](#top)", text)
        self.assertEqual(self.run_sync("--check"), 0)

    def test_codex_agent_from_claude_agent(self):
        agents = self.repo / ".claude/agents"
        agents.mkdir(parents=True)
        (agents / "lab-driver.md").write_bytes(
            (
                '---\nname: lab-driver\ndescription: "Drives the client; says \\"hi\\"."\n'
                "model: haiku\nmcpServers:\n  - cimmeria-lab\neffort: low\n---\n\n"
                "Rules with a back\\slash and \"quotes\".\nSecond line.\n"
            ).replace("\n", "\r\n").encode()
        )
        self.run_sync()
        out = self.repo / ".codex/agents/lab-driver.toml"
        data = out.read_bytes()
        self.assertNotIn(b"\n", data.replace(b"\r\n", b""), "CRLF source keeps CRLF")
        text = data.decode()
        self.assertIn('name = "lab-driver"', text)
        self.assertIn('description = "Drives the client; says \\"hi\\"."', text)
        self.assertIn('model_reasoning_effort = "low"', text)
        self.assertIn("developer_instructions = '''\r\nRules with a back\\slash", text)
        self.assertTrue(text.endswith("Second line.\r\n'''\r\n"))
        self.assertEqual(self.run_sync("--check"), 0)
        try:
            import tomllib
        except ImportError:  # Python < 3.11
            return
        parsed = tomllib.loads(text)
        self.assertEqual(parsed["description"], 'Drives the client; says "hi".')
        self.assertTrue(parsed["developer_instructions"].startswith("Rules with a back\\slash"))

    def write_agent(self, name, frontmatter):
        agents = self.repo / ".claude/agents"
        agents.mkdir(parents=True, exist_ok=True)
        (agents / f"{name}.md").write_text(
            f"---\nname: {name}\ndescription: The {name} agent.\n{frontmatter}---\n\nDo {name} things.\n",
            encoding="utf-8",
        )

    def test_every_agent_gets_a_codex_twin_with_mapped_effort(self):
        self.write_agent("coder", "model: haiku\neffort: medium\n")  # effort wins over model
        self.write_agent("advisor", "model: opus\nmemory: project\n")
        self.write_agent("reviewer", "model: sonnet\n")
        self.write_agent("deep", "effort: max\n")
        self.write_agent("plain", "")
        self.run_sync()
        out = self.repo / ".codex/agents"
        self.assertEqual(sorted(p.name for p in out.iterdir()),
                         ["advisor.toml", "coder.toml", "deep.toml", "plain.toml", "reviewer.toml"])
        import tomllib

        parsed = {p.stem: tomllib.loads(p.read_text(encoding="utf-8")) for p in out.iterdir()}
        effort = {name: doc.get("model_reasoning_effort") for name, doc in parsed.items()}
        self.assertEqual(effort, {"coder": "medium", "advisor": "high", "reviewer": "medium",
                                  "deep": "high", "plain": None})
        self.assertEqual(parsed["advisor"]["developer_instructions"], "Do advisor things.\n")
        self.assertNotIn("model", parsed["advisor"])
        self.assertEqual(self.run_sync("--check"), 0)

    def test_removed_agent_is_deleted_but_hand_written_toml_kept(self):
        self.write_agent("keep", "")
        self.write_agent("gone", "")
        self.run_sync()
        hand = self.repo / ".codex/agents/hand.toml"
        hand.write_text('name = "hand"\n', encoding="utf-8")
        (self.repo / ".claude/agents/gone.md").unlink()
        self.assertEqual(self.run_sync("--check"), 1)  # the orphan is drift
        self.run_sync()
        self.assertFalse((self.repo / ".codex/agents/gone.toml").exists())
        self.assertTrue((self.repo / ".codex/agents/keep.toml").is_file())
        self.assertTrue(hand.is_file())
        self.assertEqual(self.run_sync("--check"), 0)

    def test_check_flags_an_edited_agent(self):
        self.write_agent("coder", "")
        self.run_sync()
        self.write_agent("coder", "effort: high\n")
        self.assertEqual(self.run_sync("--check"), 1)
        self.run_sync()
        self.assertEqual(self.run_sync("--check"), 0)

    def test_crlf_source_keeps_crlf(self):
        (self.repo / ".claude/skills/demo/SKILL.md").write_bytes(SKILL.replace("\n", "\r\n").encode())
        self.run_sync()
        data = (self.repo / ".agents/skills/demo/SKILL.md").read_bytes()
        self.assertNotIn(b"\n", data.replace(b"\r\n", b""))


if __name__ == "__main__":
    unittest.main()
