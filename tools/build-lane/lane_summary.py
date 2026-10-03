#!/usr/bin/env python3
"""Summarise a build-lane job's log for an agent caller.

When lane.sh's stdout is not a terminal (an agent's Bash tool), the lane writes the full
cargo or nextest output to a log file and prints only what this script extracts from it:

    [lane] status=failed exit=101 ran=1.9s
    tests: 1 passed, 2 failed, 1 ignored
    FAIL tests::wrong_sum
      thread 'tests::wrong_sum' panicked at src\\lib.rs:16:9:
      assertion `left == right` failed: sum of 2 and 2
    ...
    failures: <file with every failure in full>
    log: <full log>

It recognises rustc diagnostics (cargo check / build / clippy / test compile errors),
libtest output (cargo test) and nextest output. A failed job it can't parse still prints
the log's last lines, so an error is never hidden behind a "see the log".

    python lane_summary.py --log FILE --exit CODE [--failures FILE] [--ran SECONDS] [--note TEXT]
"""

from __future__ import annotations

import argparse
import re
import sys
from dataclasses import dataclass, field
from pathlib import Path

# Limits on what reaches stdout. The failures file has everything.
MAX_ERRORS = 8          # diagnostics shown
MAX_ERROR_LINES = 14    # lines per diagnostic
MAX_FAILED_TESTS = 10   # failing tests shown with detail
MAX_DETAIL_LINES = 8    # lines per failing test
MAX_WARNINGS = 5        # warning headlines shown
MAX_LINE = 300          # chars per line
TAIL_LINES = 25         # fallback tail of an unparsed failure

DIAG_HEAD = re.compile(r"^(error|warning)(\[[A-Za-z0-9_:]+\])?: ")
# cargo's own roll-up lines, not diagnostics of their own.
WARN_ROLLUP = re.compile(r"^warning: (`[^`]+` \(.*\) generated \d+ warnings?|build failed, waiting)")
LIBTEST_RESULT = re.compile(
    r"^test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out")
LIBTEST_SECTION = re.compile(r"^---- (.+?) (stdout|stderr) ----$")
LIBTEST_FAILED_LINE = re.compile(r"^test (.+?) \.\.\. FAILED$")
NEXTEST_SUMMARY = re.compile(r"^\s*Summary \[[^\]]*\]\s+(.*)$")
NEXTEST_COUNT = re.compile(r"(\d+) (passed|failed|skipped|timed out|leaky|flaky|exec failed|ignored)")
NEXTEST_RUN = re.compile(r"(\d+)(?:/(\d+))? tests? run")
NEXTEST_STATUS = re.compile(r"^\s+([A-Z][A-Z0-9-]*(?: [A-Z0-9-]+)*)\s+\[[^\]]*\]\s+(?:\(\s*\d+/\s*\d+\)\s+)?(\S.*?)\s*$")
NEXTEST_FAIL_STATUSES = {"FAIL", "TIMEOUT", "SIGSEGV", "SIGABRT", "SIGKILL", "SIGBUS", "ABORT", "LEAK-FAIL",
                         "EXEC FAIL", "FLAKY FAIL"}
NOISE = re.compile(r"^(note: run with `RUST_BACKTRACE=1`|running \d+ tests?$|failures:$|test result: |"
                   r"test .+ \.\.\. FAILED$|stdout ───$|stderr ───$|Cancelling due to )")


@dataclass
class Summary:
    errors: list[list[str]] = field(default_factory=list)        # rustc / cargo error blocks
    warnings: list[list[str]] = field(default_factory=list)
    cargo_notes: list[str] = field(default_factory=list)         # one-line `error:` roll-ups
    tests: dict[str, int] = field(default_factory=dict)
    runner: str = ""                                              # "nextest", "libtest" or ""
    failed: list[str] = field(default_factory=list)
    details: dict[str, list[str]] = field(default_factory=dict)


def clip(line: str) -> str:
    return line if len(line) <= MAX_LINE else line[:MAX_LINE] + " ..."


def diagnostics(lines: list[str]) -> tuple[list[list[str]], list[list[str]], list[str]]:
    """rustc/cargo `error:` and `warning:` blocks. A block runs from its header to a blank
    line or the next header; identical blocks (nextest builds lib and lib test) count once."""
    errors, warnings, notes, seen = [], [], [], set()
    block: list[str] | None = None

    def close():
        nonlocal block
        if block:
            key = "\n".join(block)
            if key not in seen:
                seen.add(key)
                is_error = block[0].startswith("error")
                if len(block) == 1 and not DIAG_HEAD.match(block[0]).group(2):
                    # `error: could not compile ...`, `error: test failed ...`: a roll-up.
                    # `error: command `<cargo path>` ... exited` only repeats them.
                    if is_error and not block[0].startswith("error: command `"):
                        notes.append(block[0])
                else:
                    (errors if is_error else warnings).append(block)
        block = None

    for line in lines:
        if DIAG_HEAD.match(line):
            close()
            if WARN_ROLLUP.match(line):
                continue
            block = [line]
        elif block is not None:
            if not line.strip():
                close()
            else:
                block.append(line)
    close()
    return errors, warnings, notes


def panic_excerpt(chunk: list[str]) -> list[str]:
    """The useful lines of one failing test's output: from the panic line on, else the
    first non-boilerplate lines. Common indentation (nextest's 4 spaces) is removed."""
    body = [l.rstrip() for l in chunk if l.strip() and not NOISE.match(l.strip())]
    indent = min((len(l) - len(l.lstrip()) for l in body), default=0)
    body = [l[indent:] for l in body]
    for i, line in enumerate(body):
        if "panicked at" in line:
            return body[i:i + MAX_DETAIL_LINES]
    return body[:MAX_DETAIL_LINES]


def parse_nextest(lines: list[str], s: Summary) -> None:
    s.runner = "nextest"
    for line in lines:
        m = NEXTEST_SUMMARY.match(line)
        if m:
            tail = m.group(1)
            s.tests = {k: int(v) for v, k in NEXTEST_COUNT.findall(tail)}
            run = NEXTEST_RUN.search(tail)
            if run and run.group(2):
                s.tests["not run"] = int(run.group(2)) - int(run.group(1))
    current: str | None = None
    chunk: list[str] = []

    def flush():
        if current is not None and chunk:
            ex = panic_excerpt(chunk)
            # A FAIL line shows up when the test ends and again, with its output, at the end
            # of the run (--failure-output final); keep the excerpt that has the panic.
            if ex and (current not in s.details or any("panicked at" in l for l in ex)):
                s.details[current] = ex

    for line in lines:
        m = NEXTEST_STATUS.match(line)
        if m:
            flush()
            current, chunk = None, []
            if m.group(1) in NEXTEST_FAIL_STATUSES or m.group(1).endswith("FAIL"):
                current = m.group(2)
                if current not in s.failed:
                    s.failed.append(current)
            continue
        if line.startswith("────") or NEXTEST_SUMMARY.match(line) or line.startswith("error: "):
            flush()
            current, chunk = None, []
            continue
        if current is not None:
            chunk.append(line)
    flush()


def parse_libtest(lines: list[str], s: Summary) -> None:
    totals = {"passed": 0, "failed": 0, "ignored": 0, "filtered out": 0}
    found = False
    for line in lines:
        m = LIBTEST_RESULT.match(line)
        if m:
            found = True
            totals["passed"] += int(m.group(2)); totals["failed"] += int(m.group(3))
            totals["ignored"] += int(m.group(4)); totals["filtered out"] += int(m.group(6))
        m = LIBTEST_FAILED_LINE.match(line)
        if m and m.group(1) not in s.failed:
            s.failed.append(m.group(1))
    if not found and not s.failed:
        return
    s.runner = "libtest"
    s.tests = {k: v for k, v in totals.items() if v or k in ("passed", "failed")}
    current: str | None = None
    chunk: list[str] = []
    for line in lines + ["failures:"]:
        m = LIBTEST_SECTION.match(line)
        if m or line == "failures:":
            if current is not None:
                s.details.setdefault(current, []).extend(chunk)
            current, chunk = (m.group(1) if m else None), []
            continue
        if current is not None:
            chunk.append(line)
    for name in s.details:
        s.details[name] = panic_excerpt(s.details[name])
        if name not in s.failed:
            s.failed.append(name)


def parse(text: str) -> Summary:
    # The lane's own `[lane] acquired/released` lines are not the command's output.
    lines = [l for l in text.replace("\r\n", "\n").replace("\r", "\n").split("\n") if not l.startswith("[lane] ")]
    s = Summary()
    s.errors, s.warnings, s.cargo_notes = diagnostics(lines)
    if any(NEXTEST_SUMMARY.match(l) for l in lines) or any(l.startswith(" Nextest run ID") for l in lines):
        parse_nextest(lines, s)
    else:
        parse_libtest(lines, s)
    return s


def headline(block: list[str]) -> str:
    """`warning: unused variable: x (src\\lib.rs:31:9)`"""
    loc = next((l.split("--> ", 1)[1].strip() for l in block if "--> " in l), "")
    return clip(block[0] + (f" ({loc})" if loc else ""))


def render(s: Summary, log_text: str, exit_code: int, ran: str | None, note: str | None,
           failures_path: str | None, log_path: str) -> tuple[str, str]:
    """Returns (stdout summary, failures-file text). The failures text is empty when there
    is nothing to put in it."""
    status = "ok" if exit_code == 0 else "failed"
    out = [f"[lane] status={status} exit={exit_code}" + (f" ran={ran}s" if ran else "") + (f" {note}" if note else "")]
    full: list[str] = []

    if s.tests:
        order = ["passed", "failed", "timed out", "skipped", "ignored", "filtered out", "not run", "flaky", "leaky",
                 "exec failed"]
        parts = [f"{s.tests[k]} {k}" for k in order if k in s.tests]
        out.append(f"tests ({s.runner}): " + ", ".join(parts))

    if s.errors:
        out.append(f"errors: {len(s.errors)}")
        for block in s.errors[:MAX_ERRORS]:
            shown = block[:MAX_ERROR_LINES]
            out.extend(clip(l) for l in shown)
            if len(block) > len(shown):
                out.append(f"   ... {len(block) - len(shown)} more lines in the failures file")
        if len(s.errors) > MAX_ERRORS:
            out.append(f"... and {len(s.errors) - MAX_ERRORS} more errors in the failures file")
        for block in s.errors:
            full.extend(block + [""])

    if s.failed:
        out.append(f"failed tests: {len(s.failed)}")
        for name in s.failed[:MAX_FAILED_TESTS]:
            out.append(f"FAIL {name}")
            out.extend("  " + clip(l) for l in s.details.get(name, []))
        if len(s.failed) > MAX_FAILED_TESTS:
            out.append(f"... and {len(s.failed) - MAX_FAILED_TESTS} more failing tests in the failures file")
        for name in s.failed:
            full.append(f"FAIL {name}")
            full.extend("  " + l for l in s.details.get(name, []))
            full.append("")

    if s.warnings:
        out.append(f"warnings: {len(s.warnings)}")
        out.extend("  " + headline(b) for b in s.warnings[:MAX_WARNINGS])
        if len(s.warnings) > MAX_WARNINGS:
            out.append(f"  ... and {len(s.warnings) - MAX_WARNINGS} more in the log")

    if exit_code != 0:
        out.extend(clip(n) for n in s.cargo_notes[:5])
        full.extend(s.cargo_notes)
        if not s.errors and not s.failed:
            # Nothing recognised: show the end of the log rather than hide the failure.
            tail = [l for l in log_text.replace("\r", "").split("\n")
                    if l.strip() and not l.startswith("[lane] ")][-TAIL_LINES:]
            out.append(f"no compiler error or failing test recognised; last {len(tail)} lines of the log:")
            out.extend("  " + clip(l) for l in tail)
            full.extend(tail)

    failures_text = "\n".join(full).rstrip() + "\n" if full else ""
    if failures_text and failures_path:
        out.append(f"failures: {failures_path}")
    out.append(f"log: {log_path}")
    return "\n".join(out) + "\n", failures_text


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--log", required=True)
    ap.add_argument("--exit", type=int, required=True)
    ap.add_argument("--failures", help="write every failure in full here (only if there is one)")
    ap.add_argument("--ran", help="run time in seconds, for the status line")
    ap.add_argument("--note", help="extra text for the status line")
    ap.add_argument("--display-log", help="the log path to print (default: --log)")
    ap.add_argument("--display-failures", help="the failures path to print (default: --failures)")
    a = ap.parse_args(argv)
    text = Path(a.log).read_text(encoding="utf-8", errors="replace")
    summary, failures = render(parse(text), text, a.exit, a.ran, a.note,
                               a.display_failures or a.failures, a.display_log or a.log)
    if failures and a.failures:
        Path(a.failures).write_text(failures, encoding="utf-8")
    else:
        summary = "\n".join(l for l in summary.split("\n") if not l.startswith("failures: ")).rstrip("\n") + "\n"
    sys.stdout.write(summary)
    return 0


if __name__ == "__main__":
    sys.exit(main())
