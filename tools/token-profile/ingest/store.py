"""The SQLite store: opening it, finding transcript files and reading them incrementally."""

import bisect
import collections
import hashlib
import json
import os
import re
import sqlite3
from datetime import datetime, timezone
from pathlib import Path

from . import classify as cls
from .shapes import unknown_shape
from .transcript import Transcript

SCHEMA = Path(__file__).resolve().parent.parent / "schema.sql"
SCHEMA_VERSION = "7"


def now_iso():
    return datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%S.%f")[:-3] + "Z"


def open_db(path):
    db = sqlite3.connect(path)
    db.execute("PRAGMA foreign_keys = ON")
    # A local cache rebuilt from the transcripts: WAL without a sync per commit
    # (an fsync per file cost 80 s of a 3.5-minute first run on Windows).
    db.execute("PRAGMA journal_mode = WAL")
    db.execute("PRAGMA synchronous = NORMAL")
    has_meta = db.execute("SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'meta'").fetchone()
    if not has_meta:
        db.executescript(SCHEMA.read_text(encoding="utf-8"))
    version = db.execute("SELECT value FROM meta WHERE key = 'schema_version'").fetchone()[0]
    if version == "6":
        _upgrade_6_to_7(db)
        version = SCHEMA_VERSION
    if version != SCHEMA_VERSION:
        db.close()
        raise SystemExit(f"{path} has schema version {version}, this ingest writes {SCHEMA_VERSION}; "
                         "start a new database (it is rebuilt from the transcripts); a version 6 database is upgraded in place.")
    return db


def _upgrade_6_to_7(db):
    """Schema 6 to 7 in place (S4a): add agents.brief_tag and backfill it (S4b). Safe to run twice."""
    if "brief_tag" not in {row[1] for row in db.execute("PRAGMA table_info(agents)")}:
        db.execute("ALTER TABLE agents ADD COLUMN brief_tag TEXT"
                   " CHECK (brief_tag IS NULL OR brief_tag IN ('review-fix'))")
    backfill_brief_tags(db)
    db.execute("UPDATE meta SET value = '7' WHERE key = 'schema_version'")
    db.commit()


def backfill_brief_tags(db):
    """agents.brief_tag from each ingested subagent transcript's first prompt, as the ingest sets it (C19)."""
    for (name,) in db.execute("SELECT path FROM ingest_files WHERE kind = 'subagent'").fetchall():
        path = Path(name)
        agent_id = path.stem[len("agent-"):]
        row = db.execute("SELECT agent_type FROM agents WHERE agent_id = ?", (agent_id,)).fetchone()
        if row is None or row[0] == "fork" or not path.is_file():
            continue
        db.execute("UPDATE agents SET brief_tag = ? WHERE agent_id = ?", (_first_prompt_tag(path), agent_id))


def _first_prompt_tag(path):
    """The brief tag of a transcript's first user prompt (not a tool result); None without one. Never the text."""
    with open(path, "rb") as fh:
        for raw in fh:
            try:
                rec = json.loads(raw)
            except ValueError:
                continue
            if not isinstance(rec, dict) or unknown_shape(rec) is not None or rec.get("type") != "user":
                continue
            msg = rec.get("message") if isinstance(rec.get("message"), dict) else {}
            content = msg.get("content")
            if isinstance(content, list) and any(isinstance(b, dict) and b.get("type") == "tool_result"
                                                 for b in content):
                continue
            return cls.brief_tag(cls.text_of(content))
    return None


def project_prefix(repo_root):
    """Claude Code's project directory name for a checkout: every non-alphanumeric character becomes '-'."""
    return re.sub(r"[^A-Za-z0-9]", "-", str(repo_root))


def discover(projects_root, prefix):
    """(kind, path, project_dir, session_id, agent_id) for every transcript file, in a stable order.

    Main sessions are <dir>/<session>.jsonl; subagents are any agent-<id>.jsonl
    under <dir>/<session>/subagents/, including workflow agents one level down.
    Other files (workflow journals, forked-skill markers, tool-results) are not transcripts.

    A fork's transcript starts with a copy of its parent's history, and the
    first transcript read owns a request. So within a session every fork is
    read after its parent (the main session, or the subagent its meta names).
    """
    root = Path(projects_root)
    out = []
    for d in sorted(p for p in root.iterdir() if p.is_dir() and p.name.lower().startswith(prefix.lower())):
        for f in sorted(d.glob("*.jsonl")):
            out.append(("main", f, d.name, f.stem, None))
        subs = [(f.relative_to(d).parts[0], f.stem[len("agent-"):], f)
                for f in sorted(d.glob("*/subagents/**/agent-*.jsonl"))]
        parents = {(s, a): _fork_parent(f) for s, a, f in subs}
        for s, a, f in sorted(subs, key=lambda x: (x[0], _fork_depth(parents, x[0], x[1]), str(x[2]))):
            out.append(("subagent", f, d.name, s, a))
    return out


def _fork_parent(transcript):
    """The parent agent id of a fork ('' for the main session), or None if the transcript is not a fork."""
    try:
        meta = json.loads(transcript.with_name(transcript.stem + ".meta.json").read_text(encoding="utf-8"))
    except (OSError, ValueError):
        return None
    if not isinstance(meta, dict) or not meta.get("isFork"):
        return None
    parent = meta.get("parentAgentId")
    return parent if isinstance(parent, str) else ""


def _fork_depth(parents, session_id, agent_id):
    """How many forks deep a subagent is: 0 for a non-fork, 1 for a fork of a non-fork, ..."""
    depth, seen = 0, set()
    while agent_id and parents.get((session_id, agent_id)) is not None and agent_id not in seen:
        seen.add(agent_id)
        depth += 1
        agent_id = parents[(session_id, agent_id)]
    return depth


def _head_sha1(path):
    with open(path, "rb") as fh:
        return hashlib.sha1(fh.readline()).hexdigest()


def _forget_transcript(db, session_id, agent_id):
    """Drop what a transcript wrote, before it is re-read in full."""
    where = "session_id = ? AND agent_id IS ?"
    args = (session_id, agent_id)
    db.execute(f"DELETE FROM pr_attribution WHERE request_id IN (SELECT request_id FROM requests WHERE {where})", args)
    for table in ("tool_calls", "compactions", "requests", "triggers"):
        db.execute(f"DELETE FROM {table} WHERE {where}", args)
    if agent_id is None:
        db.execute("DELETE FROM pr_links WHERE session_id = ?", (session_id,))
        db.execute("DELETE FROM cost_states WHERE session_id = ?", (session_id,))


def ensure_session(db, session_id, project_dir, ts):
    db.execute("INSERT OR IGNORE INTO sessions (session_id, project_dir, first_ts, last_ts) VALUES (?, ?, ?, ?)",
               (session_id, project_dir, ts or "", ts or ""))


def _update_session(db, session_id, info):
    db.execute(
        "UPDATE sessions SET"
        " first_ts = CASE WHEN first_ts = '' OR (? IS NOT NULL AND ? < first_ts) THEN COALESCE(?, first_ts) ELSE first_ts END,"
        " last_ts = CASE WHEN ? IS NOT NULL AND ? > last_ts THEN ? ELSE last_ts END,"
        " entrypoint = COALESCE(entrypoint, ?), cc_version_first = COALESCE(cc_version_first, ?),"
        " cc_version_last = COALESCE(?, cc_version_last), worktree = COALESCE(?, worktree),"
        " forked_from = COALESCE(?, forked_from) WHERE session_id = ?",
        (info["first_ts"], info["first_ts"], info["first_ts"], info["last_ts"], info["last_ts"], info["last_ts"],
         info["entrypoint"], info["versions"][0] if info["versions"] else None,
         info["versions"][-1] if info["versions"] else None, info["worktree"], info["forked_from"], session_id))


def _update_agent(db, agent_id, session_id, info):
    db.execute("INSERT OR IGNORE INTO agents (agent_id, session_id, meta_missing) VALUES (?, ?, 1)",
               (agent_id, session_id))
    db.execute(
        "UPDATE agents SET first_ts = CASE WHEN first_ts IS NULL OR ? < first_ts THEN ? ELSE first_ts END,"
        " last_ts = CASE WHEN last_ts IS NULL OR ? > last_ts THEN ? ELSE last_ts END,"
        " worktree = COALESCE(worktree, ?) WHERE agent_id = ?",
        (info["first_ts"], info["first_ts"], info["last_ts"], info["last_ts"], info["worktree"], agent_id))


def _read_meta(db, run_id, meta_path, project_dir, agent_id):
    if not meta_path.exists():
        return
    st = meta_path.stat()
    row = db.execute("SELECT size_bytes, mtime FROM ingest_files WHERE path = ?", (str(meta_path),)).fetchone()
    mtime = str(st.st_mtime_ns)
    if row == (st.st_size, mtime):
        return
    try:
        meta = json.loads(meta_path.read_text(encoding="utf-8"))
    except (OSError, ValueError):
        return
    db.execute(
        "UPDATE agents SET agent_type = ?, custom_agent_type = ?, name = ?, model_alias = ?, task_kind = ?,"
        " spawn_depth = ?, request_shape = ?, meta_missing = 0 WHERE agent_id = ?",
        (meta.get("agentType"), meta.get("customAgentType"), meta.get("name"), meta.get("model"),
         meta.get("taskKind"), meta.get("spawnDepth"), meta.get("requestShape"), agent_id))
    db.execute(
        "INSERT INTO ingest_files (path, project_dir, kind, size_bytes, mtime, head_sha1, byte_offset, last_run_id)"
        " VALUES (?, ?, 'meta', ?, ?, '', ?, ?) ON CONFLICT (path) DO UPDATE SET size_bytes = excluded.size_bytes,"
        " mtime = excluded.mtime, byte_offset = excluded.byte_offset, last_run_id = excluded.last_run_id",
        (str(meta_path), project_dir, st.st_size, mtime, st.st_size, run_id))


def write_agent_stop(db, session_id, agent_id, state):
    """The agent_stops row of one transcript, from its last request.

    The row is deleted and written again, so a transcript that grew updates
    its row rather than adding one. A transcript with no request has none.
    A transcript whose last assistant record is a synthetic API error also
    records that (ended_in_api_error, api_error_kind).
    """
    key = agent_id or f"main:{session_id}"
    db.execute("DELETE FROM agent_stops WHERE agent_id = ?", (key,))
    where = "session_id = ? AND agent_id IS ?"
    args = (session_id, agent_id)
    last = db.execute(f"SELECT request_id, ts, stop_reason, context_tokens FROM requests WHERE {where}"
                      " ORDER BY rowid DESC LIMIT 1", args).fetchone()
    if last is None:
        return
    rid, ts, stop, context = last
    count = db.execute(f"SELECT COUNT(*) FROM requests WHERE {where}", args).fetchone()[0]
    calls = state["last_tools"]["tools"] if state["last_tools"]["rid"] == rid else []
    tool = calls[-1] if stop == "tool_use" and calls else None
    error = None
    if tool is not None:
        row = db.execute("SELECT result_is_error FROM tool_calls WHERE tool_use_id = ?", (tool["id"],)).fetchone()
        error = row[0] if row else None
    db.execute(
        "INSERT INTO agent_stops (agent_id, session_id, last_ts, requests, last_stop_reason, last_context_tokens,"
        " last_tool_name, last_tool_background, last_result_is_error, ended_after_result, ended_in_api_error,"
        " api_error_kind) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        (key, session_id, ts, count, stop, context, tool["name"] if tool else None,
         1 if tool and tool["bg"] else 0, error, 1 if state["ends_result"] else 0,
         1 if state["api_error"] else 0, state["api_error"]))


def ingest_file(db, run_id, kind, path, project_dir, session_id, agent_id, stats, touched):
    """Read the new complete lines of one transcript. Returns True if anything was read."""
    st = path.stat()
    head = _head_sha1(path)
    row = db.execute("SELECT size_bytes, head_sha1, byte_offset, parse_state FROM ingest_files WHERE path = ?",
                     (str(path),)).fetchone()
    offset, state = 0, {}
    if row is not None:
        size, old_head, old_offset, old_state = row
        if st.st_size < size or head != old_head:
            stats["files_reread"] += 1
            _forget_transcript(db, session_id, agent_id)
        else:
            offset, state = old_offset, json.loads(old_state)
    if agent_id is not None:
        ensure_session(db, session_id, project_dir, None)
        db.execute("INSERT OR IGNORE INTO agents (agent_id, session_id, meta_missing) VALUES (?, ?, 1)",
                   (agent_id, session_id))
        _read_meta(db, run_id, path.with_name(path.stem + ".meta.json"), project_dir, agent_id)
    if row is not None and offset >= st.st_size:
        return False
    with open(path, "rb") as fh:
        fh.seek(offset)
        data = fh.read()
    end = data.rfind(b"\n") + 1          # a partial last line waits for the next run
    if end == 0:
        return False
    stats["files_scanned"] += 1
    ensure_session(db, session_id, project_dir, None)
    reader = Transcript(db, run_id, str(path), session_id, agent_id, state, stats)
    line_no = state.get("line", 0)
    for raw in data[:end].splitlines():
        line_no += 1
        if not raw.strip():
            continue
        try:
            rec = json.loads(raw)
        except ValueError:
            reader._unknown("<invalid json>", {}, line_no)
            continue
        reader.feed(rec, line_no)
    reader.st["line"] = line_no
    if agent_id is None:
        _update_session(db, session_id, reader.info)
    else:
        _update_session(db, session_id, {**reader.info, "worktree": None, "forked_from": None, "entrypoint": None})
        _update_agent(db, agent_id, session_id, reader.info)
    write_agent_stop(db, session_id, agent_id, reader.st)
    db.execute(
        "INSERT INTO ingest_files (path, project_dir, kind, size_bytes, mtime, head_sha1, byte_offset, parse_state,"
        " last_run_id) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?) ON CONFLICT (path) DO UPDATE SET"
        " size_bytes = excluded.size_bytes, mtime = excluded.mtime, head_sha1 = excluded.head_sha1,"
        " byte_offset = excluded.byte_offset, parse_state = excluded.parse_state, last_run_id = excluded.last_run_id",
        (str(path), project_dir, kind, offset + end, str(st.st_mtime_ns), head, offset + end,
         json.dumps(reader.state()), run_id))
    touched.add((session_id, agent_id))
    return True


def finalize(db, touched):
    """Derived columns: coordinator sessions and context exposure of tool results."""
    db.execute(
        "UPDATE sessions SET is_coordinator = CASE WHEN"
        " EXISTS (SELECT 1 FROM agents a WHERE a.session_id = sessions.session_id)"
        " OR EXISTS (SELECT 1 FROM tool_calls t WHERE t.session_id = sessions.session_id AND t.agent_id IS NULL"
        "            AND t.tool_name IN ('Agent', 'Task'))"
        " OR EXISTS (SELECT 1 FROM triggers g WHERE g.session_id = sessions.session_id AND g.agent_id IS NULL"
        "            AND g.kind IN ('background_completion', 'idle_notification', 'teammate_message', 'agent_message'))"
        " THEN 1 ELSE 0 END")
    for session_id, agent_id in sorted(touched, key=lambda k: (k[0], k[1] or "")):
        _exposure(db, session_id, agent_id)


def _exposure(db, session_id, agent_id):
    """later_requests: requests after a tool call's request, up to the next compaction, in its transcript."""
    key = (session_id, agent_id)
    ts = sorted(t for (t,) in db.execute("SELECT ts FROM requests WHERE session_id = ? AND agent_id IS ?", key))
    cuts = sorted(t for (t,) in db.execute("SELECT ts FROM compactions WHERE session_id = ? AND agent_id IS ?", key))
    rows = []
    for tool_use_id, t, chars in db.execute(
            "SELECT tool_use_id, ts, result_chars FROM tool_calls WHERE session_id = ? AND agent_id IS ?"
            " AND result_chars IS NOT NULL", key):
        k = bisect.bisect_right(cuts, t)
        end = bisect.bisect_left(ts, cuts[k]) if k < len(cuts) else len(ts)
        later = max(0, end - bisect.bisect_right(ts, t))
        rows.append((later, chars * later, tool_use_id))
    db.executemany("UPDATE tool_calls SET later_requests = ?, exposure_chars = ? WHERE tool_use_id = ?", rows)


def new_stats():
    return collections.Counter()


def default_projects_root():
    return Path(os.path.expanduser("~")) / ".claude" / "projects"
