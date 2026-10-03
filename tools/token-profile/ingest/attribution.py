"""Request-to-PR attribution, rules A1-A6 of attribution.md.

Rebuilt in full on every run: a PR that appears later can place requests
that an earlier run left unattributed. Every request ends with rows whose
weights sum to 1; the run fails if attribution_imbalance is not empty.
"""

import bisect
import collections

from .transcript import epoch

DAY = 86400.0
A1_AGENT_KINDS = {"background_completion", "idle_notification", "teammate_message", "agent_message"}
NOT_A_BRANCH = {"", "HEAD", "main", "master"}
ANCESTRY_LOOKAHEAD = 14 * DAY      # attribution.md: the commit observed within 14 days after the request
ANCESTRY_HORIZON = 60 * DAY        # PRs merged later than this after the request are not tried
CONF = {"trigger": 0.9, "branch": 1.0, "ancestry": 0.8, "parent-session": 0.7, "pr-link": 0.6, "split": 0.6}


class Attributor:
    def __init__(self, db, ancestry=None):
        self.db = db
        self.ancestry = ancestry
        self.stats = collections.Counter()
        self.prs = {}
        for n, branch, head, merge, created, merged, closed, state in db.execute(
                "SELECT pr_number, head_branch, head_sha, merge_sha, created_at, merged_at, closed_at, state FROM prs"):
            end = epoch(merged) if merged else (epoch(closed) if closed else None)
            self.prs[n] = {"n": n, "branch": branch, "head": head, "merge": merge, "created": epoch(created),
                           "merged": epoch(merged) if merged else None, "end": end}
        self.by_branch = collections.defaultdict(list)
        for p in self.prs.values():
            self.by_branch[p["branch"]].append(p)
        self.merged = sorted((p for p in self.prs.values() if p["merged"] is not None and p["head"]),
                             key=lambda p: p["merged"])
        self.merged_ts = [p["merged"] for p in self.merged]
        if ancestry is not None:
            ancestry.prepare([sha for p in self.prs.values() for sha in (p["head"], p["merge"])])
        self.heads = collections.defaultdict(list)
        for branch, sha, observed in db.execute("SELECT branch, commit_sha, observed_at FROM branch_heads"):
            t = epoch(observed)
            if t is not None:
                self.heads[branch].append((t, sha))
        self.coordinators = {s for (s,) in db.execute("SELECT session_id FROM sessions WHERE is_coordinator = 1")}
        self.agents_by_id = {}
        self.agents_by_name = collections.defaultdict(list)
        for agent_id, session_id, name in db.execute("SELECT agent_id, session_id, name FROM agents"):
            self.agents_by_id[agent_id] = session_id
            if name:
                self.agents_by_name[(session_id, name)].append(agent_id)
        self.triggers = {t: {"kind": k, "ref": r, "session": s, "agent": a, "ts": epoch(ts)}
                         for t, k, r, s, a, ts in db.execute(
                             "SELECT trigger_id, kind, source_ref, session_id, agent_id, ts FROM triggers")}
        self.requests = [
            {"id": r, "session": s, "agent": a, "trigger": t, "ts": epoch(ts), "branch": b or ""}
            for r, s, a, t, ts, b in db.execute(
                "SELECT request_id, session_id, agent_id, trigger_id, ts, git_branch FROM requests ORDER BY ts")]
        self.by_id = {r["id"]: r for r in self.requests}
        # A5 inputs: PRs named by tool calls in each turn, and pr-link records mapped to turns.
        self.turn_prs = collections.defaultdict(set)
        for trig, pr in db.execute(
                "SELECT r.trigger_id, t.pr_ref FROM tool_calls t JOIN requests r ON r.request_id = t.request_id"
                " WHERE t.pr_ref IS NOT NULL AND t.task_id IS NULL"):
            self.turn_prs[trig].add(pr)
        self._map_pr_links()
        # A1 bash: background task id -> the PR its call named. A4: agent id -> the request that spawned it.
        self.task_pr = {}
        self.spawner = {}
        for task, session, tool, pr, rid in db.execute(
                "SELECT task_id, session_id, tool_name, pr_ref, request_id FROM tool_calls WHERE task_id IS NOT NULL"):
            if tool in ("Agent", "Task"):
                self.spawner[task] = rid
            elif pr is not None:
                self.task_pr[(session, task)] = pr

    def _map_pr_links(self):
        """Each pr-link record belongs to the main-session turn open at its timestamp."""
        turns = collections.defaultdict(list)
        for trig, s, ts in self.db.execute(
                "SELECT DISTINCT g.trigger_id, g.session_id, g.ts FROM triggers g JOIN requests r"
                " ON r.trigger_id = g.trigger_id WHERE g.agent_id IS NULL AND g.kind != 'retry'"):
            t = epoch(ts)
            if t is not None:
                turns[s].append((t, trig))
        for s in turns:
            turns[s].sort()
        for s, pr, ts in self.db.execute("SELECT session_id, pr_number, ts FROM pr_links"):
            t = epoch(ts)
            if t is None or s not in turns:
                continue
            i = bisect.bisect_right(turns[s], (t, "￿")) - 1
            if i >= 0:
                self.turn_prs[turns[s][i][1]].add(pr)

    # --- rules ---------------------------------------------------------------------

    def a2_branch(self, req):
        if req["branch"] in NOT_A_BRANCH or req["ts"] is None:
            return None
        best = None
        for p in self.by_branch.get(req["branch"], ()):
            if p["created"] is None or req["ts"] < p["created"] - DAY:
                continue
            if p["end"] is not None and req["ts"] > p["end"]:
                continue
            if best is None or p["created"] > best["created"]:
                best = p
        return best["n"] if best else None

    def a3_ancestry(self, req):
        if self.ancestry is None or req["branch"] in NOT_A_BRANCH or req["ts"] is None:
            return None
        seen = [(t, sha) for t, sha in self.heads.get(req["branch"], ())
                if req["ts"] <= t <= req["ts"] + ANCESTRY_LOOKAHEAD]
        if not seen:
            return None
        commit = max(seen)[1]
        i = bisect.bisect_right(self.merged_ts, req["ts"])
        for p in self.merged[i:]:
            if p["merged"] > req["ts"] + ANCESTRY_HORIZON:
                break
            if p["branch"] == req["branch"]:
                continue
            # Squash merges leave the packet commits out of the merge commit, so test the PR's
            # head; and a commit main already had before the merge is not this PR's work.
            before = self.ancestry.first_parent(p["merge"]) if p["merge"] else None
            if self.ancestry.is_ancestor(commit, p["head"]) and not (
                    before and self.ancestry.is_ancestor(commit, before)):
                return p["n"]
        return None

    def phase1(self, req):
        """A2 then A3: the rules that place a request by its own branch."""
        pr = self.a2_branch(req)
        if pr is not None:
            return [(pr, "branch", 1.0)]
        pr = self.a3_ancestry(req)
        if pr is not None:
            return [(pr, "ancestry", 1.0)]
        return None

    def _agent_mix(self, agent_id):
        counts = collections.Counter()
        for rid in self.agent_requests.get(agent_id, ()):
            rows = self.p1.get(rid)
            if rows:
                counts[rows[0][0]] += 1
        total = sum(counts.values())
        return [(pr, "trigger", c / total) for pr, c in sorted(counts.items())] if total else None

    def a1_trigger(self, req):
        trig = self.triggers.get(req["trigger"])
        if trig is None or req["session"] not in self.coordinators or trig["kind"] not in A1_AGENT_KINDS:
            return None
        ref = trig["ref"]
        if not ref:
            return None
        if trig["kind"] == "background_completion" and (req["session"], ref) in self.task_pr:
            pr = self.task_pr[(req["session"], ref)]
            return [(pr, "trigger", 1.0)] if pr in self.prs else None
        agents = []
        if self.agents_by_id.get(ref) is not None:
            agents = [ref]
        else:
            agents = self.agents_by_name.get((req["session"], ref), [])
        for agent_id in agents:
            mix = self._agent_mix(agent_id)
            if mix:
                return mix
        return None

    def a5_pr_link(self, req):
        prs = sorted(p for p in self.turn_prs.get(req["trigger"], ()) if p in self.prs)
        if not prs:
            return None
        if len(prs) == 1:
            return [(prs[0], "pr-link", 1.0)]
        return [(p, "split", 1.0 / len(prs)) for p in prs]

    def a4_parent(self, req, depth=0):
        parent = self.spawner.get(req["agent"])
        if parent is None or parent not in self.by_id or depth > 8:
            return None
        rows = self.final(self.by_id[parent], depth + 1)
        if not any(pr is not None for pr, _, _ in rows):
            return None
        return [(pr, "parent-session" if pr is not None else "unattributed", w) for pr, _, w in rows]

    def final(self, req, depth=0):
        rid = req["id"]
        if rid in self.done:
            return self.done[rid]
        rows = None
        if req["agent"] is None:
            rows = self.a1_trigger(req) or self.p1.get(rid) or self.a5_pr_link(req)
        else:
            rows = self.p1.get(rid) or self.a4_parent(req, depth) or self.a5_pr_link(req)
        rows = rows or [(None, "unattributed", 1.0)]
        self.done[rid] = rows
        return rows

    def run(self):
        self.p1 = {}
        self.agent_requests = collections.defaultdict(list)
        for req in self.requests:
            rows = self.phase1(req)
            if rows:
                self.p1[req["id"]] = rows
            if req["agent"] is not None:
                self.agent_requests[req["agent"]].append(req["id"])
        self.done = {}
        self.db.execute("DELETE FROM pr_attribution")
        out = []
        for req in self.requests:
            merged = collections.OrderedDict()
            for pr, method, w in self.final(req):
                key = pr
                if key in merged:
                    merged[key] = (merged[key][0], merged[key][1] + w)
                else:
                    merged[key] = (method, w)
            for pr, (method, w) in merged.items():
                conf = 0.0 if pr is None else CONF[method]
                out.append((req["id"], pr, method, min(w, 1.0), conf))
                self.stats[method] += 1
        self.db.executemany("INSERT INTO pr_attribution (request_id, pr_number, method, weight, confidence)"
                            " VALUES (?, ?, ?, ?, ?)", out)
        return self.db.execute("SELECT COUNT(*) FROM attribution_imbalance").fetchone()[0]
