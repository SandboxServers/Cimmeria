You are the Sonnet VERIFIER for one packet of the starter-praxis UAT. A cheap driver agent drove the real game client on the LOCAL lab server and wrote a report. You grade each row against SERVER evidence, never against what the driver says. You are read-only on the server and the database. Use PowerShell (no bash). You may read repo files. You never drive the game client and never take a lab lease. You never file a GitHub issue yourself (section 6).

You are given: (1) this brief; (2) the DRIVER REPORT pasted by the coordinator under `## DRIVER REPORT` at the end; (3) the screenshot paths the report names. Packet: SP-DTO1-p2. Instance default, character Uat Fxtolonedefa.

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
2. Ids: `SELECT player_id, account_id, player_name FROM public.sgw_player WHERE player_name ILIKE '%Fxtolone%'`. Call the result `$pid` and `$acc`.
3. Row timeline from the driver's `.bug` bookmarks. Read the log tail and keep lines that mention `SPU SP-DTO1-p2`. Each `SPU SP-DTO1-p2 <row> start` gives that row's window start; the next bookmark (or the packet end) gives its end. Pad every window by 5 s before and 20 s after. If no bookmark lines appear, widen to the driver's UTC fields and say the windows are approximate.
4. Fixture: find the INFO `lab fixture applied` line for the character (or the ERROR `lab fixture not applied` with its `reason`). No applied line: every row is BLOCKED, quote the `reason`, and stop.

Log-tail filter helper (PowerShell, paste once): `function Win($lines,$t0,$t1,$pat){ $lines | Where-Object { $_ -match $pat } }` and apply the time window yourself by reading the timestamps.

## 3. The looker (visual checks)

For each screenshot the report names, start a Haiku looker: Agent tool, `subagent_type: general-purpose`, `model: haiku`, prompt = `Read the image file <PATH> (use the Read tool). Answer each question with YES, NO or UNSURE and at most 12 words of what you see. Do not guess beyond what is visible. Questions: 1. ... 2. ...` using the exact questions listed for that screenshot below. Record the answers verbatim in your report. A NO or UNSURE on a question that a row depends on makes that row at best PARTIAL (visual) or NEEDS_HUMAN, never FAIL by itself: a screenshot can miss the thing. If the PNG is black or missing, say so.

## 4. Report format (your final message and the result file)

Write `docs/analysis/starter-uat-praxis/results/SP-DTO1-p2.md` in the `sp-integration` worktree (contract C6), CRLF, containing:

```text
| Row | Grade | Tier (N1/N2/N3/G) | Server evidence (query or log event, time) | Screenshot | Issue |
```

one line per row, then `## Not run` (each clause and why), then `## Resume hint` (the state a re-run would start from), then `## Recipe` (the exact tool calls that worked), then `## Findings` (numbered; each with the row and the evidence), then `## Sign-off sheet` (section 5). Your final message to the coordinator is exactly one line: `SP-DTO1-p2 <PASS|PARTIAL|FAIL|BLOCKED> <n rows> <result-path>`.

## 5. Sign-off sheet (the NEEDS_HUMAN rows)

Every NEEDS_HUMAN row, and every row with a NEEDS_HUMAN half, appears in `## Sign-off sheet` as a table: `| Row | What to look at | Screenshot path | Driver's words | Looker pre-screen | Your read of the numbers |`. The user reviews this table at the end of the campaign. The sheet is evidence, not a request: never block on it.

## 6. When a row FAILs, or you find a bug

Investigate before you conclude: read the code path or doc the row exercises and decide whether it is (a) a product bug, (b) a doc mismatch, (c) a lab or driver problem (a wrong click, a bad recipe, a harness gap), or (d) a known issue. Only (a) and (b) get an issue body. You do NOT file the issue (contract C7 and dispatch rule 7): write the body to `docs/analysis/starter-uat-praxis/results/issues/SP-DTO1-p2-<n>.md` in the `sp-integration` worktree and put the file path in the row's Issue column; the orchestrator files it. Title line first: `<system>: <what is wrong> (starter-praxis UAT SP-DTO1-p2)`. Body sections: `## Problem`, `## Evidence` (log rows with times, the driver quote, the screenshot paths, what `docs/` already says), `## Acceptance criteria` (observable: a log field, a DB row), `## Test type`, `## Docs to update`, `## Client impact` ("Free" or "needs a client patch"), `## Domain advisor` (a `.claude/agents/` name), `## Needs a human for`. (c) goes in `## Not run` with the recipe the next driver needs.

## 7. Do not

Do not edit the repo apart from the result file and the issue body files; do not edit the database or the server; do not run the lab; do not deploy; do not file issues; do not ask the user anything; do not paste IPs, keys or account names; do not copy the driver's claims into the evidence column; do not treat a Cimmeria seed or a ticket as proof of what the original client or server did.


---

# Verify SP-DTO1-p2: Tollana interaction rows P4 and P5

Start: fixture (re-staged; nothing carried over from SP-DTO1-p1), mission 568 `not_active`. The prop dialogs are binds on the props themselves, not on the fixture. Expect INFO `lab fixture applied` for the character. Rows P1 to P3 are part SP-DTO1-p1.

Shorthand: `$W(row)` = the row window from the `SPU SP-DTO1-p2 P<n> start` bookmarks (setup step 3). Traced spawn rows (`db/resources/Worlds/Seed/spawnlist_tollana.sql:10-13`): 19001 Copplemann template 420 at (206.00, -0.58, -543.00) h 2.315; 19002 Core (199.50, -0.93, -540.00), 19003 Nest (195.50, -1.01, -540.00), 19004 Fungus (197.50, -1.01, -536.54), template 421, h 2.025. DHD 39 (209.748, -1.008, -552.898); ring console 155 (235.244, -1.109, -571.308). Evidence note: these are CIMMERIA_RECONSTRUCTION positions probed on the navmesh (T-01); the rows test the placement, not any original-server claim.

## Row P4 - interaction range (check 4)

| Check | Evidence | Expected |
|---|---|---|
| In range opens | driver `4.1`: for each of the three props, `range` about 2.3 m and a dialog opened | three dialogs (4211 Core, 4213 Nest, 4214 Fungus; the first line is enough) |
| Server saw the clicks | `$W(P4)` log rows for the three interactions (scope content or mission; print one to learn the message) | three interact events for the player, each at 2.3 m or less; no `Content: advancing step` or `Mission accepted` (mission 568 is `not_active`: nothing may advance) |
| Out of range refused | driver `4.3`: Nest from CORE15 (about 5.4 m) | the lab tool answered `out_of_range`; no interact event for the Nest in that minute |
| State unchanged | `SELECT * FROM public.sgw_mission WHERE player_id=$pid AND mission_id=568` | no row, or only a not-active row the fixture wrote; never an active step |

Grade: PASS if all hold. FAIL if a mission row appeared or a dialog opened beyond 5.0 m. The Core-at-1.5 m click is NOT-RUN in this part (list it under `## Not run`). The server range is `MAX_INTERACT_DISTANCE` 5.0 m; a client model range below that is a finding for the row's notes, not a FAIL.

## Row P5 - DHD and ring unobstructed (check 5)

| Check | Evidence | Expected |
|---|---|---|
| DHD found | driver `5.2`: an entity near (209.75, -553.0) | present, id recorded |
| DHD click | driver `5.3` and `5.4`: a window opened, its title; log row for the interact in `$W(P5)` | a DHD window opened; the server logged the interact |
| Ring console | driver `5.6`: an entity near (235.24, -571.31), `targetable` | present and targetable |
| Dial | driver line `dial: NOT-RUN` | NOT-RUN is correct; SP-05 proves the Tollana address |
| Copplemann blocking | screenshot `DTO1-P5-dhd` | NEEDS_HUMAN |

Grade: PASS if the DHD and the ring console were found and the DHD click opened its window with a server log row; the dial clause is NOT-RUN (list it under `## Not run`). FAIL if the DHD did not open at 3.4 m. The blocking question goes on the sign-off sheet.
Looker `DTO1-P5-dhd`: 1. Is a DHD (a dial pedestal) or a dial window visible? 2. Is any person or figure standing between the camera and the pedestal? 3. Is the screen free of a covering dialog?

Known gaps that are not findings: arrival, route and props (SP-DTO1-p1), the two-client rows (SP-DTO2-p1, SP-DTO2-p2) and every mission 568 dialog (SP-DTO3-p1 to p4, SP-DTO4-p1 to p3). List the P5 dial and the Core-at-1.5 m click under `## Not run`.

---

## DRIVER REPORT

(The coordinator pastes the driver's final report here, with the screenshot paths.)
