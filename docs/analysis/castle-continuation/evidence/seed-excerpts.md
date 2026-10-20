# CS-00 exact seed excerpts

Baseline: `d3efb7d41aad0b6008397f112d224ce7430d7acf`. These are current seed facts; their presence alone does not prove a 2009 runtime binding.

## Excerpts

`db/resources/Missions/Seed/mission_steps.sql:1108`

```sql
INSERT INTO mission_steps (step_id, mission_id, award_xp, difficulty, step_enabled, step_display_log_text, index) VALUES (4037, 1360, false, 1, false, 'Find a way to get Cpl. Frost''s Letter to his family.', 0);
```

`db/resources/Missions/Seed/mission_steps.sql:1110`

```sql
INSERT INTO mission_steps (step_id, mission_id, award_xp, difficulty, step_enabled, step_display_log_text, index) VALUES (4038, 1360, false, 1, false, 'Give Cpl. Frost''s Letter to Col. Marsh.', 1);
```

`db/resources/Missions/Seed/mission_steps.sql:1250`

```sql
INSERT INTO mission_steps (step_id, mission_id, award_xp, difficulty, step_enabled, step_display_log_text, index) VALUES (4098, 1376, false, 1, false, 'Rescue Dr. Zuritska from the Interrogation Block!', 0);
```

`db/resources/Missions/Seed/mission_steps.sql:2052`

```sql
INSERT INTO mission_steps (step_id, mission_id, award_xp, difficulty, step_enabled, step_display_log_text, index) VALUES (4452, 1520, false, 1, false, 'Secure a supply of Ambernol at the Castle''s Infirmary', 0);
```

`db/resources/Missions/Seed/mission_steps.sql:2054`

```sql
INSERT INTO mission_steps (step_id, mission_id, award_xp, difficulty, step_enabled, step_display_log_text, index) VALUES (4453, 1520, false, 1, false, 'Hack the Med Dispenser to release the Infirmary''s supply of Ambernol.', 1);
```

`db/resources/Missions/Seed/mission_steps.sql:2056`

```sql
INSERT INTO mission_steps (step_id, mission_id, award_xp, difficulty, step_enabled, step_display_log_text, index) VALUES (4454, 1520, false, 1, false, 'Use your radio to report to Capt. Copplemann.', 2);
```

`db/resources/Missions/Seed/mission_steps.sql:2058`

```sql
INSERT INTO mission_steps (step_id, mission_id, award_xp, difficulty, step_enabled, step_display_log_text, index) VALUES (4455, 1521, false, 1, false, 'Make your way to the Infirmary at the end of the Hallway.', 0);
```

`db/resources/Missions/Seed/mission_steps.sql:2060`

```sql
INSERT INTO mission_steps (step_id, mission_id, award_xp, difficulty, step_enabled, step_display_log_text, index) VALUES (4456, 1521, false, 1, false, 'Bypass the security protocols on the Symbiote Storage tank.', 1);
```

`db/resources/Missions/Seed/mission_steps.sql:2062`

```sql
INSERT INTO mission_steps (step_id, mission_id, award_xp, difficulty, step_enabled, step_display_log_text, index) VALUES (4457, 1521, false, 1, false, 'Implant a symbiote in your pouch.', 2);
```

`db/resources/Missions/Seed/mission_steps.sql:2064`

```sql
INSERT INTO mission_steps (step_id, mission_id, award_xp, difficulty, step_enabled, step_display_log_text, index) VALUES (4458, 1521, false, 1, false, 'Use the Tau''ri radio to report to Moh''katan.', 3);
```

`db/resources/Missions/Seed/mission_steps.sql:2879`

```sql
INSERT INTO mission_steps (step_id, mission_id, award_xp, difficulty, step_enabled, step_display_log_text, index) VALUES (5224, 1664, false, 1, false, 'Speak to Sgt. Kaerlia by the Stargate for further instructions.', 0);
```

`db/resources/Missions/Seed/mission_steps.sql:2881`

```sql
INSERT INTO mission_steps (step_id, mission_id, award_xp, difficulty, step_enabled, step_display_log_text, index) VALUES (5225, 1664, false, 1, false, 'Make your way to the Command Center.', 1);
```

`db/resources/Missions/Seed/mission_steps.sql:2883`

```sql
INSERT INTO mission_steps (step_id, mission_id, award_xp, difficulty, step_enabled, step_display_log_text, index) VALUES (5226, 1664, false, 1, false, 'Report for debriefing.', 2);
```

`db/resources/Missions/Seed/mission_steps.sql:5199`

```sql
INSERT INTO mission_steps (step_id, mission_id, award_xp, difficulty, step_enabled, step_display_log_text, index) VALUES (2000, 567, false, 2, false, 'Retrieve Romney''s Files.', 0);
```

`db/resources/Missions/Seed/mission_steps.sql:5201`

```sql
INSERT INTO mission_steps (step_id, mission_id, award_xp, difficulty, step_enabled, step_display_log_text, index) VALUES (2012, 567, false, 1, false, 'Get Romney''s Files safely off of the Castle.', 1);
```

`db/resources/Missions/Seed/mission_steps.sql:5203`

```sql
INSERT INTO mission_steps (step_id, mission_id, award_xp, difficulty, step_enabled, step_display_log_text, index) VALUES (4039, 567, false, 2, false, 'Deliver Romney''s Files to Copplemann', 2);
```

`db/resources/Missions/Seed/mission_objectives.sql:1169`

```sql
INSERT INTO mission_objectives (objective_id, step_id, award_xp, difficulty, is_enabled, is_hidden, is_optional, display_log_text) VALUES (4650, 4037, false, 1, false, false, false, ' ');
```

`db/resources/Missions/Seed/mission_objectives.sql:1171`

```sql
INSERT INTO mission_objectives (objective_id, step_id, award_xp, difficulty, is_enabled, is_hidden, is_optional, display_log_text) VALUES (4651, 4038, false, 1, false, false, false, ' ');
```

`db/resources/Missions/Seed/mission_objectives.sql:2126`

```sql
INSERT INTO mission_objectives (objective_id, step_id, award_xp, difficulty, is_enabled, is_hidden, is_optional, display_log_text) VALUES (5171, 4452, false, 1, false, false, false, 'Make your way to the Infirmary at the end of the Hallway.');
```

`db/resources/Missions/Seed/mission_objectives.sql:2128`

```sql
INSERT INTO mission_objectives (objective_id, step_id, award_xp, difficulty, is_enabled, is_hidden, is_optional, display_log_text) VALUES (5172, 4452, false, 1, false, true, false, 'Speak to Ogilvie to learn where the Ambernol is stored.');
```

`db/resources/Missions/Seed/mission_objectives.sql:2130`

```sql
INSERT INTO mission_objectives (objective_id, step_id, award_xp, difficulty, is_enabled, is_hidden, is_optional, display_log_text) VALUES (5173, 4453, false, 1, false, false, false, ' ');
```

`db/resources/Missions/Seed/mission_objectives.sql:2132`

```sql
INSERT INTO mission_objectives (objective_id, step_id, award_xp, difficulty, is_enabled, is_hidden, is_optional, display_log_text) VALUES (5174, 4454, false, 1, false, false, false, ' ');
```

`db/resources/Missions/Seed/mission_objectives.sql:2134`

```sql
INSERT INTO mission_objectives (objective_id, step_id, award_xp, difficulty, is_enabled, is_hidden, is_optional, display_log_text) VALUES (5175, 4455, false, 1, false, false, false, ' ');
```

`db/resources/Missions/Seed/mission_objectives.sql:2136`

```sql
INSERT INTO mission_objectives (objective_id, step_id, award_xp, difficulty, is_enabled, is_hidden, is_optional, display_log_text) VALUES (5176, 4456, false, 1, false, false, false, ' ');
```

`db/resources/Missions/Seed/mission_objectives.sql:2138`

```sql
INSERT INTO mission_objectives (objective_id, step_id, award_xp, difficulty, is_enabled, is_hidden, is_optional, display_log_text) VALUES (5177, 4457, false, 1, false, false, false, ' ');
```

`db/resources/Missions/Seed/mission_objectives.sql:2140`

```sql
INSERT INTO mission_objectives (objective_id, step_id, award_xp, difficulty, is_enabled, is_hidden, is_optional, display_log_text) VALUES (5178, 4458, false, 1, false, false, false, ' ');
```

`db/resources/Missions/Seed/mission_objectives.sql:3164`

```sql
INSERT INTO mission_objectives (objective_id, step_id, award_xp, difficulty, is_enabled, is_hidden, is_optional, display_log_text) VALUES (6063, 5224, false, 1, false, false, false, '');
```

`db/resources/Missions/Seed/mission_objectives.sql:3166`

```sql
INSERT INTO mission_objectives (objective_id, step_id, award_xp, difficulty, is_enabled, is_hidden, is_optional, display_log_text) VALUES (6059, 5225, false, 1, false, false, true, '(Optional) Follow the path on your mini-map to reach the command center.');
```

`db/resources/Missions/Seed/mission_objectives.sql:3168`

```sql
INSERT INTO mission_objectives (objective_id, step_id, award_xp, difficulty, is_enabled, is_hidden, is_optional, display_log_text) VALUES (6060, 5225, false, 1, false, false, false, 'Interact (right-click) with the command center door to enter the command center.');
```

`db/resources/Missions/Seed/mission_objectives.sql:3170`

```sql
INSERT INTO mission_objectives (objective_id, step_id, award_xp, difficulty, is_enabled, is_hidden, is_optional, display_log_text) VALUES (6061, 5226, false, 1, false, false, false, 'Report to Col. Marsh.');
```

`db/resources/Missions/Seed/mission_objectives.sql:3172`

```sql
INSERT INTO mission_objectives (objective_id, step_id, award_xp, difficulty, is_enabled, is_hidden, is_optional, display_log_text) VALUES (6062, 5226, false, 1, false, false, false, 'Report to Moh''katan');
```

`db/resources/Missions/Seed/mission_objectives.sql:5737`

```sql
INSERT INTO mission_objectives (objective_id, step_id, award_xp, difficulty, is_enabled, is_hidden, is_optional, display_log_text) VALUES (2338, 2000, false, 2, false, false, false, ' ');
```

`db/resources/Missions/Seed/mission_objectives.sql:5739`

```sql
INSERT INTO mission_objectives (objective_id, step_id, award_xp, difficulty, is_enabled, is_hidden, is_optional, display_log_text) VALUES (2350, 2012, false, 1, false, false, false, ' ');
```

`db/resources/Missions/Seed/mission_objectives.sql:5741`

```sql
INSERT INTO mission_objectives (objective_id, step_id, award_xp, difficulty, is_enabled, is_hidden, is_optional, display_log_text) VALUES (4652, 4039, false, 2, false, false, false, ' ');
```

`db/resources/Missions/Seed/mission_tasks.sql:2267`

```sql
INSERT INTO mission_tasks (task_id, objective_id, award_xp, difficulty, is_enabled, task_type) VALUES (6372, 5171, false, 1, true, 1);
```

`db/resources/Missions/Seed/mission_tasks.sql:2269`

```sql
INSERT INTO mission_tasks (task_id, objective_id, award_xp, difficulty, is_enabled, task_type) VALUES (6373, 5172, false, 1, true, 1);
```

`db/resources/Missions/Seed/mission_tasks.sql:2271`

```sql
INSERT INTO mission_tasks (task_id, objective_id, award_xp, difficulty, is_enabled, task_type) VALUES (6374, 5172, false, 1, true, 1);
```

`db/resources/Missions/Seed/mission_tasks.sql:2273`

```sql
INSERT INTO mission_tasks (task_id, objective_id, award_xp, difficulty, is_enabled, task_type) VALUES (6375, 5173, false, 1, true, 1);
```

`db/resources/Missions/Seed/mission_tasks.sql:2275`

```sql
INSERT INTO mission_tasks (task_id, objective_id, award_xp, difficulty, is_enabled, task_type) VALUES (6376, 5174, false, 1, true, 1);
```

`db/resources/Missions/Seed/mission_tasks.sql:2277`

```sql
INSERT INTO mission_tasks (task_id, objective_id, award_xp, difficulty, is_enabled, task_type) VALUES (6377, 5175, false, 1, true, 1);
```

`db/resources/Missions/Seed/mission_tasks.sql:2279`

```sql
INSERT INTO mission_tasks (task_id, objective_id, award_xp, difficulty, is_enabled, task_type) VALUES (6378, 5175, false, 1, true, 1);
```

`db/resources/Missions/Seed/mission_tasks.sql:2281`

```sql
INSERT INTO mission_tasks (task_id, objective_id, award_xp, difficulty, is_enabled, task_type) VALUES (6379, 5176, false, 1, true, 1);
```

`db/resources/Missions/Seed/mission_tasks.sql:2283`

```sql
INSERT INTO mission_tasks (task_id, objective_id, award_xp, difficulty, is_enabled, task_type) VALUES (6380, 5177, false, 1, true, 1);
```

`db/resources/Missions/Seed/mission_tasks.sql:2285`

```sql
INSERT INTO mission_tasks (task_id, objective_id, award_xp, difficulty, is_enabled, task_type) VALUES (6381, 5178, false, 1, true, 1);
```

Dialog rows are in [dialog-excerpts.md](dialog-excerpts.md).

`db/resources/Items/Seed/items.sql:139`

```sql
INSERT INTO items (item_id, applied_science_id, description, icon_location, name, quality_id, tech_comp, tier, container_sets, max_stack_size, moniker_ids, max_ranged_range, min_ranged_range, visual_component, max_melee_range, min_melee_range, discipline_ids, flags, ammo_types, default_ammo_type, clip_size, charges) VALUES (19, NULL, 'Ambernol counteracts stasis sickness.', 'set:ItemIcon001 image:Spray_Injector', 'Ambernol Vial', 'ITEM_QUALITY_Normal', 1, 1, '{2}', 1, '{}', 0, 0, NULL, 0, 0, '{}', 35840, '{}', NULL, 0, 0);
```

`db/resources/Items/Seed/items.sql:461`

```sql
INSERT INTO items (item_id, applied_science_id, description, icon_location, name, quality_id, tech_comp, tier, container_sets, max_stack_size, moniker_ids, max_ranged_range, min_ranged_range, visual_component, max_melee_range, min_melee_range, discipline_ids, flags, ammo_types, default_ammo_type, clip_size, charges) VALUES (2718, NULL, 'A wiggly Goa''uld symbiote without a host', 'set:CoreWidgets image:IconMissing', 'Goa''uld Symbiote', 'ITEM_QUALITY_Normal', 1, 1, '{2}', 1, '{}', 0, 0, NULL, 0, 0, '{}', 35840, '{}', NULL, 0, 0);
```

`db/resources/Items/Seed/items.sql:4609`

```sql
INSERT INTO items (item_id, applied_science_id, description, icon_location, name, quality_id, tech_comp, tier, container_sets, max_stack_size, moniker_ids, max_ranged_range, min_ranged_range, visual_component, max_melee_range, min_melee_range, discipline_ids, flags, ammo_types, default_ammo_type, clip_size, charges) VALUES (4652, NULL, 'A wiggly Goa''uld symbiote without a host', 'set:CoreWidgets image:IconMissing', 'Goa''uld Symbiote', 'ITEM_QUALITY_Normal', 1, 1, '{2}', 1, '{}', 0, 0, NULL, 0, 0, '{}', 35840, '{}', NULL, 0, 0);
```

`db/resources/Items/Seed/items.sql:11707`

```sql
INSERT INTO items (item_id, applied_science_id, description, icon_location, name, quality_id, tech_comp, tier, container_sets, max_stack_size, moniker_ids, max_ranged_range, min_ranged_range, visual_component, max_melee_range, min_melee_range, discipline_ids, flags, ammo_types, default_ammo_type, clip_size, charges) VALUES (2698, NULL, 'Romney''s Files from his Castle Office', 'set:CoreWidgets image:IconMissing', 'Romney''s Files', 'ITEM_QUALITY_Normal', 1, 1, '{2}', 1, '{}', 0, 0, NULL, 0, 0, '{}', 32768, '{}', NULL, 0, 0);
```

`db/resources/Texts/Seed/texts.sql:1058`

```sql
INSERT INTO texts (moniker_id, flags, language, moniker_name, text) VALUES (10469, 0, 1033, 'DN_Ms_A01_BetaE1_LGT_23PowerBehind', 'Power Behind the Throne');
```

`db/resources/Texts/Seed/texts.sql:1428`

```sql
INSERT INTO texts (moniker_id, flags, language, moniker_name, text) VALUES (10635, 0, 1033, 'DN_ms_A00_Castle_opc_06BehindThrone', '');
```

`db/resources/Texts/Seed/texts.sql:9033`

```sql
INSERT INTO texts (moniker_id, flags, language, moniker_name, text) VALUES (14022, 0, 1033, 'DS0_ms_A00_Castle_opc_06BehindThrone', 'Get to the Throne Room.');
```

`db/resources/Texts/Seed/texts.sql:9035`

```sql
INSERT INTO texts (moniker_id, flags, language, moniker_name, text) VALUES (14023, 0, 1033, 'DS1_ms_A00_Castle_opc_06BehindThrone', 'Hack into the Goa''uld communications grid.');
```

`db/resources/Texts/Seed/texts.sql:17596`

```sql
INSERT INTO texts (moniker_id, flags, language, moniker_name, text) VALUES (18227, 0, 1033, 'DO0-1_ms_A00_Castle_opc_06BehindThrone_2790', '');
```

`db/resources/Texts/Seed/texts.sql:17598`

```sql
INSERT INTO texts (moniker_id, flags, language, moniker_name, text) VALUES (18228, 0, 1033, 'DO1-1_ms_A00_Castle_opc_06BehindThrone_2791', 'Use the access panel behind the throne to boost Zuritska''s communications signal.');
```

`db/resources/Texts/Seed/texts.sql:17600`

```sql
INSERT INTO texts (moniker_id, flags, language, moniker_name, text) VALUES (18229, 0, 1033, 'DO1-2_ms_A00_Castle_opc_06BehindThrone_2792', 'Locate the access panel behind the Throne.');
```

`db/resources/Texts/Seed/texts.sql:37229`

```sql
INSERT INTO texts (moniker_id, flags, language, moniker_name, text) VALUES (27315, 0, 1033, 'DN_npc_int_SgtKaerlia_Harset', 'Sgt. Kaerlia');
```
