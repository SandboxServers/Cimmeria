# Dakara_E1 wave 2: props, the 1570 entry and its scaffolding

> Type: how-to (work packets). Audience: the agent named in each packet, and the wave-2 packet-reviewer (read-only; a packet-coder applies its findings).
> Written 2026-10-18 against `main` @ `d1bba0400`. Ledger and status: [README](../README.md#packets). Rules and full contract: [work-packets.md](../work-packets.md).

Wave 2 starts when DK-06a, DK-08 and DK-12a are merged. DK-09 is unblocked by that alone. DK-09b has no gate and may be dispatched at once. DK-10a and DK-10e also need the owner's answer to D-DK1 (D-DK17 is decided). DK-10fs, the sixth packet, is in its own file ([wave-2-dk-10fs.md](wave-2-dk-10fs.md)) and needs D-DK1 answered F and D-DK20. DK-06b needs the architect's D-DK10 ruling first. The six packets touch disjoint files, except that DK-09b and DK-10a both edit `interact_tag_linter.rs` in non-overlapping hunks (DK-09b moves lines 353-712 out, DK-10a adds lines after 176; whichever merges second rebases cleanly). Dispatch every one whose gate is open at once, each in its own worktree. Every packet carries its own rules; you do not need CLAUDE.md.

**Architect amendments (2026-10-18, after the DK-08 review).** D-DK18: step 4902's objectives are three client-hinted regions centred on the SG-18 remains, seeded by DK-09 below (point sets 2128-2130, points 2557-2559). D-DK19: DK-10c's range is 8060-8074 and DK-10d's moves to 8160-8179; DK-10a below carries both. DK-08 rules no template change, so DK-09's template rows stand.

**Architect amendment (2026-10-18, D-DK10 ruled).** The wave-2 packets other than DK-06b merged as #1867 (DK-09b), #1869 (DK-09), #1871 (DK-10a), #1875 (DK-10fs) and #1876 (DK-10e). D-DK10 is ruled server-only: a new verb `display_radio_dialog` sends `onDialogDisplay` with `EntityId` 0. DK-06b's full packet text moved to [wave-3b-dk-06b.md](wave-3b-dk-06b.md); it is dispatchable at once. It is reviewed with DK-10d-B as wave 3b.

**Architect amendments (2026-10-18, D-DK1 option F).** The owner can ship new missions to the client, so the recommended answer to D-DK1 is F: an authored level-1 mission 63100 "Home to Dakara" (DK-10fs here, chains 8075-8077 in wave 3's DK-10fc). DK-10a's chain 8040 then fires on `mission_completed 63100` and also completes 1570's optional objective 5936; it scaffolds the bridge seed file and test submodule. DK-10e gains Teal'c's letter (chain 8007), accepted by the owner as written. If the owner answers B instead, DK-10a uses its "fallback B" variant and DK-10fs is not dispatched.

## Rules for every wave-2 packet

1. PowerShell only. No bash, WSL or Git Bash. Use absolute paths inside your worktree; never `cd` to the root checkout. Prefix each lane call with `Set-Location <worktree>;` and check that the lane log lands under `logs/<worktree>/`.
2. Every compiling `cargo` call goes through the lane, run from the worktree root: `pwsh tools/build-lane/lane.ps1 cargo ...`. Never call `cargo` directly; never add `--target-dir`. Live-DB tests run through `pwsh tools/build-lane/live-db-test.ps1 <filter>`, which reloads your own `sgw_<worktree>` database. Run lane checks in the foreground.
3. Never `git stash`, `git worktree prune`, `git push`, `gh pr`, or delete a worktree. Commit once, locally, with the subject below and the attribution lines from your brief.
4. Edit only the files your packet names. Files stay under 700 lines (soft cap 500). Match the surrounding code: comment density, `foo/mod.rs` module style, doc comments on items.
5. Markdown under `docs/` is CRLF; the seed files named here are CRLF too. Keep the line endings of every file you edit: check with `[IO.File]::ReadAllText(<abs>).Contains("`r`n")`, and convert a new file with `[IO.File]::WriteAllText(<abs>, ([IO.File]::ReadAllText(<abs>) -replace "(?<!\r)\n", "`r`n"))`.
6. Seeds change in `db/resources/` only. Never add a `db/scripts/` migration. Every seed row carries an evidence-label comment (`ORIGINAL_DATA`, `PROJECT_FINAL_RECONSTRUCTION`, `RECONSTRUCTION`, `NEW CONTENT`); every spawn sets `respawn_secs`.
7. No new dialog row, no client patch, no cooked-data override (the one exception is DK-10fs's single `MISSION_ADDITIONS` entry, D-DK1 F). Never edit templates 54 or 59 (shared with Harset).
8. Never launch the game client, the launcher or any lab tool.
9. No IPs, credentials, tokens or account names in any file.
10. Report to `<worktree>\.claude\reports\dakara\<packet>-packet-coder.md`: files changed, each check's final `status=` line, the commit hash, every deviation and why. Return exactly one line: `<packet> <done|blocked|failed> <commit-sha|-> <report-path>`. A context stop past about 90k tokens: commit only passing work (subject suffix `(part N)`), write `docs/analysis/dakara-e1-rebuild/worknotes/<packet>.md` (done, left, next step, last `status=` line) and return `blocked` with the reason `context`.

## DK-06b: moved

DK-06b's full packet text is in [wave-3b-dk-06b.md](wave-3b-dk-06b.md) (D-DK10 ruled 2026-10-18; it runs as wave 3b with DK-10d-B).

## DK-09: SG-18 remains and drop-location props

| Field | Value |
|---|---|
| Role | `packet-coder` |
| Branch / worktree | `dakara/dk-09-props-spawns` / `dk-09` |
| Commit subject | `feat(seed): Dakara_E1 SG-18 remains and drop-location spawns (DK-09)` |
| Report | `<worktree>\.claude\reports\dakara\DK-09-packet-coder.md` |
| Size | About 38k tokens |

**Contract excerpt** (C2, C6, D-DK11, D-DK16, D-DK18). Templates 440-479 are the campaign's; 440-457 are in use and DK-09 adds 458-461, four clones of 449 (Drop Location), so each drop site holds its own per-player bind. Hostiles start at 462. Spawns 8000-8007 are in use; DK-09 adds 8008-8015, all world 61, `is_stationary` true, `respawn_secs` 30, `set_name` NULL. The SG-18 remains are on the navmesh but not connected to the plaza (D-DK11): they get an on-mesh guard only. The drop locations are in the plaza's component (D-DK13): they get the on-mesh and connected guard. Point sets 2123-2127 and points 2552-2556 are in use; DK-09 adds sets 2128-2130 and points 2557-2559, the three SG-18 search regions of step 4902 (D-DK18): client-hinted (`flags` 1) `AreaSet` cylinders in world 61, radius 8, height 10, each centred on its remains' x and z and 3 m below its y (a region spans its centre's y up to y + height, so the player standing on rough ground near the remains is inside). The region name is the `enter_region` key DK-10c's chains use, byte-exact.

| Set | Point | Name | Centre (x, y, z) | Objective | Remains |
|---|---|---|---|---|---|
| 2128 | 2557 | `Dakara_E1.SG18Louis` | 483.90, -12.10, 460.60 | 5632 (required) | 8008 |
| 2129 | 2558 | `Dakara_E1.SG18Nguyn` | 503.90, -12.10, 462.00 | 5630 (required) | 8009 |
| 2130 | 2559 | `Dakara_E1.SG18Waters` | 440.10, -12.10, 497.80 | 5631 (optional) | 8010 |

| Spawn | Template | Tag | Position (x, y, z) and heading | From |
|---|---|---|---|---|
| 8008 | 450 | `Dakara_E1_SG18_Louis` | 483.90, -9.10, 460.60, 3.1416 | PL-DK-A-07 |
| 8009 | 451 | `Dakara_E1_SG18_Nguyn` | 503.90, -9.10, 462.00, 0.0000 | PL-DK-A-08 |
| 8010 | 452 | `Dakara_E1_SG18_Waters` | 440.10, -9.10, 497.80, 1.5708 | PL-DK-A-09 |
| 8011 | 449 | `Dakara_E1_Drop1` | X, Y, Z of PL-DK-A-17, heading 0.0000 | DK-12a |
| 8012 | 458 | `Dakara_E1_Drop2` | PL-DK-A-18, heading 0.0000 | DK-12a |
| 8013 | 459 | `Dakara_E1_Drop3` | PL-DK-A-19, heading 0.0000 | DK-12a |
| 8014 | 460 | `Dakara_E1_Drop4` | PL-DK-A-20, heading 0.0000 | DK-12a |
| 8015 | 461 | `Dakara_E1_Drop5` | PL-DK-A-21, heading 0.0000 | DK-12a |

Read the A-17 to A-21 coordinates from the `## Drop locations (A-17 to A-21)` section of `docs/analysis/dakara-e1-rebuild/placements/A-world61-story-placements.md` (two decimals, as written there). If that section is missing, stop and return `blocked` (DK-12a not merged).

**Steps.**

1. `db/resources/Entities/Seed/entity_templates_dakara_e1.sql`:
   - Line 8: change `-- Id block 440-459 (cast and props). 460-479 is reserved for DK-20's hostiles` to `-- Id block 440-461 (cast and props; DK-09 added 458-461). 462-479 is reserved for DK-20's hostiles`. Keep lines 9-10.
   - After the last row (457 Vocuum, the end of the file), add one blank line, this comment, and four rows:

     ```sql
     -- 458-461 Drop Location #2 to #5 (DK-09, D-DK16): clones of 449, so each of
     --   mission 1646's five drop sites holds its own per-player bind (dsm 6739 on
     --   449, 6740-6743 on 458-461) and its own glow. name_id 26727
     --   `DN_ob_DakaraE1_int_DropLoc`: ORIGINAL_DATA; the client has one string
     --   for all five. look: RECONSTRUCTION, as 449.
     ```

     Each row is a copy of the 449 `INSERT` (line 151), changing only `template_id` (458, 459, 460, 461) and `template_name` (`'Dakara E1 - Drop Location 2'` to `'Dakara E1 - Drop Location 5'`). Do not touch 449's row.
2. `db/resources/Worlds/Seed/spawnlist_dakara_e1.sql`: after the 8007 row, add one blank line, this comment, and the eight rows of the table above, one `INSERT` per row in the 8004-8007 form (`INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary, respawn_secs) VALUES (...);`):

   ```sql
   -- DK-09 PROJECT_FINAL_RECONSTRUCTION. The SG-18 remains (mission 1570 step
   -- 4901) at PL-DK-A-07 to A-09: LOW map estimates, on the navmesh but not on
   -- the plaza's component, reached over rough ground (D-DK11). The five drop
   -- locations (mission 1646 step 4908) at PL-DK-A-17 to A-21, on the plaza's
   -- component (D-DK13). No bind is applied here: every prop's dialog is a
   -- per-player bind from its mission chain.
   ```

3. `crates/cell-world/src/cell/spawner_tests/dakara_e1/travel.rs`: split the on-mesh assert out of `standable_and_connected` (line 77) into a new helper directly above it, and call it from there:

   ```rust
   /// On the shipped navmesh, with no claim about which component it is on.
   /// The SG-18 remains use this alone (D-DK11).
   pub(super) fn standable(mesh: &NavMesh, point: [f32; 3], label: &str) {
       assert!(
           mesh.is_point_valid(&vector(point)),
           "{label} is off the shipped navmesh"
       );
   }
   ```

   `standable_and_connected` keeps its signature; its body becomes `standable(mesh, point, label);` followed by the existing `reaches` assert.
4. `crates/cell-world/src/cell/spawner_tests/dakara_e1/mod.rs`: add `mod props;` between `mod population;` and `mod templates;`. Change `const CAST_AND_PROPS: (i32, i32) = (440, 459);` to `(440, 461)` and its doc comment to say "(DK-03, plus DK-09's drop-location clones 458-461). 462-479, the rest of the campaign's block, is DK-20's hostiles". Add a doc bullet after the `templates` one: `` - [`props`]: DK-09, the SG-18 remains and drop-location spawns and the SG-18 search regions: rows, positions, on-mesh and plaza-connectivity guards. ``
5. `crates/cell-world/src/cell/spawner_tests/dakara_e1/templates.rs`: line 1 `(440-459)` becomes `(440-461)`; `ROSTER` becomes `[(i32, &str, &str); 22]` with four entries after 457: `(458, "being", "DN_ob_DakaraE1_int_DropLoc")` and the same for 459, 460, 461; the doc at line 85 says "the twenty-two templates (DK-03's eighteen and DK-09's four drop-location clones)". Change nothing else (the look test's `479` bound stays).
6. New `crates/cell-world/src/cell/spawner_tests/dakara_e1/props.rs` (about 140 lines), in the style of `population.rs`:

   ```rust
   //! DK-09 production seed guards: the SG-18 remains (spawns 8008-8010) and
   //! mission 1646's five drop locations (spawns 8011-8015), and the
   //! step-4902 search regions around the remains (point sets 2128-2130).

   use super::*;

   /// The plaza point every connected Dakara_E1 spawn must reach on foot.
   const PLAZA: [f32; 3] = [100.0, -17.4, 230.0];

   /// `(spawn id, template id, tag, position, plaza-connected)`.
   const PROPS: [(i32, i32, &str, [f32; 3], bool); 8] = [
       (8008, 450, "Dakara_E1_SG18_Louis", [483.9, -9.1, 460.6], false),
       // ... 8009, 8010 (false), 8011-8015 (true) with the DK-12a coordinates
   ];
   ```

   Three tests:
   - `live_db_dakara_sg18_remains_and_drop_locations_match_the_placement_ledger`: `require_db_or_skip!()`; the count of spawns 8008-8015 is 8; for each row, `(world_id, template_id, tag, is_stationary, respawn_secs)` equals `(61, template, tag, true, Some(30))` and `[x, y, z]` equals the position (message `"spawn {id} moved: update the ledger"`); then `travel::standable_and_connected(&exterior, actual, PLAZA, tag)` when the flag is true, else `travel::standable(&exterior, actual, tag)`. `exterior` is `travel::mesh("dakara_e1.nav")`. Query shape: as `population.rs` lines 34-41.
   - `live_db_each_drop_location_has_its_own_template_wearing_449s_look`: select `(template_id, static_mesh, body_set, name_id, flags, interaction_type, class::text)` for 449 and 458-461; all five equal 449's values except the id; and each of 449, 458, 459, 460, 461 is the `template_id` of exactly one spawnlist row.
   - `live_db_each_sg18_search_region_is_centred_on_its_remains`: a `const REGIONS: [(i32, &str, i32); 3] = [(2128, "Dakara_E1.SG18Louis", 8008), (2129, "Dakara_E1.SG18Nguyn", 8009), (2130, "Dakara_E1.SG18Waters", 8010)];` (set id, name, remains spawn). The count of `point_sets` rows with `set_id BETWEEN 2128 AND 2130` is 3. For each: `SELECT s.name::text, s.type::text, s.world_id, s.radius, s.height, s.shape::text, s.flags, p.x, p.y, p.z FROM resources.point_sets s JOIN resources.point_set_points p USING (set_id) WHERE s.set_id = $1` (`fetch_one`, so exactly one point). Assert `(name, type, world_id, shape, flags)` equals `(name, "AreaSet", 61, "Cylinder", 1)`; `radius` and `height` are nullable `real` columns: read them as `Option<f32>` and assert `Some(8.0)` and `Some(10.0)`; then read the remains spawn's `x, y, z` from `spawnlist` and assert the region's x and z equal it exactly and `(region_y - (spawn_y - 3.0)).abs() < 0.01` (message `"region {set_id} no longer centred on spawn {spawn}: move both together"`). No navmesh check: the centre is 3 m under the ground by design.
7. `db/resources/Events/Seed/point_sets_dakara_e1.sql`: after the 2127 row (line 13), add one blank line, this comment and three rows in the 2123-2127 form, then change the `setval` line's `2127` to `2130`:

   ```sql
   -- DK-09, D-DK18 RECONSTRUCTION: mission 1570 step 4902 "Search the locations
   -- Loth'ta indicated" names no site; its objectives become one region around
   -- each SG-18 remains (spawns 8008-8010), so the M2 position pass moves a
   -- region with its remains. 2128 completes 5632, 2129 completes 5630,
   -- 2130 completes the optional 5631 (dakara_e1_m1570_search_chains.sql).
   ```

   `(2128, 'Dakara_E1.SG18Louis', 'AreaSet', 61, 8, 10, 'Cylinder', 1)`, `(2129, 'Dakara_E1.SG18Nguyn', ...)`, `(2130, 'Dakara_E1.SG18Waters', ...)`, each as its own one-line `INSERT INTO point_sets (set_id, name, type, world_id, radius, height, shape, flags) VALUES (...);`. Keep each `INSERT` on one physical line: the linter scans line by line.
8. `db/resources/Events/Seed/point_set_points_dakara_e1.sql`: after the 2127 row (line 12), add the comment `-- DK-09 (D-DK18): SG-18 search regions, centred on spawns 8008-8010, 3 m below each.` and three rows in the existing form: `(2128, 2557, 483.90, -12.10, 460.60, 0, 0, 0)`, `(2129, 2558, 503.90, -12.10, 462.00, 0, 0, 0)`, `(2130, 2559, 440.10, -12.10, 497.80, 0, 0, 0)`. Change the `setval` line's `2556` to `2559`.

**Checks, in order** (from the worktree root):

1. `pwsh tools/build-lane/lane.ps1 cargo fmt -p cimmeria-cell-world`
2. `pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-cell-world --all-targets -- -D warnings`
3. `pwsh tools/build-lane/live-db-test.ps1 spawner_tests::dakara_e1` (every Dakara cell-world guard, old and new, passes)

**Test type and revert proof.** Live-DB seed guards (TESTING.md). The first two new tests fail when the DK-09 spawn rows are removed (count 8, then the row query); the template guard and the roster test fail when 458-461 are removed; the region test fails when sets 2128-2130 or their points are removed (count 3, then `fetch_one`), or when a region and its remains drift apart. The four old DK-03 tests keep passing because they read the widened block.

**Docs rows owed.** None in `docs/` outside the campaign. In `docs/analysis/dakara-e1-rebuild/placements/README.md`, after the paragraph that starts "After the owner's playtest" (line 86), add: `Seeded so far: B-02/B-03 and A-01/A-03 as spawns 8000-8003 (DK-04), B-04, B-05, A-06 and DK-05-A-01 as 8004-8007 (DK-05), A-07 to A-09 as 8008-8010 with search regions 2128-2130 around them, and A-17 to A-21 as 8011-8015 (DK-09).` Check the DK-04 and DK-05 parts against the spawnlist comments before writing them; fix them if they differ and say so in the report. Name the row in the report.

**Done when** the three checks pass and the report pastes each final `status=` line.

## DK-09b: the region-key lint reads every point-set seed file

| Field | Value |
|---|---|
| Role | `packet-coder` |
| Gate | none (may be dispatched before the rest of wave 2) |
| Branch / worktree | `dakara/dk-09b-linter-point-sets` / `dk-09b` |
| Commit subject | `test(content-engine): region-key lint reads every point_sets seed file (DK-09b)` |
| Report | `<worktree>\.claude\reports\dakara\DK-09b-packet-coder.md` |
| Size | About 30k tokens |

**Why** (F20, D-DK18). `every_chain_region_key_matches_a_seeded_point_set` in `crates/content-engine/tests/it/interact_tag_linter.rs` checks that every `enter_region` / `exit_region` trigger key in `db/resources/Content/Seed/*_chains.sql` is a byte-exact `point_sets.name`. It reads only `db/resources/Events/Seed/point_sets.sql`, so the Dakara regions in `point_sets_dakara_e1.sql` (2123-2127 today, 2128-2130 after DK-09) are invisible to it, and DK-10c's three `enter_region` chains would fail it. The file is also 753 lines, over the 700-line hard cap, so the region check moves to its own file as part of the fix. The check's logic and its four existing tests do not change.

**Steps.**

1. New file `crates/content-engine/tests/it/interact_tag_linter/regions.rs` (the parent `interact_tag_linter.rs` stays a file; Rust finds the child module in the folder of the same name). Move into it, text unchanged except as stated below, these items of `interact_tag_linter.rs`; find each by name (line numbers as of `d1bba0400`):
   - `fn scan_region_triggers` with its doc (353-396);
   - `fn scan_point_set_names` with its doc (398-422);
   - `fn every_chain_region_key_matches_a_seeded_point_set` with its doc and `#[test]` (424-508);
   - `fn region_key_violations` with its doc (510-537);
   - the tests, each with its doc and `#[test]`: `scan_region_triggers_extracts_chain_and_region` (539-553), `scan_point_set_names_extracts_the_name_column_byte_exact` (572-588), `region_key_check_allows_legitimate_per_region_casing_but_catches_a_mismatch` (590-637) and `dotless_point_set_names_are_not_skipped` (639-699).

   `scan_chains_picks_up_basic_pattern` (555-570) and everything else stay in the parent. The new file starts with:

   ```rust
   //! Region-key lint: every `enter_region` / `exit_region` trigger key in a
   //! chain seed file byte-matches a seeded `point_sets.name`. Moved out of
   //! `interact_tag_linter.rs` by DK-09b, which also made the point-set scan
   //! read every `point_sets*.sql` seed file instead of `point_sets.sql` alone.

   use std::collections::HashSet;
   use std::fs;

   use super::workspace_root;
   ```

   An intra-doc link in a moved doc comment that names an item left in the parent (for example `every_interact_tag_chain_has_set_interaction_type`, `every_chain_seed_file_is_registered_in_database_sql`) gets a `super::` prefix.
2. `interact_tag_linter.rs`: delete the moved items. Directly after the `use std::path::PathBuf;` line add one blank line and `mod regions;` with the comment `// The region-key lint (DK-09b split it out to keep this file under the cap).` Remove any `use` that clippy then reports unused; add nothing else. A child module may use the parent's private `workspace_root`; do not make anything `pub`.
3. `regions.rs`: add this function after `scan_point_set_names`:

   ```rust
   /// The `name` of every `point_sets` row in every `point_sets*.sql` file of
   /// `db/resources/Events/Seed/` (`point_sets.sql` plus the per-zone files,
   /// such as `point_sets_dakara_e1.sql`), so a zone's own regions are
   /// linted the moment its file lands.
   fn seeded_point_set_names() -> HashSet<String> {
       let dir = workspace_root().join("db/resources/Events/Seed");
       let mut names = HashSet::new();
       for entry in fs::read_dir(&dir).unwrap_or_else(|e| panic!("read {}: {e}", dir.display())) {
           let path = entry.expect("dir entry").path();
           let file = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
           if !(file.starts_with("point_sets") && file.ends_with(".sql")) {
               continue;
           }
           let sql = fs::read_to_string(&path)
               .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
           names.extend(scan_point_set_names(&sql));
       }
       names
   }
   ```

   In `every_chain_region_key_matches_a_seeded_point_set`, replace the three statements that build `point_sets_path`, read it and call `scan_point_set_names` with `let canonical_names = seeded_point_set_names();`. Keep the non-empty assert; its message becomes `"point_sets*.sql scan found zero names — parser drift, not an empty seed dir"`. In its doc comment, after the **Scan coverage** paragraph, add: `**Point-set coverage.** The names come from every point_sets*.sql seed file (DK-09b). The check used to read point_sets.sql alone, so a region seeded in a per-zone file, such as Dakara_E1's 2123-2130, failed it.` (as Markdown in the `///` lines, with the file names in backticks). Update `scan_point_set_names`'s doc first line to say it scans one point-set seed file.
4. `regions.rs`, new test at the end of the file:

   ```rust
   /// DK-09b revert proof: the scan reads `point_sets.sql` AND the per-zone
   /// point-set files. Going back to `point_sets.sql` alone loses the Dakara
   /// name and fails this test.
   #[test]
   fn point_set_scan_reads_every_point_sets_seed_file() {
       let names = seeded_point_set_names();
       for name in ["Castle_CellBlock.Region8", "Dakara_E1.CommandTent"] {
           assert!(
               names.contains(name),
               "{name} is missing: the scan must read point_sets.sql and every point_sets_*.sql"
           );
       }
   }
   ```

**Checks, in order** (from the worktree root):

1. `pwsh tools/build-lane/lane.ps1 cargo fmt -p cimmeria-content-engine`
2. `pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-content-engine --all-targets -- -D warnings`
3. `pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-content-engine interact_tag_linter`: every test passes, and the count is the count before your change plus one (run it once before step 1 and paste both counts in the report).
4. Line counts: `(Get-Content <abs>).Count` for both files, each under 500. Paste them.

**Test type and revert proof.** A file-scanning lint test (TESTING.md, static seed lint). `point_set_scan_reads_every_point_sets_seed_file` fails when the scan is reverted to `point_sets.sql` alone. The four moved tests keep guarding the check's logic unchanged.

**Docs rows owed.** None outside the campaign: the linter is a test. Say so in the report.

**Done when** the checks pass and the report pastes each final `status=` line, both test counts and both line counts.

## DK-10a: the 1570 entry, seed files and test scaffolding

| Field | Value |
|---|---|
| Role | `packet-coder` |
| Gate | D-DK1 answered **F** (recommended) or **B**. Under F use the main text; under B use every "Fallback B" note instead and skip the bridge rows. Under **C** the architect drops chain 8040 and its tests from this packet first; the scaffolding (steps 1, 2, 4, 5) stays. Under **A** or **D** the packet is not dispatched. The brief names the answer. |
| Branch / worktree | `dakara/dk-10a-1570-entry` / `dk-10a` |
| Commit subject | `feat(content): Dakara_E1 mission 1570 entry chain and seed scaffolding (DK-10a)` |
| Report | `<worktree>\.claude\reports\dakara\DK-10a-packet-coder.md` |
| Size | About 40k tokens |

**Contract excerpt** (C3, C4, C5, C7 of [work-packets.md](../work-packets.md); D-DK1 F, D-DK6, D-DK14, D-DK15).

- **Under F:** when a Free Jaffa (`archetype eq 7`) on world 61 completes mission 63100 "Home to Dakara" (by talking to Rak'nor, DK-10fc's chain 8077), and 1570 is `not_active`, chain 8040 accepts 1570, moves it past its two SGC steps (4641 "Speak to Sgt. Harriman", 4642 "Enter the Stargate") to 4906 "Speak to Bra'tac at his command tent", and completes 4906's optional objective 5936 ("(Optional) Speak with Rak'nor ..."), since that conversation just happened. The trigger is `mission_completed`, key `'63100'`; chain-completed missions fire it inline, with world, archetype and mission context filled.
- `accept_mission`, `advance_step` and `complete_objective` all run inline in the executor (`crates/cell-content/src/cell/content/executor/dispatch.rs`), so the four actions run in order; `advance_step` force-completes the objectives of the step it leaves. 5936 is optional, so completing it never completes the step or the mission (C5 rule 3).
- Conditions are evaluated before any action runs, and every matching chain fires. Step 4906's Bra'tac binds live in world 62 and come from DK-10b's own restore chains (C5 rule 6); no co-firing chain is needed under F.
- A missing `mission_<id>_status` param reads as `not_active`.
- **Fallback B:** the trigger is `player_loaded`, key `'Dakara_E1'`; the conditions add 1645 `not_active`; there is no `complete_objective`. Chains 8001 and 8002 also fire on that event, which is intended, and DK-10b's 8057 co-fires to bind Rak'nor.

**Steps.**

1. Create the eight seed files of C3 in `db/resources/Content/Seed/` (CRLF; seven under fallback B, without the bridge file). Each holds only a header and the search path; DK-10a fills only the entry file. Header shape, for example:

   ```sql
   -- Dakara_E1 rebuild: mission 1570, steps 4906 and 4904 (Rak'nor, Bra'tac, Loth'ta).
   -- Campaign: docs/analysis/dakara-e1-rebuild/ (packet DK-10b).
   -- Chain ID range: 8045-8059 (work-packets.md, C3). Evidence labels: README.md, "Evidence Labels".

   SET search_path = resources, pg_catalog;
   ```

   | File | Packet | Range | Scope line |
   |---|---|---|---|
   | `dakara_e1_bridge_chains.sql` | DK-10fc | 8075-8079 | mission 63100 "Home to Dakara", the Free Jaffa bridge (`NEW CONTENT`, D-DK1 F) |
   | `dakara_e1_m1570_entry_chains.sql` | DK-10a | 8040-8044 | mission 1570, the Free Jaffa entry |
   | `dakara_e1_m1570_tent_chains.sql` | DK-10b | 8045-8059 | mission 1570, steps 4906 and 4904 |
   | `dakara_e1_m1570_search_chains.sql` | DK-10c | 8060-8074 | mission 1570, steps 4902 and 4901 |
   | `dakara_e1_m1570_report_chains.sql` | DK-10d | 8160-8179 | mission 1570, steps 4905 and 4903 |
   | `dakara_e1_m1645_chains.sql` | DK-11 | 8080-8119 | mission 1645 |
   | `dakara_e1_m1646_offer_chains.sql` | DK-12b | 8120-8139 | mission 1646, offer, accept and turn-in |
   | `dakara_e1_m1646_drop_chains.sql` | DK-12c | 8140-8159 | mission 1646, step 4908 (five drops) |

2. `db/database.sql`: directly after the line `\ir resources/Content/Seed/dakara_e1_space_chains.sql`, add the eight `\ir resources/Content/Seed/<file>` lines in the table's order (the bridge file first). Keep the file's line endings. The bridge file's chains use mission 63100, whose rows come from `missions_dakara_e1.sql` (DK-10fs), loaded earlier in `database.sql`; an empty file needs nothing.
3. In `dakara_e1_m1570_entry_chains.sql`, after the search path, add chain 8040 exactly:

   ```sql
   -- Chain 8040 PROJECT_FINAL_RECONSTRUCTION (D-DK1 option F, D-DK6): when a
   -- Free Jaffa finishes the bridge mission 63100 "Home to Dakara" (talking to
   -- Rak'nor, chain 8077), they accept 1570 and are moved past its two SGC
   -- steps, 4641 (Harriman) and 4642 (enter the gate), to 4906 (speak to
   -- Bra'tac). The character never played 4641 or 4642; the mission log shows
   -- them completed, which Teal'c's letter (chain 8007) explains. Rak'nor's
   -- optional 5936 is completed here: his 6110 has just played. Step 4906's
   -- Bra'tac binds are DK-10b's world-62 restore chains (C5 rule 6).
   INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
   VALUES (8040, '1570 - Free Jaffa finished 63100 on Dakara_E1: accept, skip SGC steps 4641 and 4642, start at 4906, Rak''nor done', 'mission', 1570, true, 0);

   INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
   VALUES (8040, 'mission_completed', '63100', 'player', false, 0);

   INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
   VALUES
     (8040, 'world', 61, NULL, 'eq', NULL, 0),
     (8040, 'archetype', NULL, NULL, 'eq', '7', 1),
     (8040, 'mission_status', 1570, NULL, 'eq', 'not_active', 2);

   INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
   VALUES
     (8040, 'accept_mission', 1570, NULL, '{}', 0, 0),
     (8040, 'advance_step', 1570, '4642', '{}', 0, 1),
     (8040, 'advance_step', 1570, '4906', '{}', 0, 2),
     (8040, 'complete_objective', 1570, '5936', '{}', 0, 3);
   ```

   **Fallback B:** the comment names option B and says "a Free Jaffa arriving on Dakara_E1" and "Step 4906's binds come from DK-10b's co-firing chain on this trigger"; the description is `'1570 - Free Jaffa arrival on Dakara_E1: accept, skip SGC steps 4641 and 4642, start at 4906'`; the trigger is `('player_loaded', 'Dakara_E1')`; add `(8040, 'mission_status', 1645, NULL, 'eq', 'not_active', 3)`; drop the `complete_objective` row.

4. `db/resources/Content/Seed/dakara_e1_space_chains.sql`, header comment lines 4-7 only: replace the sentence that names `dakara_e1_arrival_chains.sql`, `dakara_e1_betrayal_chains.sql` and `dakara_e1_climax_chains.sql` with `The mission chains live in one file per packet, dakara_e1_m*_chains.sql (8040-8179, work-packets.md C3); later missions use 8180-8499.` Change no row.
5. `crates/content-engine/tests/it/interact_tag_linter.rs`, `fn allowlist`: directly after line 176 (`| ("dakara_e1_space_chains.sql", 8006) // FromMohkatan: template 448`), add:

   ```rust
           // dakara_e1_m*_chains.sql — the Dakara_E1 mission chains (DK-10a
           // to DK-12c). reason: every NPC and prop these chains click gets its
           // interaction bit from the PER-PLAYER dialog bind (`add_dialog_set`
           // pushes the dialog_set_map row's `interaction_flags` to the binding
           // player only), as in castle_701 (D-CA15). A `set_interaction_type`
           // would light the prop for every player in world 61 at once.
           | ("dakara_e1_m1570_entry_chains.sql", 8040..=8044)
           | ("dakara_e1_m1570_tent_chains.sql", 8045..=8059)
           | ("dakara_e1_m1570_search_chains.sql", 8060..=8074)
           | ("dakara_e1_bridge_chains.sql", 8075..=8079)
           | ("dakara_e1_m1645_chains.sql", 8080..=8119)
           | ("dakara_e1_m1646_offer_chains.sql", 8120..=8139)
           | ("dakara_e1_m1646_drop_chains.sql", 8140..=8159)
           | ("dakara_e1_m1570_report_chains.sql", 8160..=8179)
   ```

   The report range sits after the 1646 ranges on purpose (D-DK19 moved it from 8070-8079); 8075-8079 are the bridge file's (D-DK1 F; drop that line under fallback B). The first comment line names `dakara_e1_m*_chains.sql`; make it `dakara_e1_m*_chains.sql and dakara_e1_bridge_chains.sql`. DK-09b moves the region check out of this file in the same wave; it does not touch `fn allowlist`, so the two packets edit separate hunks. If DK-09b merged first, the line number above has moved: find the line by its text.

6. `crates/cell-content/src/cell/content/chain_replay_tests/mod.rs`: add `mod dakara_e1_missions;` between `mod class_start_sgc;` (line 38) and `mod dakara_e1_space;` (line 39). In the module doc (lines 29-31), after the `[`dakara_e1_space`]` sentence, add: `` [`dakara_e1_missions`] pins the Dakara_E1 mission chains (DK-10a onward), one submodule per seed file. ``
7. New folder `crates/cell-content/src/cell/content/chain_replay_tests/dakara_e1_missions/`:
   - `mod.rs` (about 140 lines): a `//!` doc naming the campaign, C7 and the one-file-per-packet rule; `mod bridge; mod entry; mod m1645; mod m1646_drops; mod m1646_offer; mod report; mod search; mod tent;` (no `mod bridge;` under fallback B); and these items, private to the module (children see them):

     ```rust
     use std::ops::RangeInclusive;

     use cimmeria_content_engine::actions::Action;
     use cimmeria_content_engine::chain::{ChainEngine, ResolvedActions};
     use cimmeria_content_engine::context::ExecutionContext;
     use cimmeria_content_engine::triggers::{TriggerEvent, TriggerType};

     /// ARCHETYPE_Sholva, the Free Jaffa (char_defs 8 and 18).
     const FREE_JAFFA: i64 = 7;
     /// ARCHETYPE_Jaffa, the Praxis Jaffa: gets the Jaffa lines (D-DK15), never the entry.
     const PRAXIS_JAFFA: i64 = 8;
     /// A Tau'ri archetype (Scientist), for the `lte 6` branch.
     const TAURI: i64 = 3;
     const DAKARA: i32 = 61;
     const STORY_RM: i32 = 62;
     const DAKARA_NAME: &str = "Dakara_E1";
     const STORY_RM_NAME: &str = "Dakara_E1_StoryRm";

     /// One range per seed file (work-packets.md C3).
     const BRIDGE_CHAINS: RangeInclusive<i64> = 8075..=8079;
     const ENTRY_CHAINS: RangeInclusive<i64> = 8040..=8044;
     const TENT_CHAINS: RangeInclusive<i64> = 8045..=8059;
     const SEARCH_CHAINS: RangeInclusive<i64> = 8060..=8074;
     const REPORT_CHAINS: RangeInclusive<i64> = 8160..=8179;
     const M1645_CHAINS: RangeInclusive<i64> = 8080..=8119;
     const M1646_OFFER_CHAINS: RangeInclusive<i64> = 8120..=8139;
     const M1646_DROP_CHAINS: RangeInclusive<i64> = 8140..=8159;

     /// Set `mission_<m>_obj_<o>_status`, the key `Condition::ObjectiveStatus` reads.
     fn with_objective(mut ctx: ExecutionContext, mission: i32, objective: i32, status: &str) -> ExecutionContext { ... }

     /// Set `counter_<name>`, the key `Condition::Counter` reads (a missing
     /// counter reads 0). Used by DK-10c and DK-12c.
     fn with_counter(mut ctx: ExecutionContext, name: &str, n: i64) -> ExecutionContext { ... }

     /// Fire `enter_region` with `region_key` = `key`, as the cell does for a
     /// client-hinted point set (`event_dispatch/region.rs`). Used by DK-10c.
     fn fire_enter_region(engine: &ChainEngine, ctx: &ExecutionContext, key: &str) -> ResolvedActions { ... }

     /// `"<chain>:<verb>(<args>)"` labels for the chains in `range`, as
     /// `mission_1324::labels` plus the item and bark verbs.
     fn labels_in(resolved: &ResolvedActions, range: RangeInclusive<i64>) -> Vec<String> { ... }
     ```

     `with_objective` mirrors `mission_1324::with_step` (`ctx.set_param(format!("mission_{mission}_obj_{objective}_status"), serde_json::json!(status))`). `with_counter` is the same shape: `ctx.set_param(format!("counter_{name}"), serde_json::json!(n)); ctx` (the condition reads it with `as_i64`, `crates/content-engine/src/conditions/mod.rs:359-361`). `fire_enter_region` copies `mission_1324::fire_interact_tag` (lines 153-166) with two changes: the derived context sets `"region_key"` instead of `"entity_tag"`, and `trigger_type` is `TriggerType::RegionEnter`. `derive` is private to `mission_1324`, so inline its body: `let mut c = ExecutionContext::new(); c.world_id = ctx.world_id; c.params = ctx.params.clone(); c.set_param("region_key".to_string(), serde_json::json!(key));`. `labels_in` copies the body of `mission_1324::labels` (lines 211-243) with `range.contains(id)` as the filter, and adds five arms before `other`: `Action::GrantItem { item_id, count, .. } => format!("add_item({item_id}x{count})")`, `Action::RemoveItem { item_id, count } => format!("remove_item({item_id}x{count})")`, `Action::NpcBark { screen_id, .. } => format!("npc_bark({screen_id})")`, `Action::IncrementCounter { counter_name, amount } => format!("increment_counter({counter_name}+{amount})")`, `Action::ResetCounter { counter_name } => format!("reset_counter({counter_name})")`. With the two helpers `mod.rs` is about 140 lines; the packet stays about 40k tokens. Items not used by `entry.rs` yet (`BRIDGE_CHAINS`, the later ranges, `with_objective`, `with_counter`, `fire_enter_region`, `STORY_RM` if unused) carry `#[allow(dead_code)] // used by the wave-3 submodules (work-packets.md C7)`, so clippy stays clean; wave-3 packets do not edit `mod.rs`.
   - `bridge.rs`, `tent.rs`, `search.rs`, `report.rs`, `m1645.rs`, `m1646_offer.rs`, `m1646_drops.rs`: each one `//!` line, for example `//! DK-10b: mission 1570 steps 4906 and 4904 (dakara_e1_m1570_tent_chains.sql, 8045-8059).` and `//! DK-10fc: the bridge mission 63100 "Home to Dakara" (dakara_e1_bridge_chains.sql, 8075-8079).`
   - `entry.rs`, imports: `use super::*;`, `use super::super::assert_no_deferred_actions;`, `use super::super::mission_1324::{ctx_for, fire_mission_completed, fire_player_loaded, with_mission};`, `use crate::cell::content::engine_loader::build_engine;`, `use crate::test_support::require_db_or_skip;` (if a path differs, use the one `mission_1324.rs` resolves to and say so in the report). `fire_mission_completed(engine, ctx, mission_id)` expects a context in which that mission already reads `completed`, as the runtime does. Let `done(a, w)` mean `with_mission(ctx_for(a, w), 63100, "completed")` and `fire(ctx)` mean `fire_mission_completed(&engine, &ctx, 63100)`. Tests, each `#[tokio::test] async fn`, `let pool = require_db_or_skip!(); let engine = build_engine(Some(&pool)).await;`:
     1. `live_db_finishing_home_to_dakara_starts_1570_at_bratac`: `fire(done(FREE_JAFFA, Some(DAKARA)))`; `labels_in(&r, ENTRY_CHAINS)` equals `["8040:accept_mission(1570)", "8040:advance_step(1570->4642)", "8040:advance_step(1570->4906)", "8040:complete_objective(1570/5936)"]`; `assert_no_deferred_actions(&r, 8040)`.
     2. `live_db_no_other_archetype_is_given_1570`: `done(PRAXIS_JAFFA, ..)` and `done(TAURI, ..)` on Dakara each give an empty `labels_in(&r, ENTRY_CHAINS)`.
     3. `live_db_1570_is_given_once`: `done(FREE_JAFFA, Some(DAKARA))` with `with_mission(.., 1570, "active")`, then `"completed"`, each give an empty list.
     4. `live_db_1570_entry_needs_dakara_and_63100`: `done(FREE_JAFFA, Some(STORY_RM))` and `done(FREE_JAFFA, None)` give an empty list; `fire_mission_completed(&engine, &with_mission(ctx_for(FREE_JAFFA, Some(DAKARA)), 1324, "completed"), 1324)` gives an empty list; and `fire_player_loaded(&engine, &ctx_for(FREE_JAFFA, Some(DAKARA)), DAKARA_NAME)` gives an empty `labels_in(&r, ENTRY_CHAINS)` (arrival alone gives no 1570 under F).

     **Fallback B tests** (instead of the four above): 1. `live_db_free_jaffa_arriving_on_dakara_accepts_1570_at_bratac`: `ctx_for(FREE_JAFFA, Some(DAKARA))`, `fire_player_loaded(&engine, &ctx, DAKARA_NAME)`; the first three labels above, `assert_no_deferred_actions(&r, 8040)`. 2. Praxis and Tau'ri on Dakara: empty. 3. 1570 `active`, `completed`, then 1645 `active`: empty. 4. `Some(STORY_RM)` firing `STORY_RM_NAME`, and `None` firing `DAKARA_NAME`: empty.

**Checks, in order** (from the worktree root):

1. `pwsh tools/build-lane/lane.ps1 cargo fmt -p cimmeria-cell-content`, then `pwsh tools/build-lane/lane.ps1 cargo fmt -p cimmeria-content-engine`
2. `pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-cell-content --all-targets -- -D warnings`, then the same `-p cimmeria-content-engine`
3. `pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-content-engine interact_tag_linter` (the allowlist, `every_chain_seed_file_is_registered_in_database_sql` and the region check pass)
4. `pwsh tools/build-lane/live-db-test.ps1 chain_replay_tests::dakara_e1` (the new module and `dakara_e1_space` both pass: 8001 and 8002 are unchanged by 8040)

**Test type and revert proof.** Chain replay, live-DB (TESTING.md type 6): the tests load the real seed through `build_engine(Some(&pool))`. Test 1 fails when chain 8040 is removed or loses an action; tests 2 to 4 fail when the archetype, mission or world condition is dropped, or when the trigger is `player_loaded` again (each would then resolve 8040). Mission 63100 need not exist for these tests: the trigger key is text and the context is built by hand.

**Docs rows owed.** "Game systems, content chains, or content-engine actions": the `docs/content/mission-chains.md` row is DK-99's, once for the campaign; say so in the report. The ledger row is the reviewer's.

**Done when** the four checks pass and the report pastes each final `status=` line and the step-1 file list.

## DK-10e: reword the arrival notice and send Teal'c's letter

| Field | Value |
|---|---|
| Role | `packet-coder` |
| Gate | D-DK1 answered **F** (recommended) or **B**. D-DK17 is decided: the owner accepted both texts below as written (2026-10-18). Not dispatched under A, C or D. |
| Branch / worktree | `dakara/dk-10e-arrival-notice` / `dk-10e` |
| Commit subject | `feat(content): Dakara_E1 arrival notice names Bra'tac, and Teal'c's letter (DK-10e)` |
| Report | `<worktree>\.claude\reports\dakara\DK-10e-packet-coder.md` |
| Size | About 22k tokens |

**Why** (F19, D-DK1 option F). Chain 8002 mails every Free Jaffa, once, "The Free Jaffa command on Dakara is not staffed yet". After wave 3 it is staffed. The new chain 8007 sends a second one-time mail, Teal'c's letter, which tells the backstory the shipped dialogs assume (the player carried Bra'tac's warning to the SGC). Each mail's cooldown is per character and permanent, so only characters created after this merges see the new notice; every Free Jaffa who arrives after the merge gets the letter once, including older characters (accepted).

**Contract excerpt** (C3, C4 of [work-packets.md](../work-packets.md)). `dakara_e1_space_chains.sql` is block 8001-8039 (space and travel); DK-10e edits only chain 8002's mail text and adds chain 8007 after it. A `send_system_mail` action's params are `sender`, `subject` (at most 128 characters each), `body` (at most 1,000), `cooldown_secs`, `quiet_cooldown`; the cooldown key is `send_system_mail/<chain_id>`. Every chain on one trigger fires; 8001, 8002 and 8007 all fire on `player_loaded Dakara_E1` for archetype 7.

**Texts** (`NEW CONTENT`, accepted by the owner). Escape every `'` as `''` in SQL; in Rust strings it stays `'`.

| Chain | Sender | Subject | Body |
|---|---|---|---|
| 8002 (edit) | `Dakara Gate Watch` (unchanged) | `Dakara: report to Bra'tac` | `Bra'tac is waiting for you in the command tent beside the Stargate plaza. The DHD in front of the Stargate dials Omega Site. The DHD at Omega Site brings you back to Dakara.` |
| 8007 (new) | `Teal'c` | `Dakara: find SG-18` | `You carried Master Bra'tac's warning to Stargate Command, but Ba'al's Jaffa reached the base first. They came through with SG-18's iris code, and SG-18 has not reported from Dakara since. Now that you are home, tell Master Bra'tac what you saw at the SGC, and find SG-18.` |

**Steps.**

1. `db/resources/Content/Seed/dakara_e1_space_chains.sql` (CRLF; keep it so):
   - Chain 8002's action row (line 122): replace the `subject` and `body` values in the params JSON; keep `sender`, `cooldown_secs` and `quiet_cooldown`. In the comment above it (lines 90-93), replace the sentence "World 61 has no mission, no NPC and no dialog for a level-1 Free Jaffa yet (OD-DK01, option A), so the notice says so and says how to leave and come back." with "Since DK-10e the notice names the first mission giver and says how to leave and come back (D-DK17)."
   - After line 122 and before the `-- DK-04` banner (line 124), add chain 8007 in the same shape as 8002: a comment block (`-- Chain 8007 NEW CONTENT (DK-10e, D-DK1 F, D-DK17): Teal'c's letter, once per character, with the quiet cooldown as 8002. Written by this project; Teal'c is a canon sender, accepted by the owner (2026-10-18).`), then `content_chains` `(8007, 'Dakara_E1 - Free Jaffa arrival: Teal''c''s letter by mail (DK-10e)', 'space', 61, true, 0)`, trigger `(8007, 'player_loaded', 'Dakara_E1', 'player', false, 0)`, condition `(8007, 'archetype', NULL, NULL, 'eq', '7', 0)`, action `(8007, 'send_system_mail', NULL, NULL, '{"sender": "Teal''c", "subject": "Dakara: find SG-18", "body": "<8007 body, every '' doubled>", "cooldown_secs": 2147483647, "quiet_cooldown": true}', 0, 0)`.
2. `crates/cell-content/src/cell/content/chain_replay_tests/dakara_e1_space.rs` (the file is 400 lines or fewer after this; check):
   - Add `const LETTER_CHAIN: i64 = 8007;` after `NOTICE_CHAIN`.
   - `notice_action()` (lines 60-73): the new 8002 subject and body.
   - Add `letter_action()` (the 8007 action, `sender_name: "Teal'c"`) and `letter_mail(entity_id)` in the shape of `notice_mail` with `chain_id: LETTER_CHAIN` and the cooldown key `send_system_mail/8007`. To avoid a copy, `notice_mail` and `letter_mail` may share one private `mail_for(entity_id, chain_id, action)` helper.
   - Test 1 (line 182): the exact vec gains `(LETTER_CHAIN, letter_action())` after the notice; add `super::assert_no_deferred_actions(&resolved, LETTER_CHAIN);`. The engine orders actions by chain; if the run shows another order, match the run and say so in the report. Rename the test `..._resolves_exactly_the_grant_the_notice_and_the_letter` and update its doc comment.
   - Test 2: line 208's `len()` is 3; line 239's list is `[GRANT_CHAIN, NOTICE_CHAIN, LETTER_CHAIN]`.
   - Test 3: the destructure at lines 267-275 becomes four messages, `[update, grant, notice, letter]`; line 290 asserts both mails (`notice` against `notice_mail`, `letter` against `letter_mail`). Rename `assert_only_the_notice` to `assert_only_the_mails`: `sent` is exactly the two mail requests, notice then letter. The relog (line 313) uses it. Update the doc comments that say "one `ContentSystemMail`" or "the notice alone".
   - The visitor test (from line 321) is unchanged: a visitor still gets nothing.
3. `docs/analysis/dakara-e1-rebuild/README.md`, the M1 checklist's DK-U2 row, and `docs/guides/unified-uat.md`, its DK-U2 row: the expected subject and the three sentences match the new 8002 text, and the row adds "and a second mail from **Teal'c**, 'Dakara: find SG-18' (see DK-U13)". Drop the README row's "After DK-10e ships ..." note. Change no other row.

**Checks, in order:** `pwsh tools/build-lane/lane.ps1 cargo fmt -p cimmeria-cell-content`; `pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-cell-content --all-targets -- -D warnings`; `pwsh tools/build-lane/live-db-test.ps1 chain_replay_tests::dakara_e1_space`; `pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-content-engine interact_tag_linter`; `pwsh tools/lint-md.ps1` on the two Markdown files; CRLF check on both and on the seed file.

**Test type and revert proof.** Chain replay, resolved and executed, live-DB (TESTING.md type 6). The exact vec and the executed four-message destructure fail when chain 8007 is removed, when either text drifts from the seed, or when the quiet cooldown is dropped.

**Docs rows owed.** "UAT steps for a restored system": the campaign checklist and `docs/guides/unified-uat.md`, both in step 3. List them in the report.

**Done when** the checks pass and `rg -n "no orders yet"` finds the old subject only in `docs/analysis/dakara-e1-rebuild/worknotes/` and the ledger's F19 row.
