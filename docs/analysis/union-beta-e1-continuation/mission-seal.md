# UB-00a1: Beta E1 mission seal

Packet UB-00a1 (docs only). Answers Q1-Q6 of `docs/analysis/union-zone-planning/beta-packets/UB-00.md`. Every `change:` row names the packet file and the exact row to edit, so the orchestrator can copy it into the packet files before wave B1.

Labels: RESTORED_SOURCE (client or original data), CIMMERIA_RECONSTRUCTION (Giza seed rows in `db/resources/`), CIMMERIA_AUTHORING (Cimmeria engine code or our own chains). Items that need the live client are UB-90 rows.

Evidence base, all CIMMERIA_RECONSTRUCTION unless noted:
- Steps, objectives, tasks: `db/resources/Missions/Seed/{missions,mission_steps,mission_objectives,mission_tasks}.sql`.
- Dialogs: `db/resources/Dialogs/Seed/{dialogs,dialog_screens,dialog_screen_buttons,dialog_set_maps,speakers}.sql`.
- Engine (CIMMERIA_AUTHORING): paths cited inline.

## Verdict summary

| Q | Verdict |
|---|---|
| Q1 step matrix | 17 missions, every step row `ok` except 4 wording `change:` rows (C1-C4) |
| Q2 dialogs | 2 R1 violations (dialog 1997, dialog 3228) and 2 R3 violations (dialog 1916, dialog 3905); fallback is a Beta dialog-patch packet (C5, C6) |
| Q3 HELD-B12 `en=false` steps | works; the flag is inert on the server and never on the wire |
| Q4 F19 trap | confirmed and avoided by every packet; chain 23316 safe |
| Q5 quota and `remove_item` | counters and `once` reset on relog; `remove_item` is first-stack-only and never fails the chain |
| Q6 type-4 buttons | fire `dialog_choice`; packets may key on them; live confirmation is a UB-90 row |

## Q1: step matrix

Columns: step id, index, objective(s), owning packet, trigger and action (chain ids from the packet), verdict. Objective `en=false` marks listed where present. All tasks are type 1 and enabled; all steps and objectives are `hidden=false`, `optional=false`. Caveat (B0 review): the step, objective and task columns were verified row by row against the mission seeds; the trigger / action column is summarised from the packet chain tables and chain-id ranges and was not re-read chain row by row, so the owning packet's coder re-reads its chains.

Engine facts the matrix relies on:
- `advance_step` force-completes the old step's objectives and does NOT run the mission auto-complete (`crates/cell-content/src/cell/missions/progression.rs:28-172`, loop 161-169). CIMMERIA_AUTHORING.
- `complete_objective` completes the whole mission once the last required objective is done (`progression.rs:463-506`). CIMMERIA_AUTHORING.
- `complete()` pushes only the current step onto `completed_steps` and bumps `repeats` (`crates/entity/src/missions.rs:91-103`). So a step that is skipped by an end-of-mission `complete_mission` is never listed.

### 1423 (UB-10, chains 23020-23022)

| Step | Idx | Objectives | Trigger / action | Verdict |
|---|---|---|---|---|
| S4223 | 0 | O4877 (en=false) | 23020 next S4224 plus bind | ok |
| S4224 | 1 | O4878 (en=false) | 23021 restore; 23022 show 4643 plus complete 1423 | ok |

### 381 (UB-11, chains 23040-23053)

| Step | Idx | Objectives | Trigger / action | Verdict |
|---|---|---|---|---|
| S1408 | 0 | O1630 (en=false) | 23040-23042 accept and hand-in of inventory | change: C1 (dead trigger row `pick:1886` in chain 23042) |
| S1409 | 1 | O1631 | 23043-23052 cage object clicks, counter | ok (Q5 caveat) |
| S1410 | 2 | O1632 | 23053 remove cages, complete 381 | change: C2 (journal note) |

### 473 (UB-12, chains 23070-23083)

| Step | Idx | Objectives | Trigger / action | Verdict |
|---|---|---|---|---|
| S1723 | 0 | O2036 | region or interact fallback (P10) | ok |
| S1724 | 1 | O2037 (Arc-42), O2038 (Arc-66) | P6 partition: each click completes one objective; the second chain uses `advance_step` | ok (F19 safe, Q4) |
| S1725 | 2 | O2039 | mid-step talk to Felger (P4) | ok |
| S1726 | 3 | O2040 | turn-in to Carter, `complete_mission` (P3) | ok |

### 478 (UB-13, chains 23100-23108)

| Step | Idx | Objectives | Trigger / action | Verdict |
|---|---|---|---|---|
| S1746 | 0 | O2066 | region (P10) | ok |
| S1747 | 1 | O2067 | workstation click (P5) | ok |
| S1749 | 2 | O2069 | turn-in to Carter (P3). There is no S1748; the idx sequence is contiguous | ok |

### 1425 (UB-14, chains 23130-23136)

| Step | Idx | Objectives | Trigger / action | Verdict |
|---|---|---|---|---|
| S4242 | 0 | O4910 (en=false) | hub download | ok |
| S4243 | 1 | O4911, O6325 (both en=false) | P6 partition | ok |
| S4244 | 2 | O4912 | turn-in to Jackson | ok |

### 1426 (UB-15, chains 23160-23171)

| Step | Idx | Objectives | Trigger / action | Verdict |
|---|---|---|---|---|
| S4245 | 0 | O4913, O6326 (both en=false) | P6 partition | ok |
| S4246 | 1 | O4914 | hub upload | ok |
| S4247 | 2 | O4915 | turn-in to Carter | ok |

### 1427 (UB-16, chains 23190-23195)

| Step | Idx | Objectives | Trigger / action | Verdict |
|---|---|---|---|---|
| S4248 | 0 | O4919 (tasks T7626 and T7627), O6327 | P6 partition; O4919 has two tasks, so confirm the chain completes the objective, not a task | ok |
| S4249 | 1 | O4920 | turn-in to Carter | ok |
| S4250 | 2 | O4921 | Woolsey (type-4 button 460 on dialog 4670 final screen) | ok (Q6) |

### 1500 (UB-20, chains 23220-23222)

| Step | Idx | Objectives | Trigger / action | Verdict |
|---|---|---|---|---|
| S4369 | 0 | O5080 | arrival | ok |
| S4370 | 1 | O5081 | turn-in to Carter | ok |

### 1502 (UB-21, chains 23240-23250)

| Step | Idx | Objectives | Trigger / action | Verdict |
|---|---|---|---|---|
| S4376 | 0 | O5087 | region (P10) | ok |
| S4377 | 1 | O5088 | locate box | ok |
| S4378 | 2 | O5089 | retrieve medallion | ok |
| S4481 | 3 | O5215 | return to Ras'nor | ok |

### 1503 (UB-22, chains 23280-23286)

| Step | Idx | Objectives | Trigger / action | Verdict |
|---|---|---|---|---|
| S4379 | 0 | O5090 | Bra'tac talk | ok |
| S4380 | 1 | O5091 | antidote use | ok |
| S4759 | 2 | O5478 | scanner use | ok |
| S4381 | 3 | O5477 | report to Heimdall | ok |

The idx order (S4759 before S4381) differs from the step id order; chains must key on step id, not on id order.

### 481 (UB-30, chains 23310-23317)

| Step | Idx | Objectives | Trigger / action | Verdict |
|---|---|---|---|---|
| S1760 | 0 | O2078 | region | ok |
| S1761 | 1 | O2079 | turn-in to Carter | ok |
| S1762 | 2 | O2080 (T8598), O4015 (T4835) | chain 23316 completes O2080 only; O4015 stays open so F19 does not fire; the Mimir turn-in completes O4015 | ok |

### 420 (UB-31, chains 23340-23351)

| Step | Idx | Objectives | Trigger / action | Verdict |
|---|---|---|---|---|
| S1455 | 0 | O1689 "Your collection quota is 5 scanners" | quota chains 23340-23350, counter, 23351 removal | ok (Q5) |
| S1457 | 1 | O1691 | Mimir report, `complete_mission` | change: C3 (journal note; S1457 is never entered) |

### 421 (UB-32, chains 23360-23368)

| Step | Idx | Objectives | Trigger / action | Verdict |
|---|---|---|---|---|
| S1460 | 0 | O1698 | region | ok |
| S1461 | 1 | O1699 | signal source | ok |
| S1462 | 2 | O1700 | drone examine | ok |
| S1464 | 3 | O1702 | turn-in to Mimir. The idx skip (no S1463) is a seed gap, not an error | ok |

### 1147 (UB-33, chains 23390-23395)

| Step | Idx | Objectives | Trigger / action | Verdict |
|---|---|---|---|---|
| S3458 | 0 | O4020 | machine use | ok |
| S3459 | 1 | O4021 | turn-in to Mimir | ok |

### 443 (UB-34, chains 23405-23417)

| Step | Idx | Objectives | Trigger / action | Verdict |
|---|---|---|---|---|
| S1482 | 0 | O1721 | Carter | ok |
| S1483 | 1 | O1722 | Satterfield | ok |
| S1484 | 2 | O1723 | Mimir transmitter | ok |
| S1485 | 3 | O1724 | fire containment | ok |
| S1486 | 4 | O1725 (en=false "Examine the Fire Control Hub") | object click | change: C4 (wording only) |
| S3455 | 5 | O1726 | report to Mimir | ok |

### 477 (UB-35, chains 23435-23459)

| Step | Idx | Objectives | Trigger / action | Verdict |
|---|---|---|---|---|
| S1744 | 0 | O2060 Carter, O2061 Debeaux, O2062 Felger, O2063 Satterfield | P6 partition: four recordings; the last must go through `advance_step` or be kept open by another objective | ok (Q4) |
| S1745 | 1 | O2064 | turn-in to Mimir | ok; change: C5b (dsm flag value in the source line) |

### 479 (UB-36, chains 23460-23467)

| Step | Idx | Objectives | Trigger / action | Verdict |
|---|---|---|---|---|
| S1750 | 0 | O2070 (en=false) | process data | ok |
| S1751 | 1 | O2071 | simulation activate | ok |
| S1752 | 2 | O2072 | analyze | ok |
| S3644 | 3 | O4235 (en=false) | discuss with Mimir, Woolsey choice | ok (Q6) |

### Q1 `change:` rows

- C1. packet `UB-11` (`beta-packets/UB-10-13.md`), chain 23042: delete the trigger row `pick:1886`. Dialog 1886 is never displayed because the tag chain suppresses bind auto-display, so the row is dead. This is an engine-reachability fix, not an R1 fix: dialog 1886 itself passes R1 (final-screen buttons 8/t2 and 9/t1; see the addendum). CIMMERIA_AUTHORING.
- C2. packet `UB-11`, the mission note that says the journal shows all three steps done: rewrite. `complete()` pushes only the current step (`crates/entity/src/missions.rs:91-103`), so S1410 is never entered or listed.
- C3. packet `UB-31` (`beta-packets/UB-30-36.md`), the same note for S1457: rewrite for the same reason.
- C4. packet `UB-34`, the S1486 row: say the `en=false` applies to O1725 only and the server ignores it (Q3). Wording only.
- C5b. packet `UB-35`, the source line "dsm 2457 flag 8388608": change to 134217728. Seed `dialog_set_maps.sql` row DSM2457 reads dialog 1999, flag 134217728. RESTORED_SOURCE-derived seed, CIMMERIA_RECONSTRUCTION.

## Q2: dialog, dsm and button table

Rule R1 (`crates/content-engine/tests/it/dialog_button_linter/rules.rs:62`, allowlist `R1_ALLOWLIST: [R1Exemption; 0] = []`): a chain-keyed dialog must have a button on its FINAL screen, or no buttons at all. R3: BlurbWin draws More Info (1) and Accept (2); DialogWin draws Accept (2) and Generic 1-3 (4, 5, 6) (`mod.rs:40-58`).

Rows checked (dsm flag from `dialog_set_maps.sql`):

| Dialog | dsm (set) | Flag | UI | Screens / buttons | R1 |
|---|---|---|---|---|---|
| 1999 | 2457 (500) | 134217728 | Blurb | 1 screen (s64039): More Info, Accept | pass |
| 1994 | 2462 (502) | 134217728 | Blurb | 1 screen (s63518): More Info, Accept | pass |
| 1988 | 2349 (472) | 8388608 | Blurb | 1 screen (s49385): Accept, More Info | pass |
| 3903 | 4588 (1174) | 8388608 | Blurb | 1 screen (s49434): Accept, More Info | pass |
| 2017 | 2440 (496) | 134217728 | Blurb | 1 screen (s63887): Accept, More Info | pass |
| 4653 | 5457 (1478) | 134217728 | Blurb | 1 screen: More Info, Accept | pass |
| 4657 | 5460 (1479) | 134217728 | Blurb | 1 screen: Accept, More Info | pass |
| 4661 | 5464 (1480) | 134217728 | Blurb | 1 screen: Accept, More Info | pass |
| 5040 | 5885 (1594) | 134217728 | Blurb | 1 screen, speaker 0 (Not Set) | pass |
| 5047 | 5894 (1595) | 134217728 | Blurb | 1 screen, speaker 0 (Not Set) | pass |
| 1998 | 2460 (501) | 134217728 | Blurb | 1 screen (s98234): More Info, Accept | pass |
| 1996 | 4408 (500) | 536870912 | Dialog | 1 screen, no buttons | pass (zero-button) |
| 4670 | 5468 (1480) | 268435456 | Dialog | 3 screens; only final s103614 has button type 4 id 460 "Convince Woolsey to test O'Neill's rock." | pass |
| 1997 | 2456 (500) | 0 | Dialog | button on a non-final screen | VIOLATION (UB-35 chain 23437) |
| 3228 | 2461 (502) | 0 | Dialog | button on a non-final screen | VIOLATION (UB-36 chain 23462) |

#### Q2 addendum (B0 review): the remaining dialogs

The table above lists the offer, blurb and done dialogs plus the two R1 findings; the review completed it against `dialogs.sql`, `dialog_screens.sql`, `dialog_screen_buttons.sql` and `dialog_set_maps.sql`. Columns: dsm id, dialog set, flag, UI type, buttons on the FINAL screen (button id / type). Of these 27, 19 are `pick:`-keyed in the packets (1882, 1886, 1918, 1953, 1987, 2018, 3223, 3224, 3904, 4654, 4655, 4658, 4659, 4662, 5041, 5042, 5043, 5048, 5497); the other 8 (1883, 1884, 1915, 1916, 1917, 1995, 2014, 2015) are only displayed by `show`, so R1 does not apply to them but R3 does (the linter counts displayed dialogs, `seed_model.rs` `referenced()`). R1 holds for every row (a button on the final screen, or none at all). R3 holds for every row except 1916 and 3905, below.

| Dialog | dsm | Set | Flag | UI | Final-screen buttons |
|---|---|---|---|---|---|
| 1882 | 2306 | 463 | 16777216 | Dialog | 8/t2 (all 6 screens) |
| 1883 | 2302 | 463 | 33554432 | Dialog | none |
| 1884 | 2304 | 463 | 16777216 | Dialog | none |
| 1886 | 2305 | 463 | 8388608 | Blurb | 8/t2, 9/t1 |
| 1915 | 2465 | 504 | 16777216 | Dialog | none |
| 1916 | 2467 | 504 | 33554432 | Dialog | 9/t1 |
| 1917 | 4579 | 473 | 33554432 | Blurb | none |
| 1918 | 2352 | 473 | 16777216 | Dialog | 8/t2 |
| 1953 | 2346 | 471 | 0 | Dialog | 8/t2 |
| 1987 | 2347 | 472 | 16777216 | Dialog | 8/t2 |
| 1995 | 3884 | 501 | 536870912 | Dialog | none |
| 2014 | 2438 | 496 | 536870912 | Dialog | none |
| 2015 | 2439 | 496 | 268435456 | Dialog | none |
| 2018 | 2441 | 496 | 0 | Dialog | 8/t2 |
| 3223 | 3878 | 501 | 0 | Dialog | 8/t2 |
| 3224 | 3883 | 501 | 1073741824 | Dialog | 247/t4 |
| 3904 | 4589 | 1174 | 0 | Dialog | 8/t2 |
| 4654 | 5458 | 1478 | 0 | Dialog | 8/t2 |
| 4655 | 6936 | 1478 | 1073741824 | Dialog | 456/t4 |
| 4658 | 5461 | 1479 | 0 | Dialog | 8/t2 |
| 4659 | 6944 | 1479 | 536870912 | Dialog | 458/t4 |
| 4662 | 5465 | 1480 | 0 | Dialog | 8/t2 |
| 5041 | 5886 | 1594 | 0 | Dialog | 8/t2 |
| 5042 | 7151 | 1594 | 1073741824 | Dialog | 575/t4 |
| 5043 | 7152 | 1594 | 1073741824 | Dialog | 576/t4 |
| 5048 | 5895 | 1595 | 0 | Dialog | 8/t2 |
| 5497 | 6937 | 1478 | 1073741824 | Dialog | 457/t4 |

Dialogs 3057, 3161, 3227, 3722, 3895, 3901, 3905, 4643, 4660, 4663, 4664, 5026, 5044 and 5051 carry no pick key and no or one trailing button (3905's final screen has 9/t1); none is chain-keyed by `dialog_choice`, so R1 does not apply to them. The only R1 violations in the whole set remain 1997 and 3228.

R3 (B0 review pass 2): the DialogWin draws types 2, 4, 5 and 6 only (`rules.rs` `drawable_button_types`), and the linter checks every dialog a chain displays, not only keyed ones. Two displayed DialogWin dialogs carry a type-1 More Info button the window cannot draw: dialog 1916 (screens 95484 and 95485, 9/t1; displayed by UB-31 chain 23317) and dialog 3905 (final screen 49940, 9/t1; displayed by UB-33 chain 23395). Once `beta_e1_*_chains.sql` is in `CHAIN_FILES`, `r3_violations` fails on both (no R3 allowlist exists). Both are `show`-only, so the dead button costs nothing at runtime, but the linter goes red: see C6.

The speaker label is per dialog, not per screen. Dialogs 5040 and 5047 have speaker 0 "Not Set" so the client header is blank; cosmetic, UB-90 row UB-U2b.

Behaviour, CIMMERIA_AUTHORING:
- A zero-button dialog sends `(id, -1)` on close; a dialog with any button sends nothing on close (`crates/cell-methods/src/cell/cell_methods/player/interaction/dialog.rs:12-124`).
- `dialog_choice` is rejected unless the dialog is in the player's offered-dialog set; the gate consumes the entry once (`take_offered_dialog`, #479 gate, `crates/cell-content/src/cell/content/event_dispatch/dialog.rs:94-160`). It is a one-shot set, not a single `open_dialog_id` pin, so two dialogs can be offered at once but each choice is accepted once per display.
- Non-interact chains can only display MONOLOGUE dialogs (agent memory `dialog-chain-authoring-rules.md`).

### Q2 `change:` rows (fallbacks)

- C5. New packet `UB-DP` (a Beta dialog-patch packet, wave B1, before UB-35 and UB-36): add `DialogPatch` rows in `crates/resources/src/base/dialog_overrides/` moving Accept (button type 2, id 8) onto the final screens 64049 (dialog 1997) and 64068 (dialog 3228), with matching hand edits to `db/resources/Dialogs/Seed/dialog_screen_buttons.sql`. Model: `patches_castle.rs` (`DialogPatch { dialog_id, ui_screen_type: None, buttons: ButtonPlan::OnlyOn { screen_id, button_type, button_id, text } }`). CIMMERIA_AUTHORING.
- C5a. packet `UB-35` chain 23437 and packet `UB-36` chain 23462: if `UB-DP` is not taken, rekey the trigger from `dialog_choice` to `interact_tag`, so the linter no longer treats the dialog as chain-keyed. Pick one; do not ship both.
- C6. `UB-DP` (the C5 packet; if C5a replaces C5, UB-DP is still needed for C6): add `DialogPatch` rows with `ButtonPlan::StripAll` for dialogs 1916 and 3905 (`ui_screen_type: None`), with matching hand edits deleting the three type-1 rows from `db/resources/Dialogs/Seed/dialog_screen_buttons.sql` (screens 95484, 95485, 49940). Neither dialog is `dialog_choice`-keyed, so a zero-button close sends `(id, -1)` to no chain. Without C6, UB-31 and UB-33 fail `dialog_button_linter` R3 once their chain files are registered. CIMMERIA_AUTHORING.

## Q3: HELD-B12, `en=false` steps

Verdict: works. `step_enabled` and objective `is_enabled` are never read by server code and never put on the wire. The loaders in `crates/cell-catalog/src/cell/spawner/missions.rs` have no "enabled" field at all; only the lab workbench and test fixtures touch it. CIMMERIA_AUTHORING.

Consequences:
- All SGU-family steps (1423, 1425-1427, 1500, 1502, 1503) are en=false at step and objective level; chains drive them without a seed edit.
- Extra en=false objectives (O6325, O6326, O6327, O4235, O1630, O1725, O2070) are required objectives: they are force-completed by `advance_step`, never by a gate on the flag.
- No server-side fix packet and no edit to `mission_steps.sql`.
- Whether the client journal displays an en=false row is unobservable from the server: UB-90 row UB-U1b.

## Q4: the F19 trap

Confirmed. `complete_objective` on the last required objective completes the entire mission (`progression.rs:463-506`; vacuous-truth warning 484-503). `advance_step` bypasses it (`progression.rs:161-169`).

- UB-30 chain 23316 completes O2080 while O4015 stays open: safe.
- UB-12 (S1724) and UB-35 (S1744) partitions are right: each mid-step click completes a non-final objective, and the last transition is `advance_step`.
- No packet's mid-mission chain calls `complete_objective` on a last required objective. No `change:` rows.

## Q5: quota mechanism and `remove_item`

- The quota counter is `entity.counters`, copied into the condition context by `populate_counters_context` (`crates/cell-content/src/cell/content/mission_context.rs:80-94`) and read by the `Counter` condition as `counter_<name>`, default 0 (`crates/content-engine/src/conditions/mod.rs:354-362`).
- `once` is per `(entity, chain id)` for one cell-entity life (`crates/cell-content/src/cell/content/executor/once_gate.rs`, #802). Counters and `once` both reset on relog and on world change; neither is persisted. Effect: a player who relogs mid-quota restarts the count at 0 and can click the same objects again. There is no persistent primitive. UB-90 rows UB-U3b (UB-11 cages) and UB-U5b (UB-31 scanners) cover the restart.
- `remove_item` is fire-and-forget (`crates/cell-content/src/cell/content/executor/inventory.rs:93-209`): it sends `RemoveInventoryItemByType` (or `RemoveInventoryItem` when the context has an `instance_id`) and logs an error only if the channel send fails.
- The handler (`crates/base-methods/src/base/world_entry/methods/inventory/core/remove_by_type.rs`) takes the first stack by `ORDER BY container_id, slot_id LIMIT 1`; a missing item warns and returns (137-147); a short stack (count above stack size) warns and removes the whole stack (152-168); it never drains a second stack. It never fails the chain.
- Verdict: chains 23053 (UB-11), 23351 (UB-31) and the 3399 removal ship. Caveat: a player holding the items in two stacks keeps the second stack. No `change:` row; accepted as a known gap and listed in UB-90.

## Q6: type-4 buttons 247, 456, 457, 458, 575, 576

Type-4 (Generic 1) buttons on a DialogWin fire `dialog_choice` like any other button: `fire_dialog_choice` ignores the button type (`event_dispatch/dialog.rs:94-160`). The packets may key on them, subject to the #479 gate (the dialog must be open). Example in seed: dialog 4670 final screen s103614, button type 4 id 460. CIMMERIA_RECONSTRUCTION seed, CIMMERIA_AUTHORING dispatch.

Whether the real client sends the Generic 1-3 click for these ids is RESTORED_SOURCE territory and cannot be settled server-side: UB-90 row UB-U6b.

## UB-90 rows to add

| Row | What to confirm live |
|---|---|
| UB-U1b | Journal display of en=false steps and objectives (Q3) |
| UB-U2b | Blank speaker header for dialogs 5040, 5047 |
| UB-U3b | UB-11 cage counter and `once` after relog (Q5) |
| UB-U5b | UB-31 scanner quota after relog (Q5) |
| UB-U6b | Client click on type-4 buttons 247, 456, 457, 458, 575, 576 reaches `dialog_choice` (Q6) |
| UB-U7b | Two-stack `remove_item` leftover (Q5) |

## `change:` index

C1, C2 -> UB-11. C3 -> UB-31. C4 -> UB-34. C5b -> UB-35. C5 -> new UB-DP packet. C5a -> UB-35 chain 23437 and UB-36 chain 23462 (alternative to C5). C6 -> new UB-DP packet (gates UB-31 and UB-33).
