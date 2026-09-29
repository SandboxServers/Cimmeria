---
name: effect-script-registry-seam
description: Since #962 step 4 (2026-09-28) effect scripts live in cimmeria-cell-effect-scripts and are looked up on SpaceManager; a bare test manager has NO scripts; seed has 2 known unscripted rows
metadata:
  type: project
---

Effect scripts moved out of `cimmeria-cell-world` into the leaf `cimmeria-cell-effect-scripts`
(#962 step 4, plugin-architecture.md §4.4, effects ADR decision 33).

- `dispatch_by_name` / `dispatch_on_remove` read `ctx.space_mgr.effect_scripts()`. The registry
  (`EffectScripts`) is built by `services::plugins::effect_scripts()` from the leaf's
  `EFFECT_SCRIPTS` table and installed by `CellService::start` before any spawn.
- **Trap:** `SpaceManager::new` starts with an EMPTY registry. A test that dispatches a script
  must call `registry::install(&mut mgr)` (`test_support::install_effect_scripts` in combat,
  content, cell). A "script didn't run" test failure is usually a missing install, not a bug.
- Tests in `cell-world` cannot install the leaf (cycle): script tests belong in the leaf.
- Still in `cell-world` (called from below the leaf): `passives.rs`, `pet_scripts.rs` name
  predicates, the stat-buff ledger, `ammo_damage.rs` / `ammo_explosive.rs` shot helpers.
- The seed has exactly two effect rows whose `script_name` no script answers: 658 `Reload`
  and 2907 `""`. Pinned by the leaf's live-DB `every_seeded_script_name_is_registered`.

**Why:** a one-script edit went from rebuilding 14 crates (~103k lines) to 5 (~10k).
**How to apply:** when advising a new script, say "impl in the family module of
cell-effect-scripts + one EFFECT_SCRIPTS row + seed script_name"; never "match arm in registry.rs".
