"""Coordination overhead: the spend and context the coordination roles carry, next to the whole.

The coordination roles are the main thread, architect, packet-reviewer and
packet-review-fixer. Requests are priced with the newest price table, the same
way the other reports price them, and the report reads the database only. It is
a plan-usage proxy, never a bill (D-TP1).
"""

from .db import open_db, scope
from .render import num, pct, table, usd
from .scrub import Scrubber
from .stats import distribution, share

MAIN = "main thread"
COORDINATION_AGENTS = ("architect", "packet-reviewer", "packet-review-fixer")

NOTE = ("The main thread's first-request context is the first request of each session in the window. "
        "It is the true first request only when the window holds the session's start.")


def is_coordination(role):
    return role == MAIN or role in COORDINATION_AGENTS


def build(db, sc, since=None, until=None, session=None):
    """The coordination report as a dict, before redaction. Prices with the newest price_tables version."""
    newest = db.execute("SELECT MAX(version) FROM price_tables").fetchone()[0]
    price_table = scope(db, since, until, newest)
    where, args = ("WHERE session_id = ?", (session,)) if session else ("", ())
    rows = [dict(r) for r in db.execute(
        "SELECT session_id, scope, agent_type, model, trigger_kind, ts, context_tokens, usd"
        f" FROM wcost {where} ORDER BY session_id, ts", args)]

    total_usd = sum(r["usd"] or 0 for r in rows)
    by_role = {}
    for r in rows:
        name = MAIN if r["scope"] == "main" else sc.label(r["agent_type"], "agent_type")
        acc = by_role.setdefault(name, {"requests": 0, "usd": 0.0, "context": 0, "models": set()})
        acc["requests"] += 1
        acc["usd"] += r["usd"] or 0
        acc["context"] += r["context_tokens"] or 0
        acc["models"].add(sc.label(r["model"], "model"))

    roles = [{"role": name, "models": sorted(acc["models"]), "requests": acc["requests"], "usd": acc["usd"],
              "share": share(acc["usd"], total_usd), "avg_context": acc["context"] / acc["requests"]}
             for name, acc in by_role.items()]
    roles.sort(key=lambda x: (x["role"] != MAIN, -x["usd"], x["role"]))

    firsts = {}
    for r in rows:
        if r["scope"] == "main":
            firsts.setdefault(r["session_id"], r["context_tokens"])
    main = by_role.get(MAIN, {"requests": 0, "context": 0})
    triggers = {}
    for r in rows:
        if r["scope"] == "main":
            kind = sc.label(r["trigger_kind"], "trigger_kind")
            triggers[kind] = triggers.get(kind, 0) + 1

    coord = [acc for name, acc in by_role.items() if is_coordination(name)]
    coord_usd = sum(acc["usd"] for acc in coord)
    return {
        "layer": "coordination overhead",
        "note": NOTE,
        "session": sc.label(session, "session") if session else None,
        "window": {"since": since, "until": until},
        "price_table": sc.label(price_table, "price_table"),
        "totals": {"requests": len(rows), "usd": total_usd,
                   "unpriced_requests": sum(1 for r in rows if r["usd"] is None)},
        "by_role": roles,
        "main_thread": {
            "first_request_context": distribution(list(firsts.values())),
            "avg_context": (main["context"] / main["requests"]) if main["requests"] else None,
        },
        "trigger_kinds": sorted(({"kind": k, "requests": v} for k, v in triggers.items()),
                                key=lambda x: (-x["requests"], x["kind"])),
        "coordination": {"roles": [MAIN, *COORDINATION_AGENTS], "requests": sum(a["requests"] for a in coord),
                         "usd": coord_usd, "share": share(coord_usd, total_usd)},
    }


def render(report):
    """Markdown for the terminal: the role table, the main thread, the trigger kinds and the share."""
    t = report["totals"]
    head = (f"Coordination overhead, {report['window']['since'] or 'start'} to "
            f"{report['window']['until'] or 'end'}"
            + (f", session {report['session']}" if report["session"] else "")
            + f", priced with {report['price_table']}.")
    rows = [[r["role"], ", ".join(r["models"]) or "-", num(r["requests"]), usd(r["usd"]), pct(r["share"]),
             num(r["avg_context"])] for r in report["by_role"]]
    main = report["main_thread"]
    first = main["first_request_context"]
    lines = [
        head,
        "",
        f"Requests {num(t['requests'])}, estimated list-price USD {usd(t['usd'])}, "
        f"unpriced requests {num(t['unpriced_requests'])}. Estimated list-price USD is a plan-usage proxy, not a bill (D-TP1).",
        "",
        "## Spend by role",
        "",
        table(["role", "model", "requests", "USD", "share", "avg context"], rows, numeric=False),
        "",
        "## Main thread",
        "",
        f"First-request context p50 {num(first['p50'])} over {num(first['n'])} sessions; "
        f"average context {num(main['avg_context'])}.",
        "",
        table(["trigger kind", "requests"], [[r["kind"], num(r["requests"])] for r in report["trigger_kinds"]],
              numeric=False),
        "",
        "## Coordination share",
        "",
        f"{', '.join(report['coordination']['roles'])}: {num(report['coordination']['requests'])} requests, "
        f"{usd(report['coordination']['usd'])}, {pct(report['coordination']['share'])} of the total.",
        "",
        report["note"],
        "",
    ]
    return "\n".join(lines)


def generate(db_path, since=None, until=None, session=None, deny=()):
    """Return the Markdown. Raises scrub.PrivacyError if the gate finds anything; ReportError on bad input."""
    sc = Scrubber(deny=deny, use_local=True)
    db = open_db(db_path)
    try:
        report = sc.obj(build(db, sc, since, until, session))
    finally:
        db.close()
    md = render(report)
    sc.assert_clean(md, "coordination report")
    return md
