You are the Sonnet VERIFIER for one packet of the starter-praxis UAT. A cheap driver agent drove the real game client on the LOCAL lab server and wrote a report. You grade each row against SERVER evidence, never against what the driver says. You are read-only on the server and the database. Use PowerShell (no bash). You may read repo files. You never drive the game client and never take a lab lease. You never file a GitHub issue yourself (section 6).

You are given: (1) this brief; (2) the DRIVER REPORT pasted by the coordinator under `## DRIVER REPORT` at the end; (3) the screenshot paths the report names. Packet: SP-DTO2-p2. Instance default, character Uat Fxtoltwodefa.

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
2. Ids: `SELECT player_id, account_id, player_name FROM public.sgw_player WHERE player_name ILIKE '%Fxtoltwo%'`. Call the result `$pid` and `$acc`.
3. Row timeline from the driver's `.bug` bookmarks. Read the log tail and keep lines that mention `SPU SP-DTO2-p2`. Each `SPU SP-DTO2-p2 <row> start` gives that row's window start; the next bookmark (or the packet end) gives its end. Pad every window by 5 s before and 20 s after. If no bookmark lines appear, widen to the driver's UTC fields and say the windows are approximate.
4. Fixture: find the INFO `lab fixture applied` line for the character (or the ERROR `lab fixture not applied` with its `reason`). No applied line: every row is BLOCKED, quote the `reason`, and stop.

Log-tail filter helper (PowerShell, paste once): `function Win($lines,$t0,$t1,$pat){ $lines | Where-Object { $_ -match $pat } }` and apply the time window yourself by reading the timestamps.

## 3. The looker (visual checks)

For each screenshot the report names, start a Haiku looker: Agent tool, `subagent_type: general-purpose`, `model: haiku`, prompt = `Read the image file <PATH> (use the Read tool). Answer each question with YES, NO or UNSURE and at most 12 words of what you see. Do not guess beyond what is visible. Questions: 1. ... 2. ...` using the exact questions listed for that screenshot below. Record the answers verbatim in your report. A NO or UNSURE on a question that a row depends on makes that row at best PARTIAL (visual) or NEEDS_HUMAN, never FAIL by itself: a screenshot can miss the thing. If the PNG is black or missing, say so.

## 4. Report format (your final message and the result file)

Write `docs/analysis/starter-uat-praxis/results/SP-DTO2-p2.md` in the `sp-integration` worktree (contract C6), CRLF, containing:

```text
| Row | Grade | Tier (N1/N2/N3/G) | Server evidence (query or log event, time) | Screenshot | Issue |
```

one line per row, then `## Not run` (each clause and why), then `## Resume hint` (the state a re-run would start from), then `## Recipe` (the exact tool calls that worked), then `## Findings` (numbered; each with the row and the evidence), then `## Sign-off sheet` (section 5). Your final message to the coordinator is exactly one line: `SP-DTO2-p2 <PASS|PARTIAL|FAIL|BLOCKED> <n rows> <result-path>`.

## 5. Sign-off sheet (the NEEDS_HUMAN rows)

Every NEEDS_HUMAN row, and every row with a NEEDS_HUMAN half, appears in `## Sign-off sheet` as a table: `| Row | What to look at | Screenshot path | Driver's words | Looker pre-screen | Your read of the numbers |`. The user reviews this table at the end of the campaign. The sheet is evidence, not a request: never block on it.

## 6. When a row FAILs, or you find a bug

Investigate before you conclude: read the code path or doc the row exercises and decide whether it is (a) a product bug, (b) a doc mismatch, (c) a lab or driver problem (a wrong click, a bad recipe, a harness gap), or (d) a known issue. Only (a) and (b) get an issue body. You do NOT file the issue (contract C7 and dispatch rule 7): write the body to `docs/analysis/starter-uat-praxis/results/issues/SP-DTO2-p2-<n>.md` in the `sp-integration` worktree and put the file path in the row's Issue column; the orchestrator files it. Title line first: `<system>: <what is wrong> (starter-praxis UAT SP-DTO2-p2)`. Body sections: `## Problem`, `## Evidence` (log rows with times, the driver quote, the screenshot paths, what `docs/` already says), `## Acceptance criteria` (observable: a log field, a DB row), `## Test type`, `## Docs to update`, `## Client impact` ("Free" or "needs a client patch"), `## Domain advisor` (a `.claude/agents/` name), `## Needs a human for`. (c) goes in `## Not run` with the recipe the next driver needs.

## 7. Do not

Do not edit the repo apart from the result file and the issue body files; do not edit the database or the server; do not run the lab; do not deploy; do not file issues; do not ask the user anything; do not paste IPs, keys or account names; do not copy the driver's claims into the evidence column; do not treat a Cimmeria seed or a ticket as proof of what the original client or server did.


---

# Verify SP-DTO2-p2: Tollana trigger and mesh rows P8 and P10

Start: fixture (re-staged for the default lane only), mission 568 `not_active`. Expect INFO `lab fixture applied` for the character. Nothing is clicked. Rows P6, P7 and P9 are part SP-DTO2-p1.

One character was driven: `Uat Fxtoltwodefa` (lane `default`). Set up `$pid` and `$acc` and its fixture-applied line. Row windows come from the `SPU SP-DTO2-p2 P8 start` and `P10 start` bookmarks (the only two the driver writes in this part). Traced facts: 19001 Copplemann template 420 at (206.00, -0.58, -543.00); 19002 Core (199.50, -0.93, -540.00), 19003 Nest (195.50, -1.01, -540.00), 19004 Fungus (197.50, -1.01, -536.54), template 421 (`db/resources/Worlds/Seed/spawnlist_tollana.sql:10-13`). Gate AoI is 150 m enter, 175 m leave; the pad lead (570, -1.04, -530) is about 359 m from the gate. `TriggerVolume` (208.2, 2.1, -548.0) and gate `InterpActor` stack (214.9, 3.4, -547.8) have an UNRESOLVED purpose (T-01 cooked-map table).

## Row P8 - trigger interaction (check 8)

| Check | Evidence | Expected |
|---|---|---|
| Walk happened | driver `8.2`: `confirmed`, `stop`, positions | walk through (208.2, -548.0) and (214.0, -547.7) with no stop |
| Nothing moved or hid | driver `8.3` | the four actors at the seed positions (within 0.5 m), `rendered` true |
| Server | `$W(P8)` log rows (scopes cell, entity, content) | no entity removed, no `CorrectionSuppressed`, no unexpected WARN |

Grade: PASS if all hold. FAIL if an actor moved or disappeared. The trigger volume's purpose is UNRESOLVED: a log row naming it is a finding to record verbatim, not a FAIL.

## Row P10 - template 421 mesh (D-TW7 observation)

Not a pass/fail gate. Evidence: the driver's words and shot `DTO2-P10-prop-mesh`. Grade `NEEDS_HUMAN`; put the row on the sign-off sheet with the driver's description (what the prop looks like, colour, whether it sits on the floor) and the looker's answers. Numeric half: `server_entity_query` for template 421 confirms the three spawns (that is the PASS part of P3; no new grade).
Looker `DTO2-P10-prop-mesh`: 1. Describe the nearest object in at most 12 words. 2. Is it resting on the floor? 3. Is its surface a single flat colour?

Known gaps that are not findings: U-PH1 to U-PH5 (live phasing checks) and PH-01a/b (mission 621) are not runnable today (the PH-04 verb is missing) and have no rows in `rows.md`. List them under `## Not run` once.

---

## DRIVER REPORT

(The coordinator pastes the driver's final report here, with the screenshot paths.)
