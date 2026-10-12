"""Overstep detection: work a role did that a cheaper tier should have done, and the wasted-cost estimate.

Pure functions take plain dicts and lists. A request is a dict with its ts, model, the four token counts
(prices.TOKEN_COLUMNS), session_id, agent_id, trigger_kind, worktree (the .claude/worktrees/<name> it ran in),
agent_type (the custom agent type when set), agent_name, and calls (a list of {tool_name, fingerprint}). Packets are
(campaign, {id, implementer, size, escalated}) pairs from the ledger. The DB is read only in load_requests, which cli.py
calls.

Rules (work-packets.md, CM-07): O1 Opus coding, O2 Opus research, O3 Sonnet implementing, O4 Opus subagent in a
packet worktree (counted, not priced). Repricing is an estimate: it ignores extra turns a cheaper model may need.
"""

import sqlite3
from datetime import datetime, timedelta
from pathlib import Path

from ingest import prices

from .packets import packet_of_session

TIERS = {"haiku": "claude-haiku-5-5", "sonnet": "claude-sonnet-5-5"}
EDIT_TOOLS = {"Edit", "Write", "NotebookEdit"}
READ_TOOLS = {"Read", "Grep"}
BUILD_RUNS = {"cargo build", "cargo check", "cargo clippy", "cargo fmt", "cargo nextest", "cargo run",
              "cargo test", "lane", "lane.ps1", "lane.sh", "bash tools/build-lane/lane.sh"}
LANE_SCRIPTS = {"lane.ps1", "lane.sh"}
O4_AGENT_TYPES = {"documentation-writer", "general-purpose"}
BURST_WINDOW = timedelta(minutes=10)
FOLLOW_UPS = 3
CACHE_COLUMNS = ("cache_read", "cache_write_5m", "cache_write_1h")
ITEM_LIMIT = 20
UNATTRIBUTED = "unattributed"


def parse_ts(text):
    """A DB or ledger timestamp (ISO 8601, 'Z' or an offset) as an aware datetime."""
    return datetime.fromisoformat(text.replace("Z", "+00:00"))


def family(model):
    """'opus', 'sonnet', 'haiku' or 'fable' for a model id, else None."""
    name = prices.normalize_model(model)
    return next((f for f in ("opus", "sonnet", "haiku", "fable") if name.startswith(f"claude-{f}")), None)


def actual_usd(req, version=prices.CURRENT):
    """List-price USD of a request at its own model; None when the model has no price."""
    return prices.estimate_usd(req["model"], req, version)


def _cache_at(req, model, version):
    """The cache part (reads and writes) of a request priced at `model`; the long-prompt rule uses the whole prompt."""
    row = prices.price_for(model, version)
    if row is None:
        return None
    return sum(req.get(k, 0) * row[k] for k in CACHE_COLUMNS) * prices.tier_factor(row, prices.prompt_tokens(req)) / 1e6


def reprice(req, target_tier, version=prices.CURRENT):
    """USD of the same four token counts at the target tier's current model row; None when unpriced."""
    return prices.estimate_usd(TIERS[target_tier], req, version)


def _under(path, prefixes):
    """True when a repo-relative path is one of `prefixes` or below one (a prefix ending in '/' is a directory)."""
    return bool(path) and any(path == p or path.startswith(p if p.endswith("/") else p + "/") for p in prefixes)


def _is_code_edit(call, cfg):
    path = call.get("fingerprint") or ""
    return (call["tool_name"] in EDIT_TOOLS and _under(path, cfg["code_paths"])
            and not _under(path, cfg["allowed_coordinator_paths"]))


def _path_class(call):
    """The top-level repo directory an edit or read touched; None for a run, which has no path."""
    path = call.get("fingerprint") or ""
    return path.split("/")[0] if path and call.get("tool_name") in EDIT_TOOLS | READ_TOOLS else None


def worktree_name(path):
    """The worktree name from .claude/worktrees/<name> (a path or the bare name), lower-cased; None if none."""
    return path.replace("\\", "/").rstrip("/").split("/")[-1].lower() if path else None


def packet_for(req, packets):
    """The (campaign, packet) a request ran for, or None.

    CM-06's matcher reads the agent's name, then the worktree, for a packet ID. An ID that two campaigns hold matches
    nothing, so only one campaign's packet is returned.
    """
    pid = packet_of_session({"name": req.get("agent_name")}, req["worktree"], None, [p for _, p in packets], "")
    found = [(campaign, packet) for campaign, packet in packets if packet["id"] == pid]
    return found[0] if len(found) == 1 else None


def _small(size):
    """The ledger's size cell is S or M (its first letter)."""
    return (size or "").strip().upper()[:1] in ("S", "M")


def _is_build_run(fingerprint):
    """True for a cargo or lane fingerprint, or any command that names lane.ps1 or lane.sh (`pwsh tools/build-lane/lane.ps1 ...`)."""
    if not fingerprint:
        return False
    if fingerprint in BUILD_RUNS:
        return True
    return any(part.split("/")[-1] in LANE_SCRIPTS for part in fingerprint.replace("\\", "/").split())


def opus_coding(req, cfg):
    """O1: the first call of an Opus main-session request that edits a code path or runs a build lane, else None."""
    if req["agent_id"] is not None or family(req["model"]) != "opus":
        return None
    return next((c for c in req["calls"] if _is_code_edit(c, cfg) or _is_build_run(c.get("fingerprint"))), None)


def opus_research(req, cfg):
    """O2 direct: an Opus main-session request that calls a research tool, else None."""
    if req["agent_id"] is not None or family(req["model"]) != "opus":
        return None
    return next((c for c in req["calls"] if c["tool_name"].startswith(tuple(cfg["research_tool_prefixes"]))), None)


def sonnet_implementing(req, packets, cfg):
    """O3: a Sonnet request that edits a code path in a request whose packet is packet-coder's, size S or M, not
    escalated. A review fixer's edits are expected, so a request with the fixer role is not O3.
    Returns (call, (campaign, packet))."""
    if family(req["model"]) != "sonnet" or req["role"] == "fixer":
        return None
    found = packet_for(req, packets)
    if not found or found[1]["implementer"] != "packet-coder" or found[1]["escalated"] or not _small(found[1]["size"]):
        return None
    call = next((c for c in req["calls"] if c["tool_name"] in EDIT_TOOLS and _under(c.get("fingerprint") or "", cfg["code_paths"])), None)
    return (call, found) if call else None


def opus_subagent(req, packets):
    """O4: an Opus subagent of a documentation or general-purpose type working on a packet. Not priced."""
    return (req["agent_id"] is not None and family(req["model"]) == "opus"
            and req["agent_type"] in O4_AGENT_TYPES and packet_for(req, packets) is not None)


def _follow_ups(req, streams):
    """The next FOLLOW_UPS requests in the same session and agent, until a human prompt starts a new turn."""
    out = []
    for nxt in streams[(req["session_id"], req["agent_id"])][req["seq"] + 1:]:
        if len(out) == FOLLOW_UPS or nxt["trigger_kind"] == "human_prompt":
            break
        out.append(nxt)
    return out


def research_bursts(reqs, cfg):
    """O2 bursts: clusters of Opus main-session requests whose Read/Grep calls on code paths fall in one 10-minute
    window, `research_read_burst` or more calls. Each cluster is a list of distinct requests."""
    hits = {}
    for req in reqs:
        if req["agent_id"] is not None or family(req["model"]) != "opus":
            continue
        for c in req["calls"]:
            if c["tool_name"] in READ_TOOLS and _under(c.get("fingerprint") or "", cfg["code_paths"]):
                hits.setdefault(req["session_id"], []).append((parse_ts(req["ts"]), req))
    clusters = []
    for session in hits.values():
        session.sort(key=lambda h: h[0])
        i = 0
        while i < len(session):
            end = session[i][0] + BURST_WINDOW
            members = [h for h in session[i:] if h[0] <= end]
            if len(members) >= cfg["research_read_burst"]:
                seen = {}
                for _, req in members:
                    seen[req["request_id"]] = req
                clusters.append(list(seen.values()))
                i += len(members)
            else:
                i += 1
    return clusters


def _campaign_of_row(row, pr_map, packet_campaign):
    """A pr_attribution row's campaign: its own tag (a 'pr-' tag is a placeholder), else its PR's, else the ledger's
    campaign for the PR, else the packet's, else 'unattributed'."""
    tag = row.get("campaign")
    if tag and not tag.startswith("pr-"):
        return tag
    return row.get("pr_campaign") or (pr_map or {}).get(row.get("pr_number")) or packet_campaign or UNATTRIBUTED


def _shares(req, packets, pr_map):
    """(campaign, share) per attribution row of a request, the shares normalised to sum to 1. A request with no rows
    is one untagged row, so it takes its packet's campaign or 'unattributed'."""
    rows = req.get("rows") or [{}]
    found = packet_for(req, packets)
    total = sum(row.get("weight", 1.0) for row in rows)
    packet_campaign = found[0] if found else None
    return [(_campaign_of_row(row, pr_map, packet_campaign), row.get("weight", 1.0) / total) for row in rows]


def _split(hit):
    """Each campaign's share of a hit: the mean over its requests of their attribution shares."""
    out = {}
    for req in hit["requests"]:
        for campaign, share in req["shares"]:
            out[campaign] = out.get(campaign, 0.0) + share / len(hit["requests"])
    return out


def _hit(rule, reqs, call, follow_ups=()):
    """One overstep: its rule, its request, the call that made it, and the follow-up requests whose cache cost it also
    carries."""
    return {"rule": rule, "requests": reqs, "call": call, "follow_ups": list(follow_ups)}


def _waste(hit, target, version):
    """(usd actual, wasted at `target`) for a hit: its requests in full, its follow-ups by cache cost; wasted floors at 0."""
    actual = sum(u for u in (actual_usd(r, version) for r in hit["requests"]) if u is not None)
    actual += sum(u for u in (_cache_at(f, f["model"], version) for f in hit["follow_ups"]) if u is not None)
    repriced = sum(u for u in (reprice(r, target, version) for r in hit["requests"]) if u is not None)
    repriced += sum(u for u in (_cache_at(f, TIERS[target], version) for f in hit["follow_ups"]) if u is not None)
    return actual, max(0.0, actual - repriced)


def detect(requests, packets, cfg, pr_map=None, version=prices.CURRENT):
    """Overstep counts, USD and wasted USD per campaign, from the requests of the window.

    `packets` is as in ledger_inputs and `pr_map` ({pr: campaign}) is its third value. Each request's USD is split
    across its pr_attribution rows by weight, and each row names its campaign (_campaign_of_row); nothing is dropped,
    so a request with no campaign counts under 'unattributed' and the campaigns sum to the total. A hit counts once in
    each campaign it touches. Requests before the
    regime switch or at/after the window's end are ignored. The main session's role is coordinator; a subagent's comes
    from cfg roles by its agent type.
    """
    regime, until = parse_ts(cfg["regime_switch"]), parse_ts(cfg["window"]["until"])
    rules = cfg["overstep"]
    reqs = sorted((dict(r) for r in requests if regime <= parse_ts(r["ts"]) < until), key=lambda r: parse_ts(r["ts"]))
    streams = {}
    fixers = {}
    for req in reqs:
        req["role"] = "coordinator" if req["agent_id"] is None else cfg["roles"].get(req["agent_type"])
        req["shares"] = _shares(req, packets, pr_map)
        stream = streams.setdefault((req["session_id"], req["agent_id"]), [])
        req["seq"] = len(stream)
        stream.append(req)
        # fixer-heavy is deferred until CM-06 supplies fix_commits; CM-08 joins these per-worktree counts with them.
        name = worktree_name(req["worktree"])
        if req["role"] == "fixer" and name:
            fixers[name] = fixers.get(name, 0) + 1
    hits = []
    for req in reqs:
        call = opus_coding(req, rules)
        if call:
            hits.append(_hit("O1", [req], call, follow_ups=_follow_ups(req, streams)))
        call = opus_research(req, rules)
        if call:
            hits.append(_hit("O2", [req], call))
        found = sonnet_implementing(req, packets, rules)
        if found:
            hits.append(_hit("O3", [req], found[0]))
    hits.extend(_hit("O2", cluster, None) for cluster in research_bursts(reqs, rules))
    out, counts = {}, {}
    for hit in hits:
        rule = hit["rule"]
        target = rules["reprice"]["implementation" if rule in ("O1", "O3") else "research"]
        actual, wasted = _waste(hit, target, version)
        _, wasted_low = _waste(hit, "sonnet", version)
        for campaign, share in _split(hit).items():
            stats = out.setdefault(campaign,
                                   {"rules": {}, "recoverable_low": 0.0, "recoverable_high": 0.0, "items": []})
            row = stats["rules"].setdefault(rule, {"count": 0, "usd": 0.0, "wasted_usd": 0.0})
            row["count"] += 1
            row["usd"] += actual * share
            row["wasted_usd"] += wasted * share
            stats["recoverable_low"] += wasted_low * share
            stats["recoverable_high"] += wasted * share
            if len(stats["items"]) < ITEM_LIMIT:
                head, call = hit["requests"][0], hit["call"] or {}
                stats["items"].append({"rule": rule, "ts": head["ts"], "session_family": head["session_id"],
                                       "model": head["model"], "usd": round(actual * share, 6),
                                       "tool": call.get("tool_name"), "path_class": _path_class(call)})
        counts[rule] = counts.get(rule, 0) + 1
    opus_subagents = len({r["agent_id"] for r in reqs if opus_subagent(r, packets)})
    return {"by_campaign": out, "opus_subagents": opus_subagents, "requests": len(reqs), "hits": counts,
            "fixer_requests": fixers}


REQUEST_SQL = """
    SELECT r.request_id, r.session_id, r.agent_id, r.ts, r.model, r.input_tokens AS input,
           r.output_tokens AS output, r.cache_read, r.cache_write_5m, r.cache_write_1h, t.kind AS trigger_kind,
           COALESCE(r.cwd_worktree, a.worktree, s.worktree) AS worktree,
           COALESCE(a.custom_agent_type, a.agent_type) AS agent_type, a.name AS agent_name
    FROM requests r
    JOIN sessions s ON s.session_id = r.session_id
    JOIN triggers t ON t.trigger_id = r.trigger_id
    LEFT JOIN agents a ON a.agent_id = r.agent_id
    ORDER BY r.ts, r.request_id
"""
CALL_SQL = "SELECT request_id, tool_name, fingerprint FROM tool_calls ORDER BY ts, tool_use_id"
ROW_SQL = """
    SELECT pa.request_id, pa.pr_number, pa.weight, pa.campaign, p.campaign AS pr_campaign
    FROM pr_attribution pa
    LEFT JOIN prs p ON p.pr_number = pa.pr_number
"""


def load_requests(db_path):
    """The requests of a token-profile database, in time order, each with its tool calls and its pr_attribution rows.

    The file is opened read only (mode=ro), so a missing file is an error and is never created.
    """
    db = sqlite3.connect(Path(db_path).resolve().as_uri() + "?mode=ro", uri=True)
    db.row_factory = sqlite3.Row
    try:
        calls = {}
        for row in db.execute(CALL_SQL):
            calls.setdefault(row["request_id"], []).append({"tool_name": row["tool_name"], "fingerprint": row["fingerprint"]})
        rows = {}
        for row in db.execute(ROW_SQL):
            rows.setdefault(row["request_id"], []).append(dict(row))
        return [dict(row, calls=calls.get(row["request_id"], []), rows=rows.get(row["request_id"], []))
                for row in db.execute(REQUEST_SQL)]
    finally:
        db.close()


def ledger_inputs(ledger_doc):
    """(packets, windows, pr_map) for detect, from a ledger.json: packets as (campaign, packet) pairs, windows by
    campaign, and pr_map ({pr: campaign}) from every packet's PRs, override rows included. A PR that two campaigns
    claim is left out of the map."""
    packets = [(name, {"id": p["id"], "implementer": p["implementer"], "size": p["size"], "escalated": p["escalated"]})
               for name, campaign in ledger_doc["campaigns"].items() for p in campaign["packets"]
               if not p.get("override")]
    windows = {name: campaign["window"] for name, campaign in ledger_doc["campaigns"].items()}
    claims = {}
    for name, campaign in ledger_doc["campaigns"].items():
        for p in campaign["packets"]:
            for pr in p.get("prs", []):
                claims.setdefault(pr, set()).add(name)
    pr_map = {pr: names.pop() for pr, names in claims.items() if len(names) == 1}
    return packets, windows, pr_map
