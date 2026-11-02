"""Tests for the live guard probe in tools/agent-skills/live_guard_probe.py, offline.

The live legs run against tools/agent-skills/fake_claude.py, never against a real session. Each
live test adds its own throwaway linked worktree of this repo, so the real checkout keeps its
crates/README.md and its reports folder untouched.
"""

import importlib.util
import json
import os
import shutil
import subprocess
import sys
import tempfile
import unittest
from datetime import date
from pathlib import Path
from unittest import mock

AGENT_SKILLS = Path(__file__).resolve().parent
SCRIPT = AGENT_SKILLS / "live_guard_probe.py"
FAKE = AGENT_SKILLS / "fake_claude.py"
REPO = AGENT_SKILLS.parents[1]
DEFAULT_LEGS = ["orchestrator-edit", "hands-on-cargo", "subagent-stop"]


def run_probe(*args: str, env: dict | None = None) -> subprocess.CompletedProcess:
    """Run the probe as a child process and return the completed run."""
    full_env = dict(os.environ) if env is None else env
    return subprocess.run(
        [sys.executable, str(SCRIPT), *args],
        capture_output=True, text=True, encoding="utf-8", errors="replace",
        env=full_env, check=False,
    )


def clean_env(**extra: str) -> dict:
    """Return the parent environment without the Claude Code session keys, plus `extra`."""
    env = {key: value for key, value in os.environ.items() if key not in ("CLAUDECODE", "CLAUDE_CODE_ENTRYPOINT")}
    env.update(extra)
    return env


def add_worktree(test: unittest.TestCase) -> Path:
    """Add a throwaway linked worktree of REPO for one test and remove it again at cleanup."""
    tmp = Path(tempfile.mkdtemp()) / "wt"
    subprocess.run(["git", "-C", str(REPO), "worktree", "add", "--detach", "--no-checkout", str(tmp)],
                   check=True, capture_output=True)
    (tmp / "crates").mkdir()
    (tmp / "crates" / "README.md").write_bytes(b"# crates\n")
    test.addCleanup(shutil.rmtree, tmp.parent, True)
    test.addCleanup(lambda: subprocess.run(["git", "-C", str(REPO), "worktree", "remove", "--force", str(tmp)],
                                           check=False, capture_output=True))
    return tmp


class DryRunTest(unittest.TestCase):
    def test_dry_run_prints_default_legs_and_starts_nothing(self):
        raiser = Path(tempfile.mkdtemp()) / "must_not_run.py"
        raiser.write_text("raise SystemExit('claude must not run')\n", encoding="utf-8")
        done = run_probe("--worktree", str(add_worktree(self)), "--claude", str(raiser), "--codex")
        self.assertEqual(done.returncode, 0, done.stderr)
        for leg in [*DEFAULT_LEGS, "codex"]:
            self.assertIn(f"{leg}:", done.stdout)
        self.assertIn("orchestrator_guard:", done.stdout)
        self.assertIn("Compiling cargo commands go through the build lane", done.stdout)
        self.assertNotIn("status=", done.stdout)

    def test_only_the_orchestrator_leg_disallows_agent_dispatch(self):
        done = run_probe("--worktree", str(add_worktree(self)), "--claude", "claude")
        self.assertEqual(done.returncode, 0, done.stderr)
        lines = done.stdout.splitlines()
        orchestrator = next(line for line in lines if line.startswith("orchestrator-edit:"))
        hands_on = next(line for line in lines if line.startswith("hands-on-cargo:"))
        self.assertIn("--disallowedTools Agent", orchestrator)
        self.assertNotIn("--disallowedTools", hands_on)


class RefusalTest(unittest.TestCase):
    def test_refuses_a_main_checkout_with_git_folder(self):
        with tempfile.TemporaryDirectory() as tmp:
            Path(tmp, ".git").mkdir()
            done = run_probe("--worktree", tmp)
        self.assertEqual(done.returncode, 2)
        self.assertIn("main checkout", done.stderr)

    def test_refuses_a_folder_that_is_not_a_worktree_of_this_repo(self):
        with tempfile.TemporaryDirectory() as tmp:
            Path(tmp, ".git").write_text("gitdir: nowhere\n", encoding="utf-8")
            done = run_probe("--worktree", tmp)
        self.assertEqual(done.returncode, 2)
        self.assertIn("refused", done.stderr)


class LiveTest(unittest.TestCase):
    def setUp(self):
        self.wt = add_worktree(self)
        self.report = self.wt / ".claude" / "reports" / "harness-tuning" / f"live-guard-{date.today().isoformat()}.json"
        self.readme = self.wt / "crates" / "README.md"
        self.projects = tempfile.TemporaryDirectory()
        self.env = clean_env(LIVE_GUARD_PROJECTS_ROOT=self.projects.name)

    def tearDown(self):
        self.projects.cleanup()

    def live(self, legs: str = ",".join(DEFAULT_LEGS), **extra: str) -> tuple[subprocess.CompletedProcess, dict]:
        """Run the probe live against the fake and return the run and the JSON report."""
        env = dict(self.env, **extra)
        done = run_probe("--worktree", str(self.wt), "--claude", str(FAKE), "--legs", legs, "--live", env=env)
        report = json.loads(self.report.read_text(encoding="utf-8")) if self.report.exists() else {}
        return done, report

    def test_fake_passes_all_three_legs_and_writes_the_report(self):
        done, report = self.live()
        self.assertEqual(done.returncode, 0, done.stderr)
        self.assertIn("status=ok passed=3/3", done.stdout)
        self.assertEqual(report["claude_version"], "0.0.0 (fake)")
        self.assertEqual(report["date"], date.today().isoformat())
        for leg in DEFAULT_LEGS:
            result = report["legs"][leg]
            self.assertEqual(set(result), {"ran", "pass", "evidence"})
            self.assertTrue(result["ran"])
            self.assertTrue(result["pass"], result)
            self.assertNotIn(str(self.wt), result["evidence"])
            self.assertNotIn("\\", result["evidence"])

    def test_a_fake_without_the_marker_fails_that_leg_with_exit_zero(self):
        done, report = self.live(FAKE_CLAUDE_MODE="omit-marker")
        self.assertEqual(done.returncode, 0)
        self.assertFalse(report["legs"]["orchestrator-edit"]["pass"])
        self.assertFalse(report["legs"]["hands-on-cargo"]["pass"])

    def test_orchestrator_leg_fails_when_the_fake_changes_the_readme(self):
        done, report = self.live("orchestrator-edit", FAKE_CLAUDE_MODE="change-readme")
        self.assertEqual(done.returncode, 0)
        result = report["legs"]["orchestrator-edit"]
        self.assertFalse(result["pass"])
        self.assertEqual(result["evidence"], "crates/README.md changed")

    def test_orchestrator_leg_without_an_edit_attempt_says_so(self):
        done, report = self.live("orchestrator-edit", FAKE_CLAUDE_MODE="no-attempt")
        self.assertEqual(done.returncode, 0)
        result = report["legs"]["orchestrator-edit"]
        self.assertFalse(result["pass"])
        self.assertEqual(result["evidence"], "no edit attempted")

    def test_hands_on_leg_with_only_the_lane_wrapper_says_no_raw_cargo_attempt(self):
        done, report = self.live("hands-on-cargo", FAKE_CLAUDE_MODE="no-attempt")
        self.assertEqual(done.returncode, 0)
        result = report["legs"]["hands-on-cargo"]
        self.assertFalse(result["pass"])
        self.assertEqual(result["evidence"], "no raw cargo attempted")

    def test_a_tool_use_without_the_marker_is_reported_as_guard_silent(self):
        done, report = self.live("orchestrator-edit,hands-on-cargo", FAKE_CLAUDE_MODE="omit-marker")
        self.assertEqual(done.returncode, 0)
        self.assertTrue(report["legs"]["orchestrator-edit"]["evidence"].endswith("guard silent"))
        self.assertTrue(report["legs"]["hands-on-cargo"]["evidence"].endswith("guard silent"))

    def test_child_env_lacks_the_claude_code_session_keys(self):
        echo = Path(self.projects.name, "env-keys.json")
        env = dict(self.env, CLAUDECODE="1", CLAUDE_CODE_ENTRYPOINT="cli", FAKE_CLAUDE_ECHO_ENV=str(echo))
        done = run_probe("--worktree", str(self.wt), "--claude", str(FAKE), "--legs", "orchestrator-edit",
                         "--live", env=env)
        self.assertEqual(done.returncode, 0, done.stderr)
        keys = json.loads(echo.read_text(encoding="utf-8"))
        self.assertNotIn("CLAUDECODE", keys)
        self.assertNotIn("CLAUDE_CODE_ENTRYPOINT", keys)
        self.assertIn("CLAUDE_PROJECT_DIR", keys)


class SlugTest(unittest.TestCase):
    def test_slug_turns_every_non_alphanumeric_character_into_a_dash(self):
        spec = importlib.util.spec_from_file_location("live_guard_probe", SCRIPT)
        probe = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(probe)
        self.assertEqual(probe.slug(Path("C:/Users/Steve/source/projects/Cimmeria")),
                         "C--Users-Steve-source-projects-Cimmeria")


if __name__ == "__main__":
    unittest.main()
