#!/usr/bin/env python3
"""Fake `claude` for the live guard probe tests: a test fixture that never starts a session.

It reads its argv the way `claude -p --output-format stream-json` does and prints one stream-json
line per leg. The leg is picked by `--agent` and by a keyword in the prompt: `crates/README.md`
(orchestrator-edit), `cargo check` (hands-on-cargo) or `packet-coder` (subagent-stop). The
subagent leg writes a transcript holding `report_contract:` under
LIVE_GUARD_PROJECTS_ROOT/<project folder>/session/subagents/, where the project folder is the
CLAUDE_PROJECT_DIR path with every non-alphanumeric character turned into `-`.

The two shell-and-edit legs first print a stream-json tool_use line (the Edit or the PowerShell call)
and then the marker line.

Environment knobs, set by the tests:
  FAKE_CLAUDE_MODE=omit-marker     keep the tool_use line, print no guard marker
  FAKE_CLAUDE_MODE=no-attempt      print no Edit tool_use, and for hands-on a lane-wrapper call only
  FAKE_CLAUDE_MODE=change-readme   append `probe` to crates/README.md under the cwd
  FAKE_CLAUDE_ECHO_ENV=<file>      write the env keys it got, as a JSON list, to that file

`--version` prints `0.0.0 (fake)`.
"""

import argparse
import json
import os
import re
import sys
from pathlib import Path

ORCH_TEXT = "orchestrator_guard: the orchestrator writes only docs/analysis/**, .claude/state/**"
CARGO_TEXT = "Compiling cargo commands go through the build lane: run it with lab.ps1 cargo."
REPORT_TEXT = "report_contract: end your reply with exactly one line: <id> <done|blocked|failed>"


def emit(text: str) -> None:
    """Print one stream-json assistant line holding `text`."""
    print(json.dumps({"type": "assistant", "message": {"content": [{"type": "text", "text": text}]}}))


def tool_use(name: str, tool_input: dict) -> None:
    """Print one stream-json assistant line holding a tool_use block named `name`."""
    block = {"type": "tool_use", "name": name, "input": tool_input}
    print(json.dumps({"type": "assistant", "message": {"content": [block]}}))


def write_transcript() -> None:
    """Write a fake subagent transcript that holds the report_contract marker."""
    root = Path(os.environ["LIVE_GUARD_PROJECTS_ROOT"])
    project = re.sub(r"[^A-Za-z0-9]", "-", os.environ["CLAUDE_PROJECT_DIR"])
    folder = root / project / "session" / "subagents"
    folder.mkdir(parents=True, exist_ok=True)
    line = {"type": "user", "message": {"content": REPORT_TEXT}}
    (folder / "agent-probe.jsonl").write_text(json.dumps(line) + "\n", encoding="utf-8")


def main(argv=None) -> int:
    """Act as the fake session for one leg, or print the version."""
    parser = argparse.ArgumentParser(description="Fake claude for the live guard probe tests.")
    parser.add_argument("-p", dest="prompt", default="")
    parser.add_argument("--agent", default=None)
    parser.add_argument("--model", default=None)
    parser.add_argument("--max-turns", default=None)
    parser.add_argument("--output-format", default=None)
    parser.add_argument("--verbose", action="store_true")
    parser.add_argument("--version", action="store_true")
    parser.add_argument("--disallowedTools", default=None)
    args, _unknown = parser.parse_known_args(argv)

    if args.version:
        print("0.0.0 (fake)")
        return 0
    echo = os.environ.get("FAKE_CLAUDE_ECHO_ENV")
    if echo:
        Path(echo).write_text(json.dumps(sorted(os.environ)), encoding="utf-8")

    mode = os.environ.get("FAKE_CLAUDE_MODE", "")
    silent = mode in ("omit-marker", "no-attempt")
    if "crates/README.md" in args.prompt:
        if mode != "no-attempt":
            tool_use("Edit", {"file_path": "crates/README.md", "old_string": "", "new_string": "probe"})
        if mode == "change-readme":
            readme = Path.cwd() / "crates" / "README.md"
            with readme.open("a", encoding="utf-8", newline="\n") as handle:
                handle.write("probe\n")
        emit("" if silent else ORCH_TEXT)
    elif "cargo check" in args.prompt:
        if mode == "no-attempt":
            tool_use("PowerShell", {"command": "pwsh tools/build-lane/lane.ps1 cargo check -p x"})
        else:
            tool_use("PowerShell", {"command": "cargo check -p live-guard-probe-none"})
        emit("" if silent else CARGO_TEXT)
    elif "packet-coder" in args.prompt:
        write_transcript()
        emit("done")
    else:
        emit("unknown leg")
    return 0


if __name__ == "__main__":
    sys.exit(main())
