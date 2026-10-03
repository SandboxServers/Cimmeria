"""Reads one transcript file's new records into the database.

A transcript is a main session file (agent_id None) or a subagent file. The
reader keeps a small state between records (the open trigger, the API-error
flag, the last request), and that state is saved in ingest_files.parse_state
so an appended file resumes exactly where the last run stopped.
"""

import json
import re
from datetime import datetime, timezone

from . import classify as cls
from . import fingerprint as fp
from . import workbranch as wb
from .shapes import SYNTHETIC_MODEL, unknown_shape

WORKTREE = re.compile(r"[\\/]\.claude[\\/]worktrees[\\/]([^\\/]+)")
PERSISTED = re.compile(r"Output too large.*saved to|<persisted-output>", re.DOTALL)
AGENT_TOOLS = {"Agent", "Task"}


def worktree_of(path):
    m = WORKTREE.search(path or "")
    return f".claude/worktrees/{m.group(1)}" if m else None


def epoch(ts):
    try:
        return datetime.fromisoformat(ts.replace("Z", "+00:00")).timestamp()
    except (AttributeError, ValueError):
        return None


def ms_iso(ms):
    """Epoch milliseconds (cost-state startTime) as ISO-8601 UTC; None unless a positive number."""
    if not isinstance(ms, (int, float)) or isinstance(ms, bool) or ms <= 0:
        return None
    return datetime.fromtimestamp(ms / 1000, timezone.utc).strftime("%Y-%m-%dT%H:%M:%S.%f")[:-3] + "Z"


def usage_columns(u, stats):
    cc = u.get("cache_creation") if isinstance(u.get("cache_creation"), dict) else None
    total_cc = u.get("cache_creation_input_tokens") or 0
    if cc is None:
        # Older records carry no 5m/1h split; count the write as 5m and say so.
        w5, w1 = total_cc, 0
        if total_cc:
            stats["cache_split_missing"] += 1
    else:
        w5 = cc.get("ephemeral_5m_input_tokens") or 0
        w1 = cc.get("ephemeral_1h_input_tokens") or 0
        if w5 + w1 != total_cc:
            stats["cache_split_mismatch"] += 1
    out = u.get("output_tokens") or 0
    thinking = (u.get("output_tokens_details") or {}).get("thinking_tokens") or 0
    if thinking > out:
        stats["thinking_clamped"] += 1
        thinking = out
    server = u.get("server_tool_use") if isinstance(u.get("server_tool_use"), dict) else {}
    cols = {
        "input_tokens": u.get("input_tokens") or 0,
        "output_tokens": out,
        "thinking_tokens": thinking,
        "cache_read": u.get("cache_read_input_tokens") or 0,
        "cache_write_5m": w5,
        "cache_write_1h": w1,
        "web_search": server.get("web_search_requests") or 0,
        "web_fetch": server.get("web_fetch_requests") or 0,
    }
    cols["context_tokens"] = cols["input_tokens"] + cols["cache_read"] + w5 + w1
    return cols


def result_text(content):
    if isinstance(content, str):
        return content
    if isinstance(content, list):
        return "".join(b.get("text", "") for b in content if isinstance(b, dict) and isinstance(b.get("text"), str))
    return ""


class Transcript:
    def __init__(self, db, run_id, path, session_id, agent_id, state, stats):
        self.db = db
        self.run_id = run_id
        self.path = path
        self.session_id = session_id
        self.agent_id = agent_id
        self.st = {"cur": None, "retry": False, "sched": False, "seen_turn": False, "last_req": None,
                   "compaction": None, "gh_create": [], "git_calls": [], "last_ts": None, **state}
        self.stats = stats
        self.is_fork = agent_id is not None and db.execute(
            "SELECT agent_type = 'fork' FROM agents WHERE agent_id = ?", (agent_id,)).fetchone() == (1,)
        self.info = {"first_ts": None, "last_ts": None, "versions": [], "entrypoint": None, "worktree": None,
                     "forked_from": None}

    # --- helpers -----------------------------------------------------------------

    def _note(self, rec):
        ts = rec.get("timestamp")
        if isinstance(ts, str):
            if self.info["first_ts"] is None or ts < self.info["first_ts"]:
                self.info["first_ts"] = ts
            if self.info["last_ts"] is None or ts > self.info["last_ts"]:
                self.info["last_ts"] = ts
            self.st["last_ts"] = ts
        v = rec.get("version")
        if isinstance(v, str) and v not in self.info["versions"]:
            self.info["versions"].append(v)
        if not self.info["entrypoint"] and isinstance(rec.get("entrypoint"), str):
            self.info["entrypoint"] = rec["entrypoint"]
        if not self.info["worktree"]:
            self.info["worktree"] = worktree_of(rec.get("cwd"))

    def _unknown(self, shape, rec, line_no):
        self.stats["unknown_records"] += 1
        self.db.execute(
            "INSERT INTO unknown_shapes (shape, cc_version, first_path, first_line, count) VALUES (?, ?, ?, ?, 1)"
            " ON CONFLICT (shape, cc_version) DO UPDATE SET count = count + 1",
            (shape, rec.get("version") or "" if isinstance(rec, dict) else "", self.path, line_no))

    def _trigger_id(self, uuid):
        """uuid, unless another transcript already owns it (copied history)."""
        owner = self.db.execute("SELECT session_id, agent_id FROM triggers WHERE trigger_id = ?", (uuid,)).fetchone()
        if owner is None or owner == (self.session_id, self.agent_id):
            return uuid
        return f"{uuid}@{self.agent_id or self.session_id}"

    def _start(self, trig):
        """Make trig the open trigger; an auxiliary record never replaces a pending non-auxiliary one."""
        cur = self.st["cur"]
        if cur and cur["n"] == 0 and trig["kind"] == "auxiliary" and cur["kind"] != "auxiliary":
            self.stats["auxiliary_folded"] += 1
            return
        self.st["cur"] = trig

    def _ensure_trigger(self, trig):
        self.db.execute(
            "INSERT OR IGNORE INTO triggers (trigger_id, session_id, agent_id, ts, kind, origin_kind, source_ref, rule)"
            " VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            (trig["id"], self.session_id, self.agent_id, trig["ts"], trig["kind"], trig.get("okind"),
             trig.get("ref"), trig["rule"]))

    # --- records -----------------------------------------------------------------

    def feed(self, rec, line_no):
        if not isinstance(rec, dict):
            self._unknown("<not an object>", {}, line_no)
            return
        self.stats["records_read"] += 1
        shape = unknown_shape(rec)
        if shape is not None:
            self._unknown(shape, rec, line_no)
            return
        self._note(rec)
        t = rec["type"]
        if t == "assistant":
            self._assistant(rec)
        elif t == "user":
            self._user(rec)
        elif t == "attachment" and rec["attachment"].get("type") == "queued_command":
            self._queued(rec)
        elif t == "system":
            self._system(rec)
        elif t == "pr-link":
            self._pr_link(rec)
        elif t == "cost-state" and self.agent_id is None:
            self._cost_state(rec)
        elif t == "fork-context-ref" and self.agent_id is None:
            self.info["forked_from"] = rec.get("parentSessionId")
        elif t == "worktree-state":
            ws = rec.get("worktreeSession")
            if isinstance(ws, dict) and ws.get("worktreeName"):
                self.info["worktree"] = f".claude/worktrees/{ws['worktreeName']}"
        elif t == "relocated":
            self.info["worktree"] = worktree_of(rec.get("relocatedCwd")) or self.info["worktree"]

    def _assistant(self, rec):
        msg = rec["message"]
        if msg.get("model") == SYNTHETIC_MODEL:
            if rec.get("isApiErrorMessage"):
                self.st["retry"] = True
                self.stats["api_errors"] += 1
            return
        rid = rec["requestId"]
        cols = usage_columns(msg["usage"], self.stats)
        row = self.db.execute("SELECT session_id, agent_id FROM requests WHERE request_id = ?", (rid,)).fetchone()
        if row is not None:
            if row != (self.session_id, self.agent_id):
                # A fork's transcript opens with a copy of its parent's history; discover() reads
                # parents first, so the parent already owns these requests.
                self.stats["fork_copied_records" if self.is_fork else "duplicate_request_records"] += 1
                return
            # A later record of the same request: keep the final usage.
            sets = ", ".join(f"{k} = ?" for k in cols)
            self.db.execute(f"UPDATE requests SET {sets}, records_seen = records_seen + 1, message_uuid = ?"
                            f" WHERE request_id = ?", (*cols.values(), rec.get("uuid") or "", rid))
        else:
            self._new_request(rec, rid, msg, cols)
        self._tool_uses(rec, rid, msg)

    def _new_request(self, rec, rid, msg, cols):
        st = self.st
        ts = rec.get("timestamp") or ""
        if st["retry"]:
            trig = {"id": f"{rid}:retry", "ts": ts, "kind": "retry", "rule": "R1", "okind": None, "ref": None}
            st["retry"] = False
        elif st["cur"] is not None:
            trig = st["cur"]
        else:
            # A transcript that opens on a request (copied or resumed history).
            trig = {"id": f"{rid}:orphan", "ts": ts, "kind": "unknown", "rule": "R15", "okind": None, "ref": None,
                    "n": 0}
            st["cur"] = trig
            self.stats["orphan_requests"] += 1
        self._ensure_trigger(trig)
        if trig is st["cur"]:
            trig["n"] += 1
        gap = None
        if st["last_req"]:
            a, b = epoch(st["last_req"]["ts"]), epoch(ts)
            if a is not None and b is not None:
                gap = round(b - a, 3)
        self.db.execute(
            "INSERT INTO requests (request_id, session_id, agent_id, trigger_id, message_uuid, records_seen, ts, model,"
            " cc_version, git_branch, cwd_worktree, input_tokens, output_tokens, thinking_tokens, cache_read,"
            " cache_write_5m, cache_write_1h, web_search, web_fetch, context_tokens, prev_gap_s)"
            " VALUES (?, ?, ?, ?, ?, 1, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            (rid, self.session_id, self.agent_id, trig["id"], rec.get("uuid") or "", ts, msg.get("model") or "",
             rec.get("version"), rec.get("gitBranch"), worktree_of(rec.get("cwd")), cols["input_tokens"],
             cols["output_tokens"], cols["thinking_tokens"], cols["cache_read"], cols["cache_write_5m"],
             cols["cache_write_1h"], cols["web_search"], cols["web_fetch"], cols["context_tokens"], gap))
        self.stats["new_requests"] += 1
        if st["compaction"]:
            self.db.execute("UPDATE compactions SET request_after = ? WHERE boundary_uuid = ?", (rid, st["compaction"]))
            st["compaction"] = None
        st["last_req"] = {"id": rid, "ts": ts}

    def _tool_uses(self, rec, rid, msg):
        content = msg.get("content")
        if not isinstance(content, list):
            return
        root = fp.checkout_root(rec.get("cwd"))
        for b in content:
            if not isinstance(b, dict) or b.get("type") != "tool_use" or not b.get("id"):
                continue
            name = b.get("name") or ""
            inp = b.get("input") if isinstance(b.get("input"), dict) else {}
            fprint, server = fp.fingerprint(name, inp, root)
            pr_ref = verb = None
            worktree = wb.worktree_ref(inp)
            if name in fp.SHELL_TOOLS:
                pr_ref, is_gh = fp.gh_pr_ref(inp.get("command"))
                verb = fp.gh_pr_verb(inp.get("command"))
                if is_gh and pr_ref is None:
                    self.st["gh_create"] = (self.st["gh_create"] + [b["id"]])[-50:]
                if not is_gh and inp.get("run_in_background"):
                    pr_ref = fp.description_pr_ref(inp.get("description"))
                if wb.is_git_command(inp.get("command")) and not inp.get("run_in_background"):
                    self.st["git_calls"] = (self.st["git_calls"] + [[b["id"], worktree]])[-50:]
            self.db.execute(
                "INSERT OR IGNORE INTO tool_calls (tool_use_id, request_id, session_id, agent_id, ts, tool_name,"
                " mcp_server, fingerprint, pr_ref, pr_verb, worktree) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                (b["id"], rid, self.session_id, self.agent_id, rec.get("timestamp") or "", name, server, fprint,
                 pr_ref, verb, worktree))

    def _user(self, rec):
        msg = rec.get("message") if isinstance(rec.get("message"), dict) else {}
        content = msg.get("content")
        results = [b for b in content if isinstance(b, dict) and b.get("type") == "tool_result"] \
            if isinstance(content, list) else []
        if results:
            self._tool_results(rec, results)
            return
        st = self.st
        first_in_subagent = self.agent_id is not None and not st["seen_turn"]
        kind, rule, ref = cls.classify(
            cls.text_of(content), rec.get("origin"), bool(rec.get("isMeta")), bool(rec.get("isCompactSummary")),
            bool(rec.get("scheduledTaskId")) or st["sched"], first_in_subagent)
        st["sched"] = False
        st["seen_turn"] = True
        st["retry"] = False
        origin = rec.get("origin") if isinstance(rec.get("origin"), dict) else {}
        self._start({"id": self._trigger_id(rec.get("uuid") or f"line:{self.stats['records_read']}"),
                     "ts": rec.get("timestamp") or "", "kind": kind, "rule": rule, "okind": origin.get("kind"),
                     "ref": ref, "n": 0})

    def _queued(self, rec):
        a = rec["attachment"]
        prompt = a.get("prompt")
        kind, rule, ref = cls.classify(cls.text_of(prompt), a.get("origin"), bool(a.get("isMeta")),
                                       command_mode=a.get("commandMode"))
        origin = a.get("origin") if isinstance(a.get("origin"), dict) else {}
        cur = self.st["cur"]
        if cur and cur.get("delivery") and cur["n"] == 0:
            # Same delivery: no request since the last queued_command.
            cur["items"].append(kind)
            if len(set(cur["items"])) > 1:
                cur.update(kind="mixed", rule="R2", ref=None)
            return
        self.st["retry"] = False
        self._start({"id": self._trigger_id(rec.get("uuid") or a.get("source_uuid") or ""),
                     "ts": a.get("timestamp") or rec.get("timestamp") or "", "kind": kind, "rule": rule,
                     "okind": origin.get("kind"), "ref": ref, "n": 0, "delivery": True, "items": [kind]})

    def _tool_results(self, rec, results):
        tur = rec.get("toolUseResult") if isinstance(rec.get("toolUseResult"), dict) else {}
        task_id = tur.get("agentId") or tur.get("backgroundTaskId")
        task_id = task_id if isinstance(task_id, str) else None
        for b in results:
            tid = b.get("tool_use_id")
            text = result_text(b.get("content"))
            pr_ref = None
            if tid in self.st["gh_create"]:
                self.st["gh_create"].remove(tid)
                pr_ref = fp.result_pr_ref(text)
            branch = self._git_result(rec, tid, text)
            cur = self.db.execute(
                "UPDATE tool_calls SET result_chars = ?, result_is_error = ?, result_persisted = ?,"
                " task_id = COALESCE(?, task_id), pr_ref = COALESCE(pr_ref, ?), branch_seen = COALESCE(?, branch_seen)"
                " WHERE tool_use_id = ?",
                (len(text), 1 if b.get("is_error") else 0, 1 if PERSISTED.search(text) else 0,
                 task_id if len(results) == 1 else None, pr_ref, branch, tid))
            if cur.rowcount == 0:
                self.stats["orphan_tool_results"] += 1

    def _git_result(self, rec, tid, text):
        """The branch a git command's output names; worktree-to-branch pairs go to worktree_branches."""
        call = next((c for c in self.st["git_calls"] if c[0] == tid), None)
        if call is None:
            return None
        self.st["git_calls"].remove(call)
        when = rec.get("timestamp") or ""
        pairs = wb.worktree_list(text)
        branch = wb.branch_seen(text)
        if branch and call[1]:
            pairs.append((call[1], branch))
        self.db.executemany(
            "INSERT OR IGNORE INTO worktree_branches (worktree, branch, observed_at, source) VALUES (?, ?, ?, ?)",
            [(w, br, when, "worktree-list" if (w, br) != (call[1], branch) else "tool-result") for w, br in pairs])
        return branch

    def _system(self, rec):
        sub = rec.get("subtype")
        if sub == "scheduled_task_fire":
            self.st["sched"] = True
        elif sub == "compact_boundary":
            meta = rec.get("compactMetadata") if isinstance(rec.get("compactMetadata"), dict) else {}
            trigger = meta.get("trigger") if meta.get("trigger") in ("auto", "manual") else "unknown"
            uuid = rec.get("uuid") or f"{self.session_id}:{rec.get('timestamp')}"
            last = self.st["last_req"]
            self.db.execute(
                "INSERT OR IGNORE INTO compactions (boundary_uuid, session_id, agent_id, ts, trigger, pre_tokens,"
                " post_tokens, dropped_tokens, duration_ms, request_before) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                (uuid, self.session_id, self.agent_id, rec.get("timestamp") or "", trigger, meta.get("preTokens"),
                 meta.get("postTokens"), meta.get("cumulativeDroppedTokens"), meta.get("durationMs"),
                 last["id"] if last else None))
            self.st["compaction"] = uuid

    def _pr_link(self, rec):
        n = rec.get("prNumber")
        if isinstance(n, int):
            self.db.execute("INSERT OR IGNORE INTO pr_links (session_id, pr_number, repository, ts) VALUES (?, ?, ?, ?)",
                            (self.session_id, n, rec.get("prRepository") or "", rec.get("timestamp") or ""))

    def _cost_state(self, rec):
        usage = rec.get("modelUsage") if isinstance(rec.get("modelUsage"), dict) else {}
        numbers = {m: {k: v for k, v in u.items() if isinstance(v, (int, float)) and not isinstance(v, bool)}
                   for m, u in usage.items() if isinstance(u, dict)}
        self.db.execute(
            "INSERT INTO cost_states (session_id, observed_ts, total_cost_usd, has_unknown_cost, lines_added,"
            " lines_removed, model_usage_json, process_start) VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
            " ON CONFLICT (session_id) DO UPDATE SET observed_ts = excluded.observed_ts,"
            " total_cost_usd = excluded.total_cost_usd, has_unknown_cost = excluded.has_unknown_cost,"
            " lines_added = excluded.lines_added, lines_removed = excluded.lines_removed,"
            " model_usage_json = excluded.model_usage_json, process_start = excluded.process_start",
            (self.session_id, rec.get("timestamp") or self.st["last_ts"], float(rec.get("totalCostUSD") or 0),
             1 if rec.get("hasUnknownModelCost") else 0, rec.get("totalLinesAdded"), rec.get("totalLinesRemoved"),
             json.dumps(numbers, sort_keys=True), ms_iso(rec.get("startTime"))))

    def state(self):
        return self.st
