<!-- markdownlint-disable MD029 -- step numbers run on across sections and are cited by number -->

# AT-C1: get in the world, open the trainer, buy a root (AT-06 steps 1-2)

- Track: ability trees, AT-06 (unified-uat "Ability trees")
- Chunk: AT-C1 of 3 (C1 steps 1-2, C2 steps 3-4, C3 steps 7, 5, 6)
- Instance: `p3` (account lab3, character Labthree). Lease: `<LEASE_ID>` (the coordinator lends it; never call lease acquire/status/release).
- Server: the colo (the lab account file decides). Colo rule 6: touch only your own character and the Debug Area NPC you click. Do NOT click "Train Testing Abilities" (granter) or "Jay Test Abilities" (reset) or "Goa'uld Advanced Skills" (pet trainer).
- Preconditions: none. This chunk leaves: Soldier, level 1, standing at the Debug Area trainer, root node 612 (Heavy Weapons root) bought, 0 unspent training points, Ability window closed.
- Budget: <= 20 tool calls. Every call below passes `lease_id: "<LEASE_ID>"` (written as L).

## Facts the driver needs (read once)

- The "trainer window" in this client IS the Ability window (`AbilityWin`). It has three tab buttons `Ability_Tab1`, `Ability_Tab2`, `Ability_Tab3` and up to 30 node buttons `Ability_Button1`..`Ability_Button30`. A node button's id (`:getID()`) is the ability id. Button N on a tab = entry N of `getTrainableList(tab)`.
- Tab order for a Soldier: Tab1 = Automatic Weapons (22 nodes, first id 598), Tab2 = Heavy Weapons (25 nodes, first id 612), Tab3 = Command (25 nodes, first id 1642).
- Node state letters in the reader below: K = known, T = trainable now, L = locked (shown but greyed), H = hidden (not offered).
- A new character has level 1 and exactly 1 training point.

## Lua readers (use these strings exactly)

R_ALL (all tabs):
`local o={} for t=1,getTrainingTreeCount() do local l=getTrainableList(t); local r={} for i=1,#l do local n=getTrainableInfo(l[i]); r[#r+1]=l[i]..':'..((n.id==nil) and 'H' or (n.haveIt and 'K' or (n.trainable and 'T' or 'L'))) end o[#o+1]='TAB'..t..'['..#l..'] '..table.concat(r,' ') end return table.concat(o,'\n')`

R_STATUS:
`return 'pts='..tostring(getUnitProperty(Unit.Player,Property.TrainingPoints))..' lvl='..tostring(unitLevel(Unit.Player))..' win='..tostring(AbilityWin:isVisible())..' training='..tostring(AbilityMod.TrainingMode)..' tabs='..tostring(getTrainingTreeCount())..' known='..table.concat(getAbilityList(),',')`

R_BUTTONS:
`local r={} for i=1,30 do local b=_G['Ability_Button'..i]; if b then r[#r+1]=i..':'..b:getID()..(b:isVisible() and 'v' or 'h')..(b:isEnabled() and 'e' or 'd') end end return table.concat(r,' ')`
(v/h = visible/hidden, e/d = enabled/disabled.)

## Actions

### A0. Get in the world (precondition)

1. `lab_ensure_in_world {"lease_id":"<LEASE_ID>"}`.
   - If it reports the character is missing: `lab_ensure_in_world {"lease_id":"<LEASE_ID>","create":{"alignment":"sgu","archetype":"Soldier","gender":"male","first":"Lab"}}`. (ASSUMPTION: "sgu"/"Soldier"/"male" are the strings the debug-area spec uses; the character name comes from lab-account.p3.json.)
   - Record: `in_world`, `character`, `world_id`.
2. If the character is brand new, the server holds entity introductions for 16 s: `client_batch {"lease_id":"<LEASE_ID>","steps":[{"op":"wait","ms":17000}]}`.
3. Close any dialog/tutorial: `client_batch` with Lua steps `return tostring(DialogWin and DialogWin:isVisible())` and `return tostring(TutorialWin and TutorialWin:isVisible())`. If TutorialWin is visible: `client_window_click {"lease_id":"<LEASE_ID>","target":"TutorialWin__auto_closebutton__"}`. If DialogWin is visible: Lua `DialogMod.onDialogDoneClicked(DialogWin, DialogWin)`. If still blocked after one retry: STOP and report.

### A1. Check the character is fresh (STOP if not)

4. `client_batch` steps: `{"id":"status","op":"lua","chunk":<R_STATUS>}`, `{"id":"list1","op":"lua","chunk":"return tostring(getTrainableList(1)[1])..','..tostring(getTrainableList(2)[1])..','..tostring(getTrainableList(3)[1])"}`.
   - Expect `lvl=1`, `pts=1`, list1 `598,612,1642` (a Soldier). Record raw `known=` list (note whether 598 is in it: ASSUMPTION it is not).
   - If lvl != 1, pts != 1 or list1 differs: STOP, report the raw values (the character is not a fresh Soldier; the coordinator must reset or create another).
   - `getTrainableList` may be empty before the first trainer open (the tree arrives in `onAbilityTreeInfo`, normally at login). If list1 reads `nil,nil,nil`, continue to B1 and re-read after the window opens.

### B0. Go to the Debug Area trainer

5. Close the Ability window if open: `client_batch` Lua `AbilityMod.onCloseClicked()`.
6. Chat command. `client_ui_sequence {"actions":[{"do":"type","text":".gotolocation DebugArea 257 7.2 -928","into":"Inst1Chat_Input"}]}`; then `client_batch` Lua `return Inst1Chat_Input:getText()`; only if it equals the text exactly: `client_ui_sequence {"actions":[{"do":"key","key":"Enter"}]}`.
   - A non-GM account ignores it (no world change): then step 7 times out; STOP and report BLOCKED (account is not GM).
7. `client_wait_for {"lua_condition":"getCurrentWorldID() == 1300","timeout_ms":60000}`, then `client_batch [{"op":"wait","ms":4000}]`.
8. `client_entity_find {"name":"Archetype Skills Trainer","exact":true,"fields":["entities","id","name","distance","position","rendered"]}`. Expect one match, distance about 3.4 m (the NPC is at server x 259.42, z -930.42). Record id, distance. If missing: wait 5 s, retry once; if still missing: STOP (the Debug Area is not seeded on this server).

### STEP 1. Open the trainer: three tabs, locked nodes visible and greyed

9. `lab_screenshot {}` (path only; before the click). Do not touch the camera unless the click in step 10 fails (then `client_camera {"face_entity_id":<id from 8>,"pitch_deg":-28.8,"yaw_offset_deg":25.3,"zoom":250}`).
10. `client_world_click {"entity_id":<id from 8>,"expect":"window","settle_ms":3000}`.
    - On failure: one `lab_screenshot`, then retry once. If the hover answers other_entity 2, move: `client_move_to {"entity_id":<id>,"arrival_m":3.5}` then retry.
11. `client_batch` steps (one call): `{"id":"status","op":"lua","chunk":<R_STATUS>}`, `{"id":"all","op":"lua","chunk":<R_ALL>}`, `{"id":"btn","op":"lua","chunk":<R_BUTTONS>}`.
12. `lab_screenshot {}` and report the path.

Expected (PASS): `win=true training=true tabs=3`. R_ALL shows `TAB1[22]`, `TAB2[25]`, `TAB3[25]`, in that order, with first ids 598, 612, 1642. Roots `598:T 612:T 1642:T` (pts=1, level 1). Tier-1 nodes are locked, not hidden, for example `717:L 715:L` in TAB1, `632:L 851:L` in TAB2, `637:L 1454:L 523:L` in TAB3, and no `H` entries (H would mean "hidden, not greyed" = FAIL). R_BUTTONS: the buttons for locked nodes read `v` and `d` (visible, disabled); roots read `v` and `e`.
Record raw: the full R_ALL text, R_STATUS, R_BUTTONS. The screenshot is for the human check "greyed".

Note for 598: if R_ALL shows `598:K` the character already owns the Soldier signature ability; record it and continue (the coordinator will adapt step 4 of C2).

### STEP 2. Buy the root: counter drops at once, ability appears in known list

13. Switch to Tab2 and buy the Heavy Weapons root 612, which is Button1 on Tab2:
    `client_ui_sequence {"actions":[{"do":"click","window":"Ability_Tab2"},{"do":"wait","ms":600},{"do":"click","window":"Ability_Button1"},{"do":"wait","ms":300}]}`
14. Immediately (within ~1 s) `client_batch` steps: `{"id":"quick","op":"lua","chunk":"return tostring(getUnitProperty(Unit.Player,Property.TrainingPoints))"}`, `{"op":"wait","ms":2000}`, `{"id":"status","op":"lua","chunk":<R_STATUS>}`, `{"id":"st","op":"lua","chunk":"local ids={612,632,851,1642,598}; local o={} for _,id in ipairs(ids) do local n=getTrainableInfo(id); o[#o+1]=id..':'..((n.id==nil) and 'H' or (n.haveIt and 'K' or (n.trainable and 'T' or 'L'))) end return table.concat(o,' ')"}`.
15. `client_chat_log {"cursor":"atc1","max":20}` (record any lines the purchase produced).
16. `lab_screenshot {}`.

Expected (PASS): `quick` (and the later `pts=`) is `0` (it drops from 1 to 0 without a relog). `known=` now contains 612. 612 reads `K`. 632 and 851 read `L` (their spend gate needs 2 points spent; only 1 is). 1642 reads `L` (no points left). Record whatever each reads.
Record raw: quick, status, st, chat lines.

## Stop conditions

- Not in the world after `lab_ensure_in_world` and one retry; character not a fresh Soldier; not GM; Debug Area trainer missing; an unexpected window (read it with one `client_ui_state`, report, stop).
- Max 2 retries per step, then record the error text verbatim and stop. Past 95k tokens: stop and report.

## Report format

| step_id | action done | raw observation | PASS/FAIL/BLOCKED claim | screenshot path | UTC time |

Rows: A0, A1, B0, STEP 1, STEP 2. Include the full R_ALL text in the STEP 1 row (or "see below" and paste it after the table).

---

## VERIFY (orchestrator only, not for the driver)

Step 1:

- `server_log_tail` search `trainer_open` / `interaction` for template 301; SigNoz `scope_name = 'abilities' AND event = 'trainer_open' AND trainer_template_id = 301` (min 1 row, player_id = the p3 character).
- `server_packet_tap_start` on the p3 session before step 10 (opt.) and look for `onTrainerOpen` (client method 113) and `onAbilityTreeInfo` (141) with 3 arrays.
- `server_db_query`: `SELECT archetype, ability_index, ability_id, tree_index, level, prerequisite_abilities, required_branch_points FROM resources.archetype_ability_tree WHERE archetype='ARCHETYPE_Soldier' ORDER BY tree_index, ability_index;` to cross-check the R_ALL order (72 rows: 22/25/25). `SELECT count(*) FROM resources.trainer_abilities WHERE list_id=1 AND archetype='ARCHETYPE_Soldier';` should be 72 (template 301 uses list 1).

Step 2:

- SigNoz `scope_name = 'abilities' AND event IN ('train_requested','granted')` with ability_id 612; no `train_rejected`.
- `SELECT abilities, trained_abilities, training_points, tree_points_spent, level, exp FROM sgw.sgw_player WHERE character_name ILIKE '%labthree%';` (ASSUMPTION column names; find the row via `server_sessions` if the name filter misses). Expect trained_abilities `{612}`, tree_points_spent 1, training_points 0, abilities includes 612 once.
- packet tap: `onKnownAbilitiesUpdate` (101) then `onEntityProperty(TrainingPoints=0)`, then `onTrainerOpen` re-send in that order (AT-03 burst order).
- Check the log for `train_raw_cost_zero` WARN (should not fire for the seeded 612; if it does, note it).
