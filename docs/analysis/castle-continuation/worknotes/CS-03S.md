# CS-03S worknote: mission 1521 frozen rows (Option B skeleton)

Base `da2b82c709bb28a8d374f535b032f838ec0a743f`. Owner decision D-CA-SYM = Option B: no symbiote implant, no item grant, no debuff, no consumption. No lab used. Evidence: [g-sym.md](../evidence/g-sym.md) (see its `## R2 seal`).

**Verdict: SEALED** (Option B skeleton). UATPending: tank highlight, Livewire UI, 4991 speaker label, journal text. The chamber placement stays BlockedEvidence (G-ROUTE) and is not used.

## 1. Collision audit (base, `content/cellblock-campaign`, `content/harset-campaign`, `content/tollana-campaign`; identical on all four)

| Id | Result |
|---|---|
| entity_templates 413 | free (the `VALUES (413,` hits are `blueprints*`); 412 reserved for CS-02S |
| spawnlist spawn_id 13802 | free; 13800 Debug Area DHD, 13810-13817 rings; 13801 reserved for CS-02S |
| spawnlist template_id 413 | 0 rows |
| chains 1369-1377, 1386-1393 | 0 rows |

Raw output of `git grep -n -E "INSERT INTO (entity_templates|spawnlist) .*(VALUES \((413|13802)[,)]|, 413, |\(13802, )" <ref> -- db/resources` (identical on all four refs; the path is `db/resources/Worlds/Seed/spawnlist.sql` at line 1173, and line 1180 on `content/cellblock-campaign`):

```
db/resources/Worlds/Seed/spawnlist.sql:1173: INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (413, -336.72, 73.472, -217.15, 2.7937, 12, 313, 'CraftHub_Station_Materials', NULL, true);
```

That single hit is spawn_id 413 (Harset CraftHub), a different column. No `entity_templates` row and no spawnlist row with `template_id` 413 or `spawn_id` 13802 exists on any ref.

## 2. Placement and G-ROUTE (CIMMERIA_AUTHORING, option a)

`nav_inspect data\spaces\castle.nav --v-tol 0.05`: Ogilvie (362.0, 70.18, 886.0) poly 14205 component 250; **tank (356.0, 70.18, 896.0) poly 14513 component 250, dy 0**; medic component 250. Chamber spots (391, 55.38, 925), (386, 55.38, 940), (398.83, 55.38, 929/941) are component 468, isolated; Ogilvie plus chamber probe exits 3 (components [250, 468]). So the chamber placement is G-ROUTE-blocked and NOT playable; the tank goes in the Infirmary (box x 344.92-371.04, z 880.18-904.65). Distances: PRU1 spawn 90 about 27.0 (clears mission-actor 21 and friendly 23), Ogilvie and medic about 11.7, sick about 14.3, nearest hostile (spawn 115) about 41. Mesh `CA-Props.CA-SymbioteChamber00` is placed away from the chamber actors, so no z-fight. Reachability therefore rests on component equality only (nav_inspect; no `find_path` is run in this packet). The path replay is the CS-03c2 row `find_path Ogilvie -> tank`.

Merge check for CS-03c1: CS-02S's dispenser (13801) must be at least 4 u from 13802 and on component 250.

## 3. Frozen rows

Template (after the last Castle template; precedent Harset 245):

```sql
-- Castle symbiote tank [CIMMERIA_AUTHORING, mesh CLIENT_MAP]. Option B: world prop only.
INSERT INTO entity_templates (template_id, static_mesh, body_set, components, flags, interaction_type, event_set_id, level, alignment, faction, name_id, name, patrol_path_id, patrol_point_delay, template_name, class, buy_item_list, sell_item_list, repair_item_list, recharge_item_list, ability_set_id, ammo_type, loot_table_id, primary_color_id, secondary_color_id, skin_tint, weapon_item_id, static_interaction_sets, trainer_ability_list_id, speaker_id, has_dynamic_properties, interaction_set_id, move_speed, respawn_secs) VALUES (413, 'CA-Props.CA-SymbioteChamber00', 'GLB_Components.WorldObject_Small', NULL, 4, 256, NULL, 1, 0, 1, 8897, NULL, NULL, NULL, 'Castle - Symbiote Tank', 'being', NULL, NULL, NULL, NULL, NULL, NULL, NULL, 0, 0, 0, NULL, '{}', NULL, NULL, true, NULL, NULL, NULL);
```

`interaction_type 256` (INT_MINIGAME_LIVEWIRE) is static; clicking before step 4456/4457 is a no-op because chains 1374/1376 are step-gated. The 1073741824 mission-object highlight is end-UAT only. No per-player toggling (set_interaction_type fans out to all witnesses).

Deviation from precedent (Harset 245 and the CS-02S dispenser use `interaction_type` 0 plus a per-player bind): the static 256 shows the hack cursor to every player at every step. It is a recorded choice, not a defect: server-safe (`INT_MINIGAME_LIVEWIRE` is not read by `handle_interact`, and the step-gated chains no-op). End UAT row 1 checks it. The tank is 4.0 u from the dispenser 13801 at (356, 892), which is the merge minimum.

Spawn (`spawnlist.sql`, world-8 block):

```sql
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13802, 356.0, 70.18, 896.0, 0, 8, 413, 'Castle_SymbioteTank', NULL, true);
```

Chains, file `db/resources/Content/Seed/castle_symbiote_chains.sql`; `\ir` line goes after `db/database.sql:397` (`castle_carry_in_chains.sql`):

```sql
\ir resources/Content/Seed/castle_symbiote_chains.sql
```

(CS-03c1 copies the exact relative form of line 397.) Common to every chain: `scope_type 'mission'`, `scope_id 1521`, `enabled true`, `priority 0`; triggers `scope 'player'`, `once false`; every chain below carries its `world` row at sort_order 0 (target_id 8, operator eq, value NULL; layout of `castle_carry_in_chains.sql:11`), and the other conditions are numbered from 1. Descriptions carry the provenance label `(CIMMERIA_RECONSTRUCTION)` in the contract seed shape.

```sql
-- 1369 offer: archetype 8 only, once 704 is done
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (1369, '1521 - Infirmary entry (Jaffa): accept 1521 (CIMMERIA_RECONSTRUCTION)', 'mission', 1521, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (1369, 'enter_region', 'Castle.Infirmary', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES
  (1369, 'world',          8,    NULL, 'eq', NULL,        0),
  (1369, 'mission_status', 704,  NULL, 'eq', 'completed', 1),
  (1369, 'mission_status', 1521, NULL, 'eq', 'not_active', 2),
  (1369, 'archetype',      NULL, NULL, 'eq', '8',         3);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (1369, 'accept_mission', 1521, NULL, '{}', 0, 0);

-- 1370 step 4455 -> 4456 (never complete_objective 5175)
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (1370, '1521 - on accept: advance 4455 to 4456 (CIMMERIA_RECONSTRUCTION)', 'mission', 1521, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (1370, 'mission_accepted', '1521', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (1370, 'world', 8, NULL, 'eq', NULL, 0), (1370, 'step_status', 1521, '4455', 'eq', 'active', 1);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (1370, 'advance_step', 1521, '4456', '{}', 0, 0);

-- 1374 first tank use: Livewire
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (1374, '1521 - tank interact (step 4456): start Livewire (CIMMERIA_RECONSTRUCTION)', 'mission', 1521, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (1374, 'interact_tag', 'Castle_SymbioteTank', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (1374, 'world', 8, NULL, 'eq', NULL, 0), (1374, 'step_status', 1521, '4456', 'eq', 'active', 1);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (1374, 'start_minigame', NULL, 'Livewire', '{"difficulty": 1, "on_victory_chains": [1375]}', 0, 0);

-- 1375 victory: no trigger row (direct invoke); credits 5176, advance, dialog 4990 (monologue)
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (1375, '1521 - Livewire victory: credit 5176, advance to 4457, dialog 4990 (CIMMERIA_RECONSTRUCTION)', 'mission', 1521, true, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (1375, 'world', 8, NULL, 'eq', NULL, 0), (1375, 'step_status', 1521, '4456', 'eq', 'active', 1);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES
  (1375, 'advance_step',   1521, '4457', '{}', 0, 0),
  (1375, 'display_dialog', 4990, NULL,   '{}', 0, 1);

-- 1376 second tank use: advance to 4458 and open 4991
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (1376, '1521 - tank interact (step 4457): advance to 4458, dialog 4991 (CIMMERIA_RECONSTRUCTION)', 'mission', 1521, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (1376, 'interact_tag', 'Castle_SymbioteTank', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (1376, 'world', 8, NULL, 'eq', NULL, 0), (1376, 'step_status', 1521, '4457', 'eq', 'active', 1);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES
  (1376, 'advance_step',   1521, '4458', '{}', 0, 0),
  (1376, 'display_dialog', 4991, NULL,   '{}', 0, 1);

-- 1377 close: 4991 closes -> credit 5178 and complete
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (1377, '1521 - dialog 4991 closed: complete 1521 (CIMMERIA_RECONSTRUCTION)', 'mission', 1521, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (1377, 'dialog_choice', '4991', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (1377, 'world', 8, NULL, 'eq', NULL, 0), (1377, 'step_status', 1521, '4458', 'eq', 'active', 1);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (1377, 'complete_mission', 1521, NULL, '{}', 0, 0);
```

Notes on the rows:
- `advance_step` force-completes and credits the old step's objectives (5175 at 4455->4456 by 1370 is the one credit; 5176 and 5177 by 1375/1376), so no `complete_objective` rows exist. Objective 5178 (final step) closes only through `complete_mission` in 1377; CS-03c2 asserts all of 5175-5178 are credited at completion.
- World gate: the `world` row (target_id 8) is written at sort_order 0 into 1369, 1370, 1374, 1375, 1376 and 1377. CS-03c1 copies the rows as written and prepends nothing.
- 1374 and 1376 share trigger key `interact_tag Castle_SymbioteTank`; conditions are disjoint (4456 vs 4457). Re-clicking at 4458 does nothing.
- 1375 has no trigger; per b2-lite (CS-02L) conditions are evaluated for chains that have any. 1374 sets `"difficulty": 1` explicitly in its params (CS-02L minigame edit 2; the seed guard asserts it), with `on_victory_chains` `[1375]`.
- 1370 uses `mission_accepted`; the loader arm exists (`crates/content-engine/src/loader/trigger.rs:89`, `loader/tests/trigger_conversion.rs:14`). There is no fallback row.
- 4991 is `DUIST_DefaultDialog` (a plain window, not the radio) with Moh'katan (speaker 2499) screens, so it is not a monologue: displayed from the tank `interact_tag` chain it takes the tank as speaker. The label is per dialog. Cosmetic, UATPending. 4991 has no buttons; closing raises `dialog_choice` -1 on `open_dialog_id` 4991, which fires 1377.
- 4989 (Moh'katan briefing, speaker 2499) is unreferenced by any chain in this seal and no row offers it. It stays unreferenced.

## 4. Relog and population-guard points (as CS-02S 9 and 11)

- Relog at 4456/4457/4458: **no restore chain needed.** Tank `interaction_type` is static and the click gates are step-status, which persists with the mission row. An open Livewire at relog is lost; the player re-clicks (the session is removed on exit; defect B4 applies if the SWF never connects).
- Population guard (`live_db_castle_population.rs`): spawn 13802 and template 413 are outside the counted ranges (189-212, 247-282), so not rejected. World-8 tag uniqueness: `Castle_SymbioteTank` is unique. CS-03c1 adds the tag to `MISSION_ACTORS` (9 -> 10); this collides on array length with CS-02c1's dispenser tag (-> 11), so flag the merge.
- `castle_navmesh.rs::every_castle_spawn_is_on_the_mesh` covers the new row automatically.
- interact_tag linter: `crates/content-engine/tests/it/interact_tag_linter.rs` scans every `*_chains.sql` for an `interact_tag` chain whose tag has no `set_interaction_type` in the same file. Chains 1374 and 1376 (tag `Castle_SymbioteTank`) have none, and the scanner reads only lines that start with `(` or `VALUES`, so the one-line `INSERT ... VALUES (...)` form of this seal is skipped and the test passes vacuously. CS-03c1 must either keep the one-line form in `castle_symbiote_chains.sql` or add an allowlist entry in `interact_tag_linter.rs` (reason: static `interaction_type` 256 on template 413). That file is outside the CS-03c1 allowlist, so the allowlist edit is listed for the overlay (`r2-packets.md`), not made here.
- Dispenser clearance: CS-02S's dispenser (13801) must stay at least 4 u from 13802; the current gap is exactly 4.0 u, so CS-03c1 re-checks it after the CS-02S merge.

## 5. Replay tests

| Test | Event | Preconditions | Expected actions | Phase |
|---|---|---|---|---|
| 1369 offers 1521 to archetype 8 only (5 and 7 excluded), requires 704 completed, not re-offered while active | `enter_region` Castle.Infirmary | world 8; 704 completed; 1521 not_active; archetype 8 | `accept_mission` 1521; other archetypes get nothing | CS-03c1 (entry) |
| 1370 advances 4455 -> 4456 on accept, never `complete_objective 5175` | `mission_accepted` 1521 | world 8; step 4455 active | `advance_step` 4456 (credits 5175 by `advance_step`) | CS-03c1 (entry) |
| tank loads: template 413, spawn 13802 tag unique, on mesh, population guard green | player load (spawn) | template 413; spawn 13802 | tag `Castle_SymbioteTank` unique; spawn on mesh; guard green | CS-03c1 (entry) |
| 1374 at 4456 starts Livewire with `difficulty 1` and `on_victory_chains [1375]`; no-op at 4455/4457/4458 | `interact_tag` Castle_SymbioteTank | world 8; step 4456 active | `start_minigame` Livewire; no-op at the other steps | CS-03c1 (entry) |
| 1374/1376 pairwise disjoint on the shared trigger key | `interact_tag` Castle_SymbioteTank | steps 4456 vs 4457 | exactly one of 1374/1376 fires per step | CS-03c2 (graph) |
| 1375 victory: credits 5176, step 4457, dialog 4990 displayed | direct invoke (no trigger) | world 8; step 4456 active | `advance_step` 4457 (credits 5176); `display_dialog` 4990 | CS-03c2 (graph) |
| 1376 at 4457: credits 5177, step 4458, 4991 opens | `interact_tag` Castle_SymbioteTank | world 8; step 4457 active | `advance_step` 4458 (credits 5177); `display_dialog` 4991 | CS-03c2 (graph) |
| 1377 on 4991 close: 5178 credited, 1521 completed; re-fire does nothing | `dialog_choice` 4991 | world 8; step 4458 active | `complete_mission` 1521 (credits 5178); second event no-op | CS-03c2 (graph) |
| relog at each of 4456/4457/4458 resumes the click path | `player_loaded` | steps 4456/4457/4458 | no restore chain; click re-fires the matching chain | CS-03c2 (graph) |
| `find_path` Ogilvie -> tank succeeds (component 250) | nav query | Ogilvie spawn; tank spawn 13802 | path exists | CS-03c2 (graph) |

## 6. End UAT rows (owner / lab, not run here)

1. Tank is visible and clickable in the Infirmary; mission-object highlight shows at 4456/4457 (bit 30 UATPending).
2. Livewire window opens and the victory returns to the chain; defeat allows a re-click.
3. Dialog 4990 appears as the player's own narration.
4. Dialog 4991 opens on the second click; the speaker label (tank vs Moh'katan) is acceptable.
5. Journal text for steps 4456-4458 reads sensibly with no implant mechanic.
6. Non-Jaffa archetypes never receive the offer.

## 7. Option A deltas (not frozen)

- `add_item 5169 {"container":2}` on completion (item 5169 -> event set 1987 -> ability 1927 -> effect 2480/2481).
- Optionally `apply 1926` (debuff 2480) on accept.
- Both need G-SYM-GAME, G-SYM-CONSUME, G-SYM-DEBUFF and the radio gate G-SYM-RADIO to close; the radio (5168) use is client-inert (#334).

## 8. Verdict

`SEALED`