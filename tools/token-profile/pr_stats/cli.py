"""Command line for the per-PR stats comments.

    python tools/token-profile/pr_stats <PR> [--post] [--db FILE]
    python tools/token-profile/pr_stats --backfill [--since 2026-09-13] [--post] [--rate 6/min]

Without --post nothing is written to GitHub: one PR's comment is printed, and a
backfill prints one status line per PR (and the bodies into --out, if given).
With --post the PR's one stats comment is found by its marker and edited in
place, or created if there is none.

Exit codes: 0 printed, created, updated or unchanged; 2 error (database or
gh); 3 the privacy gate refused the comment, nothing was posted; 4 the database
has no request attributed to the PR, nothing was posted.
"""

import argparse
import sys
from pathlib import Path

from report.db import ReportError
from report.scrub import PrivacyError, Scrubber

from . import backfill as bf
from .block import NoData, comment, open_scoped
from .github import GhError, GitHub

EXIT = {"printed": 0, "created": 0, "updated": 0, "unchanged": 0, "error": 2, "refused": 3, "no-data": 4}
DEFAULT_DB = Path.home() / "token-profile.sqlite"
DEFAULT_REPO = "SandboxServers/Cimmeria"
DEFAULT_SINCE = "2026-09-13"  # D-TP5: the backfill covers PRs merged since then


def run_one(db, sc, gh, pr, price_table, post, before_gh=None, out_dir=None, err=None, out=None):
    """One PR: build, gate, then print or post. Returns the outcome (a key of EXIT)."""
    err = err or (lambda s: print(s, file=sys.stderr))
    out = out or print
    try:
        body, _ = comment(db, sc, pr, price_table, gh, before_gh)
    except NoData:
        err(f"pr={pr} status=no-data: no request is attributed to this PR; nothing was posted")
        return "no-data"
    except PrivacyError as e:
        err(f"pr={pr} status=refused {e}")
        return "refused"
    except GhError as e:
        err(f"pr={pr} status=error {e}")
        return "error"
    if out_dir:
        Path(out_dir).mkdir(parents=True, exist_ok=True)
        (Path(out_dir) / f"{pr}.md").write_text(body, encoding="utf-8", newline="\n")
    if not post:
        out(body)
        return "printed"
    try:
        status, cid, duplicates = gh.upsert(pr, body)
    except GhError as e:
        err(f"pr={pr} status=error {e}")
        return "error"
    note = f" duplicates={duplicates} (left alone)" if duplicates else ""
    err(f"pr={pr} status={status} comment={cid}{note}")
    return status


def main(argv=None, gh_run=None, limiter=None):
    ap = argparse.ArgumentParser(prog="token-profile pr_stats", description=__doc__.split("\n\n")[0])
    ap.add_argument("pr", nargs="?", type=int, help="the PR number (omit with --backfill)")
    ap.add_argument("--post", action="store_true", help="write the comment to GitHub (default: dry run)")
    ap.add_argument("--db", default=str(DEFAULT_DB), help="the profiler's SQLite database (default: %(default)s)")
    ap.add_argument("--repo", default=DEFAULT_REPO, help="owner/name (default: %(default)s)")
    ap.add_argument("--price-table", help="price_tables.version to use (default: the last ingest's)")
    ap.add_argument("--deny", action="append", default=[],
                    help="a word no comment may contain, on top of the local user and machine names (repeatable)")
    ap.add_argument("--out", help="also write each comment body to <out>/<PR>.md")
    bg = ap.add_argument_group("backfill")
    bg.add_argument("--backfill", action="store_true", help="every PR merged since --since, oldest first")
    bg.add_argument("--since", default=DEFAULT_SINCE, help="first merge date included (default: %(default)s)")
    bg.add_argument("--rate", default="6/min", help="PRs per minute, or N/h (default: %(default)s)")
    bg.add_argument("--state", help="resume file (default: next to --db)")
    bg.add_argument("--restart", action="store_true", help="ignore the resume file's earlier outcomes")
    bg.add_argument("--limit", type=int, help="stop after this many PRs")
    args = ap.parse_args(argv)
    if (args.pr is None) == (not args.backfill):
        ap.error("give either a PR number or --backfill")
    try:
        per_minute = bf.parse_rate(args.rate)
    except ValueError as e:
        ap.error(str(e))

    sc = Scrubber(deny=args.deny)
    gh = GitHub(args.repo, gh_run) if gh_run else GitHub(args.repo)
    try:
        db, price_table = open_scoped(args.db, args.price_table)
    except ReportError as e:
        print(f"status=error {e}", file=sys.stderr)
        return EXIT["error"]
    try:
        if not args.backfill:
            return EXIT[run_one(db, sc, gh, args.pr, price_table, args.post, out_dir=args.out)]

        mode = "post" if args.post else "dry-run"
        state = bf.State(args.state or Path(args.db).with_name(Path(args.db).name + ".pr-stats-state.json"))
        if args.restart:
            state.data["outcomes"].pop(mode, None)
        prs = bf.merged_prs(db, args.since)
        if args.limit is not None:
            prs = [p for p in prs if not state.done(mode, p)][:args.limit]
        print(f"status=start mode={mode} prs={len(prs)} rate={per_minute:g}/min", file=sys.stderr)

        def one(pr, before_gh):
            # A backfill dry run prints status lines only; the bodies go to --out.
            return run_one(db, sc, gh, pr, price_table, args.post, before_gh, out_dir=args.out,
                           out=lambda s: None)

        counts = bf.backfill(prs, one, limiter or bf.RateLimiter(per_minute), state, mode,
                             emit=lambda s: print(s, file=sys.stderr))
        print("status=done " + " ".join(f"{k}={v}" for k, v in sorted(counts.items())), file=sys.stderr)
        if counts.get("error") or counts.get("stopped"):
            return EXIT["error"]
        return EXIT["refused"] if counts.get("refused") else 0
    finally:
        db.close()
