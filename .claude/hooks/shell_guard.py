#!/usr/bin/env python3
"""PreToolUse guard for shell tool calls (Bash and PowerShell).

Two rules, each aimed at a habit that Claude Code telemetry showed in one week
(2026-10-03..10, see .claude/skills/telemetry-triage):

1. Compiling cargo commands go through the build lane (326 direct calls).
   `cargo check|build|test|nextest|clippy|run|bench|doc` (and the b/c/t/r
   aliases) is refused unless that stage runs it through
   tools/build-lane/lane.ps1, lane.sh or live-db-test.*.
2. (Claude only) Plain file reads and searches use the Read, Grep and Glob tools
   (about 10,000 grep/sed -n/cat/Get-Content calls). A command is refused only
   when every pipeline stage is a read or a text filter, so `git log | grep x`,
   anything that writes a file, and follows such as `tail -f` still run.

End a command with the comment `# guard-ok` to skip both rules on purpose.

The command is split into stages on unquoted `|`, `;`, `&&`, `||` and newlines,
and each stage is tokenized with its quotes respected, so text inside a commit
message or a search pattern never counts as a command. `bash -c`, `sh -c`,
`pwsh -c` / `-Command` and `cmd /c` bodies are checked recursively.

Hook protocol: the tool call arrives as JSON on stdin; exit code 2 refuses it and
the stderr text goes back to the model. Any parse failure allows the call.

Usage: shell_guard.py --harness claude|codex
Codex has no Read/Grep tools, so it gets rule 1 only.
"""

import json
import re
import sys

BYPASS = "# guard-ok"

COMPILE_SUBCOMMANDS = {
    "check", "c", "build", "b", "test", "t", "run", "r", "nextest", "clippy",
    "bench", "doc", "rustc", "llvm-cov", "install",
}
# cargo global options that take a value.
CARGO_VALUE_OPTS = {"--manifest-path", "--config", "-Z", "--color", "-C", "--target-dir"}
LANE_SCRIPT = re.compile(r"build-lane[/\\](?:lane|live-db-test)\.(?:sh|ps1)$")
SHELLS = {"bash", "sh", "bash.exe", "sh.exe", "pwsh", "pwsh.exe", "powershell", "powershell.exe", "cmd", "cmd.exe"}
INNER_FLAGS = {"-c", "-command", "/c", "/k"}

READERS = {
    "grep", "egrep", "fgrep", "rg", "cat", "head", "tail", "less", "more",
    "get-content", "gc", "select-string", "sls",
}
FILTERS = {
    "head", "tail", "wc", "sort", "uniq", "cut", "tr", "grep", "egrep", "rg",
    "select-object", "select", "measure-object", "measure", "sort-object",
    "format-table", "ft", "out-string", "select-string", "sls",
}
# Reads the Read tool cannot do: following a growing file, or raw bytes.
FOLLOW_OR_BYTES = {"-f", "-F", "--follow", "-wait", "-c", "-asbytestream"}


def split_stages(command):
    """Split on unquoted | ; && || and newlines. Returns (stages, has_redirect)."""
    stages, current, quote, redirect = [], [], None, False
    i, n = 0, len(command)
    while i < n:
        ch = command[i]
        if quote:
            current.append(ch)
            if ch == quote:
                quote = None
            elif ch == "\\" and quote == '"' and i + 1 < n:
                current.append(command[i + 1])
                i += 1
        elif ch in "'\"":
            quote = ch
            current.append(ch)
        elif ch == "#" and (i == 0 or command[i - 1].isspace()):
            while i < n and command[i] != "\n":
                i += 1
            continue
        elif ch in "|;&\n":
            two = command[i:i + 2]
            if two in ("&&", "||"):
                i += 1
            elif ch == "&" and (command[i - 1:i] == ">" or not "".join(current).strip()):
                # `2>&1` and PowerShell's `& script` are not separators.
                current.append(ch)
                i += 1
                continue
            stages.append("".join(current))
            current = []
        else:
            if ch in "<>" or command.startswith("$(", i) or ch == "`":
                redirect = True
            current.append(ch)
        i += 1
    stages.append("".join(current))
    return [s for s in stages if s.strip()], redirect


def tokenize(stage):
    """Whitespace split that keeps quoted strings whole and strips their quotes."""
    tokens, current, quote, started = [], [], None, False
    for ch in stage:
        if quote:
            if ch == quote:
                quote = None
            else:
                current.append(ch)
        elif ch in "'\"":
            quote, started = ch, True
        elif ch.isspace():
            if started or current:
                tokens.append("".join(current))
            current, started = [], False
        else:
            current.append(ch)
    if started or current:
        tokens.append("".join(current))
    return tokens


def strip_prefix(tokens):
    """Drop VAR=value assignments and the PowerShell call operator."""
    while tokens and (re.match(r"^[A-Za-z_][A-Za-z0-9_]*=", tokens[0]) or tokens[0] == "&"):
        tokens = tokens[1:]
    return tokens


def inner_command(tokens):
    """The body of `bash -c "..."`, `pwsh -Command ...` or `cmd /c ...`, if any."""
    if not tokens or tokens[0].lower().split("/")[-1].split("\\")[-1] not in SHELLS:
        return None
    for index, token in enumerate(tokens[1:], start=1):
        if token.lower() in INNER_FLAGS and index + 1 < len(tokens):
            return " ".join(tokens[index + 1:])
    return None


def is_lane_stage(tokens):
    return any(LANE_SCRIPT.search(token) for token in tokens[:4])


def direct_cargo_compile(command, depth=0):
    stages, _ = split_stages(command)
    for stage in stages:
        tokens = strip_prefix(tokenize(stage))
        if not tokens:
            continue
        inner = inner_command(tokens)
        if inner is not None:
            if depth < 3 and direct_cargo_compile(inner, depth + 1):
                return True
            continue
        if is_lane_stage(tokens):
            continue
        head = tokens[0].lower().replace("\\", "/").split("/")[-1]
        if head not in ("cargo", "cargo.exe"):
            continue
        index = 1
        while index < len(tokens):
            token = tokens[index]
            if token.startswith("+"):
                index += 1
            elif token in CARGO_VALUE_OPTS:
                index += 2
            elif token.startswith("-"):
                index += 1
            else:
                break
        if index < len(tokens) and tokens[index] in COMPILE_SUBCOMMANDS:
            return True
    return False


def is_plain_read(command):
    command = re.sub(r"\d?>\s*(?:/dev/null|\$null|&\d)", "", command)
    stages, redirect = split_stages(command)
    if redirect or not stages:
        return False
    while len(stages) > 1 and re.match(r"^\s*(?:cd|set-location|sl|pushd)\s", stages[0], re.I):
        stages = stages[1:]
    for index, stage in enumerate(stages):
        tokens = strip_prefix(tokenize(stage))
        if not tokens:
            return False
        word = tokens[0].lower()
        flags = {t.lower() for t in tokens[1:] if t.startswith("-")}
        if word == "sed":
            if "-n" not in tokens or any(t.startswith("-i") for t in tokens):
                return False
            continue
        if word in ("tail", "head", "get-content", "gc") and flags & {f.lower() for f in FOLLOW_OR_BYTES}:
            return False
        if any(t.lower() in ("-encoding", "-asbytestream") for t in tokens) and "byte" in stage.lower():
            return False
        allowed = READERS if index == 0 else FILTERS | READERS
        if word not in allowed:
            return False
    return True


def main():
    harness = "claude"
    if "--harness" in sys.argv:
        harness = sys.argv[sys.argv.index("--harness") + 1]
    try:
        payload = json.load(sys.stdin)
        command = payload.get("tool_input", {}).get("command", "")
    except Exception:
        return 0
    if not isinstance(command, str) or not command or command.rstrip().endswith(BYPASS):
        return 0
    try:
        cargo = direct_cargo_compile(command)
        read = harness == "claude" and is_plain_read(command)
    except Exception:
        return 0

    if cargo:
        sys.stderr.write(
            "Compiling cargo commands go through the build lane: "
            "`pwsh tools/build-lane/lane.ps1 cargo <args>` (add --exclusive for "
            "workspace-wide runs). The lane holds the machine-wide slot, picks the "
            "worktree's target dir and summarizes the output. See "
            ".claude/skills/lane-build/SKILL.md. End the command with `# guard-ok` "
            "only for a deliberate bypass.\n"
        )
        return 2
    if read:
        sys.stderr.write(
            "Use the Read tool to read files (with offset/limit for a slice) and "
            "Grep/Glob to search, not grep/sed -n/cat/Get-Content in the shell. "
            "They are cheaper on context and show in the permission UI. End the "
            "command with `# guard-ok` if the shell read is really needed (binary "
            "data, a path the tools cannot reach).\n"
        )
        return 2
    return 0


if __name__ == "__main__":
    sys.exit(main())
