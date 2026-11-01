#!/usr/bin/env python3
"""Lint the agent memories under .claude/agent-memory/ (C8): untyped, stale-state, unindexed and orphan memories.

Observation only: the lint reports, and it exits 1 only with --strict. A memory is
a lead, not evidence, so a state claim (`STATE`) needs an `INVALIDATED WHEN:` line.
Standard library only.

Usage:
  python tools/agent-skills/memory_lint.py [--root DIR] [--strict] [--json] [--since YYYY-MM-DD]
"""
import argparse
import json
import re
import subprocess
import sys
from collections import Counter
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
MEMORY_DIR = ".claude/agent-memory"
CHECKS = ("no_type", "state_no_invalidated", "index_over", "index_missing", "unindexed", "orphan_folders", "ip_like")
STATE = re.compile(
    r"(?i)\b(not (?:yet )?(?:implemented|wired|persisted|supported|handled|sent|fixed)"
    r"|is (?:missing|broken|stubbed|unimplemented)|no longer"
    r"|still (?:open|missing|broken|pending|unfixed)|currently|as of \d{4}-\d{2}-\d{2}"
    r"|pending|in flight|open (?:issue|PR|bug)|gap)\b"
)
IP_LIKE = re.compile(r"\b(?:\d{1,3}\.){3}\d{1,3}\b")
IP_OK = frozenset({"0.0.0.0", "127.0.0.1"})
TYPE_LINE = re.compile(r"^\s*type:\s*(.*?)\s*$")
INVALIDATED = "INVALIDATED WHEN:"
INDEX_MAX_LINES = 200
INDEX_MAX_BYTES = 25000


def split_frontmatter(text):
    """Return (frontmatter lines, body text); the frontmatter is the block between the first two `---` lines."""
    lines = text.splitlines()
    if lines[:1] == ["---"]:
        for end in range(1, len(lines)):
            if lines[end].strip() == "---":
                return lines[1:end], "\n".join(lines[end + 1:])
    return [], text


def memory_type(front):
    """Return the `type:` value: a top-level line, or one indented under `metadata:`; None when absent."""
    in_meta = False
    for line in front:
        if line[:1] and not line[0].isspace():
            in_meta = line.rstrip() == "metadata:"
            found = TYPE_LINE.match(line)
        else:
            found = TYPE_LINE.match(line) if in_meta else None
        value = found.group(1).strip("\"'").lower() if found else ""
        if value:
            return value
    return None


def folder_of(mem, directory):
    """Return a directory's path relative to the memory folder, as posix text ('.' for the folder itself)."""
    return directory.relative_to(mem).as_posix()


def added_state(root, since):
    """Count the project state memories added since a date (git log --diff-filter=A), and those with INVALIDATED WHEN:."""
    proc = subprocess.run(
        ["git", "-C", str(root), "log", "--diff-filter=A", "--name-only", "--format=%x00%cs",
         f"--since={since}", "--", MEMORY_DIR],
        capture_output=True, text=True, encoding="utf-8", errors="replace", check=False,
    )
    if proc.returncode != 0:
        raise RuntimeError(proc.stderr.strip() or "git log failed")
    paths = {line.replace("\x00", "").strip() for line in proc.stdout.splitlines()}
    added = sorted(p for p in paths if p.startswith(MEMORY_DIR + "/") and p.endswith(".md") and not p.endswith("/MEMORY.md"))
    state = with_invalidated = 0
    for rel in added:
        path = root / rel
        if not path.is_file():
            continue
        text = path.read_text(encoding="utf-8", errors="replace")
        front, body = split_frontmatter(text)
        if memory_type(front) == "project" and STATE.search(body):
            state += 1
            if INVALIDATED in text:
                with_invalidated += 1
    return {"state_added": state, "state_added_with_invalidated": with_invalidated}


def scan(root, since=None):
    """Lint the memory files under root; return the files scanned, the findings, the counts per folder and the --since figures."""
    mem = root / MEMORY_DIR
    findings = []
    folders = {}

    def flag(check, path, folder):
        """Record one finding (a file, an index or a folder) and count it under its folder."""
        findings.append((check, path.relative_to(root).as_posix(), folder))
        folders.setdefault(folder, Counter())[check] += 1

    facts = []
    if mem.is_dir():
        facts = sorted(p for p in mem.rglob("*.md") if p.name != "MEMORY.md")
        for index in sorted(mem.rglob("MEMORY.md")):
            size = len(index.read_bytes())
            if len(index.read_text(encoding="utf-8", errors="replace").splitlines()) > INDEX_MAX_LINES or size > INDEX_MAX_BYTES:
                flag("index_over", index, folder_of(mem, index.parent))
        for directory in sorted({p.parent for p in facts}):
            if not (directory / "MEMORY.md").is_file():
                flag("index_missing", directory / "MEMORY.md", folder_of(mem, directory))
        for path in facts:
            folder = folder_of(mem, path.parent)
            folders.setdefault(folder, Counter())["files"] += 1
            text = path.read_text(encoding="utf-8", errors="replace")
            front, body = split_frontmatter(text)
            kind = memory_type(front)
            if kind is None:
                flag("no_type", path, folder)
            if kind == "project" and STATE.search(body) and INVALIDATED not in text:
                flag("state_no_invalidated", path, folder)
            index = path.parent / "MEMORY.md"
            # Without an index the file is reported as index_missing for its folder, not as unindexed.
            if index.is_file() and path.name not in index.read_text(encoding="utf-8", errors="replace"):
                flag("unindexed", path, folder)
            if any(ip not in IP_OK for ip in IP_LIKE.findall(text)):
                flag("ip_like", path, folder)
        for directory in sorted(p for p in mem.iterdir() if p.is_dir()):
            if directory.name != "main-session" and not (root / ".claude" / "agents" / f"{directory.name}.md").is_file():
                flag("orphan_folders", directory, directory.name)
    totals = Counter(check for check, _, _ in findings)
    added = added_state(root, since) if since else None
    return {"files": len(facts), "findings": findings, "folders": folders, "totals": totals, "since": added}


def summary_line(result):
    """Return the last stdout line: the file count and every check's count, plus the --since figures when given."""
    totals = result["totals"]
    parts = [f"memory_lint files={result['files']}"] + [f"{check}={totals[check]}" for check in CHECKS]
    parts += [f"{key}={value}" for key, value in (result["since"] or {}).items()]
    return " ".join(parts)


def build_parser():
    """Return the command-line parser."""
    parser = argparse.ArgumentParser(description="Lint .claude/agent-memory/ for untyped, stale-state and unindexed memories.")
    parser.add_argument("--root", type=Path, default=ROOT, help="repo root holding .claude/ (default: this repo)")
    parser.add_argument("--strict", action="store_true", help="exit 1 when any check except orphan_folders is above 0")
    parser.add_argument("--json", action="store_true", help="print a per-folder table as JSON, not one line per file")
    parser.add_argument("--since", metavar="YYYY-MM-DD", help="also count state memories added since this date")
    return parser


def main(argv=None):
    """Run the lint, print its report and its summary line, and return the exit code."""
    args = build_parser().parse_args(argv)
    root = args.root.resolve()
    try:
        result = scan(root, args.since)
    except RuntimeError as err:
        print(f"memory_lint: {err}", file=sys.stderr)
        return 2
    totals = result["totals"]
    if args.json:
        folders = {name: {"files": counts["files"], **{check: counts[check] for check in CHECKS}}
                   for name, counts in sorted(result["folders"].items())}
        payload = {"files": result["files"], "totals": {check: totals[check] for check in CHECKS}, "folders": folders}
        print(json.dumps(payload, indent=2, sort_keys=True))
    else:
        for check, rel, _ in result["findings"]:
            print(f"{check} {rel}")
    print(summary_line(result))
    if args.strict and any(totals[check] for check in CHECKS if check != "orphan_folders"):
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
