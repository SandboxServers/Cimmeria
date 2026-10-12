"""Command line: campaign metrics. One subcommand so far, `ledger` (packet tables of the campaign ledgers).

    python tools/token-profile/metrics ledger --repo . --out <dir>

Writes <dir>/ledger.json and prints a JSON count summary. Exit status: 0 written; 2 an input or output error.
"""

import argparse
import json
import subprocess
import sys
from pathlib import Path

from . import ledger


def git_log(repo):
    """A runner for `git log` in `repo`: its stdout, or "" when git cannot run there."""
    def run(args):
        try:
            return subprocess.run(["git", "-C", str(repo), *args], capture_output=True, text=True,
                                  encoding="utf-8", errors="replace", check=True).stdout
        except (OSError, subprocess.CalledProcessError):
            return ""
    return run


def build_ledger(repo, git):
    """The packets of every campaign folder under docs/analysis, their PR map, and each campaign's window."""
    root = Path(repo) / "docs" / "analysis"
    rows_by_campaign = {}
    for folder in sorted(p for p in root.iterdir() if p.is_dir()):
        files = sorted([*folder.glob("README.md"), *folder.glob("work-packets*.md")])
        rows = [ledger.normalize(r) for f in files for r in ledger.parse_packet_table(f.read_text(encoding="utf-8"))]
        if rows:
            rows_by_campaign[folder.name] = rows
    pr_map, conflicts = ledger.campaign_pr_map(rows_by_campaign)
    return {
        "schema": "cimmeria-campaign-ledger/1",
        "campaigns": {name: {"packets": rows, "window": ledger.campaign_window(name, git)}
                      for name, rows in rows_by_campaign.items()},
        "pr_map": pr_map,
        "conflicts": conflicts,
    }


def summary(result):
    """Counts only: packets per campaign, packets with an unknown status, PRs cited by two campaigns."""
    packets = [p for c in result["campaigns"].values() for p in c["packets"]]
    return {"campaigns": {name: len(c["packets"]) for name, c in result["campaigns"].items()},
            "unknown_status": sum(p["status"] == "unknown" for p in packets),
            "conflicts": len(result["conflicts"])}


def main(argv=None):
    ap = argparse.ArgumentParser(prog="metrics", description=__doc__.split("\n\n")[0])
    sub = ap.add_subparsers(dest="command", required=True)
    led = sub.add_parser("ledger", help="parse the packet tables of the campaign ledgers")
    led.add_argument("--repo", required=True, help="the repository root")
    led.add_argument("--out", required=True, help="directory for ledger.json")
    args = ap.parse_args(argv)
    try:
        result = build_ledger(args.repo, git_log(args.repo))
        out = Path(args.out)
        out.mkdir(parents=True, exist_ok=True)
        (out / "ledger.json").write_text(json.dumps(result, indent=1) + "\n", encoding="utf-8")
    except OSError as e:
        print(f"status=error {type(e).__name__}", file=sys.stderr)
        return 2
    print(json.dumps(summary(result), indent=1))
    return 0
