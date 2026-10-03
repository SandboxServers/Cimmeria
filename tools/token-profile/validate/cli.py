"""Command line: score a profiler database's PR attribution against ground truth.

    python tools/token-profile/validate --db <profile.sqlite> [--labels <labels.csv>]
        [--truth-db <profile.sqlite>] [--out <file.json>]

Ground truth is derived from --truth-db (default --db), so an older database
can be scored on the same requests as a newer one. The output holds shares
and dollar totals only, never names or ids; the labels file and the
databases stay local. Exit status: 0 ok, 2 error.
"""

import argparse
import json
import sys
from pathlib import Path

from report import db as dbmod

from . import score, truth


def build(db_path, labels=None, truth_db_path=None):
    db = dbmod.open_db(db_path)
    tdb = dbmod.open_db(truth_db_path) if truth_db_path else db
    try:
        dbmod.scope(db)
        sets = {"authors": truth.from_authors(tdb)}
        unmatched = []
        if labels:
            sets["labels"], unmatched = truth.from_labels(tdb, truth.read_labels(labels))
            # A label is the stronger claim about an agent: drop those requests from the authors set.
            sets["authors"] = {r: p for r, p in sets["authors"].items() if r not in sets["labels"]}
        usd = dict(db.execute("SELECT request_id, COALESCE(usd, 0) FROM rcost"))
        return {
            "schema": "cimmeria-attribution-check/1",
            "schema_version": db.execute("SELECT value FROM meta WHERE key = 'schema_version'").fetchone()[0],
            "overall": score.overall(db, usd),
            "sets": {name: score.score(db, t, usd) for name, t in sets.items()},
            "labels_unmatched": len(unmatched),
        }
    finally:
        db.close()
        if tdb is not db:
            tdb.close()


def text(report):
    lines = [f"overall: ${report['overall']['usd']:,.0f}; " + ", ".join(
        f"{m} {s * 100:.1f}%" for m, s in report["overall"]["method_share"].items())]
    for name, s in report["sets"].items():
        u, r = s["usd"], s["requests"]
        lines.append(f"{name}: {s['labelled_requests']} requests, ${u['total']:,.0f}; by USD precision {u['precision']:.3f}"
                     f" recall {u['recall']:.3f} wrong {u['wrong_share']:.3f} campaign {u['campaign_share']:.3f}"
                     f" unattributed {u['unattributed_share']:.3f}; by requests precision {r['precision']:.3f}"
                     f" recall {r['recall']:.3f}")
        if s["wrong_usd_share_by_method"]:
            lines.append("  wrong by method (share of the set's USD): " + ", ".join(
                f"{m} {v * 100:.1f}%" for m, v in s["wrong_usd_share_by_method"].items()))
    if report["labels_unmatched"]:
        lines.append(f"labels that matched no agent: {report['labels_unmatched']}")
    return "\n".join(lines)


def main(argv=None):
    ap = argparse.ArgumentParser(prog="token-profile validate", description=__doc__.split("\n\n")[0])
    ap.add_argument("--db", required=True, help="the profiler database whose attribution is scored")
    ap.add_argument("--labels", help="CSV of name,date,pr rows (see validate/truth.py); stays local")
    ap.add_argument("--truth-db", help="derive ground truth from this database instead (same requests)")
    ap.add_argument("--out", help="also write the JSON report here")
    args = ap.parse_args(argv)
    try:
        report = build(args.db, args.labels, args.truth_db)
    except (dbmod.ReportError, OSError, ValueError) as e:
        print(f"status=error {e}", file=sys.stderr)
        return 2
    print(text(report))
    if args.out:
        Path(args.out).write_text(json.dumps(report, indent=1) + "\n", encoding="utf-8")
    return 0
