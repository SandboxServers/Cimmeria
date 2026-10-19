# ai-harness-docs: S4, token-profile counts review fix passes as `fixer`

Approved by the owner on 2026-10-18 (D-AH58). Ledger: [README.md](README.md) (F41, F53, F56). Report: [w4-s1s4-report.md](w4-s1s4-report.md). The review loop and the brief templates are in [work-packets-w4.md](work-packets-w4.md) (C12 to C14, rule 0); this file carries its own rules (§ S4 rules), so a packet-coder reads only § S4 rules, § C19 and its own packet section.

## Dispatch (for the orchestrator)

- **Starts after W4a has merged** (AH-17 puts the `review fix, pass N of 3` mode in packet-coder.md; S4a step 1 checks it). It is file-disjoint from W4b, W4c, S3, S5 and S6 and runs in parallel with them.
- **One worktree, one PR.** `ops` runs `pwsh tools/build-lane/mk-worktree.ps1 ai-harness-docs/s4-fix-pass-role ah-s4` from `origin/main`. S4a, S4b, S4c and S4d run **one after another** in `ah-s4`, each committing on top of the last.
- **Loops (C12):** each packet's packet loop finishes (`ship`) before the next packet's coder starts, because they share the worktree. After S4d's loop, one wave loop over `origin/main..HEAD` (a wave of four packets).
- **Briefs:** C14 templates; report paths `<wt>\.claude\reports\ai-harness-docs\s4a-coder.md` and so on. Every brief adds: "PowerShell only; run every check in the foreground with `Set-Location <wt>;` at the start of the call; commit as soon as the checks pass."
- **Brief caveat (C19 item 6):** a coder or reviewer brief for S4 must never contain the fix-pass phrase with a digit in it. Only a real fix-pass brief does (C14 already writes "review fix, pass N of 3" with N filled in).
- **Ship:** after the wave loop says `ship`, `ops` runs `ship.ps1 pr -C <wt> -m "feat(token-profile): count review fix passes as fixer from the brief, no text stored (ai-harness-docs S4)"` and, after rule 0, `ship.ps1 merge <PR> --retire ah-s4`. The PR body lists the doc-update-map row (S4d) and the "Not done" list below.

## S4 rules (every S4 packet)

- **S1r. Worktree.** Work only in `<wt>` (the absolute path of `.claude\worktrees\ah-s4`), with absolute paths. Start every PowerShell call with `Set-Location <wt>;` and run every check in the foreground. `shell_guard` refuses a command that only reads (`Select-String`, `Get-Content`): use the Grep tool for those checks, or end the command with `# guard-ok`.
- **S2r. Shell.** PowerShell only, never bash. No `cargo` at all. Never run `git stash`, `git push`, `git worktree prune`, `git reset`, `git checkout`, `git restore` or `gh pr`.
- **S3r. Files.** Edit only the files your packet's **Files** line names. Never edit `tools/token-profile/metrics/config.json`, `metrics/test_packets.py`, `metrics/test_overstep.py`, `ingest/test_ingest_stops.py` or any fixture file.
- **S4r. CRLF.** Every target file is CRLF in the working tree, `.py` and `.sql` included. The Edit tool keeps a file's endings; a new file written with the Write tool is LF, so after writing it run `$f='<abs>'; $t=[IO.File]::ReadAllText($f); [IO.File]::WriteAllText($f, ($t -replace "\r?\n","`r`n"))`. Check each file you touched: `$t=[IO.File]::ReadAllText('<abs>'); $t -match "(?<!\r)\n"` prints `False`.
- **S5r. Exact finds.** Every find text below must match **exactly once** in its file (count with the Grep tool). Zero or more than once: stop and return `blocked` with the step number.
- **S6r. Caps.** Files: soft cap 500 lines, hard 700 (`[IO.File]::ReadAllLines('<abs>').Length`). No IPs, credentials, account names or real prompt text in tests.
- **S7r. Small diffs.** No whole-file rewrites: `git -C <wt> diff --stat` shows small counts on every file you only edited.
- **S8r. Tests first.** Write the packet's tests before its code change, run them, and record in your report which tests fail and how (the revert proof). Then make the change and record that they pass.
- **S9r. Check.** `Set-Location <wt>; python -m unittest discover -s tools/token-profile -p "test_*.py"` must print `OK`. The baseline on 2026-10-18 was 538 tests; record the count before and after. CI runs the same command on Linux (test.yml).
- **S10r. Commit** once at the end with the brief's message: `git -C <wt> add -- <each file>`, then `git -C <wt> commit -F <message file>`. In a review fix pass, the subject is `<packet> review fixes, pass N`.

Host: windows (every S4 packet).

## Contract

### C19. The brief tag (D-AH58)

1. **What is stored.** One new column, `agents.brief_tag TEXT CHECK (brief_tag IS NULL OR brief_tag IN ('review-fix'))`. It is `'review-fix'` when a subagent's **first prompt** matches the pattern below, else NULL. **No prompt text is stored anywhere**, only this fixed tag (transcript-format.md: "Never store message text").
2. **The pattern**, in `ingest/classify.py`: `re.compile(r"\breview fix, pass [1-3] of 3\b", re.IGNORECASE)`. A real fix-pass brief (C14) has a digit; "pass N of 3" (the template's own text) does not match, and neither does "pass 4 of 3".
3. **When it is set.** The ingest sets it on the first user prompt of a subagent transcript (rule R11's `first_in_subagent`), never for a fork (a fork's first prompt is a copy of its parent's history). It is set to the result either way (tag or NULL), so a reread transcript gets it again. Later prompts never change it.
4. **Schema 7.** `SCHEMA_VERSION = "7"`. `store.open_db` upgrades a version 6 database in place (adds the column, backfills it from the transcripts, sets the version); every other old version is still refused. Reports accept 1 to 7.
5. **The role.** `metrics.packets.brief_role(role, tag)` returns `"fixer"` when the role is `"implementer-haiku"` and the tag is `"review-fix"`; otherwise the role, unchanged. The packet join (`role_of`), the cost rollup (`cost._role`) and the overstep detector (`req["role"]`) all use it. A database without the column (below 7) reads as untagged.
6. **Brief caveat.** Only a fix-pass brief may contain the phrase with a digit; S4's own coder and reviewer briefs never do.

---

## S4a: schema 7, the tag from the first prompt, no text stored

Agent: packet-coder. Worktree `ah-s4`, branch `ai-harness-docs/s4-fix-pass-role`. Size: about 30k. Commit subject: `feat(token-profile): schema v7 tags review fix passes from the first prompt, no text stored (ai-harness-docs S4a)`.

**Rules:** § S4 rules, all of them. **Files:** `tools/token-profile/schema.sql`, `tools/token-profile/ingest/store.py`, `tools/token-profile/ingest/classify.py`, `tools/token-profile/ingest/transcript.py`, `tools/token-profile/report/db.py`, `tools/token-profile/test_contract.py`, `tools/token-profile/metrics/test_cost.py` (one line), new `tools/token-profile/ingest/test_brief_tag.py`.

**Steps:**

1. Guard: the Grep tool finds the literal `review fix, pass N of 3` in `<wt>\.claude\agents\packet-coder.md` (one line or more). If it finds none, W4a has not merged into this worktree's base: stop and return `blocked` with the reason `W4a not merged`.
2. Write `ingest/test_brief_tag.py` (S8r: before the code), at most 250 lines, CRLF (S4r). Imports and fixture:
   ```text
   """S4a: a packet-coder whose first prompt names a review fix pass is tagged; no prompt text is stored."""

   import sqlite3
   import tempfile
   import unittest
   from pathlib import Path

   from . import classify, store
   from .test_support import run_ingest, transcript, write_session
   from .test_support import build_fixtures as bf

   SID = "77777777-0000-4000-8000-000000000007"
   MARKER = "zq-brief-marker-5531"


   def write(root, briefs, fork_brief=None):
       """A main session and one packet-coder per brief (agent ids c0, c1, ...); a brief may be a list of prompts.
       With fork_brief, a fork f0 of c0 whose first (copied) prompt is fork_brief. Returns the subagent list."""
       m = transcript(SID)
       m.user_text(bf.ts(1), "go", origin={"kind": "human"})
       bf.request(m, bf.ts(1, 5), "r_main", bf.usage(1, 1, 0, 1, 1, 0))
       agents = []
       for i, brief in enumerate(briefs):
           a = transcript(SID, agent_id=f"c{i}")
           for k, text in enumerate(brief if isinstance(brief, list) else [brief]):
               a.user_text(bf.ts(2 + i, 2 * k), text)
               bf.request(a, bf.ts(2 + i, 2 * k + 1), f"r_c{i}_{k}", bf.usage(1, 5, 0, 1, 1, 0))
           agents.append((f"c{i}", a, {"agentType": "packet-coder"}))
       if fork_brief is not None:
           f = transcript(SID, agent_id="f0")
           f.user_text(bf.ts(20), fork_brief)
           bf.request(f, bf.ts(20, 5), "r_f0", bf.usage(1, 5, 0, 1, 1, 0))
           agents.append(("f0", f, {"agentType": "fork", "isFork": True, "parentAgentId": "c0"}))
       write_session(root / "p", SID, m, agents)
       return agents


   def tags(db_path):
       conn = sqlite3.connect(db_path)
       try:
           return dict(conn.execute("SELECT agent_id, brief_tag FROM agents"))
       finally:
           conn.close()
   ```
   Then `class BriefTagTest(unittest.TestCase)` with a `setUp` that makes `self.root` from `tempfile.TemporaryDirectory()` (with `addCleanup`, as test_ingest_rules.py does) and `self.db = self.root / "d.sqlite"`, and these tests:
   - `test_pattern`: `classify.brief_tag(...)` is `"review-fix"` for `"AH-99 review fix, pass 2 of 3, report x"` and `"Review Fix, Pass 1 of 3"`; it is `None` for `"review fix, pass N of 3"`, `"review fix, pass 4 of 3"`, `"continuation 2 of 3"`, `""` and `None`.
   - `test_only_a_first_prompt_tags`: `write(self.root, ["AH-99: implement the packet", "AH-99 review fix, pass 2 of 3", ["AH-99: implement", "now review fix, pass 1 of 3"]])`; `run_ingest(self.root / "p", self.db, prs=[])` is 0; `tags(self.db)` is `{"c0": None, "c1": "review-fix", "c2": None}`.
   - `test_a_fork_is_never_tagged`: `write(self.root, ["plain brief"], fork_brief="review fix, pass 1 of 3")`; ingest is 0; `tags(self.db)["f0"]` is `None` and `tags(self.db)["c0"]` is `None`.
   - `test_the_tag_survives_an_incremental_run`: `agents = write(self.root, ["review fix, pass 3 of 3"])`; ingest; then append to c0's transcript (`a = agents[0][1]`; `a.user_text(bf.ts(30), "a later prompt")`; `bf.request(a, bf.ts(30, 5), "r_late", bf.usage(1, 5, 0, 1, 1, 0))`), rewrite the session with the same main transcript (rebuild it as in `write`, or keep it: change `write` to also return the main transcript if you need it) and ingest again; `tags(self.db)["c0"]` is still `"review-fix"`, and `r_late` is in `requests`.
   - `test_no_prompt_text_is_stored`: `write(self.root, [f"review fix, pass 1 of 3 {MARKER}", f"plain {MARKER}"])`; ingest; for every `name` in `SELECT name FROM sqlite_master WHERE type = 'table'`, every row of `SELECT * FROM "<name>"` has no `MARKER` in `repr(row)` (use `subTest(table=name)`); and `tags(self.db)["c0"]` is `"review-fix"`.
   - `test_a_version_6_database_is_upgraded_in_place`: `write(self.root, ["plain brief"])`; ingest; then with `sqlite3.connect(self.db)`: count `requests`, run `ALTER TABLE agents DROP COLUMN brief_tag` and `UPDATE meta SET value = '6' WHERE key = 'schema_version'`, commit, close. Then `db = store.open_db(self.db)`: the version is `"7"`, `brief_tag` is in `{r[1] for r in db.execute("PRAGMA table_info(agents)")}`, the `requests` count is unchanged; `db.close()`.
   Run S9r and record which tests fail and why (expected: `AttributeError` for `brief_tag`, `no such column: brief_tag`, `SystemExit` on the version 6 database).
3. `ingest/classify.py`. Find `SAFE_REF = re.compile(r"^[A-Za-z0-9_.:@-]{1,128}$")` and replace it with:
   ```text
   SAFE_REF = re.compile(r"^[A-Za-z0-9_.:@-]{1,128}$")
   # S4 (C19): a C12 review fix pass is briefed "review fix, pass N of 3", N a digit. Only the tag is stored.
   REVIEW_FIX = "review-fix"
   REVIEW_FIX_BRIEF = re.compile(r"\breview fix, pass [1-3] of 3\b", re.IGNORECASE)
   ```
   Then find `def _ref(value):` and insert directly above it:
   ```text
   def brief_tag(text):
       """'review-fix' when a subagent's first prompt is a C12 review fix pass, else None; never the text."""
       return REVIEW_FIX if REVIEW_FIX_BRIEF.search(text or "") else None


   ```
4. `ingest/transcript.py`, in `_user`. Find these three lines (as one find text):
   ```text
               bool(rec.get("scheduledTaskId")) or st["sched"], first_in_subagent)
           st["sched"] = False
           st["seen_turn"] = True
   ```
   and replace them with:
   ```text
               bool(rec.get("scheduledTaskId")) or st["sched"], first_in_subagent)
           if first_in_subagent and not self.is_fork:
               # S4 (C19): tag a review fix pass from the brief; the text itself is never stored.
               self.db.execute("UPDATE agents SET brief_tag = ? WHERE agent_id = ?",
                               (cls.brief_tag(cls.text_of(content)), self.agent_id))
           st["sched"] = False
           st["seen_turn"] = True
   ```
5. `schema.sql`:
   - Line 1: replace `-- Token profiler data contract, schema version 6.` with `-- Token profiler data contract, schema version 7.`
   - Find `-- depends on its context_tokens, not only its model. See ingest/prices.py.` and replace it with:
     ```text
     -- depends on its context_tokens, not only its model. See ingest/prices.py.
     -- Version 7: agents.brief_tag, 'review-fix' when a subagent's first prompt
     -- names a C12 review fix pass, else NULL; never the prompt text. A version 6
     -- database gains the column in place (ingest/store.py open_db).
     ```
   - Replace `INSERT INTO meta (key, value) VALUES ('schema_version', '6');` with `INSERT INTO meta (key, value) VALUES ('schema_version', '7');`
   - Find `    meta_missing      INTEGER NOT NULL DEFAULT 0 CHECK (meta_missing IN (0, 1))` and replace it with:
     ```text
         meta_missing      INTEGER NOT NULL DEFAULT 0 CHECK (meta_missing IN (0, 1)),
         brief_tag         TEXT CHECK (brief_tag IS NULL OR brief_tag IN ('review-fix'))  -- version 7: see the header
     ```
6. `ingest/store.py`:
   - Replace `SCHEMA_VERSION = "6"` with `SCHEMA_VERSION = "7"`.
   - Find these two lines (as one find text):
     ```text
         version = db.execute("SELECT value FROM meta WHERE key = 'schema_version'").fetchone()[0]
         if version != SCHEMA_VERSION:
     ```
     and replace them with:
     ```text
         version = db.execute("SELECT value FROM meta WHERE key = 'schema_version'").fetchone()[0]
         if version == "6":
             _upgrade_6_to_7(db)
             version = SCHEMA_VERSION
         if version != SCHEMA_VERSION:
     ```
   - Find `def project_prefix(repo_root):` and insert directly above it:
     ```text
     def _upgrade_6_to_7(db):
         """Schema 6 to 7 in place (S4a): add agents.brief_tag. Safe to run twice."""
         if "brief_tag" not in {row[1] for row in db.execute("PRAGMA table_info(agents)")}:
             db.execute("ALTER TABLE agents ADD COLUMN brief_tag TEXT"
                        " CHECK (brief_tag IS NULL OR brief_tag IN ('review-fix'))")
         db.execute("UPDATE meta SET value = '7' WHERE key = 'schema_version'")
         db.commit()


     ```
   - Find `"start a new database (it is rebuilt from the transcripts).")` and replace it with `"start a new database (it is rebuilt from the transcripts); a version 6 database is upgraded in place.")`.
7. `report/db.py`: replace `SUPPORTED_SCHEMAS = ("1", "2", "3", "4", "5", "6")` with `SUPPORTED_SCHEMAS = ("1", "2", "3", "4", "5", "6", "7")`, and in the comment line above it replace `and methods; the campaign rollup needs version 4.` with `and methods; the campaign rollup needs version 4; 7 (S4a) adds agents.brief_tag.`
8. `test_contract.py`: in `test_schema_loads_with_version`, replace `.fetchone()[0], "6")` with `.fetchone()[0], "7")`. Find `    def test_thinking_is_a_subset_of_output(self):` and insert directly above it:
   ```text
       def test_v7_tags_review_fix_briefs(self):
           db = load_schema()
           self.assertIn("brief_tag", {row[1] for row in db.execute("PRAGMA table_info(agents)")})
           db.execute("INSERT INTO sessions (session_id, project_dir, first_ts, last_ts) VALUES ('s', 'p', 't', 't')")
           with self.assertRaises(sqlite3.IntegrityError):
               db.execute("INSERT INTO agents (agent_id, session_id, brief_tag) VALUES ('a', 's', 'brief text')")

   ```
9. `metrics/test_cost.py`: replace `self.assertEqual(record["stamp"]["profiler_schema"], "6")` with `self.assertEqual(record["stamp"]["profiler_schema"], "7")`. Leave `metrics/test_packets.py` (its version 5 check), `ingest/test_ingest_stops.py` (`OlderSchemaTest`, version 5 still refused) and the `"profiler_schema": "6"` fixtures unchanged: they are recorded inputs, still valid.

**Test type:** unit and ingest-integration (temp transcripts through `cli.main`), stdlib only. **Revert proof:** S8r; the six new tests fail before steps 3 to 6 (record the failure lines). Also record that reverting step 4 alone (the transcript change) makes `test_only_a_first_prompt_tags` fail, then restore it.

**Checks:** S9r OK, with the count up by 7; R4-style CRLF check (S4r) on all eight files; `store.py` at most 330 lines, `transcript.py` at most 420, `test_brief_tag.py` at most 250; the Grep tool finds `brief_tag` in `schema.sql` exactly twice (the header line and the column).

**Docs owed:** S4d carries them (same PR).

---

## S4b: the upgrade backfills tags from the transcripts

Agent: packet-coder, in `ah-s4`, after S4a's loop said `ship`. Size: about 20k. Commit subject: `feat(token-profile): the v6 to v7 upgrade backfills brief tags from the transcripts (ai-harness-docs S4b)`.

**Why:** fix passes that ran after W4a merged and before S4 shipped are in existing version 6 databases. An unchanged transcript is never read again (its offset is at the end), so the upgrade must read each subagent's first prompt itself.

**Rules:** § S4 rules. **Files:** `tools/token-profile/ingest/store.py`; new `tools/token-profile/ingest/test_brief_backfill.py`.

**Steps:**

1. Write `ingest/test_brief_backfill.py` first (S8r), at most 150 lines, CRLF. Import `write` and `tags` from `.test_brief_tag`, plus `sqlite3`, `tempfile`, `unittest`, `Path`, `from . import store` and `from .test_support import run_ingest`. A helper `downgrade(db_path)` runs `ALTER TABLE agents DROP COLUMN brief_tag` and `UPDATE meta SET value = '6' WHERE key = 'schema_version'`, commits and closes. Tests (temp dir per test, as in S4a):
   - `test_the_upgrade_backfills_the_tag`: `write(root, ["plain brief", "AH-99 review fix, pass 3 of 3"], fork_brief="review fix, pass 1 of 3")`; ingest; `downgrade`; `store.open_db(db).close()`; `tags(db)` is `{"c0": None, "c1": "review-fix", "f0": None}`.
   - `test_a_missing_transcript_is_skipped`: as above with briefs `["review fix, pass 1 of 3", "review fix, pass 2 of 3"]`; ingest; `downgrade`; delete c1's transcript (its path is `SELECT path FROM ingest_files WHERE path LIKE '%agent-c1.jsonl'`; `Path(p).unlink()`); `store.open_db(db).close()` raises nothing; `tags(db)` is `{"c0": "review-fix", "c1": None}`.
   - `test_an_upgraded_database_ingests_again`: after the first test's upgrade, `run_ingest(root / "p", db, prs=[])` is 0 and the tags are unchanged.
   Run S9r and record that the first two fail (c1 and c0 are `None` after the upgrade).
2. `store.py`: replace `from .transcript import Transcript` with:
   ```text
   from . import classify as cls
   from .shapes import unknown_shape
   from .transcript import Transcript
   ```
3. `store.py`, in `_upgrade_6_to_7`: find `        db.execute("UPDATE meta SET value = '7' WHERE key = 'schema_version'")` and replace it with:
   ```text
           backfill_brief_tags(db)
           db.execute("UPDATE meta SET value = '7' WHERE key = 'schema_version'")
   ```
   and change its docstring `add agents.brief_tag. Safe to run twice.` to `add agents.brief_tag and backfill it (S4b). Safe to run twice.`
4. `store.py`: find `def project_prefix(repo_root):` and insert directly above it:
   ```text
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


   ```

**Test type:** ingest-integration against temp transcripts. **Revert proof:** S8r (the first two tests fail before steps 2 to 4).

**Checks:** S9r OK, count up by 3; S4r on both files; `store.py` at most 380 lines.

**Docs owed:** S4d.

---

## S4c: the metrics read the tag

Agent: packet-coder, in `ah-s4`, after S4b's loop said `ship`. Size: about 30k. Commit subject: `feat(token-profile): packets, cost and overstep count a tagged packet-coder as fixer (ai-harness-docs S4c)`.

**Rules:** § S4 rules. **Files:** `tools/token-profile/metrics/packets.py`, `tools/token-profile/metrics/cost.py`, `tools/token-profile/metrics/overstep.py`; new `tools/token-profile/metrics/test_brief_role.py`.

**Steps:**

1. Write `metrics/test_brief_role.py` first (S8r), at most 200 lines, CRLF. Imports: `sqlite3`, `tempfile`, `unittest`, `Path`; `from ingest.prices import CURRENT`; `from metrics import cost, overstep, packets, registry`; and **helper functions only, never a TestCase class**: `from metrics.test_cost import build`, `from metrics.test_overstep import _detect, _req`, `from metrics.test_packets import CONFIG, WORKTREE, _agent, _github, _ledger, _record`. A helper `tag_coder(path)` runs `UPDATE agents SET brief_tag = 'review-fix' WHERE agent_id = 'a-coder'` on `sqlite3.connect(path)`, commits and closes; a helper `drop_column(path)` runs `ALTER TABLE agents DROP COLUMN brief_tag` and `UPDATE meta SET value = '6' WHERE key = 'schema_version'`. Tests:
   - `test_brief_role_flips_only_a_haiku_coder`: `packets.brief_role("implementer-haiku", "review-fix")` is `"fixer"`; `("implementer-haiku", None)` stays `"implementer-haiku"`; `("reviewer", "review-fix")` stays `"reviewer"`; `("implementer-domain", "review-fix")` stays `"implementer-domain"`; `(None, "review-fix")` is `None`.
   - `test_a_tagged_coder_is_a_fixer_in_the_packet_join`: agents `cm01-a1` (packet-coder, `2026-10-01T06:00:00Z`), `cm01-rev1` (packet-reviewer, `10:00`) and `cm01-a2` (packet-coder, `11:00`), all in `WORKTREE.format("lr-01")`, built with `_agent(...)` and `name=` their id, as in test_packets.py's `test_fix_named_coder_is_a_fixer_not_a_second_coder`; then `cm01-a2["brief_tag"] = "review-fix"`. `_record(packets.join_campaigns(_ledger(), {"agents": agents, "stops": {}}, _github(), CONFIG), "LR-01")` has `implementer_sessions` 1 and `fixer_sessions` 1. With the tag removed: `implementer_sessions` 2 and `fixer_sessions` 0.
   - `test_load_db_reads_the_tag`: `build(path)`; `tag_coder(path)`; `{a["agent_id"]: a["brief_tag"] for a in packets.load_db(path)["agents"]}` is `{"a-coder": "review-fix", "a-rev": None}`.
   - `test_cost_counts_a_tagged_coder_as_fixer`: `build(path)`; `tag_coder(path)`; open it read-only (`sqlite3.connect(f"file:{path}?mode=ro", uri=True)`); `config = registry.load_config()`; `out = cost.rollup(cost.load_requests(conn, config["window"]["since"], config["window"]["until"]), CURRENT, config["roles"])`; `out["camp-a"]["cost"]["by_role"]["fixer"]` is `0.0012` and `["implementer-haiku"]` is `0.0` (`assertAlmostEqual(..., places=9)`). Without `tag_coder` the same rollup gives implementer-haiku `0.0012` (test_cost.py's figure), so the test fails on revert.
   - `test_overstep_counts_a_tagged_coder_as_fixer`: `_detect([_req(agent_id="a3", agent_type="packet-coder", brief_tag="review-fix", worktree=".claude/worktrees/cm-07")])["fixer_requests"]` is `{"cm-07": 1}`; the same request without `brief_tag` gives `{}`.
   - `test_overstep_load_requests_carries_the_tag`: `build(path)`; `tag_coder(path)`; in `overstep.load_requests(path)`, every request whose `agent_id` is `"a-coder"` has `brief_tag` `"review-fix"`, every other has `None`, and there is at least one of each.
   - `test_a_database_without_the_column_reads_untagged`: `build(path)`; `drop_column(path)`; `packets.brief_tags(conn)` is `{}`; `packets.load_db(path)` gives `brief_tag` `None` for every agent; `overstep.load_requests(path)` gives `None` for every request.
   Run S9r and record the failures (`AttributeError: ... brief_role`, `KeyError: 'brief_tag'`, the 0.0012 split).
2. `metrics/packets.py`. Find `from report.stats import percentile` and replace it with:
   ```text
   from ingest.classify import REVIEW_FIX
   from report.stats import percentile
   ```
3. `metrics/packets.py`. Find these three lines (as one find text):
   ```text
   def role_of(agent, roles):
       """The campaign role of a worker from the config's `roles` map, or `other`."""
       return roles.get(_key(agent), "other")
   ```
   and replace them with:
   ```text
   def brief_role(role, tag):
       """A packet-coder session briefed as a review fix pass is a fixer (C12, C19); every other role is unchanged."""
       return "fixer" if role == "implementer-haiku" and tag == REVIEW_FIX else role


   def brief_tags(db):
       """{agent_id: brief_tag} for tagged agents; {} for a database without the column (below schema 7)."""
       if "brief_tag" not in {row[1] for row in db.execute("PRAGMA table_info(agents)")}:
           return {}
       return {row[0]: row[1] for row in db.execute("SELECT agent_id, brief_tag FROM agents WHERE brief_tag IS NOT NULL")}


   def role_of(agent, roles):
       """The campaign role of a worker from the config's `roles` map, or `other`; a tagged fix pass is a fixer."""
       return brief_role(roles.get(_key(agent), "other"), agent.get("brief_tag"))
   ```
4. `metrics/packets.py`, in `load_db`: replace `        modal = _modal_worktrees(db)` with the two lines `        modal = _modal_worktrees(db)` and `        tags = brief_tags(db)`. Then find `            agent["branch_hint"] = _branch_hint(db, agent["worktree"], last_ts)` and replace it with that line followed by `            agent["brief_tag"] = tags.get(agent["agent_id"])`. In its docstring, replace `A database below schema 6 raises ValueError.` with `` `brief_tag` is the agent's C19 tag, None below schema 7. A database below schema 6 raises ValueError. ``
5. `metrics/cost.py`. Find `from report.stats import distribution, share` and replace it with:
   ```text
   from report.stats import distribution, share

   from .packets import brief_role, brief_tags
   ```
   In `load_requests`, replace `    cur = conn.execute(ROWS_SQL, (since.rstrip("Z"), until.rstrip("Z")))` with the two lines `    tags = brief_tags(conn)` and that same line. Find `               "agent_key": row["custom_agent_type"] or row["agent_type"],` and replace it with that line followed by `               "brief_tag": tags.get(row["agent_id"]),`. In `_role`, replace `    return roles.get(row["agent_key"], "other") if row["agent_key"] else "other"` with `    return brief_role(roles.get(row["agent_key"], "other"), row.get("brief_tag")) if row["agent_key"] else "other"`.
6. `metrics/overstep.py`. Replace `from .packets import packet_of_session` with `from .packets import brief_role, brief_tags, packet_of_session`. Replace `        req["role"] = "coordinator" if req["agent_id"] is None else cfg["roles"].get(req["agent_type"])` with:
   ```text
           req["role"] = "coordinator" if req["agent_id"] is None \
               else brief_role(cfg["roles"].get(req["agent_type"]), req.get("brief_tag"))
   ```
   In `load_requests`, find these two lines (as one find text):
   ```text
           return [dict(row, calls=calls.get(row["request_id"], []), rows=rows.get(row["request_id"], []))
                   for row in db.execute(REQUEST_SQL)]
   ```
   and replace them with:
   ```text
           tags = brief_tags(db)
           return [dict(row, brief_tag=tags.get(row["agent_id"]), calls=calls.get(row["request_id"], []),
                        rows=rows.get(row["request_id"], []))
                   for row in db.execute(REQUEST_SQL)]
   ```
   Leave `REQUEST_SQL`, `ROWS_SQL` and every other SQL constant unchanged.

**Test type:** unit (pure functions) and integration against a schema-built temp database. **Revert proof:** S8r; also record that reverting step 5's `_role` line alone makes `test_cost_counts_a_tagged_coder_as_fixer` fail, then restore it.

**Checks:** S9r OK, count up by 7 (every existing metrics test still passes: `role_of` is unchanged for an untagged agent); S4r on the four files; `packets.py` at most 410 lines, `overstep.py` at most 390, `cost.py` at most 225.

**Docs owed:** S4d.

---

## S4d: the docs

Agent: packet-coder, in `ah-s4`, after S4c's loop said `ship`. Size: about 15k. Commit subject: `docs(token-profile): schema 7 and the fixer role of a review fix pass (ai-harness-docs S4d)`.

**Rules:** § S4 rules (S8r does not apply: docs only). **Files:** `tools/token-profile/README.md`, `tools/token-profile/transcript-format.md`, `docs/guides/token-profiling.md`.

**Steps:**

1. `tools/token-profile/README.md`:
   - Replace `The SQLite schema the ingest writes and the reports read. Version 6. |` with `The SQLite schema the ingest writes and the reports read. Version 7. |`.
   - Replace `A database of an older schema version is refused: start a new one (about two minutes).` with `A version 6 database is upgraded in place to version 7 (the brief tag is backfilled from the transcripts); an older one is refused: start a new one (about two minutes).`
   - Find `section.` at the end of the paragraph that starts `The campaign metrics add per-campaign cost, packet, CI, defect and overstep figures` (find text: ``[Campaign metrics](../../docs/guides/token-profiling.md#campaign-metrics) section.``) and replace it with ``[Campaign metrics](../../docs/guides/token-profiling.md#campaign-metrics) section. A `packet-coder` session whose first prompt is a review fix pass ("review fix, pass N of 3", schema 7 `agents.brief_tag`) counts as `fixer`, not `implementer-haiku`, in the cost, packet and overstep figures.``
2. `tools/token-profile/transcript-format.md`: find the paragraph that starts `Never store message text in \`triggers\`.` (one line; find text: ``Never store message text in `triggers`.``) and insert this paragraph after that line, with one blank line before it:
   ```text
   **Brief tag (schema version 7).** The only fact taken from a prompt's text is `agents.brief_tag`: `review-fix` when a subagent's first prompt (rule R11) matches `review fix, pass <1-3> of 3`, case-insensitive (a C12 review fix pass), else NULL. Forks are never tagged, because their first prompt is a copy of the parent's history. The text itself is never stored. A version 6 database gets the column on its next open, and the ingest fills it from each subagent transcript's first prompt.
   ```
3. `docs/guides/token-profiling.md`:
   - Replace `(it is at version 6)` with `(it is at version 7; a version 6 database is upgraded in place)`.
   - Replace `- Campaign cost: list-price cost of a campaign's requests, by role and PR.` with `- Campaign cost: list-price cost of a campaign's requests, by role and PR. A \`packet-coder\` session briefed as a review fix pass counts as \`fixer\` (schema 7).` (without the backslashes).
   - Replace `` `packets` needs a schema v6 database, so re-ingest first. `` with `` `packets` needs a schema v6 or newer database, and the fixer role of a review fix pass needs v7, so ingest first. ``
   - Set the guide's `Last updated` line (if it has one) to today (`Get-Date -Format yyyy-MM-dd`).

**Test type:** none (docs). `test_contract.py` reads transcript-format.md's trigger table; the new paragraph is outside it (after the table section). **Checks:** S9r OK (the contract tests read transcript-format.md); S4r on the three files; the Grep tool finds `Version 7` once in the token-profile README and `schema version 7` once in transcript-format.md.

**Docs owed (the PR lists this row):** doc-update-map row "The token profiler (`tools/token-profile/`), its scheduled jobs, or the per-PR stats comment format": tools/token-profile/README.md, docs/guides/token-profiling.md, the schema version in schema.sql (S4a) and the contract doc (transcript-format.md), and the Campaign metrics sections of the guide and the README.

---

## S4 wave loop

After S4d's packet loop says `ship`, one wave pass (C12, wave row) over `origin/main..HEAD` in `ah-s4`. The reviewer brief adds, as cross-packet questions: does any path store prompt text (C19 item 1); do the ingest (S4a), the backfill (S4b) and the three metrics readers (S4c) agree on the tag value, the fork rule and the first-prompt rule; does every reader still work on a version 6 database. Fix notes go to one fresh packet-coder in `ah-s4`.

## Not done (follow-ups AH-24 lists)

- The coordination report (`tools/token-profile/report/`, README line 71) still names `packet-review-fixer` as history (D-AH42); it is not taught the tag.
- A fix pass dispatched under another agent type (a Sonnet `rust-gameserver-dev` ruling implementer) is not tagged; its role stays `implementer-domain`.
- A brief that paraphrases the phrase ("fix pass 2") is not tagged. C14 fixes the wording; AH-24's campaign review checks the briefs it can see.
- Continuation sessions ("continuation N of 3") stay implementer sessions, which is correct.
