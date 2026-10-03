"""Command line: reconcile the profiler against Claude Code's cost-state records and OTel events.

    python tools/token-profile/reconcile --db <profile.sqlite> [--out <dir>] [--since ISO] [--until ISO]
        [--otel <events.json>] [--price-table VERSION] [--deny WORD ...]

Exit status: 0 every check within tolerance; 4 a check failed; 3 the privacy
gate refused the output (nothing written); 2 an input error.
"""

import argparse
import json
import sys
from pathlib import Path

from report import db as dbmod
from report.scrub import PrivacyError, Scrubber

from . import cost_state, otel, render


def build(db_path, since=None, until=None, price_table=None, otel_path=None, sc=None):
    sc = sc or Scrubber()
    db = dbmod.open_db(db_path)
    try:
        chosen = dbmod.scope(db, since, until, price_table)
        result = {"schema": "cimmeria-token-reconcile/1",
                  "window": {"since": since, "until": until}, "price_table": sc.label(chosen, "price_table"),
                  "cost_state": cost_state.build(db, sc, since, until)}
        if otel_path:
            cs = {r[0]: r[1] for r in db.execute("SELECT session_id, total_cost_usd FROM cost_states")}
            result["otel"] = otel.compare(db, sc, otel.load(otel_path), cs)
    finally:
        db.close()
    result["ok"] = result["cost_state"]["ok"] and result.get("otel", {"ok": True})["ok"]
    return sc.obj(result)


def main(argv=None):
    ap = argparse.ArgumentParser(prog="token-profile reconcile", description=__doc__.split("\n\n")[0])
    ap.add_argument("--db", required=True, help="the profiler's SQLite database")
    ap.add_argument("--out", help="directory for reconcile.md and reconcile.json (default: print a summary only)")
    ap.add_argument("--since", help="first process start included (ISO-8601 UTC)")
    ap.add_argument("--until", help="first process start excluded (ISO-8601 UTC)")
    ap.add_argument("--otel", help="claude_code.api_request events: OTLP JSON, a saved SigNoz result, or JSON Lines")
    ap.add_argument("--price-table", help="price_tables.version to use (default: the last ingest's)")
    ap.add_argument("--deny", action="append", default=[], help="a word the output may not contain (repeatable)")
    args = ap.parse_args(argv)
    sc = Scrubber(deny=args.deny)
    try:
        result = build(args.db, args.since, args.until, args.price_table, args.otel, sc)
        md = render.markdown(result)
        js = json.dumps(result, indent=1) + "\n"
        sc.assert_clean(md, "Markdown reconciliation")
        sc.assert_clean(js, "JSON reconciliation")
    except PrivacyError as e:
        print(f"status=refused {e}", file=sys.stderr)
        return 3
    except (dbmod.ReportError, OSError, ValueError) as e:
        print(f"status=error {e}", file=sys.stderr)
        return 2
    if args.out:
        out = Path(args.out)
        out.mkdir(parents=True, exist_ok=True)
        (out / "reconcile.md").write_text(md, encoding="utf-8", newline="\n")
        (out / "reconcile.json").write_text(js, encoding="utf-8", newline="\n")
    failed = [f"{src}.{c['name']}" for src in ("cost_state", "otel") if src in result
              for c in result[src]["checks"] if not c["ok"]]
    print(f"status={'ok' if not failed else 'out-of-tolerance'}"
          + (f" failed={','.join(failed)}" if failed else "")
          + f" profiler_vs_cost_state={result['cost_state']['totals']['gap_share'] * 100:+.1f}%")
    return 0 if not failed else 4
