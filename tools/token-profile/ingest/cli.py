"""Command line: ingest Claude Code transcripts into the profiler database.

    python tools/token-profile/ingest --db <file.sqlite> [--repo <checkout>] [--fetch-prs | --prs-json <file>]

Exit status: 0 ok; 1 unknown transcript shapes are in the database (pass
--allow-unknown to accept them); 2 attribution_imbalance is not empty.
The summary printed holds counts only, never transcript content.
"""

import argparse
import json
import os
import subprocess
import sys
from pathlib import Path

from . import prices
from . import store
from .attribution import Attributor
from .gitsources import Ancestry, fetch_prs, lane_log, merge_subjects, snapshot_refs, store_prs

HERE = Path(__file__).resolve().parent


def profiler_commit():
    try:
        p = subprocess.run(["git", "-C", str(HERE), "rev-parse", "--short=12", "HEAD"], capture_output=True,
                           text=True)
        return p.stdout.strip() or "unknown"
    except OSError:
        return "unknown"


def default_lane_log():
    base = os.environ.get("LANE_ROOT") or os.path.join(
        os.environ.get("LOCALAPPDATA") or os.path.expanduser("~/.local/share"), "cimmeria-build")
    return Path(base) / "metrics" / "jobs.jsonl"


def main_checkout(repo):
    """The main checkout of repo (a worktree's common dir parent): its path names the project dirs."""
    p = subprocess.run(["git", "-C", str(repo), "rev-parse", "--path-format=absolute", "--git-common-dir"],
                       capture_output=True, text=True)
    common = p.stdout.strip()
    return Path(common).parent if p.returncode == 0 and common else Path(repo).resolve()


def run(args):
    db = store.open_db(args.db)
    try:
        return _run(db, args)
    finally:
        db.close()


def _run(db, args):
    stats = store.new_stats()
    run_id = db.execute(
        "INSERT INTO profiler_runs (started_at, profiler_commit, price_table, status) VALUES (?, ?, ?, 'running')",
        (store.now_iso(), args.profiler_commit or profiler_commit(), prices.CURRENT)).lastrowid
    db.commit()
    prices.store(db)

    prefix = args.project_prefix or store.project_prefix(main_checkout(args.repo or HERE))
    touched = set()
    files = store.discover(args.projects, prefix)
    for kind, path, project_dir, session_id, agent_id in files:
        store.ingest_file(db, run_id, kind, path, project_dir, session_id, agent_id, stats, touched)
        db.commit()
    store.finalize(db, touched)

    if args.prs_json:
        stats["prs"] = store_prs(db, json.loads(Path(args.prs_json).read_text(encoding="utf-8")))
    elif args.fetch_prs:
        stats["prs"] = store_prs(db, fetch_prs(args.repo or HERE))
    ancestry = None
    if args.repo:
        stats["heads_ref_snapshot"] = snapshot_refs(db, args.repo)
        stats["heads_merge_subject"] = merge_subjects(db, args.repo)
        ancestry = Ancestry(args.repo)
    lane = Path(args.lane_log) if args.lane_log else default_lane_log()
    if lane.exists():
        stats["heads_lane_log"] = lane_log(db, lane)

    attributor = Attributor(db, ancestry)
    imbalance = attributor.run()
    unknown = db.execute("SELECT COALESCE(SUM(count), 0) FROM unknown_shapes").fetchone()[0]
    failed = imbalance > 0 or (unknown > 0 and not args.allow_unknown)
    db.execute("UPDATE profiler_runs SET finished_at = ?, files_scanned = ?, records_read = ?, unknown_records = ?,"
               " status = ? WHERE run_id = ?",
               (store.now_iso(), stats["files_scanned"], stats["records_read"], stats["unknown_records"],
                "failed" if failed else "ok", run_id))
    db.commit()

    totals = {k: db.execute(f"SELECT COUNT(*) FROM {k}").fetchone()[0]
              for k in ("sessions", "agents", "requests", "triggers", "tool_calls", "compactions", "pr_links")}
    unpriced = sorted(m for (m,) in db.execute("SELECT DISTINCT model FROM requests")
                      if prices.price_for(m) is None)
    summary = {"run_id": run_id, "files_found": len(files), "this_run": dict(sorted(stats.items())),
               "totals": totals, "attribution": dict(sorted(attributor.stats.items())),
               "unknown_records_total": unknown, "attribution_imbalance": imbalance, "unpriced_models": unpriced}
    if not args.quiet:
        print(json.dumps(summary, indent=1))
    if imbalance:
        print(f"error: {imbalance} requests have attribution weights that do not sum to 1", file=sys.stderr)
        return 2
    if unknown and not args.allow_unknown:
        rows = db.execute("SELECT shape, cc_version, count FROM unknown_shapes ORDER BY count DESC").fetchall()
        print("error: unknown transcript shapes (update transcript-format.md and the ingest, or pass"
              " --allow-unknown):", file=sys.stderr)
        for shape, version, count in rows:
            print(f"  {shape} (Claude Code {version or '?'}): {count}", file=sys.stderr)
        return 1
    return 0


def parser():
    p = argparse.ArgumentParser(prog="ingest", description=__doc__.split("\n\n")[0])
    p.add_argument("--db", required=True, help="SQLite database to create or update (never commit it)")
    p.add_argument("--projects", type=Path, default=store.default_projects_root(),
                   help="Claude Code projects directory (default ~/.claude/projects)")
    p.add_argument("--project-prefix", help="ingest project dirs whose name starts with this"
                   " (default: derived from the main checkout's path)")
    p.add_argument("--repo", type=Path, help="checkout to read branch heads and ancestry from")
    g = p.add_mutually_exclusive_group()
    g.add_argument("--prs-json", help="output of `gh pr list --state all --json " + "...` to load into prs")
    g.add_argument("--fetch-prs", action="store_true", help="run `gh pr list` in --repo")
    p.add_argument("--lane-log", help="build lane jobs.jsonl (default: the lane's own, if present)")
    p.add_argument("--allow-unknown", action="store_true", help="exit 0 even with unknown shapes recorded")
    p.add_argument("--profiler-commit", help=argparse.SUPPRESS)
    p.add_argument("--quiet", action="store_true")
    return p


def main(argv=None):
    return run(parser().parse_args(argv))
