# NPC combat reference: work packets, waves 1-2

Same dispatch rules, rules R1-R10 and contract C1-C6 as
[work-packets.md](work-packets.md). A packet here is not self-contained
until you have read that file's § Dispatch rules, § Rules and the contract
sections the packet names.

---

## Wave 1: wiring and the first references (5 packets, parallel)

The file sets are disjoint. NC-11, NC-14 and NC-12 own separate `reference/`
stubs. Each packet ships as its own PR. Merge order: NC-10, NC-15, NC-11,
NC-14, NC-12.

### NC-10 Wire the resolver and its diagnostics

- **Agent:** packet-coder. **Host:** windows. **Depends:** NC-01 merged.
- **Branch:** `npc-combat-reference/NC-10-wire-profile`. **Worktree:** `nc-10`.
- **Commit:** `feat(npc-ai): resolve NPC combat profiles with diagnostics (NC-10)`
- **Files:**
  - new `crates/cell-world/src/cell/combat/combat_profile/report.rs` (C2,
    with its tests inline in `#[cfg(test)] mod tests`);
  - `combat_profile/mod.rs` (add `mod report; pub use report::report_profile;`
    only);
  - `crates/cell-world/src/cell/space_manager/spawn.rs` (lines 314-325 only);
  - `crates/cell-combat/src/cell/combat/mod.rs` (line 21: add
    `combat_profile` to the `pub use cimmeria_cell_world::cell::combat::{...}`
    list);
  - `crates/cell-combat/src/cell/service/npc_ai/ability_select.rs` (lines
    63-66 only);
  - `docs/gameplay/npc-ai.md`.
- **Do:**
  1. In `spawn_npc_from_record_into`, replace the `if
     record.ability_ids.is_empty() { ... } else { ... }` block with:
     ```rust
     use super::super::combat::combat_profile::{report_profile, resolve_npc_combat_profile, PLACEHOLDER_POLICY};
     let profile = resolve_npc_combat_profile(&record.ability_ids, record.components.as_deref(), PLACEHOLDER_POLICY);
     for &ability_id in &profile.abilities { e.abilities.add_ability(ability_id); }
     report_profile(record.template_id, &record.template_name, &record.body_set, &profile);
     ```
     Replace the old comment with one that points at `combat_profile`.
     Leave `spawn_npc` (lines 37-39) alone: it is the raw primitive that
     tests use, and NC-15 inventories its callers.
  2. In `choose_npc_ability`, replace `Some(combat::NPC_DEFAULT_ABILITY)` with
     `combat::combat_profile::placeholder_ability(combat::combat_profile::PLACEHOLDER_POLICY)`,
     keeping the `.filter(...)`.
  3. Under `Legacy`, behaviour is unchanged: the same abilities, plus new
     log lines.
- **Tests:**
  - in `report.rs`, using `crate::test_support::LogCapture::install()` (see
    `crates/cell-world/src/cell/combat_debug/tests.rs` for use), with
    sentinel template ids `0x7000_0201`..:
    1. A placeholder profile logs one `combat_profile_placeholder` INFO with
       `template_id`, `template_name` and `weapon_component`; a second call
       for the same id logs nothing.
    2. An unresolved profile logs one `combat_profile_unresolved` WARN.
    3. An explicit set logs nothing.
  - in the existing spawn tests (`crates/cell-world/src/cell/spawner_tests/spawn_records.rs`;
    add a case there and list it in Files when you do):
    4. A record with empty `ability_ids` spawns knowing exactly [592].
    5. A record with [584, 710] spawns knowing exactly those.
  - Revert proof: dropping the `report_profile` call fails 1 and 2 through a
    spawn-path test. Make test 1 spawn through `spawn_npc_from_record`, not
    call `report_profile` directly.
- **Lane:** fmt, clippy and `nextest run` for `-p cimmeria-cell-world`
  (filters `combat_profile` and `spawn`), then `-p cimmeria-cell-combat`
  (filter `ability_select`), then `nextest run -p cimmeria-cell npc_ai`.
- **Docs:** `docs/gameplay/npc-ai.md` gets one subsection "Combat profile and
  the Pistol Shot placeholder": the resolution table from C1, the two
  diagnostics, and that D-NC1 is pending. Run R6.
- **Rules:** R1-R3, R5, R7, R8, R10.

### NC-11 NID Guard 24 reference tests (verify)

- **Agent:** packet-coder. **Host:** windows. **Depends:** NC-01 and NC-03 merged.
- **Branch:** `npc-combat-reference/NC-11-nid-reference`. **Worktree:** `nc-11`.
- **Commit:** `test(npc-ai): NID Guard reference pipeline at Z12 (NC-11)`
- **Files:** `crates/cell/src/cell/service/tests/npc_ai/debug_area_combat/reference/nid.rs`
  only.
- **Contract:** C1 (use `cimmeria_cell_world::cell::combat::combat_profile::*`),
  plus the C4 helpers.
- **Tests:**
  - no-DB:
    1. `nid_reference_carries_smg_evidence`: the `DebugArea_Ref_NID` row is
       template 24, its components include `WP-Human.WP_SMG_1A`, and under
       **both** policies `resolve_npc_combat_profile` gives `ExplicitSet`,
       `[559]`, `Component("WP-Human.WP_SMG_1A")`.
    2. `nid_reference_takes_cover`: `use_cover` resolves true.
  - live-DB (`mod live_db`):
    3. `live_db_reference_nid_smg_damages_a_player`: `fight_space` with the
       player at the origin, `set_pools(player, 1000, 1000)`, and template
       24 spawned at x = 10 with `spawn`. Run `connect_entity`, then up to 4
       `pass`es. Assert that the player's Health + Focus dropped, and that
       the NPC knows only 559.
    4. `live_db_reference_nid_loot_and_respawn`: the Ref NID row from
       `load_spawns_from_db` resolves loot table 2 and a 20 s respawn. Copy
       the shape of `live_db_debug_area_respawn_timers_resolve`.
- **Expect:** these pin current behaviour. A failure is a finding (see
  Dispatch rules), not a fix.
- **Lane:** fmt and clippy `-p cimmeria-cell`; `nextest run -p cimmeria-cell reference::nid`;
  `live-db-test.ps1 reference_nid`.
- **Docs:** none (tests only; NC-31 records the results).
- **Rules:** R1-R3, R5, R7, R10.

### NC-12 Damage payloads for effects 736 and 1717

- **Agent:** packet-coder. **Host:** windows. **Depends:** NC-03 merged;
  D-NC2 decided.
- **Branch:** `npc-combat-reference/NC-12-melee-payloads`. **Worktree:** `nc-12`.
- **Commit:** `fix(abilities): damage for Staff Melee AA and Mob Strike (NC-12)`
- **Files:** `db/resources/Effects/Seed/effects.sql` (rows 736 and 1717
  only); `db/resources/Effects/Seed/effect_nvps.sql`;
  `crates/cell/.../debug_area_combat/reference/jaffa.rs`;
  `docs/architecture/abilities-and-effects-system.md`.
- **Do:**
  1. In the effect 736 row (ability 710) and the effect 1717 row (ability
     1433), change the trailing `script_name` value from `NULL` to
     `'MeleePhysicalDamage'`. Change nothing else; the precedent is the
     effect 656 row (Strike 594, PR #493).
  2. In `effect_nvps.sql`, add four hand rows **outside** the
     `-- ability-mechanics generated ... begin/end` markers, after the
     460-462 rows, each with a C5 comment:
     - `(470, 736, 'HealthDamage', '10')`
     - `(471, 736, 'FocusDamage', '100')`
     - `(472, 1717, 'HealthDamage', '15')`
     - `(473, 1717, 'FocusDamage', '125')`
  3. Run `python tools/ability_mechanics/effect_nvps_from_desc.py --check`.
     It must exit 0 (the generator only covers player-reachable abilities).
  4. Side effect to note in the report: pet sets 351 and 352 carry set 4, so
     Jaffa pets gain melee damage. NC-24 pins it.
- **Tests** (all live-DB, in `reference/jaffa.rs` `mod live_db`):
  1. `live_db_reference_jaffa_melee_effects_carry_damage`:
     `load_effect_defs` gives 736 `MeleePhysicalDamage` with params
     HealthDamage 10 and FocusDamage 100, and 1717 the same with 15 and 125.
  2. `live_db_reference_jaffa_staff_swing_damages_in_reach`: take the 1380
     record from `load_spawn_templates`, set `ability_ids = vec![710]`, spawn
     it 2 m from the player (pools 1000/1000), and run up to 3 `pass`es.
     Assert that the player lost Health or Focus.
  - Revert proof: putting `NULL` back, or dropping the NVPs, makes 736 inert
    (`effect_inert`), so both tests fail.
- **Lane:** fmt and clippy `-p cimmeria-cell`; `nextest run -p cimmeria-cell reference::jaffa`;
  `live-db-test.ps1 reference_jaffa`; `live-db-test.ps1 effect` (the existing
  effect and NVP guards).
- **Docs:** `abilities-and-effects-system.md` gets one paragraph, under the
  section on reconstructed payloads or the #493 note, listing 736 and 1717 as
  RECONSTRUCTED from their own descriptions. Run R6.
- **Rules:** R1-R8, R10.

### NC-14 PRU 4 reference tests (verify)

- **Agent:** packet-coder. **Host:** windows. **Depends:** NC-01 and NC-03 merged.
- **Branch:** `npc-combat-reference/NC-14-pru-reference`. **Worktree:** `nc-14`.
- **Commit:** `test(npc-ai): Prisoner Retrieval Unit reference pipeline at Z12 (NC-14)`
- **Files:** `crates/cell/.../debug_area_combat/reference/pru.rs` only.
- **Tests:**
  - no-DB:
    1. `pru_reference_has_integrated_weapon`: the `DebugArea_Ref_PRU` row is
       template 4 with `ability_ids == [221]`, and the profile is
       `ExplicitSet` with `WeaponEvidence::None` under both policies. An
       explicit set needs no `WP-` evidence.
  - live-DB:
    2. `live_db_reference_pru_energy_shock_defs`: ability 221 from
       `load_ability_defs` is ranged, with effect 264; effect 264 has script
       `RangedEnergyDamage` and HealthDamage 16 / FocusDamage 80. Read the
       field names from `AbilityDef` and `EffectDef`.
    3. `live_db_reference_pru_shock_damages_a_player`: shaped like the NC-11
       damage test, with template 4 at x = 10.
- **Expect:** these pin current behaviour; a failure is a finding. The
  CellBlock scripted activation is NC-23's job, not this packet's.
- **Lane:** as NC-11, with filters `reference::pru` and `reference_pru`.
- **Docs:** none.
- **Rules:** R1-R3, R5, R7, R10.

### NC-15 Placeholder inventory

- **Agent:** packet-coder (no code change; one doc plus an optional helper
  script in the scratchpad, not committed). **Host:** windows. **Depends:** none.
- **Branch:** `npc-combat-reference/NC-15-placeholder-inventory`. **Worktree:** `nc-15`.
- **Commit:** `docs(npc-combat): Pistol Shot placeholder inventory (NC-15)`
- **Files:** new `docs/analysis/npc-combat-reference/placeholder-inventory.md`.
- **Do:** build three tables from the seeds and code. Use Python with `-I`
  for parsing.
  1. **Templates:** every `entity_templates*.sql` row with a NULL
     `ability_set_id`. Columns: template id; name; body family (from
     `body_set`: Human, Jaffa, Goa'uld, creature `MOB_*`, mechanical
     Drone/Walker, other); the `WP-*` component or none; faction; class or
     flags (interactive, vendor, trainer, scripted); the worlds and spawn
     counts (from `spawnlist*.sql`); and the verdict under
     `RequireWeaponEvidence` (keeps 592 / holds fire). Templates 34 and 35
     must appear.
  2. **Production callers of `SpaceManager::spawn_npc`**: grep `\.spawn_npc\(`
     under `crates/`, excluding `#[cfg(test)]` modules and `*tests*` files.
     For each, say whether it is production and what it spawns.
  3. **Tests that rely on the fallback:** every test that spawns an NPC with
     empty `ability_ids` or through `spawn_npc`, and then fires or asserts
     592 or `NPC_DEFAULT_ABILITY`. Columns: file, test name, crate, and a fix
     (put `592` in the fixture's `ability_ids` explicitly, or call
     `add_ability(592)`). Group by crate (cell-world, cell-combat, other) and
     give each group's line count: these become NC-21a, b and c. Flag any
     file under `crates/cell-combat/src/cell/service/npc_ai/ability_select*`,
     which NC-20 owns.
- **Lane:** none. Run R6.
- **Rules:** R1, R3, R6, R8, R9.

---

## Wave 2: ScavDog, selection, explicit fixtures, regressions (7 packets, parallel)

The file sets are disjoint:

- NC-20 owns `ability_select.rs` and its tests;
- NC-21a, b and c skip any file NC-20 owns, and list those files in their
  reports;
- NC-13, NC-20, NC-23 and NC-24 own separate `reference/` stubs.

Each packet ships as its own PR. Merge order: NC-21a, NC-21b, NC-21c, NC-13,
NC-20, NC-23, NC-24.

### NC-13 ScavDog gets Mob Strike (RECONSTRUCTED)

- **Agent:** packet-coder. **Host:** windows. **Depends:** NC-12 merged;
  D-NC5 decided. Prefer to run after NC-05: if NC-05 contradicts 1433, stop
  and return `blocked`.
- **Branch:** `npc-combat-reference/NC-13-scavdog-mob-strike`. **Worktree:** `nc-13`.
- **Commit:** `feat(npc-ai): ScavDog melee with Mob Strike (NC-13)`
- **Files:**
  - `db/resources/Abilities/Seed/ability_sets.sql`: add
    `(6, 'ScavDog: Mob Strike (RECONSTRUCTED, NC-13)')` after set 5;
  - `db/resources/Abilities/Seed/ability_set_abilities.sql`: add `(6, 1433)`;
  - `db/resources/Entities/Seed/entity_templates.sql`: in the template 76
    row, change `ability_set_id` from NULL to 6 and nothing else;
  - `crates/cell/.../debug_area_combat/reference/scavdog.rs`;
  - `debug_area_combat/reference/mod.rs` (only the 76 → [] assertion in
    `reference_profiles_today`, which becomes [1433]);
  - `docs/content/debug-area.md` (only the Z12 ScavDog row).

  Put a C5 comment above each new or changed row. Do not assign 983 Bite.
- **Tests:**
  - no-DB, in `scavdog.rs`:
    1. `scavdog_reference_is_melee_only`: the `DebugArea_Ref_ScavDog` row has
       `ability_ids == [1433]`; the profile is `ExplicitSet`; 592 is not in
       it.
  - live-DB:
    2. `live_db_reference_scavdog_bites_in_reach_only`: spawn template 76
       (pools 1000/1000) at 15 m. After 2 `pass`es the player is undamaged;
       the dog closes or holds, because melee reach is `NPC_MELEE_RANGE`.
       Then place it at 2 m; within 3 passes the player loses Health or
       Focus. Through the whole test, the dog's known abilities never include
       592.
    3. `live_db_reference_scavdog_set_6_loads`: `ability_set_abilities` for
       set 6 is exactly [1433].
  - Revert proof: with the template row's set back to NULL, the dog knows
    [592] and fires at 15 m, so tests 1 and 2 fail.
- **Lane:** fmt and clippy `-p cimmeria-cell`; `nextest run -p cimmeria-cell debug_area`;
  `live-db-test.ps1 reference_scavdog`; `live-db-test.ps1 debug_area`. The
  lineup guard is unaffected, because the look does not change.
- **Docs:** the Z12 row in `debug-area.md` (run R6). The registry row is
  NC-31's.
- **Rules:** R1-R8, R10.

### NC-20 NPC melee preference inside reach

- **Agent:** packet-coder. **Host:** windows. **Depends:** NC-03 merged;
  D-NC4 decided.
- **Branch:** `npc-combat-reference/NC-20-melee-preference`. **Worktree:** `nc-20`.
- **Commit:** `feat(npc-ai): NPCs swing melee when the target is in reach (NC-20)`
- **Files:**
  - `crates/cell-combat/src/cell/service/npc_ai/ability_select.rs`: only
    `choose_npc_ability_within_reach` and its doc comment;
  - that function's existing test module: find it with
    `Grep choose_npc_ability_within_reach` under `crates/cell-combat`, and
    list it in your report;
  - `crates/cell/.../debug_area_combat/reference/melee_preference.rs`;
  - `docs/gameplay/npc-ai.md` (only the ability-selection paragraph).
- **Do:** before today's `in_reach` walk, add a melee-first pass that applies
  only when the NPC is **not** a pet and
  `target_dist <= combat::NPC_MELEE_RANGE`:

  ```rust
  let is_pet = /* the same test super::pet::ability_allowed uses to recognise a pet; read pet.rs */;
  if !is_pet && target_dist <= combat::NPC_MELEE_RANGE {
      if let Some(id) = ability_ids.iter().copied().find(|&id| {
          !npc.abilities.is_on_cooldown(id)
              && super::pet::ability_allowed(space_mgr, npc, id)
              && space_mgr.ability_defs.get(&id).is_some_and(|d| !d.is_ranged)
              && effective_max_range(space_mgr.ability_defs.get(&id), npc_attack_range) >= target_dist
      }) { return Some(id); }
  }
  ```

  Otherwise behaviour is unchanged. Update the doc comment's "Effect on
  single-ability sets" and the set-4/set-5 paragraphs: a Jaffa at 2 m now
  swings 710, and a Goa'uld (set 5) at 2 m now swings 711.
- **Tests:**
  - unit, in the existing selector test module:
    1. A non-pet NPC with defs 584 (ranged) and 710 (melee) at 2 m picks 710.
    2. The same NPC at 15 m picks 584.
    3. At 2 m with 710 cooling, it picks 584.
    4. A pet (built the way the existing pet selector tests build one) at
       2 m still picks 584.
  - live-DB, in `melee_preference.rs`:
    5. `live_db_reference_jaffa_swings_staff_in_reach`: spawn 1380 at 2 m.
       After one `npc_ai_tick` plus `warmup_tick` (not `pass`, which clears
       the cooldowns), 710 is on cooldown and 584 is not.
  - Revert proof: without the melee-first pass, tests 1 and 5 pick 584.
- **Lane:** fmt and clippy `-p cimmeria-cell-combat`; `nextest run -p cimmeria-cell-combat npc_ai`;
  fmt and clippy `-p cimmeria-cell`; `nextest run -p cimmeria-cell npc_ai`;
  `live-db-test.ps1 reference_jaffa`.
- **Docs:** `npc-ai.md`, ability selection. Run R6.
- **Rules:** R1-R3, R5, R7, R10.

### NC-21a / NC-21b / NC-21c Make the 592 fixture explicit

- **Agent:** packet-coder (one per letter). **Host:** windows. **Depends:**
  NC-15 merged (its § Tests table is the file list).
- **Scope:**

  | Packet | Crate | Branch | Worktree | Commit |
  |---|---|---|---|---|
  | NC-21a | `cimmeria-cell-world` | `npc-combat-reference/NC-21a-explicit-592-cell-world` | `nc-21a` | `test(cell-world): explicit Pistol Shot fixtures (NC-21a)` |
  | NC-21b | `cimmeria-cell-combat` | `npc-combat-reference/NC-21b-explicit-592-cell-combat` | `nc-21b` | `test(cell-combat): explicit Pistol Shot fixtures (NC-21b)` |
  | NC-21c | every other crate | `npc-combat-reference/NC-21c-explicit-592-other` | `nc-21c` | `test: explicit Pistol Shot fixtures, remaining crates (NC-21c)` |

  If one group's table has more than about 15 files, the orchestrator asks
  the architect to split it before dispatch.
- **Files:** exactly the test files in the packet's group of
  `placeholder-inventory.md` § Tests, except files NC-20 owns.
- **Do:** make each listed test independent of the fallback, with no
  production change:
  - where a fixture record has empty `ability_ids` and the test relies on
    592, set `ability_ids: vec![592]`;
  - where a test uses `spawn_npc` and relies on 592, leave it. `spawn_npc`
    keeps adding 592 under both policies; record it as "unaffected".

  Do not change assertions.
- **Test:** for each file you changed, the existing tests still pass. Add
  nothing new. The proof comes in NC-30: with the policy flipped, these tests
  still pass.
- **Lane:** fmt, clippy and `nextest run -p <crate>` for every crate you
  touched.
- **Docs:** none.
- **Rules:** R1-R3, R5, R10.

### NC-23 Regression: tutorial and CellBlock combat unchanged

- **Agent:** packet-coder. **Host:** windows. **Depends:** NC-10 and NC-12
  merged.
- **Branch:** `npc-combat-reference/NC-23-regression-tutorial`. **Worktree:** `nc-23`.
- **Commit:** `test(npc-ai): pin tutorial and CellBlock NPC combat profiles (NC-23)`
- **Files:** `crates/cell/.../debug_area_combat/reference/regression_tutorial.rs`
  only.
- **Tests** (live-DB, `mod live_db`):
  1. `live_db_regression_cellblock_profiles_unchanged`: every world-12 spawn
     (CellBlock) of templates 24 and 4 loads with `ability_ids` [559] and
     [221], and resolves `ExplicitSet` under both policies. Read the world
     name from the spawn rows; do not hardcode it.
  2. `live_db_regression_tutorial_no_set_mobs_keep_firing_under_legacy`:
     every world-12 spawn whose template has no set resolves
     `LegacyPlaceholder` [592] under `Legacy`. List each such template and
     its verdict under `RequireWeaponEvidence` in a test `eprintln!`, so
     NC-30 sees the impact.
  3. `live_db_regression_pru_cellblock_activation_rows_intact`: the PRU's
     scripted activation is untouched. Find its rows with
     `Grep "Prisoner Retrieval|PRU|template_id = 4"` under
     `crates/cell-content` and `db/resources`, then assert the content rows
     that drive it still load. If no such row exists, write that in the
     report and skip the test with a reason.
- **Expect:** these are pins; a failure is a finding.
- **Lane:** fmt and clippy `-p cimmeria-cell`; `live-db-test.ps1 regression_`.
- **Docs:** none.
- **Rules:** R1-R3, R5, R7, R10.

### NC-24 Regression: Harset Jaffa and Jaffa pets

- **Agent:** packet-coder. **Host:** windows. **Depends:** NC-12 merged.
- **Branch:** `npc-combat-reference/NC-24-regression-harset`. **Worktree:** `nc-24`.
- **Commit:** `test(npc-ai): pin Harset Jaffa and Jaffa pet combat (NC-24)`
- **Files:** `crates/cell/.../debug_area_combat/reference/regression_harset.rs`
  only.
- **Tests** (live-DB):
  1. `live_db_regression_harset_jaffa_profiles`: the spawns of 159 and 160
     in worlds 57 and 8 load set 4 [584, 710], faction 1 and no `WP-`
     component (D-NC3 is not yet applied; NC-22 updates this assertion).
  2. `live_db_regression_jaffa_pet_sets_carry_staff`: sets 351 and 352
     contain 584 and 710, and effect 736 now has its damage params. This is
     the intended NC-12 side effect, pinned.
  3. Pet selection is unchanged at 2 m. This is covered in NC-20 test 4; do
     not duplicate it.
- **Lane:** fmt and clippy `-p cimmeria-cell`; `live-db-test.ps1 regression_`.
- **Docs:** none.
- **Rules:** R1-R3, R5, R7, R10.
