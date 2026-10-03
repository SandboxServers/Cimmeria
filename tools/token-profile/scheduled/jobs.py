"""The scheduled jobs: a weekly report and a daily per-PR stats sweep.

    python tools/token-profile/scheduled weekly [--days 7] [--until YYYY-MM-DD] [--skip-ingest]
    python tools/token-profile/scheduled pr-sweep [--days 3] [--rate 6/min] [--dry-run] [--skip-ingest]

weekly.ps1 and pr-sweep.ps1 run these; register-tasks.ps1 puts them in Task
Scheduler. Both jobs ingest first (incrementally, with --fetch-prs), then run
the profiler's own commands as child processes, each with its output in a log
file under the job home, never in the repo. Exit status: 0 every step passed;
1 a step failed (the summary names it and its exit code); 2 a usage error or
another job holds the lock.

Paths: --home or TOKEN_PROFILE_HOME (default %LOCALAPPDATA%/cimmeria-token-profile),
--db or TOKEN_PROFILE_DB (default ~/token-profile.sqlite, pr_stats' default),
--repo (default the main checkout of this one), --labels or TOKEN_PROFILE_LABELS
(optional, for validate).
"""

import argparse
import os
import subprocess
import sys
import time
from datetime import datetime, timedelta, timezone
from pathlib import Path

from ingest.cli import main_checkout

TOOLS = Path(__file__).resolve().parent.parent
# A method misplacing more than this share of labelled spend is fixed first (attribution.md).
MAX_WRONG_SHARE = "0.10"
# A lock older than this belongs to a job that died; the next job takes it over.
STALE_LOCK_SECONDS = 6 * 3600

# What each step's exit code means, for the summary. Anything else is a crash.
MEANING = {
    "ingest": {1: "unknown transcript shapes", 2: "attribution weights do not sum to 1"},
    "reconcile": {2: "input error", 3: "privacy gate refused the output", 4: "a check is out of tolerance"},
    "validate": {2: "input error", 4: "a method misplaces more than 10% of labelled spend"},
    "report": {2: "input error", 3: "privacy gate refused the report"},
    "pr_stats": {2: "a gh call failed, or three in a row stopped the sweep", 3: "privacy gate refused a comment"},
}


def default_home():
    if os.environ.get("TOKEN_PROFILE_HOME"):
        return Path(os.environ["TOKEN_PROFILE_HOME"])
    base = os.environ.get("LOCALAPPDATA") or os.path.expanduser("~/.local/share")
    return Path(base) / "cimmeria-token-profile"


def default_db():
    return Path(os.environ.get("TOKEN_PROFILE_DB") or Path.home() / "token-profile.sqlite")


def utc_midnight(now):
    return now.astimezone(timezone.utc).replace(hour=0, minute=0, second=0, microsecond=0)


def iso(t):
    return t.strftime("%Y-%m-%dT%H:%M:%SZ")


def weekly_window(now, days=7, until=None):
    """(since, until) as ISO UTC: the `days` whole UTC days before `until` (default today's UTC midnight)."""
    end = (datetime.strptime(until, "%Y-%m-%d").replace(tzinfo=timezone.utc) if until else utc_midnight(now))
    return iso(end - timedelta(days=days)), iso(end)


def sweep_since(now, days=3):
    """The first merge date the sweep covers: `days` UTC days before today, so a run sees its days in full."""
    return (utc_midnight(now) - timedelta(days=days)).strftime("%Y-%m-%d")


class Busy(Exception):
    pass


class Lock:
    """One job at a time per home: both jobs write the database."""

    def __init__(self, path, clock=time.time):
        self.path, self.clock = Path(path), clock

    def __enter__(self):
        self.path.parent.mkdir(parents=True, exist_ok=True)
        try:
            fd = os.open(self.path, os.O_CREAT | os.O_EXCL | os.O_WRONLY)
        except FileExistsError:
            if self.clock() - self.path.stat().st_mtime < STALE_LOCK_SECONDS:
                raise Busy(f"another job holds {self.path.name}") from None
            self.path.unlink()
            fd = os.open(self.path, os.O_CREAT | os.O_EXCL | os.O_WRONLY)
        os.write(fd, str(os.getpid()).encode())
        os.close(fd)
        return self

    def __exit__(self, *exc):
        self.path.unlink(missing_ok=True)


class Steps:
    """Runs the profiler's commands one by one, logging each and recording its outcome."""

    def __init__(self, log_dir, emit=print, run=subprocess.run):
        self.log_dir, self.emit, self.run = Path(log_dir), emit, run
        self.results = []
        self.summary_name = "summary.txt"
        self.log_dir.mkdir(parents=True, exist_ok=True)

    def step(self, name, tool, args, log_name=None):
        argv = [sys.executable, str(TOOLS / tool), *[str(a) for a in args]]
        log = self.log_dir / f"{log_name or name}.log"
        with open(log, "w", encoding="utf-8") as fh:
            try:
                code = self.run(argv, stdout=fh, stderr=subprocess.STDOUT, cwd=TOOLS, timeout=3 * 3600).returncode
            except (OSError, subprocess.SubprocessError) as e:
                fh.write(f"\nstatus=error could not run: {e}\n")
                code = -1
        why = "ok" if code == 0 else MEANING.get(tool, {}).get(code, f"crashed, see {log.name}")
        self.results.append((name, code, why))
        self.emit(f"step={name} exit={code} {why}")
        return code

    def failed(self):
        return [(n, c, w) for n, c, w in self.results if c != 0]

    def summary(self, job):
        failed = self.failed()
        lines = [f"job={job} finished={iso(datetime.now(timezone.utc))}"]
        lines += [f"step={n} exit={c} {w}" for n, c, w in self.results]
        lines.append("status=ok" if not failed else
                     "status=failed " + ", ".join(f"{n} ({w})" for n, _, w in failed))
        return "\n".join(lines) + "\n"


def weekly(args, now, steps_cls=Steps):
    since, until = weekly_window(now, args.days, args.until)
    out = args.home / "reports" / until[:10]
    steps = steps_cls(out)
    steps.summary_name = "weekly-summary.txt"
    steps.emit(f"job=weekly window={since}..{until} out={out}")
    if not args.skip_ingest:
        steps.step("ingest", "ingest", ["--db", args.db, "--repo", args.repo, "--fetch-prs"])
    steps.step("reconcile", "reconcile", ["--db", args.db, "--out", out, "--since", since, "--until", until])
    validate = ["--db", args.db, "--out", out / "attribution-check.json", "--max-wrong-share", MAX_WRONG_SHARE]
    if args.labels:
        validate += ["--labels", args.labels]
    steps.step("validate", "validate", validate)
    steps.step("report", "report", ["--db", args.db, "--out", out, "--since", since, "--until", until])
    return steps


def pr_sweep(args, now, steps_cls=Steps):
    since = sweep_since(now, args.days)
    day = utc_midnight(now).strftime("%Y-%m-%d")
    steps = steps_cls(args.home / "logs")
    steps.summary_name = f"pr-sweep-{day}-summary.txt"
    steps.emit(f"job=pr-sweep merged_since={since} post={not args.dry_run}")
    if not args.skip_ingest:
        steps.step("ingest", "ingest", ["--db", args.db, "--repo", args.repo, "--fetch-prs"],
                   log_name=f"pr-sweep-{day}-ingest")
    # --restart: every PR in the window is looked at again, so a comment the
    # latest ingest made stale is edited in place; a current one is left alone.
    sweep = ["--backfill", "--since", since, "--rate", args.rate, "--restart", "--db", args.db,
             "--state", args.home / "pr-sweep-state.json"]
    if not args.dry_run:
        sweep.append("--post")
    steps.step("pr_stats", "pr_stats", sweep, log_name=f"pr-sweep-{day}")
    return steps


def parser():
    common = argparse.ArgumentParser(add_help=False)
    common.add_argument("--home", type=Path, help="where reports, logs and state go (outside the repo)")
    common.add_argument("--db", type=Path, help="the profiler database")
    common.add_argument("--repo", type=Path, help="the checkout the ingest reads (default: the main one)")
    common.add_argument("--skip-ingest", action="store_true", help="use the database as it is")
    ap = argparse.ArgumentParser(prog="token-profile scheduled", description=__doc__.split("\n\n")[0])
    sub = ap.add_subparsers(dest="job", required=True)
    w = sub.add_parser("weekly", parents=[common], help="ingest, reconcile, validate and report the last --days days")
    w.add_argument("--days", type=int, default=7, help="days in the window (default: %(default)s)")
    w.add_argument("--until", help="first UTC day excluded, YYYY-MM-DD (default: today)")
    w.add_argument("--labels", default=os.environ.get("TOKEN_PROFILE_LABELS"),
                   help="validate's labels CSV (default: TOKEN_PROFILE_LABELS, else none)")
    s = sub.add_parser("pr-sweep", parents=[common],
                       help="ingest, then post or refresh the stats comment of recently merged PRs")
    s.add_argument("--days", type=int, default=3, help="PRs merged in the last N UTC days (default: %(default)s)")
    s.add_argument("--rate", default="6/min", help="pr_stats' --rate (default: %(default)s)")
    s.add_argument("--dry-run", action="store_true", help="build the comments but post nothing")
    return ap


def main(argv=None, now=None, steps_cls=Steps):
    ap = parser()
    args = ap.parse_args(argv)
    if args.days < 1:
        ap.error("--days must be at least 1")
    if getattr(args, "until", None):
        try:
            datetime.strptime(args.until, "%Y-%m-%d")
        except ValueError:
            ap.error("--until must be YYYY-MM-DD")
    args.home = args.home or default_home()
    args.db = args.db or default_db()
    args.repo = args.repo or main_checkout(TOOLS)
    now = now or datetime.now(timezone.utc)
    try:
        with Lock(args.home / "job.lock"):
            steps = (weekly if args.job == "weekly" else pr_sweep)(args, now, steps_cls)
    except Busy as e:
        print(f"status=busy {e}", file=sys.stderr)
        return 2
    summary = steps.summary(args.job)
    (steps.log_dir / steps.summary_name).write_text(summary, encoding="utf-8")
    # The step lines were printed as they ran; the summary file has them all.
    print(summary.splitlines()[-1])
    return 1 if steps.failed() else 0
