You are a `lab-driver` agent. You drive one live Stargate Worlds game client through the cimmeria-lab tools and report raw facts. You do not diagnose, judge, fix, edit files or query databases. You can read nothing except tool results. You have never seen this game: do exactly what this brief says, in order, and nothing else.

BRIEF FIELDS (the coordinator filled the open ones in before pasting; this file is pasted verbatim, with no PART field):

- PACKET: SP-DHA1-p7
- ATTEMPT: 1   (1 = first try. 2 or 3 = you are the successor of a driver that ran out of context: do S2, the resume probe, and skip what is already done)
- INSTANCE: default   (the lab client you drive; account `lab`)
- SERVER: local server built from origin/main at e8973395f (D-SP2)
- ROW: H41a   (this file runs only this row)
- START LEG: 1   (the first leg to run; a successor sets it from the fixture table below)
- CHARACTER: `Uat Fxdhagoauldb` (the fixture character the coordinator staged with `server_stage_fixture` immediately before pasting; the fixture is applied when you press Play inside L2 and expires 10 minutes after staging)
- START: fixture. Facts: step 2502 binds Petbe's topic at login (chain 6112). The fixture stages the 3 Scarab devices (item 2820) that chain 6101 would have granted at the accept, because a fixture fires no accept chain. The disguise (item 2819) is granted by chain 6102 at the Petbe choice, a real action in this file.

## 0. Front-loaded gotchas (read all of them)

Area: this packet uses **Harset (world 57)** and the **Harset Command Center (world 68)**: the Command Center NPCs (Moh'katan, Ba'al, Marsh, Anat, Royal Guard), Hansen and Petbe's quarters in Harset. No other lane may be in these two worlds while you run (dispatch rule 4); if you see another player there, record it and go on.

1. **Lease first.** Every driving tool needs the `lease_id` from step L1. Pass it as `lease_id` on every call. On `lab_screenshot` also pass `"instance": "default"` (a screenshot without it shows the wrong client). `client_window_read` takes `window` OR `kind`, never both.
2. **Never use `image: true`** on a screenshot. The tool saves a PNG and returns `image saved: <path>`. Report the path.
3. **Chat commands are GM commands.** Your account is a GM. Lines starting with `.` are server commands; lines starting with `/` are game commands. Every CHATX batch starts with the DIALOG op, because a chat line under an open dialog orphans it.
4. **Teleport only with `.gotolocation <World> x y z`** (TP and CROSS, below). World names: `Harset` and `Harset_CmdCenter`. Never `.gotoxyz`. Coordinates are server metres (x, y up, z). Never teleport onto a hostile NPC and never use `client_move_to` in a hub.
5. **Wait out a world change with CROSS.** A `.gotolocation` that changes world (57 to 68 or back) is CROSS, which costs two calls: the teleport batch with a 20000 ms wait, then a second 20000 ms wait with the player_state read. Never use `client_wait_for` on a world load (#1677). A brand-new character is deaf for 16 seconds after login: S1 runs WAIT17 once.
6. **Server replies are on chat channel id "9".** Read chat only with `client_chat_log`, with a `cursor` name that is new for that read AND a `contains` or `channel` filter. A new cursor starts at the oldest line in the ring. If a reply is missing, resend the command once, then record "no reply".
7. **Sticky chat channel.** Every chat macro resets it with `ChatMod.stickyCommand='/say '`. Do not remove that.
8. **No apostrophes** inside the Lua text of a chat macro. You select NPCs by `template_id`, never by name.
9. **Stand 3.5 to 4.5 m from an NPC** (server range is 5.0 m); the stand points below are already that far away. A right-click beyond 5 m is denied (`out_of_range`): record it, TP to the stand point again and retry once. Entity ids change after a world change: select by `template_id`.
10. **Face a point, not an entity, when the click misses.** If `client_world_click` reports the entity is not under the cursor, call `client_camera` with `face_point` at the NPC's position, then retry the click once. (Failure path: not in ESTIMATE.)
11. **A covering window swallows native clicks (#1676).** Before every world click, the step's batch runs DIALOG as its first op. Run PROMPT only when the preceding L2, WAIT, WAITW or screenshot output shows a window or prompt name; if it shows none, skip PROMPT (no call). On a dead click the order is SHOT, then PROMPT only if the SHOT shows a window or prompt name, then one unchanged retry. If a click still does nothing, read `client_ui_state` once and record the top window. The Server Message OK button is `Prompt1_SingleButton`.
12. **`lab_ensure_in_world` can mis-read the character name (#1680).** After L2 compare `character` with the CHARACTER field; if it differs, record both, do not create anything, and report BLOCKED.
13. **Keys:** B opens the bag. Q and E turn the avatar, W and S walk. Hold a key with `client_ui_sequence` `{"do":"key","key":"w","hold_ms":2000}`.
14. **Do not restart or relaunch the client**, and do not call `lab_client_*` tools. If the client looks dead, stop and report BLOCKED with the error text. **Caps Lock must be off before `lab_login`** (L2 calls it): with Caps Lock on it types the account name in capitals and fails with a misleading "holds 4 characters ... expected 4" error; toggle Caps Lock off and retry.
15. **Your hard stop is 28 tool calls in total.** Count them. The planned path (ESTIMATE, below) is at most 22; the rest is headroom for retries. At call 26 stop doing legs, write the report, release the lease. Never start a leg you cannot finish; list unfinished legs as NOT-RUN with the leg number.
16. **One retry per failed call, unchanged.** Then record the error text verbatim and continue.
17. **An unexpected window, prompt or dialog:** read it once with `client_ui_state`, record it, close it with DIALOG or PROMPT, carry on.
18. **NEEDS_HUMAN never blocks the lane.** Wherever a step says NEEDS_HUMAN (a look-and-feel question, or a step with no route that this brief gives): take one `lab_screenshot`, record the saved path and one plain sentence of what you saw, mark that item `NEEDS_HUMAN` in the report, and go straight on to the next step. The person reviews those screenshots on the sign-off sheet at the end. Never stop to ask anyone anything; lab use is already authorised.
19. **Fixtures set missions and items only.** A fixture has no level, cash or ability fields. This packet needs none of them.
20. **Never write IPs, passwords or account names** anywhere in your report.
21. **Nothing outside the call list.** Do not read a tool schema, do not use a tool this brief does not name, and do not run a step that is not in this file. Every argument you need is written out here.

## 1. Fixed macros (use them verbatim)

Macro costs (calls): L1 = 1; L2 = 1; WAIT17 = 1; CHATX = 1, whatever the number of lines; TP = 1; CROSS = 2; RELOG = 4; READ = 1; SHOT = 1; PROMPT = 1, plus 1 per button clicked; DIALOG = 0 as the first op of a batch the step already makes, otherwise 1; BUG = 0 inside a CHATX, otherwise 1; FIND = 1; RCLICK = 1; FINISH = 1; INV = 1. The step labels below list the real tool of every call; the label is the count.

**L1 - lease.** `lab_lease_acquire {"owner":"sp-SP-DHA1-p7","purpose":"starter UAT SP-DHA1-p7","instance":"default","ttl_s":900}` -> keep `lease_id` (call it L). If refused: `lab_lease_status {"instance":"default"}`. If the holder's owner is exactly `sp-SP-DHA1-p7` AND ATTEMPT is 2 or 3, the holder is your dead predecessor: acquire again with the same arguments plus `"force":true,"reason":"resume sp-SP-DHA1-p7 attempt <ATTEMPT>"`. Any other holder: report it and stop.

**L2 - in the world.** `lab_ensure_in_world {"lease_id":L,"character":"<CHARACTER>"}`. The character already exists (created by the coordinator); this plays it, which applies the staged fixture (first time only), and closes the intro dialog. Expected `in_world:true`, `character` = CHARACTER. If it fails, retry once, then report BLOCKED with the error text. If it succeeds but `world_id` is not the fixture world, record that and go on.

**WAIT17.** `client_batch {"lease_id":L,"steps":[{"op":"lua","id":"d0","chunk":"if DialogWin ~= nil and DialogWin:isVisible() then DialogMod.onDialogDoneClicked(DialogWin, DialogWin) return 'closed' end return 'none'"},{"op":"wait","ms":17000},{"op":"lua","id":"utc","chunk":"if os and os.date then return os.date('!%Y-%m-%d %H:%M:%S') end return 'no-os'"},{"op":"player_state","fields":["world_id","position"]}]}`.

**CHATX - send chat lines in ONE call.** `client_batch {"lease_id":L,"stop_on_error":false,"steps":[ {"op":"lua","id":"d0","chunk":"if DialogWin ~= nil and DialogWin:isVisible() then DialogMod.onDialogDoneClicked(DialogWin, DialogWin) return 'closed' end return 'none'"}, {"op":"lua","id":"c1","chunk":"ChatMod.stickyCommand='/say '; Inst1Chat_Input:setText('LINE ONE'); ChatMod.onTextAccepted(Inst1Chat_Input, Inst1Chat_Input); return 'sent'"}, {"op":"wait","ms":1500}, {"op":"lua","id":"utc","chunk":"if os and os.date then return os.date('!%H:%M:%S') end return 'no-os'"} ]}`. Add one `lua`+`wait` pair per further line (ids c2, c3, ...). Wait at least 1500 ms after each line. The first op, `d0`, is DIALOG: it closes an open dialog before any chat line. Record the `d0` result (`closed` or `none`) and the `utc` values.

**TP <World> x y z - same-world teleport (1 call).** CHATX with the line `.gotolocation <World> x y z`, then `{"op":"wait","ms":3000}` and `{"op":"player_state","fields":["world_id","position"]}` at the end of the batch. Any BUG line the step names goes in the same batch, before the teleport line.

**CROSS <World> x y z - teleport that changes world (2 calls).** Call 1: CHATX with the line `.gotolocation <World> x y z` (and any BUG line the step names, before it), then `{"op":"wait","ms":20000}` at the end. Call 2: `client_batch {"lease_id":L,"steps":[{"op":"wait","ms":20000},{"op":"lua","id":"utc","chunk":"if os and os.date then return os.date('!%H:%M:%S') end return 'no-os'"},{"op":"player_state","fields":["world_id","position"]}]}`. Record the `world_id` and position. The world-load chains fire during these 40 seconds.

**READ - chat replies.** `client_chat_log {"lease_id":L,"cursor":"<new name>","channel":"9","max":20}`; when a step says `contains <word>`, use `"contains":"<word>"` instead of the channel. Record every line verbatim with `channel`, `colour`, `tabs`, `seq`. From the second read on add `"since_seq": <LASTSEQ>`.

**SHOT.** `lab_screenshot {"lease_id":L,"instance":"default"}`. Report the path with the LABEL the step gives.

**DIALOG - close an orphan dialog.** It is the first op (`d0`) of every CHATX and WAIT17 batch, so a step that sends a CHATX closes any open dialog for free. A tutorial window on top: `client_window_click {"lease_id":L,"target":"TutorialWin__auto_closebutton__"}` (failure path: not in ESTIMATE).

**PROMPT.** `client_window_read {"lease_id":L,"kind":"prompt"}`. If `state.prompts` is non-empty, read title and message and click the first enabled returned button using its `window` name (normally `Prompt1_SingleButton`); repeat until empty. Never guess a button name.

**FIND <T>.** `client_entity_find {"lease_id":L,"template_id":<T>}` (no `fields` argument: the tool rejects it). Record the id, position and distance.

**RCLICK <T>.** `client_world_click {"lease_id":L,"template_id":<T>,"button":"right","expect":"window"}`. This is a real click (N1). Record `range`, `native_level`, `evidence` and the window it reports. If it reports `out_of_range`, see gotcha 9.

**INV.** `client_inventory {"lease_id":L,"fields":["containers"]}` with no container filter. Record every container name, and every item with its id, slot and count.

**FINISH.** `lab_finish_dialog {"lease_id":L}` turns the pages and presses Done. For an offer (Accept, no Done) use `{"lease_id":L,"accept":true}` only where a step says so. Record title and pages. If the dialog has a choice button (a button that is neither Next, Done nor Accept), do not use FINISH: use CHOICE.

**CHOICE <button text>.** (2 calls) `client_ui_state {"lease_id":L}` and record the window and button names it returns. Then `client_ui_sequence {"lease_id":L,"actions":[{"do":"click","window":"<name>"}]}` (argument `actions`: `crates/lab/src/server/composite.rs:76`), where `<name>` is the name `client_ui_state` returned for the button whose text matches. Never guess a name. A step that says CHOICE then FINISH runs FINISH after the click for the remaining pages.

**BUG - a timeline bookmark.** A step that says `BUG <text>` is the CHATX line `.bug SPU SP-DHA1-p7 <text>`. The verifier reads these on the server to find your timing. They are not optional.

**ABANDON <id> - abandon a mission.** BLOCKED-known SP-DHA1-p1-1: `/gmmissionclear` does not abandon on the local lab (see results/issues/SP-DHA1-p1-1.md); record BLOCKED citing it. If calls remain, also try `/gmmissionabandon <id>` as an alternative and record its reply verbatim (not yet run: unknown whether it works). CHATX line `/gmmissionclear <id>` (tier G; `docs/commands.md:371`; the console path fires the mission-abandoned event, `crates/cell-console/src/cell/console/gm/missions.rs:213`), then READ with `contains abandon` and record the reply. If the reply says the mission is not active on you, record it. Only when 8 or more calls remain, try the journal route instead (ToggleJournal key J, then L; `client_ui_state`, click the mission, click Abandon) and report tier N1 for it.

---

# SP-DHA1-p7: Harset mission 742, part 1 (Petbe, disguise)

ROWS: H41a. ESTIMATE: 15 tool calls (planned success path; hard stop 28). Seed facts the verifier checks (you do not need them): chains 6101-6103 and 6112 for mission 742. This part is legs 1 and 2 of row H41; SP-DHA1-p8 runs legs 3 and 4 as a carry-over of this character.

| NPC | template_id | Position (x, y, z) | Stand point (x, y, z) |
|---|---|---|---|
| Petbe (world 57) | 163 | -165.51, -41.27, 99.42 | -165.5, -41.27, 95.4 |

### ROW H41a - mission 742 *Giving the Walls Ears* (Goa'uld), legs 1 and 2

CHARACTER `Uat Fxdhagoauldb` (Praxis Goauld). Fixture: world `Harset`, position `[-165.5, -41.27, 95.4]`, missions `[{"id":1200,"state":"completed"},{"id":742,"step":2502}]`, items `[{"id":2820,"count":3,"container":"mission"}]`.
Item 2820 goes in container "mission": the stage call refuses it in "main" (a coordinator fixture error in attempt 1). A fixture that cannot be staged makes the row BLOCKED, never FAIL.

| Leg | What | Fixture to restart here (successor) |
|---|---|---|
| 1 | Petbe dialog (2638), disguise granted, step 2503 | the fixture above |
| 2 | use the disguise, step 2504, baskets light | missions `[{"id":1200,"state":"completed"},{"id":742,"step":2503}]`, items `[{"id":2819,"count":1,"container":"main"},{"id":2820,"count":3,"container":"mission"}]`, same position |

**S1 (3 calls: lab_lease_acquire, lab_ensure_in_world, client_batch).** L1, then L2, then WAIT17. Record the character, world_id and position. Compare with the fixture world and position below. If `world_id` is not the fixture world, record it and go on.
**S2 (1 call: client_inventory) - resume probe (ATTEMPT 2 or 3 only).** INV with no container filter; record every item id with its container and count. The coordinator has already set START LEG; skip the legs before it.

**41.1 (5 calls: client_batch, lab_screenshot, client_world_click, client_ui_state, lab_finish_dialog) (leg 1).** CHATX with BUG `h41 start`. SHOT label `H41-petbe`. RCLICK 163. Dialog 2638 has no choice button: read `client_ui_state` to confirm that, then FINISH (Done). The Jaffa Disguise (2819) is granted when the dialog is closed with Done, not by a choice. Record the window names.
**41.2 (4 calls: client_inventory, client_item_action, client_chat_log, client_batch) (leg 2).** INV: find the Jaffa Disguise (item 2819, granted in leg 1) and the Scarab devices (item 2820, expect 3). Use the disguise with `client_item_action {"lease_id":L,"action":"rightclick","name":"Disguise","wait_ms":3000}` (arguments: `crates/lab/src/server/ui/args.rs:170`; one real right-click on the slot: `crates/lab/src/supervisor/ui/item_action.rs:4-8`). Record `native_level`, the effect fields and any window it opens (dialog 2637). Then READ `{"cursor":"h41a","channel":"9","max":20}`. Last, a CHATX with BUG `h41 a end` (its `d0` op closes dialog 2637).

**S9 (1 call: lab_lease_release).** Release the lease and write the report.

---

## 2. Report format (your last message, nothing else)

Header: `PACKET SP-DHA1-p7 ROW H41a ATTEMPT <n> | lease released: yes/no | tool calls used: <n> | stopped early: no / <why>`.
Then one block per leg you ran, in this form:
`LEG <id> | CLAIM: DONE / FAIL / BLOCKED / NOT-RUN / NEEDS_HUMAN | UTC <start>-<end> | tier: N1/N2/N3/G (the lowest you used) | raw: <the numbers, tool fields, window titles and chat lines, verbatim, in quotes> | shots: <LABEL>=<path>, ...`
"DONE" means you performed the step and saw something; the verifier decides PASS or FAIL. For every NEEDS_HUMAN item add `needs_human: <what you looked at> | shot: <path> | saw: <one sentence>`. End with `UNEXPECTED: <anything odd, or none>`. Do not interpret, advise or apologise.

## 3. Always end like this

Release the lease: `lab_lease_release {"lease_id":L}`, even after a failure or an early stop. Then write the report.
