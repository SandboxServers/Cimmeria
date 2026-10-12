"""Command line: campaign metrics. `registry` checks the campaign rules; `ledger` parses the packet tables of the
ledgers; `overstep` finds work a cheaper tier should have done; `cost` prices the profiler's requests per campaign
(CM-03); `github` collects PRs, CI and escaped defects from GitHub (read-only); `report` writes the campaign data and
reports from the five input files, with the campaign's own sessions priced in (CM-08k).

    python tools/token-profile/metrics registry --repo . [--check] [--config <json>] [--campaigns <json>]
    python tools/token-profile/metrics ledger --repo . --out <dir>
    python tools/token-profile/metrics packets --db <sqlite> --ledger <json> --github <json> --out <dir>
    python tools/token-profile/metrics overstep --db <sqlite> [--ledger <ledger.json>] --out <dir> [--config <json>]
    python tools/token-profile/metrics cost --db <profile.sqlite> --out <dir> [--config <json>] [--ledger <json>] [--since T] [--until T]
    python tools/token-profile/metrics github --repo . --out <dir> [--since 2026-09-21] [--config <json>]
        [--max-prs N] [--campaigns <file>] [--from-json <dir>]
    python tools/token-profile/metrics report --inputs <dir> --out <dir> [--db <sqlite> --self-session <id> ...] [--config <json>]

Exit status: 0 ok; 1 with registry --check, the campaign rules have a problem; 2 an input or output error;
3 the privacy gate refused the output, nothing was written.
The ledger writes <dir>/ledger.json and prints a JSON count summary; packets writes <dir>/packets.json.
Overstep writes <dir>/overstep.json and prints counts.
`cost` opens the profiler database read-only and writes cost.json to --out. --ledger is the CM-05 ledger.json: its
"pr_map" ({"<pr>": "<campaign>"}) and, per campaign, the packets whose status is an accepted status. The github
subcommand writes <dir>/github.json, a scratch file that is never committed, and prints a JSON count summary.
--from-json replays gh_prs.json, gh_issues.json and gh_checks.json from <dir> instead of calling gh.
"""

import argparse
import json
import sqlite3
import subprocess
import sys
from pathlib import Path

from ingest.campaigns import Campaigns
from ingest.prices import CURRENT, SOURCE
from report.db import ReportError, open_db
from report.scrub import PrivacyError, Scrubber
from reporting.inputs import FILES, load_inputs
from reporting.self_apply import apply_self_cost
from reporting.self_cost import load_self_cost
from reporting.write import build_outputs, write_outputs

from . import cost, ledger, overstep, packets, registry
from .github import GhError, collect, polite, replay, run_gh

OWN_CAMPAIGN = "campaign-metrics"


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

    `pr_overrides` ({campaign: [pr]}) adds PRs that no ledger table cites; each such campaign gets a pseudo-packet
    for them.
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


def overstep_counts(result):
    """Counts and USD (to cents): requests, hits, Opus subagents, and per campaign each rule's count, USD and wasted USD."""
    by_campaign = {}
    for name, stats in result["by_campaign"].items():
        rules = {rule: {"count": row["count"], "usd": round(row["usd"], 2), "wasted_usd": round(row["wasted_usd"], 2)}
                 for rule, row in stats["rules"].items()}
        by_campaign[name] = {"rules": rules, "recoverable_low": round(stats["recoverable_low"], 2),
                             "recoverable_high": round(stats["recoverable_high"], 2)}
    return {"requests": result["requests"], "hits": result["hits"], "opus_subagents": result["opus_subagents"],
            "by_campaign": by_campaign}


def run_overstep(args):
    """Detect overstep in `args.db` and write <out>/overstep.json; print counts only. Exit 0, or 2 on an error."""
    try:
        config = registry.load_config(args.config)
        ledger_doc = json.loads(Path(args.ledger).read_text(encoding="utf-8")) if args.ledger else {"campaigns": {}}
        packets, _windows, pr_map = overstep.ledger_inputs(ledger_doc)
        result = overstep.detect(overstep.load_requests(args.db), packets, config, pr_map)
        out = Path(args.out)
        out.mkdir(parents=True, exist_ok=True)
        (out / "overstep.json").write_text(json.dumps(result, indent=1) + "\n", encoding="utf-8")
    except (OSError, ValueError, KeyError, sqlite3.Error) as e:
        print(f"status=error {type(e).__name__}", file=sys.stderr)
        return 2
    print(json.dumps(overstep_counts(result), indent=1))
    return 0


def run_github(args):
    """Write github.json for `args.repo` to `args.out` and print its count summary; 0 written, 2 on an error."""
    stats = {"gh_calls": 0, "cache_hits": 0}
    cache_dir = (args.cache if args.cache is not None else Path(args.out) / "gh-cache") or None
    try:
        if args.from_json:
            run = replay(args.from_json)
        else:
            run = polite(lambda a: run_gh(a, cwd=args.repo), cache_dir=cache_dir, stats=stats)
        window = registry.load_config(args.config)["window"]
        campaigns = Campaigns(registry.load_campaigns(args.campaigns))
        pr_map = None
        if args.ledger:
            pr_map = {int(k): v for k, v in json.loads(Path(args.ledger).read_text(encoding="utf-8"))["pr_map"].items()}
        record = collect(run, args.since or window["since"][:10], campaigns, args.max_prs, window["until"], pr_map,
                         workers=args.workers)
    except (GhError, OSError, ValueError, KeyError) as e:
        print(f"status=error {e}", file=sys.stderr)
        return 2

    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    (out / "github.json").write_text(json.dumps(record, indent=2) + "\n", encoding="utf-8", newline="\n")
    summary = {
        "since": record["since"],
        "prs_collected": len(record["prs"]),
        "merged": sum(1 for r in record["prs"] if r["merged"]),
        "traced_prs": sum(1 for r in record["prs"] if r["ci"] is not None),
        "ci_checked": sum(1 for r in record["prs"] if r["ci"] and r["ci"]["runs"]),
        "queue_first_attempt_green": sum(
            1 for r in record["prs"] if r["queue"] and r["queue"]["first_attempt"] is True
        ),
        "queue_first_attempt_red": sum(
            1 for r in record["prs"] if r["queue"] and r["queue"]["first_attempt"] is False
        ),
        "untagged_prs": record["untagged_prs"],
        "escaped_defects": {c: len(rows) for c, rows in record["escaped_defects"].items()},
        "gh_calls": stats["gh_calls"],
        "cache_hits": stats["cache_hits"],
    }
    print(json.dumps(summary, indent=2))
    return 0


def run_packets(args):
    """Write the packet join to args.out/packets.json and print its counts; the exit status is 0, or 2 on an error."""
    try:
        ledger_doc = json.loads(Path(args.ledger).read_text(encoding="utf-8"))
        github_doc = json.loads(Path(args.github).read_text(encoding="utf-8"))
        config = registry.load_config(args.config)
        result = packets.join_campaigns(ledger_doc, packets.load_db(args.db), github_doc, config)
        out = Path(args.out)
        out.mkdir(parents=True, exist_ok=True)
        (out / "packets.json").write_text(json.dumps(result, indent=1) + "\n", encoding="utf-8")
    except (OSError, ValueError, KeyError, sqlite3.Error) as e:
        print(f"status=error {type(e).__name__}", file=sys.stderr)
        return 2
    print(json.dumps({"sessions": result["sessions"],
                      "packets": {name: len(c["packets"]) for name, c in result["campaigns"].items()}}))
    return 0


def run_report(args):
    """Write the campaign data and reports to `args.out`, pricing the own sessions in when `--self-session` names
    them. Prints counts only. Exit 0; 2 on an input or output error; 3 when the privacy gate refuses the output."""
    try:
        registry.load_config(args.config)
        if args.self_session and not args.db:
            print("status=error self-session-needs-db", file=sys.stderr)
            return 2
        inputs = load_inputs(args.inputs)
        if len(inputs["missing"]) == len(FILES):
            print("status=error no inputs found", file=sys.stderr)
            return 2
        self_cost = None
        if args.self_session:
            pr_numbers = [int(pr) for pr, name in inputs["ledger"].get("pr_map", {}).items() if name == OWN_CAMPAIGN]
            self_cost = load_self_cost(args.db, args.self_session, OWN_CAMPAIGN, pr_numbers)
            inputs = apply_self_cost(inputs, self_cost, OWN_CAMPAIGN)
        written = write_outputs(build_outputs(inputs, self_cost=self_cost), args.out)
    except (OSError, ValueError, KeyError, TypeError, AttributeError, sqlite3.Error, ReportError) as e:
        print(f"status=error {type(e).__name__}", file=sys.stderr)
        return 2
    except PrivacyError as e:
        print(f"status=refused {e}", file=sys.stderr)
        return 3
    print(json.dumps({"campaigns": sum(p.startswith("reports/") and p != "reports/summary.md" for p in written),
                      "files": len(written), "self_sessions": self_cost["sessions"] if self_cost else 0}))
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
    pk = sub.add_parser("packets", help="join worker sessions to packets: retries, first pass, escalation, stops")
    pk.add_argument("--db", required=True, help="the profiler database, opened read-only")
    pk.add_argument("--ledger", required=True, help="ledger.json from the ledger subcommand")
    pk.add_argument("--github", required=True, help="github.json with the PRs' merge times and commits")
    pk.add_argument("--out", required=True, help="directory for packets.json")
    pk.add_argument("--config", help="metrics config.json (default: the package's)")
    cst = sub.add_parser("cost", help="list-price cost per campaign, role and PR (read-only database)")
    cst.add_argument("--db", required=True, help="the profiler's SQLite database")
    cst.add_argument("--out", required=True, help="directory for cost.json")
    cst.add_argument("--config", help="metrics config.json (default: the package's)")
    cst.add_argument("--ledger", help="the CM-05 ledger.json: pr_map and each campaign's packets")
    cst.add_argument("--since", help="first timestamp included (default: config.json's window)")
    cst.add_argument("--until", help="first timestamp excluded (default: config.json's window)")
    gh = sub.add_parser("github", help="PRs, CI per head commit and escaped defects, read-only")
    gh.add_argument("--repo", required=True, help="the checkout gh reads the remote from")
    gh.add_argument("--out", required=True, help="scratch directory for github.json")
    gh.add_argument(
        "--since", default=None, help="PRs and issues created on or after this date (default: config window)"
    )
    gh.add_argument("--config", help="metrics config.json, whose window ends the collection (default: the package's)")
    gh.add_argument("--max-prs", type=int, default=None, help="collect CI for the first N PRs only (a trial)")
    gh.add_argument("--workers", type=int, default=1,
                    help="tagged PRs fetched in parallel (default 1: one at a time; the output is the same)")
    gh.add_argument("--campaigns", default=None, help="campaign tags; default ingest/campaigns.json")
    gh.add_argument("--ledger", default=None, help="ledger.json whose pr_map tags the PRs it names")
    gh.add_argument("--cache", default=None, metavar="DIR",
                    help="cache gh api answers in DIR so a failed run resumes (default OUT/gh-cache; \"\" turns it off; "
                         "scratch only, never committed)")
    gh.add_argument("--from-json", default=None, metavar="DIR", help="replay saved gh answers, no network")
    ovs = sub.add_parser("overstep", help="find overstep: work a cheaper tier should have done, and its cost")
    ovs.add_argument("--db", required=True, help="the token-profile database, opened read only")
    ovs.add_argument("--ledger", help="ledger.json from the ledger subcommand; without it no campaign is priced")
    ovs.add_argument("--out", required=True, help="directory for overstep.json")
    ovs.add_argument("--config", help="metrics config.json (default: the package's)")
    rep = sub.add_parser("report", help="campaign data and Markdown reports, with the own-session cost")
    rep.add_argument("--inputs", required=True, help="folder with the cost, ledger, github, packets and overstep .json")
    rep.add_argument("--out", required=True, help="directory for the data/ and reports/ folders")
    rep.add_argument("--db", help="the profiler's SQLite database, opened read-only (needed by --self-session)")
    rep.add_argument("--self-session", action="append", metavar="ID", help="a coordinator session to price (repeatable)")
    rep.add_argument("--config", help="metrics config.json (default: the package's)")
    args = ap.parse_args(argv)
    if args.cmd == "ledger":
        return run_ledger(args)
    if args.cmd == "packets":
        return run_packets(args)
    if args.cmd == "cost":
        return cmd_cost(args)
    if args.cmd == "github":
        return run_github(args)
    if args.cmd == "overstep":
        return run_overstep(args)
    if args.cmd == "report":
        return run_report(args)
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
