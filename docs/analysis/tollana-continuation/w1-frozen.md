# Tollana W1 frozen literals

> **Title**: Tollana W1 frozen literals
> **Type**: Reference
> **Audience**: packet-coder and packet-reviewer for the W1 packets
> **Last updated**: 2026-10-17
> **Companions**: [W1 packets](w1-packets.md), [568 chain spec](evidence/T-03a-568-chain-spec.md), [offline placement](evidence/T-01-offline-placement.md), [phasing](evidence/phasing.md)

Copy these blocks exactly. A packet may not change a literal; if one fails, the packet returns `blocked` with the error. Every row here is CIMMERIA_RECONSTRUCTION unless marked ORIGINAL_DATA. Coordinates are offline navmesh candidates; the in-client check is UATPending.

## F1. Templates 420 and 421 (T-02a)

New file `db/resources/Entities/Seed/entity_templates_tollana.sql`. No sequence footer: `entity_templates.sql` keeps its own floor at 1399.

```sql
-- Tollana (World 19) cast templates, campaign tollana-continuation W1 (T-02a).
--
-- CIMMERIA_RECONSTRUCTION. Evidence and decisions:
-- docs/analysis/tollana-continuation/w1-packets.md (D-TW6, D-TW7) and
-- docs/analysis/tollana-continuation/evidence/T-01-offline-placement.md.
--   420 Tollana_Copplemann: identity copied from template 48 (CmdCenter
--       Copplemann). 48 is NOT reused: Harset bindings 6503/6504 key on its tag
--       and slot. event_set 570 is the universal NPC event set, kept as on 48.
--   421 Tollana food-duplication unit: one shared prop template for spawns
--       19002-19004, modelled on Dakara template 445 (flags 4,
--       INT_MissionWorldObject 1073741824, class being). Mesh ViewScreen02 is a
--       placeholder; the swap to the HT food-unit mesh is a UAT-time change.

SET search_path = resources, pg_catalog;

INSERT INTO entity_templates (template_id, static_mesh, body_set, components, flags, interaction_type, event_set_id, level, alignment, faction, name_id, name, patrol_path_id, patrol_point_delay, template_name, class, buy_item_list, sell_item_list, repair_item_list, recharge_item_list, ability_set_id, ammo_type, loot_table_id, primary_color_id, secondary_color_id, skin_tint, weapon_item_id, static_interaction_sets, trainer_ability_list_id, speaker_id, has_dynamic_properties, interaction_set_id, move_speed, use_cover, respawn_secs, display_name) VALUES
(420, NULL, 'BS_HumanFemale.BS_HumanFemale', '{AR_H_Clothing00.AR_HM_PL1_PL100AB100,AR_H_Clothing00.AR_HM_PT1_PT100,BS_HumanFemale.BS_HF_Base_Boots00_00,BS_HumanFemale.BS_HF_Base_Hands00_00,BS_HumanFemale.BS_HF_Base_Legs00_00,BS_HumanFemale.BS_HF_Base_Torso00_00,BS_HumanFemale.BS_HF_Boots_00,BS_HumanFemale.BS_HF_Head_00,NPC_Human.NPC_HF_Coppl_Hair_BC,NPC_Human.NPC_HF_Coppl_Head_BC,AR_Global.Prisoner_Boots}', 0, 0, 570, 50, 0, 1, 7035, NULL, NULL, NULL, 'Tollana - Capt. Copplemann', 'mob', NULL, NULL, NULL, NULL, NULL, NULL, NULL, 0, 0, -52773120, 21, '{}', NULL, 968, true, NULL, NULL, NULL, 300, NULL),
(421, 'Em-Props.EM-ViewScreen02', 'GLB_Components.WorldObject_Small', NULL, 4, 1073741824, NULL, 1, 0, 1, 7558, NULL, NULL, NULL, 'Tollana - Food Duplication Unit', 'being', NULL, NULL, NULL, NULL, NULL, NULL, NULL, 0, 0, 0, NULL, '{}', NULL, NULL, true, NULL, NULL, NULL, 300, NULL);
```

Expected loader values (the test asserts these): 420 class `mob`, interaction_type 0, event_set_id 570, flags 0, name_id 7035, speaker_id 968, has_dynamic_properties true, static_mesh NULL, body_set `BS_HumanFemale.BS_HumanFemale`. 421 class `being`, interaction_type 1073741824, event_set_id NULL, flags 4, name_id 7558, speaker_id NULL, has_dynamic_properties true, static_mesh `Em-Props.EM-ViewScreen02`, body_set `GLB_Components.WorldObject_Small`.

## F2. Spawns 19001-19004 (T-02a)

New file `db/resources/Worlds/Seed/spawnlist_tollana.sql`.

```sql
-- Tollana (World 19) mission cast, campaign tollana-continuation W1 (T-02a).
--
-- CIMMERIA_RECONSTRUCTION: offline navmesh candidates beside gate 4
-- (211.045, 0.606, -547.644; stargates.sql). In-client placement is
-- UATPending. Evidence: docs/analysis/tollana-continuation/evidence/
-- T-01-offline-placement.md. Reserve 19001-19010; 19005-19010 unused.

SET search_path = resources, pg_catalog;

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary, respawn_secs) VALUES (19001, 206.00, -0.58, -543.00, 2.3150, 19, 420, 'Tollana_Copplemann', NULL, true, 30);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary, respawn_secs) VALUES (19002, 199.50, -0.93, -540.00, 2.0250, 19, 421, 'Tollana_FoodUnit_Core', NULL, true, 30);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary, respawn_secs) VALUES (19003, 195.50, -1.01, -540.00, 2.0250, 19, 421, 'Tollana_FoodUnit_Nest', NULL, true, 30);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary, respawn_secs) VALUES (19004, 197.50, -1.01, -536.54, 2.0250, 19, 421, 'Tollana_FoodUnit_Fungus', NULL, true, 30);

SELECT pg_catalog.setval('spawnlist_spawn_id_seq', GREATEST((SELECT MAX(spawn_id) FROM spawnlist), (SELECT last_value FROM spawnlist_spawn_id_seq), 19010), true);
```

Fallback set F2-B, used **wholesale** only if the nav test fails on any primary row (change x, y, z and heading in all four rows, nothing else). If F2-B also fails, return `blocked` with both sets' failing rows.

| spawn | x | y | z | heading |
|---|---|---|---|---|
| 19001 | 205.00 | -0.56 | -544.00 | 2.1140 |
| 19002 | 218.50 | -1.01 | -536.80 | -2.7090 |
| 19003 | 214.50 | -1.01 | -536.80 | -2.7090 |
| 19004 | 216.50 | -1.01 | -533.34 | -2.7090 |

## F3. Chains 9001-9010 (T-03a1)

New file `db/resources/Content/Seed/tollana_568_chains.sql`. One statement per row.

```sql
-- Tollana (World 19) content chains: mission 568 "Finding Tollan Technology".
--
-- CIMMERIA_RECONSTRUCTION, every chain. missions.script_name is NULL for 568
-- and no 2009 chain survives. Spec:
-- docs/analysis/tollana-continuation/evidence/T-03a-568-chain-spec.md;
-- packets: docs/analysis/tollana-continuation/w1-packets.md (T-03a1, T-03a2).
-- Chain ids 9001-9018 (Tollana reserve 9000-9799). 9001-9010 Copplemann,
-- accept, turn-in, restore, abandon; 9011-9018 food-unit props.
-- Item-less pilot (D-TW1). Reward unresolved (owner O1): no grant rows.
-- Never complete_objective 2311: it auto-completes the mission.

SET search_path = resources, pg_catalog;

-- 4.1 Copplemann click (interact_tag Tollana_Copplemann); disjoint by state.

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (9001, '568 - Copplemann click, not active: offer briefing 2153', 'mission', 568, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (9001, 'interact_tag', 'Tollana_Copplemann', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (9001, 'mission_status', 568, NULL, 'eq', 'not_active', 0);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (9001, 'display_dialog', 2153, NULL, '{}', 0, 0);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (9002, '568 - Copplemann click, in progress (1964/1965): briefing 2153', 'mission', 568, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (9002, 'interact_tag', 'Tollana_Copplemann', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (9002, 'mission_status', 568, NULL, 'eq', 'active', 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (9002, 'step_status', 568, '1966', 'neq', 'active', 1);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (9002, 'display_dialog', 2153, NULL, '{}', 0, 0);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (9003, '568 - Copplemann click, step 1966: turn-in 2154', 'mission', 568, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (9003, 'interact_tag', 'Tollana_Copplemann', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (9003, 'step_status', 568, '1966', 'eq', 'active', 0);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (9003, 'display_dialog', 2154, NULL, '{}', 0, 0);

-- 4.2 Accept, turn-in, progress bind. Action order is load-bearing.

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (9004, '568 - accept on close of 2153', 'mission', 568, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (9004, 'dialog_choice', '2153', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (9004, 'world', 19, NULL, 'eq', NULL, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (9004, 'mission_status', 568, NULL, 'eq', 'not_active', 1);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (9004, 'remove_dialog_set', 2656, NULL, '{"slot": 420}', 0, 0);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (9004, 'accept_mission', 568, NULL, '{}', 0, 1);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (9004, 'advance_step', 568, '1964', '{}', 0, 2);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (9005, '568 - turn-in on close of 2154', 'mission', 568, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (9005, 'dialog_choice', '2154', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (9005, 'world', 19, NULL, 'eq', NULL, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (9005, 'step_status', 568, '1966', 'eq', 'active', 1);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (9005, 'remove_dialog_set', 6393, NULL, '{"slot": 420}', 0, 0);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (9005, 'complete_mission', 568, NULL, '{}', 0, 1);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (9006, '568 - progress bind 4959 after accept', 'mission', 568, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (9006, 'mission_accepted', '568', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (9006, 'world', 19, NULL, 'eq', NULL, 0);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (9006, 'add_dialog_set', 4959, NULL, '{"slot": 420, "mission_id": 568}', 0, 0);

-- 4.3 Tollana entry restore (binds are per player and rebuilt empty).

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (9007, '568 - Tollana entry restore: offer bind 2656', 'mission', 568, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (9007, 'player_loaded', 'Tollana', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (9007, 'world', 19, NULL, 'eq', NULL, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (9007, 'mission_status', 568, NULL, 'eq', 'not_active', 1);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (9007, 'add_dialog_set', 2656, NULL, '{"slot": 420}', 0, 0);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (9008, '568 - Tollana entry restore: progress bind 4959', 'mission', 568, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (9008, 'player_loaded', 'Tollana', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (9008, 'world', 19, NULL, 'eq', NULL, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (9008, 'mission_status', 568, NULL, 'eq', 'active', 1);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (9008, 'step_status', 568, '1966', 'neq', 'active', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (9008, 'add_dialog_set', 4959, NULL, '{"slot": 420, "mission_id": 568}', 0, 0);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (9009, '568 - Tollana entry restore: turn-in bind 6393', 'mission', 568, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (9009, 'player_loaded', 'Tollana', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (9009, 'world', 19, NULL, 'eq', NULL, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (9009, 'step_status', 568, '1966', 'eq', 'active', 1);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (9009, 'add_dialog_set', 6393, NULL, '{"slot": 420, "mission_id": 568}', 0, 0);

-- 4.4 Abandon repaint (shape of Harset 6120).

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (9010, '568 - abandon repaint: offer bind 2656', 'mission', 568, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (9010, 'mission_abandoned', '568', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (9010, 'world', 19, NULL, 'eq', NULL, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (9010, 'mission_status', 568, NULL, 'eq', 'not_active', 1);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (9010, 'remove_dialog_set', 4959, NULL, '{"slot": 420}', 0, 0);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (9010, 'remove_dialog_set', 6393, NULL, '{"slot": 420}', 0, 1);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (9010, 'add_dialog_set', 2656, NULL, '{"slot": 420}', 0, 2);
```

## F4. Chains 9011-9018 (T-03a2, appended to the same file)

Append after chain 9010, with this heading comment.

```sql
-- 4.5 Food-unit props (interact_tag on the prop tag). No binds: template 421
-- carries INT_MissionWorldObject. A = step 1964 active, B = step 1965 active.

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (9011, '568 - Core at 1964: advance to 1966, turn-in bind', 'mission', 568, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (9011, 'interact_tag', 'Tollana_FoodUnit_Core', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (9011, 'step_status', 568, '1964', 'eq', 'active', 0);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (9011, 'advance_step', 568, '1965', '{}', 0, 0);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (9011, 'advance_step', 568, '1966', '{}', 0, 1);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (9011, 'remove_dialog_set', 4959, NULL, '{"slot": 420}', 0, 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (9011, 'add_dialog_set', 6393, NULL, '{"slot": 420, "mission_id": 568}', 0, 3);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (9011, 'display_dialog', 4211, NULL, '{}', 0, 4);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (9012, '568 - Core at 1965: advance to 1966, turn-in bind', 'mission', 568, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (9012, 'interact_tag', 'Tollana_FoodUnit_Core', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (9012, 'step_status', 568, '1965', 'eq', 'active', 0);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (9012, 'advance_step', 568, '1966', '{}', 0, 0);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (9012, 'remove_dialog_set', 4959, NULL, '{"slot": 420}', 0, 1);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (9012, 'add_dialog_set', 6393, NULL, '{"slot": 420, "mission_id": 568}', 0, 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (9012, 'display_dialog', 4211, NULL, '{}', 0, 3);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (9013, '568 - Core, other states: dialog 4211 only', 'mission', 568, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (9013, 'interact_tag', 'Tollana_FoodUnit_Core', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (9013, 'step_status', 568, '1964', 'neq', 'active', 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (9013, 'step_status', 568, '1965', 'neq', 'active', 1);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (9013, 'display_dialog', 4211, NULL, '{}', 0, 0);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (9014, '568 - Nest at 1964: advance to 1965, credit 2312', 'mission', 568, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (9014, 'interact_tag', 'Tollana_FoodUnit_Nest', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (9014, 'step_status', 568, '1964', 'eq', 'active', 0);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (9014, 'advance_step', 568, '1965', '{}', 0, 0);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (9014, 'complete_objective', 568, '2312', '{}', 0, 1);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (9014, 'display_dialog', 4213, NULL, '{}', 0, 2);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (9015, '568 - Nest at 1965: credit 2312', 'mission', 568, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (9015, 'interact_tag', 'Tollana_FoodUnit_Nest', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (9015, 'step_status', 568, '1965', 'eq', 'active', 0);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (9015, 'complete_objective', 568, '2312', '{}', 0, 0);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (9015, 'display_dialog', 4213, NULL, '{}', 0, 1);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (9016, '568 - Nest, other states: dialog 4213 only', 'mission', 568, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (9016, 'interact_tag', 'Tollana_FoodUnit_Nest', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (9016, 'step_status', 568, '1964', 'neq', 'active', 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (9016, 'step_status', 568, '1965', 'neq', 'active', 1);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (9016, 'display_dialog', 4213, NULL, '{}', 0, 0);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (9017, '568 - Fungus at 1964: advance to 1965', 'mission', 568, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (9017, 'interact_tag', 'Tollana_FoodUnit_Fungus', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (9017, 'step_status', 568, '1964', 'eq', 'active', 0);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (9017, 'advance_step', 568, '1965', '{}', 0, 0);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (9017, 'display_dialog', 4214, NULL, '{}', 0, 1);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (9018, '568 - Fungus, other states: dialog 4214 only', 'mission', 568, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (9018, 'interact_tag', 'Tollana_FoodUnit_Fungus', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (9018, 'step_status', 568, '1964', 'neq', 'active', 0);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (9018, 'display_dialog', 4214, NULL, '{}', 0, 0);
```

## F5. Expected shapes (tests assert these)

`Action` is `cimmeria_content_engine::actions::Action`. Slot is always 420. `cond` = expected `chain.conditions.len()`.

| Chain | Trigger (type, key) | cond | Actions in order |
|---|---|---|---|
| 9001 | InteractTag `Tollana_Copplemann` | 1 | `DisplayDialog { dialog_id: 2153 }` |
| 9002 | InteractTag `Tollana_Copplemann` | 2 | `DisplayDialog { dialog_id: 2153 }` |
| 9003 | InteractTag `Tollana_Copplemann` | 1 | `DisplayDialog { dialog_id: 2154 }` |
| 9004 | DialogChoice `2153` | 2 | `RemoveDialogSet { dialog_set_id: 2656, slot: 420 }`, `AcceptMission { mission_id: 568 }`, `AdvanceStep { mission_id: 568, step_id: 1964 }` |
| 9005 | DialogChoice `2154` | 2 | `RemoveDialogSet { dialog_set_id: 6393, slot: 420 }`, `CompleteMission { mission_id: 568 }` |
| 9006 | MissionAccepted `568` | 1 | `AddDialogSet { dialog_set_id: 4959, slot: 420, mission_id: Some(568) }` |
| 9007 | PlayerLoaded `Tollana` | 2 | `AddDialogSet { dialog_set_id: 2656, slot: 420, mission_id: None }` |
| 9008 | PlayerLoaded `Tollana` | 3 | `AddDialogSet { dialog_set_id: 4959, slot: 420, mission_id: Some(568) }` |
| 9009 | PlayerLoaded `Tollana` | 2 | `AddDialogSet { dialog_set_id: 6393, slot: 420, mission_id: Some(568) }` |
| 9010 | MissionAbandoned `568` | 2 | `RemoveDialogSet { 4959, 420 }`, `RemoveDialogSet { 6393, 420 }`, `AddDialogSet { 2656, 420, None }` |
| 9011 | InteractTag `Tollana_FoodUnit_Core` | 1 | `AdvanceStep { 568, 1965 }`, `AdvanceStep { 568, 1966 }`, `RemoveDialogSet { 4959, 420 }`, `AddDialogSet { 6393, 420, Some(568) }`, `DisplayDialog { 4211 }` |
| 9012 | InteractTag `Tollana_FoodUnit_Core` | 1 | `AdvanceStep { 568, 1966 }`, `RemoveDialogSet { 4959, 420 }`, `AddDialogSet { 6393, 420, Some(568) }`, `DisplayDialog { 4211 }` |
| 9013 | InteractTag `Tollana_FoodUnit_Core` | 2 | `DisplayDialog { 4211 }` |
| 9014 | InteractTag `Tollana_FoodUnit_Nest` | 1 | `AdvanceStep { 568, 1965 }`, `CompleteObjective { mission_id: 568, objective_id: 2312 }`, `DisplayDialog { 4213 }` |
| 9015 | InteractTag `Tollana_FoodUnit_Nest` | 1 | `CompleteObjective { 568, 2312 }`, `DisplayDialog { 4213 }` |
| 9016 | InteractTag `Tollana_FoodUnit_Nest` | 2 | `DisplayDialog { 4213 }` |
| 9017 | InteractTag `Tollana_FoodUnit_Fungus` | 1 | `AdvanceStep { 568, 1965 }`, `DisplayDialog { 4214 }` |
| 9018 | InteractTag `Tollana_FoodUnit_Fungus` | 1 | `DisplayDialog { 4214 }` |

Fired-chain table per state (resolve level, ids filtered to 9001..=9018). States: S0 = mission not_active; S1964, S1965, S1966 = that step active, earlier steps completed; SDone = mission completed.

| Event | S0 | S1964 | S1965 | S1966 | SDone |
|---|---|---|---|---|---|
| InteractTag Copplemann | 9001 | 9002 | 9002 | 9003 | none |
| InteractTag Core | 9013 | 9011 | 9012 | 9013 | 9013 |
| InteractTag Nest | 9016 | 9014 | 9015 | 9016 | 9016 |
| InteractTag Fungus | 9018 | 9017 | 9018 | 9018 | 9018 |
| DialogChoice 2153 | 9004 | none | none | none | none |
| DialogChoice 2154 | none | none | none | 9005 | none |
| PlayerLoaded Tollana | 9007 | 9008 | 9008 | 9009 | none |
| MissionAbandoned 568 | 9010 | none | none | none | none |
| MissionAccepted 568 | 9006 | 9006 | 9006 | 9006 | 9006 |

Expected slot-420 binds after each executor step (dialog_set_map ids): S0 after entry `[2656]`; after accept `[4959]`; after Nest or Fungus `[4959]`; after Core `[6393]`; after turn-in `[]`; after abandon from any step `[2656]`.

## F6. Phase model names (PH-01a, PH-01b)

| Item | Exact name and shape |
|---|---|
| module | `crates/entity/src/cell_entity/phase.rs`, private `mod phase;` re-exported from `cell_entity` |
| key | `pub struct PhaseKey(String)`; `PhaseKey::new(&str) -> Result<PhaseKey, PhaseKeyError>`; `PhaseKey::MAX_LEN: usize = 64`; `as_str(&self) -> &str`; `Display`; derives `Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord` |
| key rule | 1 to 64 chars, each in `[a-z0-9_]` |
| error | `pub enum PhaseKeyError { Empty, TooLong(usize), BadChar(char) }`, derives `Debug, Clone, PartialEq, Eq`; implements `Display` and `std::error::Error` |
| state | `pub struct PhaseState { pub entity_phase: Option<PhaseKey>, pub player_phases: HashSet<PhaseKey> }`, derives `Debug, Clone, Default, PartialEq, Eq`; stored in `CellEntity.extensions` |
| visibility | `pub fn phase_visible(viewer_phases: &HashSet<PhaseKey>, other: &CellEntity) -> bool` |
| viewer set | `pub fn viewer_phases(entity: &CellEntity) -> HashSet<PhaseKey>` |
| setter | `SpaceManager::set_player_phase(&mut self, player_id: u32, phase: &PhaseKey, member: bool) -> Result<Vec<CellToBaseMsg>, PhaseError>` |
| setter error | `pub enum PhaseError { PlayerMissing, NotAPlayer }` in `crates/cell-world/src/cell/space_manager/phase.rs` |
| telemetry | INFO, `event = "phase.changed"`, fields `player_id`, `phase`, `member`, `entered`, `left` |
