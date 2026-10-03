"""Ground truth for attribution: which PR an agent's or worker session's requests belong to.

Two sources, scored apart because they differ in how independent they are of
the rules being scored:

- `labels`: a CSV of `name,date,pr` rows a person wrote from what they know
  (a ledger's packet-to-PR table, say). `name` is the agent's teammate name,
  `date` the UTC day of its first request. Independent of every rule.
- `authors`: a subagent, or a main session run in one worktree, that created
  exactly one PR (`gh pr create`) is taken to have spent all of its requests
  on it. A5 uses the same fact for requests nothing else places, so this set
  measures misplacement better than coverage.

Both give {request_id: pr}. A label wins over the authors rule for the same agent.
"""

import csv
from pathlib import Path


def read_labels(path):
    """[(name, date, pr)] from a CSV with a header row `name,date,pr`."""
    with open(Path(path), encoding="utf-8", newline="") as fh:
        return [(r["name"].strip(), r["date"].strip(), int(r["pr"])) for r in csv.DictReader(fh)
                if r.get("name") and r.get("pr")]


def from_labels(db, labels):
    """({request_id: pr}, unmatched labels). A label matches every agent of that name first seen that day."""
    truth, unmatched = {}, []
    for name, day, pr in labels:
        agents = [a for (a,) in db.execute("SELECT agent_id FROM agents WHERE name = ? AND substr(first_ts, 1, 10) = ?",
                                           (name, day))]
        if not agents:
            unmatched.append((name, day, pr))
        for agent in agents:
            for (rid,) in db.execute("SELECT request_id FROM requests WHERE agent_id = ?", (agent,)):
                truth[rid] = pr
    return truth, unmatched


def from_authors(db):
    """{request_id: pr} for subagents and single-worktree main sessions that created exactly one PR."""
    truth = {}
    known = {n for (n,) in db.execute("SELECT pr_number FROM prs")}
    for agent, prs in _created(db, "agent_id IS NOT NULL", "agent_id").items():
        if len(prs) == 1 and prs <= known:
            pr = next(iter(prs))
            for (rid,) in db.execute("SELECT request_id FROM requests WHERE agent_id = ?", (agent,)):
                truth[rid] = pr
    for session, prs in _created(db, "agent_id IS NULL", "session_id").items():
        worktrees = db.execute("SELECT COUNT(DISTINCT COALESCE(cwd_worktree, '')), MIN(cwd_worktree) FROM requests"
                               " WHERE session_id = ? AND agent_id IS NULL", (session,)).fetchone()
        if len(prs) == 1 and prs <= known and worktrees[0] == 1 and worktrees[1]:
            pr = next(iter(prs))
            for (rid,) in db.execute("SELECT request_id FROM requests WHERE session_id = ? AND agent_id IS NULL",
                                     (session,)):
                truth[rid] = pr
    return truth


def _created(db, where, key):
    out = {}
    for k, pr in db.execute(f"SELECT {key}, pr_ref FROM tool_calls WHERE pr_verb = 'create' AND pr_ref IS NOT NULL"
                            f" AND {where}"):
        out.setdefault(k, set()).add(pr)
    return out
