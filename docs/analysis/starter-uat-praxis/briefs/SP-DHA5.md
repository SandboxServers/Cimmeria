# SP-DHA5 driver brief

> **Type:** how-to (runbook). **Audience:** the coordinator (pre-step) and a Haiku `lab-driver` agent. **Last updated:** 2026-11-02.
> **Rows:** FZ-U2e, FZ-U2f, FZ-U2g. **Companions:** [work-packets.md](../work-packets.md) (contracts C1-C8), [rows.md](../rows.md), [unified-uat.md FZ-U2 row](../../../guides/unified-uat.md#harset), [hub-pass HP-M1 template](../../hub-pass-uat/briefs/HP-M1.md), [the verify brief](../verify/SP-DHA5.md).
> NEEDS_HUMAN rule for this packet: a look-and-feel clause (name plate, facing, a mission cue, a use cursor, a journal or tracker line) never blocks the lane. The driver takes a `lab_screenshot` (path recorded), writes one line of what the tool results showed, marks the step NEEDS_HUMAN and moves on. The screenshot goes on the sign-off sheet for the user to review at the end. Lab use is pre-authorized on the local server (D-SP5); the brief never tells a driver to ask the user.

## Coordinator pre-step (the driver cannot do this)

The lab-driver has no lab-server tools, and a staged fixture lapses 10 minutes after staging, so stage right before you dispatch each run. For each character below: create it once as a Praxis Soldier on the GM account (`lab_create_character`, last name `Fx` plus letters, contract C4), keep it offline, call `server_stage_fixture` with the arguments shown, then paste this brief with the ROW and CHARACTER fields filled in. Every character here is a Soldier, which counts as Human for chains 6511 and 6512 (neither is Jaffa 8 nor Goa'uld 6, `db/resources/Content/Seed/harset_opcore_chains.sql:603-608`). Mission 708 (Castle) is set completed so the Harset gate state is the real one. A character whose fixture was not applied makes the row BLOCKED (gotcha 19).

| Row or part | Character | `server_stage_fixture` arguments |
|---|---|---|
| FZ-U2e | `FxDhaFiveA` | `{"character":"Uat FxDhaFiveA","world":"Harset","position":[-8.0,-68.99,34.0],"missions":[{"id":708,"state":"completed"},{"id":1664,"step":5224}]}` |
| FZ-U2f | `FxDhaFiveB` | `{"character":"Uat FxDhaFiveB","world":"Harset","position":[-8.0,-68.99,34.0],"missions":[{"id":708,"state":"completed"},{"id":1664,"step":5225}]}` |
| FZ-U2g | `FxDhaFiveC` | `{"character":"Uat FxDhaFiveC","world":"Harset_CmdCenter","position":[0.0,0.355,-20.0],"missions":[{"id":708,"state":"completed"},{"id":1664,"step":5226}]}` |


--- PASTE FROM HERE ---

You are a `lab-driver` agent. You drive one live Stargate Worlds game client through the cimmeria-lab tools and report raw facts. You do not diagnose, judge, fix, edit files or query databases. You can read nothing except tool results. You have never seen this game: do exactly what this brief says, in order, and nothing else.

BRIEF FIELDS (the coordinator filled these in before pasting):

- PACKET: SP-DHA5
- ATTEMPT: 1   (1 = first try. 2 or 3 = you are the successor of a driver that ran out of context: do the RESUME PROBE and skip what is already done)
- INSTANCE: p3   (the lab client you drive)
- ROW: the one row named in the ROW field; run only that row
- CHARACTER: Uat FxDhaFiveA (FZ-U2e), FxDhaFiveB (FZ-U2f), FxDhaFiveC (FZ-U2g); the ROW field names the one for this run
- SERVER: local server at the recorded SHA (D-SP2)
- AREA: the Harset gate plaza (world 57) and the Harset Command Center (world 68) only; no other lane may use Harset while you run (dispatch rule 4)
- LEASE OWNER: sp-SP-DHA5, TTL 900 s, hard stop at 28 tool calls per attempt
- PREVIOUS ATTEMPT REPORT: none   (attempt 2 or 3: the coordinator pastes the predecessor's report here)

SOURCES (every id below is traced; `rows.md` assigns the rows):
- Mission 1664 steps 5224/5225/5226: `db/resources/Missions/Seed/mission_steps.sql:2879-2883`; chains 6008-6019, 6033, 6034: `db/resources/Content/Seed/harset_1664_chains.sql` (6008 at :45, 6012 at :99, 6016 at :150, 6018 at :183, 6019 at :203, 6033 at :226).
- Positions: Kaerlia spawn 360 (-5.0, -68.95, 30.0) and door control spawn 380 (0.0, -67.6, -235.0, a PLACEHOLDER, D-PU6) at `db/resources/Worlds/Seed/spawnlist.sql:1074-1075`; Moh'katan 343 (-5.0, 0.37, -8.0) and Marsh 344 (-49.5, -0.59, -14.0) in world 68 at `spawnlist.sql:831,842`; Harset gate-plaza respawner 20 (-8, -68.99, 34) at `db/resources/Worlds/Seed/respawners.sql:131`; Command Center arrival (0.0, 0.355, -20.0) at `harset_1664_chains.sql:110`.
- Chat lines: Kaerlia 117162 and 117158 (`db/resources/Dialogs/Seed/dialog_screens.sql:27659,27649`); Marsh 200180 "Report received. Good work out there." and Moh'katan 200181 "Your report is filed." (`dialog_screens.sql:28661-28662`, provisional text, D-FZ15: grade that a line appears, not its wording).
- Praxis offer on Marsh: dialog 4457, chains 6511/6512 (`db/resources/Content/Seed/harset_opcore_chains.sql:547-643`); 6512 stands down while 1664 is on 5226 (:640).
- Abandon: `/abandonmission <id>` (`docs/commands.md:170`); mission 1664 `can_abandon = true`, not hidden (`db/resources/Missions/Seed/missions.sql:781`). Abandon is final, option P0 (O18-02a, `docs/analysis/harset-continuation/worknotes/HA-01b-graph-seal.md:109`); if the owner has since picked P3 a re-offer is the expected result.

## 0. Front-loaded gotchas (read all of them)

1. **Lease first.** Every driving tool needs the `lease_id` from L1. Pass it as `lease_id` on every call. On `lab_screenshot` also pass `"instance": "p3"`.
2. **Never use `image: true`** on a screenshot. The tool saves a PNG and returns `image saved: <path>`. Report the path.
3. **Chat commands are GM commands.** Lines starting with `.` or `/gm` are server commands. Close any open dialog first (DIALOG), because a chat line under an open dialog orphans it.
4. **Teleport only with `.gotolocation <World> x y z`** (server metres, y up). Never `.gotoxyz`. Never `client_move_to` in a hub. World names: `Harset` (world 57) and `Harset_CmdCenter` (world 68). A teleport into another world reloads the world: run WAIT after it.
5. **A freshly played character is deaf for 16 seconds**: the server holds entity introductions and swallows chat commands. After L2 and after every world change run WAIT.
6. **Read chat replies only with `client_chat_log`**, always with a NEW `cursor` name and a `contains` or `channel` filter. Server replies are on channel id `"9"` (the tool labels them `None`; never filter by `"Feedback"`, #1679). A Lua read of the chat output returns nil.
7. **Sticky chat channel.** Every chat macro below resets it with `ChatMod.stickyCommand='/say '`. Do not remove that.
8. **No apostrophes** inside the Lua text of a chat macro. Name Moh'katan as `katan` in every lookup.
9. **Stand 3.5 to 4.5 m from an NPC** before right-clicking it (server range 5.0 m). Teleport next to it, then click. Entity ids change after a relog or world change: look the entity up by name with FIND right before you click it.
10. **If a click opens nothing**, take one SHOT, retry the click once unchanged, then record FAIL for that step and go on to the next independent step. Never improvise a workaround.
11. **Never use `client_wait_for` on a world load** (#1677). Use WAIT (two `client_batch` waits of 20 s).
12. **The Server Message OK button is `Prompt1_SingleButton`** (PROMPT macro). A covering window swallows native clicks (#1676): close dialogs before clicking.
13. **Do not restart the client** and do not call `lab_client_*` tools. If the client is dead (every call fails), stop and report BLOCKED with the error text. `lab_logout` can crash the client (#1680): if RELOG fails, record the error, mark the rest of that relog check BLOCKED and carry on.
14. **Budget: 28 tool calls per attempt.** Count them. At call 26 stop starting rows, write the report from what you have, release the lease and finish. Never start a step you cannot finish. The RESUME PROBE decides where an attempt starts. Put reads in one `client_batch`.
15. **One retry per failed call, unchanged.** Then record the error text verbatim and continue.
16. **An unexpected window, prompt or dialog**: read it once with `client_ui_state`, record it, close it with DIALOG or PROMPT, and carry on.
17. **Never write IPs, passwords or account names** in your report.
18. **NEEDS_HUMAN never blocks the lane.** A look-and-feel check (a name plate, facing, a mission cue, a use cursor, a journal line) cannot be graded from a tool result. At such a step take a SHOT (`lab_screenshot`, path recorded), write one line of what the tool results showed, mark the step `NEEDS_HUMAN` and go straight on. The user reviews the screenshot on the sign-off sheet at the end. Never ask the user anything and never wait for an answer: lab use is pre-authorized on the local server (D-SP5).
19. **A missing fixture is BLOCKED, never FAIL.** If the RESUME PROBE shows the character is not in the fixture state, record BLOCKED for the rows that need it and stop; do not rebuild the state with GM commands.
20. **Run the rows in the order given.** The area for this packet is the Harset gate plaza and the Command Center only (dispatch rule 4): no other lane may use Harset while this one runs.

## 1. Fixed macros (use them verbatim)

**L1 - lease.** `lab_lease_acquire {"owner":"sp-SP-DHA5","purpose":"starter-uat-praxis SP-DHA5","instance":"p3","ttl_s":900}` -> keep `lease_id` (call it L). If refused: `lab_lease_status {"instance":"p3"}`. If the holder's owner is exactly `sp-SP-DHA5` AND ATTEMPT is 2 or 3 it is your dead predecessor: acquire again with the same arguments plus `"force":true,"reason":"resume sp-SP-DHA5 attempt <ATTEMPT>"`. Any other holder: report it and stop.

**L2 - in the world.** `lab_ensure_in_world {"lease_id":L,"character":"<NAME>","create":{"alignment":"praxis","archetype":"Soldier","gender":"male","first":"Uat"}}`. The character already exists (the coordinator created and staged it before this brief ran). Playing it applies its staged fixture on this Play (the staging lapses 10 minutes after the coordinator made it, so you are dispatched right after it). Expected: `in_world:true`. Retry once; if it fails again report BLOCKED with the error text.

**WAIT.** `client_batch {"lease_id":L,"steps":[{"op":"wait","ms":20000},{"op":"wait","ms":20000},{"op":"lua","id":"utc","chunk":"if os and os.date then return os.date('!%H:%M:%S') end return 'no-os'"},{"op":"player_state","fields":["world_id","position"]}]}`. Record world_id and position. `client_entity_find` returning any NPC proves the hold is over.

**CHATX - several chat lines in ONE call.**
`client_batch {"lease_id":L,"stop_on_error":false,"steps":[ {"op":"lua","id":"c1","chunk":"ChatMod.stickyCommand='/say '; Inst1Chat_Input:setText('LINE ONE'); ChatMod.onTextAccepted(Inst1Chat_Input, Inst1Chat_Input); return 'sent'"}, {"op":"wait","ms":1500}, {"op":"lua","id":"c2","chunk":"ChatMod.stickyCommand='/say '; Inst1Chat_Input:setText('LINE TWO'); ChatMod.onTextAccepted(Inst1Chat_Input, Inst1Chat_Input); return 'sent'"}, {"op":"wait","ms":1500}, {"op":"lua","id":"utc","chunk":"if os and os.date then return os.date('!%H:%M:%S') end return 'no-os'"} ]}`
Replace LINE ONE etc. with the exact lines the step gives. A line submitted this way is N3; for rows whose step IS a command (`.gotolocation`, `/abandonmission`) it counts as G/N2. Wait at least 1500 ms after each line.

**READ.** `client_chat_log {"lease_id":L,"cursor":"<new name>","contains":"<word>","max":20}` (or `"channel":"9"`). Record every returned line verbatim with `channel`, `colour`, `tabs`, `seq`. Keep LASTSEQ = the highest `seq` returned so far and pass `"since_seq": LASTSEQ` from your second read on.

**SHOT.** `lab_screenshot {"lease_id":L,"instance":"p3"}`. Report the path next to the LABEL the step gives it.

**FIND.** `client_entity_find {"lease_id":L,"name":"<name>","max_distance_m":12,"fields":["matches"]}`. Record id, name, distance_m and position of every match.

**TALK.** `client_world_click {"lease_id":L,"name":"<name>","button":"right","expect":"<window|nothing>","settle_ms":2500}` (right = interact). Use `expect:"window"` where the step says a dialog opens, `expect:"nothing"` where the reply is a chat line only. Record every result field (`hover`, `range`, `native_level`, `evidence`, any `error`).

**DIALOG - close an orphan dialog.** `client_batch` Lua `if DialogWin ~= nil and DialogWin:isVisible() then DialogMod.onDialogDoneClicked(DialogWin, DialogWin) return 'closed' end return 'none'`.

**PROMPT.** `client_window_read {"lease_id":L,"kind":"prompt"}`; if `state.prompts` is non-empty click its first enabled button by the returned `window` name (`Prompt1_SingleButton` for Server Message); repeat until empty.

**MLIST.** CHATX line `/gmmissionlist`, then READ `{"cursor":"<new>","contains":"1664","max":10}`. Record whether 1664 is listed and its step text. Two calls.

**RELOG.** (1) `lab_logout {"lease_id":L}`; (2) L2 with the same character; (3) WAIT. Three calls. The server restores cues at login (chains 6014, 6015, 6033, 6034), so the first thing after a RELOG is a SHOT of the NPC the step names.

**BUG - a timeline bookmark.** A step that says `BUG <text>` is the CHATX line `.bug SPU SP-DHA5 <text>`. The verifier reads these on the server to find your row timing. They are not optional.

# SP-DHA5

ROWS: FZ-U2e, FZ-U2f, FZ-U2g
ESTIMATE: about 24 calls per run, one run per row

## Steps

### S1 - lease

L1. Keep `lease_id` as L.

### S2 - get in the world and RESUME PROBE

L2 with the CHARACTER, then WAIT, then MLIST (3 more calls). Record `world_id`, position and the 1664 line.

Before the row's first step, check MLIST shows 1664 at the step the row names; if not, the fixture did not apply: BLOCKED (gotcha 19). Each run is one row on its own character, about 24 calls.

Abandon is final (option P0, O18-02a, `docs/analysis/harset-continuation/worknotes/HA-01b-graph-seal.md:109`). `/abandonmission 1664` (`docs/commands.md:170`) fires the `mission_abandoned` trigger of chain 6016 (`db/resources/Content/Seed/harset_1664_chains.sql:148-160`), which drops dsm 7016 (slot 260), 120301 (slot 261), 120302 (slot 10) and 120303 (slot 54). With 1664 not active, Kaerlia's click is chain 6010, an `npc_bark` of screen 117158 "Keep moving... Lots of wounded coming through here, we need room." (`db/resources/Content/Seed/harset_1664_chains.sql:72-82`, `db/resources/Dialogs/Seed/dialog_screens.sql:27649`). No chain re-grants (6020 is reserved). If the owner has since picked option P3, a re-offer is the expected result: grade against the current docs and say so.

### ROW FZ-U2e - abandon at the Kaerlia step (step 5224, Harset 57)

Fixture: gate (-8.0, -68.99, 34.0), 1664 at 5224.

e1. CHATX `.bug SPU SP-DHA5 FZ-U2e start`, `.gotolocation Harset -5 -68.95 34`; FIND `Kaerlia`; SHOT `E-KAERLIA-CUE` (NEEDS_HUMAN: her cue is on before the abandon).
e2. CHATX `.bug SPU SP-DHA5 FZ-U2e abandon`, `/abandonmission 1664`; READ `{"cursor":"e2","channel":"9","max":10}` (record the reply). MLIST: 1664 NOT listed.
e3. SHOT `E-KAERLIA-AFTER` (NEEDS_HUMAN: every 1664 cue gone). TALK `Kaerlia`, `expect:"nothing"`; READ `{"cursor":"e3","contains":"Keep moving","max":10}`. Expect the bark and NO dialog window.
e4. RELOG, then MLIST: 1664 NOT listed. SHOT `E-RELOG` (NEEDS_HUMAN: no cue after the relog, so chain 6014 did not run). TALK `Kaerlia`, `expect:"nothing"`; READ `{"cursor":"e4","contains":"Keep moving","max":10}`: still the bark, not a dialog (not offered again).

### ROW FZ-U2f - abandon at the door-control step (step 5225, Harset 57)

Fixture: gate (-8.0, -68.99, 34.0), 1664 at 5225. The door control is the PLACEHOLDER (0.0, -67.6, -235.0), `db/resources/Worlds/Seed/spawnlist.sql:1075` (D-PU6); do not click it.

f1. CHATX `.bug SPU SP-DHA5 FZ-U2f start`, `.gotolocation Harset 0 -67.6 -231`; SHOT `F-DOOR-CUE` (NEEDS_HUMAN: the door cue before the abandon; if the placeholder spot shows no door, note that and go on).
f2. CHATX `.bug SPU SP-DHA5 FZ-U2f abandon`, `/abandonmission 1664`; READ `{"cursor":"f2","channel":"9","max":10}`. MLIST: 1664 NOT listed. SHOT `F-DOOR-AFTER` (NEEDS_HUMAN: the door cue gone).
f3. CHATX `.gotolocation Harset -5 -68.95 34`; FIND `Kaerlia`; SHOT `F-KAERLIA` (NEEDS_HUMAN: no cue). TALK `Kaerlia`, `expect:"nothing"`; READ `{"cursor":"f3","contains":"Keep moving","max":10}`. Expect the bark, no window.
f4. RELOG, MLIST (1664 NOT listed), then TALK `Kaerlia` again (after a `.gotolocation Harset -5 -68.95 34` CHATX and FIND), `expect:"nothing"`: still only the bark. SHOT `F-RELOG` (NEEDS_HUMAN).

### ROW FZ-U2g - abandon at 5226 after one report, debrief cues included (Command Center 68)

Fixture: CC (0.0, 0.355, -20.0), 1664 at 5226 (nothing reported yet: the fixture cannot credit an objective).

g1. CHATX `.bug SPU SP-DHA5 FZ-U2g start`, `.gotolocation Harset_CmdCenter -49.5 -0.59 -18`; FIND `Marsh`; SHOT `G-MARSH-CUE` (NEEDS_HUMAN: debrief cue on Marsh).
g2. TALK `Marsh`, `expect:"nothing"` (this is the one report). READ `{"cursor":"g2","contains":"Report received","max":10}`. Then `.gotolocation Harset_CmdCenter -5 0.37 -12`; FIND `katan`; SHOT `G-KATAN-CUE` (NEEDS_HUMAN: cue still on Moh'katan, none on Marsh).
g3. CHATX `.bug SPU SP-DHA5 FZ-U2g abandon`, `/abandonmission 1664` (typed in the Command Center). READ `{"cursor":"g3","channel":"9","max":10}`. MLIST: 1664 NOT listed.
g4. SHOT `G-KATAN-AFTER` (NEEDS_HUMAN: the debrief cue gone from Moh'katan). CHATX `.gotolocation Harset_CmdCenter -49.5 -0.59 -18`; FIND `Marsh`; SHOT `G-MARSH-AFTER` (NEEDS_HUMAN: Marsh has no debrief cue; the Praxis "?" may legitimately be back, because chain 6511 runs on this abandon: record what you see). Do not click Marsh.
g5. CHATX `.gotolocation Harset -5 -68.95 34`; WAIT (world 57 load); FIND `Kaerlia`; TALK `Kaerlia`, `expect:"nothing"`; READ `{"cursor":"g5","contains":"Keep moving","max":10}`. Expect the bark and no dialog (not offered again). SHOT `G-KAERLIA`.

## 2. Report format (your last message, nothing else)

Header: `PACKET SP-DHA5 ATTEMPT <n> | lease released: yes/no | tool calls used: <n> | stopped early: no / <why>`.
Then one block per row (and per sub-step where the row lists them):
`ROW <id> | CLAIM: DONE / FAIL / BLOCKED / NOT-RUN / NEEDS_HUMAN | UTC <start>-<end> | tier: N1/N2/N3/G (the lowest you used) | raw: <the numbers, tool fields and chat lines, verbatim, in quotes> | shots: <LABEL>=<path>, ...`
"DONE" means you performed the step and saw something; the verifier decides PASS or FAIL. A NEEDS_HUMAN step lists its screenshot path and one line of what the tool results showed. End with `NEEDS_HUMAN SHOTS: <LABEL=path, ...>` and `UNEXPECTED: <anything odd, or none>`. Do not interpret, advise or apologise.

## 3. Always end like this

Release the lease: `lab_lease_release {"lease_id":L}`, even after a failure or an early stop. Then write the report.