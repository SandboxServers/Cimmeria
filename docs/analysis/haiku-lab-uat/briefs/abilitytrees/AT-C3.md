<!-- markdownlint-disable MD029 -- step numbers run on across sections and are cited by number -->

# AT-C3: double-click, relog persistence, walk away and press (AT-06 steps 7, 5, 6)

- Track: ability trees, AT-06. Chunk AT-C3 of 3 (order is 7, 5, 6 on purpose). Instance `p3`. Lease: `<LEASE_ID>` (lent; never acquire/status/release).
- Colo rule 6: your own character only; do not click the granter, reset NPC or pet trainer.
- Preconditions (left by AT-C2): Soldier "Labthree", level 10, bought 612, 1642, 1454, 632 (spend 4), 6 unspent points, node 1452 trainable and not bought, Ability window open (Tab3) at the Debug Area trainer. If R_STATUS shows otherwise, STOP and report.
- Leaves: spend 5 (1452 bought), 5 unspent points, character standing about 27 m south of the trainer.
- Budget: <= 22 tool calls. Every call passes `lease_id: "<LEASE_ID>"`.

## Lua readers

R_STATUS:
`return 'pts='..tostring(getUnitProperty(Unit.Player,Property.TrainingPoints))..' lvl='..tostring(unitLevel(Unit.Player))..' win='..tostring(AbilityWin:isVisible())..' training='..tostring(AbilityMod.TrainingMode)..' known='..table.concat(getAbilityList(),',')`

R_KEY:
`local ids={612,632,851,658,1642,637,1454,523,1452}; local o={} for _,id in ipairs(ids) do local n=getTrainableInfo(id); o[#o+1]=id..':'..((n.id==nil) and 'H' or (n.haveIt and 'K' or (n.trainable and 'T' or 'L'))) end return table.concat(o,' ')..' pts='..tostring(getUnitProperty(Unit.Player,Property.TrainingPoints))`

## Actions

### A0. Check state

1. `client_batch [{"id":"s","op":"lua","chunk":<R_STATUS>},{"id":"k","op":"lua","chunk":<R_KEY>}]`. Expect `lvl=10 pts=6 win=true`, `1452:T`, known contains 612,1642,1454,632.
   If `win=false`: reopen the trainer (`client_entity_find` name "Archetype Skills Trainer" exact, `client_world_click {entity_id, expect:"window", settle_ms:3000}`), then re-read.

### STEP 7. Double-click a purchase: only one point is spent

2. `client_ui_sequence {"actions":[{"do":"click","window":"Ability_Tab3"},{"do":"wait","ms":600}]}` then `client_batch` Lua `local b=_G['Ability_Button5']; return 'btn5 id='..b:getID()..' enabled='..tostring(b:isEnabled())` (expect id 1452, enabled true). If not, find the button whose id is 1452 (scan `Ability_Button1..30`) and use that one below.
3. `lab_screenshot {}`.
4. Double click: `client_window_click {"target":"Ability_Button5","double":true}`.
5. `client_batch [{"op":"wait","ms":2500},{"id":"k","op":"lua","chunk":<R_KEY>},{"id":"s","op":"lua","chunk":<R_STATUS>}]`.
6. `client_chat_log {"cursor":"atc3","max":30}`.

Expected (PASS): `1452:K`, `pts=5` (6 minus exactly 1), known contains 1452 once. Not `pts=4`. Record raw. If the second click of the double click was swallowed because the button went disabled in between, that is also PASS (still 5); the server evidence below decides whether a second request arrived.

### STEP 5. Relog and reopen: learned nodes and points persist

7. Before logging out, record `client_batch [{"id":"pre","op":"lua","chunk":<R_KEY>},{"id":"prestat","op":"lua","chunk":<R_STATUS>}]` (expected as step 5: spend 5, pts=5, known ... 1452).
8. Close the Ability window: `client_batch [{"op":"lua","chunk":"AbilityMod.onCloseClicked() return 'closed'"}]`.
9. `lab_logout {}` (arguments: only `lease_id`). Then `lab_ensure_in_world {"lease_id":"<LEASE_ID>"}`. Record the returned `world_id`, `pos`.
10. `client_batch [{"op":"wait","ms":4000},{"id":"post0","op":"lua","chunk":<R_STATUS>}]` (the Ability window is closed; this reads the persisted known list and points). Expect `pts=5`, known contains 612,1642,1454,632,1452.
11. If the character is not within 5 m of the trainer, go back: type `.gotolocation DebugArea 257 7.2 -928` into `Inst1Chat_Input`, read it back with Lua `return Inst1Chat_Input:getText()`, Enter only if exact; then `client_wait_for {"lua_condition":"getCurrentWorldID() == 1300","timeout_ms":60000}` and wait 4 s. (Close any dialog first.)
12. Reopen the trainer: `client_entity_find {"name":"Archetype Skills Trainer","exact":true,"fields":["id","distance"]}`, then `client_world_click {"entity_id":<id>,"expect":"window","settle_ms":3000}`.
13. `client_batch [{"id":"post","op":"lua","chunk":<R_KEY>},{"id":"poststat","op":"lua","chunk":<R_STATUS>}]` and `lab_screenshot {}`.

Expected (PASS): `post` equals `pre` exactly (same node letters, `pts=5`, `1452:K`, `612:K 632:K 1642:K 1454:K`). If the tree arrays are empty after the relog (R_KEY shows `H` for everything), record it and wait 10 s, re-read once.

### STEP 6. Walk away from the trainer and press a buy: rejected with feedback

14. With the trainer window open (from step 12), walk about 27 m away: `client_move_to {"point":{"x":252.0,"y":7.0,"z":-950.0},"arrival_m":2.0}`. (This is south of the plaza ring, in the gap; open floor.) Record the final position from the result.
15. `client_batch [{"id":"away","op":"lua","chunk":<R_STATUS>},{"id":"btn","op":"lua","chunk":"local a=getTrainableList(2); local r=''; for i=1,#a do if a[i]==851 then local b=_G['Ability_Button'..i]; r='851 at Tab2 button '..i..' enabled='..tostring(b:isEnabled())..' visible='..tostring(b:isVisible()) end end return r"}]`.
    Record whether the window is still open (`win=`) and whether the 851 button is enabled. 851 (Heavy, level 5, needs 612 and spend 2) is buyable at the trainer, so this is a real purchase attempt out of range.
16. Press it:
    - If `win=true`: `client_ui_sequence {"actions":[{"do":"click","window":"Ability_Tab2"},{"do":"wait","ms":600},{"do":"click","window":"Ability_Button3"},{"do":"wait","ms":300}]}`. (Button 3 = the 851 entry on Tab2; verify with the step 15 output: use the button index it printed.)
    - If the window closed or the button is disabled: Lua `buyTrainable(851) return 'sent'` and say so in the report.
17. `client_batch [{"op":"wait","ms":2500},{"id":"after","op":"lua","chunk":"return 'pts='..tostring(getUnitProperty(Unit.Player,Property.TrainingPoints))..' win='..tostring(AbilityWin:isVisible())..' known='..table.concat(getAbilityList(),',')"}]`; `client_chat_log {"cursor":"atc3b","max":30}`; `lab_screenshot {}`.

Expected (PASS): `pts=5` still, 851 not in known, and visible feedback (record exactly what appeared: chat line text, a window refresh, or nothing). Record "feedback seen: yes/no (what)".

18. Walk back and leave the character at the trainer for the next run: `client_move_to {"name":"Archetype Skills Trainer","arrival_m":3.5}`; `client_batch` Lua `AbilityMod.onCloseClicked() return 'closed'`.

## Stop conditions

- Wrong starting state; not in the world after the relog and one retry; a click failing twice (one screenshot each); an unexpected window or dialog (read once with `client_ui_state`, report, stop).
- Never `lab_login` by hand; never restart the client. Max 2 retries per step. Past 95k tokens: stop and report.

## Report format

| step_id | action done | raw observation | PASS/FAIL/BLOCKED claim | screenshot path | UTC time |

Rows: A0, STEP 7, STEP 5 (pre vs post lines side by side), STEP 6 (position after walking, win state, button state, feedback seen).

---

## VERIFY (orchestrator only)

Step 7:

- SigNoz `scope_name = 'abilities' AND ability_id = 1452 AND event IN ('train_requested','granted','train_rejected')`. PASS evidence: exactly one `granted`. A second `train_requested` for 1452 with a `train_rejected` (duplicate, silent by design, may have no reason row) or no second request at all is fine. Two `granted` rows is FAIL.
- DB: `trained_abilities` = `{612,1642,1454,632,1452}` (1452 once), `tree_points_spent` = 5, `training_points` = 5, `abilities` contains 1452 once. (`SELECT abilities, trained_abilities, tree_points_spent, training_points FROM sgw.sgw_player WHERE character_name ILIKE '%labthree%';`)

Step 5:

- Run the same DB query before the logout and after the relog; the rows must be identical. SigNoz: `trainer_open` after the relog; the `InitPlayerState` hydrate (log `init_player_state`) carries level 10.
- `server_sessions` shows one p3 session replaced by a new one.

Step 6:

- SigNoz `scope_name = 'abilities' AND event = 'train_rejected' AND ability_id = 851`; expected reason is the out-of-range / not-at-trainer one (feedback code 43 `OutsideDistanceCheck`). Log `trainer_pin` / interaction messages. Packet tap: `onErrorCode(0, 851, 43)`. If instead the walk-away cleared the trainer pin and the press was silently ignored (no `train_rejected`), that is a FAIL against "rejected with feedback": note it as a finding.
- DB after: unchanged (`tree_points_spent` 5, `training_points` 5, 851 absent).
- If the press reached the server as `train_requested` and got `granted`, that is a critical FAIL (range check missing): capture the tap and log immediately.
