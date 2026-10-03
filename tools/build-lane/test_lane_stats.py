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
import time
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
        self.assertIn("disk_free_gb", first)
        self.assertEqual(first["pruned_mb"], 0)


@unittest.skipUnless(find_bash() and shutil.which("git"), "needs bash and git")
class LaneDiskTests(unittest.TestCase):
    """The low-disk guard and the incremental-session pruning (#1023)."""

    def setUp(self):
        self.root = Path(tempfile.mkdtemp())
        self.addCleanup(shutil.rmtree, self.root, ignore_errors=True)
        # A Dev Drive root with a dir for this checkout makes lane.sh build into it.
        top = subprocess.run(["git", "rev-parse", "--show-toplevel"], cwd=HERE, capture_output=True,
                             text=True, check=True).stdout.strip()
        self.target = self.root / "targets" / Path(top).name
        self.target.mkdir(parents=True)
        self.env = dict(os.environ, LANE_ROOT=str(self.root / "lane-root"), LANE_SLOTS="2", RUSTC_WRAPPER="",
                        CIMMERIA_TARGET_ROOT=str(self.root / "targets"))
        for var in ("LANE_METRICS_DIR", "LANE_MIN_FREE_GB", "LANE_PRUNE", "CIMMERIA_FORCE_DEV_DRIVE"):
            self.env.pop(var, None)

    def lane(self, *cmd, **env):
        return subprocess.run([find_bash(), (HERE / "lane.sh").as_posix(), *cmd], cwd=HERE,
                              env=dict(self.env, **env), capture_output=True, text=True)

    def jobs(self):
        return lane_stats.load(self.root / "lane-root" / "metrics" / "jobs.jsonl")[0]

    def test_refuses_to_start_below_the_free_space_floor(self):
        marker = self.root / "ran"
        r = self.lane("touch", marker.as_posix(), LANE_MIN_FREE_GB="100000000")
        self.assertEqual(r.returncode, 28, r.stderr)
        self.assertFalse(marker.exists(), "the command must not run")
        self.assertIn("refusing to start", r.stderr)
        self.assertIn("rm-worktree.sh --merged", r.stderr)
        self.assertIn("sweep.ps1", r.stderr)
        self.assertFalse([p for p in (self.root / "lane-root" / "lane").iterdir() if p.name.startswith("slot.")],
                         "the slot is released")
        # 0 turns the guard off.
        self.assertEqual(self.lane("touch", marker.as_posix(), LANE_MIN_FREE_GB="0").returncode, 0)
        self.assertTrue(marker.exists())

    def test_prunes_stale_incremental_sessions_after_a_job(self):
        unit = self.target / "debug" / "incremental" / "cimmeria_wire-0abc123def456"
        old, newest = unit / "s-hmpqok75qv-15y8c65-oldsvh", unit / "s-hmptxz1mr4-1oq5s11-newsvh"
        live_working, dead_working = unit / "s-hmpuaaaaaa-2222222-working", unit / "s-hmp0000000-3333333-working"
        for d in (old, newest, live_working, dead_working):
            d.mkdir(parents=True)
            (d / "dep-graph.bin").write_bytes(b"\0" * (2 << 20))
            (unit / (d.name.rsplit("-", 1)[0] + ".lock")).touch()
        hour_ago = time.time() - 2 * 3600
        os.utime(dead_working, (hour_ago, hour_ago))

        r = self.lane("true")
        self.assertEqual(r.returncode, 0, r.stderr)
        left = sorted(p.name for p in unit.iterdir())
        self.assertEqual(left, sorted([newest.name, live_working.name, "s-hmptxz1mr4-1oq5s11.lock",
                                       "s-hmpuaaaaaa-2222222.lock"]),
                         "keeps the newest finished session and a recent -working one, with their locks")
        self.assertIn("pruned", r.stdout + r.stderr)   # the summary line in quiet mode
        self.assertGreaterEqual(self.jobs()[-1]["pruned_mb"], 3)

        # LANE_PRUNE=0 leaves a stale session alone.
        old.mkdir()
        self.assertEqual(self.lane("true", LANE_PRUNE="0").returncode, 0)
        self.assertTrue(old.exists())


@unittest.skipUnless(shutil.which("rustc"), "needs rustc")
class SccacheWrapperTests(unittest.TestCase):
    """sccache-wrap.rs hides the target-dir variables from sccache (#1023): with the
    per-worktree CARGO_TARGET_DIR in sccache's key, no worktree ever hit another's cache."""

    def test_wrapper_hides_target_dir_variables_and_passes_the_rest(self):
        tmp = Path(tempfile.mkdtemp())
        self.addCleanup(shutil.rmtree, tmp, ignore_errors=True)
        exe = tmp / ("sccache.exe" if os.name == "nt" else "sccache")
        # Compiled outside the repo, so rustup uses its default toolchain, not the pin.
        subprocess.run(["rustc", "--edition", "2021", "-o", str(exe), str(HERE / "sccache-wrap.rs")],
                       cwd=tmp, check=True, capture_output=True)
        # Python stands in for sccache: it prints the variables it sees and exits with 7.
        probe = ("import os, sys, json; print(json.dumps({k: v for k, v in os.environ.items() "
                 "if k.startswith(('CARGO_', 'KEEP_'))})); print(sys.argv[1:]); sys.exit(7)")
        env = dict(os.environ, CIMMERIA_SCCACHE_REAL=sys.executable, CARGO_TARGET_DIR="B:/targets/wt-a",
                   CARGO_BUILD_TARGET_DIR="x", CARGO_BUILD_BUILD_DIR="y", CARGO_PKG_NAME="tokio", KEEP_ME="1")
        r = subprocess.run([str(exe), "-c", probe, "rustc", "--crate-name", "tokio"], env=env,
                           capture_output=True, text=True)
        self.assertEqual(r.returncode, 7, "the wrapper passes sccache's exit code through")
        seen = json.loads(r.stdout.splitlines()[0])
        self.assertNotIn("CARGO_TARGET_DIR", seen)
        self.assertNotIn("CARGO_BUILD_TARGET_DIR", seen)
        self.assertNotIn("CARGO_BUILD_BUILD_DIR", seen)
        self.assertEqual(seen.get("CARGO_PKG_NAME"), "tokio", "other CARGO_ variables still reach sccache")
        self.assertEqual(seen.get("KEEP_ME"), "1")
        self.assertIn("'--crate-name', 'tokio'", r.stdout, "arguments pass through unchanged")

        env.pop("CIMMERIA_SCCACHE_REAL")
        r = subprocess.run([str(exe), "rustc", "-vV"], env=env, capture_output=True, text=True)
        self.assertEqual(r.returncode, 2)
        self.assertIn("CIMMERIA_SCCACHE_REAL", r.stderr)


if __name__ == "__main__":
    unittest.main()
