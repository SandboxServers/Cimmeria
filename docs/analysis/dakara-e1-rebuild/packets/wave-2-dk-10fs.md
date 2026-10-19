# Dakara_E1 wave 2: DK-10fs, the bridge mission's catalog entry and seed

> Type: how-to (work packet). Audience: the packet-coder, and the wave-2 packet-reviewer (read-only; a packet-coder applies its findings).
> Written 2026-10-18 against `main` @ `d1bba0400`, for D-DK1 option F. Ledger and status: [README](../README.md#packets). The other wave-2 packets and the wave rules: [wave-2.md](wave-2.md).

## DK-10fs: mission 63100 "Home to Dakara", catalog entry and seed

| Field | Value |
|---|---|
| Role | `packet-coder` |
| Gate | D-DK1 answered **F** and D-DK20 answered (Dakara goes first). Not dispatched under A, B, C, D or E. |
| Branch / worktree | `dakara/dk-10fs-home-catalog` / `dk-10fs` |
| Commit subject | `feat(content): Dakara_E1 authored level-1 mission 63100 catalog entry and seed (DK-10fs)` |
| Report | `<worktree>\.claude\reports\dakara\DK-10fs-packet-coder.md` |
| Size | About 38k tokens |

### Rules (this packet carries its own; you do not need CLAUDE.md)

1. PowerShell only. No bash, WSL or Git Bash. Use absolute paths inside your worktree; never `cd` to the root checkout. Prefix each lane call with `Set-Location <worktree>;` and check that the lane log lands under `logs/<worktree>/`.
2. Every compiling `cargo` call goes through the lane, run from the worktree root, in the foreground: `pwsh tools/build-lane/lane.ps1 cargo ...`. Never call `cargo` directly; never add `--target-dir`. Live-DB tests run through `pwsh tools/build-lane/live-db-test.ps1 <filter>`, which reloads your own `sgw_<worktree>` database from `db/database.sql`.
3. Never `git stash`, `git worktree prune`, `git push`, `gh pr`, or delete a worktree. Commit once, locally, with the subject above and the attribution lines from your brief.
4. Edit only the files named below. Files stay under 700 lines (soft cap 500); the existing `tests/mission_additions.rs` is 610 lines, so your new tests go in a new file and that file must not grow.
5. Markdown under `docs/` and the new seed file are CRLF. Keep the line endings of every file you edit: check with `[IO.File]::ReadAllText(<abs>).Contains("`r`n")`; convert a new file with `[IO.File]::WriteAllText(<abs>, ([IO.File]::ReadAllText(<abs>) -replace "(?<!\r)\n", "`r`n"))`. Rust files stay as they are (LF).
6. Seeds change in `db/resources/` only. Never add a `db/scripts/` migration. Every seed row carries an evidence-label comment; every row here is `NEW CONTENT`.
7. This packet is the campaign's one exception to "no cooked-data override": it adds one `MISSION_ADDITIONS` entry (D-DK1 F). No dialog row, no dialog or step-text override, no client patch. The XML below is frozen: copy it byte for byte.
8. Never launch the game client, the launcher or any lab tool.
9. No IPs, credentials, tokens or account names in any file.
10. Report to the path above: files changed, each check's final `status=` line, the commit hash, every deviation and why. Return exactly one line: `DK-10fs <done|blocked|failed> <commit-sha|-> <report-path>`. A context stop past about 90k tokens: commit only passing work (subject suffix `(part N)`), write `docs/analysis/dakara-e1-rebuild/worknotes/DK-10fs.md` (done, left, next step, last `status=` line) and return `blocked` with the reason `context`.

### Contract excerpt (README § D-DK1 option F, F23, F24; work-packets.md C2, C6)

- `MissionAddition { mission_id: u32, xml: &'static str }` and `pub const MISSION_ADDITIONS: &[MissionAddition] = &[];` live in `crates/resources/src/base/mission_additions.rs`. `ResourceCache::load_all(dir)` appends `MISSION_ADDITIONS` to the served mission category (3) and bumps its metadata by `compute_metadata_bump(MISSION_OVERRIDES, STEP_TEXT_OVERRIDES, MISSION_ADDITIONS)`. `ResourceCache::load_all_with_mission_additions(dir, additions)` uses `additions` **instead of** the production list (test-only path).
- The stock category 3 has metadata 7538 and 1040 entries. After this packet, `load_all` serves 1041, the extra one being 63100.
- Ids: mission 63100, step 63101, objective 63102, task 63103. All four are absent from the stock catalog and from `db/resources`.
- The server's loaders read `resources.missions`, `resources.mission_steps` and `resources.mission_objectives` (`crates/cell-catalog/src/cell/spawner/missions.rs`: `load_mission_defs`, `load_step_objectives`). `accept_mission 63100` needs those rows; the client's journal needs the XML. The two must agree, and a test checks that.
- The chains that use 63100 are DK-10fc's (wave 3). This packet adds no chain.

**The frozen XML** (exactly 1206 bytes, no trailing newline). It is `TEST_MISSION_XML` with these replacements and nothing else: `CB-N05a catalog fixture` (twice) to `Home to Dakara`; `MissionID="61000"` to `"63100"`; `StepID="61001"` to `"63101"`; `ObjectiveID="61002"` to `"63102"`; `TaskID="61003"` to `"63103"`; `Verify the authored mission catalog.` to `Speak with Rak'nor at the Stargate.`; `<DisplayLogText> </DisplayLogText>` to `<DisplayLogText>Rak'nor keeps watch over the Stargate plaza. Tell him what you saw at Stargate Command.</DisplayLogText>`. The apostrophes are raw `'`, as stock text carries them.

```text
<?xml version="1.0" encoding="UTF-8"?>
<COOKED_MISSION xmlns:SOAP-ENV="http://schemas.xmlsoap.org/soap/envelope/" xmlns:SOAP-ENC="http://schemas.xmlsoap.org/soap/encoding/" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns:xsd="http://www.w3.org/2001/XMLSchema" xmlns:CookedData1="SGW" AwardXP="true" ShowFactionChangeIcon="false" ShowInstanceIcon="false" ShowPVPIcon="false" IsShareable="false" NumRepeats="1" CanAbandon="false" CanRepeatOnFail="true" CanFail="false" Level="1" IsHidden="false" IsOverrideMission="false" IsEnabled="true" IsAStory="true" Difficulty="1" MissionLabel="General" MissionDefn="Home to Dakara" MissionID="63100"><HistoryText>Home to Dakara</HistoryText><Steps StepEnabled="false" StepID="63101" AwardXP="false" Difficulty="1"><StepDisplayLogText>Speak with Rak'nor at the Stargate.</StepDisplayLogText><Objectives IsOptional="false" ObjectiveID="63102" AwardXP="false" IsHidden="false" IsEnabled="false" Difficulty="1"><DisplayLogText>Rak'nor keeps watch over the Stargate plaza. Tell him what you saw at Stargate Command.</DisplayLogText><Tasks TaskType="1" TaskID="63103" AwardXP="false" IsEnabled="true" Difficulty="1"></Tasks></Objectives></Steps></COOKED_MISSION>
```

The two lines are one Rust raw string with one `\n` between them, exactly as `TEST_MISSION_XML` is written.

### Steps

1. New file `crates/resources/src/base/mission_additions_dakara.rs` (about 15 lines):

   ```rust
   //! Dakara_E1 authored missions (docs/analysis/dakara-e1-rebuild/, D-DK1 F).

   /// CIMMERIA_AUTHORING; NEW CONTENT. Mission 63100 "Home to Dakara": the
   /// level-1 bridge a Free Jaffa gets on first arrival on Dakara_E1 (chains
   /// 8075-8077), finished by talking to Rak'nor, which starts mission 1570.
   /// The exact transform of `TEST_MISSION_XML` (1206 bytes) is pinned by
   /// `tests/mission_additions_dakara.rs`; the seed rows that match it are in
   /// `db/resources/Missions/Seed/missions_dakara_e1.sql`.
   pub const DAKARA_HOME_XML: &str = r#"<the frozen XML>"#;
   ```

2. `crates/resources/src/lib.rs`: directly after line 34 (`pub mod mission_additions;`) add `pub mod mission_additions_dakara;`. If the line moved, find it by its text.
3. `crates/resources/src/base/mission_additions.rs`:
   - Line 2 becomes `//! Production entries are added here only once a content packet seals their XML.`
   - Line 13's doc becomes `/// Production authored entries. Each XML lives in its zone's own file.`
   - Line 14 becomes:

     ```rust
     pub const MISSION_ADDITIONS: &[MissionAddition] = &[
         // Dakara_E1 rebuild, D-DK1 F (DK-10fs).
         MissionAddition {
             mission_id: 63100,
             xml: crate::base::mission_additions_dakara::DAKARA_HOME_XML,
         },
     ];
     ```

4. `crates/resources/src/base/resources/tests/mission_additions.rs`, two edits and no other change:
   - Line 125, `assert!(MISSION_ADDITIONS.is_empty());`, becomes `assert_eq!(MISSION_ADDITIONS.iter().map(|a| a.mission_id).collect::<Vec<_>>(), [63100]);` (the fixture ids stay reserved; the production list now holds exactly the Dakara entry).
   - In `insertion_preserves_served_baseline_and_bumps_once`: line 130 becomes `let baseline = ResourceCache::load_all_with_mission_additions(DIR, &[]).unwrap();` and delete lines 131-138 (the `empty` load and its loop: it compared production with an empty list, which the new file now does with the expected difference). Lines 139 onward are unchanged.
   - Run `(Get-Content <abs>).Count` before and after: the file must not grow.
5. New file `crates/resources/src/base/resources/tests/mission_additions_dakara.rs` (about 140 lines). Header and imports:

   ```rust
   //! DK-10fs: the production Dakara entry 63100 (D-DK1 F). Its XML is the
   //! exact fixture transform, its ids are new, the served catalog gains it
   //! and nothing else, and the seed rows say what the XML says.

   use super::super::*;
   use crate::base::mission_additions::{MISSION_ADDITIONS, TEST_MISSION_XML};
   use crate::base::mission_additions_dakara::DAKARA_HOME_XML;
   use crate::base::mission_overrides::{MISSION_OVERRIDES, STEP_TEXT_OVERRIDES};

   const DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../data/cache");
   const IDS: [u32; 4] = [63100, 63101, 63102, 63103];
   ```

   Four `#[test]` functions:
   1. `dakara_home_is_the_exact_fixture_transform`: build the expected string from `TEST_MISSION_XML` with the `.replace(..)` chain of the contract (seven replacements, in the order written there) and `assert_eq!(DAKARA_HOME_XML, expected)`; `assert_eq!(DAKARA_HOME_XML.len(), 1206)`; `assert!(!DAKARA_HOME_XML.ends_with('\n'))`.
   2. `dakara_home_ids_are_absent_from_the_stock_catalog`: read `ResourceCache::load_raw_mission_category_for_test(DIR).unwrap()`; assert none of `IDS` is a key of `elements`; then scan every stock XML for `MissionID`, `StepID`, `ObjectiveID` and `TaskID` attributes and assert none of their values is in `IDS`. Copy the `quick_xml` reader loop of `tests/mission_additions.rs` lines 92-110 (`Reader::from_str`, `Event::Start(e) | Event::Empty(e)`, `e.attributes()`); add `use quick_xml::events::Event; use quick_xml::Reader;`.
   3. `load_all_serves_dakara_home_and_changes_nothing_else`: `let served = ResourceCache::load_all(DIR).unwrap();` and `let empty = ResourceCache::load_all_with_mission_additions(DIR, &[]).unwrap();`. For every `&(id, _)` in `CATEGORY_PAKS` with `id != 3`, `served.category(id) == empty.category(id)`. For category 3: `served` has `empty`'s entry count plus 1 (and that is 1041); `served.elements[&63100] == DAKARA_HOME_XML.as_bytes()`; every `(id, bytes)` of `empty`'s category 3 is in `served` unchanged; `served`'s metadata equals `7538u32.wrapping_add(compute_metadata_bump(MISSION_OVERRIDES, STEP_TEXT_OVERRIDES, MISSION_ADDITIONS))` and differs from `empty`'s; `served.overridden_elements(3)` contains 63100 exactly once.
   4. `dakara_seed_rows_match_the_xml`: read `concat!(env!("CARGO_MANIFEST_DIR"), "/../../db/resources/Missions/Seed/missions_dakara_e1.sql")` to a string; assert it contains each of: `VALUES (63100, 'Home to Dakara',`, `VALUES (63101, 63100,`, `'Speak with Rak''nor at the Stargate.'`, `VALUES (63102, 63101,`, `'Rak''nor keeps watch over the Stargate plaza. Tell him what you saw at Stargate Command.'`, `VALUES (63103, 63102,`. Message: `"missions_dakara_e1.sql drifted from DAKARA_HOME_XML: change both together"`.

   If a helper name in the snippets does not resolve (`CATEGORY_PAKS`, `compute_metadata_bump`, `overridden_elements`), use the one `tests/mission_additions.rs` uses and say so in the report.
6. `crates/resources/src/base/resources/tests/mod.rs`: add `mod mission_additions_dakara;` directly after `mod mission_additions;`, and a doc bullet after the `committed_paks` one: `` //! - [`mission_additions_dakara`]: the production Dakara mission 63100 (DK-10fs): exact XML, new ids, served catalog, seed agreement. ``
7. New file `db/resources/Missions/Seed/missions_dakara_e1.sql` (CRLF), exactly:

   ```sql
   -- Dakara_E1 rebuild: mission 63100 "Home to Dakara" (D-DK1 option F, packet DK-10fs).
   -- Campaign: docs/analysis/dakara-e1-rebuild/. NEW CONTENT: written by this
   -- project, no recovered counterpart. The client's copy is the MISSION_ADDITIONS
   -- entry in crates/resources/src/base/mission_additions_dakara.rs; a resources
   -- test fails if the two disagree. Ids 63100-63103. Level 1, no reward (D-DK7).

   SET search_path = resources, pg_catalog;

   INSERT INTO missions (mission_id, history_text, award_xp, can_abandon, can_fail, can_repeat_on_fail, difficulty, is_a_story, is_enabled, is_hidden, is_override_mission, is_shareable, level, mission_defn, mission_label, num_repeats, show_faction_change_icon, show_instance_icon, show_pvp_icon, script_name, reward_naq, reward_xp, script_spaces) VALUES (63100, 'Home to Dakara', true, false, false, true, 1, true, true, false, false, false, 1, 'Home to Dakara', 'General', 1, false, false, false, NULL, 0, 0, NULL);
   INSERT INTO mission_steps (step_id, mission_id, award_xp, difficulty, step_enabled, step_display_log_text, index) VALUES (63101, 63100, false, 1, false, 'Speak with Rak''nor at the Stargate.', 0);
   INSERT INTO mission_objectives (objective_id, step_id, award_xp, difficulty, is_enabled, is_hidden, is_optional, display_log_text) VALUES (63102, 63101, false, 1, false, false, false, 'Rak''nor keeps watch over the Stargate plaza. Tell him what you saw at Stargate Command.');
   INSERT INTO mission_tasks (task_id, objective_id, award_xp, difficulty, is_enabled, task_type) VALUES (63103, 63102, false, 1, true, 1);
   ```

   `missions_mission_id_seq` starts at 100000, so no `setval` is needed.
8. `db/database.sql`: directly after the line `\ir resources/Missions/Seed/missions.sql` add `\ir resources/Missions/Seed/missions_dakara_e1.sql`. Keep the file's line endings.
9. New file `crates/cell-catalog/src/cell/spawner/tests/live_db_dakara_bridge_mission.rs` (about 60 lines), in the shape of `live_db_content_loaders.rs` (`mod live_db { use crate::cell::spawner::*; use crate::test_support::require_db_or_skip; ... }`), one `#[tokio::test]`:
   - `live_db_dakara_bridge_mission_63100_loads_one_step_one_objective`: `load_mission_defs(&pool)` has key 63100 with `step_id == 63101`, `is_hidden == false`, `num_repeats == 1`, `can_repeat_on_fail == true`, and `objectives` exactly one entry `{ objective_id: 63102, is_hidden: false, is_optional: false }`; `load_step_objectives(&pool)[&63101]` is that one objective. A `//!` doc names DK-10fs and says why: `accept_mission 63100` (DK-10fc's chain 8075) reads these caches.
   - `crates/cell-catalog/src/cell/spawner/tests/mod.rs`: add `mod live_db_dakara_bridge_mission;` in alphabetical order (after `mod live_db_crafting_hub;`) and a doc bullet after the `live_db_content_loaders` one: `` //! - [`live_db_dakara_bridge_mission`]: the Dakara bridge mission 63100's seed rows load into the accept and advance caches (DK-10fs). ``
10. `docs/architecture/mission-pak-overrides.md` (CRLF), two passages only:
    - Lines 18-19, the sentence `` `MISSION_ADDITIONS` stays empty until an authored content packet seals production XML. `` becomes `` `MISSION_ADDITIONS` holds sealed production XML only: the first entry is Dakara_E1's mission 63100 (DK-10fs, docs/analysis/dakara-e1-rebuild/), whose XML lives in `mission_additions_dakara.rs`. `` (rewrap the paragraph to the file's width).
    - Lines 55-56 become: `Client gSOAP parsing, journal/dialog presentation, progression and relog remain **UATPending** until the owner's Dakara_E1 M2 run (DK-U11 to DK-U16 in the campaign ledger) supplies live evidence for mission 63100.`

### Checks, in order (from the worktree root)

1. `pwsh tools/build-lane/lane.ps1 cargo fmt -p cimmeria-resources`, then the same `-p cimmeria-cell-catalog`
2. `pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-resources --all-targets -- -D warnings`, then the same `-p cimmeria-cell-catalog`
3. `pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-resources -p cimmeria-base-session` (every test passes: the `mission_additions*`, `committed_paks`, `overrides` and `cooked_sync` tests read the production list)
4. `pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-base cache_routing`
5. `pwsh tools/build-lane/live-db-test.ps1 live_db_dakara_bridge` (the reload also proves the new seed file loads)
6. `pwsh tools/lint-md.ps1 docs/architecture/mission-pak-overrides.md`; CRLF check on that file and the new seed file.

If check 3 fails in a test this packet does not name, paste the failure in the report and return `blocked` with the reason `baseline`: do not weaken the test.

### Test type and revert proof

Unit and catalog tests (TESTING.md) for the resources crate; a live-DB seed guard (type 3) for the loader. Revert proof: removing the `MISSION_ADDITIONS` entry fails test 3 (63100 not served) and the edited line-125 assert; editing the XML fails test 1; removing or changing a seed row fails test 4 and the live-DB guard; dropping the `\ir` line fails the live-DB guard.

### Docs rows owed

"Cooked-data overrides" (doc-update-map): `docs/architecture/mission-pak-overrides.md`, step 10. The UAT rows (DK-U11 to DK-U16) are in the campaign ledger already; `docs/guides/unified-uat.md` is DK-99's. Name both in the report.

**Done when** the six checks pass and the report pastes each final `status=` line and the line counts of `tests/mission_additions.rs` before and after.
