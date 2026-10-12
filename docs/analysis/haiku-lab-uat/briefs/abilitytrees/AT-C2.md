<!-- markdownlint-disable MD029 -- step numbers run on across sections and are cited by number -->

# AT-C2: level to 10, a refused purchase, the archetype-wide spend gate (AT-06 steps 3-4)

- Track: ability trees, AT-06. Chunk AT-C2 of 3. Instance `p3`. Lease: `<LEASE_ID>` (lent; never acquire/status/release).
- Colo rule 6: your own character only. Do not click the granter, the reset NPC or the pet trainer.
- Preconditions (left by AT-C1): fresh Soldier "Labthree", level 1, in world 1300 within 3.5 m of "Archetype Skills Trainer", node 612 bought (known), 0 unspent points, Ability window closed. If a precondition is false (read R_STATUS first), STOP and report.
- Leaves: level 10, node spend 4 (612, 1642, 1454, 632 bought), 6 unspent points, node 1452 trainable but NOT bought, Ability window open.
- Budget: <= 22 tool calls. Every call passes `lease_id: "<LEASE_ID>"`.

## Facts

- The trainer window is `AbilityWin` (tabs `Ability_Tab1..3`, buttons `Ability_Button1..30`; button N on a tab = entry N of `getTrainableList(tab)`; its id is the ability id).
- Soldier tabs: Tab1 Automatic Weapons, Tab2 Heavy Weapons, Tab3 Command. Seed data used here:
  - Tab2 (Heavy): entry 1 = 612 root, entry 2 = 632 (level 5, needs spend 2, needs 612), entry 3 = 851 (level 5, spend 2, needs 612), entry 4 = 658 (level 10, spend 4, needs 632).
  - Tab3 (Command): entry 1 = 1642 root, entry 2 = 637, entry 3 = 1454 (level 5, spend 2, needs 1642), entry 4 = 523, entry 5 = 1452 (level 10, spend 4, needs 1454).
- "Spend" = training points spent at the trainer across all three tabs together (roots count). Gates in order: level, prerequisite, spend, points.
- XP: a character reaches level 5 with 601 total XP and level 10 with 6001 total XP; each level gives 1 point (level 10 = 10 points in total).
- Node letters: K known, T trainable, L locked (visible, greyed), H hidden.

## Lua readers

R_STATUS:
`return 'pts='..tostring(getUnitProperty(Unit.Player,Property.TrainingPoints))..' lvl='..tostring(unitLevel(Unit.Player))..' win='..tostring(AbilityWin:isVisible())..' training='..tostring(AbilityMod.TrainingMode)..' known='..table.concat(getAbilityList(),',')`

R_KEY (state of the nodes this chunk cares about):
`local ids={612,632,851,658,1642,637,1454,523,1452}; local o={} for _,id in ipairs(ids) do local n=getTrainableInfo(id); o[#o+1]=id..':'..((n.id==nil) and 'H' or (n.haveIt and 'K' or (n.trainable and 'T' or 'L'))) end return table.concat(o,' ')..' pts='..tostring(getUnitProperty(Unit.Player,Property.TrainingPoints))`

R_MAP (verify button positions before clicking):
`local a=getTrainableList(2); local b=getTrainableList(3); return 'T2e1='..tostring(a[1])..' T2e2='..tostring(a[2])..' T2e4='..tostring(a[4])..' T3e1='..tostring(b[1])..' T3e3='..tostring(b[3])..' T3e5='..tostring(b[5])`
Expect `T2e1=612 T2e2=632 T2e4=658 T3e1=1642 T3e3=1454 T3e5=1452`. If it differs, use the position the actual list gives for the id you want; record the difference.

## Actions

### A0. Check state and level up

1. `client_batch [{"id":"s","op":"lua","chunk":<R_STATUS>},{"id":"m","op":"lua","chunk":<R_MAP>}]`. Expect `lvl=1 pts=0`, known contains 612. (If `lvl` is already 10 from an earlier try: skip step 2-3 and go on.)
2. Give XP: type `/gmgivexp 6001`. `client_ui_sequence {"actions":[{"do":"type","text":"/gmgivexp 6001","into":"Inst1Chat_Input"}]}`; `client_batch [{"op":"lua","chunk":"return Inst1Chat_Input:getText()"}]`; only if it matches exactly: `client_ui_sequence {"actions":[{"do":"key","key":"Enter"}]}`. (If the Ability window is open, close it first with Lua `AbilityMod.onCloseClicked()`.)
3. `client_wait_for {"lua_condition":"unitLevel(Unit.Player) == 10","timeout_ms":15000}`, then `client_batch [{"op":"wait","ms":1500},{"op":"lua","chunk":<R_STATUS>}]`. Expect `lvl=10 pts=9`. (If the level stays 1: BLOCKED, the account is not GM; stop.)
4. Open the trainer fresh: `client_entity_find {"name":"Archetype Skills Trainer","exact":true,"fields":["id","distance"]}`; if distance > 4.5 m: `client_move_to {"entity_id":<id>,"arrival_m":3.5}`. Then `client_world_click {"entity_id":<id>,"expect":"window","settle_ms":3000}`. On failure one `lab_screenshot`, one retry.
5. `client_batch [{"id":"k0","op":"lua","chunk":<R_KEY>}]`. Expect `612:K`, `632:L` (spend is 1, needs 2), `851:L`, `658:L`, `1642:T`, `1454:L`, `1452:L`, `pts=9`.

### STEP 3. A press on a node whose prerequisite you lack: error, nothing spent

Node 658 (Heavy, level 10, needs 632 which you do not know) is locked, so its button is disabled and cannot be clicked. The stale-window press is made by calling the same native the button calls.

6. `client_batch` steps: `{"id":"before","op":"lua","chunk":<R_KEY>}`, `{"id":"press","op":"lua","chunk":"buyTrainable(658) return 'sent'"}`, `{"op":"wait","ms":2500}`, `{"id":"after","op":"lua","chunk":<R_KEY>}`, `{"id":"win","op":"lua","chunk":"return 'win='..tostring(AbilityWin:isVisible())"}`.
7. `client_chat_log {"cursor":"atc2","max":30}` (every line since the last read; record all, including any with the words "ability", "cannot", "prerequisite").
8. `lab_screenshot {}`.

Expected (PASS): `after` equals `before` (pts=9 both, 658 still `L`, 632 not known). The server answered with an error (`onErrorCode` 167) and re-sent the trainer window: the window is still open. Record whether ANY text showed up in chat or on screen (the open question is whether the error shows text, or only a window refresh). Record both facts: "text seen: yes/no (what)", "window refreshed: yes/no".

### STEP 4. Spend gate across branches

Goal: buy roots and tier-1 nodes and watch node 1452 (Command tab, needs spend 4 and node 1454) flip from L to T when the Heavy node 632 is bought.

Each purchase = one `client_ui_sequence` click on the right tab and button, then a read. Do them in this order. After EACH purchase run `client_batch [{"op":"wait","ms":1500},{"id":"k","op":"lua","chunk":<R_KEY>}]` (you may combine the click and read into the same turn as two parallel calls only if the read waits 1500 ms first).

9. Buy Command root 1642: `client_ui_sequence {"actions":[{"do":"click","window":"Ability_Tab3"},{"do":"wait","ms":600},{"do":"click","window":"Ability_Button1"},{"do":"wait","ms":300}]}`. Read. Expect spend 2: `1642:K`, `1454:T` (needs 1642, spend 2 met), `632:T`, `851:T`, `1452:L`, `pts=8`.
10. Buy 1454 (Command tier 1): `client_ui_sequence {"actions":[{"do":"click","window":"Ability_Button3"},{"do":"wait","ms":300}]}` (still on Tab3). Read. Expect spend 3: `1454:K`, `1452:L` (needs spend 4, only 3), `pts=7`.
11. `lab_screenshot {}`, and record the R_KEY line: this is the "locked at spend 3" evidence for 1452.
12. Buy 632 in ANOTHER branch (Heavy, Tab2 entry 2): `client_ui_sequence {"actions":[{"do":"click","window":"Ability_Tab2"},{"do":"wait","ms":600},{"do":"click","window":"Ability_Button2"},{"do":"wait","ms":300}]}`. Read. Expect spend 4: `632:K`, `pts=6`, and `1452:T` (the Command tab node opened because archetype-wide spend reached 4 and its own prerequisite 1454 is known).
13. Switch to Tab3 to look at it: `client_ui_sequence {"actions":[{"do":"click","window":"Ability_Tab3"},{"do":"wait","ms":600}]}`; `client_batch` Lua `local b=_G['Ability_Button5']; return 'btn5 id='..b:getID()..' enabled='..tostring(b:isEnabled())` (expect id 1452, enabled true); `lab_screenshot {}`.
14. DO NOT buy 1452 here (AT-C3 does it).

Expected summary (PASS): after step 10 node 1452 is `L`; after step 12 it is `T`. If it flips earlier (for example already `T` after step 10), record exactly after which purchase it flipped: that means the character holds free branch credit (a class signature, e.g. 598 in the known list). Do not retry; report it.

## Stop conditions

- Not GM, wrong state, unexpected window/dialog (read once with `client_ui_state`, report, stop), a click that fails twice (one screenshot each), or a purchase whose result is not what R_KEY shows (record raw, do not repeat the purchase more than once).
- Max 2 retries per step. Past 95k tokens: stop and report.

## Report format

| step_id | action done | raw observation | PASS/FAIL/BLOCKED claim | screenshot path | UTC time |

Rows: A0 (level), STEP 3, STEP 4 (one row per purchase: 9, 10, 12, with the R_KEY line each time). Paste the full chat_log lines for step 7 under the table.

---

## VERIFY (orchestrator only)

Step 3:

- SigNoz `scope_name = 'abilities' AND event = 'train_rejected' AND ability_id = 658`, read `reason` (expected a missing-prerequisite reason; ASSUMPTION the string is something like `missing_prerequisite`). Also `train_requested` for 658 just before it. No `granted` for 658.
- Packet tap: `onErrorCode(system 0, instance 658, code 167)` then an `onTrainerOpen` re-send. This answers "does the client show text" only on the human side; the tap proves the server sent both.
- DB: `trained_abilities` unchanged `{612}`, `tree_points_spent` 1, `training_points` 9 right after the press.

Step 4:

- SigNoz `event IN ('train_requested','granted')` for ability ids 1642, 1454, 632 in that order; no `train_rejected` between them.
- DB: `trained_abilities` = `{612,1642,1454,632}`, `tree_points_spent` = 4, `training_points` = 6, `level` = 10, `exp` = 6001. The three buys each logged a points property `onEntityProperty(TrainingPoints)` in the tap (9 -> 8 -> 7 -> 6 after the first level-up burst).
- Cross-check the gate from the seed: `SELECT ability_id, level, prerequisite_abilities, required_branch_points FROM resources.archetype_ability_tree WHERE archetype='ARCHETYPE_Soldier' AND ability_id IN (1452,1454,632,658);` (1452: 10 / {1454} / 4; 1454: 5 / {1642} / 2; 632: 5 / {612} / 2; 658: 10 / {632} / 4).
- `server_log_tail` search `level` / `GrantXP` to confirm the XP grant (6001) and levels 2..10 each pushed `ProgressionChanged` (the cell level feeds the gate; if 1452 never turned `T`, check this first).
