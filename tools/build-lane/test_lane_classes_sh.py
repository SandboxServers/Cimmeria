"""Tests for the bash lane's job classes: lane-class.sh and lane.sh (heavy, light, db), and
the two-phase live-db-test.sh. The PowerShell twin is test-lane-classes.ps1; both read
fixtures/lane-classes.tsv. Every lane call uses a scratch LANE_ROOT, never the real lane.
Bash comes from find_bash (Git Bash on Windows); the tests skip with a reason without it.
"""

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
from test_lane_stats import find_bash  # noqa: E402  Git Bash on Windows, PATH bash elsewhere

BASH = find_bash()
LANE_SH = HERE / "lane.sh"
CLASS_SH = HERE / "lane-class.sh"
LIVE_DB_SH = HERE / "live-db-test.sh"
FIXTURE = HERE / "fixtures" / "lane-classes.tsv"
SKIP = "no bash found (Git Bash on Windows)"
DROP_PREFIXES = ("LANE_", "CIMMERIA_", "CARGO_", "DATABASE_URL")


def posix(path):
    """A path in the form bash and Git Bash take as an argument (C:/x)."""
    return Path(path).as_posix()


def lane_env(root, **extra):
    """The environment of a test lane call: scratch root, no slot holders' noise, no sccache."""
    env = {k: v for k, v in os.environ.items() if not k.startswith(DROP_PREFIXES)}
    env.update({"LANE_ROOT": str(root), "LANE_MIN_FREE_GB": "0", "LANE_VERBOSE": "1",
                "LANE_PRUNE": "0", "LANE_METRICS": "1", "RUSTC_WRAPPER": "none"})
    env.update(extra)
    return env


def run(args, env, timeout=60):
    """Runs bash with the given args. Output goes to temp files, not pipes: a lane job left
    running after a timeout would keep a pipe open and hang the test. On a timeout the whole
    process tree is killed and the result has returncode -1."""
    with tempfile.TemporaryFile() as out, tempfile.TemporaryFile() as err:
        proc = subprocess.Popen([BASH, *args], cwd=str(HERE), env=env, stdout=out, stderr=err)
        try:
            code = proc.wait(timeout=timeout)
        except subprocess.TimeoutExpired:
            if os.name == "nt":
                subprocess.run(["taskkill", "/T", "/F", "/PID", str(proc.pid)], capture_output=True)
            else:
                proc.kill()
            proc.wait()
            code = -1
        texts = []
        for stream in (out, err):
            stream.seek(0)
            texts.append(stream.read().decode("utf-8", errors="replace"))
    return subprocess.CompletedProcess(args, code, texts[0], texts[1])


def job_lines(root):
    """The raw jobs.jsonl lines of a scratch lane, oldest first."""
    path = root / "metrics" / "jobs.jsonl"
    if not path.exists():
        return []
    return [line for line in path.read_text(encoding="utf-8").splitlines() if line.strip()]


def jobs(root):
    return [json.loads(line) for line in job_lines(root)]


@unittest.skipUnless(BASH, SKIP)
class ClassRuleTests(unittest.TestCase):
    def test_every_fixture_row_classifies_as_listed(self):
        script = (
            '. "$1"\n'
            "set -f\n"
            "while IFS=$'\\t' read -r want line; do\n"
            "  case \"$want\" in ''|'#'*) continue ;; esac\n"
            "  set -- $line\n"
            "  printf '%s\\t%s\\t%s\\n' \"$want\" \"$(lane_class \"$@\")\" \"$line\"\n"
            "done < \"$2\"\n")
        with tempfile.TemporaryDirectory(prefix="lane-classes-rows-") as tmp:
            script_path = Path(tmp) / "rows.sh"
            script_path.write_text(script, encoding="utf-8")
            result = subprocess.run([BASH, posix(script_path), posix(CLASS_SH), posix(FIXTURE)],
                                    capture_output=True, text=True, encoding="utf-8",
                                    errors="replace", timeout=120)
        self.assertEqual(result.returncode, 0, result.stderr)
        rows = [line.split("\t") for line in result.stdout.splitlines()]
        self.assertGreaterEqual(len(rows), 30)
        for want, got, line in rows:
            self.assertEqual(got, want, f"{line!r} classified as {got}")

    def test_counts_follow_env_then_file_then_default(self):
        with tempfile.TemporaryDirectory(prefix="lane-classes-count-") as tmp:
            lock = Path(tmp) / "lane"
            lock.mkdir()
            (lock / "LIGHT_SLOTS").write_text("2\n", encoding="utf-8")
            (lock / "DB_SLOTS").write_text("junk\n", encoding="utf-8")
            cases = [
                ({}, "light", "2"),                          # file
                ({"LANE_LIGHT_SLOTS": "0"}, "light", "0"),   # env beats file; 0 is kept
                ({"LANE_LIGHT_SLOTS": "x"}, "light", "2"),   # junk env counts as unset
                ({}, "db", "3"),                             # junk file counts as unset
                ({"LANE_SLOTS": "5"}, "heavy", "5"),
                ({"LANE_SLOTS": "0"}, "heavy", "2"),         # heavy is never 0: default
            ]
            for extra, cls, want in cases:
                env = lane_env(tmp, **extra)
                script = f'LOCKDIR="$1"; . "$2"; lane_class_count {cls}'
                result = run(["-c", script, "_", posix(lock), posix(CLASS_SH)], env)
                self.assertEqual(result.stdout.strip(), want, f"{extra} {cls}: {result.stderr}")

    def test_lane_class_jobs(self):
        with tempfile.TemporaryDirectory(prefix="lane-classes-jobs-") as tmp:
            env = lane_env(tmp)
            script = 'LOCKDIR="$1"; . "$2"; lane_class_jobs light 6; lane_class_jobs heavy 6'
            result = run(["-c", script, "_", posix(Path(tmp)), posix(CLASS_SH)], env)
            light, heavy = result.stdout.split()
            self.assertEqual(light, "4")
            self.assertGreaterEqual(int(heavy), 4)


@unittest.skipUnless(BASH, SKIP)
class LaneClassTests(unittest.TestCase):
    def setUp(self):
        self.root = Path(tempfile.mkdtemp(prefix="lane-classes-sh-"))
        self.lock = self.root / "lane"
        self.lock.mkdir()

    def tearDown(self):
        shutil.rmtree(self.root, ignore_errors=True)

    def lane(self, args, timeout=60, **extra):
        return run([posix(LANE_SH), *args], lane_env(self.root, **extra), timeout)

    def test_light_job_runs_while_heavy_slot_held(self):
        # A holder with no pid file reads as busy; slot.light.1 must exist during the job.
        (self.lock / "slot.1").mkdir()
        result = self.lane(["--class", "light", "bash", "-c",
                            'test -d "$LANE_ROOT/lane/slot.light.1"'], LANE_SLOTS="1")
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(jobs(self.root)[-1]["class"], "light")

    def test_heavy_job_runs_while_light_slot_held(self):
        (self.lock / "slot.light.1").mkdir()
        result = self.lane(["bash", "-c", "exit 0"], LANE_SLOTS="1", LANE_LIGHT_SLOTS="1")
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(jobs(self.root)[-1]["class"], "heavy")

    def test_zero_light_count_takes_a_heavy_slot(self):
        result = self.lane(["--class", "light", "bash", "-c",
                            'test -d "$LANE_ROOT/lane/slot.1"'], LANE_SLOTS="1", LANE_LIGHT_SLOTS="0")
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        job = jobs(self.root)[-1]
        self.assertEqual((job["class"], job["class_req"], job["slots"]), ("heavy", "light", 1))

    def test_nested_call_runs_in_parent_slot_and_passes_exit_code(self):
        # The outer job holds the only heavy slot; the inner lane call must not wait for it.
        result = self.lane(["bash", posix(LANE_SH), "bash", "-c", "exit 7"], timeout=60,
                           LANE_SLOTS="1")
        self.assertEqual(result.returncode, 7, result.stdout + result.stderr)
        self.assertIn("not taking a slot", result.stderr)
        self.assertTrue(any(j["nested"] is True and j["slots"] == 0 for j in jobs(self.root)))

    def test_usage_errors_exit_2(self):
        for args in (["--class", "bogus", "bash", "-c", "exit 0"],
                     ["--exclusive", "--class", "light", "bash", "-c", "exit 0"],
                     ["--class"]):
            with self.subTest(args=args):
                self.assertEqual(self.lane(args).returncode, 2)

    def test_job_log_has_class_fields_in_order(self):
        result = self.lane(["--class", "light", "bash", "-c", "exit 0"])
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        line = job_lines(self.root)[-1]
        job = json.loads(line)
        self.assertEqual((job["class"], job["class_req"], job["class_slots_total"]),
                         ("light", "light", 4))
        self.assertEqual((job["busy_class_at_start"], job["nested"]), (0, False))
        self.assertLess(line.index('"slots_total"'), line.index('"class"'))
        self.assertLess(line.index('"busy_class_at_start"'), line.index('"nested"'))
        self.assertLess(line.index('"nested"'), line.index('"busy_at_start"'))

    def test_light_job_sees_cargo_build_jobs_4(self):
        result = self.lane(["--class", "light", "bash", "-c", 'echo "JOBS=$CARGO_BUILD_JOBS"'])
        self.assertIn("JOBS=4", result.stdout)


@unittest.skipUnless(BASH, SKIP)
class TwoPhaseLiveDbTests(unittest.TestCase):
    FILTER = "chain_replay_tests::mission_701"

    def setUp(self):
        self.root = Path(tempfile.mkdtemp(prefix="lane-classes-livedb-"))
        self.calls = self.root / "calls.txt"
        self.tier = self.root / "tier.sh"
        self.reload = self.root / "reload.sh"
        self.reload.write_text(
            "#!/usr/bin/env bash\n"
            f"echo reload >> '{posix(self.calls)}'\n"
            "echo DATABASE_URL=postgres://stub\n", encoding="utf-8")

    def tearDown(self):
        shutil.rmtree(self.root, ignore_errors=True)

    def write_tier(self, build_rc):
        self.tier.write_text(
            "#!/usr/bin/env bash\n"
            f"echo \"tier $* db=${{DATABASE_URL:-}}\" >> '{posix(self.calls)}'\n"
            f"for a in \"$@\"; do [ \"$a\" = --build-only ] && exit {build_rc}; done\n"
            "exit 0\n", encoding="utf-8")

    def run_live_db(self):
        env = lane_env(self.root, LANE_SLOTS="1",
                       LIVE_DB_TIER_SCRIPT=posix(self.tier), LIVE_DB_RELOAD_SCRIPT=posix(self.reload))
        return run([posix(LIVE_DB_SH), self.FILTER], env, timeout=120)

    def read_calls(self):
        return self.calls.read_text(encoding="utf-8").splitlines() if self.calls.exists() else []

    def test_build_then_reload_and_run(self):
        self.write_tier(build_rc=0)
        result = self.run_live_db()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(self.read_calls(), [
            "tier --build-only db=",
            "reload",
            f"tier {self.FILTER} db=postgres://stub"])
        built, ran = jobs(self.root)
        self.assertEqual((built["class"], "--build-only" in built["cmd"]), ("heavy", True))
        self.assertEqual(ran["class"], "db")

    def test_build_failure_stops_before_reload(self):
        self.write_tier(build_rc=3)
        result = self.run_live_db()
        self.assertEqual(result.returncode, 3, result.stdout + result.stderr)
        self.assertEqual(self.read_calls(), ["tier --build-only db="])
        self.assertEqual(len(jobs(self.root)), 1)


if __name__ == "__main__":
    unittest.main()
