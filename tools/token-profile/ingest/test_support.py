"""Shared setup for the ingest tests: build transcripts, run the CLI, dump the database."""

import contextlib
import io
import json
import sqlite3
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent / "fixtures"))
import build_fixtures  # noqa: E402

from . import cli  # noqa: E402

PREFIX = build_fixtures.PROJECT_DIR

# Tables whose content a run produces; ingest_files and profiler_runs hold run bookkeeping.
CONTENT_TABLES = ("sessions", "agents", "triggers", "requests", "tool_calls", "compactions", "pr_links",
                  "cost_states", "unknown_shapes", "pr_attribution")


def run_ingest(projects, db, *extra, prs=None, repo=None):
    """Run the CLI quietly; returns its exit status."""
    args = ["--db", str(db), "--projects", str(projects), "--project-prefix", PREFIX, "--quiet",
            "--profiler-commit", "test", "--lane-log", str(Path(projects) / "no-lane-log.jsonl")]
    if prs is not None:
        prs_path = Path(projects).parent / "prs.json"
        prs_path.write_text(json.dumps(prs), encoding="utf-8")
        args += ["--prs-json", str(prs_path)]
    if repo is not None:
        args += ["--repo", str(repo)]
    with contextlib.redirect_stderr(io.StringIO()):
        return cli.main(args + list(extra))


def dump(db_path):
    """Every content row, sorted, for comparing two databases."""
    db = sqlite3.connect(db_path)
    out = {}
    for table in CONTENT_TABLES:
        cols = [c[1] for c in db.execute(f"PRAGMA table_info({table})")]
        keep = [c for c in cols if c not in ("first_path", "first_line")]
        out[table] = sorted(db.execute(f"SELECT {', '.join(keep)} FROM {table}").fetchall(), key=repr)
    db.close()
    return out


def pr(number, branch, created, merged=None, head=None, merge=None, state=None):
    """One `gh pr list --json` row."""
    return {"number": number, "headRefName": branch, "headRefOid": head, "createdAt": created,
            "mergeCommit": {"oid": merge} if merge else None, "mergedAt": merged, "closedAt": merged,
            "state": state or ("MERGED" if merged else "OPEN"), "additions": 1, "deletions": 0, "changedFiles": 1}


def write_session(projects, session_id, transcript, agents=()):
    """Write a main transcript and its subagents ((agent_id, Transcript, meta or None)) under the fixture dir."""
    proj = Path(projects) / PREFIX
    proj.mkdir(parents=True, exist_ok=True)
    build_fixtures.write_jsonl(proj / f"{session_id}.jsonl", transcript.lines)
    for agent_id, t, meta in agents:
        sub = proj / session_id / "subagents"
        sub.mkdir(parents=True, exist_ok=True)
        build_fixtures.write_jsonl(sub / f"agent-{agent_id}.jsonl", t.lines)
        if meta is not None:
            (sub / f"agent-{agent_id}.meta.json").write_text(json.dumps(meta), encoding="utf-8")


def transcript(session_id, agent_id=None, branch="main"):
    """A build_fixtures.Transcript writing records for session_id instead of the fixture's session."""
    t = build_fixtures.Transcript(agent_id=agent_id, branch=branch)
    base = t._base

    def _base(kind, when):
        rec = base(kind, when)
        rec["sessionId"] = session_id
        rec["uuid"] = f"{session_id[:8]}-{agent_id or 'm'}-{rec['uuid']}"
        t.parent = rec["uuid"]
        return rec

    t._base = _base
    return t
