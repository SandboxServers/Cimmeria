# UB-02b, UB-02c, UB-02d: Beta spawns and regions

> **Type:** How-to (work packets) · **Index:** [beta-work-packets.md](../beta-work-packets.md) · **Wave:** B1b

Three packets, one area each. Each packet owns its own three seed files, so they can run in parallel. They share only `db/database.sql` and `spawner_tests/beta_e1/mod.rs`, and each adds its own lines (keep both sides on a conflict).

## Shared contract (all three)

- **Anchors come from UB-00b**: `docs/analysis/union-beta-e1-continuation/placement-seal.md`. Each spawn or region row uses the sealed anchor's x, y, z, heading and snapped y. If an anchor is missing or marked unreachable, return `blocked` and name it. **Never invent a position.**
- Spawn row shape (copy `db/resources/Worlds/Seed/spawnlist_dakara_e1.sql`):
  `INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary, respawn_secs) VALUES (<id>, <x>, <y>, <z>, <heading>, 23, <template>, '<tag>', NULL, true, 30);`
- Region rows (copy `point_sets_dakara_e1.sql` and `point_set_points_dakara_e1.sql`): `type 'AreaSet'`, `world_id 23`, `shape 'Cylinder'`, `flags 1`, with radius and height from the seal. Each set has one centre point; the loader expands it into a client-hinted box (`crates/cell-catalog/src/cell/spawner/regions.rs`). The region name is the `enter_region` key the chain packets use. End each point-sets file with the Dakara-style `SELECT pg_catalog.setval('point_sets_set_id_seq', GREATEST((SELECT last_value FROM point_sets_set_id_seq), <highest id>), true);`.
- **Spawn footer (required).** End each spawnlist file with the spawn-sequence footer, using 23199 (the top of Beta's spawn block, not the last id the file uses), the same GREATEST form as `spawnlist_debug_area_lords.sql`:
  `SELECT pg_catalog.setval('spawnlist_spawn_id_seq', GREATEST((SELECT MAX(spawn_id) FROM spawnlist), (SELECT last_value FROM spawnlist_spawn_id_seq), 23199), true);`
  Beta's spawns sit at 23000-23199, far above the sequence. `seeded_sequences_allocate_past_every_seeded_row` (`crates/cell-catalog/src/cell/spawner/tests/live_db_seed_sequences.rs`, floor 23199) fails if every Beta footer is missing, but it cannot tell whose footer ran: each file re-sets the same sequence, so deleting one footer fails nothing there. Each packet's own file-text footer guard (`beta_hub_seed_files_carry_their_own_sequence_footers` for UB-02b) is what proves that file's footer. No packet in this file inserts `item_lists` or `item_list_items`; if one ever does, add the matching footer with its block top. UB-40 (`spawnlist_beta_e1_life.sql`) adds the same footer to its own file.
- `database.sql` order: point-set-points files next to line 334 (`point_set_points_dakara_e1.sql`), point-sets files next to line 336, spawnlist files next to line 384 (`spawnlist_dakara_e1.sql`). Put Beta lines directly after each Dakara line, in the order b, c, d.
- Interact tags must be unique across all worlds, because `interact_tag` triggers do not filter by world. Every tag starts `BetaE1_`.
- Provenance comment above each block: `CIMMERIA_RECONSTRUCTION: anchor <NAME> from placement-seal.md (<map pin> on nav <component>)`.
- Guard helpers: copy `mesh`, `vector`, `reaches` and `standable_and_connected` from `crates/cell-world/src/cell/spawner_tests/dakara_e1/travel.rs:57-92` into `crates/cell-world/src/cell/spawner_tests/beta_e1/nav.rs`. Only UB-02b creates that file. UB-02c and UB-02d use it, and if they run before UB-02b merges they stack on UB-02b's branch. Mesh: `beta_site_evo_1.nav`. `ARRIVAL = [635.761, -60.134, 388.448]`.

## Rules (binding, all three)

1. PowerShell only; start every command with `Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\<wt>;`.
2. Never run `cargo` directly; use `pwsh tools/build-lane/lane.ps1` and `pwsh tools/build-lane/live-db-test.ps1`. Run in the foreground and check test counts are above 0.
3. Edit only your allowlist. Never edit `spawnlist.sql`, `point_sets.sql` or the Dakara files.
4. New seed files match their neighbouring seeds in the working tree (CRLF here; never mixed; normalise after writing). Keep `database.sql`'s line endings.
5. Seeds in `db/resources/` only; no migrations.
6. Under 500 lines per file.
7. Never touch `sgw`; no lab, no client.
8. No `git stash`, prune or push; one local commit.
9. Never spawn Jackson, Bra'tac, Thor, Heimdall, Woolsey, Pierce or a High Council actor (rejected additions).
10. No spawn in the underground complex (y below -200) or on nav components 1047 or 1060 (F8).

## Lane checks (each packet)

```powershell
Set-Location <wt>; pwsh tools/build-lane/lane.ps1 cargo fmt -p cimmeria-cell-world
Set-Location <wt>; pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-cell-world --all-targets -- -D warnings
Set-Location <wt>; pwsh tools/build-lane/live-db-test.ps1 spawner_tests::beta_e1
Set-Location <wt>; pwsh tools/build-lane/live-db-test.ps1 live_db_loaders
Set-Location <wt>; pwsh tools/build-lane/live-db-test.ps1 seeded_sequences_allocate_past_every_seeded_row
```

`live_db_loaders` proves the region loader still accepts every AreaSet, and that no respawner is at the origin. `seeded_sequences_allocate_past_every_seeded_row` proves the sequence ends past every seeded spawn and past Beta's block (floor 23199); it does not prove this file's footer, since the other Beta and Debug Area footers set the same sequence. Check that each reports a test count above 0.

## Docs rows

None. Per-world seed coordinates have no doc-update-map row. UB-99 writes the Beta content doc.

---

## UB-02b: hub, command and science spawns (Human cast and objects)

| Field | Value |
|---|---|
| Role | `packet-coder` |
| Size | about 35k |
| Branch / worktree | `uzp-ub02b` / `uzp-ub02b` (based on `uzp-e1` after UB-02a merges into it) |
| Commit subject | `feat(beta-e1): UB-02b hub cast, objects and regions on Beta Site` |
| Depends on | UB-02a merged (templates), UB-00b (anchors) |

### Allowlist

| File | Change |
|---|---|
| `db/resources/Worlds/Seed/spawnlist_beta_e1_hub.sql` | new |
| `db/resources/Events/Seed/point_sets_beta_e1_hub.sql` | new |
| `db/resources/Events/Seed/point_set_points_beta_e1_hub.sql` | new |
| `db/database.sql` | three `\ir` lines |
| `crates/cell-world/src/cell/spawner_tests/beta_e1/nav.rs` | new (shared helpers) |
| `crates/cell-world/src/cell/spawner_tests/beta_e1/hub.rs` | new |
| `crates/cell-world/src/cell/spawner_tests/beta_e1/mod.rs` | `mod hub;`, `mod nav;` |

### Rows

| Spawn | Template | Tag | Anchor (UB-00b) |
|---|---|---|---|
| 23000 | 2300 | `BetaE1_Carter` | Carter (Command Post area) |
| 23001 | 2301 | `BetaE1_Felger` | Felger |
| 23002 | 2302 | `BetaE1_Satterfield` | Satterfield (Quartermaster fire side, F18) |
| 23003 | 2304 | `BetaE1_Debeaux` | Debeaux (HELD-B8 answer) |
| 23010-23014 | 2320 | `BetaE1_XenoCage_1` .. `_5` | XenoCage 1-5 (between the gate and the Admin Hub, per 381 S1409's text) |
| 23015 | 2325 | `BetaE1_Crate_Arc42` | Crate_Arc42 (inside or at the Storage region) |
| 23016 | 2325 | `BetaE1_Crate_Arc66` | Crate_Arc66 |
| 23017 | 2322 | `BetaE1_HubComputer` | HubComputer (command centre) |
| 23018 | 2323 | `BetaE1_AdminWorkstation` | AdminWorkstation (inside the AdminBuilding region) |
| 23019 | 2324 | `BetaE1_ScienceComputer` | ScienceComputer (Science Facility; seeded at the exterior point (652.900, 19.77, 814.100), not the prefab's interior (649.700), because nav 890 is behind a 0.90 m door gap no walk crosses; placement-seal.md UB-00b-3) |
| 23020 | 2321 | `BetaE1_FourPiecesCase` | FourPiecesCase (Science Facility interior) |
| 23021 | 2335 | `BetaE1_RuinNote` | RuinNote (Barak'Ka ruin, INFERRED) |

| Set / point | Name | Anchor |
|---|---|---|
| 23000 / 23000 | `Beta_Site_Evo_1.Storage` | Storage region (UB-00b; Quartermaster candidate) |
| 23001 / 23001 | `Beta_Site_Evo_1.AdminBuilding` | AdminBuilding region |

### Tests (`hub.rs`; each `#[tokio::test]`, `require_db_or_skip!()`, with a `///` revert line)

- `live_db_beta_hub_spawns_match_the_placement_seal`: a `const HUB: [(i32, i32, &str, [f32; 3]); 16]` copied from the seal, plus the Dakara population assertions: count in 23000-23059 = 16; world 23; template; tag; `is_stationary` true; `respawn_secs` `Some(30)`; exact position. Each spawn is `standable_and_walkable` from `ARRIVAL` (forward, one `find_path` leg per authored hop, `Ok` and within 2.0 m; the walk is not symmetric). RuinNote (23021) needs the three hops H1..H3 and the Science pair (23019, 23020) one hop S1, all from placement-seal.md UB-00b-3; the other 13 are one direct leg. A greedy restart-from-the-last-waypoint chain is not used. `walk_guard_rejects_unreachable_and_unhopped_anchors` holds the negative controls (no hops fails, the interior Science point is an island).
- `live_db_beta_hub_regions_are_named_and_reachable`: both sets have their name, world 23, flags 1, and their centre; each centre is `standable_and_walkable` from `ARRIVAL`.
- `live_db_beta_hub_tags_are_unique_across_worlds`: no tag in 23000-23059 appears on any other spawn row (`SELECT tag, count(*) ... GROUP BY tag HAVING count(*) > 1`). Revert: duplicate a tag.

### Revert proof

Comment out the spawnlist `\ir` line; `live_db_beta_hub_spawns_match_the_placement_seal` fails. Move Carter 50 m into a cliff; the standable check fails. Delete the `setval` footer from `spawnlist_beta_e1_hub.sql` or `point_sets_beta_e1_hub.sql`; `beta_hub_seed_files_carry_their_own_sequence_footers` fails (the DB sequence test does not: other footers re-set the sequence). Restore and rerun green. Quote all of it.

### Acceptance

- Three tests green; `live_db_loaders` and `seeded_sequences_allocate_past_every_seeded_row` still green.
- SigNoz (`cimmeria-cell`, UB-90 row UB-U2): on a Beta login, the spawner logs show 16 hub entities created for world 23 with no `spawn` warnings for spawns 23000-23021. Grep the spawner's create and refuse lines in `crates/cell-world/src/cell/spawner/` and quote them.

### Return

`UB-02b <done|blocked|failed> <sha|-> C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\uzp-ub02b\.claude\reports\union-beta-e1\UB-02b-coder.md`

---

## UB-02c: Asgard base, deck, infirmary and fire site

| Field | Value |
|---|---|
| Role | `packet-coder` |
| Size | about 35k |
| Branch / worktree | `uzp-ub02c` / `uzp-ub02c` (stacked on `uzp-ub02b`) |
| Commit subject | `feat(beta-e1): UB-02c Mimir, Asgard objects, infirmary and fire-site regions` |
| Depends on | UB-02a merged, UB-00b (anchors, HELD-B3, HELD-B7), UB-02b (for `nav.rs`; stack on its branch if it has not merged) |

### Allowlist

| File | Change |
|---|---|
| `db/resources/Worlds/Seed/spawnlist_beta_e1_asgard.sql` | new |
| `db/resources/Events/Seed/point_sets_beta_e1_asgard.sql` | new |
| `db/resources/Events/Seed/point_set_points_beta_e1_asgard.sql` | new |
| `db/database.sql` | three `\ir` lines |
| `crates/cell-world/src/cell/spawner_tests/beta_e1/asgard.rs` | new |
| `crates/cell-world/src/cell/spawner_tests/beta_e1/mod.rs` | `mod asgard;` |

### Rows

| Spawn | Template | Tag | Anchor |
|---|---|---|---|
| 23060 | 2305 | `BetaE1_Mimir` | Mimir on the deck (nav 355; reachable only by beam) |
| 23061-23065 | 2330 | `BetaE1_AsgardScanner_1` .. `_5` | AsgardScanner 1-5 (ground, nav 251) |
| 23066 | 2331 | `BetaE1_AnalyticalMachine` | AnalyticalMachine (deck) |
| 23067 | 2334 | `BetaE1_ConsciousnessSimulator` | ConsciousnessSimulator (deck) |
| 23068 | 2332 | `BetaE1_HologramEmitter` | HologramEmitter inside EM-Infirmary01 (HELD-B3) |
| 23069 | 2333 | `BetaE1_DisabledDrone` | DisabledDrone inside EM-Infirmary01 (HELD-B3) |
| 23070 | 2328 | `BetaE1_FireSuppressantManager` | FireSuppressantManager (Quartermaster cluster) |
| 23071 | 2329 | `BetaE1_FireControlHub` | FireControlHub (Quartermaster cluster) |

| Set / point | Name | Anchor |
|---|---|---|
| 23030 / 23100 | `Beta_Site_Evo_1.Infirmary` | INFIRMARY_CENTRE (961.300, 15.97, 284.900), radius 10, height 6 (UB-00b seal) |
| 23031 / 23101 | `Beta_Site_Evo_1.FireSite` | the Quartermaster fire cluster (1022.3, 14.5, 271.2), radius 12, height 6 (UB-00b seal: manager and hub 3.0 m inside, Satterfield outside by 4.7 m; 443 S1484, UB-34) |

### Tests (`asgard.rs`)

- `live_db_beta_asgard_spawns_match_the_placement_seal`: same shape as the hub test (count 12 in 23060-23109). Ground rows are `standable_and_connected` to `ARRIVAL`. The deck rows (23060, 23066, 23067) are standable, connected to each other, and **not** connected to `ARRIVAL`: the deck can be reached only by beam (F8). If they were reachable on foot, the beam would be pointless and the anchor wrong.
- `live_db_beta_beam_pads_stand_on_their_decks`: the seal's `BEAM_DECK_DEST` is standable and connected to Mimir's spawn; `BEAM_GROUND_DEST` is `standable_and_connected` to `ARRIVAL`. Both sit 1.5-2.5 m from spawns 151 and 149. UB-30's beam chains use these constants, so pin them here as `pub(super) const`.
- `live_db_beta_infirmary_region_covers_its_objects`: the region exists. Using the expanded four-corner box from `regions.rs` (`is_point_in_region`), both the emitter and the drone positions are inside it, and the centre reaches `ARRIVAL`.
- `live_db_beta_fire_site_holds_its_objects_but_not_satterfield`: `FireSite` exists. Both fire objects (23070, 23071) are inside it, and Satterfield (spawn 23002, from UB-02b) is **outside** it by at least 2 m. UB-34 advances 443 S1484 on entering this region after the Satterfield talk, so a Satterfield inside the region would leave the player unable to fire the event. If 23002 is not seeded yet, the test fails with a message naming UB-02b.

### Revert proof

Remove Mimir's row; the count fails. Move `BEAM_DECK_DEST` 20 m off the deck; the pad test fails. Delete the `setval` footer from `spawnlist_beta_e1_asgard.sql`; only a file-text footer guard for that file fails (the DB sequence test does not: other footers re-set the sequence). Restore and rerun green.

### Acceptance

- Four tests green; `seeded_sequences_allocate_past_every_seeded_row` still green.
- SigNoz (UB-90 rows UB-U6, UB-U7): Mimir appears on the deck after a beam, with no spawn refusal for 23060-23071.

### Return

`UB-02c <done|blocked|failed> <sha|-> C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\uzp-ub02c\.claude\reports\union-beta-e1\UB-02c-coder.md`

---

## UB-02d: Nine Peaks camp and the ring pair (D-UB4 = A)

| Field | Value |
|---|---|
| Role | `packet-coder` |
| Size | about 40k (parts A and B) |
| Branch / worktree | `uzp-ub02d` / `uzp-ub02d` (stacked on `uzp-ub02b`) |
| Commit subject | `feat(beta-e1): UB-02d Ras'nor, sick Jaffa, Jaffa box and the Nine Peaks region` |
| Depends on | UB-02a merged, UB-00b (west-valley verdict), UB-02b (`nav.rs`). D-UB4 is ruled A (ring pair; ARCH-B1), so part B is in scope |

### Part A: rows (always)

| Spawn | Template | Tag | Anchor |
|---|---|---|---|
| 23110 | 2306 | `BetaE1_RasNor` | Ras'nor (west Jaffa camp, nav 70) |
| 23111-23113 | 2307 | `BetaE1_SickJaffa_1` .. `_3` | SickJaffa 1-3 |
| 23114 | 2326 | `BetaE1_JaffaBox` | JaffaBox |

| Set / point | Name | Anchor |
|---|---|---|
| 23060 / 23180 | `Beta_Site_Evo_1.NinePeaks` | the NinePeaks region |

Allowlist: `spawnlist_beta_e1_ninepeaks.sql`, `point_sets_beta_e1_ninepeaks.sql`, `point_set_points_beta_e1_ninepeaks.sql` (all new), `db/database.sql`, `spawner_tests/beta_e1/nine_peaks.rs` (new), and `mod nine_peaks;` in `spawner_tests/beta_e1/mod.rs`.

Test `live_db_beta_nine_peaks_spawns_match_the_placement_seal`: count 5 in 23110-23159. Each row is standable and connected to Ras'nor's spawn. Reachability: assert each row does **not** reach `ARRIVAL` on foot (UB-00b Q1: no walk route between nav 251 and 70; D-UB4 = A). Part B adds the ring pair that links them.

### Part B: ring pair (D-UB4 = A, ruled)

- Two `ring_transport_regions` rows, 2300 at ring 148 `BetaSite_Ring_00040001` (146.716, 8.591, 403.273; nav 251) and 2301 at the SW ring (-791.6, -51.8, -92.6; nav 70, snapped y -51.43, from the seal). The map ships a real ring prefab there: `GLB-RingTransporter00` InterpActor with a five-ring matinee at (-791.57, -51.80, -92.59) in chunk `Beta_Site_Evo_1-fffffff8` (extract_map census, ARCH-B1). Ring 148's prefab is at (149.20, 8.32, 403.04) in chunk `-00040001`. Copy the column list and the Omega pattern from `db/resources/Worlds/Seed/ring_transport_regions.sql` (region 10: `height 1.7`, `radius 2.53`, `display_name_id 7508`, `required_mission_id NULL`). `destination_region_ids` are `'{2301}'` and `'{2300}'`. Each row's `point_set_id` is a travel-range AreaSet (23080 and 23081, points 23260 and 23261). `event_set_id`: NULL (UB-00b found no Beta ring event set), with a comment saying the ceremony has no animation (castle_cellblock_chains.sql:2594 note). The ring matinee the client ships may still play; UB-U8 records what the player sees.
- A new ring spawn 23115, template 3, tag `BetaE1_Ring_SW`, at the SW ring.
- Files: `db/resources/Worlds/Seed/ring_transport_regions_beta_e1.sql` (new; its `\ir` line goes directly after the existing `ring_transport_regions.sql` line in `database.sql`), plus the travel-range rows appended to the Nine Peaks point-set files.
- Test `live_db_beta_ring_pair_links_both_valleys`: both rows exist and name each other. Each centre is standable: 2300 reaches `ARRIVAL`, and 2301 reaches Ras'nor. Revert: drop 2301; the test fails.
- Label: RESTORED_SOURCE for the two ring positions (map prefabs), CIMMERIA_RECONSTRUCTION for the pairing (the routing is lost). Beta has no `ring_transport_regions` row today, so this is the first. Lab UAT row UB-U8 proves the ring UI lists the destination.

### Revert proof

Remove the Ras'nor row; the count fails. Remove 2301; the pair test fails. Delete the `setval` footer from `spawnlist_beta_e1_ninepeaks.sql`; only a file-text footer guard for that file fails (the DB sequence test does not: other footers re-set the sequence). Restore and rerun green.

### Acceptance

- Part A and part B tests green; `seeded_sequences_allocate_past_every_seeded_row` still green.
- SigNoz (UB-90 row UB-U8): a ring use logs the transporter event for region 2300 to 2301 (grep `executor/transport.rs` and the ring handler for the exact text).

### Return

`UB-02d <done|blocked|failed> <sha|-> C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\uzp-ub02d\.claude\reports\union-beta-e1\UB-02d-coder.md`
