"""Packet join (CM-06): each packet's worker sessions, retries, first pass, escalation, stop context and cycle time.

Pure functions take plain dicts and lists. `load_db` is the only database access: it opens the profiler database
read-only and reads the agents, their worktree's last branch and the agent_stops rows.
"""

import re
import sqlite3
from datetime import datetime, timezone
from pathlib import Path

from report.stats import percentile

SCHEMA = "cimmeria-campaign-packets/1"
CONTEXT_CUTOFF = 90_000
EXCLUDED_PREFIXES = ("Dropped", "Deferred", "Superseded", "Folded")
ERROR_KINDS = ("rate_limit", "authentication_failed", "other")
STOP_CLASSES = ("end_turn", "tool_use", "error", "no_record")
RETRY_REASONS = ("context-exhausted", "stalled-background", "permission-denied", "review-fix", "ci-failure",
                 "merge-conflict", "other")
_EPOCH = datetime.min.replace(tzinfo=timezone.utc)


def _ts(text):
    """The timestamp as an aware datetime, or None when it is missing. A trailing `Z` is UTC."""
    return datetime.fromisoformat(text.replace("Z", "+00:00")) if text else None


def _started(agent):
    """When the agent's first record is. The epoch when unknown, so an unknown start sorts first."""
    return _ts(agent.get("first_ts")) or _EPOCH


def _key(agent):
    """The agent's type: its custom agent type (.claude/agents/<x>.md), else its agent type."""
    return agent.get("custom_agent_type") or agent.get("agent_type") or ""


def role_of(agent, roles):
    """The campaign role of a worker from the config's `roles` map, or `other`."""
    return roles.get(_key(agent), "other")


def _role_name(agent, roles):
    """The role of a worker, or its agent type when the role map does not name it."""
    role = role_of(agent, roles)
    return _key(agent) if role == "other" else role


def _is_sonnet_implementer(agent, roles):
    """A Sonnet domain implementer or general-purpose agent: the targets of an escalation."""
    return agent.get("model_alias") == "sonnet" and (
        role_of(agent, roles) == "implementer-domain" or _key(agent) == "general-purpose")


def _packet_pattern(packet_id):
    """The pattern that finds a packet ID in a lowercased name, dash optional: `lr-10` and `lr10` both match `LR-10`.

    The end is bounded, so `lr-1` does not match inside `lr-10`.
    """
    return re.compile(r"(?<![a-z0-9])" + re.escape(packet_id.lower()).replace(r"\-", "-?") + r"(?![0-9])")


def _named(texts, ids):
    """The packet IDs the texts name. An ID that another named ID extends (TP-05 in TP-05b) names nothing of its own."""
    found = {i for i in ids if any(_packet_pattern(i).search(text) for text in texts)}
    return {i for i in found if not any(j != i and _packet_pattern(i).search(j.lower()) for j in found)}


def packet_of_session(agent, worktree, branch_hint, ledger_rows, campaign):
    """The packet ID a worker session ran for, or None.

    Matched in order: the worktree name or the branch hint (the campaign's `<campaign>/<id>-<slug>` branch, read
    without its campaign prefix), then the agent's name. A text that names two packets matches neither.
    """
    ids = [row["id"] for row in ledger_rows]
    branch = branch_hint or ""
    if branch.startswith(campaign + "/"):
        branch = branch[len(campaign) + 1:]
    tree = re.split(r"[\\/]", worktree or "")[-1]
    for texts in ((tree.lower(), branch.lower()), ((agent.get("name") or "").lower(),)):
        found = _named(texts, ids)
        if len(found) == 1:
            return found.pop()
        if found:
            return None
    return None


def stop_class(stop):
    """How a worker stopped, by the Contract's `by_stop` rule: end_turn, tool_use, error or no_record."""
    if stop is None:
        return "no_record"
    if stop["ended_in_api_error"]:
        return "error"
    if stop["last_stop_reason"] is None:
        return "no_record"
    return stop["last_stop_reason"] if stop["last_stop_reason"] in ("end_turn", "tool_use") else "error"


def stop_reason(stop):
    """The retry reason of a session by how it stopped.

    An API error, or no record, is `other`. A backgrounded last tool with no result is `stalled-background`. At least
    90k of context is `context-exhausted`. A refused tool call (a tool_use stop with an error result) is `permission-denied`.
    """
    if stop is None or stop["ended_in_api_error"]:
        return "other"
    if stop["last_tool_background"] and not stop["ended_after_result"]:
        return "stalled-background"
    if stop["last_context_tokens"] >= CONTEXT_CUTOFF:
        return "context-exhausted"
    if stop["last_result_is_error"] == 1:
        return "permission-denied"
    return "other"


def _fix_commits(pr, reviewers):
    """Commits on the PR head after the first reviewer started. None without a PR or its commit times; 0 with no reviewer."""
    if pr is None or pr.get("commits") is None:
        return None
    if not reviewers:
        return 0
    start = _started(reviewers[0])
    times = (_ts(c.get("committed")) for c in pr["commits"])
    return sum(1 for when in times if when and when > start)


def _escalation_target(coders, agents, roles):
    """The first Sonnet implementer that started after the first Haiku coder in one of the coders' worktrees, or None."""
    trees = {c["worktree"] for c in coders if c.get("worktree")}
    if not coders or not trees:
        return None
    later = [a for a in agents if a.get("worktree") in trees and _started(a) > _started(coders[0])
             and _is_sonnet_implementer(a, roles)]
    return min(later, key=_started, default=None)


def _join_packet(row, sessions, agents, stops, pr, roles, accepted):
    """One packet's record from its worker `sessions` (oldest first); `agents` is every worker, for escalation."""
    coders = [a for a in sessions if role_of(a, roles) == "implementer-haiku"]
    reviewers = [a for a in sessions if role_of(a, roles) == "reviewer"]
    fixers = [a for a in sessions if role_of(a, roles) == "fixer"]
    target = _escalation_target(coders, agents, roles)
    escalated = bool(row["escalated"]) or target is not None
    fix_commits = _fix_commits(pr, reviewers)
    first_pass = None
    if accepted and fix_commits is not None:
        first_pass = len(coders) == 1 and not fixers and fix_commits == 0 and not escalated
    starts = [when for when in (_ts(a.get("first_ts")) for a in sessions) if when]
    merged = _ts(pr.get("mergedAt")) if pr else None
    cycle = round((merged - min(starts)).total_seconds() / 3600, 2) if merged and starts else None
    return {"id": row["id"], "accepted": accepted, "implementer_sessions": len(coders),
            "reviewer_sessions": len(reviewers), "fixer_sessions": len(fixers), "fix_commits": fix_commits,
            "escalated": escalated, "escalation_to": _role_name(target, roles) if target else None,
            "why": stop_reason(stops.get(coders[-1]["agent_id"])) if coders and escalated else None,
            "retries": [stop_reason(stops.get(c["agent_id"])) for c in coders[:-1]],
            "first_pass": first_pass, "cycle_hours": cycle}


def _first_pass(records):
    """The first-pass rate over accepted packets whose first pass is known, and the accepted packets' contract fields."""
    accepted = [r for r in records if r["accepted"]]
    judged = [r for r in accepted if r["first_pass"] is not None]
    return {"rate": round(sum(r["first_pass"] for r in judged) / len(judged), 2) if judged else None,
            "eligible": len(judged),
            "packets": [{"id": r["id"], "implementer_sessions": r["implementer_sessions"],
                         "reviewer_sessions": r["reviewer_sessions"], "fix_commits": r["fix_commits"],
                         "first_pass": r["first_pass"]} for r in accepted]}


def _retries(records):
    """Retries over every packet, by the reason of the session that was retried."""
    by_reason = dict.fromkeys(RETRY_REASONS, 0)
    for record in records:
        for reason in record["retries"]:
            by_reason[reason] += 1
    return {"total": sum(by_reason.values()), "by_reason": by_reason}


def _escalation(records):
    """Escalations among the packets a Haiku coder was dispatched to; the items name the packets escalated."""
    dispatched = [r for r in records if r["implementer_sessions"]]
    escalated = [r for r in dispatched if r["escalated"]]
    return {"dispatched_to_haiku": len(dispatched), "escalated": len(escalated),
            "rate": round(len(escalated) / len(dispatched), 2) if dispatched else None,
            "items": [{"id": r["id"], "to": r["escalation_to"], "why": r["why"]} for r in escalated]}


def _stop_context(workers, stops):
    """How the workers stopped. The share above 90k is over the workers that have a stop row."""
    by_stop = dict.fromkeys(STOP_CLASSES, 0)
    by_kind = dict.fromkeys(ERROR_KINDS, 0)
    contexts = []
    for worker in workers:
        stop = stops.get(worker["agent_id"])
        by_stop[stop_class(stop)] += 1
        if stop_class(stop) == "error" and stop["ended_in_api_error"]:
            kind = stop["api_error_kind"]
            by_kind[kind if kind in ERROR_KINDS else "other"] += 1
        if stop is not None:
            contexts.append(stop["last_context_tokens"])
    ordered = sorted(contexts)
    return {"workers": len(workers), "by_stop": by_stop, "by_error_kind": by_kind,
            "context_at_stop": {"p50": _pct(ordered, 50), "p90": _pct(ordered, 90),
                                "max": ordered[-1] if ordered else None},
            "share_above_90k": round(sum(c >= CONTEXT_CUTOFF for c in ordered) / len(ordered), 2) if ordered else None}


def _pct(values, p):
    """The p-th percentile of sorted `values`, rounded to 2 places; None when there are none."""
    value = percentile(values, p)
    return None if value is None else round(value, 2)


def _cycle_time(records, window):
    """Campaign hours from the ledger window (None without a close-out commit) and the packets' cycle hours."""
    start, end = _ts(window.get("start")), _ts(window.get("end"))
    hours = sorted(r["cycle_hours"] for r in records if r["cycle_hours"] is not None)
    out = {"campaign_hours": round((end - start).total_seconds() / 3600, 2) if start and end else None}
    if out["campaign_hours"] is None:
        out["campaign_hours_gap"] = "no close-out commit in the ledger window" if start else "no campaign window"
    out["packet_hours"] = {"n": len(hours), "p50": _pct(hours, 50), "p90": _pct(hours, 90)}
    return out


def _campaign_out(campaign, records, workers, stops):
    """One campaign's contract fields, and its per-packet records."""
    return {"accepted": {"packets": sum(r["accepted"] for r in records)},
            "first_pass": _first_pass(records), "retries": _retries(records),
            "escalation": _escalation(records), "stop_context": _stop_context(workers, stops),
            "cycle_time": _cycle_time(records, campaign.get("window") or {}), "packets": records}


def _eligible_rows(packets):
    """The packet rows that count: not the PR overrides, and not dropped, deferred, superseded or folded."""
    return [r for r in packets if not r.get("override") and not r["status_raw"].startswith(EXCLUDED_PREFIXES)]


def _sessions_by_packet(name, rows, agents):
    """The worker sessions each packet of campaign `name` claims, by packet_of_session, oldest first."""
    by_packet = {}
    for agent in sorted(agents, key=_started):
        packet = packet_of_session(agent, agent.get("worktree"), agent.get("branch_hint"), rows, name)
        if packet is not None:
            by_packet.setdefault(packet, []).append(agent)
    return by_packet


def join_campaigns(ledger_doc, db, github_doc, config):
    """The packet join for every campaign of the ledger, and the share of worker sessions no packet claims.

    `db` is `load_db`'s result; `github_doc` is github.json (its `prs` carry `number`, `mergedAt` and, when known,
    `commits` as {sha, committed}); `config` holds `roles` and `accepted_statuses`.
    """
    roles, statuses = config["roles"], config["accepted_statuses"]
    prs = {pr["number"]: pr for pr in github_doc.get("prs", [])}
    agents, stops = db["agents"], db["stops"]
    campaigns = {}
    mapped = set()
    for name, campaign in ledger_doc["campaigns"].items():
        rows = _eligible_rows(campaign["packets"])
        by_packet = _sessions_by_packet(name, rows, agents)
        workers = [a for group in by_packet.values() for a in group]
        mapped |= {a["agent_id"] for a in workers}
        records = [_join_packet(row, by_packet.get(row["id"], []), agents, stops,
                                prs.get(row["prs"][0]) if row["prs"] else None, roles, row["status"] in statuses)
                   for row in rows]
        campaigns[name] = _campaign_out(campaign, records, workers, stops)
    total = len(agents)
    unmapped = total - len(mapped)
    return {"schema": SCHEMA,
            "sessions": {"total": total, "mapped": len(mapped), "unmapped": unmapped,
                         "unmapped_share": round(unmapped / total, 2) if total else None},
            "campaigns": campaigns}


def load_db(path):
    """The worker agents (with their worktree's latest branch) and the agent_stops rows, read-only from `path`."""
    db = sqlite3.connect(Path(path).resolve().as_uri() + "?mode=ro", uri=True)
    db.row_factory = sqlite3.Row
    try:
        agents = [dict(r) for r in db.execute(
            "SELECT a.agent_id, a.agent_type, a.custom_agent_type, a.name, a.model_alias, a.worktree, a.first_ts,"
            " (SELECT b.branch FROM worktree_branches b WHERE b.worktree = a.worktree"
            "  ORDER BY b.observed_at DESC LIMIT 1) AS branch_hint"
            " FROM agents a ORDER BY a.agent_id")]
        stops = {r["agent_id"]: dict(r) for r in db.execute("SELECT * FROM agent_stops")}
    finally:
        db.close()
    return {"agents": agents, "stops": stops}
