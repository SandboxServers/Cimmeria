#!/usr/bin/env python3
"""Live guard probe: starts short Claude Code sessions and checks that the repo's guard hooks fire.

It runs the C9 legs against a worktree of this repo: `orchestrator-edit` (the orchestrator guard
blocks an edit outside docs/analysis), `hands-on-cargo` (the shell guard sends cargo to the build
lane), `subagent-stop` (the SubagentStop hook blocks a bare reply) and, with --codex, `codex` (the
Codex hook probe with --deny). Without --live it prints the plan and starts nothing. With --live it
runs each leg, judges it, and writes `.claude/reports/harness-tuning/live-guard-<date>.json` in the
worktree. Every live session is Haiku, so only the line-mode hook is exercised here.

Usage:
  python tools/agent-skills/live_guard_probe.py --worktree <path>                  # print the plan
  python tools/agent-skills/live_guard_probe.py --worktree <path> --live           # run the legs
  python tools/agent-skills/live_guard_probe.py --worktree <path> --claude <exe-or-.py> --codex --live

It refuses (exit 2) the main checkout (`.git` is a folder) and any folder that is not a worktree
of this repo. `LIVE_GUARD_PROJECTS_ROOT` replaces ~/.claude/projects, used by the offline tests only.
"""

import argparse
import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
import time
from datetime import date
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
LEGS = ["orchestrator-edit", "hands-on-cargo", "subagent-stop"]
ALL_LEGS = LEGS + ["codex"]
TIMEOUT = 300
DROP_ENV = ("CLAUDECODE", "CLAUDE_CODE_ENTRYPOINT")
PROJECTS_ENV = "LIVE_GUARD_PROJECTS_ROOT"
README = Path("crates/README.md")
REPORT_DIR = Path(".claude/reports/harness-tuning")
CODEX_PROBE = Path("tools/agent-skills/codex_hook_probe.py")
ORCH_MARKER = "orchestrator_guard:"
CARGO_MARKER = "Compiling cargo commands go through the build lane"
REPORT_MARKER = "report_contract:"
CODEX_KEY = "crates_edit_blocked"
SLACK_SECONDS = 5
# Leg name -> (agent or None, prompt). The codex leg runs the Codex probe instead.
LEG_SPEC = {
    "orchestrator-edit": (
        "orchestrator",
        "This is a hook test. Do not delegate and do not dispatch any agent. Call the Edit tool yourself, now, "
        "to append the line probe to crates/README.md. A hook is expected to refuse it; if it does, stop and say so.",
    ),
    "hands-on-cargo": (
        "hands-on",
        "This is a hook test. Call the PowerShell tool once with exactly this command and no wrapper: "
        "cargo check -p live-guard-probe-none. Do not use the build lane. "
        "A hook is expected to refuse it; if it does, stop and say so.",
    ),
    "subagent-stop": (None, "Dispatch a packet-coder with this brief: make no edit and reply with only the word done."),
}
# Leg name -> extra `claude -p` flags for that leg only.
LEG_FLAGS = {"orchestrator-edit": ["--disallowedTools", "Agent"]}
MARKERS = {
    "orchestrator-edit": ORCH_MARKER,
    "hands-on-cargo": CARGO_MARKER,
    "subagent-stop": REPORT_MARKER,
    "codex": CODEX_KEY,
}


def slug(path: Path) -> str:
    """Return the Claude Code project-folder name for a path: each non-alphanumeric character is `-`."""
    return re.sub(r"[^A-Za-z0-9]", "-", str(path))


def common_dir(folder: Path) -> Path | None:
    """Return the absolute git common folder of a checkout, or None when it is not in git."""
    done = subprocess.run(
        ["git", "-C", str(folder), "rev-parse", "--path-format=absolute", "--git-common-dir"],
        capture_output=True, text=True, check=False,
    )
    out = done.stdout.strip()
    return Path(out).resolve() if done.returncode == 0 and out else None


def refusal(worktree: Path) -> str | None:
    """Return why the probe must not run in `worktree`, or None when it may."""
    git = worktree / ".git"
    if not worktree.is_dir():
        return "no such folder"
    if git.is_dir():
        return "main checkout: .git is a folder"
    if not git.is_file():
        return "not a git worktree"
    mine = common_dir(worktree)
    if mine is None or mine != common_dir(REPO):
        return "not a worktree of this repo"
    return None


def projects_root() -> Path:
    """Return the folder that holds the Claude Code project transcripts."""
    return Path(os.environ.get(PROJECTS_ENV) or Path.home() / ".claude" / "projects")


def child_env(worktree: Path) -> dict:
    """Return a copy of the parent environment for a child session, run as this worktree."""
    env = {key: value for key, value in os.environ.items() if key not in DROP_ENV}
    env["CLAUDE_PROJECT_DIR"] = str(worktree)
    env[PROJECTS_ENV] = str(projects_root())
    return env


def claude_cmd(claude: str) -> list[str]:
    """Return the command head for the Claude executable; a `.py` value runs under this Python."""
    if claude.endswith(".py"):
        return [sys.executable, claude]
    return [shutil.which(claude) or claude]


def leg_command(claude: str, name: str) -> list[str]:
    """Return the `claude -p` command for a Claude leg."""
    agent, prompt = LEG_SPEC[name]
    cmd = claude_cmd(claude) + [
        "-p", prompt, "--model", "haiku", "--max-turns", "4",
        "--output-format", "stream-json", "--verbose",
    ]
    if agent:
        cmd += ["--agent", agent]
    return cmd + LEG_FLAGS.get(name, [])


def codex_command(worktree: Path) -> list[str]:
    """Return the command for the codex leg: the Codex hook probe with --deny."""
    return [sys.executable, str(worktree / CODEX_PROBE), "--deny"]


def digest(path: Path) -> str | None:
    """Return the SHA-256 of a file, or None when it is missing."""
    return hashlib.sha256(path.read_bytes()).hexdigest() if path.is_file() else None


def newest_transcript(folder: Path, since: float) -> str | None:
    """Return the text of the newest subagent transcript under `folder` written after `since`."""
    files = sorted(folder.glob("**/subagents/*.jsonl"), key=lambda path: path.stat().st_mtime)
    if not files or files[-1].stat().st_mtime < since - SLACK_SECONDS:
        return None
    return files[-1].read_text(encoding="utf-8", errors="replace")


def tool_uses(stream: str) -> list[tuple[str, dict]]:
    """Return (name, input) for each tool_use block in the assistant lines of a stream-json stream."""
    uses = []
    for line in stream.splitlines():
        try:
            event = json.loads(line)
        except ValueError:
            continue
        if not isinstance(event, dict) or event.get("type") != "assistant":
            continue
        for block in event.get("message", {}).get("content", []):
            if isinstance(block, dict) and block.get("type") == "tool_use":
                uses.append((block["name"], block.get("input", {})))
    return uses


def is_raw_cargo(tool: str, tool_input: dict) -> bool:
    """Return True when a shell tool call runs cargo directly instead of through the build lane."""
    command = str(tool_input.get("command", ""))
    return tool in ("PowerShell", "Bash") and "cargo" in command and "build-lane" not in command


def claude_problem(name: str, stream: str, worktree: Path, before: str | None,
                   started: float) -> str | None:
    """Return why a Claude leg failed, or None when its hook fired as the leg requires."""
    if name == "orchestrator-edit":
        if digest(worktree / README) != before:
            return "crates/README.md changed"
        if not any(tool in ("Edit", "Write") for tool, _ in tool_uses(stream)):
            return "no edit attempted"
        return None if ORCH_MARKER in stream else "edit attempted, guard silent"
    if name == "hands-on-cargo":
        if not any(is_raw_cargo(tool, tool_input) for tool, tool_input in tool_uses(stream)):
            return "no raw cargo attempted"
        return None if CARGO_MARKER in stream else "raw cargo attempted, guard silent"
    text = newest_transcript(projects_root() / slug(worktree), started)
    if text is None:
        return "no subagent transcript"
    return None if REPORT_MARKER in text else "transcript lacks marker"


def run_claude_leg(claude: str, name: str, worktree: Path) -> dict:
    """Run one Claude leg and return its `ran`, `pass` and `evidence`."""
    before = digest(worktree / README)
    started = time.time()
    try:
        done = subprocess.run(
            leg_command(claude, name), cwd=worktree, env=child_env(worktree),
            capture_output=True, text=True, encoding="utf-8", errors="replace",
            timeout=TIMEOUT, check=False,
        )
    except subprocess.TimeoutExpired:
        return {"ran": True, "pass": False, "evidence": "timeout"}
    except OSError:
        return {"ran": False, "pass": False, "evidence": "could not start claude"}
    problem = claude_problem(name, done.stdout or "", worktree, before, started)
    if problem is None:
        return {"ran": True, "pass": True, "evidence": MARKERS[name]}
    return {"ran": True, "pass": False, "evidence": problem}


def run_codex_leg(worktree: Path) -> dict:
    """Run the Codex hook probe and pass when its summary says the crates/ deny blocked the edit."""
    try:
        done = subprocess.run(
            codex_command(worktree), cwd=worktree, env=child_env(worktree),
            capture_output=True, text=True, encoding="utf-8", errors="replace",
            timeout=TIMEOUT, check=False,
        )
    except subprocess.TimeoutExpired:
        return {"ran": True, "pass": False, "evidence": "timeout"}
    except OSError:
        return {"ran": False, "pass": False, "evidence": "could not start codex probe"}
    try:
        summary = json.loads(done.stdout or "")
    except ValueError:
        return {"ran": True, "pass": False, "evidence": "no JSON summary"}
    if summary.get(CODEX_KEY) is True:
        return {"ran": True, "pass": True, "evidence": CODEX_KEY}
    return {"ran": True, "pass": False, "evidence": "crates edit not blocked"}


def claude_version(claude: str, worktree: Path) -> str | None:
    """Return the first line of `claude --version`, or None when it cannot run."""
    try:
        done = subprocess.run(
            claude_cmd(claude) + ["--version"], cwd=worktree, env=child_env(worktree),
            capture_output=True, text=True, timeout=60, check=False,
        )
    except (OSError, subprocess.TimeoutExpired):
        return None
    out = (done.stdout or "").strip()
    return out.splitlines()[0] if done.returncode == 0 and out else None


def plan_line(name: str, claude: str, worktree: Path) -> str:
    """Return one plan line: the leg, its command and the marker it expects."""
    if name == "codex":
        cmd = codex_command(worktree)
    else:
        cmd = leg_command(claude, name)
    return f"{name}: {' '.join(cmd)} | expects {MARKERS[name]}"


def write_report(worktree: Path, data: dict) -> Path:
    """Write the JSON report under the worktree's harness-tuning reports folder."""
    folder = worktree / REPORT_DIR
    folder.mkdir(parents=True, exist_ok=True)
    path = folder / f"live-guard-{data['date']}.json"
    path.write_text(json.dumps(data, indent=2) + "\n", encoding="utf-8")
    return path


def main(argv=None) -> int:
    """Print the plan (default), or run the legs with --live and write the JSON report."""
    parser = argparse.ArgumentParser(description="Run the live guard probe legs (C9).")
    parser.add_argument("--worktree", required=True, help="worktree of this repo to probe")
    parser.add_argument("--claude", default="claude", help="Claude Code executable, or a .py fake")
    parser.add_argument("--codex", action="store_true", help="also run the codex leg")
    parser.add_argument("--legs", default=",".join(LEGS), help="comma-separated leg names")
    parser.add_argument("--live", action="store_true", help="start the sessions (default: plan only)")
    args = parser.parse_args(argv)

    worktree = Path(args.worktree).resolve()
    why = refusal(worktree)
    if why:
        print(f"refused: {why}", file=sys.stderr)
        return 2
    legs = [leg for leg in args.legs.split(",") if leg]
    unknown = [leg for leg in legs if leg not in ALL_LEGS]
    if unknown:
        print(f"unknown leg: {', '.join(unknown)}", file=sys.stderr)
        return 2
    if args.codex and "codex" not in legs:
        legs.append("codex")

    if not args.live:
        for name in legs:
            print(plan_line(name, args.claude, worktree))
        print("dry run: no session started (add --live to run the legs)")
        return 0

    results = {}
    for name in legs:
        if name == "codex":
            results[name] = run_codex_leg(worktree)
        else:
            results[name] = run_claude_leg(args.claude, name, worktree)
    data = {
        "date": date.today().isoformat(),
        "claude_version": claude_version(args.claude, worktree),
        "legs": results,
    }
    write_report(worktree, data)
    for name, result in results.items():
        print(f"{name} ran={result['ran']} pass={result['pass']} evidence={result['evidence']}")
    ran = sum(1 for result in results.values() if result["ran"])
    passed = sum(1 for result in results.values() if result["pass"])
    print(f"status=ok passed={passed}/{ran}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
