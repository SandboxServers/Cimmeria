"""Command line for the campaign metrics. Only the `github` subcommand so far (CM-04).

    python tools/token-profile/metrics github --repo . --out <dir> [--since 2026-09-21] [--max-prs N]
        [--campaigns <file>] [--from-json <dir>]

Writes <dir>/github.json, a scratch file that is never committed, and prints a JSON
count summary. --from-json replays gh_prs.json, gh_issues.json and gh_checks.json
from <dir> instead of calling gh. Exit codes: 0 written; 2 error (gh or the replay).
"""

import argparse
import json
import sys
from pathlib import Path

from ingest.campaigns import Campaigns

from .github import GhError, collect, polite, replay, run_gh

DEFAULT_SINCE = "2026-09-21"  # config.json window.since (CM-01)


def main(argv=None):
    ap = argparse.ArgumentParser(prog="metrics", description=__doc__.split("\n")[0])
    sub = ap.add_subparsers(dest="cmd", required=True)
    gh = sub.add_parser("github", help="PRs, CI per head commit and escaped defects, read-only")
    gh.add_argument("--repo", required=True, help="the checkout gh reads the remote from")
    gh.add_argument("--out", required=True, help="scratch directory for github.json")
    gh.add_argument("--since", default=DEFAULT_SINCE, help="PRs and issues created on or after this date")
    gh.add_argument("--max-prs", type=int, default=None, help="collect CI for the first N PRs only (a trial)")
    gh.add_argument("--campaigns", default=None, help="campaign tags; default ingest/campaigns.json")
    gh.add_argument("--from-json", default=None, metavar="DIR", help="replay saved gh answers, no network")
    args = ap.parse_args(argv)

    try:
        if args.from_json:
            run = replay(args.from_json)
        else:
            run = polite(lambda a: run_gh(a, cwd=args.repo))
        campaigns = Campaigns.load(args.campaigns)
        record = collect(run, args.since, campaigns, args.max_prs)
    except GhError as e:
        print(f"status=error {e}", file=sys.stderr)
        return 2

    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    (out / "github.json").write_text(json.dumps(record, indent=2) + "\n", encoding="utf-8", newline="\n")
    summary = {
        "since": record["since"],
        "prs_collected": len(record["prs"]),
        "merged": sum(1 for r in record["prs"] if r["merged"]),
        "ci_checked": sum(1 for r in record["prs"] if r["ci"]["runs"]),
        "escaped_defects": {c: len(rows) for c, rows in record["escaped_defects"].items()},
    }
    print(json.dumps(summary, indent=2))
    return 0
