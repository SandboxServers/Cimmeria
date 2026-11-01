"""The `returns` subcommand (C2), on seven synthetic dispatches in a temp folder: the group counts, the coder lines,
the stdout line and the written file."""

import argparse
import contextlib
import io
import json
import tempfile
import unittest
import unittest.mock
from pathlib import Path

from metrics import cli, returns
from metrics.transcript_fixtures import FIXTURE_PREFIX, write_agent_file, write_subagent

R1 = ".claude/reports/harness-tuning/R1.md"
R2 = ".claude/reports/harness-tuning/R2.md"
R3 = ".claude/reports/harness-tuning/R3.md"
OPS = ".claude/reports/harness-tuning/OPS1.md"
DENY = ("PreToolUse:Edit hook error: Context cap reached: 90,515 tokens > 90,000. STOP NOW. "
        "Do not call any more tools.")
PROSE = "The review found several issues worth discussing in the notes. " * 4


def _fixtures(root):
    """The seven dispatches of the test: two reviewers and a buried one, a coder with no report path, a capped coder,
    a continuation coder with no line, and an ops dispatch that names a report path."""
    write_subagent(root, "s1", "a1", "packet-reviewer",
                   f"Review HT-01 in C:\\Users\\fixture\\wt. Write the report to {R1}",
                   [("Write", {"file_path": "C:\\fixture\\.claude\\reports\\harness-tuning\\R1.md"}, "ok", 1000)],
                   f"R1 done - {R1}")
    write_subagent(root, "s1", "a2", "packet-reviewer", f"Review HT-02. Write the report to {R2}",
                   [("Read", {"file_path": "notes.md"}, "text", 500)], PROSE)
    write_subagent(root, "s1", "a3", "packet-reviewer", f"Review HT-03. Write the report to {R3}",
                   [("PowerShell", {"command": "Set-Content -Path .claude/reports/harness-tuning/R3.md -Value $t"},
                     "", 800)],
                   f"Two notes on the change.\nR3 done - {R3}")
    write_subagent(root, "s1", "a4", "packet-coder", "Implement the HT-01 fix and run the checks.",
                   [("Read", {"file_path": "transcripts.py"}, "    1\tx", 2000)], "HT-01 done abc1234 -")
    write_subagent(root, "s1", "a5", "packet-coder", "Implement the HT-02 returns subcommand.",
                   [("Edit", {"file_path": "returns.py"}, DENY, 3000)],
                   "Stopped at the cap. The branch holds the partial work, described in prose so that the text is "
                   "not a return line.")
    write_subagent(root, "s1", "a6", "packet-coder",
                   "Continuation 2 of 3 for HT-02. Read the worknote and carry on.",
                   [("Read", {"file_path": "returns.py"}, "ok", 1500)],
                   "Still working through the tests, nothing to report yet.")
    write_subagent(root, "s1", "a7", "ops", f"Run the registry check and write the result to {OPS}.",
                   [("PowerShell", {"command": "python tools/token-profile/metrics registry --check"}, "ok", 600)],
                   "Registry checked.")


class ReturnsTest(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.root = Path(self._tmp.name)
        self.projects = self.root / "projects"
        self.repo = self.root / "repo"
        self.out = self.root / "out"
        write_agent_file(self.repo, "packet-reviewer", "sonnet")
        write_agent_file(self.repo, "packet-coder", "haiku")
        write_agent_file(self.repo, "ops", "haiku", reporting=False)
        _fixtures(self.projects)

    def tearDown(self):
        self._tmp.cleanup()

    def _run(self, since=None, until=None):
        """Run the subcommand on the fixtures; return its exit code and its stdout line."""
        args = argparse.Namespace(cmd="returns", projects_root=self.projects, repo=self.repo, since=since,
                                  until=until, out=str(self.out), prefix=FIXTURE_PREFIX)
        buf = io.StringIO()
        with contextlib.redirect_stdout(buf):
            code = returns.run(args)
        return code, buf.getvalue().strip()

    def _json(self):
        """The written returns.json, parsed."""
        return json.loads((self.out / "returns.json").read_text(encoding="utf-8"))

    def test_all_counts(self):
        code, _ = self._run()
        self.assertEqual(code, 0)
        all_ = self._json()["groups"]["all"]
        expected = {
            "dispatches": 7, "contract_dispatches": 4, "strict": 1, "line_first": 0, "buried": 1, "prose": 1,
            "short": 1, "empty": 0, "no_usable_line": 2, "no_report_write": 2, "context_stops": 1,
            "cap_denials": 1, "continuation_dispatches": 1, "redispatches": 0, "reviewer_lines": 2,
            "not_assessed_suffix": 0, "tokens_total": 10800, "tokens_fresh": 10800, "no_usable_tokens_total": 1500,
            "no_usable_tokens_fresh": 1500, "no_report_tokens_total": 1500, "no_report_tokens_fresh": 1500,
        }
        self.assertEqual({k: all_[k] for k in expected}, expected)

    def test_groups_and_report_writers(self):
        self._run()
        groups = self._json()["groups"]
        self.assertEqual(set(groups), {"all", "all_but_ops", "report_writers", "packet-reviewer", "packet-coder",
                                       "ops"})
        self.assertEqual(groups["report_writers"]["dispatches"], 3)
        self.assertEqual(groups["report_writers"]["no_report_write"], 1)
        self.assertEqual(groups["all_but_ops"]["dispatches"], 6)
        self.assertEqual(groups["packet-coder"]["context_stops"], 1)
        self.assertEqual(groups["packet-coder"]["tokens_total"], 7100)

    def test_coder_lines(self):
        self._run()
        self.assertEqual(self._json()["coder_lines"], {
            "dispatches": 3, "capped": 1, "capped_usable_line": 0, "uncapped": 2, "no_usable_line_uncapped": 1,
            "dash_report_lines": 1, "not_run_suffix": 0,
        })

    def test_stdout_line(self):
        _, line = self._run()
        self.assertEqual(line, "status=ok returns contract=4 writers_no_line=33.3 writers_no_report=33.3 "
                               "coder_no_line_uncapped=50.0 coder_capped=1")

    def test_written_file_holds_no_session_or_user_path(self):
        self._run()
        text = (self.out / "returns.json").read_text(encoding="utf-8")
        self.assertEqual(json.loads(text)["schema"], "cimmeria-harness-returns/1")
        self.assertNotIn('"s1"', text)
        self.assertNotIn("Users", text)

    def test_window_excludes_later_dispatches(self):
        code, _ = self._run(since="2026-10-11")
        self.assertEqual(code, 0)
        self.assertEqual(self._json()["groups"]["all"]["dispatches"], 0)
        self.assertEqual(self._json()["window"], {"since": "2026-10-11", "until": None})

    def test_redispatch_counts_repeat_report_paths_but_not_continuations(self):
        write_subagent(self.projects, "s1", "a8", "packet-reviewer",
                       f"Review HT-01 again. Write the report to {R1}", [], f"R1 done - {R1}",
                       ts="2026-10-10T11:00:00Z")
        write_subagent(self.projects, "s1", "a9", "packet-coder",
                       f"Continuation 3 of 3 for HT-01. Report path {R1}", [], "still going",
                       ts="2026-10-10T12:00:00Z")
        self._run()
        groups = self._json()["groups"]
        self.assertEqual(groups["all"]["redispatches"], 1)
        self.assertEqual(groups["packet-reviewer"]["redispatches"], 1)

    def test_privacy_gate_refuses(self):
        with unittest.mock.patch.object(returns.Scrubber, "assert_clean",
                                        side_effect=returns.PrivacyError("x")), \
                contextlib.redirect_stderr(err := io.StringIO()):
            code, _ = self._run()
        self.assertEqual(code, 3)
        self.assertIn("status=error privacy", err.getvalue())
        self.assertFalse((self.out / "returns.json").exists())

    def test_missing_projects_root_exits_2(self):
        self.projects = self.root / "nowhere"
        err = io.StringIO()
        with contextlib.redirect_stderr(err):
            code, _ = self._run()
        self.assertEqual(code, 2)
        self.assertTrue(err.getvalue().startswith("status=error"))

    def test_cli_main_runs_returns(self):
        argv = ["returns", "--projects-root", str(self.projects), "--repo", str(self.repo), "--out", str(self.out)]
        with contextlib.redirect_stdout(io.StringIO()):
            code = cli.main(argv)
        self.assertEqual(code, 0)
        self.assertTrue((self.out / "returns.json").is_file())

    def test_suffix_and_edge_counts(self):
        self.projects = self.root / "p2"
        write_subagent(self.projects, "s1", "b1", "packet-reviewer", f"Review HT-01. Write the report to {R1}", [],
                       f"R1 done abc1234 {R1} not_assessed=2")
        write_subagent(self.projects, "s1", "b2", "packet-reviewer", f"Review HT-02. Write the report to {R2}", [],
                       f"R2 done abc1234 {R2}\nExtra text.")
        write_subagent(self.projects, "s1", "b3", "packet-reviewer", f"Review HT-03. Write the report to {R3}", [], "")
        write_subagent(self.projects, "s1", "b4", "packet-coder", "Implement the HT-04 returns check.",
                       [("Edit", {"file_path": "returns.py"}, DENY, 1000)], "HT-04 blocked abc1234 - reason=context")
        write_subagent(self.projects, "s1", "b5", "packet-coder", "Implement the HT-04 returns check again.", [],
                       "HT-04 done abc1234 - not_run=2,3")
        self._run()
        all_ = self._json()["groups"]["all"]
        self.assertEqual(all_["line_first"], 1)
        self.assertEqual(all_["empty"], 1)
        self.assertEqual(all_["not_assessed_suffix"], 1)
        self.assertEqual(all_["reviewer_lines"], 2)
        coder = self._json()["coder_lines"]
        self.assertEqual(coder["capped_usable_line"], 1)
        self.assertEqual(coder["not_run_suffix"], 1)


if __name__ == "__main__":
    unittest.main()
