---
name: damage-seams-and-gm-index-pins
description: Health/Focus damage has exactly two entity-level seams (apply_hit, fire_pulse) because effect scripts write stats directly; implementing a GM index breaks two tests that pin it as "unimplemented"
metadata:
  type: project
---

**Damage seams (verified 2026-10-04, AB-N2 god mode).** Every ability or DoT
loss of Health/Focus passes one of two functions in `cimmeria-cell-combat`:
`abilities::damage_apply::apply_hit` (single target, AoE/cone secondaries,
splash, deployable pulses, damage scripts and after-hit scripts) and
`effects::pulsing::tick::fire_pulse` (NVP and scripted DoTs, invoker-gone
fallback). Below them there is no choke point: `cell-effect-scripts` scripts
call `stats.get_mut(HEALTH)` directly. A per-target damage rule (god mode,
duel clamp D-SS20, surrender floor) goes in both seams, after
`settle_absorb_shields` and before the death check / dirty flush.
`combat::god_mode::GodModeGuard` (arm before, restore after) is the pattern.
`damage_apply/mod.rs` sits near the 700-line cap: put new logic in a
submodule and call it.

**GM index pins.** `gm/tests/mod.rs::unimplemented_gm_index_returns_false`
and `cell/dispatch/gm_dispatch_tests.rs::gm_tail_unimplemented_index_falls_through_without_panic`
each hard-code an unimplemented SGWGmPlayer index (143 since AB-N2; it was
142). Implementing that index breaks both; move them to another
unimplemented index. Also add the new constant to
`implemented_indices_are_in_gm_tail`; `wire`'s `def_conformance` checks the
constant name against `SGWGmPlayer.def` automatically.

**Rewriting a known-ability set.** A weapon only tags an ability it added;
one the row already held (579 known before equipping the pistol) is
untagged. Anything that strips abilities from the row (reset, respec-like
paths) must re-run the active slot's grant
(`bandolier::weapon_ability_set` + `swap_weapon_granted_abilities`) and
call `interrupt_unlearned_cast` for removed ids, mobs included. Base
helpers that run inside a row-locking `txn` must take `&mut *txn`, not
the pool (Copilot on #1170).

**Why:** all of these cost a test round trip during AB-N2.
**How to apply:** check these before wiring any new damage rule or GM index.
Related: [[ai-state-private-and-revert-proof-mtime]].
