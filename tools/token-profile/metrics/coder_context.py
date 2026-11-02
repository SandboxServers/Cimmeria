"""The `coder-context` harness subcommand: what fills a packet-coder's context.

Reads the packet-coder dispatches of a checkout and writes coder-context.json: how many stopped on the context cap, what
their results were made of (by tool category and by the Read target), and the medians of their requests and calls.
Counts and shares only; no text, paths or session ids are written.
"""

import json
import re
import statistics
import sys
from pathlib import Path

from report.scrub import PrivacyError, Scrubber

from .transcripts import CONTINUATION_RE, SHELL_TOOLS, Dispatch, cap_denied, is_context_stop, iter_dispatches, norm_path

CATEGORIES = ("Read", "Grep", "Glob", "shell-read", "shell-git", "lane-check", "lane-clippy", "lane-test", "lane-fmt",
              "lane-other", "shell-other", "Edit/Write", "other")
TARGETS = ("docs/analysis", "docs-other", "crates", "tools", ".claude", "other")
MEDIANS = ("first_context", "last_context", "output_tokens", "requests", "result_tokens", "lane_calls", "reads",
           "greps", "edits")
TARGET_MARKS = (("/docs/analysis/", "docs/analysis"), ("/docs/", "docs-other"), ("/crates/", "crates"),
                ("/tools/", "tools"), ("/.claude/", ".claude"))
EDIT_TOOLS = ("Edit", "Write", "MultiEdit")
COMMIT_RE = re.compile(r"\bgit\b[^\n]*\bcommit\b")
CHECK_RE = re.compile(r"lane\.(?:ps1|sh)|unittest|pytest|regen\.py|sync\.py --check")
PASS_RE = re.compile(r"status=ok\b|^OK\b|test result: ok\b", re.M)
FAIL_RE = re.compile(r"status=(?:failed|error)\b|^FAILED\b|test result: FAILED\b", re.M)


def category(name, inp):
    """The result category of a tool use: a lane call, a shell read, a git call, an edit, or the tool's own name."""
    if name in ("PowerShell", "Bash"):
        c = (inp or {}).get("command", "")
        if "lane.ps1" in c or "lane.sh" in c:
            if " fmt" in c: return "lane-fmt"
            if "clippy" in c: return "lane-clippy"
            if "nextest" in c or " test" in c: return "lane-test"
            if " check" in c: return "lane-check"
            return "lane-other"
        if re.search(r"\bgit\b", c): return "shell-git"
        if "Get-Content" in c or "cat " in c or "Select-String" in c: return "shell-read"
        return "shell-other"
    if name in ("Read", "Grep", "Glob"): return name
    if name in ("Edit", "Write", "MultiEdit"): return "Edit/Write"
    return "other"


def read_target(file_path: str) -> str:
    """The part of the repository a Read names, by its normalised path; "other" when none of the marks is in it."""
    n = norm_path(file_path)
    for mark, name in TARGET_MARKS:
        if mark in n:
            return name
    return "other"


def _fresh(d: Dispatch) -> int:
    """The fresh tokens of a dispatch: input, cache creation and output, summed over its requests."""
    return sum(r.input + r.cache_create + r.output for r in d.requests)


def _command(name: str, inp: dict) -> str:
    """The command of a shell tool use, or "" for any other tool."""
    return str((inp or {}).get("command", "")) if name in SHELL_TOOLS else ""


def _passed(result: str) -> bool:
    """True when the last 3,000 chars of a check's result show a pass and no failure."""
    tail = result[-3000:]
    return PASS_RE.search(tail) is not None and FAIL_RE.search(tail) is None


def _habits(d: Dispatch, denied: set[str]) -> dict:
    """The commit, passing-check, read and grep habits of one dispatch. A cap-denied call never ran, so it counts for
    nothing."""
    uses = [t for t in d.tool_uses if t.id not in denied]
    commits = [i for i, t in enumerate(uses) if COMMIT_RE.search(_command(t.name, t.input))]
    edits = [i for i, t in enumerate(uses) if t.name in EDIT_TOOLS]
    pass_uncommitted = False
    for i, t in enumerate(uses):
        if not CHECK_RE.search(_command(t.name, t.input)) or not _passed(d.results.get(t.id, "")):
            continue
        if commits and commits[-1] > i:
            continue
        last = max((j for j in commits if j < i), default=-1)
        if any(last < k < i for k in edits):
            pass_uncommitted = True
    h = {"committed": bool(commits), "pass_uncommitted": pass_uncommitted, "read_chars": 0, "read_whole_chars": 0,
         "read_repeat_chars": 0, "grep_chars": 0, "grep_unbounded_chars": 0}
    seen = set()
    for t in uses:
        n = len(d.results.get(t.id, ""))
        if t.name == "Read":
            path = norm_path(str(t.input.get("file_path") or ""))
            h["read_chars"] += n
            if "offset" not in t.input and "limit" not in t.input:
                h["read_whole_chars"] += n
            if path in seen:
                h["read_repeat_chars"] += n
            seen.add(path)
        elif t.name == "Grep":
            h["grep_chars"] += n
            if t.input.get("output_mode") == "content" and not t.input.get("head_limit"):
                h["grep_unbounded_chars"] += n
    return h


def _row(d: Dispatch) -> dict:
    """The counts of one packet-coder dispatch: result chars by category and by Read target, its medians and habits."""
    chars = dict.fromkeys(CATEGORIES, 0)
    targets = dict.fromkeys(TARGETS, 0)
    denied = cap_denied(d)
    cats = []
    for t in d.tool_uses:
        c = category(t.name, t.input)
        n = len(d.results.get(t.id, ""))
        chars[c] += n
        cats.append(c)
        if t.name == "Read":
            targets[read_target(str(t.input.get("file_path") or ""))] += n
    reqs = d.requests
    return {
        "chars": chars, "targets": targets, "stopped": is_context_stop(d), "capped": bool(denied),
        "fresh": _fresh(d), **_habits(d, {t.id for t in denied}),
        "first_context": reqs[0].context if reqs else 0,
        "last_context": reqs[-1].context if reqs else 0,
        "output_tokens": sum(r.output for r in reqs),
        "requests": len(reqs),
        "result_tokens": sum(chars.values()) // 4,
        "lane_calls": sum(c.startswith("lane-") for c in cats),
        "reads": cats.count("Read"),
        "greps": cats.count("Grep"),
        "edits": cats.count("Edit/Write"),
    }


def _share(part: int, total: int) -> float:
    """A fraction of the total, rounded to 3 places; 0 when the total is 0."""
    return round(part / total, 3) if total else 0.0


def _median(rows: list[dict], key: str):
    """The median of one count over the rows; 0 for no rows."""
    return statistics.median([r[key] for r in rows]) if rows else 0


def _sum(rows: list[dict], key: str) -> int:
    """The sum of one count over the rows."""
    return sum(r[key] for r in rows)


def _group(rows: list[dict]) -> dict:
    """One group of the coder-context output: its counts, result shares by category and Read target, medians, and
    habit shares."""
    total = sum(sum(r["chars"].values()) for r in rows)
    stops = sum(r["stopped"] for r in rows)
    reads = _sum(rows, "read_chars")
    return {
        "coders": len(rows),
        "stops": stops,
        "stop_rate": round(stops / len(rows), 3) if rows else 0.0,
        "cap_denied": sum(r["capped"] for r in rows),
        "result_chars_share": {k: _share(sum(r["chars"][k] for r in rows), total) for k in CATEGORIES},
        "read_targets_share": {k: _share(sum(r["targets"][k] for r in rows), total) for k in TARGETS},
        "median": {k: _median(rows, k) for k in MEDIANS},
        "committed_share": _share(_sum(rows, "committed"), len(rows)),
        "pass_uncommitted_share": _share(_sum(rows, "pass_uncommitted"), len(rows)),
        "read_whole_share": _share(_sum(rows, "read_whole_chars"), reads),
        "read_repeat_share": _share(_sum(rows, "read_repeat_chars"), reads),
        "grep_unbounded_share": _share(_sum(rows, "grep_unbounded_chars"), _sum(rows, "grep_chars")),
    }


def build(dispatches) -> dict:
    """The coder-context document for the packet-coder dispatches among `dispatches`, without the window."""
    coders = [d for d in dispatches if d.agent_type == "packet-coder"]
    rows = [_row(d) for d in coders]
    cont = [r for d, r in zip(coders, rows) if CONTINUATION_RE.search(d.brief)]
    fresh = [r["fresh"] for r in cont]
    stops = sum(r["stopped"] for r in cont)
    return {
        "groups": {"all": _group(rows), "stopped": _group([r for r in rows if r["stopped"]])},
        "continuations": len(fresh),
        "continuation_tokens_fresh": sum(fresh),
        "median_continuation_fresh": statistics.median(fresh) if fresh else 0,
        "continuation_stops": stops,
        "continuation_stop_rate": _share(stops, len(fresh)),
    }


def run(args):
    """Write coder-context.json under args.out and print its status line; returns the exit status."""
    try:
        dispatches = list(iter_dispatches(args.projects_root, args.prefix, args.since, args.until))
    except OSError as e:
        print(f"status=error {type(e).__name__}", file=sys.stderr)
        return 2
    doc ={"schema": "cimmeria-harness-coder-context/1", "window": {"since": args.since, "until": args.until}}
    doc.update(build(dispatches))
    text = json.dumps(doc, indent=2, sort_keys=True) + "\n"
    try:
        Scrubber().assert_clean(text, "coder-context.json")
    except PrivacyError:
        print("status=error privacy", file=sys.stderr)
        return 3
    try:
        out = Path(args.out)
        out.mkdir(parents=True, exist_ok=True)
        (out / "coder-context.json").write_text(text, encoding="utf-8", newline="\n")
    except OSError as e:
        print(f"status=error {type(e).__name__}", file=sys.stderr)
        return 2
    al, st = doc["groups"]["all"], doc["groups"]["stopped"]
    print(f"status=ok coder-context coders={al['coders']} stops={al['stops']} stop_rate={al['stop_rate']} "
          f"read_share={al['result_chars_share']['Read']} grep_share={al['result_chars_share']['Grep']} "
          f"continuations={doc['continuations']} stopped_committed={st['committed_share']} "
          f"stopped_pass_uncommitted={st['pass_uncommitted_share']}")
    return 0