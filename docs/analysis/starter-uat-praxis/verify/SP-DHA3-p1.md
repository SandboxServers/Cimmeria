# SP-DHA3-p1 verify brief

> **Type:** how-to (runbook). **Audience:** a Sonnet verifier agent. **Last updated:** 2026-11-02.
> **Rows:** FZ-U2a. **Start:** fixture (1664 at step 5224, Harset gate). **Companions:** [work-packets.md](../work-packets.md) (contracts C1-C8), [rows.md](../rows.md), [unified-uat.md FZ-U2 row](../../../guides/unified-uat.md#harset), [hub-pass HP-M1 template](../../hub-pass-uat/briefs/HP-M1.md), [the driver brief](../briefs/SP-DHA3-p1.md). Sibling parts: SP-DHA3-p1 (FZ-U2a), SP-DHA3-p2 (FZ-U2b), SP-DHA3-p3 (FZ-U2c).
> NEEDS_HUMAN rule for this packet: a look-and-feel clause (name plate, facing, a mission cue, a use cursor, a journal or tracker line) never blocks the lane. The driver takes a `lab_screenshot` (path recorded), writes one line of what the tool results showed, marks the step NEEDS_HUMAN and moves on. The screenshot goes on the sign-off sheet for the user to review at the end. Lab use is pre-authorized on the local server (D-SP5); the brief never tells a driver to ask the user.

You are the Sonnet VERIFIER for one part of the starter-uat-praxis UAT. A cheap driver agent drove the real game client on the local server and wrote a report. You grade each row against SERVER evidence, never against what the driver says. You are read-only. Use PowerShell (no bash). You may read repo files. You may run `gh` to file issues (section 6). You never drive the game client, never take a lab lease and never call the lab-server tools that change state (`server_console_exec`, `server_stage_fixture`, `server_content_reload`).

You are given: (1) this brief; (2) the DRIVER REPORT pasted by the coordinator under `## DRIVER REPORT` (one report: one run of this part on one character); (3) the screenshot paths the report names. Packet: SP-DHA3-p1. Server: local, at the recorded SHA (D-SP2), so evidence comes from the lab-server MCP, not from colo ssh. Start: fixture (1664 at step 5224, Harset gate).

## 1. Rules of grading

- Vocabulary (docs/guides/automated-uat.md): **PASS**, **NATIVE_SHORTFALL** (the server did what the row expects but the step ran below the required tier), **PARTIAL**, **FAIL** (a clause is contradicted by server evidence), **BLOCKED** (the row never reached its step, including a fixture that did not apply), **NOT-RUN**, **NEEDS_HUMAN**.
- Tiers: N1 a real click; N2 a typed slash command standing in for a UI action; N3 the stock UI Lua called directly; G a GM command as the step or as setup. `/abandonmission` and `.gotolocation` are typed commands that ARE the step: G/typed, no downgrade. The NPC clicks are `client_world_click` (N1 when its result says a native click landed; record the `native_level` it reports).
- **NEEDS_HUMAN never blocks the lane.** Name plates, facing, cues, use cursors and journal lines cannot be graded from server data. When every server clause of a row holds and only such look-and-feel clauses remain, grade the row **NEEDS_HUMAN** and write "server clauses PASS" in Notes; the screenshots go on the SIGN-OFF SHEET (section 5) for the user. A server clause that fails is FAIL whatever the pictures show. Never ask the user a question; never wait.
- A driver claim with no matching server row is not evidence. A row for the wrong player or outside the row window does not count.
- Any WARN or ERROR in a relevant target inside the window that the row does not expect is a **finding**.
- Dialog text is provisional (D-FZ15): grade that the right line (screen id) appeared, not its wording.
- The server prints no mission step text. The driver's `gmMissionList` entries carry step ids (`#1664 active step 5225 obj 0/2`); grade the step id, never a step name.

## 2. Setup you do first (4 calls)

1. Clock and buffer: `server_log_tail {"limit":1}` and note `newest_ms`. The log buffer is in memory and finite: grade right after the driver run, and say so if a window has rolled out (grade that clause UNVERIFIED and the row at best PARTIAL).
2. Ids: `server_db_query` `SELECT player_id, account_id, player_name FROM public.sgw_player WHERE player_name ILIKE '%FxDhaThreeA%'`. Call the result `$pid`. The name is `Uat FxDhaThreeA`.
3. Row timeline from the driver bookmarks: `server_log_tail {"contains":"SPU SP-DHA3-p1","limit":200}`. Print ONE entry in full first to learn how the note is carried (message or field). Each `SPU SP-DHA3-p1 <row> start` opens that row's window; the next `start`, an `end` or the report end closes it. Pad 5 s before, 20 s after. Convert `timestamp_ms` programmatically (PowerShell `[DateTimeOffset]::FromUnixTimeMilliseconds`).
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
PACKET SP-DHA3-p1 VERIFIED <UTC now> by Sonnet; driver attempts: <n>; server evidence window <t0>-<t1>
| Row | Result | Tier | Server evidence (time, event or query, key fields) | Client evidence | Looker | Notes |
```

One line per row, then `FINDINGS:` (numbered), `NOT RUN / BLOCKED:` with reasons, `RESUME HINT:` (what the next attempt must redo), and:

`SIGN-OFF SHEET:` one line per NEEDS_HUMAN screenshot: `<row> | <LABEL> | <path> | what the row wants the user to look for | looker pre-screen`. The coordinator copies it to the end-of-run sign-off sheet.

Also write the same table to `docs/analysis/starter-uat-praxis/results/SP-DHA3-p1.md` in the `sp-integration` worktree (contract C6) and return one line.

## 6. When a row FAILs, or you find a bug

Investigate before you file: read the chain or handler the row exercises (the file:line in the row), reproduce the reasoning with the server rows, and decide whether it is (a) a product bug, (b) a doc mismatch, (c) a lab or driver problem (a wrong click, a bad stand position, a placeholder spawn: report only, with the recipe the next driver needs), or (d) a known issue (unified-uat.md K1-K33, or the D-PU6 placeholder door position: do not file). Only (a) and (b) get a GitHub issue: `gh issue create` from PowerShell with `--body-file`. Title `<system>: <what is wrong> (starter-uat-praxis SP-DHA3-p1)`. Labels `bug`; add `needs-triage` only if it exists. Never label `ready-for-agent`. The body follows docs/agents/issue-tracker.md: `## Problem`, `## Evidence`, `## Acceptance criteria`, `## Test type`, `## Docs to update`, `## Client impact`, `## Domain advisor`, `## Needs a human for`, `## Implementation plan`. Mention `@Cadacious`. Search first (`gh issue list --search "<words>" --state all`) and comment on an existing issue instead of filing a duplicate.

## 7. Do not

Do not edit the repo, the database or the server; do not run the lab; do not call state-changing lab-server tools; do not deploy; do not paste IPs, keys or account names; do not copy the driver's claims into the evidence column; do not ask the user anything.

---

# Verify SP-DHA3-p1: Kaerlia (character `Uat FxDhaThreeA`)

Start: fixture. Row windows come from `SPU SP-DHA3-p1 FZ-U2a start` and `SPU SP-DHA3-p1 FZ-U2a clicked`.

## Row FZ-U2a - Kaerlia

| Check | Query | Expected |
|---|---|---|
| Fixture applied | the 1664 `sgw_mission` row before the click (the first driver MLIST, S2) | step 5224 active |
| Step advanced | logs `mission.advance_step` for `$pid` in the window | step 5225 (`db/resources/Content/Seed/harset_1664_chains.sql:54`) |
| Chain 6008 ran | chain-id logs for 6008 | `advance_step`, `display_dialog 6169`, `remove_dialog_set 7016`, `add_dialog_set 120301` |
| State after | `sgw_mission` | `current_step_id`=5225 |
| Relog restore | chain-id logs for 6015 after the driver's login in the window | ran once, `add_dialog_set 120301` (and 6014 did NOT run) |

Grade: PASS needs every server clause and the driver's MLIST entry showing step 5225 before and after the relog. Name plate, facing, cue and the tracker line are look-and-feel: NEEDS_HUMAN with `A-KAERLIA-BEFORE`, `A-TRACKER`, `A-RELOG` on the sign-off sheet. FAIL if the step is not 5225 or 6008 did not run.
Looker `A-KAERLIA-BEFORE`: 1. Is a name above the NPC? 2. Does the NPC face the camera or the player? 3. Is there a "?" or marker over her? Looker `A-DIALOG`: 1. Is a dialog window open with text? 2. Does it mention the command center? Looker `A-TRACKER`: 1. Is a mission line mentioning the Command Center visible?

---

## DRIVER REPORT

(The coordinator pastes the driver's final report here, with the screenshot paths.)
