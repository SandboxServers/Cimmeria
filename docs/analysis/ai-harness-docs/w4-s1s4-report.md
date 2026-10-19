# ai-harness-docs: the S1 veto and the S4 design (2026-10-18)

Architect report. I changed only files under `docs/analysis/ai-harness-docs/`, in the main checkout, and left them uncommitted; AH-W40 ships them. Nothing was dispatched, compiled or pushed.

- New finding: F56.
- New decisions: D-AH57 (S1 vetoed) and D-AH58 (the S4 design).
- New contract: C19.
- New packets: S4a, S4b, S4c and S4d, in work-packets-s4.md.

## S1: vetoed by the owner (D-AH57)

S1 is out of the schedule:

- removed from the order (work-packets-w4b.md § Order, README D-AH56, RESUME);
- removed from the BlockedDecision list;
- the packet-table row reads "Dropped (owner veto, D-AH57)";
- § S1 in work-packets-w4b.md is now a stub; the full packet text stays in git history;
- the spicy-ideas pitch row is marked Vetoed.

What depended on it:

- **AH-23b's rules-and-gotchas bullet "No hook refuses raw git".** S1's step 3 would have rewritten it to name the guard. Without S1 the bullet stays as AH-23b writes it, and it is true.
- **AH-16's close-out list.** Its step 4 now records S1 as "Not done: the owner vetoed it, 2026-10-18 (D-AH57)".
- **Nothing else.** No other packet's find text or check named S1's guard; I grepped the ledger to confirm.

## S4: approved; the open question answered

**The question:** does the token-profile database store the brief text?

**The answer: no, and it does not need to (F56).** Schema v6 has no text column, and transcript-format.md says "Never store message text". But the ingest already reads each subagent's first prompt:

- `Transcript._user` computes `first_in_subagent` for rule R11;
- `seen_turn` persists in parse_state, so an incremental run does not see the prompt twice;
- a reread transcript resets that state, so its prompt is seen again.

The ingest can therefore derive a fixed tag at that point and store only the tag. This is option (d) without the stored text. It is exact and needs no join against commit subjects, which option (b) did.

### Design (D-AH58, contract C19)

- **Column:** `agents.brief_tag TEXT CHECK (brief_tag IS NULL OR brief_tag IN ('review-fix'))`, in schema 7. No prompt text is stored anywhere.
- **Pattern:** `\breview fix, pass [1-3] of 3\b`, case-insensitive, defined in `ingest/classify.py`.
  - A real C14 fix-pass brief has a digit, so it matches.
  - The template text "pass N of 3" and "pass 4 of 3" do not match.
- **When the tag is set:**
  - only on a subagent's first user prompt, never on a fork's;
  - on every first-prompt read, to the tag or to NULL, so a reread gives the same result;
  - later prompts never change it.
- **Upgrade:** `store.open_db` upgrades a version 6 database in place. It adds the column, backfills the tag from each subagent transcript's first prompt (S4b), and sets the version to 7. Versions below 6 are still refused (the existing tests stay).
  - **Why the upgrade:** today any version mismatch forces a full rebuild.
  - **Why the backfill:** fix passes that run between W4a and S4 would otherwise stay untagged, because an unchanged transcript is never read again.
- **Roles:** `metrics.packets.brief_role(role, tag)` turns `implementer-haiku` plus `review-fix` into `fixer`. Three readers use it:
  - the packet join (`role_of`);
  - the cost rollup (`cost._role`);
  - the overstep detector.

  A database without the column reads as untagged, so the reports keep accepting versions 1 to 7.
- **Brief caveat (C19 item 6):** an S4 coder or reviewer brief must never contain the phrase with a digit. Test fixtures may contain it; only first prompts count.

### Packets (work-packets-s4.md)

All four packets run in one worktree, `ah-s4` (branch `ai-harness-docs/s4-fix-pass-role`), cut from `origin/main` after W4a merges. They run one after another, each with its own packet loop. One wave loop then covers all four, and they ship as one PR. Every packet has `Host: windows` and inlines the rules S1r to S10r: worktree, shell, files, CRLF (the `.py` and `.sql` targets are CRLF too), exact finds, caps, small diffs, tests first, check, commit.

**Check (all four):** `Set-Location <wt>; python -m unittest discover -s tools/token-profile -p "test_*.py"` passes. The baseline is 538 tests (2026-10-18).

**Sizes and dependencies:**

| Packet | Size | Depends on |
|---|---|---|
| S4a | about 30k | W4a merged (step 1 guard) |
| S4b | about 20k | S4a |
| S4c | about 30k | S4b |
| S4d | about 15k | S4c |

**S4a: schema 7, the tag from the first prompt, no text stored.**

- Files:
  - `schema.sql`, `ingest/store.py` (the upgrade only), `ingest/classify.py`, `ingest/transcript.py`, `report/db.py`;
  - tests: `test_contract.py` (the version assert and a new column test), `metrics/test_cost.py:283` (the stamp changes from "6" to "7");
  - new: `ingest/test_brief_tag.py`.
- New tests (6):
  - the pattern;
  - only a first prompt tags;
  - a fork is never tagged;
  - the tag survives an incremental run;
  - no prompt text in any table (a marker string is searched across every table);
  - a version 6 database is upgraded in place.
- Revert proof: the tests fail first with AttributeError, "no such column" and SystemExit. Reverting the transcript change alone makes "only a first prompt tags" fail.

**S4b: the upgrade backfills the tag.**

- Files: `ingest/store.py` (`backfill_brief_tags`, `_first_prompt_tag`); new `ingest/test_brief_backfill.py`.
- New tests (3):
  - the backfill tags the fix pass and leaves the plain coder and the fork NULL;
  - a missing transcript is skipped;
  - an upgraded database ingests again.
- Revert proof: without the backfill the tags stay NULL.

**S4c: the metrics read the tag.**

- Files: `metrics/packets.py` (`brief_role`, `brief_tags`, `role_of`, `load_db`), `metrics/cost.py`, `metrics/overstep.py`; new `metrics/test_brief_role.py`, which imports helpers only.
- New tests (7):
  - the role mapping;
  - a tagged coder in the packet join (implementer_sessions 1, fixer_sessions 1);
  - load_db reads the tag;
  - the cost split (fixer 0.0012, implementer-haiku 0.0);
  - an overstep fixer request;
  - overstep `load_requests` carries the tag;
  - a database without the column reads as untagged.
- Revert proof: the tests fail first. Reverting `_role` alone makes the cost test fail.

**S4d: docs.**

- Files: `tools/token-profile/README.md`, `tools/token-profile/transcript-format.md` (the "Brief tag" paragraph), `docs/guides/token-profiling.md`.
- Doc-update-map row: "The token profiler (`tools/token-profile/`), its scheduled jobs, or the per-PR stats comment format". The PR body lists it.

**Parallelism.** The packets are file-disjoint from W4b, W4c, S3, S5 and S6, so they run in parallel with those. AH-24 (W5) now also waits for S4.

**Not done (AH-24 lists these as follow-ups):**

- The coordination report still names `packet-review-fixer`.
- A fix pass dispatched under another agent type, such as the Sonnet ruling implementer, is not tagged.
- A paraphrased brief is not tagged.
- Continuations stay implementer sessions, which is correct.

## Files touched (all under docs/analysis/ai-harness-docs/, CRLF)

- **New:** work-packets-s4.md, this report.
- **Edited:**
  - README.md: the header, F53 amended, F56, D-AH49, D-AH52 and D-AH56 amended, D-AH57, D-AH58, the BlockedDecision list, the packet table, the ID note, the spicy section;
  - work-packets-w4b.md: § Order, the Dropped and Approved line, AH-24's dependencies, the AH-16 addenda, the spicy intro, the § S1 stub, the § S4 pointer;
  - work-packets-w4.md: the header, and AH-W40 now copies 18 files;
  - RESUME.md (32 lines).

## Final dispatch order (everything remaining)

Rule 0 applies at every merge: nothing merges while `main` is red.

1. **AH-W40.** `ops` makes `ah-w4-ledger`. A packet-coder copies the 18 ledger files; its guard hash `3b2a99d6e` was unchanged at the time of this report. `ops` ships and merges it. It is docs-only and has no review loop.
2. **W3c.** AH-11g and AH-11h, each with its packet loop and then a wave loop. Then **AH-11i**, an owner action: post pr-1797-close.md and close #1797.
3. **W4a.** AH-17 to AH-21 in their own worktrees, merged into `ah-w4a`, then AH-22 there, then the wave loop. It ships as one PR. The user relaunches afterwards.
4. **In parallel after W4a:**
   - **W4b:** AH-23a, then AH-23b, in `ah-23`; one PR. S2 is folded into AH-23a.
   - **S6:** one PR.
   - **S5a** and **S5b**, which also need AH-11g merged: one PR each.
   - **S4:** S4a, S4b, S4c and S4d in sequence in `ah-s4`; one PR.
5. **After their prerequisites:**
   - **W4c** (AH-25 to AH-27) after W4b; one PR. The user relaunches afterwards.
   - **S5c** after S5a and S5b.
   - **S3** after W4b.
6. **W5:**
   - **AH-24** (the campaign review loop) after W4b, W4c, AH-11i, S3, S5c, S6 and S4;
   - then **AH-15** (the owner's smoke test; D-AH33 decides its Linux leg);
   - then **AH-16** (close-out).

Retire `ah-17` to `ah-21` and `ah-25` to `ah-27` with `--merged-into <PR>` once S5c has merged. Before that, the owner retires them with `--force`.

## Measured versus not measured

**Verified in the repo today:**

- the schema v6 columns, and that no text is stored;
- the ingest code paths:
  - R11 `first_in_subagent`;
  - `seen_turn` in parse_state;
  - the fork flag;
  - the `ingest_files` path and kind;
- `open_db`'s refusal of any other version;
- the role joins in packets.py, cost.py and overstep.py;
- the test helpers each S4 test imports;
- the token-profile suite baseline: 538 tests, OK;
- that SQLite 3.45.3 supports `ALTER TABLE ... DROP COLUMN` and `ADD COLUMN ... CHECK`;
- that every target file is CRLF;
- the AH-W40 guard hash.

**Not verified:**

- **The packet sizes.** They are estimates against the 40k cap.
- **The exact test counts after each packet** (+7 for S4a: six in the new file plus one in test_contract.py; +3 for S4b; +7 for S4c). They are counted from the packet text, not from a run.
- **That every find text matches exactly once in the S4 worktree.** I checked them against main on 2026-10-18; W4a does not touch `tools/token-profile/`.
- **That the CI Linux run passes.** It runs the same stdlib tests.

## BlockedDecision (for the orchestrator to put to the owner)

- **D-AH33 (the only open owner decision):** is AH-15's Linux or WSL leg required (recommended) or best effort?
