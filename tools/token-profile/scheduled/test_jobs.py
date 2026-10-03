"""The scheduled jobs' windows, lock, step order and exit status, with a fake runner that starts no process."""

import io
import os
import subprocess
import tempfile
import unittest
from contextlib import redirect_stderr, redirect_stdout
from datetime import datetime, timezone
from pathlib import Path

from . import jobs

# A Monday 08:00 in UTC-5 local time.
NOW = datetime(2026, 10, 5, 13, 0, tzinfo=timezone.utc)


class FakeRun:
    """Records each child command; `codes` maps a tool directory name to its exit code."""

    def __init__(self, codes=None):
        self.codes = codes or {}
        self.calls = []

    def __call__(self, argv, **kw):
        tool = Path(argv[1]).name
        self.calls.append((tool, argv[2:]))
        kw["stdout"].write(f"fake {tool}\n")
        return subprocess.CompletedProcess(argv, self.codes.get(tool, 0))


class WindowTest(unittest.TestCase):
    def test_weekly_window_is_the_seven_whole_utc_days_before_today(self):
        self.assertEqual(jobs.weekly_window(NOW), ("2026-09-28T00:00:00Z", "2026-10-05T00:00:00Z"))

    def test_weekly_window_uses_the_utc_day_not_the_local_one(self):
        late_local = datetime(2026, 10, 5, 1, 30, tzinfo=timezone.utc)  # still Sunday evening in UTC-5
        self.assertEqual(jobs.weekly_window(late_local)[1], "2026-10-05T00:00:00Z")

    def test_weekly_window_until_and_days(self):
        self.assertEqual(jobs.weekly_window(NOW, 14, "2026-10-01"), ("2026-09-17T00:00:00Z", "2026-10-01T00:00:00Z"))

    def test_sweep_since_counts_whole_days_back(self):
        self.assertEqual(jobs.sweep_since(NOW), "2026-10-02")
        self.assertEqual(jobs.sweep_since(NOW, 1), "2026-10-04")


class LockTest(unittest.TestCase):
    def test_second_holder_is_refused_and_a_stale_lock_is_taken_over(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "job.lock"
            with jobs.Lock(path):
                with self.assertRaises(jobs.Busy):
                    with jobs.Lock(path):
                        pass
                later = path.stat().st_mtime + jobs.STALE_LOCK_SECONDS + 1
                with jobs.Lock(path, clock=lambda: later):
                    self.assertTrue(path.exists())
            self.assertFalse(path.exists())


class JobTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.home = Path(self.tmp.name) / "home"
        self.runner = FakeRun()

    def tearDown(self):
        self.tmp.cleanup()

    def run_job(self, *argv):
        runner = self.runner

        class FakeSteps(jobs.Steps):
            def __init__(self, log_dir):
                super().__init__(log_dir, emit=lambda s: None, run=runner)

        out = io.StringIO()
        with redirect_stdout(out), redirect_stderr(io.StringIO()):
            code = jobs.main([*argv, "--home", str(self.home), "--db", "p.sqlite", "--repo", "checkout"],
                             now=NOW, steps_cls=FakeSteps)
        return code, out.getvalue()

    def test_weekly_runs_every_step_on_the_window(self):
        code, out = self.run_job("weekly")
        self.assertEqual(code, 0)
        self.assertEqual([t for t, _ in self.runner.calls], ["ingest", "reconcile", "validate", "report"])
        ingest = self.runner.calls[0][1]
        self.assertIn("--fetch-prs", ingest)
        self.assertEqual(ingest[ingest.index("--repo") + 1], "checkout")
        for tool, args in self.runner.calls[1::2]:  # reconcile and report share the window
            self.assertEqual(args[args.index("--since") + 1], "2026-09-28T00:00:00Z", tool)
            self.assertEqual(args[args.index("--until") + 1], "2026-10-05T00:00:00Z", tool)
        validate = self.runner.calls[2][1]
        self.assertEqual(validate[validate.index("--max-wrong-share") + 1], "0.10")
        out_dir = self.home / "reports" / "2026-10-05"
        self.assertIn("status=ok", (out_dir / "weekly-summary.txt").read_text(encoding="utf-8"))
        self.assertTrue((out_dir / "reconcile.log").is_file())
        self.assertFalse((self.home / "job.lock").exists())

    def test_weekly_keeps_going_after_a_failed_check_and_names_it(self):
        self.runner.codes = {"reconcile": 4}
        code, out = self.run_job("weekly")
        self.assertEqual(code, 1)
        self.assertEqual(len(self.runner.calls), 4)
        self.assertIn("status=failed reconcile (a check is out of tolerance)", out)

    def test_an_unknown_exit_code_is_reported_as_a_crash(self):
        self.runner.codes = {"report": 1}
        code, out = self.run_job("weekly", "--skip-ingest")
        self.assertEqual(code, 1)
        self.assertNotIn("ingest", [t for t, _ in self.runner.calls])
        self.assertIn("report (crashed, see report.log)", out)

    def test_weekly_passes_labels(self):
        self.run_job("weekly", "--labels", "labels.csv")
        validate = self.runner.calls[2][1]
        self.assertEqual(validate[validate.index("--labels") + 1], "labels.csv")

    def test_pr_sweep_restarts_its_own_state_over_the_window_and_posts(self):
        code, _ = self.run_job("pr-sweep")
        self.assertEqual(code, 0)
        self.assertEqual([t for t, _ in self.runner.calls], ["ingest", "pr_stats"])
        args = self.runner.calls[1][1]
        for flag in ("--backfill", "--restart", "--post"):
            self.assertIn(flag, args)
        self.assertEqual(args[args.index("--since") + 1], "2026-10-02")
        self.assertEqual(args[args.index("--rate") + 1], "6/min")
        self.assertEqual(Path(args[args.index("--state") + 1]), self.home / "pr-sweep-state.json")
        logs = self.home / "logs"
        self.assertTrue((logs / "pr-sweep-2026-10-05.log").is_file())
        self.assertTrue((logs / "pr-sweep-2026-10-05-summary.txt").is_file())

    def test_pr_sweep_dry_run_posts_nothing(self):
        self.run_job("pr-sweep", "--dry-run", "--days", "5", "--rate", "2/min")
        args = self.runner.calls[1][1]
        self.assertNotIn("--post", args)
        self.assertEqual(args[args.index("--since") + 1], "2026-09-30")
        self.assertEqual(args[args.index("--rate") + 1], "2/min")

    def test_a_held_lock_exits_2_and_runs_nothing(self):
        self.home.mkdir(parents=True)
        (self.home / "job.lock").write_text("1", encoding="utf-8")
        code, _ = self.run_job("pr-sweep")
        self.assertEqual(code, 2)
        self.assertEqual(self.runner.calls, [])

    def test_bad_arguments_are_usage_errors(self):
        for argv in (["weekly", "--days", "0"], ["weekly", "--until", "Monday"], ["nightly"]):
            with self.assertRaises(SystemExit) as cm, redirect_stderr(io.StringIO()):
                jobs.main(argv, now=NOW)
            self.assertEqual(cm.exception.code, 2, argv)

    def test_env_vars_set_the_defaults(self):
        old = {k: os.environ.get(k) for k in ("TOKEN_PROFILE_HOME", "TOKEN_PROFILE_DB")}
        try:
            os.environ["TOKEN_PROFILE_HOME"] = str(self.home)
            os.environ["TOKEN_PROFILE_DB"] = "elsewhere.sqlite"
            self.assertEqual(jobs.default_home(), self.home)
            self.assertEqual(jobs.default_db(), Path("elsewhere.sqlite"))
        finally:
            for k, v in old.items():
                if v is None:
                    os.environ.pop(k, None)
                else:
                    os.environ[k] = v


if __name__ == "__main__":
    unittest.main()
