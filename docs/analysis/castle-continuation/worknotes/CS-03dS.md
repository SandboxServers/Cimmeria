# CS-03dS worknote: mission 1521 implant item seal (Option A)

Base `2fad73404`. Owner decision OD-1 (2026-10-18): Option A, the implant is an item. Builds on [CS-03S](CS-03S.md) (Option B skeleton, chains 1369-1377 stay byte-identical) and [g-sym.md](../evidence/g-sym.md). Packet: CS-03d (one packet, c1 list only). No DB, no lab, no compile used.

## Verdict

`SEALED` for the item grant (chain 1386). `UATPending (end UAT)`: inventory tooltip, mailed-item placement, native-use silence. G-SYM-GAME, G-SYM-CONSUME, G-SYM-DEBUFF and G-SYM-RADIO each have a ruling below; none blocks the grant.

## 1386 IDs

Subset of the D-CA20 1521 A row (`138[6-9]|139[0-3]`):

| Id | Use |
|---|---|
| chain 1386 | `mission_completed 1521` -> `add_item 5169` (this seal) |
| chains 1387-1393 | reserved, unused |

No mission, step, objective, task, dialog, set, template or spawn ids are used.

## Collision check

Command (the §5 form, restricted to the five refs; the first run was the unfiltered §5 command):

```powershell
foreach ($r in '2fad73404','content/cellblock-campaign','content/harset-campaign','content/tollana-campaign','origin/main') { "== $r"; git grep -n -E "VALUES \((138[6-9]|139[0-3])," $r -- db/resources/Missions db/resources/Dialogs db/resources/Content }
```

REPO result, identical on all five refs: the unfiltered command returns only rows in other tables that happen to use the same numbers as their own primary keys (Dialogs: `dialog_screen_buttons` 8, `dialog_screens` 8, `dialog_set_maps` 6, `dialog_sets` 3, `dialogs` 8; Missions: `missions` 8, `mission_steps` 6, `mission_objectives` 2, `mission_tasks` 4; for example `dialogs.sql` `VALUES (1386, ...)` is dialog 1386 "Escort Anor"). None is a chain id. The filtered form `git grep -n -E "INSERT INTO content_\w+ .*VALUES \((138[6-9]|139[0-3])," <ref> -- db/resources/Content` returns **0 hits** on all five refs. `content_chains` 1386-1393 are free. The `content_actions`, `content_conditions` and `content_triggers` rows below key on `chain_id`, so they are also free.

## Frozen (a) XML

None (stock mission). 1521 keeps its cooked XML; the item grant needs no XML edit.

## Frozen (b) Missions seeds

None (stock mission). `missions.sql:539` carries 1521 with `num_repeats` 1, `can_abandon` false, `can_repeat_on_fail` true (REPO); it is not touched.

## Frozen (c) Dialogs seeds and override

None (stock mission). No new dialog: the grant gives no monologue (rule: at most 2 per quest, none needed). Feedback on the first press is the inventory refresh (section Behavior).

## Frozen (d) Chains

File `db/resources/Content/Seed/castle_symbiote_chains.sql`. Insert **after the last line of the file** (line 36, the `1377` `content_actions` row; the file ends with CRLF and has no trailing blank line, so c1 adds one blank line first and keeps CRLF), as a new block. Chain 1386 is a NEW chain, so chains 1369, 1370, 1374-1377 and every test that pins them stay byte-identical. Statements stay on one line (interact_tag_linter reads only lines that start with `(` or `VALUES`, and 1386 has no `interact_tag`, so the linter is unaffected).

```sql
-- 1386 completion item: on a real active->completed 1521 grant the Immature Goa'uld Symbiote (5169) into the mission bag (container 2), once. mission_completed fires only on the active->completed transition (executor/mission.rs), so a second completion event needs 1521 to be reset first (CS-03dS, OD-1)
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (1386, '1521 - completed: grant the implant item 5169 (CIMMERIA_AUTHORING)', 'mission', 1521, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (1386, 'mission_completed', '1521', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (1386, 'world', 8, NULL, 'eq', NULL, 0), (1386, 'mission_status', 1521, NULL, 'eq', 'completed', 1);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (1386, 'add_item', 5169, NULL, '{"container": 2}', 0, 0);
```

The header comment of the file (lines 1-2: "Entry 1369-1370; tank 1374-1377 ... Option B (no implant, item or debuff)") becomes false; c1 rewrites lines 1-2 to say "Option A: 1386 grants 5169 on completion" and nothing else on those lines (the pinned tests read rows, not comments).

Why the shape:
- `mission_completed` with key `1521` loads (`loader/trigger.rs:77`) and fires after `complete_mission_direct` and only when the mission was `MISSION_ACTIVE` before (`executor/mission.rs:172-200`). So the grant happens once per real completion and never on a retry of 1377.
- The `world 8` row is the family convention (sort_order 0). 1377 is world-8 gated, so the completion happens in Castle; a completion by another path (the GM) does not grant and is an OPEN edge (see Second completion).
- The `mission_status 1521 completed` row defends against a future second `mission_completed 1521` source and gives the wrong-state test a real condition to flip. `fire_mission_completed` runs after the status flip ("see the post-mutation state", `event_dispatch/mission.rs:5`), so the row is true on the real path.
- `add_item` loads as `Action::GrantItem` (`loader/action.rs:44-56`): `qty` defaults to 1, `container` 2. Item 5169 has `container_sets {2}` (g-sym 3.1) so the explicit container equals the item's own set; it states the intent.
- No `remove_item` and no `item_use` chain exist for 5169, so the double-consume trap cannot arise: nothing consumes the item (G-SYM-CONSUME below).

## Frozen (e) Templates and spawns

None (stock mission). No template, spawn or navmesh change, so no `nav_inspect` run is needed and none was made.

## Frozen (f) Reward

One item, no cash, no XP: `add_item 5169 qty 1 container 2` in chain 1386. The mission already awards no XP or Naquadah (`missions.sql:539` `reward_naq` 0, `reward_xp` 0, `award_xp` true is the stock flag and unchanged). Economy sign-off: 5169 is a mission item, `IsSellable="false"`, `IsDeletable="false"`, `MaxStackSize="1"` (g-sym 3.2), so it adds no vendor value and cannot be dropped or sold. Sign-off: **approved, no economy effect.**

## Gate rulings

| Gate | Ruling | Evidence | What ships |
|---|---|---|---|
| G-SYM-GAME | **Left open** (game identity), and not needed | No shipped source names the tank's game (g-sym 2.1, 2.4). The grant point is mission completion (OD-1), not the Livewire victory, so the grant does not depend on the game type | CS-03S's Livewire (CIMMERIA_AUTHORING) stays; the item is granted at completion regardless of which game opened the path |
| G-SYM-CONSUME | **Closed by REPO evidence: using 5169 does not consume it** | `items_event_sets.sql:3979` binds (5169, ability 1927, event 5). `consumable_use::classify` (`consumable_use.rs:124-159`) sends it to `NotNative("effect_not_native")` because effect 2481 `script_name` is NULL (`effects.sql:3617`) and the ability is not the 597 filler. `is_unimplemented_bag_consumable` refuses only bag items (`{1,17}`); 5169 is `{2}` and "stays silent" (`consumable_use.rs:208-212`). No `item_use` chain exists for 5169 (`git grep -n -w 5169` over `db/resources/Content` returns nothing at base) so no `remove_item` can run | Using 5169 does nothing and spends nothing. The item stays as a keepsake; it is `IsDeletable false`. UATPending: one double-click confirms silence |
| G-SYM-DEBUFF | **Left open**: ship "no debuff" | Nothing in REPO applies 1926/2480 (g-sym 3.10, negative check: `git grep -n -w -E '1926\|1927\|2480\|2481' -- db/resources/Content` is 0 hits). Dialog 4989 implies weakness but is unreferenced | No `apply 1926`. Chain 1370 stays unchanged. Cure ability 1927 is unreachable (no use path), which matches "no debuff" |
| G-SYM-RADIO | **Left open** | The radio 5168 has no Castle grant and its Use is client-inert (#334; g-sym 4.4, 4.8). It gates only the original report opener | 1521 still completes through dialog 4991's close (1377), unchanged. No radio is granted |

## Behavior

| State | Event | Chain | Actions | Player sees |
|---|---|---|---|---|
| 1521 step 4458 active, world 8 | 4991 closes (`dialog_choice -1`) | 1377 | `complete_mission 1521`, then (awaited) `mission_completed 1521` | Mission complete in the journal (existing) |
| 1521 just completed, world 8 | `mission_completed 1521` | 1386 | `add_item 5169 {"container": 2}` | Inventory refresh with the new "Immature Goa'uld Symbiote" in the mission bag, in the same tick as the completion |
| same, mission bag full | same | 1386 | base `handle_content_grant_item` finds `ContainerFull`, mails `SystemItem::Minted {5169, qty 1}` from "Mission Rewards" | System line "Your bag was full. Your item is in your mailbox." plus a mail notification; the mission is still completed |
| 1521 completed, item held | 4991 re-closed, tank clicked | 1377 / 1374 / 1376 | none (step gates fail; `complete` on a non-active mission skips the event) | nothing; no second grant |
| 1521 not completed (any step) | any | 1386 | none: it only listens to `mission_completed 1521` | nothing |
| 1521 completed, world not 8 | `mission_completed 1521` | 1386 | none (world row fails) | nothing (a GM completion elsewhere grants nothing; OPEN, see below) |
| 5169 held | double-click (event 5) | none | classify -> NotNative, `{2}` item, silent | nothing; item kept |

Every click has feedback on the first press: the completing click is the existing 1377 dialog close; the grant adds a visible inventory change or a mail line, never silence.

### Full inventory (items advisor)

- `Action::GrantItem` from a chain is fire-and-forget: `inventory::grant` sends `CellToBaseMsg::GrantItem` and does not wait (`executor/inventory.rs:32-87`). The chain does not gate the mission on the grant. This is correct here because the grant is the *last* thing and 1521 is already completed; nothing downstream needs the item.
- The base routes a chain grant (`notify_gm` false, no loot source) to `handle_content_grant_item` (`item_grant_dispatch.rs:48-61`). On `GrantRefusal::ContainerFull` it mints the item into a system mail (`grant_item.rs:82-174`), so the item is not lost, and sends the player a line.
- Other refusals are not mailed: `CommitUnknown` is deliberately not retried (the row may exist); `NoDatabase`, `DatabaseError`, `NotGrantable`, `StorageOnly` log and drop the item. These are fault paths, not player states. OPEN: a dropped grant is not retried and the mission is not reopened. Accepted for the seal; the c1 live-DB test covers only `ContainerFull`.
- The mission still completes in every case because 1377 completes before 1386 runs.

### Second completion and duplicate grant (items advisor)

- 1521 `num_repeats` is 1 (`missions.sql:539`). The offer guard refuses a COMPLETED mission when `repeats > num_repeats` (`lifecycle.rs:95`); after one completion `repeats` is 1, so by the guard a re-accept would pass (1 > 1 is false). It is the chain, not the guard, that blocks a replay: 1369 requires `mission_status 1521 not_active` (`castle_symbiote_chains.sql:6`) and a completed mission is `completed`, so the Infirmary entry never re-offers. Net: **in normal play 1521 completes once, and 1386 fires once.**
- Duplicate grant: `persist_grant` merges only when `max_stack_size > 1` (`persist.rs:186`). 5169 has stack 1, so a second grant creates a **second instance** in a new slot; there is no unique-item refusal in the grant path and no `has_item` condition type (`loader/condition.rs` lists mission_status, step_status, archetype, objective_status, counter, stat_below_max, world, entity_tag_state). A second `mission_completed 1521` therefore WOULD grant a second 5169. It can happen only after a GM reset (`gmMissionClear` deletes the row, `lifecycle.rs` abandon path) and a second full run; the item is `IsUnique` in the cooked requirements but the server does not enforce it. Ruling: accepted, GM-only, one extra keepsake with no gameplay effect. No extra guard row is frozen because no existing condition can express "item not held".
- Wrong-world edge: a GM `complete_mission 1521` outside world 8 grants nothing. Accepted and recorded.

## Test list

Only the c1 list (CS-03d is one packet). `s`, `d` and `c2` are not used. All in `crates/cell-content/src/cell/content/chain_replay_tests/` as a new file `mission_1521_implant.rs` (c1 wires it into the module list) so `mission_1521.rs`, `mission_1521_entry.rs` and `mission_1521_seed_guard.rs` are not edited. Live-DB tests carry the `live_db_` prefix.

### c1

| # | Test | Assertion | Revert proof |
|---|---|---|---|
| 1 | `live_db_chain_1386_loads_with_the_implant_grant` | `content_chains` has 1386 enabled, scope `mission`/1521; the chain loads into the engine; exactly one trigger `mission_completed '1521'`, conditions `world 8` + `mission_status 1521 eq completed`, exactly one action `add_item` target 5169 with `container` 2 and `delay_ms` 0 | Delete the 1386 rows (or change the target to 5168): the row count / target assert fails |
| 2 | `chain_1386_grants_5169_once_on_completion` | Replay `fire_mission_completed(1521)` in world 8 with 1521 completed: exactly one `CellToBaseMsg::GrantItem { item_id: 5169, container_id: 2, count: 1 }` and nothing else (no `RemoveInventoryItem*`, no `ApplyEffect`/ability action) | Drop the `add_item` action: zero `GrantItem` messages; change qty/container: the field assert fails |
| 3 | `chain_1386_grants_nothing_for_the_wrong_world_or_mission_or_state` | (a) world 12 + completed: no `GrantItem`; (b) `mission_completed 1520`/`1522` in world 8: none; (c) world 8 + 1521 still `active` (the staged status): none | Remove the `world` row: (a) grants; remove the `mission_status` row: (c) grants; change `event_key`: (b) grants |
| 4 | `chain_1386_follows_the_1377_close_and_never_reopens_1521` | End to end through the executor with `engine_with(&pool, &[1377, 1386])`: staged player in world 8 with 1521 active at step 4458, `dialog_choice 4991` (-1): 1521 is COMPLETED, exactly one `GrantItem { item_id: 5169, container_id: Some(2), count: 1 }` is emitted after the `complete` mission update, and no step or objective is reopened. A second `dialog_choice 4991` on the now-completed mission emits no second `GrantItem`. The cell cannot see a full bag (the grant is fire-and-forget), so the full-inventory half is test 5 on the base side | Delete the 1386 rows: no `GrantItem` after the close (first assert fails). The `world` and `mission_status` rows do not affect this path; tests 1 and 3 own them |
| 5 | `live_db_base_full_bag_mails_the_implant` (base crate, `handle_content_grant_item`) | Stage a full container 2 for the player, run the content grant of 5169: no inventory row, one system mail with `SystemItem::Minted{5169,1}`, the "Your bag was full" feedback line, and the grant is not retried | Make the mail fallback skip `ContainerFull`: the mail-row assert fails (owner: items; the helper exists, c1 reuses its fixture) |
| 6 | `chain_1386_second_completion_event_grants_a_second_copy_only_after_reset` | Document the stack-1 behaviour: after the first grant, a second `fire_mission_completed(1521)` (only reachable after a GM reset) emits a second `GrantItem`; with 1369 gating (`not_active`), the Infirmary entry for a COMPLETED 1521 emits no `accept_mission` | Flip 1369's `not_active` to anything that matches `completed`: the entry assert fails |
| 7 | `chains_1369_1370_1374_1375_1376_1377_are_unchanged_by_1386` | Static: the six existing chains' trigger/condition/action rows equal the CS-03S frozen rows; the existing `mission_1521*` tests are not edited | Edit any row of the six: the existing pins and this one fail |

Existing pins that must stay green and are not changed: `mission_1521.rs` (`ALL_CHAINS` stays the six ids), `mission_1521_entry.rs:188-199` (selects `chain_id IN (1369, 1370, 1374, 1375, 1376, 1377)`; 1386 is not in the list, so the count is unaffected), `mission_1521_seed_guard.rs`. Test 6 documents the duplicate edge so a later "unique item" fix changes it deliberately.

Regression filters c1 adds (the lane must run these): `chain_replay_tests::mission_1521` (matches the existing four 1521 files plus `mission_1521_implant`), `consumable_use` (guard that 5169 stays silent), and the base `handle_content_grant_item` tests. The `live_db_` tests run with the lane's live-DB mode.

A test that stays green when the 1386 rows are removed is a happy-path test, not a guard; tests 1-3 each fail on removal.

## End-UAT rows

All `UATPending (end UAT)`:

- CS-03d-U1: completing 1521 shows "Immature Goa'uld Symbiote" in the mission bag with a name, icon and tooltip (icon and tooltip text are client data, not checked here).
- CS-03d-U2: double-click the item; nothing happens, nothing is consumed, no error line.
- CS-03d-U3: complete 1521 with the mission bag full; the mail arrives with the item and the bag line shows. Confirm where the mailed item lands when claimed (a `{2}` item from mail: OPEN, depends on the mail claim placement).
- CS-03d-U4: relog after completion; the item is still there and the mission stays complete, with no second grant on login.
- CS-03d-U5: no debuff icon (Symbiote Loss) appears at any step (G-SYM-DEBUFF left open).

## Handoffs

- mission-systems-advisor: confirm `mission_completed` fires exactly once on the active-to-completed path for 1521 when the final objective closes through 1377 (the executor already guards it) and that no other completer exists.
- rust-gameserver-dev (c1): add chain 1386 to `castle_symbiote_chains.sql`, add `mission_1521_implant.rs`, and rewrite the two header comment lines.
- database-persistence: no schema change. Item 5169 persists as a normal inventory row.
