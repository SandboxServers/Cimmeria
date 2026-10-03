"""Command line for the token profiler reports.

    python tools/token-profile/report --db <profile.sqlite> --out <dir> [--since ISO] [--until ISO]
        [--price-table VERSION] [--top N] [--deny WORD ...]

Writes token-report.md and token-report.json into --out, or nothing at all
if the privacy gate finds something (exit code 3).
"""

import argparse
import subprocess
import sys
from pathlib import Path

from .db import ReportError
from .generate import generate
from .scrub import PrivacyError


def report_commit():
    try:
        out = subprocess.run(["git", "rev-parse", "HEAD"], cwd=Path(__file__).resolve().parent,
                             capture_output=True, text=True, timeout=10)
        return out.stdout.strip() or None
    except (OSError, subprocess.SubprocessError):
        return None


def main(argv=None):
    ap = argparse.ArgumentParser(prog="token-profile report", description=__doc__.split("\n\n")[0])
    ap.add_argument("--db", required=True, help="the profiler's SQLite database")
    ap.add_argument("--out", required=True, help="directory for token-report.md and token-report.json")
    ap.add_argument("--since", help="first timestamp included (ISO-8601 UTC)")
    ap.add_argument("--until", help="first timestamp excluded (ISO-8601 UTC)")
    ap.add_argument("--price-table", help="price_tables.version to use (default: the last ingest's)")
    ap.add_argument("--top", type=int, default=20, help="rows in the top-N tables")
    ap.add_argument("--deny", action="append", default=[],
                    help="a word no report may contain, on top of the local user and machine names (repeatable)")
    args = ap.parse_args(argv)
    try:
        md, js = generate(args.db, args.since, args.until, args.price_table, report_commit(), args.deny, args.top)
    except PrivacyError as e:
        print(f"status=refused {e}", file=sys.stderr)
        return 3
    except ReportError as e:
        print(f"status=error {e}", file=sys.stderr)
        return 2
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    (out / "token-report.md").write_text(md, encoding="utf-8", newline="\n")
    (out / "token-report.json").write_text(js, encoding="utf-8", newline="\n")
    print(f"status=ok wrote token-report.md and token-report.json to {args.out}")
    return 0
