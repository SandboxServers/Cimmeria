"""Tests for .claude/hooks/py.sh, the interpreter launcher every Python hook goes through."""

import json
import os
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
PY_SH = REPO / ".claude/hooks/py.sh"
BANNER = REPO / ".claude/hooks/mode_banner.sh"


def posix_shell():
    """A POSIX shell to run py.sh under: Git Bash on Windows, sh elsewhere; None when absent."""
    if os.name == "nt":
        # ProgramFiles can point at the 32-bit folder, so also try the 64-bit one.
        for var in ("ProgramFiles", "ProgramW6432"):
            bash = Path(os.environ.get(var, "")) / "Git" / "bin" / "bash.exe"
            if os.environ.get(var) and bash.is_file():
                return bash
        return None
    return shutil.which("sh")


SHELL = posix_shell()


def run_launcher(args, path, ostype=None):
    """Source py.sh with PATH set to path, passing args as the script and its arguments."""
    prefix = f"OSTYPE={ostype}; " if ostype else ""
    env = {"PATH": path, "PY_SH": PY_SH.as_posix()}
    env.update({k: os.environ[k] for k in ("SYSTEMROOT", "TEMP", "TMP") if k in os.environ})
    return subprocess.run(
        [SHELL, "-c", prefix + '. "$PY_SH"', "sh", *args],
        env=env,
        capture_output=True,
        text=True,
    )


def fake_interpreters(bin_dir, names):
    """Write an executable fake interpreter per name; each prints its own name."""
    for name in names:
        script = Path(bin_dir) / name
        script.write_text(f"#!/bin/sh\necho {name}\n", encoding="utf-8")
        os.chmod(script, 0o755)


@unittest.skipIf(SHELL is None, "no POSIX shell")
class PyLauncherTest(unittest.TestCase):
    def test_execs_python_with_script_and_args(self):
        with tempfile.TemporaryDirectory() as tmp:
            script = Path(tmp) / "echo_args.py"
            script.write_text("import json, sys\nprint(json.dumps(sys.argv[1:]))\n", encoding="utf-8")
            result = run_launcher([str(script), "--harness", "claude"], os.path.dirname(sys.executable))
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(json.loads(result.stdout), ["--harness", "claude"])

    def test_missing_interpreter_warns_and_exits_1(self):
        with tempfile.TemporaryDirectory() as tmp:
            result = run_launcher(["/hooks/guard.py"], tmp)
        self.assertEqual(result.returncode, 1)
        self.assertIn("no python3, python or py", result.stderr)
        self.assertIn("guard.py", result.stderr)

    @unittest.skipIf(os.name == "nt", "fake interpreters are POSIX scripts")
    def test_prefers_python3_off_windows(self):
        with tempfile.TemporaryDirectory() as tmp:
            fake_interpreters(tmp, ["python3", "python"])
            path = tmp + os.pathsep + "/usr/bin:/bin"
            result = run_launcher(["/hooks/guard.py"], path)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout.strip(), "python3")

    def test_prefers_python_on_msys(self):
        with tempfile.TemporaryDirectory() as tmp:
            fake_interpreters(tmp, ["python3", "python"])
            path = tmp + os.pathsep + "/usr/bin:/bin"
            result = run_launcher(["/hooks/guard.py"], path, ostype="msys")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout.strip(), "python")


def run_banner(payload, path):
    """Run mode_banner.sh with payload on stdin and PATH set to path; the banner needs no Python."""
    env = {"PATH": path, "BANNER": BANNER.as_posix()}
    env.update({k: os.environ[k] for k in ("SYSTEMROOT", "TEMP", "TMP") if k in os.environ})
    return subprocess.run(
        [SHELL, "-c", '. "$BANNER"'],
        env=env,
        input=json.dumps(payload),
        capture_output=True,
        text=True,
    )


@unittest.skipIf(SHELL is None, "no POSIX shell")
class ModeBannerTest(unittest.TestCase):
    def test_default_session_names_orchestrator_and_hands_on(self):
        result = run_banner({"session_id": "s", "source": "startup"}, os.path.dirname(sys.executable))
        self.assertEqual(result.returncode, 0, result.stderr)
        output = json.loads(result.stdout)
        self.assertIn("Mode: orchestrator", output["systemMessage"])
        self.assertIn("claude --agent hands-on", output["systemMessage"])
        self.assertIn("additionalContext", output["hookSpecificOutput"])
        self.assertNotIn("WARNING", result.stdout)

    def test_agent_flag_names_that_agent(self):
        result = run_banner({"agent_type": "hands-on"}, os.path.dirname(sys.executable))
        self.assertEqual(result.returncode, 0, result.stderr)
        output = json.loads(result.stdout)
        self.assertTrue(output["systemMessage"].startswith("Mode: hands-on"))
        self.assertNotIn("hookSpecificOutput", output)

    def test_unsafe_agent_type_falls_back_and_stays_valid_json(self):
        result = run_banner({"agent_type": "x;rm -rf"}, os.path.dirname(sys.executable))
        self.assertEqual(result.returncode, 0, result.stderr)
        output = json.loads(result.stdout)
        self.assertIn("Mode: orchestrator", output["systemMessage"])

    def test_warns_when_no_python(self):
        with tempfile.TemporaryDirectory() as tmp:
            result = run_banner({"session_id": "s", "source": "startup"}, tmp)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("WARNING: no python3, python or py", json.loads(result.stdout)["systemMessage"])


if __name__ == "__main__":
    unittest.main()
