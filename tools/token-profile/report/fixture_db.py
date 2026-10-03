"""Test support: load the synthetic fixture tree into a profiler database.

This is not an ingest. It reads the tree that fixtures/build_fixtures.py
writes, takes the token counts, trigger kinds and fingerprints from its
expected.json (what a correct ingest produces), and fills schema.sql with
them plus synthetic PRs, attribution and prices, so the reports can be tested
without TP-01a's code.

hostile=True plays a broken ingest instead: every free-text column a report
could show gets a raw, private value (whole commands, absolute paths, URL
credentials, tokens), so the tests can prove the report still leaks nothing.
"""

import json
import sqlite3
import sys
from datetime import datetime
from pathlib import Path

HERE = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(HERE / "fixtures"))
import build_fixtures  # noqa: E402

PRICE_VERSION = "test-prices-1"
# Synthetic prices (USD per MTok), round numbers so tests can check sums by hand.
PRICES = {"claude-opus-5-5": dict(input=5.0, output=25.0, cache_read=0.25, cache_write_5m=6.25, cache_write_1h=10.0)}
PROFILER_COMMIT = "0123456789abcdef0123456789abcdef01234567"
PR_MAIN, PR_OTHER = 4242, 4243
MERGED_AT = "2026-10-01T13:00:00.000Z"

# Request -> [(pr or None, method, weight, confidence)].
ATTRIBUTION = {
    "req_A": [(PR_MAIN, "pr-link", 1.0, 0.6)],
    "req_B": [(PR_MAIN, "pr-link", 1.0, 0.6)],
    "req_C": [(PR_MAIN, "trigger", 1.0, 0.9)],
    "req_G": [(PR_MAIN, "pr-link", 1.0, 0.6)],
    "req_H": [(PR_MAIN, "split", 0.5, 0.6), (PR_OTHER, "split", 0.5, 0.6)],
    "req_S1": [(PR_MAIN, "branch", 1.0, 1.0)],
    "req_S2": [(PR_MAIN, "branch", 1.0, 1.0)],
    "req_J": [(PR_OTHER, "pr-link", 0.25, 0.6), (None, "unattributed", 0.75, 0.0)],
}


# Named columns, so an additive schema change does not shift the values.
PR_INSERT = ("INSERT INTO prs (pr_number, head_branch, head_sha, merge_sha, created_at, merged_at, closed_at,"
             " state, additions, deletions, changed_files)")

def parse_ts(s):
    return datetime.fromisoformat(s.replace("Z", "+00:00"))


def build(tmp, hostile=False):
    """Build the fixture tree under tmp and load it. Returns (db_path, expected)."""
    root = build_fixtures.build(Path(tmp) / "tree")
    expected = json.loads((root / "expected.json").read_text(encoding="utf-8"))
    db_path = Path(tmp) / "profile.sqlite"
    db = sqlite3.connect(db_path)
    db.executescript((HERE / "schema.sql").read_text(encoding="utf-8"))
    load(db, root, expected, hostile)
    db.commit()
    db.close()
    return db_path, expected


def transcripts(root):
    for path in sorted(root.rglob("*.jsonl")):
        yield path, [json.loads(line) for line in path.read_text(encoding="utf-8").splitlines()]


def load(db, root, expected, hostile):
    session = build_fixtures.SESSION
    agent = build_fixtures.AGENT
    bad = expected["hostile"]
    commit = "--token supersecret" if hostile else PROFILER_COMMIT
    db.execute("INSERT INTO profiler_runs VALUES (1, '2026-10-02T00:00:00Z', '2026-10-02T00:01:00Z', ?, ?,"
               " 2, 60, 1, 'ok')", (commit, PRICE_VERSION))
    for model, p in PRICES.items():
        db.execute("INSERT INTO price_tables VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
                   (PRICE_VERSION, model, p["input"], p["output"], p["cache_read"], p["cache_write_5m"],
                    p["cache_write_1h"], bad[3] if hostile else "https://platform.claude.com/docs/en/about-claude/pricing"))
    db.execute("INSERT INTO sessions VALUES (?, ?, '2026-10-01T12:00:00.000Z', '2026-10-01T12:41:00.000Z', 'cli',"
               " ?, ?, NULL, 1, NULL)", (session, build_fixtures.PROJECT_DIR, build_fixtures.VERSION,
                                         bad[1] if hostile else build_fixtures.VERSION))
    meta = expected["agents"][agent]
    db.execute("INSERT INTO agents (agent_id, session_id, agent_type, custom_agent_type, name, model_alias,"
               " task_kind, spawn_depth, request_shape, worktree, meta_missing) VALUES (?, ?, ?, ?, ?, 'opus',"
               " 'in_process_teammate', 0, 'background', ?, 0)",
               (agent, session, "fixture-worker", bad[0] if hostile else meta["custom_agent_type"],
                bad[2] if hostile else "fixture-worker", bad[0] if hostile else None))
    db.execute("INSERT INTO unknown_shapes VALUES ('future-record-type', ?, ?, 1, 1)",
               (build_fixtures.VERSION, bad[0]))

    request_rows, tool_rows, triggers = [], {}, {}
    compaction = None
    for path, records in transcripts(root):
        is_sub = "subagents" in path.parts
        aid = agent if is_sub else None
        final = {}
        order = []
        trigger = None
        for i, rec in enumerate(records):
            t = rec.get("type")
            if t == "user" and not isinstance(rec["message"]["content"], list):
                trigger = rec["uuid"]
            elif t == "attachment" and rec["attachment"]["type"] == "queued_command":
                trigger = rec["uuid"]
            elif t == "assistant" and not rec.get("isApiErrorMessage"):
                rid = rec["requestId"]
                if rid not in final:
                    order.append(rid)
                final[rid] = (rec, trigger)
            elif t == "system" and rec.get("subtype") == "compact_boundary":
                compaction = (rec, len(order))
        prev = None
        for n, rid in enumerate(order):
            rec, trg = final[rid]
            want = expected["requests"][rid]
            trg = want.get("trigger_id", trg)
            triggers.setdefault(trg, (aid, rec["timestamp"], want["trigger_kind"], want["rule"]))
            ts = parse_ts(rec["timestamp"])
            gap = (ts - prev).total_seconds() if prev else None
            prev = ts
            ctx = want["input_tokens"] + want["cache_read"] + want["cache_write_5m"] + want["cache_write_1h"]
            request_rows.append((rid, session, aid, trg, rec["uuid"], rec["timestamp"], rec["message"]["model"],
                                 bad[1] if hostile else rec["version"], bad[3] if hostile else rec["gitBranch"], want, ctx, gap, path, n))
            for block in rec["message"]["content"]:
                if block.get("type") == "tool_use":
                    tool_rows[block["id"]] = (rid, aid, rec["timestamp"], block, path, n)
        results = {}
        for rec in records:
            content = rec.get("message", {}).get("content") if rec.get("type") == "user" else None
            if isinstance(content, list):
                for c in content:
                    if c.get("type") == "tool_result":
                        results[c["tool_use_id"]] = len(c["content"])
        for tid, row in list(tool_rows.items()):
            if row[4] == path:
                tool_rows[tid] = row + (results.get(tid), len(order))

    for trg, (aid, ts, kind, rule) in triggers.items():
        db.execute("INSERT INTO triggers VALUES (?, ?, ?, ?, ?, NULL, ?, ?)",
                   (trg, session, aid, ts, kind, bad[2] if hostile else None, rule))
    for rid, sess, aid, trg, uuid, ts, model, version, branch, w, ctx, gap, _, _ in request_rows:
        db.execute("INSERT INTO requests VALUES (?, ?, ?, ?, ?, 2, ?, ?, ?, ?, NULL, ?, ?, ?, ?, ?, ?, 0, 0, ?, ?, 0)",
                   (rid, sess, aid, trg, uuid, ts, model, version, branch, w["input_tokens"], w["output_tokens"],
                    w["thinking_tokens"], w["cache_read"], w["cache_write_5m"], w["cache_write_1h"], ctx, gap))

    fingerprints = expected["tool_fingerprints"]
    for tid, (rid, aid, ts, block, _, n, chars, total) in tool_rows.items():
        if hostile:
            raw = block["input"]
            fp = raw.get("command") or raw.get("file_path") or raw.get("pattern")
            name = "mcp__" + bad[2] if block["name"] == "Glob" else block["name"]
        else:
            fp, name = fingerprints[tid], block["name"]
        later = total - n - 1
        db.execute("INSERT INTO tool_calls (tool_use_id, request_id, session_id, agent_id, ts, tool_name, mcp_server,"
                   " fingerprint, result_chars, result_is_error, result_persisted, later_requests, exposure_chars)"
                   " VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, 0, ?, ?, ?)",
                   (tid, rid, session, aid, ts, name, bad[4] if hostile and name.startswith("mcp__") else None,
                    fp, chars, 1 if chars and chars > 10000 else 0, later, (chars or 0) * later))

    rec, idx = compaction
    main_order = [r[0] for r in request_rows if r[2] is None]
    meta_c = rec["compactMetadata"]
    db.execute("INSERT INTO compactions VALUES (?, ?, NULL, ?, ?, ?, ?, ?, ?, ?, ?)",
               (rec["uuid"], session, rec["timestamp"], meta_c["trigger"], meta_c["preTokens"],
                meta_c["postTokens"], meta_c["cumulativeDroppedTokens"], meta_c["durationMs"],
                main_order[idx - 1], main_order[idx]))
    db.execute("INSERT INTO pr_links VALUES (?, ?, 'SandboxServers/Cimmeria', '2026-10-01T12:01:10.000Z')",
               (session, PR_MAIN))
    db.execute("INSERT INTO cost_states (session_id, observed_ts, total_cost_usd, has_unknown_cost, lines_added,"
               " lines_removed, model_usage_json) VALUES (?, NULL, 12.5, 0, 10, 2, '{}')", (session,))
    db.execute(PR_INSERT + " VALUES (?, ?, NULL, NULL, '2026-10-01T11:00:00.000Z', ?, ?, 'MERGED', 120, 30, 4)",
               (PR_MAIN, bad[0] if hostile else build_fixtures.WORKER_BRANCH, MERGED_AT, MERGED_AT))
    db.execute(PR_INSERT + " VALUES (?, 'docs/other', NULL, NULL, '2026-10-01T11:30:00.000Z', ?, ?, 'MERGED',"
               " 5, 1, 1)", (PR_OTHER, MERGED_AT, MERGED_AT))

    attributed = set()
    for rid, rows in ATTRIBUTION.items():
        for pr, method, weight, conf in rows:
            db.execute("INSERT INTO pr_attribution VALUES (?, ?, ?, ?, ?)", (rid, pr, method, weight, conf))
        attributed.add(rid)
    for row in request_rows:
        if row[0] not in attributed:
            db.execute("INSERT INTO pr_attribution VALUES (?, NULL, 'unattributed', 1.0, 0.0)", (row[0],))
    assert not db.execute("SELECT * FROM attribution_imbalance").fetchall()
