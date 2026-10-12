"""Command line: campaign metrics. `registry` checks the campaign rules; `ledger` parses the packet tables of the ledgers.

    python tools/token-profile/metrics registry --repo . [--check] [--config <json>] [--campaigns <json>]
    python tools/token-profile/metrics ledger --repo . --out <dir>

Exit status: 0 ok; 1 with registry --check, the campaign rules have a problem; 2 an input or output error.
The ledger writes <dir>/ledger.json and prints a JSON count summary.
"""

import argparse
import json
import subprocess
import sys
from pathlib import Path

from . import ledger, registry


def git_log(repo, failures):
    """A runner for `git log` in `repo`: its stdout. A run that fails returns "" and is appended to `failures`."""
    def run(args):
        try:
            return subprocess.run(["git", "-C", str(repo), *args], capture_output=True, text=True,
                                  encoding="utf-8", errors="replace", check=True).stdout
        except (OSError, subprocess.CalledProcessError):
            failures.append(args)
            return ""
    return run


def _one_per_id(rows):
    """One row per packet id: the first row with a status cell, or the first row when none has one."""
    kept = {}
    for row in rows:
        if row["id"] not in kept or (not kept[row["id"]]["status_raw"] and row["status_raw"]):
            kept[row["id"]] = row
    return list(kept.values())


def _retry_reason(campaign_dir, row):
    """The retry reason of an escalated packet or one with a worknote, else None. A worknote's first 60 lines count."""
    note = campaign_dir / "worknotes" / f"{row['id']}.md"
    if not (row["escalated"] or note.is_file()):
        return None
    head = "\n".join(note.read_text(encoding="utf-8").splitlines()[:60]) if note.is_file() else ""
    return ledger.retry_reason(f"{row['implementer_note'] or ''} {row['status_raw']} {head}")


def build_ledger(repo, git, pr_overrides=None):
    """The packets of every campaign folder under docs/analysis, their PR map, and each campaign's window.

    `pr_overrides` ({campaign: [pr]}) adds PRs that no ledger table cites; each such campaign gets a pseudo-packet for them.
    """
    root = Path(repo) / "docs" / "analysis"
    rows_by_campaign = {}
    for folder in sorted(p for p in root.iterdir() if p.is_dir()):
        files = sorted([*folder.glob("README.md"), *folder.glob("work-packets*.md")])
        rows = [ledger.normalize(r) for f in files for r in ledger.parse_packet_table(f.read_text(encoding="utf-8"))]
        rows = _one_per_id(rows)
        for row in rows:
            row["retry_reason"] = _retry_reason(folder, row)
        if rows:
            rows_by_campaign[folder.name] = rows
    pr_map, conflicts = ledger.campaign_pr_map(rows_by_campaign)
    for campaign, prs in (pr_overrides or {}).items():
        rows_by_campaign.setdefault(campaign, []).append(
            {"id": campaign, "title": "pr override", "status": "unknown", "status_raw": "", "prs": list(prs),
             "implementer": "other", "implementer_note": None, "escalated": False, "retry_reason": None,
             "size": "", "wave": "", "depends": ""})
        pr_map.update({pr: campaign for pr in prs})
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


def run_ledger(args):
    """Write the ledger for `args.repo` to `args.out` and print its summary; the exit status is 0, or 2 on an error."""
    failures = []
    try:
        pr_overrides = registry.load_config().get("pr_overrides", {})
        result = build_ledger(args.repo, git_log(args.repo, failures), pr_overrides)
        out = Path(args.out)
        out.mkdir(parents=True, exist_ok=True)
        (out / "ledger.json").write_text(json.dumps(result, indent=1) + "\n", encoding="utf-8")
    except (OSError, ValueError) as e:
        print(f"status=error {type(e).__name__}", file=sys.stderr)
        return 2
    if failures:
        print("status=warn git_unavailable", file=sys.stderr)
    print(json.dumps(summary(result), indent=1))
    return 0


def main(argv=None):
    """Run the subcommand and return its exit status."""
    ap = argparse.ArgumentParser(prog="token-profile metrics", description=__doc__.split("\n\n")[0])
    sub = ap.add_subparsers(dest="cmd", required=True)
    reg = sub.add_parser("registry", help="the campaign registry and regime config")
    reg.add_argument("--repo", required=True, help="the repository root; docs/analysis/ is read from it")
    reg.add_argument("--check", action="store_true", help="exit 1 if a campaign has no folder or two prefixes overlap")
    reg.add_argument("--config", help="metrics config.json (default: the package's)")
    reg.add_argument("--campaigns", help="campaigns.json (default: the ingest's)")
    led = sub.add_parser("ledger", help="parse the packet tables of the campaign ledgers")
    led.add_argument("--repo", required=True, help="the repository root")
    led.add_argument("--out", required=True, help="directory for ledger.json")
    args = ap.parse_args(argv)
    if args.cmd == "ledger":
        return run_ledger(args)
    try:
        config = registry.load_config(args.config)
        campaigns = registry.load_campaigns(args.campaigns)
        regime_switch = config["regime_switch"]
    except (OSError, ValueError, KeyError) as e:
        print(f"status=error {type(e).__name__}", file=sys.stderr)
        return 2
    found = registry.problems(campaigns, args.repo)
    names = [name for name in campaigns if not name.startswith("_")]
    print(json.dumps({"campaigns": len(names),
                      "prefixes": sum(len(campaigns[name].get("prefixes", ())) for name in names),
                      "regime_switch": regime_switch, "problems": len(found)}))
    for line in found:
        print(f"problem: {line}", file=sys.stderr)
    return 1 if args.check and found else 0
