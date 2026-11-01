"""The `returns` harness subcommand: return-line kinds and report-contract counts, per agent type (C2).

Reads the subagent dispatches in the window, counts what each one returned and whether a contract dispatch wrote its
report, and writes returns.json. The file holds counts only: no text, paths, session ids or free-form labels: an agent
type with no agent file and no built-in or retired name is counted under `other`.
"""

import json
import re
import sys
from pathlib import Path

from report.scrub import PrivacyError, Scrubber

from .transcripts import (Dispatch, brief_report_paths, cap_denied, is_context_stop, iter_dispatches, report_suffix,
                          report_writer_types, report_written, return_line)

SCHEMA = "cimmeria-harness-returns/1"
CONTINUATION = re.compile(r"(?i)continuation \d of 3")
USABLE = ("strict", "line_first", "buried")
NO_LINE = ("prose", "short", "empty")
BUILTIN_TYPES = frozenset({"general-purpose", "Explore", "Plan", "claude", "fork", "claude-code-guide",
                           "statusline-setup"})
RETIRED_TYPES = frozenset({"packet-review-fixer"})
GROUP_FIELDS = ("dispatches", "contract_dispatches", "strict", "line_first", "buried", "prose", "short", "empty",
                "no_usable_line", "no_report_write", "context_stops", "cap_denials", "continuation_dispatches",
                "redispatches", "reviewer_lines", "not_assessed_suffix", "tokens_total", "tokens_fresh",
                "no_usable_tokens_total", "no_usable_tokens_fresh", "no_report_tokens_total",
                "no_report_tokens_fresh")
CODER_FIELDS = ("dispatches", "capped", "capped_usable_line", "uncapped", "no_usable_line_uncapped",
                "dash_report_lines", "not_run_suffix")


def _row(d: Dispatch, paths: list[str], flagged: bool) -> dict:
    """The facts of one dispatch that the counts use: its agent type, return line, checks and token totals."""
    kind, m = return_line(d.final_text)
    return {
        "type": d.agent_type or "unknown",
        "contract": bool(paths),
        "kind": kind,
        "usable": kind in USABLE,
        "report": m.group("report") if m else "",
        "extra": m.group("extra") if m else "",
        "continuation": CONTINUATION.search(d.brief) is not None,
        "redispatch": flagged,
        "capped": bool(cap_denied(d)),
        "stop": is_context_stop(d),
        "no_report": bool(paths) and not report_written(d, paths),
        "total": sum(r.input + r.cache_read + r.cache_create + r.output for r in d.requests),
        "fresh": sum(r.input + r.cache_create + r.output for r in d.requests),
    }


def _rows(args) -> list[dict]:
    """One row per dispatch in the window, in first_ts order. A contract dispatch is redispatched when its first
    report path was already named by an earlier contract dispatch, and it is not a continuation."""
    dispatches = sorted(iter_dispatches(args.projects_root, args.prefix, args.since, args.until),
                        key=lambda d: d.first_ts or "")
    seen: set[str] = set()
    rows = []
    for d in dispatches:
        paths = brief_report_paths(d.brief)
        flagged = False
        if paths:
            suffix = report_suffix(paths[0])
            flagged = suffix in seen and CONTINUATION.search(d.brief) is None
            seen.add(suffix)
        rows.append(_row(d, paths, flagged))
    return rows


def _count(g: dict, r: dict) -> None:
    """Add one row to a group's counts. The return-line and report counts cover the contract dispatches only."""
    g["dispatches"] += 1
    g["tokens_total"] += r["total"]
    g["tokens_fresh"] += r["fresh"]
    g["context_stops"] += int(r["stop"])
    g["cap_denials"] += int(r["capped"])
    g["continuation_dispatches"] += int(r["continuation"])
    if not r["contract"]:
        return
    g["contract_dispatches"] += 1
    g[r["kind"]] += 1
    g["redispatches"] += int(r["redispatch"])
    if r["kind"] in NO_LINE:
        g["no_usable_line"] += 1
        g["no_usable_tokens_total"] += r["total"]
        g["no_usable_tokens_fresh"] += r["fresh"]
    if r["no_report"]:
        g["no_report_write"] += 1
        g["no_report_tokens_total"] += r["total"]
        g["no_report_tokens_fresh"] += r["fresh"]
    if r["usable"] and r["type"] == "packet-reviewer":
        g["reviewer_lines"] += 1
        g["not_assessed_suffix"] += int("not_assessed=" in r["extra"])


def known_types(repo) -> set[str]:
    """The agent types the output may name: the agent files under <repo>/.claude/agents, the built-in types and the
    retired names. Any other type is counted under `other`."""
    return {p.stem for p in (Path(repo) / ".claude" / "agents").glob("*.md")} | BUILTIN_TYPES | RETIRED_TYPES


def _group_key(t: str, known: set[str]) -> str:
    """The group a dispatch's agent type is counted under: the type when it is known or `unknown`, else `other`."""
    return t if t in known or t == "unknown" else "other"


def _groups(rows: list[dict], writers: set[str], known: set[str]) -> dict:
    """The counts per agent type, plus `all`, `all_but_ops` and `report_writers`. A type that is not known is counted
    under `other`."""
    groups = {name: dict.fromkeys(GROUP_FIELDS, 0) for name in ("all", "all_but_ops", "report_writers")}
    for r in rows:
        names = [_group_key(r["type"], known), "all"]
        if r["type"] != "ops":
            names.append("all_but_ops")
        if r["type"] in writers:
            names.append("report_writers")
        for name in names:
            _count(groups.setdefault(name, dict.fromkeys(GROUP_FIELDS, 0)), r)
    return groups


def _coder_lines(rows: list[dict]) -> dict:
    """The counts over every packet-coder dispatch, with or without a report path: capped or not, and its line."""
    g = dict.fromkeys(CODER_FIELDS, 0)
    for r in rows:
        if r["type"] != "packet-coder":
            continue
        g["dispatches"] += 1
        if r["capped"]:
            g["capped"] += 1
            g["capped_usable_line"] += int(r["usable"])
        else:
            g["uncapped"] += 1
            g["no_usable_line_uncapped"] += int(r["kind"] in NO_LINE)
        if r["usable"] and r["report"] == "-":
            g["dash_report_lines"] += 1
        if r["usable"] and "not_run=" in r["extra"]:
            g["not_run_suffix"] += 1
    return g


def _pct(part: int, whole: int) -> str:
    """A share as a percentage with one decimal; 0.0 when there is nothing to divide by."""
    return f"{100.0 * part / whole:.1f}" if whole else "0.0"


def run(args) -> int:
    """Write returns.json for the window and print the status line. Returns 0, 2 on an IO error, or 3 when the
    privacy gate refuses the file (nothing is written then)."""
    try:
        rows = _rows(args)
        groups = _groups(rows, report_writer_types(args.repo), known_types(args.repo))
        coder = _coder_lines(rows)
        record = {"schema": SCHEMA, "window": {"since": args.since, "until": args.until}, "groups": groups,
                  "coder_lines": coder}
        text = json.dumps(record, indent=2, sort_keys=True) + "\n"
        try:
            Scrubber().assert_clean(text, "returns.json")
        except PrivacyError:
            print("status=error privacy", file=sys.stderr)
            return 3
        out = Path(args.out)
        out.mkdir(parents=True, exist_ok=True)
        (out / "returns.json").write_text(text, encoding="utf-8", newline="\n")
    except OSError as e:
        print(f"status=error {type(e).__name__}", file=sys.stderr)
        return 2
    everyone, writers = groups["all"], groups["report_writers"]
    print(f"status=ok returns contract={everyone['contract_dispatches']} "
          f"writers_no_line={_pct(writers['no_usable_line'], writers['contract_dispatches'])} "
          f"writers_no_report={_pct(writers['no_report_write'], writers['contract_dispatches'])} "
          f"coder_no_line_uncapped={_pct(coder['no_usable_line_uncapped'], coder['uncapped'])} "
          f"coder_capped={coder['capped']}")
    return 0
