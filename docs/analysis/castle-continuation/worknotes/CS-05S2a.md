# CS-05S2a worknote: return-hub services re-seal

> Type: worknote. Packet CS-05S2a of the Castle continuation R3b set. Branch `castle/cs-05s2a-hub-services`, worktree `castle-cs-05s2a`, base `224e7692e`.
> Written 2026-10-18. The spawn and placement leg (agent 1) ran `nav_inspect` and the castle_navmesh test; no lab, no server, no seeds, no code. The `## Economy` and `## Authority` legs are written (agents 2 and 3).
> This closes the CS-05S gap "spot not measured". Services are sealed for the quartermaster; see "## Services verdict".

## Spawn-leg draft verdict (superseded)

Draft written by agent 1 before the Economy and Authority legs. The final verdict is "## Services verdict" near the end of this note: quartermaster SEALED, medic BlockedEvidence and dropped. Where a spawn-leg section below still names the medic (template 414, spawn 13803), the final sections win.

## Spawn and placement

### What the engine can do (service type)

- The only service types a template can carry are the `NpcInteractionType` variants (`crates/entity/src/cell_entity/mod.rs:130`): Dialog, Vendor, Trainer, Loot, Banker, Auctioneer. There is **no heal service**. A "medic" is therefore a Vendor NPC with a medic name and body (medical stock is Agent 2's call). Repair and recharge are fields of the same vendor store (`repair_item_list`, `recharge_item_list`).
- Vendor = `interaction_type 65536` (`INT_VendorGeneral`) plus a `buy_item_list`. Template 300 `Debug Hub - Vendor` is the working precedent (right-click answers `OpenVendorStore`, `debug_hub_dispatch_tests.rs`).
- No stock template fits. The three templates with a vendor list are 25 (`Interaction Debug NPC - DO NOT USE`), 300 and 314 (Debug Hub). All three are debug fixtures; none is a Castle actor. The world-8 rows (105 spawnlist rows, 35 templates, per the CS-05S review) all have NULL lists. Verified again at `224e7692e`: only templates 25, 300, 314 carry any of the four list columns.
- So two **new** `entity_templates` rows and two new `spawnlist` rows are frozen below. Friendly faction 1: reaction row is FRIENDLY to every faction, so a faction-1 vendor never fights and no hostile attacks it (#1009 rule); `faction != 10` also means a player cannot damage it. The vendor is not killable, so it needs no `respawn_secs`.

### The two spots (nav_inspect, `data\spaces\castle.nav`)

Command: `pwsh tools/build-lane/lane.ps1 cargo run -p cimmeria-navmesh-extractor --bin nav_inspect -- data\spaces\castle.nav --probe NAME=X,Y,Z --v-tol 0.05` (exit 0 ok, 2 off tolerance, 3 another component). `nav_inspect` prints the component only in the probe table; component values below are from the lane logs.

| Role | Probe | Result | Component | Notes |
|---|---|---|---|---|
| Medic vendor | `MED_D2=348,70.31,891` | exit 0, h 0.00 m, dy 0.00 m (v-tol 0.05) | 250 | Infirmary, south end of the Copplemann corridor (region 2051). Ogilvie (362,70.18,886) probes as component 250, poly 14205. |
| Quartermaster | `QM=440,70.22,992` | exit 0, h 0.00 m, dy +0.04 m (v-tol 0.05) | 250 | Armory, 10 u west of the armory soldiers; Gerschon at (429.6,996.6) is 11.4 u away. |

Rejected probes (so the next reader does not retry them): `MED_D=348,70.2,891` and `MED_E=346,70.2,886` exit 2 only because y was 0.11 and 0.17 under the floor at `--v-tol 0.05` (floors are 70.31 and 70.37 there; both still component 250); `MED_C=348,70.2,879` exit 2, dy +13.82 (lands on a different floor poly 13908); `MED_H=348,70.31,889` and `MED_F`/`MED_G` (z 880) snap 1.9 m / 0.8 m off the poly, so they are not exact. `castle_navmesh` (`lane.ps1 cargo nextest run -p cimmeria-entity --test castle_navmesh`): 7 passed at the base; it checks the existing world-8 rows, and CS-05a's new rows extend it.

### Component and route

- Both spots are component **250**, the same as Ogilvie, the Infirmary tank (13802), the Armory respawner "Armory Respawn" (466.4,70.4,991.5; probe exit 0, 250) and the "Op-Core Triangle Respawn" (371.0,70.15,904.6; 250). A player arriving by the Armory ring or dying to either respawner can walk to both vendors on the navmesh.
- **Boundary finding (not a defect of these spots):** the Castle DHD (806.27,55.10,517.24) and "Checkpoint Alpha Respawn" (800,55.21,513) are component **116**, not 250. A player who lands from Harset at the DHD (or respawns at Alpha) has no walkable route to either vendor; the Alpha-to-Armory walk is the open G-ROUTE row (W-2 in `uat/CS-01-walk.md` is Armory to DHD). The hub is reachable from the Cellblock-arrival side only. CS-05a/CS-05c must not assume a Harset-return player reaches the hub on foot. This is recorded for the end UAT (row S1 below).
- A `find_path` replay (not just component equality) is owed to CS-05a; component equality is what the R2 tank seal accepted (`evidence/g-sym.md`).

### Aggro clearance

Rule: horizontal distance minus aggro radius minus wander radius at least 5 u for every faction-10 world-8 spawn (aggro default `DEFAULT_AGGRO_RADIUS` 18.0 u, vertical band `AGGRO_VERTICAL_BAND` 4.0 u, `crates/cell-world/src/cell/combat/aggression.rs:24,36`; template `aggro_radius` overrides). Computed over all 64 faction-10 world-8 spawn rows (re-derived at review from the seeds; world 8 has 105 rows: 27 faction 1, 9 faction 3, 64 faction 10, 5 NULL). No hostile has a patrol path or wander radius within range; the faction-10 rows near the hub have `wander_radius` NULL. Faction 3 (Praxis) and NULL-faction rows are not hostile to players (`faction_reaction.rs`, `HOSTILE_FACTION` is 10).

| Spot | Nearest hostile (spawn, tag) | Distance | Aggro | Margin over aggro + 5 |
|---|---|---|---|---|
| Medic (348,891) | 90 `Castle_PRU1` (364.8,921.6) | 34.9 u | 18 | +11.9 (margin 16.9 over aggro) |
| Medic | 115 `Castle_NidGuard17Inside` (326.4,933.5) | 47.7 u | 18 | +24.7 |
| Medic | 259 `Castle_Pop_Symbiote_3` and 116 `CastleNidGuard18Inside` | 53.9, 54.0 u, 14.9 u higher (outside the 4 u band) | 18 | well clear |
| Quartermaster (440,992) | 89 `CastleNidGuardXInside` (381.8,997.2) | 58.4 u | 18 | +35.4 |
| Quartermaster | 257 `Castle_Pop_Symbiote_1` | 75.0 u (14.8 u higher) | 15 | clear |
| Quartermaster | 114 `Castle_PRU5` (359.8,983.1) | 80.7 u | 18 | clear |

Assist radius (default 10 u) matters only for neighbours of an attacked hostile; no hostile is within 34 u of either spot. Hostile respawn radius: hostile spawns are static rows with `respawn_secs` only after a kill; none is closer than 34.9 u, so no respawn point enters either spot's 23 u (aggro + 5) circle. Friendlies near the spots (medic 198 at 7.2 u, sick 199 at 7.6 u, dispenser 13801 at 8.1 u, tank 13802 at 9.4 u from the medic spot) are all faction 1 and need no clearance; the nearest is more than 3 u away, so no overlap.

### Frozen rows (placement part; stock columns are Agent 2's)

Only the quartermaster rows (template 415, spawn 13804) are part of the freeze; their final form is under "## Frozen SQL rows (for CS-05a)". The medic inserts (414, 13803) below are kept for the record with their `<buy>` placeholders. They are not frozen and must not be copied.

IDs follow the CS-00Be precedent (templates 412/413, spawns 13801/13802 are the Castle's own, outside every reserved block). **Coordinator confirms** both pairs at CS-05a dispatch (D-CA19 note).

- Template ids **414, 415**: absent from the `entity_templates` first column on `224e7692e`, `content/cellblock-campaign`, `content/harset-campaign` and `content/tollana-campaign` (structured parse of every `db/resources/**/*.sql` at each ref; template ids defined: 451, 449, 449, 450; max 1595 on all four; none is 414 or 415). `git grep -c -w -E "414|415" <ref> -- db crates tools docs/content` returns the same per-file counts on all four refs, and every hit is another namespace: floats and node indices in `AI/Seed/cover_nodes.sql` (26) and `cover_sets.sql` (5), `Entities/Seed/blueprints_components.sql` (11) and `blueprints.sql` (2); Dialog ids (`dialog_screen_buttons` 2, `dialog_screens` 5, `dialog_set_maps` 7, `dialog_sets` 2, `dialogs` 2, `speakers` 2); Mission ids (`missions` 2, `mission_steps` 9, `mission_objectives` 4, `mission_tasks` 4); `char_creation_choices` 2, `effect_nvps` 2, `items_event_sets` 2, `crafting_item_effects` 1; and in the two seeds this packet edits: `spawnlist.sql:519` (the z of `Castle_Muelbach`, 414.0), `:1106` and `:1155` (comments), `:1170` (**spawn_id** 414, `CraftHub_Supplies`, world 12, template 314) and `entity_templates.sql:1034` (a comment). Spawn id 414 is a different column from template id 414; no `spawnlist.template_id` is 414 or 415. In `crates/` the hits are numerics (launcher, admin-api, upk, navmesh-extractor), `castle_navmesh.rs:43` (z 414.0) and debug-hub comments. The strict "above max" form is 1596/1597 (max 1595, the Debug Area lineup); 414/415 are used instead because the max is a different campaign's block and 412/413 set the Castle precedent.
- Spawn ids **13803, 13804**: absent from `spawnlist` on all four refs (spawn max 14095 on three refs, 19004 on tollana, whose 19001-19004 are Tollana rows). `git grep -c -w -E "13803|13804" <ref> -- db crates tools docs/content` hits only `db/resources/Texts/Seed/texts.sql` (2, moniker ids) and `crates/names/src/namebook_gaps.txt` (1, a list of unused text ids) on all four refs. The strict "above max" form would be 19005+, which sits inside Tollana's 19xxx range, so it is not used.
- Name ids: 6964 `Castle Medic` and 7153 `Op-Core Quartermaster` are non-empty strings in `texts.sql` (not the empty-moniker trap, memory `castle-world8-layout-and-authoring`). Moniker 7153 is named `DN_NPC_VN_OpCoreQuartermaster_Tollana`, a Tollana-side string reused for the Castle; the displayed text is generic.

```sql
-- Medic vendor (CS-05S2a, CIMMERIA_AUTHORING): clone of template 177 with interaction_type 65536. Lists are Agent 2's.
INSERT INTO entity_templates (template_id, static_mesh, body_set, components, flags, interaction_type, event_set_id, level, alignment, faction, name_id, name, patrol_path_id, patrol_point_delay, template_name, class, buy_item_list, sell_item_list, repair_item_list, recharge_item_list, ability_set_id, ammo_type, loot_table_id, primary_color_id, secondary_color_id, skin_tint, weapon_item_id, static_interaction_sets, trainer_ability_list_id, speaker_id, has_dynamic_properties, interaction_set_id, move_speed) VALUES (414, NULL, 'BS_HumanFemale.BS_HumanFemale', '{AR_Global.Prisoner_Boots,AR_H_Clothing00.AR_HM_PL1_PL100AB100,AR_H_Clothing00.AR_HM_PT1_PT100,BS_HumanFemale.BS_HF_Base_Boots00_00,BS_HumanFemale.BS_HF_Base_Hands00_00,BS_HumanFemale.BS_HF_Base_Legs00_00,BS_HumanFemale.BS_HF_Base_Torso00_00,BS_HumanFemale.BS_HF_Boots_00,BS_HumanFemale.BS_HF_Hair_01,BS_HumanFemale.BS_HF_Head_00}', 0, 65536, 570, 4, 0, 1, 6964, NULL, NULL, NULL, 'Castle_Medic_Vendor', 'mob', <buy>, <sell>, <repair>, <recharge>, NULL, NULL, NULL, -256, -256, -52773120, NULL, '{}', NULL, NULL, true, NULL, NULL);
-- Quartermaster (CS-05S2a, CIMMERIA_AUTHORING): clone of template 176 with interaction_type 65536. Lists are Agent 2's.
INSERT INTO entity_templates (template_id, static_mesh, body_set, components, flags, interaction_type, event_set_id, level, alignment, faction, name_id, name, patrol_path_id, patrol_point_delay, template_name, class, buy_item_list, sell_item_list, repair_item_list, recharge_item_list, ability_set_id, ammo_type, loot_table_id, primary_color_id, secondary_color_id, skin_tint, weapon_item_id, static_interaction_sets, trainer_ability_list_id, speaker_id, has_dynamic_properties, interaction_set_id, move_speed) VALUES (415, NULL, 'BS_HumanMale.BS_HumanMale', '{AR_Global.Prisoner_Boots,AR_H_Clothing00.AR_HM_PL1_PL100,AR_H_Clothing00.AR_HM_PT1_PT100,BS_HumanMale.BS_HM_Boots_00,BS_HumanMale.BS_HM_Feet_00,BS_HumanMale.BS_HM_Hands_00,BS_HumanMale.BS_HM_Head_02,BS_HumanMale.BS_HM_Legs_00,BS_HumanMale.BS_HM_Torso_00}', 0, 65536, 570, 4, 0, 1, 7153, NULL, NULL, NULL, 'Castle_Quartermaster', 'mob', <buy>, <sell>, <repair>, <recharge>, NULL, NULL, NULL, -256, -256, -52773120, NULL, '{}', NULL, NULL, true, NULL, NULL);
-- Spawns (world 8, component 250, floor y from nav_inspect, stationary: no wander, no patrol).
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13803, 348.0, 70.31, 891.0, 1.5708, 8, 414, 'Castle_HubMedic', NULL, true);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13804, 440.0, 70.22, 992.0, 1.5708, 8, 415, 'Castle_HubQuartermaster', NULL, true);
```

The `<buy>`, `<sell>`, `<repair>` and `<recharge>` slots are list ids only Agent 2 may fill (existing lists only). If Agent 2 returns no usable list for a role, that role's two rows are dropped, not left NULL (a vendor template with a NULL `buy_item_list` opens an empty store).

### Dialog and chain ids (D-CA19, reserved, none used by this leg)

A vendor needs no dialog: `NpcInteractionType::Vendor` answers right-click directly. The reserved sub-ranges stay unused unless a greeting is wanted: chains 10220-10229, dialogs 10750-10759, dialog sets 10790-10799, screens 10890-10899, buttons 10990-10999. This leg freezes no chain, dialog, set, screen or button row.

### CS-05a test list (spawn part, superseded by "## CS-05a test list (final)")

The final list drops the medic (item 1 here names 13803) and fits CS-05a's allowlist.

`live_db_castle_hub_service_is_seeded_safe_and_stocked` in `crates/cell-catalog/src/cell/spawner/tests/live_db_castle_hub.rs` (new) should check, for both spawns:

1. spawn 13803/13804 exists in world 8 at the frozen transform;
2. template 414/415 has `interaction_type 65536`, faction 1 and the sealed lists;
3. no faction-10 world-8 spawn lies within `aggro_radius + wander_radius + 5` horizontally (the margins above are 11.9 u and 35.4 u over that line, so the test has real slack);
4. both spawns are `is_stationary`.

`castle_navmesh` picks up the rows by itself only if they sit in `db/resources/Worlds/Seed/spawnlist.sql`: its `every_castle_spawn_is_on_the_mesh` reads that one file and lines starting `INSERT INTO spawnlist` (`castle_navmesh.rs:136-148`). CS-05a must put the spawn row there (and the template row in `Entities/Seed/entity_templates.sql`, where 412 and 413 are). Spawn the rows through `spawn_npc_from_record` (as `debug_hub_dispatch_tests.rs` does) and assert the interaction type resolves to `NpcInteractionType::Vendor`; that is the regression the debug hub learned.

### End-UAT rows (spawn part, superseded by "## End-UAT rows")

Rows S1 and S3 below name the medic; the final table replaces them.

| Row | Expectation | State |
|---|---|---|
| S1 | A character arriving at the Castle Armory walks to the medic at the Infirmary south end and right-clicks it: the vendor window opens. | UATPending (end UAT, D-CA11) |
| S2 | Same at the quartermaster in the Armory. | UATPending (end UAT, D-CA11) |
| S3 | Both are untargetable for attack (no weapon damage lands) and no hostile pulls toward either while the character stands at the spot. | UATPending (end UAT, D-CA11) |
| S4 | A character landing at the DHD (component 116) cannot reach the hub on foot; this is expected, not a bug, until the Alpha-to-Armory route is proven. | UATPending (end UAT, D-CA11) |

## Economy

> Agent 2 (items-systems-advisor). Provenance: [seed] = read from `db/resources` at this worktree (base `224e7692e` plus the spawn leg's one file); [code] = read from `crates/`; [hand] = computed by hand from the seed; [unexecuted] = not run against a live database (the packet allows no seeds, no DB and no compile).
> Rule from Agent 1: existing item lists only. No `item_lists`, `item_list_items` or `item_list_prices` row is added by this section.

**Economy verdict:** `Quartermaster (template 415): SEALED on existing lists (buy 1300, sell 2, repair 2, recharge 2). Medic (template 414): BlockedEvidence, no existing list carries medical stock; its two rows are dropped.`

### The lists that exist

| Item list | Name | Rows | Used by (as) | Fit |
|---|---|---|---|---|
| 1 | Test vendor buy list | item 1: 5228 Processor (Electronics) 100; item 3: 5192 Cell (Bio-Medical) 0 naq plus one design-55 pistol from `item_list_prices`; item 7: 3241 SI 3 9mm Pistol 50 | 25, 300 (buy) | Test fixture. Contains a pistol-for-crafting-cell swap. Not for a campaign vendor. |
| 2 | Test vendor sell/repair/recharge list | item 4: 21 SGHC 6 SMG 1000; item 5: 55 SI 3 9mm Pistol 300; item 6: 3437 BDU Pants 100 | 25, 300, 1302 (sell, repair, recharge) | The only list any template uses as a sell, repair or recharge list. |
| 310 | Debug hub crafting supplies | 24 crafting resources, 1 naq each (items 3101-3124) | 314 (buy) | Crafting campaign list. Not a service. |
| 1300 | Debug Area munitions | 19 rows, items 13001-13019 (below) | 1302 (buy) | The only list with weapons, ammo and a medical consumable. |

[seed] `db/resources/Items/Seed/item_lists.sql`, `item_list_items.sql`, `item_list_prices.sql`, `item_lists_debug_area_plaza.sql`. Templates that carry any of the four list columns, by a structured parse of all 451 `entity_templates` inserts under `db/resources` (the three Debug Area NPC rows that a naive parse flags have a different column list; their numbers are colour columns): **25, 300, 314 and 1302**. Correction to the placement leg: it counted 25, 300 and 314; template 1302 (`entity_templates_debug_area_plaza.sql:43`, buy 1300, sell 2, repair 2, recharge 2) exists at `224e7692e` too. This does not change the placement.

### Medic (template 414, spawn 13803): dropped

- The only medical consumable in the seed is design 2893 Health Slappack TC1 (+500 Health). It appears in exactly one list, 1300, as row 13016 (quantity 5 for 1 naq) [seed], among 15 special-ammo stacks and three weapons. A medic on list 1300 would sell an SMG and 100-round armor-piercing stacks. That is a quartermaster, not a medic.
- Lists 1, 2 and 310 hold no consumable. Design 5192 is a crafting resource ("Cell (Bio-Medical)", containers 17 and 15), not a heal item.
- With the existing-lists-only rule there is no medic stock to freeze. Per Agent 1's rule the template 414 and spawn 13803 rows are dropped, not left with NULL lists (a vendor with a NULL `buy_item_list` opens an empty store). Placement of spot `MED_D2` stays valid should the medic return.
- **Option (owner decision, not frozen):** a medic stays if a new list is allowed. The minimal shape is one `item_lists` row plus one `item_list_items` row (design 2893, quantity 5, any price at or above 1; no sell list pays for design 2893, so no price can arbitrage). The ids must come from the coordinator: the `item_lists_item_list_id_seq` floor is 329 and `item_list_items` runs to 13019. A new list also needs its own `live_db` seed-sequence check (`live_db_seed_sequences.rs`).

### Quartermaster (template 415, spawn 13804): frozen stock

Substitute in the placement leg's template 415 insert: `<buy>` = **1300**, `<sell>` = **2**, `<repair>` = **2**, `<recharge>` = **2**. These are the same four values as template 1302, the Debug Area munitions vendor [seed `entity_templates_debug_area_plaza.sql:43`], which `live_db_vendor_arbitrage` already guards. No other value is used.

Buy list 1300, in the order the store shows (item_id order; the wire addresses a row by this index) [seed `item_lists_debug_area_plaza.sql`]:

| item_id | Design | Item | Qty | Naquadah |
|---|---|---|---|---|
| 13001 | 9000 | Armor Piercing Rounds | 100 | 1 |
| 13002 | 9001 | Hollow Point Rounds | 100 | 1 |
| 13003 | 9002 | Incendiary Rounds | 100 | 1 |
| 13004 | 9003 | EMP Rounds | 100 | 1 |
| 13005 | 9004 | Explosive Rounds | 100 | 1 |
| 13006 | 9005 | Poison Darts | 100 | 1 |
| 13007 | 9006 | Disease Darts | 100 | 1 |
| 13008 | 9007 | Tranquilizer Darts | 100 | 1 |
| 13009 | 9008 | EMP Darts | 100 | 1 |
| 13010 | 9009 | Radioactive Darts | 100 | 1 |
| 13011 | 9010 | Stim Darts | 100 | 1 |
| 13012 | 9011 | Coagulant Darts | 100 | 1 |
| 13013 | 9012 | Nanite Darts | 100 | 1 |
| 13014 | 9013 | Antidote Darts | 100 | 1 |
| 13015 | 9014 | Adrenaline Darts | 100 | 1 |
| 13016 | 2893 | Health Slappack TC1 | 5 | 1 |
| 13017 | 55 | SI 3 9mm Pistol | 1 | 300 |
| 13018 | 21 | SGHC 6 SMG | 1 | 1000 |
| 13019 | 3584 | CO2 Pistol Dartgun | 1 | 1 |

None of the 19 rows has an `item_list_prices` row (the one price row in the seed belongs to list 1 item 3). Sell, repair and recharge list 2: design 21 for 1000, 55 for 300, 3437 BDU Pants for 100 [seed `item_list_items.sql`]. Repair and recharge price a full restore at the list price and scale by missing durability, minimum 1 naq [code `vendor/paid_repair/mod.rs`: `GREATEST(naquadah * (100 - durability) / 100, 1)`].

The ammo and darts are an authoring choice carried over from the Debug Area: the special-ammo reserve is drawn by a reload only once `ammo.finite_special` is on (header of `item_lists_debug_area_plaza.sql`).

### No-arbitrage check against `live_db_vendor_arbitrage` [hand, unexecuted]

The test (`crates/cell-catalog/src/cell/spawner/tests/live_db_vendor_arbitrage.rs`) joins every list used as some template's `buy_item_list` to every list used as some template's `sell_item_list` on `design_id`, skips buy rows that also cost items, and fails when `b.naquadah < s.naquadah * b.quantity`.

1. Sell side after template 415: still only list 2 (designs 21, 55, 3437). Buy side after template 415: lists 1, 310, 1300 (415 reuses 1300).
2. Designs shared by a buy list and list 2: list 1 {5228, 5192, 3241} has none. List 310 {5254, 5256, 5401, 5192, 5189, 5481, 5668-5671, 5369, 8415, 8402, 8441, 8405, 8461, 8406, 8451, 7805-7809, 6483} has none. List 1300 shares 55 and 21.
3. List 1300, design 55: buy 300 for quantity 1; list 2 pays 300 x 1 = 300. `300 < 300` is false. Pass, margin 0.
4. List 1300, design 21: buy 1000 for quantity 1; list 2 pays 1000. `1000 < 1000` is false. Pass, margin 0.
5. Adding template 415 adds no new (buy list, sell list) pair: the query selects `DISTINCT` list ids, and 1300 x 2 already exists through template 1302. The control (`pairs > 0`) stays satisfied by the same two designs.
6. Buyback: a sale writes the sale price into the buyback row's `flags` and a buyback charges `unit_price * quantity` from it [code `vendor/buyback/mod.rs:110,196`], so sell then buy back is net 0. Repair and recharge only cost cash. A sale pays the flat list price at any durability [code `vendor/sell/mod.rs`, `ili.naquadah AS unit_price`], but every purchase from 1300 delivers durability 100 at a price at or above that sell price, so no buy-sell loop gains cash.

The margin is exactly zero for the pistol and the SMG. Any later repricing of rows 13017 or 13018 below 300 or 1000, or of list 2 above them, breaks the guard. This is by design (DA-02 review F1).

### Faucets and drains to hand to the owner (not arbitrage, not blockers)

- **Starter-kit sale (one-off, at most 400 naq, only for characters that hold the items).** Not every character does. The universal starter pistol was removed from the canonical profiles (header of `db/resources/Archetypes/Seed/char_creation_items.sql`, OD-CS01): design 55 is granted only to char_defs 9, 10 and 19 (PRA_GOAULD and SGU_ASGARD; bandolier, container 3). Design 3437 BDU Pants comes from the `AR_Global.Prisoner_Legs` visual choice (10 rows in `char_creation_choices.sql`, all `item_bound false`; container 11), so only characters that chose it hold them. Both designs carry flag bit 1024 (can be sold) and not bit 4 (bind on acquire): items.sql flags 36224. The sell filter accepts containers `[1,3..15]` and `bound = false` [code `vendor/mod.rs:9`, `vendor/sell/mod.rs:124-128`], so such a character can sell the pistol for 300 and the pants for 100 at list 2's price. This is already true at vendors 25, 300 and 1302. It is not a loop (the pistol re-buys at 300).
- **Near-free consumables.** A Slappack stack (5 items, +500 Health each by name) costs 1 naq, and 100-round special-ammo stacks cost 1 naq each. A Castle quartermaster therefore doubles as a free heal source and special-ammo faucet. Accepting the Debug Area pricing keeps the economy guard simple and needs no new list; the alternative is a Castle-specific list with real prices, which this packet's rule excludes. Recommend accept for the hub, owner to confirm.
- **Purchases land in the main bag and ignore `max_stack_size`** (header of `item_lists_debug_area_plaza.sql`). Bag-full and cash refusals belong to the Authority leg.

### CS-05a tests (economy part, superseded by "## CS-05a test list (final)")

1. `live_db_castle_quartermaster_lists_match_the_sealed_stock` (new, beside `live_db_castle_hub.rs`): template 415 has `(buy, sell, repair, recharge) = (1300, 2, 2, 2)` and list 1300 has 19 rows. Bug shape: a NULL list opens an empty store, and a list swapped for list 1 sells a free test item. Fails when the template inserts revert to NULL or the `<buy>` slot takes list 1.
2. `live_db_no_buy_row_is_cheaper_than_a_sell_back` (existing, must stay green with the new rows loaded). It is the arbitrage guard; no new test duplicates it.
3. A vendor round trip against template 415 through the existing purchase and sell test helpers: buy the pistol (300), sell it back (300), assert the player's cash is unchanged. Fails if row 13017 is repriced under 300.
4. Template 414 and spawn 13803 are absent: assert no `entity_templates` row 414 and no `spawnlist` row 13803, so the dropped medic cannot come back with NULL lists.

### Open items for Agent 3, the coordinator and the owner

- Agent 3: spawn leg expectations S1 and S3 (medic) and CS-05a test list item 1 (`13803/13804`) must drop the medic, or keep it once an owner-approved list exists.
- Coordinator: confirm 415 and 13804 (ids), and decide whether to allocate a medic list (see option above).
- Owner: accept Debug Area pricing (free heal and ammo, a starter-item sale of at most 400 naq) in the Castle hub.
- Not run: `live_db_vendor_arbitrage` and any vendor test (no DB in this packet). The check above is by hand from the seed files and must be re-run in CS-05a.

## Authority

> Agent 3 (server-authority-enforcer). Read-only review of the vendor path at this worktree (base `224e7692e`); no code, seed, DB, lab or server run. Every claim carries a file anchor. [unexecuted] = not run.

### Client-asserted fields on the vendor path

- `purchaseItems`/`sellItems`/`buybackItems` payload lines: `(store index | item id, quantity)` pairs. `repairItems`/`rechargeItems`: item ids (`crates/cell-methods/src/cell/cell_methods/player/vendor/wire.rs`, decoded in `store_ops.rs:72-75`).
- Nothing else. No store method carries a vendor id, template id, price, list id or cash figure on the wire (`session.rs:1-4`), so a lying client has no field to forge a price, a stock list or a vendor with.

### Validation audit

| Field or fact | Where it is checked | Result |
|---|---|---|
| Which vendor, which template | `session.rs:20-33`: `player.vendor_entity` (set by the server at `interactions/vendor.rs:20` after the interact gate) then the vendor entity's own `template_id`; `store_ops.rs:104-135` refuses `no_vendor_session`/`vendor_has_no_template` with a feedback line | Server-authoritative. A forged template cannot reach the base. |
| Right-click open | `interact.rs:36` (actor alive), `interact.rs:108` `interact_target_in_range` (5 u), vendor arm `dispatch/interact.rs:211-219` | Cleared |
| Stock row and price | `purchase_helpers.rs:91-300`: row by store index from the template's own `buy_item_list`; unknown index refuses `not_on_vendor_list` | Cleared |
| Quantity | `purchase_helpers.rs:24,176` cap 9 999, `<= 0` refused; `checked_mul` on grant, cash and item cost (`:200,:227,:256`) | Cleared |
| Cash | `purchase/mod.rs:97-115` negative and overflow refused; `:132` player-wide advisory lock, `:178` `FOR UPDATE` balance, `:198` `insufficient_cash` before the debit; whole buy in one tx, every error path rolls back | Cleared (atomic) |
| Binding | `purchase/mod.rs:268` `bound = (flags & BIND_ON_ACQUIRE) <> 0` from the design, not the wire | Cleared. None of the 19 stock designs is a BoA flag item that the quartermaster could mint unbound [seed, hand]. |
| Sell: ownership, container, bound | `sell/mod.rs:118-129`: `inv.character_id` row, `container_id = ANY([1,3..15])`, `inv.bound = false`, price from `ili.naquadah` of the vendor's sell list, `FOR UPDATE`; `quantity_exceeds_stack`, `not_sellable`, checked adds | Cleared |
| Repair and recharge | `paid_repair/mod.rs:110-116`: character-keyed, container-limited, `FOR UPDATE`; `vendor_does_not_repair` when the list is NULL | Cleared |
| Buyback | `buyback/mod.rs:112-116`: character and container 16 keyed; price read back from the row the sale wrote | Cleared |
| Every row is item_id keyed, not type_id | sell, buyback, repair all pin `(character_id, item_id, container_id)` | Cleared (no TOCTOU shape) |
| Framing dedup and token | Upstream of the cell dispatch; not re-audited here (Castle adds no wire) | Unchanged by this packet |
| Distance and liveness at buy time | MISSING (finding R1) | See below |

### Exploit analysis

- Forged price, list, template, quantity, negative or overflow quantity: mitigated (rows above).
- Dupe via sell/buyback or double-consume: mitigated. The sale writes the price into the buyback row, so sell then buyback nets zero, and a sale moves or splits the row under `FOR UPDATE` in the same tx. No `remove_item` plus `UseInventoryItem` path exists in the vendor handlers [grep, not exhaustive over content chains; CS-05a adds none].
- Arbitrage: the Economy leg's by-hand check (design 55: 300 vs 300; design 21: 1000 vs 1000) passes with zero margin; the guard is `live_db_vendor_arbitrage`.
- Attacking or killing the vendor: faction 1 template; faction is not 10, so no damage lands (Spawn leg); a vendor that cannot die needs no respawn.
- Binding abuse: a vendor purchase cannot mint a bound item as unbound (`purchase/mod.rs:268`). Starter-kit items are not bound, so the one-off sale of at most 400 naq (Economy leg) is allowed by design, not an exploit.
- **R1, remote vending (Medium, not a client lie, a missing server check).** `vendor_entity` is set on open and never cleared (grep: `vendor_entity =` has one non-test write, `interactions/vendor.rs:20`), and the five store ops never re-check range, liveness or that the vendor still stands (`store_ops.rs:77-135`). After one right-click on the quartermaster, a client can send `purchaseItems` from anywhere in the same cell space, while dead or in combat, until the vendor entity leaves the space. At the Debug Area prices that is a 1 naq Slappack (+500 Health, five per stack) and 100-round special-ammo stacks on demand anywhere in the Castle, in combat, with no hub trip. The trainer already closes this class (`trainer_authority.rs:6-46` re-checks `interact_target_in_range` at purchase); the vendor does not. Not a regression of this packet (templates 25/300/1302 behave the same), but the Castle hub is the first campaign zone where the stock makes it matter.
  - **Minimal fix (route to items-systems-advisor / cell-methods owner, a code change outside this docs packet):** in `handle_store_op`, after `vendor_context`, require `interact_target_in_range(entity_id, vendor_entity_id, space_mgr)` and an alive actor; refuse `out_of_range` with a feedback line and a `vendor` WARN, and clear `vendor_entity` on refusal. Sell/buyback/repair/recharge take the same gate (same function). Note: `handle_store_op` takes `&SpaceManager` (`store_ops.rs:65`), so the clear changes its signature to `&mut`; its only caller (`vendor/mod.rs:40,59`) already holds `&mut SpaceManager`.
- **R2, stale entity id (Low).** `vendor_entity` is a bare id; if it is recycled to another NPC in the same space, `vendor_context` prices against that NPC's template. The quartermaster is not killable so it does not free its id; a hostile reusing it carries NULL lists (empty store). Cleared by R1's fix (range plus clear on refusal).
- **R3, free-faucet pricing (owner decision, from the Economy leg).** Not an authority defect: the prices are server-authored. Flagged so the seal is not read as approval of the pricing.

### Refusal paths and telemetry

Every refusal is one WARN or INFO on `target: "vendor"`, `event = "refused"`, with `reason`, account/player/entity ids and names, vendor entity name, `op` and `args_len` (`store_ops.rs:208-228`, base `telemetry.rs`). Reasons on the path: `malformed_args` (silent to the client by design, `store_ops.rs:83-85`), `no_vendor_session`, `vendor_has_no_template` (both with the "You are not trading with a vendor." line), `empty_request`, `not_on_vendor_list`, `negative_price`, `price_overflow`, `missing_item_cost`, `insufficient_cash`, `bags_full`, `not_sellable`, `quantity_exceeds_stack`, `buyback_full`, `vendor_buys_nothing`, `vendor_does_not_repair`, `not_repairable`. Every button press gets a visible answer except malformed payloads, which a stock client cannot build. There is no `out_of_range` reason on the store ops yet (R1). SigNoz: filter logs `target = vendor`, `event = refused`, then group by `reason`.

### Authority verdict

Authority: PASS on the wire fields, prices, cash, binding and row-keying; R1 open (Medium) with a minimal fix. R1 does not forge any value, so it does not block the placement or the stock, but it is the one thing a seal should not hide.

## Services verdict

`Services: SEALED for the quartermaster (template 415, spawn 13804; placement, economy and authority legs pass, with R1 for the coordinator). The medic (template 414, spawn 13803) is BlockedEvidence (no existing medical stock list; Economy leg) and is dropped, not sealed.`

CS-05a unblocks for the quartermaster only. Not decided by this seal, and listed so the verdict is not read as covering them: the ids 415 and 13804 (coordinator confirms), R1 (route the range gate into a separate code packet, since CS-05a's allowlist has no `cell-methods` file), the Debug Area free-faucet pricing (owner, R3) and the medic list (a new `item_lists` row, owner call).

## Frozen SQL rows (for CS-05a)

Quartermaster only. Template 415 and spawn 13804 are "coordinator confirms" ids (no-collision proof in the spawn section). Statements are unexecuted [unexecuted]; the column list follows templates 176/177.

```sql
-- Quartermaster (CS-05S2a, CIMMERIA_AUTHORING): clone of template 176 with interaction_type 65536 and lists 1300/2/2/2.
INSERT INTO entity_templates (template_id, static_mesh, body_set, components, flags, interaction_type, event_set_id, level, alignment, faction, name_id, name, patrol_path_id, patrol_point_delay, template_name, class, buy_item_list, sell_item_list, repair_item_list, recharge_item_list, ability_set_id, ammo_type, loot_table_id, primary_color_id, secondary_color_id, skin_tint, weapon_item_id, static_interaction_sets, trainer_ability_list_id, speaker_id, has_dynamic_properties, interaction_set_id, move_speed) VALUES (415, NULL, 'BS_HumanMale.BS_HumanMale', '{AR_Global.Prisoner_Boots,AR_H_Clothing00.AR_HM_PL1_PL100,AR_H_Clothing00.AR_HM_PT1_PT100,BS_HumanMale.BS_HM_Boots_00,BS_HumanMale.BS_HM_Feet_00,BS_HumanMale.BS_HM_Hands_00,BS_HumanMale.BS_HM_Head_02,BS_HumanMale.BS_HM_Legs_00,BS_HumanMale.BS_HM_Torso_00}', 0, 65536, 570, 4, 0, 1, 7153, NULL, NULL, NULL, 'Castle_Quartermaster', 'mob', 1300, 2, 2, 2, NULL, NULL, NULL, -256, -256, -52773120, NULL, '{}', NULL, NULL, true, NULL, NULL);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13804, 440.0, 70.22, 992.0, 1.5708, 8, 415, 'Castle_HubQuartermaster', NULL, true);
```

Dropped: template 414 and spawn 13803 (and their two `INSERT`s in the spawn section above) are not part of the freeze. No chain, dialog, set, screen or button row is frozen (a vendor needs none; D-CA19 ranges stay reserved).

## CS-05a test list (final)

1. `live_db_castle_hub_service_is_seeded_safe_and_stocked` (`crates/cell-catalog/src/cell/spawner/tests/live_db_castle_hub.rs`, new): spawn 13804 exists in world 8 at the frozen transform; template 415 has `interaction_type 65536`, faction 1 and lists `(1300, 2, 2, 2)`; no faction-10 world-8 spawn within `aggro_radius + wander_radius + 5` (margin +35.4 u); `is_stationary`; spawned through `spawn_npc_from_record` it resolves to `NpcInteractionType::Vendor`. Fails if the interaction type or a list reverts.
2. `live_db_castle_quartermaster_lists_match_the_sealed_stock`, in the same file `live_db_castle_hub.rs` (the allowlist names one new test file): list 1300 has 19 rows; template 415 lists are not NULL and not list 1. Fails when a list reverts to NULL or becomes list 1.
3. `live_db_no_buy_row_is_cheaper_than_a_sell_back` (existing) stays green with the rows loaded. CS-05a's planned lane (`r3-packets-2.md`) does not run it: add `live-db-test.ps1 live_db_vendor_arbitrage -p cimmeria-cell-catalog` to that lane.
4. Vendor round trip on template 415: buy the pistol (300), sell it back (300), cash unchanged. Fails if row 13017 is repriced under 300. **Outside CS-05a's allowlist** (it lives with the `base-methods` vendor tests); needs an allowlist addition or a follow-up packet. The by-hand check and test 3 cover the same prices meanwhile.
5. `live_db_castle_medic_is_absent`: no `entity_templates` row 414 and no `spawnlist` row 13803 (unless the owner approves a list).
6. `castle_navmesh` picks up the 13804 row by itself (7 passed at base); extend it to the 440,70.22,992 probe.
7. If the coordinator accepts R1 (a code change plus tests in `cell-methods`, **outside CS-05a's allowlist**, so a separate packet): in `store_ops_tests.rs`, a player 40 u from the opened vendor sends `purchaseItems`; assert nothing reaches the base channel, a feedback line is sent, and the WARN carries `reason = out_of_range`; same for a dead player. Fails when the range gate is removed.

## End-UAT rows

| Row | Expectation | State |
|---|---|---|
| S1 | A character arriving at the Armory (ring/respawner side) walks to the quartermaster and right-clicks it: the vendor window opens with the 19 rows. | UATPending (end UAT, D-CA11) |
| S2 | Buy a 100-round stack for 1 naq: cash drops by 1, the stack lands in the main bag. With a character that holds BDU pants (or, on char_def 9, 10 or 19, the starter pistol), sell the item to the quartermaster and buy it back: cash unchanged. | UATPending (end UAT, D-CA11) |
| S3 | The quartermaster cannot be damaged and no hostile pulls toward the spot while the character stands there. | UATPending (end UAT, D-CA11) |
| S4 | A character landing at the DHD (component 116) cannot reach the hub on foot; expected until the Alpha-to-Armory route is proven. | UATPending (end UAT, D-CA11) |
| S5 | With the vendor window opened once, walk 30 u away and attempt a purchase: after the R1 fix it is refused with a visible line; before it, it succeeds (record which). | UATPending (end UAT, D-CA11) |
| S6 | Medic rows (old S1/S3) are void while the medic is dropped. | Void |
