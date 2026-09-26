"""Tests for lane_stats.py and the job log lane.sh writes.

    python -m unittest discover -s tools/build-lane -p "test_*.py"
"""

from __future__ import annotations

import json
import os
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import lane_stats  # noqa: E402


def find_bash() -> str | None:
    """A bash that runs lane.sh natively. On Windows that is Git Bash: the `bash` on PATH
    is usually a WSL launcher (System32 or a WindowsApps alias), which runs elsewhere."""
    if os.name != "nt":
        return shutil.which("bash")
    git = shutil.which("git")
    roots = [Path(git).resolve().parent.parent] if git else []   # <Git>\cmd\git.exe -> <Git>
    roots.append(Path(r"C:\Program Files\Git"))
    for root in roots:
        for candidate in (root / "bin" / "bash.exe", root / "usr" / "bin" / "bash.exe"):
            if candidate.exists():
                return str(candidate)
    return None


class ClassifyTests(unittest.TestCase):
    def test_package_check(self):
        self.assertEqual(
            lane_stats.classify("cargo check -p cimmeria-services -p cimmeria-cell-combat --all-targets"),
            ("check", "services,cell-combat", "dev"))

    def test_wrapped_cargo_with_toolchain_and_workspace(self):
        self.assertEqual(
            lane_stats.classify("env -u RUSTC_WRAPPER cargo +1.98.1 clippy --workspace --exclude cimmeria-app"),
            ("clippy", "workspace", "dev"))

    def test_windows_cargo_path_and_release(self):
        kind, scope, profile = lane_stats.classify(r"C:\Users\x\.cargo\bin\cargo.exe build -p cimmeria-server --release")
        self.assertEqual((kind, scope, profile), ("build", "server", "release"))

    def test_nextest_profile_is_not_the_cargo_profile(self):
        self.assertEqual(
            lane_stats.classify("cargo nextest run --profile=ci --workspace --cargo-profile dev-debug"),
            ("nextest", "workspace", "dev-debug"))

    def test_doctest_and_alias(self):
        self.assertEqual(lane_stats.classify("cargo test --doc -p cimmeria-commands")[0], "doctest")
        self.assertEqual(lane_stats.classify("cargo c -p cimmeria-wire")[:2], ("check", "wire"))

    def test_wrapper_scripts(self):
        self.assertEqual(lane_stats.classify("bash -c set -e ... bash $HERE/../test-live-db.sh x")[0], "live-db")
        self.assertEqual(lane_stats.classify("pwsh -File tools/build-metrics/measure-build.ps1 -Label x")[0], "measure")
        self.assertEqual(lane_stats.classify("true")[0], "true")


class ReportTests(unittest.TestCase):
    def job(self, t, kind_cmd, run, exit_code=0, dev=True, wait=0.0):
        return {"v": 1, "t": t, "start": "", "worktree": "wt", "cmd": kind_cmd, "exit": exit_code,
                "wait_s": wait, "run_s": run, "dev_drive": dev, "min_free_mb": 20000,
                "sccache_hits": 3, "sccache_misses": 1}

    def write_log(self, lines):
        d = tempfile.mkdtemp()
        self.addCleanup(shutil.rmtree, d)
        p = Path(d) / "jobs.jsonl"
        p.write_text("\n".join(lines) + "\n", encoding="utf-8")
        return p

    def test_percentile(self):
        self.assertIsNone(lane_stats.percentile([], 50))
        self.assertEqual(lane_stats.percentile([5, 1, 3], 50), 3)
        self.assertAlmostEqual(lane_stats.percentile([0, 10], 90), 9)

    def test_load_skips_bad_lines_and_sorts(self):
        good = [json.dumps(self.job(200, "cargo check", 5)), json.dumps(self.job(100, "cargo build", 50))]
        p = self.write_log([good[0], "{not json", '{"t": 1}', good[1]])
        jobs, bad = lane_stats.load(p)
        self.assertEqual(bad, 2)
        self.assertEqual([j["kind"] for j in jobs], ["build", "check"])

    def test_select_window_and_filters(self):
        jobs = [dict(self.job(t, cmd, 1), kind=k, worktree=w)
                for t, cmd, k, w in [(1000, "cargo check -p a", "check", "bo-c2"),
                                     (90000, "cargo build -p b", "build", "bo-b3"),
                                     (99000, "cargo check -p b", "check", "bo-b3")]]
        now = 100000
        self.assertEqual(len(lane_stats.select(jobs, 1, None, None, None, now)), 2)
        self.assertEqual(len(lane_stats.select(jobs, None, "check", None, None, now)), 2)
        self.assertEqual(len(lane_stats.select(jobs, None, None, "B3", "check", now)), 1)

    def test_report_and_html_render(self):
        lines = [json.dumps(self.job(1790000000 + i * 3600, cmd, run, code, dev))
                 for i, (cmd, run, code, dev) in enumerate([
                     ("cargo check -p cimmeria-services", 42.0, 0, False),
                     ("cargo check -p cimmeria-services", 30.5, 0, True),
                     ("cargo nextest run --workspace", 300.0, 100, True)])]
        jobs, _ = lane_stats.load(self.write_log(lines))
        text = lane_stats.report(jobs, None)
        self.assertIn("3 jobs, 1 failed", text)
        self.assertIn("Dev Drive vs local", text)
        self.assertIn("75%", text)  # sccache hit rate: 3 hits / 4
        out = Path(tempfile.mkdtemp()) / "lane.html"
        self.addCleanup(shutil.rmtree, out.parent)
        lane_stats.write_html(jobs, out, None)
        page = out.read_text(encoding="utf-8")
        self.assertEqual(page.count("<svg"), 2)
        self.assertIn('class="dev fail"', page)


@unittest.skipUnless(find_bash() and shutil.which("git"), "needs bash and git")
class LaneJobLogTests(unittest.TestCase):
    def test_lane_writes_a_readable_line_per_job(self):
        root = tempfile.mkdtemp()
        self.addCleanup(shutil.rmtree, root, ignore_errors=True)
        env = dict(os.environ, LANE_ROOT=root, LANE_SLOTS="2", RUSTC_WRAPPER="")
        env.pop("LANE_METRICS_DIR", None)
        # The awkward command travels in an environment variable: Git Bash re-parses a
        # Windows command line and would eat one of the two backslashes before lane.sh saw it.
        tricky = 'echo "quote\\" back\\\\slash\ttab"; exit 3'
        env["TRICKY"] = tricky
        rc = subprocess.run([find_bash(), "-c", 'exec bash "$0" bash -c "$TRICKY"', (HERE / "lane.sh").as_posix()],
                            cwd=HERE, env=env, capture_output=True).returncode
        self.assertEqual(rc, 3, "lane.sh must pass the command's exit code through")
        subprocess.run([find_bash(), (HERE / "lane.sh").as_posix(), "--exclusive", "true"],
                       cwd=HERE, env=env, check=True, capture_output=True)

        jobs, bad = lane_stats.load(Path(root) / "metrics" / "jobs.jsonl")
        self.assertEqual(bad, 0)
        self.assertEqual(len(jobs), 2)
        first, second = jobs
        self.assertEqual(first["cmd"], "bash -c " + tricky)
        self.assertEqual(first["exit"], 3)
        self.assertFalse(first["exclusive"])
        self.assertEqual((second["exclusive"], second["slots"], second["slots_total"]), (True, 2, 2))
        self.assertEqual(second["busy_at_start"], 0)
        self.assertFalse(first["sccache"], "RUSTC_WRAPPER set by the caller keeps sccache out")
        self.assertGreaterEqual(first["run_s"], 0)
        self.assertFalse(list((Path(root) / "lane").iterdir()), "every slot is released")


if __name__ == "__main__":
    unittest.main()
