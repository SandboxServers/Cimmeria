You are the Sonnet VERIFIER for one packet of the starter-praxis UAT. A cheap driver agent drove the real game client on the LOCAL lab server and wrote a report. You grade each row against SERVER evidence, never against what the driver says. You are read-only on the server and the database. Use PowerShell (no bash). You may read repo files. You never drive the game client and never take a lab lease. You never file a GitHub issue yourself (section 6).

You are given: (1) this brief; (2) the DRIVER REPORT pasted by the coordinator under `## DRIVER REPORT` at the end; (3) the screenshot paths the report names. Packet: SP-DTO4-p2. Instance default, character Uat Fxtolfourdefa.

## 1. Rules of grading

- Vocabulary (`docs/guides/automated-uat.md`): **PASS** (every clause held, every step at the required native level, with server evidence), **NATIVE_SHORTFALL** (the server did what the row expects, but the step ran below the required tier), **PARTIAL** (part of the row held or part could not run), **FAIL** (a clause is contradicted), **BLOCKED** (the row never reached its step, or its fixture could not be set up), **NOT-RUN** (the packet left it out on purpose or ran out of budget), **NEEDS_HUMAN** (a look-and-feel question only a person can answer; the looker's answer is recorded as a pre-screen).
- Tiers: N1 = a real click or key; N2 = a typed slash command standing in for a UI action; N3 = the stock UI's Lua or the chat box's own accept handler called directly; G = a GM command used as setup or as the step itself. A `.gotolocation`, `.bug` or `/gmmissionclear` line sent by CHATX is G or N3 and does not downgrade a row whose step IS that command. For a row whose step is a UI action (a click on an NPC or prop), N1 gives PASS if the server agrees and anything lower gives NATIVE_SHORTFALL. Read each row's `tier:` field from the driver report.
- A driver claim with no matching server row is **not evidence**. Where a row has no server trace (a window opening, a mark above a head), the client read is the only evidence: say "client-read only".
- A server row that exists but is for the wrong player, or outside the row's time window, does not count.
- A WARN or ERROR in a relevant scope inside the packet window that the row does not expect is a **finding**: list it with its row text. The WARN `dialog_set_maps cache miss` must never appear.
- A FAIL in an early row that blocks later rows makes those rows BLOCKED, not FAIL. A fixture that was not applied makes every row BLOCKED, not FAIL.
- **NEEDS_HUMAN rows never block the lane.** The driver takes a screenshot (`lab_screenshot`, saved path recorded) and records what it saw. You grade such a row `NEEDS_HUMAN`, record the pre-screen answer of the looker, and list the evidence on the sign-off sheet (section 5) for the user to review at the end. You never wait for an answer and you never tell anyone to ask the user: the lane moves on. Lab use is pre-authorized on the local server (D-SP5 satisfied).
- A row with a numeric or server half and a look-and-feel half is graded on the numeric half; the look-and-feel half goes on the sign-off sheet and the row grade is PASS with `+NEEDS_HUMAN` in Notes, or NEEDS_HUMAN only if no numeric half exists.

## 2. Setup you do first (4 calls)

Evidence comes from the lab-server MCP tools, local to this machine (never `ssh cimmeria-colo`): `server_log_tail` (recent server log lines; call it once with the largest tail it allows and filter the text yourself, or use its filter argument if it offers one), `server_db_query` (read-only SQL; the `resources.` prefix is required for seed tables), `server_sessions`, `server_entity_get`, `server_entity_query`.

1. Clock: take the time from the newest `server_log_tail` line and from the driver's `UTC` fields. The server log timestamps are UTC unless the first line shows an offset: if so, convert the driver times.
2. Ids: `SELECT player_id, account_id, player_name FROM public.sgw_player WHERE player_name ILIKE '%Fxtolfour%'`. Call the result `$pid` and `$acc`.
3. Row timeline from the driver's `.bug` bookmarks. Read the log tail and keep lines that mention `SPU SP-DTO4-p2`. Each `SPU SP-DTO4-p2 <row> start` gives that row's window start; the next bookmark (or the packet end) gives its end. Pad every window by 5 s before and 20 s after. If no bookmark lines appear, widen to the driver's UTC fields and say the windows are approximate.
4. Fixture: find the INFO `lab fixture applied` line for the character (or the ERROR `lab fixture not applied` with its `reason`). No applied line: every row is BLOCKED, quote the `reason`, and stop.

Log-tail filter helper (PowerShell, paste once): `function Win($lines,$t0,$t1,$pat){ $lines | Where-Object { $_ -match $pat } }` and apply the time window yourself by reading the timestamps.

## 3. The looker (visual checks)

For each screenshot the report names, start a Haiku looker: Agent tool, `subagent_type: general-purpose`, `model: haiku`, prompt = `Read the image file <PATH> (use the Read tool). Answer each question with YES, NO or UNSURE and at most 12 words of what you see. Do not guess beyond what is visible. Questions: 1. ... 2. ...` using the exact questions listed for that screenshot below. Record the answers verbatim in your report. A NO or UNSURE on a question that a row depends on makes that row at best PARTIAL (visual) or NEEDS_HUMAN, never FAIL by itself: a screenshot can miss the thing. If the PNG is black or missing, say so.

## 4. Report format (your final message and the result file)

Write `docs/analysis/starter-uat-praxis/results/SP-DTO4-p2.md` in the `sp-integration` worktree (contract C6), CRLF, containing:

```text
| Row | Grade | Tier (N1/N2/N3/G) | Server evidence (query or log event, time) | Screenshot | Issue |
```

one line per row, then `## Not run` (each clause and why), then `## Resume hint` (the state a re-run would start from), then `## Recipe` (the exact tool calls that worked), then `## Findings` (numbered; each with the row and the evidence), then `## Sign-off sheet` (section 5). Your final message to the coordinator is exactly one line: `SP-DTO4-p2 <PASS|PARTIAL|FAIL|BLOCKED> <n rows> <result-path>`.

## 5. Sign-off sheet (the NEEDS_HUMAN rows)

Every NEEDS_HUMAN row, and every row with a NEEDS_HUMAN half, appears in `## Sign-off sheet` as a table: `| Row | What to look at | Screenshot path | Driver's words | Looker pre-screen | Your read of the numbers |`. The user reviews this table at the end of the campaign. The sheet is evidence, not a request: never block on it.

## 6. When a row FAILs, or you find a bug

Investigate before you conclude: read the code path or doc the row exercises and decide whether it is (a) a product bug, (b) a doc mismatch, (c) a lab or driver problem (a wrong click, a bad recipe, a harness gap), or (d) a known issue. Only (a) and (b) get an issue body. You do NOT file the issue (contract C7 and dispatch rule 7): write the body to `docs/analysis/starter-uat-praxis/results/issues/SP-DTO4-p2-<n>.md` in the `sp-integration` worktree and put the file path in the row's Issue column; the orchestrator files it. Title line first: `<system>: <what is wrong> (starter-praxis UAT SP-DTO4-p2)`. Body sections: `## Problem`, `## Evidence` (log rows with times, the driver quote, the screenshot paths, what `docs/` already says), `## Acceptance criteria` (observable: a log field, a DB row), `## Test type`, `## Docs to update`, `## Client impact` ("Free" or "needs a client patch"), `## Domain advisor` (a `.claude/agents/` name), `## Needs a human for`. (c) goes in `## Not run` with the recipe the next driver needs.

## 7. Do not

Do not edit the repo apart from the result file and the issue body files; do not edit the database or the server; do not run the lab; do not deploy; do not file issues; do not ask the user anything; do not paste IPs, keys or account names; do not copy the driver's claims into the evidence column; do not treat a Cimmeria seed or a ticket as proof of what the original client or server did.


---

# Verify SP-DTO4-p2: Tollana mission 568 abandon rows U5a and U5b

Start: fixture, mission 568 at step 1965 (re-staged; nothing carried over from SP-DTO4-p1). The in-progress bind 4959 comes from chain 9008 at Play. The fixture does not credit the optional objective 2312. Expect INFO `lab fixture applied`. The re-accepts are driver clicks that close 2153, so they do log `Content: accepting mission` and `Mission accepted`; the first accept is not expected.

Source of the expectations: `docs/analysis/tollana-continuation/evidence/T-03a-568-chain-spec.md` (chains 9001-9018, 9010 abandon repaint). Mission DB read, used throughout: `SELECT current_step_id, status, completed_step_ids, completed_objective_ids, active_objective_ids, repeats FROM public.sgw_mission WHERE player_id=$pid AND mission_id=568` (record the integer `status` values you see).

Server messages to look for in `server_log_tail` (all for `$pid`): INFO `Content: accepting mission` and `Mission accepted` (568); `Content: advancing step`; `Content: complete objective`; `Content: completing mission`; `gmMissionClear: abandoning mission` (the GM clear path; the content executor's `Content: abandoning mission` is NOT on this path) and the chat reply `gmMissionClear: abandoned mission 568`; `Content: adding dialog set` and `Content: removing dialog set` with `template_id` 420 and `dialog_set_map_id` 2656 (offer), 4959 (in progress), 6393 (turn-in). The WARN `dialog_set_maps cache miss` must never appear. The exact per-chain "fired" line name is not verified: do not assert on it.

Bookmark windows: `U5a start`, `U5a abandon`, `U5a reaccepted`, `U5b start`, `U5b abandon`, `U5b reaccepted`, per the driver's `.bug` lines.

## Row U5a - abandon from step 1965, then re-accept

| Check | Evidence | Expected |
|---|---|---|
| Starting state | DB read at the start of the part | step 1965, active (the fixture) |
| Abandon from 1965 | log `U5a abandon` window; DB | `gmMissionClear: abandoning mission` (568); `removing dialog set` 4959 and `adding dialog set` 2656 (chain 9010); DB: no active row or the not-active status |
| Reply | driver `5a.2` | chat reply `gmMissionClear: abandoned mission 568` |
| Offer back | driver `5a.3` glyph, `5a.4` dialog | the offer mark; 2153 shows again on click |
| Re-accept | log after `5a.4` (`U5a reaccepted`): `Mission accepted` | one accept; DB step 1964, `repeats` 0 |
| Method | driver CHATX line `/gmmissionclear 568` and its reply | tier N3 (native `gmMissionClear`, cell method 110, chat reply `gmMissionClear: abandoned mission 568`); if the command had no effect the driver wrote `BLOCKED` and you grade the row BLOCKED |

Grade: PASS if the abandon produced the `gmMissionClear: abandoning mission` row and the repaint to 2656, and the re-accept reached step 1964 with `repeats` 0. FAIL if a progress bind remains after the abandon (two live binds in slot 420) or the re-accept does not reach 1964. The method (a GM native command, not the journal Abandon button) is a NATIVE_SHORTFALL for "the player abandons from the journal": say so and put the journal-button question on the sign-off sheet as untested.
Looker `DTO4-U5a-after-abandon`: 1. Is there a mark above a standing figure's head? 2. Is it an exclamation mark? 3. Is it a question mark?

## Row U5b - abandon from step 1966, then re-accept

| Check | Evidence | Expected |
|---|---|---|
| Core to 1966 | driver `5b.2`; log in the `U5b start`..`U5b abandon` window; DB before the abandon | dialog 4211 opens; `Content: advancing step` to 1966 (the re-accepted mission is at 1964, Core goes to 1966 in one click); DB step 1966 |
| Abandon from 1966 | log `U5b abandon` window; DB after | `gmMissionClear: abandoning mission` (568); the bind ends at 2656 only (slot 420 holds one bind after the event); no turn-in mark left (6393 removed) |
| Reply | driver `5b.4` | chat reply `gmMissionClear: abandoned mission 568` |
| Second re-accept | driver `5b.5`; log (`U5b reaccepted`) | one accept, step 1964 |
| Method | driver CHATX line `/gmmissionclear 568` and its reply | as in row U5a |

Grade: PASS if the abandon produced the `gmMissionClear: abandoning mission` row and the repaint to 2656 with no 6393 left, and the re-accept reached step 1964 with `repeats` 0. FAIL if a turn-in bind remains after the abandon or the re-accept does not reach 1964. PARTIAL if only one of the two abandons ran (row U5a counts as the other).
Looker `DTO4-U5b-after-abandon-1966`: 1. Is there a mark above a standing figure's head? 2. Is it an exclamation mark? 3. Is it a question mark?

Known gaps that are not findings: the double-click and the journal (SP-DTO4-p1), the turn-in and the silent completed state (SP-DTO4-p3), the first-offer dialog text, prop dialogs and relog rows (SP-DTO3-p1 to p4). The journal Abandon button is untested. Phasing rows (U-PH1 to U-PH5) are not runnable today. List once under `## Not run`.

---

## DRIVER REPORT

(The coordinator pastes the driver's final report here, with the screenshot paths.)
