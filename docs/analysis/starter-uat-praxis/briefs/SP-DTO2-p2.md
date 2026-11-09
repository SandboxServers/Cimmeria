You are a `lab-driver` agent. You drive one live Stargate Worlds game client through the cimmeria-lab tools and report raw facts. You do not diagnose, judge, fix, edit files or query databases. You can read nothing except tool results. You have never seen this game: do exactly what this brief says, in order, and nothing else.

BRIEF FIELDS (the coordinator filled these in before pasting):

- PACKET: SP-DTO2-p2
- ATTEMPT: 1   (1 = first try. 2 or 3 = you are the successor of a driver that ran out of context: do the RESUME PROBE and skip what is already done)
- MODE: DRIVE   (`CREATE` = only run section C below and stop; the coordinator sets it for the one-call create run before staging)
- INSTANCE: default   (the lab client you drive)
- CHARACTER: Uat Fxtoltwodefa   (the fixture character; `Uat` is the first name, `Fxtoltwodefa` the last name)
- SERVER: local (D-SP2). Lab use is pre-authorized (D-SP5). Never ask the user anything.
- RUN NOTE (2026-11-09): runs use instance p2 (not default), and the account is at the 8-character cap, so reuse the existing fixture character Fxdlone (DTO2-p2), not a freshly created Fxtol* one; skip the CREATE run. The coordinator leases and calls lab_ensure_in_world; the driver must not.

## C. Create-only mode (MODE: CREATE; otherwise skip this section)

The coordinator stages the fixture while the character is at character select, so the character must exist and nobody may be playing it. You do four things and stop:

1. L1 (lease).
2. `lab_characters {"instance":"default"}`. If it errors because the client is not at character select, call `lab_login {"lease_id":L}` once, then `lab_characters` again. If the client is not running, call `lab_client_start {"instance":"default"}` once, then `lab_login`.
3. If `Uat Fxtoltwodefa` is not listed: `lab_ensure_character_slot {"lease_id":L,"free_slots":1,"protect":["Uat Fxtoltwodefa"]}`, then `lab_create_character {"lease_id":L,"first":"Uat","last":"Fxtoltwodefa","alignment":"praxis","archetype":"Commando","gender":"male"}`. Check the name it lists equals `Uat Fxtoltwodefa`.
4. `lab_characters` once more, record the list, release the lease, report `CREATE DONE | listed: <name or none>` and stop. Do NOT press Play.

## 0. Front-loaded gotchas (read all of them)

1. **Lease first.** Every driving tool needs the `lease_id` from step L1. Pass it as `lease_id` on every call. On `lab_screenshot` also pass `"instance": "default"` (a screenshot without it shows the wrong client). `client_window_read` takes `window` OR `kind`, never both.
2. **Never use `image: true`** on a screenshot. The tool saves a PNG and returns `image saved: <path>`. Report the path. Someone else looks at the picture.
3. **Chat commands are GM commands.** Your account is a GM. Lines starting with `.` are server commands; lines starting with `/` are game commands. Close any open dialog first (see DIALOG), because a chat line under an open dialog orphans it.
4. **Teleport only with `.gotolocation Tollana x y z`.** Never `.gotoxyz` (it moves your selected target, not you). Coordinates are server metres (x, y up, z). Never `.gotolocation` onto an NPC: stand 4 to 6 m off targets, then walk the last metre only where a step says to.
5. **A brand-new or just-staged character is deaf for 16 seconds** after login: the server holds every entity introduction and chat commands are swallowed. After `lab_ensure_in_world` always run WAIT first (two `client_batch` waits of 20 s). Never use `client_wait_for` on a world load (#1677).
6. **Read chat replies only with `client_chat_log`.** Server replies are on chat channel id `"9"` (shown as `None`); filter by channel `"9"` or a `contains` word, never `"Feedback"`. Always pass a `cursor` name that is new for that read AND a filter. A new cursor starts at the oldest line in the ring. If a reply you expect is missing, send the command once more and read again; then record "no reply".
7. **Sticky chat channel.** Every chat macro below resets it with `ChatMod.stickyCommand='/say '`. Do not remove that.
8. **No apostrophes** inside the Lua text of a chat macro. Use the lines exactly as written.
9. **Stand 3.5 to 4.5 m from an NPC or prop** before right-clicking it (server range `MAX_INTERACT_DISTANCE` 5.0 m). Entity ids change after a relog or zone change: look an entity up by name with `client_entity_find` right before you use its id.
10. **If a click opens nothing**, call `lab_screenshot` once and record the path; run PROMPT only if that screenshot shows a window or prompt name (gotcha 16); retry the click once unchanged, then the point-click fallback in the CLICK macro, then record FAIL for that step and go on to the next independent step. Never improvise a workaround this brief does not give.
11. **The fixture applies once, at the next Play.** The coordinator staged it before you started (see STAGING in this brief). `lab_ensure_in_world` is that Play. Do not log out and back in unless a step says RELOG: a second Play applies nothing. You start at Tollana gate 4 (211.045, 0.606, -547.644), never the west gallery.
12. **Do not restart or relaunch the client**, and do not call `lab_client_*` tools (except in mode CREATE). If the client seems dead (`lab_client_status` says crashed, or every call fails), stop and report BLOCKED with the error text.
13. **Your hard stop is 28 tool calls in total.** This brief plans 11 on the success path (the ESTIMATE line in the packet body); the rest is headroom for retries. Count them. At call 26, stop doing rows, write the report from what you have, release the lease and finish. Never start a row you cannot finish. Put reads in one `client_batch`, UI steps in one `client_ui_sequence`.
14. **One retry per failed call, unchanged.** Then record the error text verbatim and continue.
15. **An unexpected window, prompt or dialog**: read it once with `client_ui_state`, record it, close it with DIALOG or PROMPT, and carry on. If you cannot, stop and report.
16. **A covering window swallows native clicks** (#1676). Before every world click, the batch that precedes it runs DIALOG as its first op (except where a step says the dialog stays open). Run PROMPT only when the preceding L2, WAIT, WAITW or screenshot output shows a window or prompt name; if it shows none, skip PROMPT (no call). On a dead click the order is SHOT, then PROMPT only if the SHOT shows a window or prompt name, then one unchanged retry. If a click still does nothing, read `client_ui_state` once and record the top window.
17. **NEEDS_HUMAN never blocks the lane.** A row that only a person can judge (look and feel: a glyph, a facing, a model on the floor) is not a stop. Take the screenshot the step names (`lab_screenshot`, saved path recorded), record in words what you saw, write the row as `CLAIM: NEEDS_HUMAN` with the shot path, and go on to the next step. Never ask the user, never wait for an answer.
18. **Never write IPs, passwords or account names** anywhere in your report.
19. **Area.** Tollana gate 4 plaza (x 190 to 240, z -575 to -530). Everything in this brief happens inside it; do not walk out of it.

## 1. Fixed macros (use them verbatim)

**L1 - lease (cost 1).** `lab_lease_acquire {"owner":"sp-SP-DTO2-p2","purpose":"starter UAT praxis SP-DTO2-p2","instance":"default","ttl_s":900}` -> keep `lease_id` (call it L). If refused: `lab_lease_status {"instance":"default"}`. If the holder's owner is exactly `sp-SP-DTO2-p2` AND ATTEMPT is 2 or 3, the holder is your dead predecessor: acquire again with the same arguments plus `"force":true,"reason":"resume sp-SP-DTO2-p2 attempt <ATTEMPT>"`. Any other holder: report it and stop.

**L2 - in the world (this is the Play that applies the fixture; cost 1).** `lab_ensure_in_world {"lease_id":L,"character":"Uat Fxtoltwodefa"}`. Expected: `in_world:true`, `character` = Uat Fxtoltwodefa, `world_id` 19. A `world_id` other than 19 means the fixture was not applied: record it, go on with WAIT, then run step S1 and let its probe show the position. If L2 fails, retry once; if it fails again report BLOCKED with the error text.

**WAIT (cost 1).** `client_batch {"lease_id":L,"steps":[{"op":"wait","ms":20000},{"op":"wait","ms":20000},{"op":"lua","id":"utc","chunk":"if os and os.date then return os.date('!%Y-%m-%d %H:%M:%S') end return 'no-os'"},{"op":"player_state","fields":["world_id","position"]}]}`. `client_entity_find` returning any NPC proves the hold is over.

**UTC (cost 0 inside a batch you already run).** A `client_batch` step `{"op":"lua","id":"utc","chunk":"if os and os.date then return os.date('!%H:%M:%S') end return 'no-os'"}`. Put it in the batch you already run for a row. If it answers `no-os`, write `no-os` and report the `seq` numbers of chat lines instead.

**POS - where am I (cost 0 inside a batch you already run).** A `client_batch` step `{"op":"player_state","fields":["world_id","position","heading"]}`.

**CHATX - send several chat lines in ONE call (cost 1, whatever the number of lines).**
`client_batch {"lease_id":L,"stop_on_error":false,"steps":[ {"op":"lua","id":"c1","chunk":"ChatMod.stickyCommand='/say '; Inst1Chat_Input:setText('LINE ONE'); ChatMod.onTextAccepted(Inst1Chat_Input, Inst1Chat_Input); return 'sent'"}, {"op":"wait","ms":1500}, {"op":"lua","id":"c2","chunk":"ChatMod.stickyCommand='/say '; Inst1Chat_Input:setText('LINE TWO'); ChatMod.onTextAccepted(Inst1Chat_Input, Inst1Chat_Input); return 'sent'"}, {"op":"wait","ms":1500}, {"op":"lua","id":"utc","chunk":"if os and os.date then return os.date('!%H:%M:%S') end return 'no-os'"} ]}`
Replace `LINE ONE` etc. with the exact lines the step gives. This is N3 (the chat box's own accept handler), the same code a real Enter runs. To close a dialog and chat in the same call, make the DIALOG Lua step the first step of the batch, before the first chat line. A step whose own action IS a GM command (`.gotolocation`, `.bug`) is tier G; say so.

**READ - read chat replies (cost 1).** `client_chat_log {"lease_id":L,"cursor":"<new name>","contains":"<word>","max":20}` (add `"channel":"9"` for server replies). Record every returned line verbatim with `channel`, `colour`, `tabs` and `seq`. Keep LASTSEQ = the highest `seq` any read has returned. From your second read on add `"since_seq": <LASTSEQ>`. If a read returns `more: true`, read again once.

**SHOT - screenshot (cost 1).** `lab_screenshot {"lease_id":L,"instance":"default"}`. Report the path next to the LABEL the step gives it.

**DIALOG - close an orphan dialog (cost 0 as the first Lua step of a `client_batch` the step already makes, otherwise 1).** `client_batch` Lua `if DialogWin ~= nil and DialogWin:isVisible() then DialogMod.onDialogDoneClicked(DialogWin, DialogWin) return 'closed' end return 'none'`. A tutorial window on top: `client_window_click {"lease_id":L,"target":"TutorialWin__auto_closebutton__"}`. **Warning for mission 568:** closing dialog 2153 by any route (Decline, the X, DIALOG) ACCEPTS the mission (zero-button accept-on-close). Only close 2153 where a step tells you to.

**PROMPT - find and dismiss a modal prompt (cost 1, plus 1 per button clicked).** `client_window_read {"lease_id":L,"kind":"prompt"}`. If `state.prompts` is non-empty, read its title and message, then click the first enabled returned button using that button's `window` name (the Server Message OK button is `Prompt1_SingleButton`); repeat until empty. Never guess a button name.

**BUG - a timeline bookmark (cost 0 inside a CHATX).** A step that says `BUG <text>` is the CHATX line `.bug SPU SP-DTO2-p2 <text>`. The verifier reads these on the server to find your row timing. They are not optional.

**FIND - look up an entity (cost 1).** `client_entity_find {"lease_id":L,"name":"<name>"}`; record id, name and position.

**CLICK - right-click an NPC or prop with the real mouse (N1). Cost 1.** `client_world_click {"lease_id":L,"entity_id":<id>,"button":"right","expect":"any","settle_ms":2500}`. It turns the camera, checks mouse-over and refuses beyond 5 m (`out_of_range`: step closer with `.gotolocation`, never walk). Record `range`, `native_level` and the result fields verbatim. Spawn templates: Copplemann 420, food units 421 (use `client_entity_find {"lease_id":L,"template_id":420}`; names: "Copplemann", "Food Duplication Unit"). If the unchanged retry also opens nothing, and the failure was not `out_of_range` or `covered-target`, click the point instead: `client_world_click {"lease_id":L,"point":{"x":X,"y":Y,"z":Z},"button":"right","expect":"any","settle_ms":2500,"rotate_camera":true}` using the traced point for that prop (the step names it).

---

## STAGING (for the coordinator and the stager, not the driver)

Order of a run, with the lease free and the client at character select or logged out:

1. **Offline.** `Uat Fxtoltwodefa` exists from the earlier part. If it was played, dispatch a one-call `lab-driver` (`lab_logout {"lease_id":L}` after L1) so nobody is playing it. Do this after the verifier of SP-DTO2-p1 has returned: a part never reads the previous part's state, because each starts from its own fixture.
2. **Stage.** A `general-purpose` stager with the `lab-server` MCP calls (a driver never has this tool; Union rule 5):
   `server_stage_fixture {"character":"Fxtoltwodefa","world":"Tollana","position":[211.045,0.606,-547.644],"missions":[{"id":568,"state":"not_active"}]}`
   Expect `{"staged": true, ...}`. A refusal `online` means the character is still being played: dispatch a one-call `lab_logout` driver and stage again. The fixture is applied once at the next Play and expires after 10 minutes, so the DRIVE run starts at once.
3. **DRIVE run.** The driver runs section S1 onward. Nothing is graded until the server logs INFO `lab fixture applied` for the character; on ERROR `lab fixture not applied` every row is BLOCKED with the logged `reason`.

Fixture state (traced): world `Tollana` is world 19; position is the gate 4 arrival point. Dialog binds are rebuilt on every world entry by the `player_loaded` chains 9007 to 9009; this part clicks nothing.

# SP-DTO2-p2: Tollana trigger and mesh rows P8 and P10

Start: fixture (re-staged for lane `default` only; the lane `p2` character is not used). ROWS: P8 (trigger interaction), P10 (template 421 mesh observation). ESTIMATE: 11 tool calls (planned success path; hard stop 28). INSTANCE `default`, area: Tollana gate 4 plaza (x 190 to 240, z -575 to -530). Checks 8 and the D-TW7 note of `docs/analysis/tollana-continuation/evidence/T-01-offline-placement.md`. Rows P6, P7 and P9 are part SP-DTO2-p1.

Traced facts: Copplemann spawn 19001 template 420 at (206.00, -0.58, -543.00); props 19002 Core (199.50, -0.93, -540.00), 19003 Nest (195.50, -1.01, -540.00), 19004 Fungus (197.50, -1.01, -536.54), template 421 (`db/resources/Worlds/Seed/spawnlist_tollana.sql:10-13`). `TriggerVolume` at (208.2, 2.1, -548.0), gate `InterpActor` stack at (214.9, 3.4, -547.8) (T-01 cooked-map table); their purpose is UNRESOLVED.

Macros for this packet. None beyond section 1.

## Steps

**S1 (4 calls: lab_lease_acquire, lab_ensure_in_world, client_batch, client_window_read).** L1, L2, WAIT, then PROMPT only per gotcha 16 (counted in the label as the maximum). Record the character, `world_id` (expect 19) and position. If `world_id` is not 19, `.gotolocation Tollana 211.045 0.606 -547.644` and record it (failure path, not counted).

**S2 - resume probe (ATTEMPT 2 or 3 only).** The coordinator pastes your predecessor's report under `PREDECESSOR`. Skip every row it reports as DONE or NEEDS_HUMAN with a shot path. Start at the first row it did not finish; do S1 first. A first attempt skips S2.

### ROW P8 - trigger interaction (check 8)

**8.1 (1 call: client_batch).** CHATX on `default`: `.bug SPU SP-DTO2-p2 P8 start` plus UTC.
**8.2 (2 calls: client_move_to, client_move_to).** Walk through the cooked-map trigger and the gate stack with real movement keys: `client_move_to {"lease_id":L,"point":{"x":208.2,"y":0.0,"z":-548.0},"waypoints":[{"x":214.0,"y":0.0,"z":-547.7}],"arrival_m":1.0,"timeout_ms":30000}`; then the same tool to `{"x":206.0,"y":-0.3,"z":-545.5}` (a second call). Record `confirmed`, `stop` and positions. Note: the P8 waypoint (214.0, -547.7) has no server trace, so the walk is PARTIAL by construction.
**8.3 (1 call: client_entity_find).** `client_entity_find {"lease_id":L,"max_distance_m":30,"limit":10}`. Record every entity (id, name, position, `rendered`). Expected: Copplemann and the three props are at the seed positions listed in the traced facts (within 0.5 m) and `rendered:true` (the triggers must not move or hide actors). This part has no earlier read, so the comparison is against the seed.

### ROW P10 - template 421 mesh (D-TW7 observation, not a pass/fail gate)

**10.1 (1 call: client_batch).** CHATX on `default`: `.bug SPU SP-DTO2-p2 P10 start`, then `.gotolocation Tollana 201.5 -0.8 -541.5` (4 m from Core), wait 2500 ms, POS step. Then aim the camera at the prop: `client_camera` with `face_entity_id` = the Core id (FIND it first), so the shot shows the prop and not the player's back.
**10.2 (1 call: lab_screenshot).** SHOT, label `DTO2-P10-prop-mesh`, lane `default`. `CLAIM: NEEDS_HUMAN`: say in words what object the prop looks like (a screen? a machine? a plain box?), its colour, and whether it sits on the floor. Do not judge whether it is "better".

**S9 (1 call: lab_lease_release).** Release the lease and write the report. Rows: P8, P10 and the shot label `DTO2-P10-prop-mesh`.
Not in this part: the two-client rows and the server-log row P9 (SP-DTO2-p1); P1 to P5 (SP-DTO1-p1, SP-DTO1-p2); mission 568 (SP-DTO3, SP-DTO4). The live-client phasing checks U-PH1 to U-PH5 are not runnable today.

---

## 2. Report format (your last message, nothing else)

Header: `PACKET SP-DTO2-p2 ATTEMPT <n> | lease released: yes/no | tool calls used: <n> | stopped early: no / <why>`.
Then one block per row in the packet, in this form:
`ROW <id> | CLAIM: DONE / FAIL / BLOCKED / NOT-RUN / NEEDS_HUMAN | UTC <start>-<end> | tier: N1/N2/N3/G (the lowest you used) | raw: <the numbers, tool fields and chat lines, verbatim, in quotes> | shots: <LABEL>=<path>, ...`
"DONE" means you performed the step and saw something; the verifier decides PASS or FAIL. For a look-and-feel step write `NEEDS_HUMAN`, the screenshot path, and in plain words what you saw (for example "Copplemann stands on the floor about 4 m away, facing the camera"). Put every verbatim chat line with its `channel`, `colour`, `tabs` and `seq` when the step asks for them. End with a line `UNEXPECTED: <anything odd, or none>`. Do not interpret, advise or apologise.

## 3. Always end like this

Release the lease: `lab_lease_release {"lease_id":L}`, even after a failure or an early stop. Then write the report.
