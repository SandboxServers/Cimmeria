# Beta E1 mission seal, part 2 (UB-00a2): HELD verdicts and object census

Answers questions 7-14 of [UB-00](../union-zone-planning/beta-packets/UB-00.md). Part 1 (questions 1-6) is UB-00a1's `mission-seal.md`. This file is docs only: no code or seed was edited.

Labels: RESTORED_SOURCE (the seed or client says it), CIMMERIA_RECONSTRUCTION (inferred from evidence), CIMMERIA_AUTHORING (our decision, no source). Seed paths are under `db/resources/`; the line is the row's line in the file.

## Summary

| Q | Item | Verdict | Packet effect |
|---|---|---|---|
| 7 | HELD-B1 Human order | Confirm 381, 473, 478 (inferred; the seed holds no prerequisite data) | none |
| 7 | HELD-B1b item 3399 | Remove at S1761 (the O2079 hand-in), in chain 23312 as written | none |
| 8 | HELD-B5 1425 data item | Packet claim "no data item exists" is wrong: 4833 exists. Still ship no item | reword UB-14 line 28 |
| 8 | HELD-B9 1426 camera | Packet default "none" is superseded: grant item 4832 | UB-14-16 chains 23165/23166/23169 |
| 8 | HELD-B7 beam | Confirmed: the teleporter objective is the beam; destinations stay with UB-00b | none |
| 9 | M2 flag 268435456 | Confirmed: glyph only | none |
| 10 | Meshes | One row per template below; 2336 stays reserved | UB-02a table |
| 11 | Mission rows | Confirmed: all 17 missions are xp 0 / naq 0; no `missions.sql` edit | none |
| 12 | HELD-B15 | No interaction-only dsm for 477 exists. Keep the dsm 2457 default | none |
| 13 | HELD-B14 | No transmitter dialog for 443. Keep the step update | none |
| 14 | O2080 | O2080 is not the last required objective. Chain 23316 is safe | none |

## Q7. HELD-B1 and HELD-B1b

### HELD-B1: Human order after 1423

Verdict: confirm 381 (Xenobiology), then 473, then 478. CIMMERIA_AUTHORING; the seed cannot prove it.

- No prerequisite data exists. Every `dialog_set_maps` row for these missions has `missions_completed '{}'` and `missions_not_accepted '{}'`, and `dialogs.accepts_mission_id` is NULL on their dialogs (`Dialogs/Seed/dialog_set_maps.sql`, `dialogs.sql`). The order has to come from our chains.
- Narrative evidence for 381 before 473: Carter's 381 dialog 1882 (dsm 2306, `dialog_set_maps.sql:1177`; dialog `dialogs.sql:3347`) tells the player to deliver the inventory to Felger and see what else he needs. Felger's 473 offer, dialog 2018 (dsm 2441, `dialog_set_maps.sql:1327`), opens cold ("Hey there, please tell me you're here to help") and does not mention Carter. 381 hands the player to Felger, so 473 follows it.
- Evidence for 478 last: Carter's 478 offer, dialog 3223 (dsm 3878, `dialog_set_maps.sql:3511`), says "My IT guys are going crazy trying to get everything wrapped up", a wrap-up line. The levels run 6, 6, 8 (`missions.sql` rows for 381, 473, 478), so 478 is the highest.
- The three share the label "Packing up Beta Site", which marks one arc (`missions.sql`).
- Handoff step text for 381/473/478 agrees (handoff pack `SGW_Beta_Site_Complete_Dev_Master_v1.md`, lines 96-106). That is a claim to check, not proof.

Change: none. D-UB15 stays "Decided"; a lab walk of the three offers is UB-90 row 1.

### HELD-B1b: which task consumes item 3399

Verdict: remove 3399 at S1761, on the O2079 hand-in ("Bring the communiqué ... to Colonel Carter"). Chain 23312 already does this (`show 1915`, `unbind 2465/2300`, `remove_item 3399 {"qty":1}`, `next 481 1762`, `bind 2467/2305`). CIMMERIA_AUTHORING.

- Item 3399 is "Communiqué from the Asgard High Council", description "Authorizing you to work under Mimir at the SGC's Beta Site", flags 36864, max stack 1, `moniker_ids '{2641421205}'` (`Items/Seed/items.sql:11994`).
- Flags 36864 = 0x8000 + 4096. 4096 is UNIQUE and 2048 (CAN_BE_DELETED) is absent, so the player cannot delete it. It must be removed by a chain or it sits in the bag for good (`crates/cell-catalog/src/crafting/constants.rs` flag bits).
- No `mission_tasks.sql` row carries an item. All 481 tasks (T2713, T2714, T8598, T4835) are `task_type 1` with no payload (`Missions/Seed/mission_tasks.sql:5771-5777`, S1762's two). Consumption is chain-authored in this engine; the seed cannot decide it.
- O2079 is the only objective tied to the communiqué by its text (`mission_objectives.sql:5287`).
- Executor: `remove_item` goes by item type when the event carries no `instance_id`, and a failed channel send only logs (`crates/cell-content/src/cell/content/executor/inventory.rs:93-209`). A missing item is a base-side outcome and belongs to UB-00a1 Q5.

Change: none.

## Q8. HELD-B5, HELD-B9, HELD-B7

### HELD-B5: 1425 data item

Verdict: the packet's reason is wrong, the default is right. Ship no item for 1425.

- UB-14-16 line 28 says "no data item exists in the seed". That is false. The seed has item 4833 "Science Center Data", "Artifiact research data from the Beta Site Science Lab." (flags 34816, deletable, container `{2}`; `Items/Seed/items.sql:12090`), and 4834 "Rock with Note" (`items.sql:12092`, the Four Pieces family). `Texts/Seed/texts.sql` has the matching moniker `02SGUDatabase_ScienceCenterData` (rows 6724/6725). Items 4833/4834 have `moniker_ids '{}'`, so no item is bound to a step in the data. RESTORED_SOURCE for the item, nothing for the binding.
- Item 4831 is a placeholder ("NO ITEM NAME", `items.sql:12086`). Never grant it.
- No task consumes 4833, and the Omega side that turns it in at S4244 (Jackson) is not in this repo's data. Granting it here without a remove on the other side would strand it.
- A deletable item can be deleted by the player, so a stranded 4833 is not a block, but it is clutter.

Change: reword the line in `UB-14-16.md` (line 28) to "item 4833 exists but no step consumes it; add no item. UB-00a2 Q8". Chains keep their current shape.

### HELD-B9: 1426 camera

Verdict: grant item 4832. CIMMERIA_RECONSTRUCTION for the choice of item, CIMMERIA_AUTHORING for the grant.

- 2766 "Camera" ("A camera to explore areas out of reach", flags 32768) belongs to the Ihpet crater ritual (`items.sql:11780`). Reject it.
- 4830 and 4832 are identical rows: "Digital Camera", "A standard issue digital camera.", flags 34816 (deletable), container `{2}`, max stack 1 (`items.sql:4757` and `items.sql:12088`). `texts.sql` has `MsIt_Fn_BetaE1_03TheFourPieces_DigitalCamera_NAME/DESC` (6718/6719), which ties the camera to the Four Pieces family, hence 1426 "Amateur Photography".
- Which of the two is canonical cannot be proved. 4832 is picked because it sits inside the contiguous 4831-4835 block with the other Four Pieces items (4833, 4834) and 4830 sits alone in an earlier batch. A coin flip with a reason; 4830 stays unused.
- The mission is named for taking photographs. A deletable camera costs nothing and avoids a quest whose title promises a prop.

Change (`UB-14-16.md`, the 1426 table): at the end of chains 23165 and 23166 add `add_item 4832 {"container":0,"qty":1}` (container 0 falls through to the item's `container_sets`, 2; `inventory.rs:52-54`); at chain 23169 add `remove_item 4832 {"qty":1}`. Update the HELD-B9 default text (line 76) from "no item" to "item 4832". Guard the grant with the same one-shot condition those chains already use.

### HELD-B7: 481 S1762 teleporter

Verdict: confirmed. O2080 "Use the teleporter at the Asgard Base to teleport up to Mimir's hovering vessel" is the beam (D-UB12). Only the destinations are open, and UB-00b owns them. CIMMERIA_RECONSTRUCTION. Change: none.

## Q9. M2: dialog 4643 flag 268435456

Verdict: confirmed. The flag changes the indicator glyph only.

- 268435456 is `INT_NON_A_STORY_MISSION_ACTIVE` (2^28; `crates/entity/src/interaction_flags.rs:77-78`). It is a state bit for the NPC's marker.
- Dialog 4643 sits in dsm 5433 with that flag (`dialog_set_maps.sql:5411`; dialog `dialogs.sql:7757`, `dialog_flags 0`). Its neighbours are dsm 5431 (4640, flag 134217728 = side-mission available) and 5432 (4641, flag 0 = offer).
- Beta shows 4643 from chain 23022 (`show 4643`, `unbind 5433/2300`, `complete_mission 1423`; UB-10-13 lines 40-50). The chain shows the dialog directly, so the dsm flag is read only for the marker while the bind is live.
- The two dialect families in this seed (old 8388608/16777216/33554432/1073741824, new 134217728/268435456/536870912) differ only in which glyph is drawn (F13).

Change: none. A lab glance at the marker is UB-90 row 2.

## Q10. Object meshes for templates 2320-2336

Method: read-only. I read the name tables of the cooked packages under `..\SGW\Stargate Worlds-QA\Working\SGWGame\CookedPC\Packages` (the first 3-4 MB of each file, where the names sit) and listed the base names. I changed nothing in the client tree.

What this proves and does not prove: a name present in a package's name table shows the string exists there. It does not show the object's class is StaticMesh, nor how it looks. Every row below is CIMMERIA_RECONSTRUCTION until a lab visual check (UB-90 row 3). The seed already writes meshes as `<Package>.<Mesh>` (for example `GP-Props.SGC-ElevatorButton`, `Em-Props.EM-ShelfBox13`).

Body sets present in `GLB_Components`: `WorldObject_Small`, `WorldObject_WallTerminal`, `WorldObject_StandingTerminal_High`, `_Med`, `_Low`. Floor props take `WorldObject_Small`; consoles take a standing terminal set.

Rule used: the Beta Site is an SGC (Earth) facility, so human objects draw from `SGC_Props` and `Em-Props`; Asgard objects draw from `ASG-Props` and `GP-Props`.

| Template | Object | static_mesh | body_set | Note |
|---|---|---|---|---|
| 2320 | Xenobiology Sample Cage | `Em-Props.EM-Fauna_Cage00` | `GLB_Components.WorldObject_Small` | alt `Em-Props.EM-HoldingTank00` |
| 2321 | Artifact Display Case | `LUS-Props.LUS-DisplayCase00` | `GLB_Components.WorldObject_Small` | alt `JF-Props.JF-Display00` |
| 2322 | Beta Site Hub Computer | `SGC_Props.SGC-console05` | `GLB_Components.WorldObject_StandingTerminal_Med` | |
| 2323 | Admin Hub Workstation | `SGC_Props.SGC-console01` | `GLB_Components.WorldObject_StandingTerminal_Low` | alt `SGC_Props.SGC-Table_Desk00` |
| 2324 | Science Center Computer | `SGC_Props.SGC-console02` | `GLB_Components.WorldObject_StandingTerminal_Med` | |
| 2325 | Wrecked SGC Shipping Crates | `Em-Props.EM-Wooden_Crate_open00` | `GLB_Components.WorldObject_Small` | no wrecked crate mesh found; an opened crate is the closest. alt `Em-Props.EM-SupplyCrate01` |
| 2326 | Box of Jaffa Tools | `GA-Props.GA-AmmoCrate00` | `GLB_Components.WorldObject_Small` | alt `GA-Props.GA-JewelryBox00` |
| 2327 | reserved (Accelerant, UB-52) | none | none | not created |
| 2328 | Asgard Fire Suppressant Manager | `ASG-Props.ASG-ComputerConsole01` | `GLB_Components.WorldObject_StandingTerminal_Low` | |
| 2329 | Fire Control Hub | `Em-Props.EM-SGC_Failsafe_Console00` | `GLB_Components.WorldObject_StandingTerminal_Med` | a human manual control; alt `GA-Props.GA-CtrlConsole00` |
| 2330 | Environmental Reading Device | `GP-Props.GP-Asgard_Terminal_Console00` | `GLB_Components.WorldObject_StandingTerminal_Low` | do not use the misspelt `GP-Agard_Terminal_Console00` |
| 2331 | Analytical Assistance Machine | `ASG-Props.ASG-ComputerConsole03` | `GLB_Components.WorldObject_StandingTerminal_Med` | |
| 2332 | Missing Asgard Hologram Emitter | `ASG-Props.ASG-Hologram_Platform00` | `GLB_Components.WorldObject_Small` | |
| 2333 | Disabled Malfunctioning Drone | NULL | `MOB_Goauld_Drone` body set from template 69 | unchanged from UB-02a (class `being`, id-pinned). `ASG-Props.AG-Drone_Sta_00` exists as a static drone; use it only if the body draw misbehaves |
| 2334 | Consciousness Simulator | `ASG-Props.ASG-Hologram_Console00` | `GLB_Components.WorldObject_StandingTerminal_Med` | |
| 2335 | Note at the ruins | `SGC_Props.SGC-paper_holder` | `GLB_Components.WorldObject_Small` | nearest paper prop; keep 445's look if it renders badly |
| 2336 | reserved (UB-52) | none | none | not created |

Default kept for any template whose pick fails the visual check: 445's `GLB-Global.GLB-RingTransporterSwitch_00` with `GLB_Components.WorldObject_WallTerminal`.

Change: in `UB-02a.md` (the template table, lines 69-83) add the two columns above. Rows 2327 and 2336 stay "reserved and not created".

## Q11. Mission rows

Verdict: confirmed. No `missions.sql` edit is needed.

All 17 Beta missions (1423, 381, 473, 478, 1425, 1426, 1427, 1500, 1502, 1503, 481, 420, 421, 1147, 443, 477, 479) have `reward_xp 0`, `reward_naq 0` and `script_name NULL` (`Missions/Seed/missions.sql:71, 433-439, 493-499, 1221, 1295-1301, 1335-1351`). Their `award_xp` column is a boolean `true`, not a quantity, so D-UB8 (xp 0 / naq 0) already holds. `mission_reward_groups.sql` has no rows for any of them, so no item reward fires either.

Levels, for reference: 1423=6, 381=6, 473=6, 478=8, 1425=8, 1426=8, 1427=10, 1500=5, 1502=6, 1503=7, 481=6, 420=6, 421=6, 1147=6, 443=7, 477=6, 479=7. RESTORED_SOURCE.

Two facts the coders need:

- The SGU family (1423, 1425-1427, 1500, 1502, 1503) has every step `step_enabled` false and every objective `is_enabled` false. Also disabled: O6325 (S4243), O6326 (S4245), O6327 (S4248); O1630 (381 S1408), O1725 (443 S1486), O2070 (479 S1750), O4235 (479 S3644). Chains that advance these steps work in the engine, but a client that filters on `is_enabled` may hide the entries. That is UB-90 row 4.
- S4248 has two tasks (T7626, T7627) on O4919. Every other objective has exactly one task.

## Q12. HELD-B15: 477 subject bind

Verdict: no interaction-only row exists for 477. Keep UB-35's default of binding dsm 2457 on the four subject slots.

- Set 500 ("Covert Observations") has exactly three rows: 2456 (dialog 1997, flag 0, the offer), 2457 (dialog 1999, flag 134217728, the blurb) and 4408 (dialog 1996, flag 536870912, completion) (`Dialogs/Seed/dialog_set_maps.sql:1351, 1353, 4099`). None has `dialog_id` NULL. RESTORED_SOURCE.
- NULL-dialog rows do exist for other sets (2348 in set 472, 4582 in set 473, 4592 in set 1174, 2313 in set 465, 5527 in set 502), so the pattern is real in the seed. 477 has none. Authoring one would be a seed edit (a new dsm id) and is outside this packet; D-UB15 prefers fewer seed edits.
- Consequence for UB-35: binding 2457 gives the subject a clickable bind and a blurb dialog on click. The blurb text is Carter's observation briefing, not a subject-specific line. UB-35 must show the subject's own dialog by chain on the tap, and unbind 2457 when its objective completes, so the blurb does not re-show (trap 4).

Objective-to-subject map versus `mission_tasks.sql`: no disagreement. O2060 Carter ("Central Admin"), O2061 Debeaux ("Traning Range"; the seed misspells Training), O2062 Felger ("Admin Storage") and O2063 Satterfield ("Artifact Storage") map to T2678-T2681 in order (`mission_objectives.sql:5315-5321`, `mission_tasks.sql:5801-5807`). The tasks carry no subject data at all, so the mapping lives only in the objective texts. The subject for each objective is therefore chain-authored. CIMMERIA_RECONSTRUCTION.

Change: none. If the owner wants the glyph-only bind, the alternative is a new interaction-only dsm for set 500 with the flag 134217728; that goes to UB-90 row 5, not to this wave.

## Q13. HELD-B14: 443 S1484 transmitter dialog

Verdict: none exists. UB-34 keeps the step update as the feedback.

- Speaker 763 appears on exactly two dialog screens in the seed: dialog 3722 (screen 49408, the hologram emitter, "Hidden behind some medical supplies you find a small crystalline object") and dialog 3941 (screen 50816, "Hidden behind some equipment you find a strange crystalline emitter ... report this directly to Colonel Carter") (`Dialogs/Seed/dialog_screens.sql:15721, 16270`). Neither is a radio call.
- 3722 belongs to mission 421 (UB-30-36 chain 23365). 3941 is dsm 2318 in set 465 "Malfunction", flag 1073741824 (`dialog_set_maps.sql:1195`), a drone-malfunction object dialog. Neither set is 443's.
- 443's own set is 473 "Incendiary". Its rows are 2352 (dialog 1918, Mimir #2), 3728 (3895), 3780 (3161), 4579 (1917, flag 33554432), 4581 (1919, flag 8388608) and 4582 (NULL, flag 1073741824). Speaker 730 "Mimir #2" speaks dialog 1918; no row is a transmitter line. Speaker 763 has a blank name, so it is not a usable Mimir label either.

Change: none. UB-34 chain 23412 stays as written.

## Q14. 481 S1762: O2080 is not the last required objective

Verdict: confirmed. Chain 23316 (`complete_objective 481 2080`) cannot complete the mission by itself.

- S1762 has two objectives: O2080 (task T8598) and O4015 "Converse with Mimir" (task T4835). Both are `is_enabled` true and neither is optional (`mission_objectives.sql:5321` area and `mission_tasks.sql`). RESTORED_SOURCE.
- Completing O2080 leaves O4015 open, so F19 (the last objective completes the mission) does not fire. Mimir's conversation completes O4015 and then the mission.
- No 481 task carries an item or a parameter (see Q7).

Change: none.

## Changes for the orchestrator to copy into packets

| # | File | Row | Change |
|---|---|---|---|
| 1 | `UB-14-16.md` | line 28 (HELD-B5) | reword: item 4833 exists, but no step consumes it; add no item |
| 2 | `UB-14-16.md` | line 76 (HELD-B9) | default becomes item 4832 |
| 3 | `UB-14-16.md` | chains 23165, 23166 | append `add_item 4832 {"container":0,"qty":1}` |
| 4 | `UB-14-16.md` | chain 23169 | append `remove_item 4832 {"qty":1}` |
| 5 | `UB-02a.md` | template table, lines 69-83 | add `static_mesh` and `body_set` columns from Q10 |
| 6 | `beta-work-packets.md` | HELD table rows B1, B1b, B5, B9, B14, B15 | mark answered, point at this file |

No other packet needs a change.

## UB-90 rows (need the live client)

| # | Check | Why |
|---|---|---|
| 1 | Walk the three Human offers 1423 to 381 to 473 to 478 | order is authored, not sourced (Q7) |
| 2 | Look at Carter's marker while 4643 is shown | M2 says glyph only (Q9) |
| 3 | Spawn each Q10 template and compare the draw with the object name | the name table cannot prove class or look |
| 4 | Open the log after a chain advances an `is_enabled`-false SGU objective | client filtering is unknown (Q11) |
| 5 | Click each of the four 477 subjects with dsm 2457 bound | confirms the blurb-bind default (Q12) |
| 6 | Take item 4832 into the bag, then finish 1426 | confirms grant and removal (Q8) |
