#!/usr/bin/env python3
"""Report on the build lane's job log.

tools/build-lane/lane.sh appends one JSON line per job to
``%LOCALAPPDATA%\\cimmeria-build\\metrics\\jobs.jsonl``: when it started, how long it
waited for a slot and ran, its exit code, worktree and commit, the settings it ran
with (jobs, incremental, Dev Drive or local target, sccache), the lowest free RAM
while it ran and the sccache hits and misses it caused.

Usage:
    python tools/build-lane/lane_stats.py                  # last 14 days: by kind, by day, Dev Drive vs local
    python tools/build-lane/lane_stats.py --days 60 --kind check
    python tools/build-lane/lane_stats.py --match cell-combat --worktree bo-c2
    python tools/build-lane/lane_stats.py --recent 20      # the last 20 jobs
    python tools/build-lane/lane_stats.py --csv jobs.csv   # every field, for a spreadsheet
    python tools/build-lane/lane_stats.py --html lane.html # run time over time, one chart per kind

The sccache counters belong to the shared server, so a job's hits and misses include
those of any build that overlapped it (``busy_at_start`` says how many did at the start).
"""

from __future__ import annotations

import argparse
import csv
import html
import json
import os
import re
import statistics
import sys
import time
from collections import defaultdict
from pathlib import Path

CARGO_ALIASES = {"b": "build", "c": "check", "t": "test", "r": "run", "d": "doc"}
FIELDS = [
    "start", "worktree", "branch", "commit", "kind", "scope", "profile", "cmd", "exit",
    "wait_s", "run_s", "exclusive", "slots", "slots_total", "busy_at_start", "jobs",
    "incremental", "dev_drive", "target", "sccache", "sccache_hits", "sccache_misses",
    "min_free_mb", "mem_total_mb",
]


def default_log() -> Path:
    """The lane's log path, following the same variables lane.sh reads."""
    if os.environ.get("LANE_METRICS_DIR"):
        base = native_path(os.environ["LANE_METRICS_DIR"])
    elif os.environ.get("LANE_ROOT"):
        base = native_path(os.environ["LANE_ROOT"]) / "metrics"
    else:
        root = os.environ.get("LOCALAPPDATA") or os.path.expanduser("~/.local/share")
        base = Path(root) / "cimmeria-build" / "metrics"
    return base / "jobs.jsonl"


def native_path(p: str) -> Path:
    """Git Bash hands out /c/Users/...; Windows Python needs C:/Users/..."""
    m = re.match(r"^/([a-zA-Z])(/.*)?$", p)
    if os.name == "nt" and m:
        return Path(f"{m.group(1).upper()}:{m.group(2) or '/'}")
    return Path(p)


def classify(cmd: str) -> tuple[str, str, str]:
    """(kind, scope, profile) for a lane command line.

    kind is the cargo subcommand (check, build, clippy, test, nextest, doctest, ...) or,
    for wrapper scripts, what they do (live-db, measure). scope is "workspace", the
    packages built (without the "cimmeria-" prefix), or "default".
    """
    low = cmd.lower()
    if "measure-build" in low:
        return "measure", "", ""
    if "test-live-db" in low or "live-db-test" in low:
        return "live-db", "", ""
    toks = cmd.split()
    for i, tok in enumerate(toks):
        if re.sub(r"\.exe$", "", tok.replace("\\", "/").rsplit("/", 1)[-1].lower()) == "cargo":
            rest = toks[i + 1:]
            break
    else:
        first = toks[0].replace("\\", "/").rsplit("/", 1)[-1] if toks else "?"
        return re.sub(r"\.exe$", "", first.lower()), "", ""

    j = 0
    while j < len(rest) and rest[j].startswith(("+", "-")):
        j += 1
    sub = CARGO_ALIASES.get(rest[j], rest[j]) if j < len(rest) else "?"
    args = rest[j + 1:]
    if sub == "test" and "--doc" in args:
        kind = "doctest"
    else:
        kind = sub

    pkgs = [args[k + 1] for k, a in enumerate(args) if a in ("-p", "--package") and k + 1 < len(args)]
    pkgs += [a.split("=", 1)[1] for a in args if a.startswith("--package=")]
    if "--workspace" in args or "--all" in args:
        scope = "workspace"
    elif pkgs:
        scope = ",".join(p.removeprefix("cimmeria-") for p in pkgs)
    else:
        scope = "default"

    # nextest's own --profile picks a nextest profile; the cargo profile is --cargo-profile.
    flag = "--cargo-profile" if kind == "nextest" else "--profile"
    profile = "release" if ("--release" in args or "-r" in args) else "dev"
    for k, a in enumerate(args):
        if a == flag and k + 1 < len(args):
            profile = args[k + 1]
        elif a.startswith(flag + "="):
            profile = a.split("=", 1)[1]
    return kind, scope, profile


def load(path: Path) -> tuple[list[dict], int]:
    """Jobs from the log, oldest first, plus the number of lines that didn't parse."""
    jobs, bad = [], 0
    if not path.exists():
        return jobs, bad
    with path.open(encoding="utf-8", errors="replace") as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            try:
                job = json.loads(line)
                job["t"] = int(job["t"])
                job["run_s"] = float(job["run_s"])
                job["wait_s"] = float(job["wait_s"])
            except (ValueError, KeyError, TypeError):
                bad += 1
                continue
            job["kind"], job["scope"], job["profile"] = classify(job.get("cmd", ""))
            job["day"] = time.strftime("%Y-%m-%d", time.localtime(job["t"]))
            jobs.append(job)
    jobs.sort(key=lambda j: j["t"])
    return jobs, bad


def select(jobs: list[dict], days: float | None, kind: str | None, worktree: str | None,
           match: str | None, now: float | None = None) -> list[dict]:
    cutoff = (now or time.time()) - days * 86400 if days else None
    out = []
    for j in jobs:
        if cutoff is not None and j["t"] < cutoff:
            continue
        if kind and j["kind"] != kind:
            continue
        if worktree and worktree.lower() not in j.get("worktree", "").lower():
            continue
        if match and match.lower() not in j.get("cmd", "").lower():
            continue
        out.append(j)
    return out


def percentile(values: list[float], q: float) -> float | None:
    """Linear-interpolated percentile, q in [0, 100]; None for no values."""
    if not values:
        return None
    v = sorted(values)
    pos = (len(v) - 1) * q / 100
    lo = int(pos)
    hi = min(lo + 1, len(v) - 1)
    return v[lo] + (v[hi] - v[lo]) * (pos - lo)


def fmt_dur(s: float | None) -> str:
    if s is None:
        return "-"
    if s < 60:
        return f"{s:.1f}s"
    if s < 3600:
        return f"{int(s // 60)}m{int(s % 60):02d}s"
    return f"{int(s // 3600)}h{int(s % 3600 // 60):02d}m"


def fmt_gb(mb: float | None) -> str:
    return "-" if mb is None else f"{mb / 1024:.1f} GB"


def lowest(jobs: list[dict], field: str) -> float | None:
    vals = [j[field] for j in jobs if j.get(field) is not None]
    return min(vals) if vals else None


def hit_rate(jobs: list[dict]) -> str:
    hits = sum(j.get("sccache_hits") or 0 for j in jobs)
    misses = sum(j.get("sccache_misses") or 0 for j in jobs)
    return f"{100 * hits / (hits + misses):.0f}%" if hits + misses else "-"


def table(headers: list[str], rows: list[list[str]]) -> str:
    widths = [max(len(str(x)) for x in col) for col in zip(headers, *rows)]
    def line(cells):
        return "  ".join(str(c).ljust(w) if i == 0 else str(c).rjust(w) for i, (c, w) in enumerate(zip(cells, widths)))
    return "\n".join([line(headers), line(["-" * w for w in widths])] + [line(r) for r in rows])


def report(jobs: list[dict], days: float | None) -> str:
    if not jobs:
        return "No lane jobs in the selected window."
    out = []
    failed = sum(1 for j in jobs if j.get("exit") != 0)
    window = f"last {days:g} days" if days else "all time"
    out.append(f"Build lane, {window}: {len(jobs)} jobs, {failed} failed, "
               f"{fmt_dur(sum(j['run_s'] for j in jobs))} running, "
               f"{fmt_dur(sum(j['wait_s'] for j in jobs))} waiting for a slot")

    by_kind: dict[str, list[dict]] = defaultdict(list)
    for j in jobs:
        by_kind[j["kind"]].append(j)
    rows = []
    for kind, js in sorted(by_kind.items(), key=lambda kv: -len(kv[1])):
        runs = [j["run_s"] for j in js]
        rows.append([kind, len(js), sum(1 for j in js if j.get("exit") != 0),
                     fmt_dur(percentile(runs, 50)), fmt_dur(percentile(runs, 90)), fmt_dur(max(runs)),
                     fmt_dur(percentile([j["wait_s"] for j in js], 90)),
                     fmt_gb(lowest(js, "min_free_mb")), hit_rate(js)])
    out += ["", "By kind", table(["kind", "jobs", "failed", "p50 run", "p90 run", "max run",
                                  "p90 wait", "lowest free RAM", "sccache hits"], rows)]

    by_day: dict[str, list[dict]] = defaultdict(list)
    for j in jobs:
        by_day[j["day"]].append(j)
    rows = []
    for day, js in sorted(by_day.items()):
        def p50(kinds):
            return fmt_dur(percentile([j["run_s"] for j in js if j["kind"] in kinds], 50))
        rows.append([day, len(js), sum(1 for j in js if j.get("exit") != 0),
                     fmt_dur(sum(j["run_s"] for j in js)), p50({"check"}), p50({"clippy"}),
                     p50({"build"}), p50({"test", "nextest"}),
                     fmt_dur(max(j["wait_s"] for j in js)), fmt_gb(lowest(js, "min_free_mb"))])
    out += ["", "By day (median run time per kind)",
            table(["day", "jobs", "failed", "total run", "check", "clippy", "build", "test",
                   "max wait", "lowest free RAM"], rows)]

    rows = []
    for kind, js in sorted(by_kind.items()):
        local = [j["run_s"] for j in js if not j.get("dev_drive")]
        dev = [j["run_s"] for j in js if j.get("dev_drive")]
        if local and dev:
            rows.append([kind, len(local), fmt_dur(percentile(local, 50)), len(dev), fmt_dur(percentile(dev, 50))])
    if rows:
        out += ["", "Dev Drive vs local target (different jobs; narrow with --match to compare like with like)",
                table(["kind", "local jobs", "local p50", "Dev Drive jobs", "Dev Drive p50"], rows)]

    slow = sorted(jobs, key=lambda j: -j["run_s"])[:5]
    out += ["", "Slowest jobs", recent_table(slow)]
    return "\n".join(out)


def recent_table(jobs: list[dict]) -> str:
    rows = [[time.strftime("%m-%d %H:%M", time.localtime(j["t"])), j.get("worktree", ""), j["kind"],
             j["scope"][:40], fmt_dur(j["wait_s"]), fmt_dur(j["run_s"]), j.get("exit"),
             "B:" if j.get("dev_drive") else "local", fmt_gb(j.get("min_free_mb"))] for j in jobs]
    return table(["started", "worktree", "kind", "scope", "wait", "run", "exit", "target", "free RAM"], rows)


def write_csv(jobs: list[dict], path: Path) -> None:
    with path.open("w", newline="", encoding="utf-8") as f:
        w = csv.DictWriter(f, fieldnames=FIELDS, extrasaction="ignore")
        w.writeheader()
        w.writerows(jobs)


def svg_chart(js: list[dict], t0: int, t1: int, width: int = 760, height: int = 200) -> str:
    """Run time against start time: one dot per job, a line through the daily medians."""
    ml, mr, mt, mb = 56, 12, 10, 24
    pw, ph = width - ml - mr, height - mt - mb
    ymax = max(j["run_s"] for j in js) * 1.08 or 1
    span = max(t1 - t0, 1)
    def x(t):
        return ml + pw * (t - t0) / span
    def y(s):
        return mt + ph * (1 - s / ymax)
    parts = [f'<svg viewBox="0 0 {width} {height}" role="img">']
    for k in range(5):
        s = ymax * k / 4
        parts.append(f'<line class="grid" x1="{ml}" x2="{width - mr}" y1="{y(s):.1f}" y2="{y(s):.1f}"/>'
                     f'<text class="axis" x="{ml - 6}" y="{y(s) + 4:.1f}" text-anchor="end">{fmt_dur(s)}</text>')
    days = sorted({j["day"] for j in js})
    step = max(1, len(days) // 6)
    for d in days[::step]:
        t = time.mktime(time.strptime(d, "%Y-%m-%d")) + 43200
        if t0 <= t <= t1:
            parts.append(f'<text class="axis" x="{x(t):.1f}" y="{height - 6}" text-anchor="middle">{d[5:]}</text>')
    medians = []
    for d in days:
        day_js = [j for j in js if j["day"] == d]
        medians.append((statistics.mean(j["t"] for j in day_js), percentile([j["run_s"] for j in day_js], 50)))
    if len(medians) > 1:
        pts = " ".join(f"{x(t):.1f},{y(s):.1f}" for t, s in medians)
        parts.append(f'<polyline class="median" points="{pts}"/>')
    for j in js:
        cls = "dev" if j.get("dev_drive") else "local"
        if j.get("exit") != 0:
            cls += " fail"
        tip = html.escape(f'{j.get("start", "")} {j.get("worktree", "")} {j["scope"]}: '
                          f'ran {fmt_dur(j["run_s"])}, waited {fmt_dur(j["wait_s"])}, exit {j.get("exit")}')
        parts.append(f'<circle class="{cls}" cx="{x(j["t"]):.1f}" cy="{y(j["run_s"]):.1f}" r="3.5"><title>{tip}</title></circle>')
    parts.append("</svg>")
    return "".join(parts)


def write_html(jobs: list[dict], path: Path, days: float | None) -> None:
    by_kind: dict[str, list[dict]] = defaultdict(list)
    for j in jobs:
        by_kind[j["kind"]].append(j)
    t0 = min(j["t"] for j in jobs) if jobs else 0
    t1 = max(j["t"] for j in jobs) if jobs else 1
    sections = []
    for kind, js in sorted(by_kind.items(), key=lambda kv: -len(kv[1])):
        runs = [j["run_s"] for j in js]
        sections.append(
            f'<section><h2>{html.escape(kind)} <span class="meta">{len(js)} jobs, median '
            f'{fmt_dur(percentile(runs, 50))}, p90 {fmt_dur(percentile(runs, 90))}</span></h2>'
            f'{svg_chart(js, t0, t1)}</section>')
    summary = html.escape(report(jobs, days))
    window = f"last {days:g} days" if days else "all time"
    page = f"""<!doctype html>
<html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1">
<title>Build Lane Metrics</title>
<style>
:root {{ --bg:#fbfbfa; --fg:#1d1d1f; --muted:#6b6b70; --grid:#e4e4e2; --dev:#2f6fd6; --local:#9a9aa0; --fail:#d0342c; --median:#1d1d1f; }}
@media (prefers-color-scheme: dark) {{ :root {{ --bg:#17171a; --fg:#ececef; --muted:#9a9aa3; --grid:#2c2c31; --dev:#6ea0ff; --local:#77777f; --fail:#ff6b61; --median:#ececef; }} }}
body {{ background:var(--bg); color:var(--fg); font:15px/1.5 system-ui, sans-serif; margin:0 auto; max-width:820px; padding:24px 16px; }}
h1 {{ font-size:22px; margin:0 0 4px; }} h2 {{ font-size:16px; margin:28px 0 6px; }}
.meta {{ color:var(--muted); font-weight:normal; font-size:14px; }}
svg {{ width:100%; height:auto; display:block; }}
.grid {{ stroke:var(--grid); }} .axis {{ fill:var(--muted); font-size:11px; }}
.median {{ fill:none; stroke:var(--median); stroke-width:1.5; opacity:.6; }}
circle.dev {{ fill:var(--dev); }} circle.local {{ fill:var(--local); }}
circle.fail {{ fill:none; stroke:var(--fail); stroke-width:2; }}
.legend span {{ margin-right:16px; }} .dot {{ display:inline-block; width:9px; height:9px; border-radius:50%; margin-right:5px; }}
pre {{ font-size:12px; overflow-x:auto; background:color-mix(in srgb, var(--fg) 5%, transparent); padding:12px; border-radius:6px; }}
</style></head><body>
<h1>Build lane metrics</h1>
<p class="meta">{len(jobs)} jobs, {window}, generated {time.strftime("%Y-%m-%d %H:%M")}</p>
<p class="legend meta"><span><i class="dot" style="background:var(--dev)"></i>Dev Drive target</span>
<span><i class="dot" style="background:var(--local)"></i>local target</span>
<span><i class="dot" style="border:2px solid var(--fail)"></i>failed</span><span>line: daily median</span></p>
{''.join(sections)}
<h2>Summary</h2><pre>{summary}</pre>
</body></html>
"""
    path.write_text(page, encoding="utf-8")


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--file", type=Path, default=None, help="job log (default: the lane's jobs.jsonl)")
    ap.add_argument("--days", type=float, default=14, help="window in days; 0 for everything (default 14)")
    ap.add_argument("--kind", help="only this kind (check, build, clippy, nextest, test, live-db, ...)")
    ap.add_argument("--worktree", help="only worktrees whose name contains this")
    ap.add_argument("--match", help="only commands containing this text")
    ap.add_argument("--recent", type=int, metavar="N", help="list the last N jobs instead of the summary")
    ap.add_argument("--csv", type=Path, metavar="PATH", help="write the selected jobs as CSV")
    ap.add_argument("--html", type=Path, metavar="PATH", help="write charts of run time over time")
    args = ap.parse_args(argv)

    path = args.file or default_log()
    jobs, bad = load(path)
    if not jobs and not path.exists():
        print(f"No job log at {path}. lane.sh writes it once a job has run through the lane.", file=sys.stderr)
        return 1
    days = args.days or None
    jobs = select(jobs, days, args.kind, args.worktree, args.match)
    if bad:
        print(f"(skipped {bad} unreadable line(s) in {path})", file=sys.stderr)

    if args.csv:
        write_csv(jobs, args.csv)
        print(f"wrote {len(jobs)} jobs to {args.csv}")
    if args.html:
        write_html(jobs, args.html, days)
        print(f"wrote {args.html}")
    if args.recent:
        print(recent_table(jobs[-args.recent:]))
    elif not (args.csv or args.html):
        print(report(jobs, days))
    return 0


if __name__ == "__main__":
    sys.exit(main())
