---
name: combat-debug-notes-and-flush-points
description: AB-N1 combat debug: notes go to SpaceManager.combat_debug beside the AB-T3 rows, flush only where a cast scope closes; damage_apply/mod.rs sits near the 700-line cap; pin damage by reading health
metadata:
  type: project
---

AB-N1 (2026-10-04) put the in-game combat debug in `cimmeria-cell-world`
`cell::combat_debug` (`SpaceManager::combat_debug`), not on `CellEntity`.

- **Adding a decision to the debug lines**: call
  `space_mgr.combat_debug.note(caster, cast_id, ability_id, Note::...)` at
  the same point as the AB-T3 row. It is a no-op while no watcher exists.
  Sites that only hold `&SpaceManager` or a borrowed `&mut StatList`
  (`roll_hit`, `apply_nvp_damage`, `PlannedEffect::log`) cannot note
  directly: the hit collects into `damage_apply::debug_notes::HitDebug`
  and notes once `space_mgr` is free again.
- **Lines only leave at a flush** (`combat_debug::flush`), which skips the
  record of the scope still open. Flush sites: `handle.rs` after
  `exit_cast_scope`, `warmup/tick.rs` `fire_due_cast`, `pulse.rs`
  `pulse_one`, both ground-cast functions after `apply_secondaries`. A new
  cast entry point with its own scope needs its own flush or its lines wait
  for the next cast in the cell.
- **`damage_apply/mod.rs` is at ~689 lines** after AB-N1 extracted
  `pulse_registration.rs`; the next addition there needs another seam.
- **Tests:** `warmup_mgr`'s INSTANT_ABILITY (effect 500, HealthDamage 5)
  rolls a seeded QR, so the damage is 3 not 5. Read the NPC's health and
  build the expected text from it rather than pinning the number.

Related: [[timed-effect-ledger-and-stat-routing]], [[damage-apply-miss-gate-and-seeded-rolls]].
