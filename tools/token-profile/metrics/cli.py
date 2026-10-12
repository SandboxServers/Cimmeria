"""Command line: campaign metrics. `registry` checks the campaign rules; `ledger` parses the packet tables of the ledgers;
`cost` prices the profiler's requests per campaign (CM-03).

    python tools/token-profile/metrics registry --repo . [--check] [--config <json>] [--campaigns <json>]
    python tools/token-profile/metrics ledger --repo . --out <dir>
    python tools/token-profile/metrics cost --db <profile.sqlite> --out <dir> [--config <json>] [--ledger <json>] [--since T] [--until T]

Exit status: 0 ok; 1 with registry --check, the campaign rules have a problem; 2 an input or output error;
3 the privacy gate refused the output, nothing was written.
The ledger writes <dir>/ledger.json and prints a JSON count summary. `cost` opens the profiler database read-only
and writes cost.json to --out. --ledger is the CM-05 ledger.json: its "pr_map" ({"<pr>": "<campaign>"}) and, per
campaign, the packets whose status is an accepted status.
"""

import argparse
import json
import sqlite3
import subprocess
import sys
from pathlib import Path

from ingest.prices import CURRENT, SOURCE
from report.db import ReportError, open_db
from report.scrub import PrivacyError, Scrubber

from . import cost, ledger, registry


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
             "size": "", "wave": "", "depends": "", "override": True})
        for pr in prs:
            if pr in pr_map and pr_map[pr] != campaign:
                conflicts.append({"pr": pr, "campaigns": sorted([pr_map.pop(pr), campaign])})
            else:
                pr_map[pr] = campaign
    return {
        "schema": "cimmeria-campaign-ledger/1",
        "campaigns": {name: {"packets": rows, "window": ledger.campaign_window(name, git)}
                      for name, rows in rows_by_campaign.items()},
        "pr_map": pr_map,
        "conflicts": conflicts,
    }


def summary(result):
    """Counts only: packets per campaign, packets with an unknown status, PRs cited by two campaigns.

    A PR override is not a packet, so it counts nowhere here.
    """
    packets = [p for c in result["campaigns"].values() for p in c["packets"] if not p.get("override")]
    return {"campaigns": {name: sum(not p.get("override") for p in c["packets"])
                          for name, c in result["campaigns"].items()},
            "unknown_status": sum(p["status"] == "unknown" for p in packets),
            "conflicts": len(result["conflicts"])}


def run_ledger(args):
    """Write the ledger for `args.repo` to `args.out` and print its summary; the exit status is 0, or 2 on an error."""
    failures = []
    try:
        pr_overrides = registry.load_config(args.config).get("pr_overrides", {})
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
    led.add_argument("--config", help="metrics config.json (default: the package's)")
    cst = sub.add_parser("cost", help="list-price cost per campaign, role and PR (read-only database)")
    cst.add_argument("--db", required=True, help="the profiler's SQLite database")
    cst.add_argument("--out", required=True, help="directory for cost.json")
    cst.add_argument("--config", help="metrics config.json (default: the package's)")
    cst.add_argument("--ledger", help="the CM-05 ledger.json: pr_map and each campaign's packets")
    cst.add_argument("--since", help="first timestamp included (default: config.json's window)")
    cst.add_argument("--until", help="first timestamp excluded (default: config.json's window)")
    args = ap.parse_args(argv)
    if args.cmd == "ledger":
        return run_ledger(args)
    if args.cmd == "cost":
        return cmd_cost(args)
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


def load_ledger(path, accepted_statuses):
    """The CM-05 ledger.json as ({pr: campaign}, {campaign: packets}). A packet counts when its status is in
    `accepted_statuses`; override rows are not packets. JSON keys are strings; PR numbers go to int."""
    data = json.loads(Path(path).read_text(encoding="utf-8"))
    pr_map = {int(pr): name for pr, name in data["pr_map"].items()}
    accepted = {name: sum(1 for p in c["packets"] if not p.get("override") and p["status"] in accepted_statuses)
                for name, c in data["campaigns"].items()}
    return pr_map, accepted


def cmd_cost(args):
    """Write cost.json: each campaign's list-price cost, plus the stamp. Input errors exit 2, the privacy gate 3."""
    try:
        config = registry.load_config(args.config)
        since = args.since or config["window"]["since"]
        until = args.until or config["window"]["until"]
        roles = config["roles"]
        pr_map, accepted = load_ledger(args.ledger, config["accepted_statuses"]) if args.ledger else (None, None)
        db = open_db(args.db)
    except (OSError, ValueError, KeyError, TypeError, AttributeError, ReportError) as e:
        print(f"status=error {type(e).__name__}", file=sys.stderr)
        return 2
    print(f"prices {CURRENT} source {SOURCE}", file=sys.stderr)
    try:
        campaigns = cost.rollup(cost.load_requests(db, since, until, pr_map), CURRENT, roles, accepted)
        record = {"stamp": cost.stamp(db, since, until, pr_map, CURRENT), "campaigns": campaigns}
    except sqlite3.Error as e:
        print(f"status=error {type(e).__name__}", file=sys.stderr)
        return 2
    finally:
        db.close()
    text = json.dumps(record, indent=2, sort_keys=True) + "\n"
    try:
        Scrubber().assert_clean(text, "cost")
    except PrivacyError as e:
        print(f"status=refused {e}", file=sys.stderr)
        return 3
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    (out / "cost.json").write_text(text, encoding="utf-8", newline="\n")
    total = sum(c["cost"]["total_usd"] for c in campaigns.values())
    print(f"status=ok wrote cost.json campaigns={len(campaigns)} total_usd={total:.4f}")
    return 0
