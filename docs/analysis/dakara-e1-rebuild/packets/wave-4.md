# Dakara_E1 wave 4: scaffolding for missions 1648-1654, and the story props

> Type: how-to (work packets). Audience: the packet-coder named in each packet, and the read-only wave-4 packet-reviewer (no packet-review-fixer: the reviewer's findings go to a fresh packet-coder).
> Written 2026-10-19 (architect, autonomous) against `main` @ `eb9eaebe7`. Ledger and status: [README](../README.md#packets). Rules and full contract: [work-packets.md](../work-packets.md).

Wave 4 has two packets that touch disjoint files, so they run in parallel, each in its own worktree:

| Packet | Gate | Files |
|---|---|---|
| [DK-40](#dk-40-seed-files-and-test-scaffolding-for-missions-1648-1654) | DK-06b merged (it adds `Action::DisplayRadioDialog` and its `labels_in` arm) | nine new seed files, `db/database.sql`, the interact-tag linter, the chain-replay `dakara_e1_missions/` folder |
| [DK-41](#dk-41-story-props-terminals-ring-controls-power-supplies) | none | the Dakara template and spawnlist seeds, the `spawner_tests/dakara_e1/` folder, the placements README |

Waves 5 and 6 need both merged. Every packet carries its own rules; you do not need CLAUDE.md.

## Rules for every wave-4 packet

1. PowerShell only. No bash, WSL or Git Bash. Use absolute paths inside your worktree; never `cd` to the root checkout. Prefix each lane call with `Set-Location <worktree>;` and check that the lane log lands under `logs/<worktree>/`.
2. Every compiling `cargo` call goes through the lane, run from the worktree root: `pwsh tools/build-lane/lane.ps1 cargo ...`. Never call `cargo` directly; never add `--target-dir`. Live-DB tests run through `pwsh tools/build-lane/live-db-test.ps1 <filter>`. Run lane checks in the foreground.
3. Never `git stash`, `git worktree prune`, `git push`, `gh pr`, or delete a worktree. Commit once, locally, with the subject below and the attribution lines from your brief.
4. Edit only the files your packet names. Files stay under 700 lines (soft cap 500).
5. Seed files and every file under `docs/` are CRLF; keep them CRLF (check with `[IO.File]::ReadAllText(<abs>).Contains("`r`n")` and that no bare LF is left). A new file written with LF is converted with `[IO.File]::WriteAllText(<abs>, ([IO.File]::ReadAllText(<abs>) -replace "(?<!\r)\n", "`r`n"))`. `db/database.sql` keeps whatever ending it has.
6. Seeds change in `db/resources/` only; never add a `db/scripts/` migration. Never edit templates 54 or 59 (shared with Harset).
7. No new dialog row, no client patch, no rewards (D-DK7).
8. Never launch the game client, the launcher or any lab tool.
9. No IPs, credentials, tokens or account names in any file.
10. Report to `<worktree>\.claude\reports\dakara\<packet>-packet-coder.md`: files changed, each check's final `status=` line, the commit hash, every deviation and why. Return exactly one line: `<packet> <done|blocked|failed> <commit-sha|-> <report-path>`. A context stop past about 90k tokens: commit only passing work (subject suffix `(part N)`), write `docs/analysis/dakara-e1-rebuild/worknotes/<packet>.md` and return `blocked` with the reason `context`.

## DK-40: seed files and test scaffolding for missions 1648-1654

| Field | Value |
|---|---|
| Role | `packet-coder` |
| Gate | DK-06b merged; branch from a `main` that has it (`git log --oneline -1 -- crates/content-engine/src/actions.rs` shows DK-06b, and `labels_in` in `dakara_e1_missions/mod.rs` has a `DisplayRadioDialog` arm) |
| Branch / worktree | `dakara/dk-40-wave5-scaffold` / `dk-40` |
| Commit subject | `test(content): Dakara_E1 missions 1648-1654 seed files and replay scaffolding (DK-40)` |
| Report | `<worktree>\.claude\reports\dakara\DK-40-packet-coder.md` |
| Size | About 35k tokens |

**Contract excerpt** (C3 and C7 of [work-packets.md](../work-packets.md); D-DK21, D-DK22). Waves 5 and 6 write one seed file and one test submodule each, and never edit `db/database.sql`, the linter or `mod.rs`. This packet creates all of those shared pieces up front, as DK-10a did for wave 3.

**Steps.**

1. Create these nine seed files in `db/resources/Content/Seed/`, CRLF, each holding only the header and the search path:

   ```sql
   -- Dakara_E1 rebuild: mission 1648 "Loth'ta's Withdrawal" (offer, Loth'ta, turn-in).
   -- Campaign: docs/analysis/dakara-e1-rebuild/ (packet DK-13).
   -- Chain ID range: 8180-8199 (work-packets.md, C3). Evidence labels: README.md, "Evidence Labels".

   SET search_path = resources, pg_catalog;
   ```

   | File | Packet | Range | Scope line (the first header line after "mission") |
   |---|---|---|---|
   | `dakara_e1_m1648_chains.sql` | DK-13 | 8180-8199 | `1648 "Loth'ta's Withdrawal" (offer, Loth'ta, turn-in)` |
   | `dakara_e1_m1649_offer_chains.sql` | DK-14a | 8200-8219 | `1649 "Moh'katan" (offer, Bra'tac, the radio call, turn-in)` |
   | `dakara_e1_m1649_terminal_chains.sql` | DK-14b | 8220-8249 | `1649 "Moh'katan", step 4922 (the Command Terminal and the drops)` |
   | `dakara_e1_m1650_chains.sql` | DK-15 | 8250-8289 | `1650 "Confront Moh'katan"` |
   | `dakara_e1_m1651_chains.sql` | DK-30a | 8290-8309 | `1651 "Renewed Attack" (accept, Bra'tac, return)` |
   | `dakara_e1_m1651_gate_chains.sql` | DK-30b | 8310-8319 | `1651 "Renewed Attack", step 4930 (the DHD departure to SGC_W1, crossing C2)` |
   | `dakara_e1_m1652_chains.sql` | DK-31b | 8320-8349 | `1652 "Fireball" (the two ring controls)` |
   | `dakara_e1_m1653_chains.sql` | DK-32a | 8350-8389 | `1653 "Superweapon" (courtyard, power supplies, radio, Bra'tac)` |
   | `dakara_e1_m1654_chains.sql` | DK-33 | 8430-8449 | `1654 "Aftermath" (offer and the DHD departure, crossing C3)` |

   8390-8429 (DK-21, mission 1647) and 8450-8479 (DK-32b, the 1653 guards) are reserved and get no file now (D-DK22).

2. `db/database.sql`: directly after the line `\ir resources/Content/Seed/dakara_e1_m1646_drop_chains.sql`, add the nine `\ir resources/Content/Seed/<file>` lines in the table's order. Find the line by its text.
3. `crates/content-engine/tests/it/interact_tag_linter.rs`, `fn allowlist`: directly after `| ("dakara_e1_m1570_report_chains.sql", 8160..=8179)`, add one line per file in the table's order, for example `| ("dakara_e1_m1648_chains.sql", 8180..=8199)`. In the comment above that block, change `(DK-10a to DK-12c)` to `(DK-10a to DK-33)`. Change nothing else.
4. `crates/cell-content/src/cell/content/chain_replay_tests/dakara_e1_missions/mod.rs`:
   - Add `mod labels; mod m1648; mod m1649_offer; mod m1649_terminal; mod m1650; mod m1651; mod m1651_gate; mod m1652; mod m1653; mod m1654;` in alphabetical order among the existing `mod` lines (rustfmt sorts them).
   - Add nine range consts after `M1646_DROP_CHAINS`, each with `#[allow(dead_code)] // used by the wave-5 and wave-6 submodules (work-packets.md C7)`: `M1648_CHAINS` 8180..=8199, `M1649_OFFER_CHAINS` 8200..=8219, `M1649_TERMINAL_CHAINS` 8220..=8249, `M1650_CHAINS` 8250..=8289, `M1651_CHAINS` 8290..=8309, `M1651_GATE_CHAINS` 8310..=8319, `M1652_CHAINS` 8320..=8349, `M1653_CHAINS` 8350..=8389, `M1654_CHAINS` 8430..=8449.
   - In `labels_in`, before the `other =>` arm, add:

     ```rust
     Action::CrossWorldTeleport { world_name, position: p } => format!(
         "cross_world_teleport({world_name}@{:.2},{:.2},{:.2})",
         p[0], p[1], p[2]
     ),
     Action::SpawnEntity { template_id, tag, .. } => {
         format!("spawn_entity({template_id}#{tag})")
     }
     Action::DespawnEntity { entity_tag } => format!("despawn_entity({entity_tag})"),
     Action::DestroyTaggedEntity { entity_tag } => format!("destroy_entity({entity_tag})"),
     Action::PlaySequence { sequence_id } => format!("play_sequence({sequence_id})"),
     ```

     If a variant's field names differ from these (read `crates/content-engine/src/actions.rs`), use the real names and keep the label text; say so in the report.
   - After `labels_in`, add two helpers, each with `#[allow(dead_code)] // used by the wave-5 and wave-6 submodules (work-packets.md C7)`:

     ```rust
     /// The `delay_ms` of each action of `chain`, in order (`ResolvedActions`
     /// keeps one delay per action, index for index).
     fn delays_of(resolved: &ResolvedActions, chain: i64) -> Vec<i32> {
         resolved
             .actions
             .iter()
             .zip(resolved.action_delays.iter())
             .filter(|((id, _), _)| *id == chain)
             .map(|(_, d)| *d)
             .collect()
     }

     /// How many `display_radio_dialog` actions the whole engine resolved.
     fn radio_count(resolved: &ResolvedActions) -> usize {
         resolved
             .actions
             .iter()
             .filter(|(_, a)| matches!(a, Action::DisplayRadioDialog { .. }))
             .count()
     }
     ```

     If `report.rs` (DK-10d-A) already defines its own `delays_of` or `radio_count`, leave it: a local item shadows the glob import, and clippy accepts it. Do not edit `report.rs`.
5. New submodule files, each one `//!` line naming the packet, the mission, the seed file and the range, for example `//! DK-13: mission 1648 "Loth'ta's Withdrawal" (dakara_e1_m1648_chains.sql, 8180-8199).`: `m1648.rs`, `m1649_offer.rs`, `m1649_terminal.rs`, `m1650.rs`, `m1651.rs`, `m1651_gate.rs`, `m1652.rs`, `m1653.rs`, `m1654.rs`.
6. New `labels.rs`, the scaffold's own test (plain `#[test]`, no database):

   ```rust
   //! DK-40: the label arms and helpers that waves 5 and 6 assert with.
   use std::collections::HashMap;

   use super::*;

   #[test]
   fn labels_and_delays_cover_the_wave_5_and_6_verbs() {
       let r = ResolvedActions {
           actions: vec![
               (8290, Action::DisplayRadioDialog { dialog_id: 5833 }),
               (8290, Action::CrossWorldTeleport { world_name: "SGC_W1".into(), position: [172.79, -10.9, 12.0] }),
               (8290, Action::SpawnEntity { template_id: 443, position: [66.2, -0.12, 34.3], heading: 1.5708, tag: "Dakara_E1_StoryRm_BaalHologram".into(), is_stationary: Some(true), aggression: None, allow_shared: None }),
               (8290, Action::DespawnEntity { entity_tag: "Dakara_E1_StoryRm_BaalHologram".into() }),
               (8290, Action::PlaySequence { sequence_id: 1194 }),
               (9999, Action::DisplayRadioDialog { dialog_id: 1 }),
           ],
           action_delays: vec![2000, 0, 0, 0, 0, 0],
           params: HashMap::new(),
       };
       assert_eq!(
           labels_in(&r, M1651_CHAINS),
           [
               "8290:display_radio_dialog(5833)",
               "8290:cross_world_teleport(SGC_W1@172.79,-10.90,12.00)",
               "8290:spawn_entity(443#Dakara_E1_StoryRm_BaalHologram)",
               "8290:despawn_entity(Dakara_E1_StoryRm_BaalHologram)",
               "8290:play_sequence(1194)",
           ]
       );
       assert_eq!(delays_of(&r, 8290), [2000, 0, 0, 0, 0]);
       assert_eq!(radio_count(&r), 2);
   }
   ```

   Build the struct literal with the real field set (if `ResolvedActions` has more fields, fill them with their empty value; if it is `#[non_exhaustive]`, build it with `Default` and push). Using `M1651_CHAINS` here means it no longer needs its `allow`; keep the attribute anyway (harmless) or drop it, and say which.

**Checks, in order** (from the worktree root):

1. `pwsh tools/build-lane/lane.ps1 cargo fmt -p cimmeria-cell-content`, then the same `-p cimmeria-content-engine`
2. `pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-cell-content --all-targets -- -D warnings`, then the same `-p cimmeria-content-engine`
3. `pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-content-engine interact_tag_linter` (`every_chain_seed_file_is_registered_in_database_sql` passes with the nine files)
4. `pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-cell-content dakara_e1_missions::labels`
5. `pwsh tools/build-lane/live-db-test.ps1 chain_replay_tests::dakara_e1_missions` (every wave-3 test still passes; the empty files load)

**Test type and revert proof.** `labels.rs` is a unit test (TESTING.md type 1): removing any new arm turns its label into `OTHER(...)`, and removing `delays_of` or `radio_count` stops the build. The linter check fails if a file is not registered in `database.sql`.

**Docs rows owed.** None: test scaffolding and empty seed files. Say so in the report.

## DK-41: story props (terminals, ring controls, power supplies)

| Field | Value |
|---|---|
| Role | `packet-coder` |
| Gate | none |
| Branch / worktree | `dakara/dk-41-story-props` / `dk-41` |
| Commit subject | `feat(content): Dakara_E1 story props for missions 1649-1653 (DK-41)` |
| Report | `<worktree>\.claude\reports\dakara\DK-41-packet-coder.md` |
| Size | About 35k tokens |

**Contract excerpt** (D-DK29, D-DK31; placements ledger rows A-05, A-12, A-13, A-15, A-16, B-06). Missions 1649, 1650, 1652 and 1653 click six props that have templates (DK-03) but no spawn. Each prop's click is a per-player chain; the prop's interaction bit comes only from a per-player dialog bind, so every template stays `interaction_type 0`. Mission 1652 has two ring controls and 1653 two power supplies, each with its own per-player bind, so the second of each pair gets its own template, a clone (as DK-09 cloned 449 into 458-461). No vocuum (457) spawn: its objective 5643 is optional and has no placement.

**Steps.**

1. `db/resources/Entities/Seed/entity_templates_dakara_e1.sql`:
   - Header: change `Id block 440-461 (cast and props; DK-09 added 458-461). 462-479 is reserved for DK-20's hostiles` to `Id block 440-463 (cast and props; DK-09 added 458-461, DK-41 462-463). 464-479 is reserved for DK-20's hostiles`.
   - At the end of the file, add a comment and two rows. 462 copies 455's row exactly except `template_id` 462 and `template_name` `'Dakara E1 - Ring Control 2'`; 463 copies 456's row exactly except `template_id` 463 and `template_name` `'Dakara E1 - Turret Power Supply 2'`. Copy the column list and the values from the 455 and 456 rows byte for byte.

     ```sql
     -- 462-463 (DK-41, D-DK29): Ring Control #2 and Power Supply #2, clones of
     --   455 and 456, so each of mission 1652's two ring controls and mission
     --   1653's two power supplies holds its own per-player bind. name_ids
     --   26731 and 26732: ORIGINAL_DATA; the client has one string for each
     --   pair. look: RECONSTRUCTION, as 455 and 456.
     ```

2. `db/resources/Worlds/Seed/spawnlist_dakara_e1.sql`, at the end, a comment and six rows in the file's row shape:

   ```sql
   -- DK-41 PROJECT_FINAL_RECONSTRUCTION. The story props of missions 1649
   -- (Command Terminal, PL-DK-A-05, off-mesh on the tent table), 1650
   -- (Moh'katan's Terminal, PL-DK-B-06, world 62), 1652 (the ring controls on
   -- the two Ha'tak ring pads, PL-DK-A-12 and A-13) and 1653 (the power
   -- supplies by the two ion cannons, PL-DK-A-15 and A-16). No bind is applied
   -- here: every prop's dialog and its interaction bit are a per-player bind
   -- from its mission chain (D-DK31).
   INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary, respawn_secs) VALUES (8016, 137.10, -21.38, 274.20, 4.1891, 61, 453, 'Dakara_E1_CommandTerminal', NULL, true, 30);
   INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary, respawn_secs) VALUES (8017, 62.50, 0.02, 40.50, 1.8158, 62, 454, 'Dakara_E1_StoryRm_MohkatanTerminal', NULL, true, 30);
   INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary, respawn_secs) VALUES (8018, -139.33, -16.17, 48.95, 0.0000, 61, 455, 'Dakara_E1_RingControl_XMinus', NULL, true, 30);
   INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary, respawn_secs) VALUES (8019, 358.03, -16.02, 102.07, 0.0000, 61, 462, 'Dakara_E1_RingControl_XPlus', NULL, true, 30);
   INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary, respawn_secs) VALUES (8020, 56.50, -12.36, -73.70, 1.4724, 61, 456, 'Dakara_E1_PowerSupply1', NULL, true, 30);
   INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary, respawn_secs) VALUES (8021, 202.20, -12.36, -59.40, 4.6152, 61, 463, 'Dakara_E1_PowerSupply2', NULL, true, 30);
   ```

   If the file ends with a sequence footer (`setval`), put the rows above it and raise its floor to 8021 if it names 8015.
3. `crates/cell-world/src/cell/spawner_tests/dakara_e1/`:
   - `mod.rs`: `CAST_AND_PROPS` becomes `(440, 463)`; its doc comment says `(DK-03, plus DK-09's drop-location clones 458-461 and DK-41's prop clones 462-463). 464-479, the rest of the campaign's block, is DK-20's ...`. Add `mod story_props;` and a doc bullet: `` - [story_props]: DK-41, the six story-prop spawns of missions 1649-1653 (8016-8021) and the two prop clones 462-463. ``
   - `templates.rs`: `ROSTER` becomes `[(i32, &str, &str); 24]` with two rows after 461: `(462, "being", "DN_ob_DakaraE1_int_RingControl")` and `(463, "being", "DN_ob_DakaraE1_int_TurretPowerSupply")`. Change its `//!` line `(440-461)` to `(440-463)`. Grep the folder for `461`, `22` and `twenty-two`; update only text that means the block's end or size, and list each change in the report.
   - Grep `crates/` for `8015` and `BETWEEN 8000`; if another test counts Dakara spawn rows, update its count and say so. Do not change DK-09's `props.rs` count (it counts 8008-8015 only).
   - New `story_props.rs` (about 120 lines), modelled on `props.rs`:

     ```rust
     //! DK-41 production seed guards: the story props of missions 1649-1653
     //! (spawns 8016-8021) and the clones 462 (Ring Control) and 463 (Power Supply).
     use super::*;

     /// The plaza point every connected Dakara_E1 spawn must reach on foot.
     const PLAZA: [f32; 3] = [100.0, -17.4, 230.0];

     /// `(spawn id, world, template id, tag, position, plaza-connected)`.
     /// A-05 sits on the tent table and B-06 on its pedestal, both off-mesh,
     /// so neither gets a mesh check.
     const STORY_PROPS: [(i32, i32, i32, &str, [f32; 3], bool); 6] = [
         (8016, 61, 453, "Dakara_E1_CommandTerminal", [137.1, -21.38, 274.2], false),
         (8017, 62, 454, "Dakara_E1_StoryRm_MohkatanTerminal", [62.5, 0.02, 40.5], false),
         (8018, 61, 455, "Dakara_E1_RingControl_XMinus", [-139.33, -16.17, 48.95], true),
         (8019, 61, 462, "Dakara_E1_RingControl_XPlus", [358.03, -16.02, 102.07], true),
         (8020, 61, 456, "Dakara_E1_PowerSupply1", [56.5, -12.36, -73.7], true),
         (8021, 61, 463, "Dakara_E1_PowerSupply2", [202.2, -12.36, -59.4], true),
     ];
     ```

     Tests (`#[tokio::test] async fn`, `let pool = require_db_or_skip!();`):
     1. `live_db_dakara_story_props_match_the_placement_ledger`: `count(*)` of `spawn_id BETWEEN 8016 AND 8021` is 6; for each row, `(world_id, template_id, tag, is_stationary, respawn_secs)` equals `(world, template, tag, true, Some(30))` and `[x, y, z]` equals the position (message `"spawn {id} moved: update the ledger"`); for the connected rows, `travel::standable_and_connected(&exterior, actual, PLAZA, tag)` with `let exterior = travel::mesh("dakara_e1.nav");` as in `props.rs`.
     2. `live_db_prop_clones_wear_their_sources_look`: for the pairs (455, 462) and (456, 463), the `static_mesh, body_set, name_id, flags, interaction_type, class` columns (as text, the query in `props.rs`'s look test) are equal, and `interaction_type` is `"0"` on all four; each of the four templates is the template of exactly one spawn row.

   If a connected row fails `standable_and_connected`, do not move it: change that row's flag to `false`, call `travel::standable` for it instead (as `props.rs` does for the remains), and write the failure in the report as a finding for the M2 position pass. If `standable` fails too, return `blocked` with the message.
4. `docs/analysis/dakara-e1-rebuild/placements/README.md` (CRLF): extend the line that begins `Seeded so far:` with `, and A-05, B-06, A-12, A-13, A-15 and A-16 as 8016-8021 (DK-41)`.

**Checks, in order** (from the worktree root):

1. `pwsh tools/build-lane/lane.ps1 cargo fmt -p cimmeria-cell-world`
2. `pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-cell-world --all-targets -- -D warnings`
3. `pwsh tools/build-lane/live-db-test.ps1 spawner_tests::dakara_e1` (all of templates, props, population, travel and story_props pass)
4. `pwsh tools/build-lane/live-db-test.ps1 debug_area_lineup` (`cimmeria-cell-catalog`; the lineup counts looks, the clones add none, so it still passes)

**Test type and revert proof.** Seed-data guard, live-DB (TESTING.md type 6). Removing a spawn row fails test 1's count; removing 462 or 463 fails `templates.rs`'s roster and test 2; giving a clone a different look fails test 2.

**Docs rows owed.** The placements README line (step 4). The `docs/content/` rows are DK-99's.
