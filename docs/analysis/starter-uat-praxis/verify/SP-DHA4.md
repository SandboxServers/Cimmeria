# SP-DHA4 verify brief

> **Type:** how-to (runbook). **Audience:** a Sonnet verifier agent. **Last updated:** 2026-11-02.
> **Rows:** FZ-U2d, FZ-U2h. **Companions:** [work-packets.md](../work-packets.md) (contracts C1-C8), [rows.md](../rows.md), [unified-uat.md FZ-U2 row](../../../guides/unified-uat.md#harset), [hub-pass HP-M1 template](../../hub-pass-uat/briefs/HP-M1.md), [the driver brief](../briefs/SP-DHA4.md).
> NEEDS_HUMAN rule for this packet: a look-and-feel clause (name plate, facing, a mission cue, a use cursor, a journal or tracker line) never blocks the lane. The driver takes a `lab_screenshot` (path recorded), writes one line of what the tool results showed, marks the step NEEDS_HUMAN and moves on. The screenshot goes on the sign-off sheet for the user to review at the end. Lab use is pre-authorized on the local server (D-SP5); the brief never tells a driver to ask the user.

You are the Sonnet VERIFIER for one packet of the starter-uat-praxis UAT. A cheap driver agent drove the real game client on the local server and wrote a report. You grade each row against SERVER evidence, never against what the driver says. You are read-only. Use PowerShell (no bash). You may read repo files. You may run `gh` to file issues (section 6). You never drive the game client, never take a lab lease and never call the lab-server tools that change state (`server_console_exec`, `server_stage_fixture`, `server_content_reload`).

You are given: (1) this brief; (2) the DRIVER REPORT pasted by the coordinator under `## DRIVER REPORT` (one report per run: one run for FZ-U2d and three for FZ-U2h (h1, h2, h3)); (3) the screenshot paths the report names. Packet: SP-DHA4. Server: local, at the recorded SHA (D-SP2), so evidence comes from the lab-server MCP, not from colo ssh.

## 1. Rules of grading

- Vocabulary (docs/guides/automated-uat.md): **PASS**, **NATIVE_SHORTFALL** (the server did what the row expects but the step ran below the required tier), **PARTIAL**, **FAIL** (a clause is contradicted by server evidence), **BLOCKED** (the row never reached its step, including a fixture that did not apply), **NOT-RUN**, **NEEDS_HUMAN**.
- Tiers: N1 a real click; N2 a typed slash command standing in for a UI action; N3 the stock UI Lua called directly; G a GM command as the step or as setup. `/abandonmission` and `.gotolocation` are typed commands that ARE the step: G/typed, no downgrade. The NPC clicks are `client_world_click` (N1 when its result says a native click landed; record the `native_level` it reports).
- **NEEDS_HUMAN never blocks the lane.** Name plates, facing, cues, use cursors and journal lines cannot be graded from server data. When every server clause of a row holds and only such look-and-feel clauses remain, grade the row **NEEDS_HUMAN** and write "server clauses PASS" in Notes; the screenshots go on the SIGN-OFF SHEET (section 5) for the user. A server clause that fails is FAIL whatever the pictures show. Never ask the user a question; never wait.
- A driver claim with no matching server row is not evidence. A row for the wrong player or outside the row window does not count.
- Any WARN or ERROR in a relevant target inside the window that the row does not expect is a **finding**.
- Dialog text is provisional (D-FZ15): grade that the right line (screen id) appeared, not its wording.

## 2. Setup you do first (4 calls)

1. Clock and buffer: `server_log_tail {"limit":1}` and note `newest_ms`. The log buffer is in memory and finite: grade right after the driver run, and say so if a window has rolled out (grade that clause UNVERIFIED and the row at best PARTIAL).
2. Ids: `server_db_query` `SELECT player_id, account_id, player_name FROM public.sgw_player WHERE player_name ILIKE '%FxDhaFour%'`. Call the result `$pid` (one per character; the names are in the driver report).
3. Row timeline from the driver bookmarks: `server_log_tail {"contains":"SPU SP-DHA4","limit":200}`. Print ONE entry in full first to learn how the note is carried (message or field). Each `SPU SP-DHA4 <row> start` opens that row's window; the next `start` or the packet end closes it. Pad 5 s before, 20 s after. Convert `timestamp_ms` programmatically (PowerShell `[DateTimeOffset]::FromUnixTimeMilliseconds`).
4. If the buffer holds no `mission.` lines at all for the window, grade every server clause UNVERIFIED and stop. DEBUG lines may be filtered: say so and use INFO lines and the DB.

## 3. Evidence methods

- **Mission state** (authoritative): `server_db_query` `SELECT mission_id, current_step_id, status, completed_step_ids, completed_objective_ids, active_objective_ids FROM public.sgw_mission WHERE player_id=$pid AND mission_id=1664` (the table is `db/sgw/Missions/Tables/sgw_mission.sql`). No row or a non-active status after an abandon or completion; learn the status integers from the 708 row of a completed mission if needed. The row is written on `mission.persist` (`crates/base-methods/src/base/world_entry/methods/missions/mod.rs:119`), so a read right after an action may lag: re-read once after 5 s.
- **Mission events** (logs): `server_log_tail` with `contains` set to `mission.advance_step`, `mission.complete_objective`, `mission.abandon`, `mission_abandoned` or `mission.accept` (`crates/cell-content/src/cell/missions/progression.rs:23,320`, `lifecycle.rs:256,295`), then filter by time and `$pid` in what you printed.
- **Chain runs**: `server_log_tail` `contains` a chain id such as `6018`, or the content-engine target; print one entry to learn the field names. A chain that ran shows its id and action list in the window.
- **Chat lines**: the barks are server chat (`npc_bark`, screen ids 200180, 200181, 117158, 117162); the DB and logs name the screen id. The client chat log lines in the driver report are the second witness.
- **World and position**: `server_sessions` and `server_entity_get` for the character (world id 57 or 68, position).

## 4. The looker (visual checks)

For each screenshot the report names, start a Haiku looker: Agent tool, `subagent_type: general-purpose`, `model: haiku`, prompt = `Read the image file <PATH> (use the Read tool). Answer each question with YES, NO or UNSURE and at most 12 words of what you see. Do not guess beyond what is visible. Questions: 1. ... 2. ...` using the questions listed for that screenshot. Record answers verbatim. A looker answer is a pre-screen for the sign-off sheet, never a grade by itself. A black or missing PNG: say so.

## 5. Report format (your final message)

```text
PACKET SP-DHA4 VERIFIED <UTC now> by Sonnet; driver attempts: <n>; server evidence window <t0>-<t1>
| Row | Result | Tier | Server evidence (time, event or query, key fields) | Client evidence | Looker | Notes |
```

One line per row, then `FINDINGS:` (numbered), `NOT RUN / BLOCKED:` with reasons, `RESUME HINT:` (what the next attempt must redo), and:

`SIGN-OFF SHEET:` one line per NEEDS_HUMAN screenshot: `<row> | <LABEL> | <path> | what the row wants the user to look for | looker pre-screen`. The coordinator copies it to the end-of-run sign-off sheet.

## 6. When a row FAILs, or you find a bug

Investigate before you file: read the chain or handler the row exercises (the file:line in the row), reproduce the reasoning with the server rows, and decide whether it is (a) a product bug, (b) a doc mismatch, (c) a lab or driver problem (a wrong click, a bad stand position, a placeholder spawn: report only, with the recipe the next driver needs), or (d) a known issue (unified-uat.md K1-K33, or the D-PU6 placeholder door position: do not file). Only (a) and (b) get a GitHub issue: `gh issue create` from PowerShell with `--body-file`. Title `<system>: <what is wrong> (starter-uat-praxis SP-DHA4)`. Labels `bug`; add `needs-triage` only if it exists. Never label `ready-for-agent`. The body follows docs/agents/issue-tracker.md: `## Problem`, `## Evidence`, `## Acceptance criteria`, `## Test type`, `## Docs to update`, `## Client impact`, `## Domain advisor`, `## Needs a human for`, `## Implementation plan`. Mention `@Cadacious`. Search first (`gh issue list --search "<words>" --state all`) and comment on an existing issue instead of filing a duplicate.

## 7. Do not

Do not edit the repo, the database or the server; do not run the lab; do not call state-changing lab-server tools; do not deploy; do not paste IPs, keys or account names; do not copy the driver's claims into the evidence column; do not ask the user anything.

---

# Verify SP-DHA4: the other order and the Praxis offer (characters `Uat FxDhaFourA` to `D`)

Row windows come from `SPU SP-DHA4 FZ-U2d start|end` and `FZ-U2h1|2|3 start|abandon`.

## Row FZ-U2d - Moh'katan first, then Marsh

| Check | Query | Expected |
|---|---|---|
| Fixture | the first MLIST and `sgw_mission` | 1664 at 5226 |
| Moh'katan first | chain 6019 logs, `mission.complete_objective` | objective 6062 once; screen 200181 |
| Re-click | logs after the second click | no second 6062 credit |
| Relog restore | 6033 and 6034 after the relog | 6033 ran (Marsh cue); 6034 did NOT run |
| Marsh second | chain 6018, `complete_objective` | objective 6061; screen 200180; 1664 completed in `sgw_mission` |

Grade as FZ-U2c in SP-DHA3 (same chains, reverse order): PASS on every server clause, cues NEEDS_HUMAN (`D-KATAN-CUE`, `D-KATAN-RELOG`, `D-MARSH-CUE`, `D-DONE`). FAIL on a double credit or no completion.

## Row FZ-U2h - Praxis offer on Marsh

Each part is graded from its own report. Server evidence for the offer: chain 6511 run (the bind, `db/resources/Content/Seed/harset_opcore_chains.sql:557-623`) and chain 6512 run with `display_dialog 4457` (the click, `db/resources/Content/Seed/harset_opcore_chains.sql:625-643`), both only while 1664 is not on 5226. The mission 1361 row must NOT appear active at any point (Accept never pressed): `SELECT mission_id, status FROM public.sgw_mission WHERE player_id=$pid AND mission_id=1361`.

| Part | Checks | Expected |
|---|---|---|
| h1 before | Marsh click on 5226 | chain 6018 ran (report); 6512 did NOT run; no dialog 4457 |
| h1 after | 1664 completed by Moh'katan, then Marsh click | 6511 ran on `mission_completed 1664`; 6512 ran `display_dialog 4457` |
| h2 abandon in CC | `mission.abandon`/`mission_abandoned` 1664 while in world 68, then Marsh click | 6016 ran (cues dropped) AND 6511 ran in the same event; 6512 ran `display_dialog 4457` |
| h3 abandon elsewhere | abandon while in world 57 | 6016 ran; 6511 did NOT bind (its `world 68` condition fails); after the later entry to world 68, 6511 ran on `player_loaded Harset_CmdCenter`, then 6512 `display_dialog 4457` |

Grade each part, then the row: PASS when all three parts PASS on the server clauses (the row's text: before and after 1664 ends, abandon in CC versus elsewhere). A part whose fixture left no 1664 on 5226 is BLOCKED ("a Human could not hold 1664"); a BLOCKED part makes the row PARTIAL. FAIL if 4457 opened while 1664 was on 5226, if the offer did not return after completion or the abandon, or if h3 shows a bind with no world-68 entry. The "?" cue itself, the dialog look and the difference between "immediate" and "on the next entry" as seen on screen: NEEDS_HUMAN (`H1-BEFORE`, `H1-AFTER`, `H1-DIALOG`, `H2-CUE`, `H2-DIALOG`, `H3-CUE`, `H3-DIALOG`). No WARN or ERROR in a mission or content target in the window that the row does not expect.
Looker `H1-DIALOG`, `H2-DIALOG`, `H3-DIALOG`: 1. Is a dialog window open? 2. Does it show an Accept button? Looker `H1-AFTER`, `H2-CUE`, `H3-CUE`: 1. Is a "?" or marker over the NPC?

---

## DRIVER REPORT

(The coordinator pastes each driver report here in turn, with the screenshot paths.)
