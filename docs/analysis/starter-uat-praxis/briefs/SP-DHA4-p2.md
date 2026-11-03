# SP-DHA4-p2 driver brief

> **Type:** how-to (runbook). **Audience:** the coordinator (pre-step) and a Haiku `lab-driver` agent. **Last updated:** 2026-11-02.
> **Rows:** FZ-U2h part h1. **Start:** fixture (1664 at step 5226, Command Center). **Companions:** [work-packets.md](../work-packets.md) (contracts C1-C8), [rows.md](../rows.md), [unified-uat.md FZ-U2 row](../../../guides/unified-uat.md#harset), [hub-pass HP-M1 template](../../hub-pass-uat/briefs/HP-M1.md), [the verify brief](../verify/SP-DHA4-p2.md). Sibling parts: SP-DHA4-p1 (FZ-U2d), SP-DHA4-p2 (FZ-U2h h1), SP-DHA4-p3 (FZ-U2h h2), SP-DHA4-p4 (FZ-U2h h3).
> NEEDS_HUMAN rule for this packet: a look-and-feel clause (name plate, facing, a mission cue, a use cursor, a journal or tracker line) never blocks the lane. The driver takes a `lab_screenshot` (path recorded), writes one line of what the tool results showed, marks the step NEEDS_HUMAN and moves on. The screenshot goes on the sign-off sheet for the user to review at the end. Lab use is pre-authorized on the local server (D-SP5); the brief never tells a driver to ask the user.

## Coordinator pre-step (the driver cannot do this)

The lab-driver has no lab-server tools, and a staged fixture lapses 10 minutes after staging, so stage right before you dispatch. This part runs on one character, so you fill in nothing: set ATTEMPT (1 for the first dispatch) and paste. Create the character once as a Praxis Soldier on the GM account (`lab_create_character`, last name `FxDhaFourB`, contract C4), keep it offline, call `server_stage_fixture` with the arguments below, then paste this brief from the PASTE line. A Soldier counts as Human for chains 6511 and 6512 (neither is Jaffa 8 nor Goa'uld 6, `db/resources/Content/Seed/harset_opcore_chains.sql:603-608`). Mission 708 (Castle) is set completed so the Harset gate state is the real one. A character whose fixture was not applied makes the row BLOCKED (gotcha 19).

| Part | Character (last name) | `server_stage_fixture` arguments |
|---|---|---|
| SP-DHA4-p2 | `FxDhaFourB` | `{"character":"Uat FxDhaFourB","world":"Harset_CmdCenter","position":[0.0,0.355,-20.0],"missions":[{"id":708,"state":"completed"},{"id":1664,"step":5226}]}` |

Start: fixture. The part needs nothing from its siblings, so a FAIL in one part does not block another (the login chains replay the cues, see SOURCES). Row FZ-U2h is split by its three parts: h1 here, h2 in -p3, h3 in -p4; the row grade is the combination.

Attempt 2 or 3 of a single-row run: the coordinator first logs the character out (`lab_logout`), calls `server_stage_fixture` again with the same arguments, then pastes the brief with ATTEMPT set. The successor runs the row from its first step. (The fixture is taken once, at the first Play, so without a fresh stage the successor would find the mission already advanced.)


--- PASTE FROM HERE ---

You are a `lab-driver` agent. You drive one live Stargate Worlds game client through the cimmeria-lab tools and report raw facts. You do not diagnose, judge, fix, edit files or query databases. You can read nothing except tool results. You have never seen this game: do exactly what this brief says, in order, and nothing else.

BRIEF FIELDS (the coordinator set ATTEMPT before pasting; everything else is fixed):

- PACKET: SP-DHA4-p2
- ATTEMPT: 1   (1 = first try. 2 or 3 = you are the successor of a driver that ran out of context; the coordinator re-staged the character, so run the row from its first step)
- INSTANCE: p3   (the lab client you drive)
- ROW: FZ-U2h part h1
- CHARACTER: FxDhaFourB (Praxis Soldier; full name Uat FxDhaFourB)
- SERVER: local server at the recorded SHA (D-SP2)
- AREA: the Harset gate plaza (world 57) and the Harset Command Center (world 68) only; no other lane may use Harset while you run (dispatch rule 4)
- LEASE OWNER: sp-SP-DHA4-p2, TTL 900 s, hard stop at 28 tool calls per attempt
- PREVIOUS ATTEMPT REPORT: none   (attempt 2 or 3: the coordinator pastes the predecessor's report here)

SOURCES (every id below is traced; `rows.md` assigns the rows):
- Mission 1664 steps 5224/5225/5226: `db/resources/Missions/Seed/mission_steps.sql:2879-2883`; chains 6008-6019, 6033, 6034: `db/resources/Content/Seed/harset_1664_chains.sql` (6008 at :45, 6012 at :99, 6016 at :150, 6018 at :183, 6019 at :203, 6033 at :226).
- Positions: Kaerlia spawn 360 (-5.0, -68.95, 30.0) and door control spawn 380 (0.0, -67.6, -235.0, a PLACEHOLDER, D-PU6) at `db/resources/Worlds/Seed/spawnlist.sql:1074-1075`; Moh'katan 343 (-5.0, 0.37, -8.0) and Marsh 344 (-49.5, -0.59, -14.0) in world 68 at `spawnlist.sql:831,842`; Harset gate-plaza respawner 20 (-8, -68.99, 34) at `db/resources/Worlds/Seed/respawners.sql:131`; Command Center arrival (0.0, 0.355, -20.0) at `harset_1664_chains.sql:110`.
- Chat lines: Kaerlia 117162 and 117158 (`db/resources/Dialogs/Seed/dialog_screens.sql:27659,27649`); Marsh 200180 "Report received. Good work out there." and Moh'katan 200181 "Your report is filed." (`dialog_screens.sql:28661-28662`, provisional text, D-FZ15: grade that a line appears, not its wording).
- Praxis offer on Marsh: dialog 4457, chains 6511/6512 (`db/resources/Content/Seed/harset_opcore_chains.sql:547-643`); 6512 stands down while 1664 is on 5226 (:640).
- Abandon: `/abandonmission <id>` (`docs/commands.md:170`); mission 1664 `can_abandon = true`, not hidden (`db/resources/Missions/Seed/missions.sql:781`). Abandon is final, option P0 (O18-02a, `docs/analysis/harset-continuation/worknotes/HA-01b-graph-seal.md:109`); if the owner has since picked P3 a re-offer is the expected result.
- Login restore chains: 6014 (step 5224, Harset) and 6015 (step 5225, Harset) at `harset_1664_chains.sql:124-146`; 6033 and 6034 (step 5226, Command Center) at `harset_1664_chains.sql:221-248`. They run on `player_loaded`, so a fixture login restores the cue the click chain of the previous row would have added.

## 0. Front-loaded gotchas (read all of them)

1. **Lease first.** Every driving tool needs the `lease_id` from L1. Pass it as `lease_id` on every call. On `lab_screenshot` also pass `"instance": "p3"`.
2. **Never use `image: true`** on a screenshot. The tool saves a PNG and returns `image saved: <path>`. Report the path.
3. **Chat commands are GM commands.** Lines starting with `.` or `/gm` are server commands. Close any open dialog first (DIALOG), because a chat line under an open dialog orphans it.
4. **Teleport only with `.gotolocation <World> x y z`** (server metres, y up). Never `.gotoxyz`. Never `client_move_to` in a hub. World names: `Harset` (world 57) and `Harset_CmdCenter` (world 68). A teleport into another world reloads the world: run WAIT after it.
5. **A freshly played character is deaf for 16 seconds**: the server holds entity introductions and swallows chat commands. After L2 and after every world change run WAIT (its two 20 s waits cover the hold).
6. **Read chat replies only with `client_chat_log`**, always with a NEW `cursor` name and a `contains` or `channel` filter. Server replies are on channel id `"9"` (the tool labels them `None`; never filter by `"Feedback"`, #1679). A Lua read of the chat output returns nil. A new cursor starts at the OLDEST line in the ring, so a read without a filter returns old lines.
7. **Sticky chat channel.** Every chat macro below resets it with `ChatMod.stickyCommand='/say '`. Do not remove that.
8. **No apostrophes** inside the Lua text of a chat macro. Name Moh'katan as `katan` in every lookup.
9. **Stand 3.5 to 4.5 m from an NPC** before right-clicking it (server range 5.0 m). The step's `.gotolocation` puts you 4.0 m off. TALK picks the nearest entity whose name contains your text and reports `range`: record it. Entity ids change after a relog or a world change, so never reuse an id from before.
10. **If a click opens nothing**, take one SHOT, retry the click once unchanged, then record FAIL for that step and go on to the next independent step. Never improvise a workaround.
11. **Never use `client_wait_for` on a world load** (#1677). Use WAIT (two `client_batch` waits of 20 s).
12. **PROMPT is a failure-path macro, not a routine one.** The Server Message OK button is `Prompt1_SingleButton`. A covering window swallows native clicks (#1676). Before a world click, the step's batch runs DIALOG as its first op where a dialog can be open. When a click opens nothing, run PROMPT, then SHOT, then one unchanged retry (gotcha 10). If a click still does nothing, read `client_ui_state` once and record the top window.
13. **Do not restart the client** and do not call `lab_client_*` tools. If the client is dead (every call fails), stop and report BLOCKED with the error text. `lab_logout` can crash the client (#1680): if RELOG fails, record the error, mark the rest of that relog check BLOCKED and carry on.
14. **Budget: 28 tool calls per attempt, counted by tool invocation.** Every `client_batch` is one call, whatever its length. The ESTIMATE below is the planned success path and is at most 22; the other calls are headroom for gotchas 10 and 15. Count your calls. At call 26 stop starting steps, write the report from what you have, release the lease and finish. Never start a step you cannot finish. Never look up a tool schema: every argument is written out in this brief.
15. **One retry per failed call, unchanged.** Then record the error text verbatim and continue.
16. **An unexpected window, prompt or dialog**: read it once with `client_ui_state`, record it, close it with DIALOG or PROMPT, and carry on.
17. **Never write IPs, passwords or account names** in your report.
18. **NEEDS_HUMAN never blocks the lane.** A look-and-feel check (a name plate, facing, a mission cue, a use cursor, a journal line) cannot be graded from a tool result. At such a step take a SHOT (`lab_screenshot`, path recorded), write one line of what the tool results showed, mark the step `NEEDS_HUMAN` and go straight on. The user reviews the screenshot on the sign-off sheet at the end. Never ask the user anything and never wait for an answer: lab use is pre-authorized on the local server (D-SP5).
19. **A missing fixture is BLOCKED, never FAIL.** If the RESUME PROBE shows the character is not in the fixture state, record BLOCKED for the row and stop; do not rebuild the state with GM commands.
20. **Run the steps in the order given.** The area for this packet is the Harset gate plaza and the Command Center only (dispatch rule 4): no other lane may use Harset while this one runs.

## 1. Fixed macros (use them verbatim)

Every macro states its cost in calls. A step's label in section 2 is the sum of the macros it uses.

**L1 - lease.** Cost: 1 call. `lab_lease_acquire {"owner":"sp-SP-DHA4-p2","purpose":"starter-uat-praxis SP-DHA4-p2","instance":"p3","ttl_s":900}` -> keep `lease_id` (call it L). If refused: `lab_lease_status {"instance":"p3"}` (a failure-path call). If the holder's owner is exactly `sp-SP-DHA4-p2` AND ATTEMPT is 2 or 3 it is your dead predecessor: acquire again with the same arguments plus `"force":true,"reason":"resume sp-SP-DHA4-p2 attempt <ATTEMPT>"`. Any other holder: report it and stop.

**L2 - in the world.** Cost: 1 call. `lab_ensure_in_world {"lease_id":L,"character":"<NAME>"}`. `<NAME>` is the last name only, as in CHARACTER. Never pass a `create` argument: it can delete a character. The character already exists (the coordinator created and staged it before this brief ran). Playing it applies its staged fixture on this Play (the staging lapses 10 minutes after the coordinator made it, so you are dispatched right after it). Expected: `in_world:true`. Retry once; if it fails again report BLOCKED with the error text.

**WAIT.** Cost: 1 call. `client_batch {"lease_id":L,"steps":[{"op":"wait","ms":20000},{"op":"wait","ms":20000},{"op":"lua","id":"utc","chunk":"if os and os.date then return os.date('!%H:%M:%S') end return 'no-os'"},{"op":"player_state","fields":["world_id","position"]}]}`. Record world_id and position.

**WAITM - WAIT that also sends `/gmmissionlist`.** Cost: 1 call. `client_batch {"lease_id":L,"stop_on_error":false,"steps":[{"op":"wait","ms":20000},{"op":"wait","ms":20000},{"op":"lua","id":"utc","chunk":"if os and os.date then return os.date('!%H:%M:%S') end return 'no-os'"},{"op":"lua","id":"ml","chunk":"ChatMod.stickyCommand='/say '; Inst1Chat_Input:setText('/gmmissionlist'); ChatMod.onTextAccepted(Inst1Chat_Input, Inst1Chat_Input); return 'sent'"},{"op":"wait","ms":1500},{"op":"player_state","fields":["world_id","position"]}]}`. The two 20 s waits cover the 16 s hold, so the chat line is not swallowed. Record world_id and position, then READ the reply (MLIST).

**CHATX - several chat lines in ONE call.** Cost: 1 call, whatever the number of lines.
`client_batch {"lease_id":L,"stop_on_error":false,"steps":[ {"op":"lua","id":"c1","chunk":"ChatMod.stickyCommand='/say '; Inst1Chat_Input:setText('LINE ONE'); ChatMod.onTextAccepted(Inst1Chat_Input, Inst1Chat_Input); return 'sent'"}, {"op":"wait","ms":1500}, {"op":"lua","id":"c2","chunk":"ChatMod.stickyCommand='/say '; Inst1Chat_Input:setText('LINE TWO'); ChatMod.onTextAccepted(Inst1Chat_Input, Inst1Chat_Input); return 'sent'"}, {"op":"wait","ms":1500}, {"op":"lua","id":"utc","chunk":"if os and os.date then return os.date('!%H:%M:%S') end return 'no-os'"} ]}`
Replace LINE ONE and LINE TWO with the exact lines the step gives; add one lua op and one 1500 ms wait per further line. A line submitted this way is N3; for rows whose step IS a command (`.gotolocation`, `/abandonmission`) it counts as G/N2. Wait at least 1500 ms after each line. A `.bug` line (BUG) rides inside the CHATX of its step and costs nothing extra.

**READ.** Cost: 1 call. `client_chat_log {"lease_id":L,"cursor":"<new name>","contains":"<word>","max":20}` (or `"channel":"9"` for command replies). Record every returned line verbatim with `channel`, `colour`, `tabs`, `seq`. Keep LASTSEQ = the highest `seq` returned so far and pass `"since_seq": LASTSEQ` from your second read on.

**MLIST.** Cost: 2 calls (a CHATX with the line `/gmmissionlist`, then READ), or 1 call (the READ alone) right after a WAITM, which already sent the line. READ is `{"cursor":"<new>","contains":"gmMissionList","max":10}`. The server prints step ids, never step text: the reply reads like `gmMissionList (1): #1664 active step 5225 obj 0/2`. Record the whole `#1664 ...` entry, or the whole line when 1664 is absent.

**SHOT.** Cost: 1 call. `lab_screenshot {"lease_id":L,"instance":"p3"}`. Report the path next to the LABEL the step gives it.

**FIND.** Cost: 1 call. `client_entity_find {"lease_id":L,"name":"<name>","max_distance_m":12,"fields":["matches"]}`. Record id, name, distance_m and position of every match.

**TALK.** Cost: 1 call. `client_world_click {"lease_id":L,"name":"<name>","button":"right","expect":"<window|nothing>","settle_ms":2500}` (right = interact). Use `expect:"window"` where the step says a dialog opens, `expect:"nothing"` where the reply is a chat line only. Record every result field (`hover`, `range`, `native_level`, `evidence`, any `error`).

**DIALOG - close an orphan dialog.** Cost: 0 calls when it is the first lua op of a `client_batch` the step already makes (the step says so), otherwise 1 call. The op: `{"op":"lua","id":"dlg","chunk":"if DialogWin ~= nil and DialogWin:isVisible() then DialogMod.onDialogDoneClicked(DialogWin, DialogWin) return 'closed' end return 'none'"}`. On its own: `client_batch {"lease_id":L,"steps":[<the op>]}`.

**PROMPT (failure path only, gotcha 12).** Cost: 1 call, plus 1 per button clicked. `client_window_read {"lease_id":L,"kind":"prompt"}`; if `state.prompts` is non-empty click its first enabled button by the returned `window` name (`Prompt1_SingleButton` for Server Message); repeat until empty.

**RELOG.** Cost: 3 calls: (1) `lab_logout {"lease_id":L}`; (2) L2 with the same character; (3) WAIT, or WAITM where the step says so. The server restores cues at login (chains 6014, 6015, 6033, 6034), so the first thing after a RELOG is a SHOT of the NPC the step names.

**BUG - a timeline bookmark.** A step that says `BUG <text>` is a CHATX line `.bug SPU SP-DHA4-p2 <text>`. The verifier reads these on the server to find your row timing. They are not optional.

# SP-DHA4-p2

ROWS: FZ-U2h part h1
ESTIMATE: 18 tool calls (planned success path; hard stop 28)

## Steps

### S1 - lease, character, wait

**S1 (3 calls: lab_lease_acquire, lab_ensure_in_world, client_batch).** L1, keep `lease_id` as L. L2 with the CHARACTER. WAITM.

### S2 - RESUME PROBE

**S2 (1 call: client_chat_log).** READ `{"cursor":"s2","contains":"gmMissionList","max":10}`. Record the world_id and position from the WAITM and the `#1664 ...` entry.

On ATTEMPT 1 only: the entry must read `#1664 active step 5226 obj 0/2`. Anything else means the fixture did not apply: record BLOCKED for the row (gotcha 19), skip the steps and go to the end. On ATTEMPT 2 or 3 the coordinator re-staged the character first (see the pre-step), so the same entry is expected; if it is not, record BLOCKED the same way.

Server facts: a Human (a Soldier) with mission 1361 not active. Marsh's "?" is dialog 4457 on chains 6511 (bind, `db/resources/Content/Seed/harset_opcore_chains.sql:557-623`) and 6512 (click, `db/resources/Content/Seed/harset_opcore_chains.sql:625-643`). Both need world 68, archetype not Jaffa/Goa'uld, 1361 `not_active` and 1664 not active on 5226. 6511 re-runs on `player_loaded Harset_CmdCenter`, `mission_completed 1664` and `mission_abandoned 1664` (`db/resources/Content/Seed/harset_opcore_chains.sql:594-599`). Never press Accept on dialog 4457 (that starts mission 1361): close it with DIALOG.

This part, h1: before and after 1664 ends.

### ROW FZ-U2h part h1 - before and after 1664 ends (Command Center 68)

**h1.1 (2 calls: client_batch, lab_screenshot).** CHATX `.bug SPU SP-DHA4-p2 FZ-U2h1 start`, `.gotolocation Harset_CmdCenter -49.5 -0.59 -18`. SHOT `H1-BEFORE` (NEEDS_HUMAN: Marsh shows the 1664 debrief cue, not the Praxis "?").

**h1.2 (3 calls: client_world_click, client_chat_log, client_ui_state).** TALK `Marsh`, `expect:"nothing"` (a Marsh click on 5226 is the 1664 report, so no dialog 4457). READ `{"cursor":"h1a","contains":"Report received","max":10}`. Call `client_ui_state {"lease_id":L,"chat_lines":0}` once and record whether any dialog is open (expect none).

**h1.3 (3 calls: client_batch, client_world_click, client_chat_log).** CHATX `.gotolocation Harset_CmdCenter -5 0.37 -12`. TALK `katan`, `expect:"nothing"` (the second report completes 1664). READ `{"cursor":"h1b","contains":"filed","max":10}`.

**h1.4 (5 calls: client_batch, lab_screenshot, client_world_click, lab_screenshot, client_batch).** CHATX `.gotolocation Harset_CmdCenter -49.5 -0.59 -18`. SHOT `H1-AFTER` (NEEDS_HUMAN: the "?" offer cue is back). TALK `Marsh`, `expect:"window"`: dialog 4457 should open. Record the result fields. SHOT `H1-DIALOG`. DIALOG on its own, then the part is over.


## 2. Report format (your last message, nothing else)

Header: `PACKET SP-DHA4-p2 ATTEMPT <n> | lease released: yes/no | tool calls used: <n> | stopped early: no / <why>`.
Then one block per row (and per sub-step where the row lists them):
`ROW <id> | CLAIM: DONE / FAIL / BLOCKED / NOT-RUN / NEEDS_HUMAN | UTC <start>-<end> | tier: N1/N2/N3/G (the lowest you used) | raw: <the numbers, tool fields and chat lines, verbatim, in quotes> | shots: <LABEL>=<path>, ...`
"DONE" means you performed the step and saw something; the verifier decides PASS or FAIL. A NEEDS_HUMAN step lists its screenshot path and one line of what the tool results showed. End with `NEEDS_HUMAN SHOTS: <LABEL=path, ...>` and `UNEXPECTED: <anything odd, or none>`. Do not interpret, advise or apologise.


## 3. Always end like this

**Release (1 call: lab_lease_release).** `lab_lease_release {"lease_id":L}`, even after a failure or an early stop. Then write the report.
