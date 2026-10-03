"""Tests for lane_summary.py and lane.sh's quiet output for agents.

    python -m unittest discover -s tools/build-lane -p "test_*.py"

The fixtures under fixtures/summary/ are real cargo, clippy and nextest output from a
throwaway crate (paths scrubbed), with the lane's own lines removed.
"""

from __future__ import annotations

import os
import shutil
import subprocess
import sys
import tempfile
import time
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import lane_stats  # noqa: E402
import lane_summary  # noqa: E402
from test_lane_stats import find_bash  # noqa: E402

FIXTURES = HERE / "fixtures" / "summary"


def summarise(name: str, exit_code: int) -> tuple[str, str]:
    text = (FIXTURES / name).read_text(encoding="utf-8")
    return lane_summary.render(lane_summary.parse(text), text, exit_code, "1.000", None, "F.txt", "L.log")


class SummaryTests(unittest.TestCase):
    def test_compile_error_shows_each_error_with_its_location(self):
        out, full = summarise("check_error.txt", 101)
        self.assertTrue(out.startswith("[lane] status=failed exit=101 ran=1.000s\n"), out)
        self.assertIn("errors: 2", out)
        self.assertIn("error[E0425]: cannot find function `undefined_fn` in this scope", out)
        self.assertIn("  --> src\\lib.rs:32:5", out)
        self.assertIn("error[E0308]: mismatched types", out)
        self.assertIn("42.to_string()", out, "the help suggestion is part of the error")
        self.assertIn("error: could not compile `demo` (lib) due to 2 previous errors", out)
        self.assertNotIn("Checking demo", out)
        self.assertNotIn("Some errors have detailed explanations", out)
        self.assertIn("failures: F.txt", out)
        self.assertTrue(out.endswith("log: L.log\n"))
        self.assertIn("error[E0308]: mismatched types", full)

    def test_nextest_build_error_counts_each_error_once(self):
        out, _ = summarise("nextest_compile_error.txt", 101)
        self.assertIn("errors: 2", out, "the lib and lib-test builds report the same two errors")
        self.assertEqual(out.count("error[E0425]"), 1)
        self.assertNotIn("cargo.exe' test --no-run", out, "the repeated cargo command line is noise")
        self.assertNotIn("tests (", out)

    def test_clippy_denied_warning_is_an_error(self):
        out, _ = summarise("clippy_deny.txt", 101)
        self.assertIn("errors: 1", out)
        self.assertIn("error: unused variable: `unused`", out)
        self.assertIn("implied by `-D warnings`", out)

    def test_success_with_warnings_lists_them_one_line_each(self):
        out, full = summarise("clippy_warning.txt", 0)
        self.assertTrue(out.startswith("[lane] status=ok exit=0"), out)
        self.assertIn("warnings: 1\n  warning: unused variable: `unused` (src\\lib.rs:31:9)\n", out)
        self.assertNotIn("generated 1 warning", out, "cargo's roll-up line is not a warning of its own")
        self.assertNotIn("failures:", out)
        self.assertEqual(full, "")

    def test_cargo_test_failure_and_panic(self):
        out, full = summarise("cargo_test_fail.txt", 101)
        self.assertIn("tests (libtest): 1 passed, 2 failed, 1 ignored", out)
        self.assertIn("failed tests: 2", out)
        self.assertIn("FAIL tests::wrong_sum\n  thread 'tests::wrong_sum' (2287708) panicked at src\\lib.rs:16:9:\n"
                      "  assertion `left == right` failed: sum of 2 and 2\n    left: 4\n   right: 5\n", out)
        self.assertIn("FAIL tests::explodes\n", out)
        self.assertIn("index out of bounds: the len is 0 but the index is 3", out)
        self.assertNotIn("RUST_BACKTRACE", out)
        self.assertIn("error: test failed, to rerun pass `--lib`", out)
        self.assertIn("FAIL tests::explodes", full)

    def test_nextest_failure_and_panic(self):
        out, _ = summarise("nextest_fail.txt", 100)
        self.assertIn("tests (nextest): 1 passed, 2 failed, 1 skipped", out)
        self.assertIn("FAIL demo tests::explodes\n  thread 'tests::explodes' (2260832) panicked at src\\lib.rs:22:18:\n"
                      "  index out of bounds: the len is 0 but the index is 3\n", out)
        self.assertIn("    left: 4\n   right: 5\n", out, "nextest's indent is removed, the alignment kept")
        self.assertNotIn("test result:", out, "the per-binary libtest line inside nextest output is not a count")
        self.assertEqual(out.count("FAIL demo tests::wrong_sum"), 1, "listed once though nextest prints it twice")

    def test_nextest_default_flags_skip_progress_noise(self):
        out, _ = summarise("nextest_fail_default_flags.txt", 100)
        self.assertIn("failed tests: 2", out)
        self.assertNotIn("Cancelling", out)

    def test_successful_runs_are_three_lines(self):
        out, _ = summarise("cargo_test_ok.txt", 0)
        self.assertEqual(out, "[lane] status=ok exit=0 ran=1.000s\ntests (libtest): 33 passed, 0 failed\nlog: L.log\n")
        out, _ = summarise("nextest_ok.txt", 0)
        self.assertEqual(out, "[lane] status=ok exit=0 ran=1.000s\ntests (nextest): 296 passed, 0 skipped\nlog: L.log\n")

    def test_unrecognised_failure_shows_the_end_of_the_log(self):
        text = "[lane] acquired 1/4 slot(s)\n" + "".join(f"line {i}\n" for i in range(40)) + "psql: could not connect\n"
        out, full = lane_summary.render(lane_summary.parse(text), text, 2, None, None, "F.txt", "L.log")
        self.assertIn("no compiler error or failing test recognised", out)
        self.assertIn("  psql: could not connect\n", out)
        self.assertNotIn("line 15\n", out, "only the last lines")
        self.assertNotIn("[lane] acquired", out)
        self.assertIn("psql: could not connect", full)

    def test_long_output_is_capped(self):
        block = "error[E0425]: cannot find value `x{}` in this scope\n  --> src\\lib.rs:{}:5\n" + "   |\n" * 30 + "\n"
        text = "".join(block.format(i, i) for i in range(20))
        out, full = lane_summary.render(lane_summary.parse(text), text, 101, None, None, "F.txt", "L.log")
        self.assertIn("errors: 20", out)
        self.assertIn("... and 12 more errors in the failures file", out)
        self.assertIn("more lines in the failures file", out)
        self.assertLess(len(out), 6000)
        self.assertEqual(full.count("error[E0425]"), 20, "the failures file has every error")


@unittest.skipUnless(find_bash() and shutil.which("git"), "needs bash and git")
class LaneQuietTests(unittest.TestCase):
    """lane.sh with stdout captured (not a terminal), as an agent's Bash tool runs it."""

    def setUp(self):
        self.root = Path(tempfile.mkdtemp())
        self.addCleanup(shutil.rmtree, self.root, ignore_errors=True)
        self.env = dict(os.environ, LANE_ROOT=str(self.root), LANE_SLOTS="2", RUSTC_WRAPPER="", LANE_MIN_FREE_GB="0")
        for var in ("CI", "LANE_VERBOSE", "LANE_METRICS_DIR", "LANE_LOG_KEEP", "LANE_LOG_DAYS",
                    "NEXTEST_STATUS_LEVEL", "NEXTEST_SHOW_PROGRESS", "NEXTEST_FAILURE_OUTPUT"):
            self.env.pop(var, None)

    def lane(self, script: str, **env) -> subprocess.CompletedProcess:
        # The command travels in an environment variable; see test_lane_stats.py.
        return subprocess.run([find_bash(), "-c", 'exec bash "$0" bash -c "$JOB"', (HERE / "lane.sh").as_posix()],
                              cwd=HERE, env=dict(self.env, JOB=script, **env), capture_output=True, text=True)

    def logs(self) -> list[Path]:
        return sorted((self.root / "logs").glob("*/*.log"))

    def test_quiet_run_summarises_and_keeps_the_full_log(self):
        fixture = (FIXTURES / "cargo_test_fail.txt").as_posix()
        r = self.lane(f'cat "{fixture}"; echo "nextest: $NEXTEST_STATUS_LEVEL $NEXTEST_SHOW_PROGRESS '
                      f'$NEXTEST_FAILURE_OUTPUT"; exit 101')
        self.assertEqual(r.returncode, 101, r.stderr)
        self.assertTrue(r.stdout.startswith("[lane] status=failed exit=101 ran="), r.stdout)
        self.assertIn("FAIL tests::wrong_sum", r.stdout)
        self.assertNotIn("running 4 tests", r.stdout, "the full output stays in the log")
        self.assertNotIn("[lane] acquired", r.stdout + r.stderr)
        [log] = self.logs()
        text = log.read_text(encoding="utf-8")
        self.assertIn("running 4 tests", text)
        self.assertIn("nextest: fail none final", text, "nextest's quiet flags are exported")
        self.assertIn("[lane] acquired", text)
        self.assertIn("[lane] released (exit 101", text)
        failures = log.with_name(log.name[:-len(".log")] + ".failures.txt")
        self.assertIn(f"failures: {failures.as_posix()}", r.stdout.replace("\\", "/"))
        self.assertIn("index out of bounds", failures.read_text(encoding="utf-8"))
        self.assertIn(f"log: {log.as_posix()}", r.stdout.replace("\\", "/"))
        job = lane_stats.load(self.root / "metrics" / "jobs.jsonl")[0][-1]
        self.assertTrue(job["quiet"])
        self.assertEqual(Path(job["log"]).resolve(), log.resolve())

    def test_success_prints_only_the_summary(self):
        r = self.lane("echo lots of output; echo more")
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertEqual(r.stdout.splitlines()[0][:29], "[lane] status=ok exit=0 ran=0")
        self.assertEqual(len(r.stdout.splitlines()), 2, r.stdout)
        self.assertNotIn("lots of output", r.stdout + r.stderr)
        [log] = self.logs()
        self.assertEqual(r.stderr.replace("\\", "/"), f"[lane] running; log: {log.as_posix()}\n",
                         "stderr has only the log path, printed before the job runs")

    def test_verbose_and_ci_keep_the_full_output(self):
        for env in ({"LANE_VERBOSE": "1"}, {"CI": "true"}):
            with self.subTest(env=env):
                r = self.lane('echo "full output $NEXTEST_STATUS_LEVEL"; exit 3', **env)
                self.assertEqual(r.returncode, 3)
                self.assertEqual(r.stdout, "full output \n", "no summary and no nextest overrides")
                self.assertIn("[lane] acquired", r.stderr)
                self.assertIn("[lane] released (exit 3", r.stderr)
        self.assertEqual(self.logs(), [], "no log file outside quiet mode")
        jobs = lane_stats.load(self.root / "metrics" / "jobs.jsonl")[0]
        self.assertEqual([j["quiet"] for j in jobs], [False, False])

    def test_caller_nextest_settings_win(self):
        self.lane('echo "level=$NEXTEST_STATUS_LEVEL"', NEXTEST_STATUS_LEVEL="all")
        self.assertIn("level=all", self.logs()[0].read_text(encoding="utf-8"))

    def test_logs_are_pruned(self):
        for _ in range(4):
            self.assertEqual(self.lane("true", LANE_LOG_KEEP="2").returncode, 0)
            time.sleep(1.1)                       # log names have one-second resolution
        self.assertEqual(len(self.logs()), 2, "keeps the newest LANE_LOG_KEEP")
        # Another worktree's log older than LANE_LOG_DAYS goes too, and its empty dir.
        other = self.root / "logs" / "gone-worktree"
        other.mkdir()
        old = other / "20200101-000000-1.log"
        old.write_text("x")
        week = time.time() - 8 * 86400
        os.utime(old, (week, week))
        self.lane("true")
        self.assertFalse(other.exists())

    def test_falls_back_to_the_log_tail_without_python(self):
        r = self.lane("echo the real error; exit 5", PATH="/usr/bin:/bin")
        if "lane_summary.py did not run" not in r.stdout:
            self.skipTest("a Python is still reachable on the reduced PATH")
        self.assertEqual(r.returncode, 5)
        self.assertIn("status=failed exit=5", r.stdout)
        self.assertIn("the real error", r.stdout)
        self.assertIn("log: ", r.stdout)


if __name__ == "__main__":
    unittest.main()
