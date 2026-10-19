# UB-02a: Beta cast and object templates 2300-2335

> **Type:** How-to (work packet) · **Index:** [beta-work-packets.md](../beta-work-packets.md) · **Wave:** B1

| Field | Value |
|---|---|
| Role | `packet-coder` |
| Size | about 40k |
| Branch / worktree | `union-beta-e1/ub-02a-templates` / `ub-02a` |
| Commit subject | `feat(beta-e1): UB-02a Beta cast and object templates named by the client's own strings` |
| Depends on | UB-00a question 10 (object meshes) and UB-00c (ID block clear). Runs alongside UB-01b |
| Ship order | Before UB-02b, UB-02c and UB-02d, which spawn these templates |

## Why

Beta has no NPCs. Chains bind dialogs per player using `slot` = the NPC's **template id** (F14). Templates used only on Beta keep those binds from lighting up Carter (template 33) on SGC_W1 or Thor (64) on Omega. Every template is named by the client's own Beta display-name string in `texts.sql` (RESTORED_SOURCE). The look of each one is copied from an existing template (CIMMERIA_RECONSTRUCTION).

## Contract excerpt

- Column list and row style: copy `db/resources/Entities/Seed/entity_templates_dakara_e1.sql`, which uses the long column list ending `use_cover, respawn_secs, display_name`. NPCs are `class 'mob'`; objects are `class 'being'`.
- NPC look sources (`entity_templates.sql`): 33 Sam Carter (`BS_HumanFemale`, SamCarter head, speaker 708), 31 Airman (`BS_HumanMale`), 64 Thor (`BS_Asgard`). From `entity_templates_dakara_e1.sql`: 440 Loth'ta (`BS_JaffaMale`, armour and staff) and 444 Free Jaffa Warrior.
- Object look source: 445 (`static_mesh 'GLB-Global.GLB-RingTransporterSwitch_00'`, `body_set 'GLB_Components.WorldObject_WallTerminal'`, `flags 4`, `interaction_type 1073741824` = INT_MissionWorldObject). **Use UB-00a's per-template mesh row where it gives one**, and keep 445's values where it says "default".
- Friendly settings for every row: `faction 1`, `alignment 0`, `event_set_id 570` for NPCs (as 33) and NULL for objects (as 445), `ability_set_id NULL` (Ras'nor and the sick Jaffa drop 440's `4`), `loot_table_id NULL`, no vendor, trainer or weapon lists, `respawn_secs 300`, `use_cover NULL`.
- Guard precedent: `crates/cell-world/src/cell/spawner_tests/dakara_e1/templates.rs`. Its tests cover the roster, shows-a-name, not hostile, visual and spawn settings, and clickability. It is registered through `spawner_tests/dakara_e1/mod.rs`, with `mod dakara_e1;` at `spawner_tests/mod.rs:40`.

## Rules (binding)

1. PowerShell only; start every command with `Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\ub-02a;`.
2. Never run `cargo` directly; use `pwsh tools/build-lane/lane.ps1` and `pwsh tools/build-lane/live-db-test.ps1`. Run in the foreground and check that test counts are above 0.
3. Edit only the allowlist.
4. The seed file is CRLF (normalise after writing). Keep `db/database.sql`'s existing line endings.
5. Seeds in `db/resources/` only; no migrations; never edit `entity_templates.sql` or the Dakara file.
6. Under 500 lines per file. If `templates.rs` grows past 450, split it into `templates.rs` and `templates_props.rs`.
7. Never touch `sgw`; no lab, no client, no Ghidra (UB-00a has already done the mesh census).
8. No `git stash`, prune or push; one local commit.
9. Above each row, write a comment with: the template's role, its moniker as `name_id <id> '<moniker_name>'` labelled RESTORED_SOURCE, and `look: CIMMERIA_RECONSTRUCTION, copied from <template>`.
10. Templates 2303 (Dr. Pierce, 7905), 2327 (Accelerant, 7381) and 2336 are **reserved and not created**. They are for UB-52's twins 383 and 384. Put a comment line in the file saying so.

## Allowlist

| File | Change |
|---|---|
| `db/resources/Entities/Seed/entity_templates_beta_e1.sql` | new |
| `db/database.sql` | `\ir resources/Entities/Seed/entity_templates_beta_e1.sql` directly after line 320 (`entity_templates_dakara_e1.sql`) |
| `crates/cell-world/src/cell/spawner_tests/beta_e1/mod.rs` | new |
| `crates/cell-world/src/cell/spawner_tests/beta_e1/templates.rs` | new |
| `crates/cell-world/src/cell/spawner_tests/mod.rs` | `mod beta_e1;` before `mod dakara_e1;` |

## Steps

1. **NPC rows** (`class 'mob'`, `interaction_type 0`, `flags 0`):

   | Template | template_name | name_id (moniker_name) | display_name | speaker_id | Look copied from | level |
   |---|---|---|---|---|---|---|
   | 2300 | `Beta E1 - Lt. Col. Carter` | 7366 `DN_npc_MG_BetaE1_Admin_Carter` | NULL | 708 | 33 (body, components, skin_tint) | 50 |
   | 2301 | `Beta E1 - Dr. Felger` | 7368 `DN_npc_MG_BetaE1_Admin_Felger` (text is empty) | `'Dr. Felger'` | 710 | 31 | 50 |
   | 2302 | `Beta E1 - Captain Satterfield` | 7373 `DN_npc_MG_BetaE1_Admin_Satterfield` | NULL | 715 | 31 | 50 |
   | 2304 | `Beta E1 - Lt. Debeaux` | 8552 `DN_NPC_Int_BetaE1_Admin_Debeaux` | NULL | NULL (no speaker exists) | 31 | 50 |
   | 2305 | `Beta E1 - Mimir` | 7388 `DN_npc_MG_BetaE1_AsgardBase_Mimir` | NULL | 730 | 64 | 50 |
   | 2306 | `Beta E1 - Ras''nor` | 7374 `DN_npc_MG_BetaE1_RasnorsCamp_Rasnor` | NULL | 716 | 440 (drop `WP-Jaffa.WP_Staff_Plasma_4A` from components; nobody fights on a first visit) | 50 |
   | 2307 | `Beta E1 - Sick Nine Peaks Warrior` | 7414 `DN_npc_Int_BetaE1_Various_SickJaffa` | NULL | NULL | 444 (drop the staff) | 5 |

   Felger's moniker text is empty in the client, so its `display_name` is the speaker 710 name (RESTORED_SOURCE from `speakers.sql`).

2. **Object rows** (`class 'being'`, `flags 4`, `interaction_type 1073741824`, `speaker_id NULL`, `level 1`). The default mesh and body set are 445's unless UB-00a gives a row.

   | Template | template_name | name_id (moniker_name) | Used by (packet) |
   |---|---|---|---|
   | 2320 | `Beta E1 - Xenobiology Sample Cage` | 7376 `DN_Ob_It_Human_BetaE1_XenobiologyCage` | UB-11 |
   | 2321 | `Beta E1 - Artifact Display Case` | 7377 `DN_Ob_It_Human_BetaE1_FourPiecesCase` | UB-15 |
   | 2322 | `Beta E1 - Beta Site Hub Computer` | 7383 `DN_Ob_D_Human_BetaE1_HubComputer` | UB-14, UB-15 |
   | 2323 | `Beta E1 - Admin Hub Workstation` | 8493 `DN_Ob_It_Human_BetaE1_AdminWorkstation` | UB-13 |
   | 2324 | `Beta E1 - Science Center Computer` | 8489 `DN_Ob_D_Human_BetaE1_ScienceCenterComputer_02SGUDatabase` | UB-14 |
   | 2325 | `Beta E1 - Wrecked SGC Shipping Crates` | 8490 `DN_Ob_It_Human_BetaE1_CrateWreckage` | UB-12 |
   | 2326 | `Beta E1 - Box of Jaffa Tools` | 7402 `DN_Ob_D_Jaffa_BetaE1_BoxOfJaffaTools` | UB-21 |
   | 2328 | `Beta E1 - Asgard Fire Suppressant Manager` | 7397 `DN_Ob_it_Asgard_BetaE1_FireSuppressantManager` | UB-34 |
   | 2329 | `Beta E1 - Fire Control Hub` | 7957 `DN_Ob_G_Human_BetaE1_SprinklerSystemManularControl` | UB-34 |
   | 2330 | `Beta E1 - Environmental Reading Device` | 7392 `DN_Ob_It_Asgard_BetaE1_AsgardScanner` | UB-31 |
   | 2331 | `Beta E1 - Analytical Assistance Machine` | 7909 `DN_Ob_G_ASG_BetaE1_AnalyticalAssistanceMachine` | UB-33, UB-36 |
   | 2332 | `Beta E1 - Missing Asgard Hologram Emitter` | 8164 `DN_Ob_It_Asgard_BetaE1_MissingAsgardHologramEmitter` | UB-32 |
   | 2333 | `Beta E1 - Disabled Malfunctioning Drone` | 8211 `DN_Ob_G_Human_BetaE1_DisabledMalfunctioningDrone` | UB-32. Look: body set and components from template 69 `MOB_Goauld_Drone`, with `static_mesh NULL`, but **class `'being'`** and the object interaction bit. This is the one prop that draws a body; the visual test allows it by id |
   | 2334 | `Beta E1 - Consciousness Simulator` | 8497 `DN_Ob_It_Asgard_BetaE1_ConsciousnessSimulator` | UB-36 |
   | 2335 | `Beta E1 - Note at the ruins` | name_id NULL; `display_name 'Note'` (CIMMERIA_RECONSTRUCTION: dialog 4663 is the "post-it" object dialog, and the client has no moniker for it) | UB-16 |

3. **Tail:** if the Dakara file ends with a `setval` for a templates sequence, copy that line with `2335`. If it has none, add none.

4. **database.sql.** One `\ir` line after line 320.

5. **Tests** (`spawner_tests/beta_e1/templates.rs`). Copy the Dakara module shape: `block_templates`, `spawned`, and the `super::*` imports. `beta_e1/mod.rs` holds `const BLOCK: (i32, i32) = (2300, 2349);` and `mod templates;`, plus whatever `use` lines the Dakara `mod.rs` carries. Every test is `#[tokio::test]`, starts `require_db_or_skip!()`, and has a `///` revert line.
   - `live_db_beta_e1_cast_and_objects_match_the_roster`: pin `(template id, class, moniker_name)` for every row in steps 1-2. 2335 pins `None` for the moniker. Revert: remove the `\ir` line, or change a `name_id`.
   - `live_db_every_beta_e1_template_shows_a_name`: copy the Dakara test. Exactly one of non-empty text and `display_name` holds for each row. 2301 and 2335 are the literals.
   - `live_db_no_beta_e1_template_is_hostile_or_attackable`: copy it exactly (Union players must never aggro the Beta cast).
   - `live_db_every_beta_e1_template_has_visual_and_spawn_settings`: copy it, with one change. Mob rows draw a body. Object rows draw a mesh, **except 2333**, which draws a body and is pinned by id with a comment.
   - `live_db_beta_e1_objects_are_mission_world_objects_and_npcs_are_not`: each object template has the `INT_MISSION_WORLD_OBJECT` bit (`cimmeria_entity::interaction_flags`), and each NPC template has interaction_type 0. Chains reach NPCs through per-player binds only (D-UB10). Revert: set 2300's `interaction_type` to 1073741824.
   - `live_db_beta_e1_reserved_ids_are_absent`: 2303, 2327 and 2336 have no row. This stops a coder from placing Pierce before D-UB7.

6. **Revert proof.** Comment out the `\ir` line and rerun `spawner_tests::beta_e1`; it must fail. Restore it. Then clear 2301's `display_name`; the shows-a-name test must fail. Restore and rerun green. Quote all of it in the report.

## Lane checks (in order)

```powershell
Set-Location <wt>; pwsh tools/build-lane/lane.ps1 cargo fmt -p cimmeria-cell-world
Set-Location <wt>; pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-cell-world --all-targets -- -D warnings
Set-Location <wt>; pwsh tools/build-lane/live-db-test.ps1 spawner_tests::beta_e1
Set-Location <wt>; pwsh tools/build-lane/live-db-test.ps1 spawner_tests::dakara_e1
```

The Dakara rerun proves the shared loader still reads both blocks.

## Acceptance

- Six template tests green; the Dakara template tests are still green.
- SigNoz: not reached until UB-02b spawns these templates. UB-90 row UB-U2 checks the nameplates live (Carter shows "Lt. Col. Carter" and Felger shows "Dr. Felger"). This packet adds no log line.

## Docs rows

None. Template seeds have no doc-update-map row. UB-99 adds the cast to the Beta content doc.

## Return

`UB-02a <done|blocked|failed> <sha|-> C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\ub-02a\.claude\reports\union-beta-e1\UB-02a-coder.md`
