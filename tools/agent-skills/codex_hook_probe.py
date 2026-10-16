#!/usr/bin/env python3
"""Probe which .codex/hooks.json hooks this Codex build runs (SessionStart, context injection, PreToolUse, resume).

The probe adds two hooks to .codex/hooks.json, one SessionStart and one PreToolUse,
that append a line to .claude/state/codex-hook-probe.jsonl whenever Codex runs them.
It then runs two `codex exec` calls: a fresh session that asks for a token from its
context and makes an edit, then a `resume --last` session. The original hooks.json
bytes are restored in every case, and the printed summary says which hooks ran.

Usage:
  python tools/agent-skills/codex_hook_probe.py                 # run the probe
  python tools/agent-skills/codex_hook_probe.py --timeout 180   # per-call timeout (s)
  python tools/agent-skills/codex_hook_probe.py --codex <exe>   # Codex executable

The hook mode is `--record <event>`; the probe's own hook commands use it.
"""

import argparse
import copy
import json
import os
import shutil
import subprocess
import sys
from pathlib import Path

TOKEN = "CODEX-HOOK-PROBE-7731"
HOOKS = Path(".codex/hooks.json")
LOG = Path(".claude/state/codex-hook-probe.jsonl")
EDIT = Path(".claude/state/codex-probe-edit.txt")
SHELL_TOOLS = {"Bash", "shell", "local_shell"}
PROMPT_1 = (
    "If your context contains a token that starts with CODEX-HOOK-PROBE, reply with "
    "that token only, otherwise reply NONE. Then create the file "
    ".claude/state/codex-probe-edit.txt containing the word probe."
)
PROMPT_2 = "Reply DONE."


def repo_root() -> Path:
    """Return the git top-level folder, or the current folder outside git."""
    done = subprocess.run(
        ["git", "rev-parse", "--show-toplevel"], capture_output=True, text=True, check=False
    )
    top = done.stdout.strip()
    return Path(top) if done.returncode == 0 and top else Path.cwd()


def default_codex() -> str:
    """Return the newest Codex desktop-app binary on Windows, else the bare name `codex`.

    The npm `codex` shim on PATH can be an older CLI that rejects newer models
    (for example gpt-6-sol with a ChatGPT login). The desktop app's bundled CLI
    reads the same ~/.codex config and login, so the probe prefers it."""
    local = os.environ.get("LOCALAPPDATA")
    if local:
        bundled = Path(local, "OpenAI", "Codex", "bin").glob("*/codex.exe")
        found = sorted(bundled, key=lambda path: path.stat().st_mtime)
        if found:
            return str(found[-1])
    return "codex"


def probe_hooks(original: dict, script: Path) -> dict:
    """Deep copy of `original` plus one SessionStart and one PreToolUse entry,
    both with no matcher, each running `python "<script>" --record <event>`."""
    hooks = copy.deepcopy(original)
    entries = hooks.setdefault("hooks", {})
    for event, name in (("SessionStart", "session-start"), ("PreToolUse", "pre-tool-use")):
        command = f'python "{script}" --record {name}'
        entry = {"hooks": [{"type": "command", "command": command, "timeout": 30}]}
        entries.setdefault(event, []).append(entry)
    return hooks


def record(event: str, stdin_text: str, root: Path) -> str | None:
    """Append {"event", "source", "tool_name", "keys"} as one JSON line to LOG
    ("keys" is the sorted list of tool_input keys). For session-start, return the
    SessionStart hook JSON with additionalContext = TOKEN; otherwise None."""
    try:
        payload = json.loads(stdin_text or "{}")
    except json.JSONDecodeError:
        payload = {}
    if not isinstance(payload, dict):
        payload = {}
    tool_input = payload.get("tool_input")
    keys = sorted(tool_input) if isinstance(tool_input, dict) else []
    row = {
        "event": event,
        "source": payload.get("source"),
        "tool_name": payload.get("tool_name"),
        "keys": keys,
    }
    log = root / LOG
    log.parent.mkdir(parents=True, exist_ok=True)
    with log.open("a", encoding="utf-8") as handle:
        handle.write(json.dumps(row) + "\n")
    if event == "session-start":
        output = {"hookSpecificOutput": {"hookEventName": "SessionStart", "additionalContext": TOKEN}}
        return json.dumps(output)
    return None


def read_rows(root: Path) -> list[dict]:
    """Return every parsable line of LOG, in order."""
    path = root / LOG
    if not path.exists():
        return []
    rows = []
    for line in path.read_text(encoding="utf-8").splitlines():
        try:
            rows.append(json.loads(line))
        except json.JSONDecodeError:
            continue
    return rows


def distinct(values: list) -> list:
    """Return the non-null values once each, in first-seen order."""
    seen = []
    for value in values:
        if value is not None and value not in seen:
            seen.append(value)
    return seen


def codex_call(args: list[str], root: Path, timeout: int, label: str, notes: list[str]):
    """Run one codex command; on a timeout or a launch error note it and return None."""
    try:
        return subprocess.run(
            args, capture_output=True, text=True, encoding="utf-8", errors="replace",
            timeout=timeout, cwd=root,
        )
    except subprocess.TimeoutExpired:
        notes.append(f"{label}: timed out after {timeout}s")
    except OSError as exc:
        notes.append(f"{label}: could not run ({exc})")
    return None


def run_probe(root: Path, codex: str, timeout: int) -> dict:
    """Back up .codex/hooks.json bytes, write probe_hooks(), clear LOG, run the two
    codex calls below, and ALWAYS restore the original bytes in a finally block.
    Also delete .claude/state/codex-probe-edit.txt in the finally. Return the summary."""
    hooks_path = root / HOOKS
    original = hooks_path.read_bytes()
    notes: list[str] = []
    first = second = None
    try:
        script = Path(__file__).resolve()
        hooks = probe_hooks(json.loads(original.decode("utf-8")), script)
        hooks_path.write_text(json.dumps(hooks, indent=2) + "\n", encoding="utf-8")
        (root / LOG).parent.mkdir(parents=True, exist_ok=True)
        (root / LOG).write_text("", encoding="utf-8")

        version = codex_call([codex, "--version"], root, timeout, "codex --version", notes)
        codex_version = version.stdout.strip() if version and version.returncode == 0 else None

        first = codex_call(
            [codex, "exec", "--sandbox", "workspace-write", PROMPT_1],
            root, timeout, "call 1 (exec)", notes,
        )
        if first is not None and first.returncode != 0:
            notes.append(f"call 1 (exec): exit {first.returncode}")
        rows_1 = read_rows(root)

        second = codex_call(
            [codex, "exec", "resume", "--last", PROMPT_2],
            root, timeout, "call 2 (resume)", notes,
        )
        if second is None or second.returncode != 0:
            notes.append("call 2 (resume): unsupported or failed; resume_ran cannot be shown")
        rows_2 = read_rows(root)
    finally:
        hooks_path.write_bytes(original)
        (root / EDIT).unlink(missing_ok=True)

    new_rows = rows_2[len(rows_1):]
    starts_1 = [r for r in rows_1 if r.get("event") == "session-start"]
    starts_2 = [r for r in new_rows if r.get("event") == "session-start"]
    pre_rows = [r for r in rows_1 + new_rows if r.get("event") == "pre-tool-use"]
    tools = distinct([r.get("tool_name") for r in pre_rows])
    return {
        "codex_version": codex_version,
        "session_start_ran": bool(starts_1),
        "session_start_sources": distinct([r.get("source") for r in starts_1]),
        "context_injected": first is not None and TOKEN in first.stdout,
        "pre_tool_use_tools": tools,
        "edit_tool_hooked": any(
            r.get("tool_name") is not None and r["tool_name"] not in SHELL_TOOLS
            for r in pre_rows
        ),
        "resume_ran": bool(starts_2),
        "resume_source": starts_2[0].get("source") if starts_2 else None,
        "notes": notes,
    }


def main(argv=None) -> int:
    """Run the probe (default), or act as the hook when given --record <event>."""
    parser = argparse.ArgumentParser(description="Probe which Codex hooks run.")
    parser.add_argument("--record", metavar="EVENT", help="hook mode: record one event from stdin")
    parser.add_argument(
        "--codex", default=None,
        help="Codex executable (default: the desktop app's bundled CLI, else codex on PATH)",
    )
    parser.add_argument("--timeout", type=int, default=180, help="per-call timeout in seconds")
    args = parser.parse_args(argv)

    root = repo_root()
    if args.record:
        try:
            output = record(args.record, sys.stdin.read(), root)
        except OSError:
            return 0
        if output:
            print(output)
        return 0

    requested = args.codex or default_codex()
    codex = shutil.which(requested)
    if codex is None:
        print(f"codex executable not found: {requested}", file=sys.stderr)
        return 3
    print(json.dumps(run_probe(root, codex, args.timeout), indent=2))
    return 0


if __name__ == "__main__":
    sys.exit(main())
