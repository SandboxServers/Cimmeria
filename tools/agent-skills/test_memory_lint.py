"""memory_lint.py flags untyped, stale-state, unindexed, oversized, orphan and IP-like memories (C8)."""
import contextlib
import io
import json
import re
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path

import memory_lint

SUMMARY = re.compile(
    r"^memory_lint files=\d+ no_type=\d+ state_no_invalidated=\d+ index_over=\d+ index_missing=\d+"
    r" unindexed=\d+ orphan_folders=\d+ ip_like=\d+$"
)
MEM = ".claude/agent-memory"


def doc(meta, body):
    """Return a memory file with frontmatter `meta` (the lines between the `---` markers) and a body."""
    return f"---\nname: n\ndescription: d\n{meta}---\n{body}"


def fact(root, folder, name, text, index=True):
    """Write a memory file under a folder and, unless index is False, list it in that folder's MEMORY.md."""
    base = root / MEM / folder
    base.mkdir(parents=True, exist_ok=True)
    (base / name).write_text(text, encoding="utf-8")
    if index:
        with (base / "MEMORY.md").open("a", encoding="utf-8") as out:
            out.write(f"- [{name}]({name})\n")


def tree(root):
    """Write a clean tree: alpha (a user memory), main-session (a reference) and stray, which has no agent file."""
    agents = root / ".claude" / "agents"
    agents.mkdir(parents=True)
    (agents / "alpha.md").write_text("# alpha\n", encoding="utf-8")
    fact(root, "alpha", "a.md", doc("type: user\n", "Prefers short replies.\n"))
    fact(root, "main-session", "m.md", doc("type: reference\n", "Where the ledger lives.\n"))
    fact(root, "stray", "s.md", doc("type: user\n", "Plain note.\n"))


def run(root, *args):
    """Run main() against root; return (exit code, stdout lines)."""
    out = io.StringIO()
    with contextlib.redirect_stdout(out):
        code = memory_lint.main(["--root", str(root), *args])
    return code, out.getvalue().splitlines()


class MemoryLintTest(unittest.TestCase):
    """Each check on a temporary repo root."""

    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        self.root = Path(tmp.name)
        tree(self.root)

    def found(self):
        """Return the (check, path) pairs the scan reports."""
        return {(check, path) for check, path, _ in memory_lint.scan(self.root)["findings"]}

    def test_state_without_invalidated_is_flagged(self):
        fact(self.root, "alpha", "p.md", doc("metadata:\n  type: project\n", "X is not yet implemented.\n"))
        self.assertIn(("state_no_invalidated", f"{MEM}/alpha/p.md"), self.found())

    def test_state_with_invalidated_is_not_flagged(self):
        body = "X is not yet implemented.\nINVALIDATED WHEN: X lands\n"
        fact(self.root, "alpha", "p.md", doc("metadata:\n  type: project\n", body))
        self.assertNotIn(("state_no_invalidated", f"{MEM}/alpha/p.md"), self.found())

    def test_feedback_with_state_phrase_is_not_flagged(self):
        fact(self.root, "alpha", "f.md", doc("type: feedback\n", "X is not yet implemented.\n"))
        self.assertNotIn(("state_no_invalidated", f"{MEM}/alpha/f.md"), self.found())

    def test_file_without_type_is_no_type(self):
        fact(self.root, "alpha", "t.md", doc("", "Untyped note.\n"))
        self.assertIn(("no_type", f"{MEM}/alpha/t.md"), self.found())

    def test_index_of_201_lines_is_index_over(self):
        index = self.root / MEM / "alpha" / "MEMORY.md"
        index.write_text("".join(f"- line {i}\n" for i in range(201)), encoding="utf-8")
        self.assertIn(("index_over", f"{MEM}/alpha/MEMORY.md"), self.found())

    def test_file_missing_from_index_is_unindexed(self):
        fact(self.root, "alpha", "x.md", doc("type: user\n", "Not listed.\n"), index=False)
        self.assertIn(("unindexed", f"{MEM}/alpha/x.md"), self.found())

    def test_only_folder_without_agent_file_is_orphan(self):
        orphans = {path for check, path, _ in memory_lint.scan(self.root)["findings"] if check == "orphan_folders"}
        self.assertEqual(orphans, {f"{MEM}/stray"})

    def test_ip_like_except_loopback(self):
        fact(self.root, "alpha", "ip.md", doc("type: user\n", "Host 10.1.2.3 is the lab.\n"))
        fact(self.root, "alpha", "loop.md", doc("type: user\n", "Loopback 127.0.0.1 is fine.\n"))
        found = self.found()
        self.assertIn(("ip_like", f"{MEM}/alpha/ip.md"), found)
        self.assertNotIn(("ip_like", f"{MEM}/alpha/loop.md"), found)

    def test_summary_line_format(self):
        _, lines = run(self.root)
        self.assertRegex(lines[-1], SUMMARY)
        self.assertIn("orphan_folders=1", lines[-1])

    def test_exit_code_depends_on_strict(self):
        self.assertEqual(run(self.root)[0], 0)
        self.assertEqual(run(self.root, "--strict")[0], 0)
        fact(self.root, "alpha", "bad.md", doc("", "Note.\n"))
        self.assertEqual(run(self.root)[0], 0)
        self.assertEqual(run(self.root, "--strict")[0], 1)

    def test_json_parses(self):
        _, lines = run(self.root, "--json")
        data = json.loads("\n".join(lines[:-1]))
        self.assertRegex(lines[-1], SUMMARY)
        self.assertEqual(data["totals"]["orphan_folders"], 1)
        self.assertIn("alpha", data["folders"])

    @unittest.skipUnless(shutil.which("git"), "git is needed for --since")
    def test_since_counts_added_state_memories(self):
        def git(*args):
            """Run git in the temporary repo, without commit signing."""
            subprocess.run(["git", "-C", str(self.root), "-c", "commit.gpgsign=false",
                            "-c", "user.name=test", "-c", "user.email=test@example.invalid", *args],
                           check=True, capture_output=True)

        git("init", "-q")
        fact(self.root, "alpha", "p.md", doc("metadata:\n  type: project\n", "X is not yet implemented.\n"))
        fact(self.root, "alpha", "q.md", doc("metadata:\n  type: project\n",
                                             "X is not yet implemented.\nINVALIDATED WHEN: X lands\n"))
        git("add", "-A")
        git("commit", "-q", "-m", "memories")
        _, lines = run(self.root, "--since", "2000-01-01")
        self.assertTrue(lines[-1].endswith("state_added=2 state_added_with_invalidated=1"), lines[-1])


if __name__ == "__main__":
    unittest.main()
