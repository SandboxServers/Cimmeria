"""The coder-context subcommand on synthetic packet-coder transcripts: the counts, shares and medians of C2."""

import contextlib
import io
import json
import statistics
import tempfile
import unittest
from argparse import Namespace
from pathlib import Path

from metrics import coder_context
from metrics.transcript_fixtures import write_subagent

DENY = ("PreToolUse:Edit hook error: Context cap reached: 90,515 tokens > 90,000. STOP NOW. "
        "Do not call any more tools.")
LANE = "pwsh -NoProfile -File tools\\build-lane\\lane.ps1 cargo check -p crate"
DOC = "C:\\x\\docs\\analysis\\c\\README.md"


def _fixtures(root):
    """Four packet-coder dispatches and one packet-reviewer, which the subcommand must ignore."""
    write_subagent(root, "s1", "a1", "packet-coder", "Packet HT-03.", [
        ("Read", {"file_path": DOC}, "r" * 4000, 1000),
        ("Grep", {"pattern": "x"}, "g" * 1000, 2000),
        ("PowerShell", {"command": LANE}, "l" * 100, 3000),
    ], "HT-03 done abc1234 -")
    write_subagent(root, "s2", "a2", "packet-coder", "Packet HT-03.", [], "HT-03 blocked - - reason=context")
    write_subagent(root, "s3", "a3", "packet-coder", "Packet HT-03.", [
        ("Edit", {"file_path": "C:\\x\\crates\\a.rs"}, DENY, 5000),
    ], "I stopped at the cap, so the context is full.")
    write_subagent(root, "s4", "a4", "packet-coder", "Continuation 2 of 3, base=b9987073f. Packet HT-03.", [],
                   "HT-03 done abc1234 -")
    write_subagent(root, "s5", "a5", "packet-reviewer", "Review HT-03.", [
        ("Read", {"file_path": DOC}, "r" * 9000, 100),
    ], "HT-03 blocked - - reason=context")


class CoderContextTest(unittest.TestCase):
    def test_counts_shares_and_medians(self):
        with tempfile.TemporaryDirectory() as tmp:
            root, out = Path(tmp) / "projects", Path(tmp) / "out"
            _fixtures(root)
            args = Namespace(projects_root=root, prefix="C--fixture-repo", since=None, until=None, out=out)
            buf = io.StringIO()
            with contextlib.redirect_stdout(buf):
                rc = coder_context.run(args)
            self.assertEqual(rc, 0)
            doc = json.loads((out / "coder-context.json").read_text(encoding="utf-8"))
        self.assertEqual(doc["schema"], "cimmeria-harness-coder-context/1")
        al, st = doc["groups"]["all"], doc["groups"]["stopped"]
        self.assertEqual((al["coders"], al["stops"], al["stop_rate"], al["cap_denied"]), (4, 2, 0.5, 1))
        self.assertEqual((st["coders"], st["stops"], st["stop_rate"]), (2, 2, 1.0))
        total = 4000 + 1000 + 100 + len(DENY)
        shares = al["result_chars_share"]
        self.assertEqual(shares["Read"], round(4000 / total, 3))
        self.assertEqual(shares["Grep"], round(1000 / total, 3))
        self.assertEqual(shares["lane-check"], round(100 / total, 3))
        self.assertEqual(shares["Edit/Write"], round(len(DENY) / total, 3))
        self.assertEqual(al["read_targets_share"]["docs/analysis"], round(4000 / total, 3))
        self.assertEqual(st["result_chars_share"]["Edit/Write"], 1.0)
        med = al["median"]
        self.assertEqual(med["first_context"], 500)
        self.assertEqual(med["last_context"], 0)
        self.assertEqual(med["output_tokens"], 150)
        self.assertEqual(med["requests"], 1.5)
        self.assertEqual(med["result_tokens"], statistics.median([5100 // 4, 0, len(DENY) // 4, 0]))
        self.assertEqual((med["lane_calls"], med["reads"], med["greps"], med["edits"]), (0, 0, 0, 0))
        self.assertEqual((doc["continuations"], doc["continuation_tokens_fresh"]), (1, 100))
        self.assertEqual(doc["median_continuation_fresh"], 100)
        self.assertEqual(buf.getvalue().strip(),
                         f"status=ok coder-context coders=4 stops=2 stop_rate=0.5 read_share={round(4000 / total, 3)} "
                         f"grep_share={round(1000 / total, 3)} continuations=1")


class CategoryTest(unittest.TestCase):
    def test_categories_and_targets(self):
        cat = coder_context.category
        self.assertEqual(cat("Bash", {"command": "git status"}), "shell-git")
        self.assertEqual(cat("PowerShell", {"command": "Get-Content x.md"}), "shell-read")
        self.assertEqual(cat("PowerShell", {"command": LANE}), "lane-check")
        self.assertEqual(cat("Write", {}), "Edit/Write")
        self.assertEqual(cat("WebFetch", {}), "other")
        self.assertEqual(coder_context.read_target("C:\\repo\\.claude\\agents\\x.md"), ".claude")
        self.assertEqual(coder_context.read_target("C:\\repo\\docs\\index.md"), "docs-other")
        self.assertEqual(coder_context.read_target("C:\\repo\\LICENSE"), "other")


if __name__ == "__main__":
    unittest.main()