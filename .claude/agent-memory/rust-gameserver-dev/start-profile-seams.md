---
name: start-profile-seams
description: Class Start v6 CS-02 start profiles - where start world/kit lives, the process registry, EnterableWorlds, legacy_kit vs provenance starters, debug_kit, and the test traps (Acquire HRTB, registry installs, enterable worlds)
metadata:
  type: project
---

Learned implementing CS-02 (2026-10-05).

- **One source:** `resources.char_creation` (+ `_abilities` with `source_kind`,
  `_items`, `char_creation_debug_kit_*`). `cimmeria_resources::base::start_profiles`
  loads it (`load_all(&mut PgConnection)`), validates (`problems()`), and holds a
  process registry (`install` / `installed`) filled by `load_at_boot` in both base
  and cell. `chardef.rs` keeps identity only; a live-DB test pins it to the table.
- **`load_all` takes `&mut PgConnection`, not a generic `Acquire`:** the generic
  future broke `Send` ("not general enough") on the base's spawned cell-message
  task, reached through `gm_ability_bulk`.
- **"Starters" now means `source_kind = 'legacy_kit'`** (no provenance row, no
  credit): `gm_ability_bulk::starter_abilities`, `player_init_row` credit filter,
  content-grant starter skip. Only the Goa'uld/Asgard holding states have them, so
  tests that need a Soldier starter insert a temporary legacy row for char_def 1.
  The GM reset set is `StartProfiles::reset_abilities(archetype, sgw_player.debug_kit)`.
- **Creation fail-closed needs `EnterableWorlds`** (new `CellToBaseMsg`, sent at
  cell startup): instanced worlds (Castle_CellBlock, SGC_W1) have no SpaceData.
  Creation tests must call `live_db_tests::register_start_worlds(pool)`.
- **`resolve_space_id_fallback` no longer defaults to Castle_CellBlock:** it uses a
  registered space id, then the three fixed ids, else None + ERROR. A no-cell test
  travelling to another world must `register_space` it first.
- **Sync consumers read the registry; tests install `fixture::seeded()`**
  (`test-support` feature on cimmeria-resources). `fixture_is_the_seed_live_db`
  keeps the fixture equal to the seed. Use `*_in(profiles, ..)` variants for
  no-profile cases instead of leaving the global empty (cargo test shares it).
- Dakara_E1 plaza spawn (100, -17.4, 230) is on dakara_e1.nav component 279
  (gate + DHD); control point (-152, -20.55, 300) is on component 473.

Related: [[ability-grant-provenance-seams]], [[navmesh-onmesh-assertions-are-weak]].
