# CS-02S: mission 1520 frozen-row seal

> **Type:** Reference (worknote). **Audience:** CS-02c1 and CS-02c2 coders, CS-02L, the wave reviewer.
> **Last updated:** 2026-10-17. **Inspected at:** `FEATURE_SHA` `da2b82c709bb28a8d374f535b032f838ec0a743f`. No lab, no client, no server.
> **Companions:** [G-MED evidence](../evidence/g-med.md) (and its `## R2 seal`), [R2 packets](../r2-packets.md), [entry seal](../work-packets.md).

**Verdict: `SEALED`.** Every row below is frozen and complete. Three conditions bind later packets and are not open questions:

1. Chain 1372 is only safe once CS-02L's b2-lite seam exists. Until then the victory callback runs unconditioned, so CS-02c2 must not enable 1371 before CS-02L is integrated.
2. Every claim that only the client can prove is `UATPending (end UAT, D-CA11)` (see [End UAT rows](#end-uat-rows)).
3. **Reachability is component equality only.** `nav_inspect` (exit 0) puts the spot, Ogilvie and the dispenser in navmesh component 250. No A* `find_path` was run, so the path from Ogilvie in point 2 is not proven by this seal. CS-02c1 row E8 runs that path test, and it must pass before spawn 13801 ships.

Provenance labels follow the contract: `SHIPPED_DATA`, `CLIENT_MAP`, `RECONSTRUCTION` (O14-03), `CIMMERIA_AUTHORING`. Seed descriptions carry the `CIMMERIA_RECONSTRUCTION` form, matching the chain 1366 seed precedent.

## Design in one paragraph

Per-player interactability is carried by `add_dialog_set` binds, **not** by `set_interaction_type`. A bind is stored on the player (`available_interactions[slot]`), so two players on different steps see different glows. The slot is the template id (179 for Ogilvie, 412 for the dispenser). Dialog-set maps 5827 (dialog 4983, `!` bit 24) and 5828 (NULL dialog, bit 30 `INT_MissionWorldObject`) already ship in set 1571 (`dialog_set_maps.sql`, rows 5827-5828), so **no dialog-set row is added**. Both templates ship with `interaction_type 0`: nobody can click either actor until a chain binds it for them.

## Point-by-point answers

### 1. Dispenser template 412 (RECONSTRUCTION + CIMMERIA_AUTHORING)

Clone of template 147 (`entity_templates.sql`, `Castle - Access Panel`, `being`, `GLB_Components.WorldObject_Small`, flags 0, `interaction_type 0`, `has_dynamic_properties true`) with `name_id` 8896 (SHIPPED_DATA, `texts.sql:57107`) and `template_name` `Castle_MedDispenser`. The mesh stays `Ga-Props.GA-PuzzleStation00`: it is the only mesh already proven to render on a Castle `being` (templates 147 and 245). The medical-prop swap (`Em-Props.EM-MedicalBox00`, G-MED E10) is an end-UAT option and is **not frozen**, because no loaded-package proof exists for that name.

```sql
INSERT INTO entity_templates (template_id, static_mesh, body_set, components, flags, interaction_type, event_set_id, level, alignment, faction, name_id, name, patrol_path_id, patrol_point_delay, template_name, class, buy_item_list, sell_item_list, repair_item_list, recharge_item_list, ability_set_id, ammo_type, loot_table_id, primary_color_id, secondary_color_id, skin_tint, weapon_item_id, static_interaction_sets, trainer_ability_list_id, speaker_id, has_dynamic_properties, interaction_set_id, move_speed) VALUES (412, 'Ga-Props.GA-PuzzleStation00', 'GLB_Components.WorldObject_Small', NULL, 0, 0, NULL, NULL, NULL, NULL, 8896, NULL, NULL, NULL, 'Castle_MedDispenser', 'being', NULL, NULL, NULL, NULL, NULL, NULL, NULL, 0, 0, 0, NULL, '{}', NULL, NULL, true, NULL, NULL);
```

Placement: append after the last Castle template row in `db/resources/Entities/Seed/entity_templates.sql` (the 147 block's neighbour is not required; the file is ordered by id, so place it at the end of the file).

**Collision audit** (commands run 2026-10-17, from `castle-cs-02s`, on `FEATURE_SHA`, `content/cellblock-campaign` `634b76558`, `content/harset-campaign` `e7dd74336`, `content/tollana-campaign` `a1e99c0e5`):

```text
$ git grep -n -E 'INSERT INTO entity_templates .*VALUES \(412,' <ref> -- db/resources
  (no output, exit 1) on all four refs
$ git grep -n -E 'VALUES \([0-9]+, [^)]*, 412,' <ref> -- db/resources/Worlds/Seed/spawnlist.sql
  (no output, exit 1) on all four refs
$ git grep -n -E 'VALUES \([0-9]+, [^)]*, 413,' <ref> -- db/resources/Worlds/Seed/spawnlist.sql
  (no output, exit 1) on all four refs
$ git grep -n -E 'INSERT INTO spawnlist .*VALUES \(13801,' <ref> -- db/resources
  (no output, exit 1) on all four refs
control, same form on the base ref (template 145):
$ git grep -c -E 'VALUES \([0-9]+, [^)]*, 145,' da2b82c709bb28a8d374f535b032f838ec0a743f -- db/resources/Worlds/Seed/spawnlist.sql
da2b82c709bb28a8d374f535b032f838ec0a743f:db/resources/Worlds/Seed/spawnlist.sql:8
```

The base-ref census in G-MED (15 raw hits for `-w 412`) is other namespaces (blueprints, a coordinate, spawn_id 412); none is a template id.

### 2. Spawn 13801 (CIMMERIA_AUTHORING)

Row: world 8, `(356.0, 70.18, 892.0)`, heading 0, tag `Castle_MedDispenser`, template 412.

```sql
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (13801, 356.0, 70.18, 892.0, 0, 8, 412, 'Castle_MedDispenser', NULL);
```

Placement: directly after spawn 199 (`Castle_Pop_Infirmary_Sick1`) in `db/resources/Worlds/Seed/spawnlist.sql`, inside the "Level-5 infirmary" comment block.

Evidence for the coordinate:

| Check | Result | Proof |
|---|---|---|
| Region 2051 `Castle.Infirmary` box | x 344.92-371.04, z 880.18-904.65, y 70.15-74.48. The spot is inside. | `Events/Seed/point_set_points.sql:137-143` |
| Navmesh | component 250, poly 14435, h 0.00 m, dy 0.00 m, **exit 0** | `nav_inspect` run below |
| Same component as Ogilvie | Ogilvie `(362.0, 70.18, 886.0)` is poly 14205, component 250 (exit 0 means no probe was in another component, so exit 3 did not occur) | same run |
| Hostile clearance | the only hostile within 45 u is `Castle_PRU1` (spawn 90, template 145, faction 10, default aggro 18 u) at 30.8 u. 30.8 > 18 + 3 = 21, margin 9.8 u. | `spawnlist.sql` row 90; `live_db_castle_population.rs:60` (`DEFAULT_AGGRO_RADIUS = 18.0`) |
| Friendlies | Medic1 at 8.1 u, Ogilvie 8.5 u, Sick1 11.0 u. All are static, faction 1, and may stand near a mission prop. | `spawnlist.sql` rows 197-199 |

```text
nav_inspect data\spaces\castle.nav --probe Ogilvie=362.0,70.18,886.0 --probe DispA=366.0,70.18,892.0
  --probe DispB=356.0,70.18,892.0 --probe DispC=367.0,70.18,884.0 --probe Medic=352.0,70.18,885.0
  Ogilvie poly=14205 component=250 h=0.00 dy=-0.00 ok     DispB poly=14435 component=250 h=0.00 dy=-0.00 ok   (chosen)
  DispA   poly=14434 component=250 h=0.00 dy=-0.05 ok     DispC poly=14218 component=250 h=0.00 dy=-0.23 ok
  Medic   poly=14230 component=250 h=0.00 dy=-0.01 ok     exit 0
```

**What was not run:** an A* `find_path` Ogilvie to dispenser. Reachability is component equality only; the spec's path is deferred to E8 (verdict item 3), which adds the `mesh.find_path` assertion (last waypoint within 2.0 of the target) as a test row below.

**Collision audit:**

```text
git grep -n -E "INSERT INTO spawnlist .*VALUES \(13801," <ref> -- db/resources   -> 0 hits on all four refs
git grep -n -w 13801 <ref> -- db/resources                                       -> 1 hit on all four refs: texts.sql:8537, a moniker row (other namespace)
```

### 3. Interaction scope (SHIPPED code fact)

`set_interaction_type` is **global**. `executor/world/mod.rs:30-91` mutates `target.interaction_type_flags` on the one tagged entity, then sends `WitnessEntityMethod` (`INTERACTION_TYPE`) to every witness from `space_mgr.get_witnesses_of(target_id)`. Mission 702's own test says so (`chain_replay_tests/mission_702.rs:478`).

The design therefore never calls it for 1520. Per-player glow is `add_dialog_set` / `remove_dialog_set` on slot 412 and slot 179 (`executor/dialog/mod.rs:166-340`): they edit only the calling player's `available_interactions` and push the merged flags to that player's client. Two players on steps 4452 and 4453 get independent glows. The server-side gate for a forged click is the chain condition on step 4453 (point 5), and `handle_interact` reads only the template's interaction type and `available_interactions`. Neither template carries a static glow, so there is no bit-30 clearing and no global toggle.

The client-side highlight (bit 30 over the dispenser, bit 24 over Ogilvie) is end-UAT only.

### 4. Ogilvie at 4452 (RECONSTRUCTION; flags from CLIENT_MAP)

Native dialog-set path, not an `interact_tag` chain. Reasons:

- Ogilvie is unclickable (`interaction_type 0`) until bound; map 5827 supplies the `!` and dialog 4983 (speaker 2490 on screens 96921, 96925, 96927; speaker 0 elsewhere, so it is **not** a monologue and needs the resolved NPC).
- `handle_interact` opens 4983 and pins the speaker, as in the canonical Castle 1012/1204 shape.
- No `interact_tag` chain exists for `Castle_Ogilvie`, so nothing short-circuits `handle_interact`.

Current code pins `last_interaction_target` **before** the tag/template dispatch (`cell_methods/player/interaction/interact.rs`, `pin_interaction_target`, directly above the trainer check), and the in-range gate runs above it. Two older notes ("tag chains never pin" and "no distance check on tag chains") no longer hold on this base. The native path is still chosen because it needs one fewer chain.

Dialog 4983 has no buttons (`dialog_screen_buttons` has no row for screens 96921-96927), so closing it raises `dialog_choice 4983` with `button_id -1`, gated by `open_dialog_id` (#479).

- **Bind** (chains 1380 on accept, 1381 on relog): `add_dialog_set 5827 {"slot": 179}`.
- **Close** (chain 1378, `dialog_choice '4983'`): `remove_dialog_set 5827 {"slot": 179}`; `advance_step 1520 '4453'`; `add_dialog_set 5828 {"slot": 412}`.

`advance_step` force-completes the open objectives of step 4452, 5171 **and** the hidden 5172, and ticks both (`missions/progression.rs`). So the "credit 5172" is the advance itself; no `complete_objective` is authored. 5171 is credited earlier by chain 1368.

### 5. The game (RECONSTRUCTION, O14-03 game choice; params are CIMMERIA_AUTHORING)

Chain **1371**: `interact_tag 'Castle_MedDispenser'`. Conditions: world 8, `mission_status 1520 active`, `step_status 1520 '4453' active`. `fire_interact_tag` reports "handled" only when the resolved action list is non-empty (`event_dispatch/interaction.rs`, `matched = !resolved.actions.is_empty()`), so a click from a player not on 4453 matches nothing and falls through to `handle_interact`, which finds no bind and does nothing.

Action: `start_minigame` with target_key `Livewire` and params `{"difficulty": 1, "on_victory_chains": [1372]}`, `delay_ms 0`. Difficulty 1 equals the default (the 1233 precedent omits it); it is written out so the seal is explicit. The original game and difficulty stay `UNRESOLVED (G-MED game)`. A defeat leaves the 5828 bind in place, so the player can click again (same property as 1233).

### 6. Victory chain 1372 (RECONSTRUCTION)

No trigger row. Conditions: world 8, `mission_status 1520 active`, `step_status 1520 '4453' active`. Every action `delay_ms 0`:

| # | Action | Why |
|---|---|---|
| 0 | `remove_dialog_set 5828 {"slot": 412}` | before the mission change, so a nested lifecycle dispatch sees a clean slot |
| 1 | `advance_step 1520 '4454'` | force-completes and ticks objective 5173 (sole objective of 4453) |
| 2 | `display_dialog 4984` | speaker 0 only (screen 96928), so it is a monologue and binds the player; callable from a no-params callback |

**Do not author `complete_objective 5173`.** It is the sole required objective of 4453, so the progression code would complete the whole mission (`advance-step-vs-complete-objective`, `missions/progression.rs`).

### 7. The 4984 close (RECONSTRUCTION of the G-MED radio call, O14-03)

Chain **1379**: `dialog_choice '4984'`. Conditions: world 8, `mission_status 1520 active`, `step_status 1520 '4454' active`. Action: `display_dialog 4985`.

4984 has no buttons, so it raises `dialog_choice 4984, -1` on close. 4985 is `DUIST_DefaultDialog` (type 2), so it must open immediately and never as a radio lure (G-MED E5). 4985 has speaker 1110 (Copplemann's radio voice) on screens 96929 and 96931, so it is not a monologue: the speaker resolves from `last_interaction_target`, which is the **dispenser** (the last native/tag interaction). Expect the dialog label to read "Med Dispenser". That is cosmetic and client-side only (see `dialog-chain-authoring-rules` rule 6): `UATPending`. No server-side fix exists.

### 8. The 4985 close, chain 1373 (RECONSTRUCTION)

`dialog_choice '4985'` (zero buttons, close sends -1). Conditions: world 8, `mission_status 1520 active`, `step_status 1520 '4454' active`. Single action: `complete_mission 1520`.

- **Advancing past the last step does not complete a mission.** 4454 is the last step; nothing in `advance_step` runs the all-done auto-complete, so an explicit `complete_mission` is required.
- `complete_mission_direct` force-completes the final step's objectives. So objective 5174 is credited by `complete_mission` itself (the turn-in shape, as for 622).
- **Do not author `complete_objective 5174`**: it is the sole required objective of 4454 and would complete the mission mid-list, before `complete_mission` runs.

### 9. Relog (RECONSTRUCTION)

`available_interactions` is in memory only, so every bind is lost on relog. Chains with `player_loaded 'Castle'` (the 1240 precedent):

| Chain | Condition (besides `world 8`, `mission_status 1520 active`) | Action |
|---|---|---|
| **1381** | step 4452 active | `add_dialog_set 5827 {"slot": 179}` |
| **1382** | step 4453 active | `add_dialog_set 5828 {"slot": 412}` |
| **1383** | step 4454 active | `complete_mission 1520` |

1383 is a deliberate recovery: a relog between the 4984 close and the 4985 close would otherwise strand the player, because `display_dialog 4985` cannot resolve a speaker on a fresh login (the pin is gone and the dialog is not a monologue). Completing straight away skips only a radio call that cannot be shown. This is a RECONSTRUCTION choice; the alternative (strand the player) is worse.

The three conditions are pairwise disjoint (one step each). `player_loaded` populates archetype, but nothing here gates on it.

### 10. Entry chains 1367 and 1368 (RECONSTRUCTION, signed 2026-10-14)

Both trigger on `enter_region 'Castle.Infirmary'` and are disjoint by `mission_status 1520` (`not_active` vs `active`). `archetype` is populated by `fire_enter_region` (`event_dispatch/region.rs:46`). The archetype set {1,2,3,4,6} is written as three conditions (`gte 1`, `lte 6`, `neq 5`) because rows AND together and the default for an unpopulated archetype is -1, which fails `gte 1` closed. Archetypes 7 and 8 fail `lte 6`; 5 fails `neq 5`.

```sql
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (1367, '1520 - Castle.Infirmary enter: offer Ambernol mission (CIMMERIA_RECONSTRUCTION)', 'mission', 1520, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (1367, 'enter_region', 'Castle.Infirmary', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES
  (1367, 'world', 8, NULL, 'eq', NULL, 0), (1367, 'mission_status', 704, NULL, 'eq', 'completed', 1), (1367, 'mission_status', 1520, NULL, 'eq', 'not_active', 2),
  (1367, 'archetype', NULL, NULL, 'gte', '1', 3), (1367, 'archetype', NULL, NULL, 'lte', '6', 4), (1367, 'archetype', NULL, NULL, 'neq', '5', 5);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (1367, 'accept_mission', 1520, NULL, '{}', 0, 0);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (1368, '1520 - Castle.Infirmary enter: credit objective 5171 (CIMMERIA_RECONSTRUCTION)', 'mission', 1520, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (1368, 'enter_region', 'Castle.Infirmary', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES
  (1368, 'world', 8, NULL, 'eq', NULL, 0), (1368, 'mission_status', 1520, NULL, 'eq', 'active', 1), (1368, 'step_status', 1520, '4452', 'eq', 'active', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (1368, 'complete_objective', 1520, '5171', '{}', 0, 0);
```

`accept_mission` replays the first step's region (`accepting_a_mission_replays_the_first_steps_regions`), so 1368 fires inside 1367's action list; 4452 holds two required objectives (5171, 5172), so crediting 5171 does not auto-complete the mission. A second entry re-credits 5171 as a data no-op: `complete_objective` (`crates/cell-content/src/cell/missions/progression.rs:325`, `crates/entity/src/missions.rs:129`) dedupes `completed_objectives`, sends at most one duplicate `onObjectiveUpdate` frame, and cannot advance the mission, because 5172 is credited only by `advance_step`. Row E5 pins this.

### 11. Population guard (SHIPPED code fact)

`live_db_castle_population.rs` does **not** reject the dispenser:

- the pinned counts (24 friendly rows 189-212, 36 hostile rows 247-282) are keyed by spawn-id lists, and spawn 13801 is in neither;
- the unique-tag assertion needs `Castle_MedDispenser` unique in world 8, which it is;
- the `Castle_` prefix rule covers only the two blocks.

The D-CA06 text in `spawnlist.sql` ("Nothing here is chain-spawned or tagged by a content chain") is a statement about the population blocks. The dispenser is a **static** row that a chain references by tag, exactly like `Castle_AccessPanel` and `Castle_DHD`; nothing is chain-spawned.

**Minimal edit (CS-02c1, optional but recommended):** add `"Castle_MedDispenser"` to `MISSION_ACTORS` (`live_db_castle_population.rs:72`, array length 9 to 10) so the existing hostile-clearance test (loop at `:420`) covers the new prop, and add one sentence to the spawnlist comment saying the dispenser block is the single mission prop outside the population. The edit was **not run** here (docs-only packet).

## Frozen rows

1. Template 412 and spawn 13801: point 1 and point 2 above, verbatim.
2. Chains 1367 and 1368: point 10 above, verbatim.
3. Chains 1371-1373 and 1378-1383:

```sql
-- 1371: dispenser click while 4453 is active starts Livewire
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (1371, '1520 - Med Dispenser interact (step 4453): start Livewire (CIMMERIA_RECONSTRUCTION)', 'mission', 1520, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (1371, 'interact_tag', 'Castle_MedDispenser', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES
  (1371, 'world', 8, NULL, 'eq', NULL, 0), (1371, 'mission_status', 1520, NULL, 'eq', 'active', 1), (1371, 'step_status', 1520, '4453', 'eq', 'active', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (1371, 'start_minigame', NULL, 'Livewire', '{"difficulty": 1, "on_victory_chains": [1372]}', 0, 0);

-- 1372: Livewire victory (no trigger; fired through the b2-lite guarded seam)
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (1372, '1520 - Livewire victory: advance to 4454, show 4984 (CIMMERIA_RECONSTRUCTION)', 'mission', 1520, true, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES
  (1372, 'world', 8, NULL, 'eq', NULL, 0), (1372, 'mission_status', 1520, NULL, 'eq', 'active', 1), (1372, 'step_status', 1520, '4453', 'eq', 'active', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES
  (1372, 'remove_dialog_set', 5828, NULL, '{"slot": 412}', 0, 0),
  (1372, 'advance_step', 1520, '4454', '{}', 0, 1),
  (1372, 'display_dialog', 4984, NULL, '{}', 0, 2);

-- 1373: 4985 close completes the mission
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (1373, '1520 - Dialog 4985 close: complete 1520 (CIMMERIA_RECONSTRUCTION)', 'mission', 1520, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (1373, 'dialog_choice', '4985', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES
  (1373, 'world', 8, NULL, 'eq', NULL, 0), (1373, 'mission_status', 1520, NULL, 'eq', 'active', 1), (1373, 'step_status', 1520, '4454', 'eq', 'active', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (1373, 'complete_mission', 1520, NULL, '{}', 0, 0);

-- 1378: Ogilvie talk closed (4983): advance to 4453, hand the player the dispenser glow
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (1378, '1520 - Dialog 4983 close (Ogilvie): advance to 4453, bind dispenser (CIMMERIA_RECONSTRUCTION)', 'mission', 1520, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (1378, 'dialog_choice', '4983', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES
  (1378, 'world', 8, NULL, 'eq', NULL, 0), (1378, 'mission_status', 1520, NULL, 'eq', 'active', 1), (1378, 'step_status', 1520, '4452', 'eq', 'active', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES
  (1378, 'remove_dialog_set', 5827, NULL, '{"slot": 179}', 0, 0),
  (1378, 'advance_step', 1520, '4453', '{}', 0, 1),
  (1378, 'add_dialog_set', 5828, NULL, '{"slot": 412}', 0, 2);

-- 1379: 4984 close shows the 4985 radio call
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (1379, '1520 - Dialog 4984 close: show radio call 4985 (CIMMERIA_RECONSTRUCTION)', 'mission', 1520, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (1379, 'dialog_choice', '4984', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES
  (1379, 'world', 8, NULL, 'eq', NULL, 0), (1379, 'mission_status', 1520, NULL, 'eq', 'active', 1), (1379, 'step_status', 1520, '4454', 'eq', 'active', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (1379, 'display_dialog', 4985, NULL, '{}', 0, 0);

-- 1380: bind Ogilvie when 1520 is accepted
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (1380, '1520 - Accepted: bind Ogilvie talk (CIMMERIA_RECONSTRUCTION)', 'mission', 1520, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (1380, 'mission_accepted', '1520', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES
  (1380, 'world', 8, NULL, 'eq', NULL, 0), (1380, 'step_status', 1520, '4452', 'eq', 'active', 1);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (1380, 'add_dialog_set', 5827, NULL, '{"slot": 179}', 0, 0);

-- 1381 / 1382 / 1383: relog restores (one step each)
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (1381, '1520 - Login at 4452: rebind Ogilvie (CIMMERIA_RECONSTRUCTION)', 'mission', 1520, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (1381, 'player_loaded', 'Castle', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES
  (1381, 'world', 8, NULL, 'eq', NULL, 0), (1381, 'mission_status', 1520, NULL, 'eq', 'active', 1), (1381, 'step_status', 1520, '4452', 'eq', 'active', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (1381, 'add_dialog_set', 5827, NULL, '{"slot": 179}', 0, 0);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (1382, '1520 - Login at 4453: rebind dispenser (CIMMERIA_RECONSTRUCTION)', 'mission', 1520, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (1382, 'player_loaded', 'Castle', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES
  (1382, 'world', 8, NULL, 'eq', NULL, 0), (1382, 'mission_status', 1520, NULL, 'eq', 'active', 1), (1382, 'step_status', 1520, '4453', 'eq', 'active', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (1382, 'add_dialog_set', 5828, NULL, '{"slot": 412}', 0, 0);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (1383, '1520 - Login at 4454: complete (radio call cannot be shown) (CIMMERIA_RECONSTRUCTION)', 'mission', 1520, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (1383, 'player_loaded', 'Castle', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES
  (1383, 'world', 8, NULL, 'eq', NULL, 0), (1383, 'mission_status', 1520, NULL, 'eq', 'active', 1), (1383, 'step_status', 1520, '4454', 'eq', 'active', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (1383, 'complete_mission', 1520, NULL, '{}', 0, 0);
```

4. File and include line. The chains go in a **new** file `db/resources/Content/Seed/castle_1520_chains.sql` (entry chains 1367-1368 included; the same `SET search_path` footer convention as the neighbours applies), included from `db/database.sql` immediately **after line 397**:

```text
\ir resources/Content/Seed/castle_1520_chains.sql
```

Line 397 is `\ir resources/Content/Seed/castle_carry_in_chains.sql`. The b2-lite 1521 file (CS-03S) takes its own line after this one.

5. Mission steps 4452-4454 and objectives 5171-5174 stay disabled (`step_enabled`/`is_enabled` are not read at runtime; contract excerpt). Dialog-set rows 5827 and 5828 already exist. No other seed changes.

**Chain-ID use:** 1367, 1368, 1371-1373 (reserved to CS-02c1) and 1378-1383 (this packet). 1384 and 1385 stay free; chains 1369/1370/1374-1377 belong to 1521.

**Collision audit for chains** (same four refs):

```text
git grep -n -E "\(\s*13(6[7-9]|7[0-7])\s*," <ref> -- db/resources/Content        -> 0 hits on all four refs
git grep -n -E "VALUES \(\s*13(7[89]|8[0-5])\s*," <ref> -- db/resources/Content  -> 0 hits on all four refs
```

## Dispatch checks (the traps)

- **Disjoint siblings.** Per trigger key the sets are: `enter_region Castle.Infirmary` {1367 (1520 not_active), 1368 (1520 active + 4452)} disjoint; `player_loaded Castle` {1381, 1382, 1383} disjoint by step (and disjoint from 1521's chains, which gate on mission 1521); `dialog_choice` keys 4983, 4984, 4985 each have one chain; `interact_tag Castle_MedDispenser` has one chain. Chains 1204/1205 and others use other keys.
- **Frozen conditions.** In 1378 the three actions run after the conditions were frozen, so `remove_dialog_set` before `advance_step` is just the safe order for the nested lifecycle dispatch; the add runs last.
- **`add_dialog_set` has no dedupe** (first bind wins): each bind chain is reachable once per state (accept, or login), never both in one event.
- **Abandon.** An abandoned 1520 leaves its in-memory bind until the next relog. Clicking Ogilvie then opens 4983, and the close chain's `mission_status active` condition rejects the advance. Cosmetic.
- **Interact-tag linter.** `crates/content-engine/tests/it/interact_tag_linter.rs` scans every `*_chains.sql` and requires a `set_interaction_type` for each `interact_tag` tag in the same file. Chain 1371 (`Castle_MedDispenser`) has none, by design: its glow is the per-player bind (`add_dialog_set 5828`, slot 412). The scanner reads only lines that start with `(` or `VALUES`, so the one-line `INSERT ... VALUES (...)` rows frozen here are skipped and the test passes vacuously for 1371. Keep each row on one line. A multi-line reformat makes 1371 fail; the fix is a `// reason:` allowlist entry in `interact_tag_linter.rs` for `castle_1520_chains.sql` chain 1371. That file is outside the CS-02c1 allowlist, so CS-02c1 must list it or keep the rows one-line.

## Replay test list

All use the live-DB replay harness (`load_single_chain_for_test`, `require_db_or_skip`). Module names are suggestions. "E" rows go to **CS-02c1** (entry); "G" rows go to **CS-02c2** (graph).

| Row | Event | Preconditions | Expected actions | Owner |
|---|---|---|---|---|
| E1 | `enter_region Castle.Infirmary` | world 8, 704 completed, 1520 not_active, archetype 1, 2, 3, 4, 6 (five cases) | 1367: `accept_mission 1520` | CS-02c1 |
| E2 | same | archetype 5, 7, 8, and unpopulated | nothing | CS-02c1 |
| E3 | same | 704 not completed; 1520 active; 1520 completed; wrong world | 1367 nothing | CS-02c1 |
| E4 | same | 1520 active, step 4452 active | 1368: `complete_objective 1520 5171`; 1367 absent (disjoint) | CS-02c1 |
| E5 | same, twice | after E4 | `completed_objectives` holds 5171 once; mission active on 4452; at most one extra `onObjectiveUpdate` frame; no advance (point 10) | CS-02c1 |
| E6 | accept via the real region dispatcher | 1367 accepted | the replay fires 1368 inside 1367's list; 5171 credited; mission still active on 4452; 1380 fires `add_dialog_set 5827 slot 179` | CS-02c1 |
| E7 | seed rows | n/a | template 412 and spawn 13801 load; template `interaction_type 0`; `has_dynamic_properties true`; tag unique in world 8; spawn on navmesh (`castle_navmesh.rs`) | CS-02c1 |
| E8 | navmesh | n/a | `find_path(Ogilvie, dispenser)` last waypoint within 2.0 of the target | CS-02c1 |
| E9 | population guard | `MISSION_ACTORS` gains the tag | hostile-clearance test passes for the dispenser | CS-02c1 |
| G1 | `dialog_choice 4983` | 4452 active | 1378: remove 5827, `advance_step 4453`, add 5828 | CS-02c2 |
| G2 | same | 4453, 4454, not_active or completed | nothing | CS-02c2 |
| G3 | `interact_tag Castle_MedDispenser` | 4453 active | 1371: `start_minigame Livewire` with difficulty 1 and victory `[1372]` | CS-02c2 |
| G4 | same | 4452, 4454, 1520 not_active | nothing (no action; `handled` false) | CS-02c2 |
| G5 | victory callback 1372 through the b2-lite seam | 4453 active | remove 5828, `advance_step 4454`, `display_dialog 4984` in that order, all `delay_ms 0` | CS-02c2 (needs CS-02L) |
| G6 | victory callback 1372 | 4452/4454, other player, entity missing | nothing fires | CS-02c2 (needs CS-02L) |
| G7 | `dialog_choice 4984` | 4454 active | 1379: `display_dialog 4985`; with 4453 active nothing | CS-02c2 |
| G8 | `dialog_choice 4985` | 4454 active | 1373: `complete_mission 1520`; no `complete_objective` anywhere in 1372/1373 | CS-02c2 |
| G9 | `player_loaded Castle` | 4452 / 4453 / 4454 / other | 1381 / 1382 / 1383 / nothing, exactly one chain per case | CS-02c2 |
| G10 | two players | A on 4452, B on 4453 | A's bind list holds 5827 only, B's holds 5828 only (no global state) | CS-02c2 |
| G11 | full walk | accept to complete | final state: 1520 completed, final-step objective 5174 completed, persisted `current_step_id` NULL | CS-02c2 |
| G12 | static guard | seed query | no `set_interaction_type` and no `complete_objective` rows for chains 1367-1383 except 1368's 5171 | CS-02c2 |

## End UAT rows

All `UATPending (end UAT, D-CA11)`; none can be proven without the client.

1. Ogilvie shows the `!` (bit 24) after accepting 1520 at the Infirmary, and nobody else's view changes.
2. 4983 opens with Ogilvie's portrait and name; the close advances the journal to "Hack the Med Dispenser...".
3. The dispenser is visible (Goa'uld puzzle-station mesh) and shows the bit-30 glow only to a player on 4453; a player on another step sees none and cannot click.
4. A right-click starts the Livewire UI; defeat allows a retry; victory advances to 4454 and shows 4984.
5. 4985 opens after the 4984 close. Its name label is expected to read the dispenser's (cosmetic, see point 7). Closing it completes the mission and shows the completed journal entry.
6. Relog at each of 4452, 4453, 4454 restores (bind, bind, auto-complete).
7. Optional: swap template 412's mesh to `Em-Props.EM-MedicalBox00`; accept only if it renders.
