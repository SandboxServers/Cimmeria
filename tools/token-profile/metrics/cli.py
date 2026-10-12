"""Command line: the campaign metrics subcommands: `registry` (CM-01) and `cost` (CM-03).

    python tools/token-profile/metrics registry --repo . [--check] [--config <json>] [--campaigns <json>]
    python tools/token-profile/metrics cost --db <profile.sqlite> --out <dir> [--ledger <json>] [--since T] [--until T]

`cost` opens the profiler database read-only and writes cost.json to --out. --ledger is the CM-05 output:
{"pr_map": {"<pr>": "<campaign>"}, "accepted": {"<campaign>": <packets>}}.

Exit status: 0 ok; 1 with registry --check, the campaign rules have a problem; 2 an input error;
3 the privacy gate refused the output, nothing was written.
"""

import argparse
import json
import sys
from pathlib import Path

from ingest.prices import CURRENT, SOURCE
from report.db import ReportError, open_db
from report.scrub import PrivacyError, Scrubber

from . import cost, registry


def main(argv=None):
    """Run the registry subcommand and return its exit status."""
    ap = argparse.ArgumentParser(prog="token-profile metrics", description=__doc__.split("\n\n")[0])
    sub = ap.add_subparsers(dest="cmd", required=True)
    reg = sub.add_parser("registry", help="the campaign registry and regime config")
    reg.add_argument("--repo", required=True, help="the repository root; docs/analysis/ is read from it")
    reg.add_argument("--check", action="store_true", help="exit 1 if a campaign has no folder or two prefixes overlap")
    reg.add_argument("--config", help="metrics config.json (default: the package's)")
    reg.add_argument("--campaigns", help="campaigns.json (default: the ingest's)")
    cst = sub.add_parser("cost", help="list-price cost per campaign, role and PR (read-only database)")
    cst.add_argument("--db", required=True, help="the profiler's SQLite database")
    cst.add_argument("--out", required=True, help="directory for cost.json")
    cst.add_argument("--ledger", help="the CM-05 ledger output: pr_map and accepted counts")
    cst.add_argument("--since", help="first timestamp included (default: config.json's window)")
    cst.add_argument("--until", help="first timestamp excluded (default: config.json's window)")
    args = ap.parse_args(argv)
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


def load_ledger(path):
    """The CM-05 ledger output as ({pr: campaign}, {campaign: packets}). JSON keys are strings; PR numbers go to int."""
    data = json.loads(Path(path).read_text(encoding="utf-8"))
    return {int(pr): name for pr, name in data.get("pr_map", {}).items()}, dict(data.get("accepted", {}))


def cmd_cost(args):
    """Write cost.json: each campaign's list-price cost, plus the stamp. Input errors exit 2, the privacy gate 3."""
    config = registry.load_config()
    since = args.since or config["window"]["since"]
    until = args.until or config["window"]["until"]
    try:
        pr_map, accepted = load_ledger(args.ledger) if args.ledger else (None, None)
        db = open_db(args.db)
    except (OSError, ValueError, KeyError, ReportError) as e:
        print(f"status=error {type(e).__name__}", file=sys.stderr)
        return 2
    print(f"prices {CURRENT} source {SOURCE}", file=sys.stderr)
    campaigns = cost.rollup(cost.load_requests(db, since, until, pr_map), CURRENT, config["roles"], accepted)
    record = {"stamp": cost.stamp(db, since, until, pr_map, CURRENT), "campaigns": campaigns}
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
