You are a `lab-driver` agent. You drive one live Stargate Worlds game client through the cimmeria-lab tools and report raw facts. You do not diagnose, judge, fix, edit files or query databases. You can read nothing except tool results. You have never seen this game: do exactly what this brief says, in order, and nothing else.

BRIEF FIELDS (the coordinator filled these in before pasting):

- PACKET: SP-DCA3-p1
- ATTEMPT: 1   (1 = first try. 2 or 3 = you are the successor of a driver that ran out of context: do the RESUME PROBE and skip what is already done)
- INSTANCE: default   (the lab client you drive; account `the lab account of that instance`)
- CHARACTER: <FIXTURE-NAME>   (the coordinator replaces this with the exact character name `server_stage_fixture` returned, before pasting)
- SERVER: the local lab server. Lab use is pre-authorized. Never ask the user anything.

## 0. Front-loaded gotchas (read all of them)

1. **Lease first.** Every driving tool needs the `lease_id` from step L1. Pass it as `lease_id` on every call. On `lab_screenshot` also pass `"instance": "default"` (a screenshot without it shows the wrong client).
2. **Never use `image: true`** on a screenshot. The tool saves a PNG and returns `image saved: <path>`. Report the path. Someone else looks at the picture.
3. **Chat commands are GM commands.** Your account is a GM. Lines starting with `.` are server commands; lines starting with `/` are game commands. Close any open dialog first (see DIALOG below), because a chat line under an open dialog orphans it.
4. **Teleport only with `.gotolocation <World> x y z`.** Never `.gotoxyz` (it moves your selected target, not you). Coordinates are server metres (x, y up, z). Never teleport onto a hostile NPC and never use `client_move_to` in a hub. The Castle interior and exterior navmesh are not connected: move between them only with a `.gotolocation` (tier G).
5. **The character is staged by the coordinator** (`server_stage_fixture`, a `world = Castle` fixture), so it lands at character select. Never log a fresh client straight into Castle (#756). `lab_ensure_in_world` plays the staged character. A just-played character is deaf for about 16 seconds: always run WAIT after L2 before anything else.
6. **Read chat replies only with `client_chat_log`.** Server replies arrive on chat channel id `"9"`. Pass a `cursor` name that is new for that read AND a `contains` or `channel` filter (a new cursor starts at the oldest line in the ring). A Lua read of the chat output returns nil. If a reply is missing, send the command once more, read again, then record "no reply".
7. **Sticky chat channel.** Every chat macro below resets it with `ChatMod.stickyCommand='/say '`. Do not remove that.
8. **No apostrophes** inside the Lua text of a chat macro (they end the Lua string). Use the lines exactly as written.
9. **Stand 4 to 6 m from a target** before clicking it (server interact range 5.0 m; the Castle rows give the exact standing point). Teleport next to it, then click. Entity ids change after a relog or zone change: call `client_entity_find` by name right before you use an id.
10. **A click that opens nothing:** `lab_screenshot` once and record the path; run PROMPT only if that SHOT shows a window or prompt name (otherwise skip it, no call); retry the click once unchanged; then record FAIL for that step and go on to the next independent step. Never improvise.
11. **A covering window swallows native clicks (#1676).** Before every world click, the step's batch runs DIALOG as its first op. Run PROMPT only when the preceding L2, WAIT, WAITW or screenshot output shows a window or prompt name; if it shows none, skip PROMPT (no call). On a dead click the order is SHOT, then PROMPT only if the SHOT shows a window or prompt name, then one unchanged retry. If a click still does nothing, read `client_ui_state` once and record the top window.
12. **The Server Message OK button is `Prompt1_SingleButton`.** Use PROMPT; never guess a button name.
13. **No `client_wait_for` on a world load (#1677).** After `lab_logout` + `lab_ensure_in_world` use WAIT (two `client_batch` waits of 20 s). After a death, `lab_ensure_in_world` can misread the character name (#1680): re-check `player_state` before trusting it.
14. **Friendly NPCs give no kill credit (K20).** Never `.givexp` or fight to advance a row unless the row says so. Level, cash and ability changes use `.givexp`, `/gmgivecash`, `/gmgiveability` (tier G); say so in the report.
15. **Do not restart or relaunch the client** and do not call `lab_client_*` or `lab_client_restart`. If the client is dead (every call fails), stop and report BLOCKED with the error text.
16. **Budget: 28 tool calls hard stop** (lease TTL 900 s). Count them. At call 26 stop doing rows, write the report, release the lease. Never start a row you cannot finish. Put reads in one `client_batch`.
17. **One retry per failed call, unchanged.** Then record the error text verbatim and continue. Never write IPs, passwords or account names.
18. **NEEDS_HUMAN never blocks the lane.** Where a step says NEEDS_HUMAN (a look or feel the server log cannot prove), take a `lab_screenshot` (SHOT), record the saved path and a one-line description of what you saw (window titles, visible text), mark the row `CLAIM: NEEDS_HUMAN`, and carry on at once. The user reviews the screenshots at the end. Never wait, never ask.
19. **Area.** All rows here use the Castle Infirmary (region `Castle.Infirmary`, x 344.92-371.04, z 880.18-904.65, y 70.15-74.48; `db/resources/Events/Seed/point_sets.sql` set 2051). Two lanes must not run in the Infirmary at the same time: the coordinator schedules this part alone in the Infirmary.

## 1. Fixed macros (use them verbatim)

**Counting (read once).** A call is one MCP tool invocation, whatever the tool: a `client_batch` of 12 steps is 1 call, two tools in one step are 2 calls, and a tool-schema or tool-search lookup is a call too. The label on each step lists one tool per call, in order. ESTIMATE is the sum of the labels on the planned success path (L1 to the lease release). Failure-path calls are not counted and use the headroom to the hard stop of 28: gotcha 10 SHOT and retry, gotcha 17 retries, a lease refusal, PROMPT after a click that opened nothing, the JOURNAL key-and-reread fallback. Macro costs: L1 = 1; L2 = 1; WAIT = 1; CHATX = 1 whatever the number of lines; CHAT1 = 3; READ = 1; SHOT = 1; JOURNAL = 1; PROMPT = 1 plus 1 per button clicked; DIALOG = 0 as the first `lua` op of a `client_batch` the step already makes, otherwise 1; BUG = 0 inside a CHATX, otherwise 1. Never read a tool schema: every tool argument you need is written out below.

**L1 - lease.** `lab_lease_acquire {"owner":"sp-SP-DCA3-p1","purpose":"starter UAT SP-DCA3-p1","instance":"default","ttl_s":900}` -> keep `lease_id` (L). If refused: `lab_lease_status {"instance":"default"}`. If the holder owner is exactly `sp-SP-DCA3-p1` AND ATTEMPT is 2 or 3, call `lab_lease_acquire` again with the same arguments plus `"force":true,"reason":"resume sp-SP-DCA3-p1 attempt <ATTEMPT>"`. Any other holder: report it and stop. Cost: 1 (lab_lease_acquire).

**L2 - in the world.** `lab_ensure_in_world {"lease_id":L,"character":"<FIXTURE-NAME>"}` (the character exists already; do not pass `create`; `<FIXTURE-NAME>` is the CHARACTER field above, copied exactly). Expected `in_world:true`, `world_id` 8 (Castle). If it fails, retry once; then BLOCKED with the error text. Cost: 1 (lab_ensure_in_world).

**WAIT.** `client_batch {"lease_id":L,"steps":[{"op":"wait","ms":20000},{"op":"wait","ms":20000},{"op":"lua","id":"utc","chunk":"if os and os.date then return os.date('!%H:%M:%S') end return 'no-os'"},{"op":"player_state","fields":["world_id","position"]}]}`. `client_entity_find` returning any NPC proves the hold is over. Cost: 1 (client_batch).

**UTC.** `{"op":"lua","id":"utc","chunk":"if os and os.date then return os.date('!%H:%M:%S') end return 'no-os'"}` in every batch you already run. On `no-os` report chat `seq` numbers instead. Cost: 0, it is an op inside a batch you already run.

**CHATX - send chat lines in ONE call.** `client_batch {"lease_id":L,"stop_on_error":false,"steps":[ {"op":"lua","id":"c1","chunk":"ChatMod.stickyCommand='/say '; Inst1Chat_Input:setText('LINE ONE'); ChatMod.onTextAccepted(Inst1Chat_Input, Inst1Chat_Input); return 'sent'"}, {"op":"wait","ms":1500}, {"op":"lua","id":"utc","chunk":"if os and os.date then return os.date('!%H:%M:%S') end return 'no-os'"} ]}` Replace `LINE ONE` with the exact lines the step gives; repeat the lua+wait pair per line. Tier N3. Cost: 1 (client_batch), whatever the number of lines.

**CHAT1 - TYPED line (N1/N2), only where a step says TYPED.** (a) `client_ui_sequence {"lease_id":L,"actions":[{"do":"click","window":"Inst1Chat_Input"},{"do":"type","text":"<LINE>","into":"Inst1Chat_Input"}]}` (b) `client_batch` Lua `return Inst1Chat_Input:getText()` must equal `<LINE>`; if not, `Inst1Chat_Input:setText('')` and repeat (a) once. (c) only if matched: `client_ui_sequence {"lease_id":L,"actions":[{"do":"key","key":"Enter"},{"do":"wait","ms":2000}]}`. Cost: 3 (client_ui_sequence, client_batch, client_ui_sequence).

**READ - chat replies.** `client_chat_log {"lease_id":L,"cursor":"<new name>","contains":"<word>","max":20}`. Record every line verbatim with `channel`, `colour`, `tabs`, `seq`. Keep LASTSEQ = highest `seq` returned so far; from the second read add `"since_seq": LASTSEQ`. Cost: 1 (client_chat_log).

**SHOT.** `lab_screenshot {"lease_id":L,"instance":"default"}`; report the path next to the LABEL the step gives. Cost: 1 (lab_screenshot).

**DIALOG - close an orphan dialog.** `client_batch` Lua `if DialogWin ~= nil and DialogWin:isVisible() then DialogMod.onDialogDoneClicked(DialogWin, DialogWin) return 'closed' end return 'none'`. A tutorial window on top: `client_window_click {"lease_id":L,"target":"TutorialWin__auto_closebutton__"}`. Cost: 0 as the first `lua` op of a `client_batch` the step already makes, otherwise 1.

**PROMPT.** `client_window_read {"lease_id":L,"kind":"prompt"}`. If `state.prompts` is non-empty, read title and message, click the first enabled returned button (normally `Prompt1_SingleButton`) by its `window` name; repeat until empty. Cost: 1 (client_window_read) plus 1 per button clicked.

**BUG - timeline bookmark.** `BUG <text>` is the CHATX line `.bug SPU SP-DCA3-p1 <text>`. The coordinator reads these on the server to find your row timing. Not optional. Cost: 0 inside a CHATX, otherwise 1.

**JOURNAL - read the journal.** `client_window_read {"lease_id":L,"kind":"mission"}` and record the mission names and step text it returns verbatim. If it returns nothing, open the journal with a real key (`L`), run it again, and record the error text if it still returns nothing. Never invent a Lua journal call. Cost: 1 (client_window_read); the key-and-reread fallback is a failure-path call.

---

# SP-DCA3-p1: mission 1520 relog restore at 4452 and 4453 (D6a, D6b)

ROWS: D6a, D6b. Run order: D6a, D6b. D6 is split by step: D6c (the 4454 relog) is `SP-DCA3-p2`, D7 is in `SP-DCA3-p2`, and D5 is in `SP-DCA3-p3`. ESTIMATE: 22 tool calls (planned success path; hard stop 28)
Start: fixture. The coordinator staged it with `server_stage_fixture` (C4): `world = Castle`, `position = [466.365, 70.397, 991.466]`, `missions = [{"id":704,"state":"completed"},{"id":1520,"step":4452}]`, a Praxis Soldier profile (archetype 1). Fixture failure makes the rows BLOCKED, never FAIL. Within this part D6b continues the same character after D6a (the relog at 4453 is a real logout and login).

Facts this brief relies on (traced from the seed in this worktree):

- Steps and chains: the same as SP-DCA2-p1 and SP-DCA2-p2 (`db/resources/Missions/Seed/mission_steps.sql:2052-2056`; `db/resources/Content/Seed/castle_1520_chains.sql`, all `enabled = true`, included at `db/database.sql:427`). Relog chains on `player_loaded Castle`: 1381 at step 4452 rebinds dialog set 5827 slot 179 (lines 49-52); 1382 at 4453 rebinds 5828 slot 412 (54-57); 1383 at 4454 runs `complete_mission 1520` (59-62; the radio call cannot be shown). 1379 (`dialog_choice 4984`, `display_dialog 4985`, lines 37-40) and 1373 (`dialog_choice 4985`, `complete_mission 1520`, lines 25-28) are the D5 path. 1378 (lines 31-34) advances 4452 to 4453 when 4983 closes.
- Spawns: Ogilvie (362.0, 70.18, 886.0), tag `Castle_Ogilvie`, `db/resources/Worlds/Seed/spawnlist.sql:623`; dispenser tag `Castle_MedDispenser`, (356.0, 70.18, 892.0), template 412, line 628. Dialogs 4984 and 4985: `db/resources/Dialogs/Seed/dialogs.sql:8333-8335`. The 4985 label reads "Med Dispenser" (cosmetic; do not report it as a failure).
- Standing points (4.5 m off): Ogilvie (362.0, 70.18, 890.5); dispenser (356.0, 70.18, 887.5).
- The fixture login IS a `player_loaded Castle` load, so chain 1381 (step 4452) runs at the login: it adds dialog set 5827 slot 179 to Ogilvie. That is what D6a checks.

## Steps

**S1 (3 calls: lab_lease_acquire, lab_ensure_in_world, client_batch).** L1, L2, WAIT. Record world_id and position.

**S2 resume probe (1 call: client_window_read).** `client_window_read {"lease_id":L,"kind":"mission"}`. Record the step text. Never start a row that is already past its precondition: step text "Hack the Med Dispenser..." means D6a is done, go to ROW D6b step 6b.1.

### ROW D6a - login at 4452 rebinds Ogilvie; the click opens 4983; the close advances to 4453

**6a.1 (6 calls: client_batch, client_entity_find, client_window_read, lab_screenshot, client_world_click, client_window_read).** One batch: the DIALOG op first, then the CHATX lines `.bug SPU SP-DCA3-p1 d6a start` and `.gotolocation Castle 362.0 70.18 890.5` with a wait of 3500 after the move, then the UTC op. Then `client_entity_find {"lease_id":L,"name":"Ogilvie"}`; the mission read (`client_window_read {"lease_id":L,"kind":"mission"}`); SHOT label `D6a-ogilvie-at-4452` (record whether Ogilvie is found; the glyph look is NEEDS_HUMAN). Then `client_world_click {"lease_id":L,"entity_id":<Ogilvie id>,"expect":"window"}` (N1; if it opens nothing, gotcha 10 then PROMPT, failure path) and `client_window_read {"lease_id":L,"kind":"dialog"}`: record that dialog 4983 opened (this proves the rebind worked).
**6a.2 (2 calls: client_batch, client_window_read).** Close it: one batch with the DIALOG op first, a wait of 2500 and the UTC op. Then the mission read: expect step text "Hack the Med Dispenser..." (`mission_steps.sql:2054`).

### ROW D6b - relog at 4453 rebinds the dispenser; the click starts Livewire

**6b.1 (2 calls: client_batch, lab_logout).** One batch with the DIALOG op first, the CHATX line `.bug SPU SP-DCA3-p1 d6b start` and the UTC op. Then `lab_logout {"lease_id":L}`.
**6b.2 (1 call: lab_ensure_in_world).** L2.
**6b.3 (1 call: client_batch).** WAIT (17 s hold plus world load).
**6b.4 (5 calls: client_batch, client_entity_find, client_window_read, lab_screenshot, client_world_click).** One batch: the DIALOG op first, then `.gotolocation Castle 356.0 70.18 887.5` with a wait of 3000 and the UTC op. Then `client_entity_find {"lease_id":L,"name":"Dispenser"}` (tag `Castle_MedDispenser`; if nothing is found, run once `client_entity_find {"lease_id":L,"max_distance_m":10,"limit":10}` as a failure-path call and pick the entity whose name contains `Dispenser` or `Med`); the mission read; SHOT label `D6b-dispenser-at-4453` (the glow look is NEEDS_HUMAN). Then click the dispenser: `client_world_click {"lease_id":L,"entity_id":<dispenser id>,"expect":"window"}` (N1); record that a Livewire window opens (this proves chain 1382 rebound it). Do not play the minigame; leave the window open and go to S9.

**S9 (1 call: lab_lease_release).** Release the lease and write the report. Rows: D6a, D6b; screenshot labels `D6a-ogilvie-at-4452`, `D6b-dispenser-at-4453`.

---

## 2. Report format (your last message, nothing else)

Header: `PACKET SP-DCA3-p1 ATTEMPT <n> | lease released: yes/no | tool calls used: <n> | stopped early: no / <why>`.
One block per row: `ROW <id> | CLAIM: DONE / FAIL / BLOCKED / NOT-RUN / NEEDS_HUMAN | UTC <start>-<end> | tier: N1/N2/N3/G (the lowest you used) | raw: <numbers, tool fields and chat lines, verbatim, in quotes> | shots: <LABEL>=<path>, ...`
"DONE" means you performed the step and saw something; the verifier decides PASS or FAIL. NEEDS_HUMAN rows always carry a screenshot path and a one-line "I saw". End with `UNEXPECTED: <anything odd, or none>`. Do not interpret, advise or apologise.

## 3. Always end like this

Release the lease: `lab_lease_release {"lease_id":L}`, even after a failure or an early stop. Then write the report.
