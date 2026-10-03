"""Build a report from the profiler database and pass it through the privacy gate."""

import json

from . import db as dbmod
from . import render
from .scrub import Scrubber
from .sections import cache_sim, context, cost, prs, tokens, tools


def build(db, sc, since=None, until=None, price_table=None, report_commit=None, top=20):
    """The report as a dict, before redaction. Field validation happens here, through `sc`."""
    chosen = dbmod.scope(db, since, until, price_table)
    report = {
        "schema": "cimmeria-token-report/1",
        "stamp": None,
        "tokens": tokens.build(db, sc),
        "cost": cost.build(db, sc),
        "context": context.build(db, sc),
        "tools": tools.build(db, sc, top=top),
        "prs": prs.build(db, sc, top=top),
        "cache_policy": cache_sim.build(db, sc),
    }
    stamp = dbmod.stamp(db, chosen, report_commit, since, until)
    for key in ("price_table", "profiler_commit", "report_commit"):
        stamp[key] = sc.label(stamp[key], key)
    stamp["claude_code"] = {k: (sc.label(v, "cc_version") if not isinstance(v, list)
                                else [sc.label(x, "cc_version") for x in v])
                            for k, v in stamp["claude_code"].items()}
    stamp["models"] = [sc.label(m, "model") for m in stamp["models"]]
    stamp["rejected"] = dict(sorted(sc.rejected.items()))
    report["stamp"] = stamp
    return report


def generate(db_path, since=None, until=None, price_table=None, report_commit=None, deny=(), top=20,
             use_local_deny=True):
    """Return (markdown, json_text). Raises scrub.PrivacyError if the gate finds anything."""
    sc = Scrubber(deny=deny, use_local=use_local_deny)
    db = dbmod.open_db(db_path)
    try:
        report = sc.obj(build(db, sc, since, until, price_table, report_commit, top))
    finally:
        db.close()
    md = render.markdown(report)
    js = json.dumps(report, indent=1, ensure_ascii=False) + "\n"
    sc.assert_clean(md, "Markdown report")
    sc.assert_clean(js, "JSON report")
    return md, js
