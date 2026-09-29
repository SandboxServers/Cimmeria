---
name: pulsing-script-reapply-and-npc-cc
description: A pulsing effect's script on_apply runs on the hit, on every pulse and on every same-source refresh, but on_remove runs once; stateful scripts need an instance guard. NPC AI ignores BSF_MOVEMENT_LOCK; slows go through MOVEMENT_SPEED_MOD.
metadata:
  type: project
---

Found 2026-09-28 while building AM-11a (ammo campaign, dart Tranquilizer). The Stun bug is filed as #1049.

- **on_apply runs many times per instance.** `damage_apply` dispatches the script on the hit, before `register_active_effect`. `pulsing/tick.rs::fire_pulse` re-dispatches it on every pulse, and a same-source re-hit refreshes the instance and still dispatches it. `on_remove` runs once, at the sweep, duel strip or channel cancel, and every one of those paths drops the instance *before* calling it.
- **So a stateful script (a flag, a stat delta) must guard itself.** `on_apply` skips when `target.active_effects` already holds an instance of the same `effect_id`. `on_remove` restores only when none is left. The pattern is `MovementSlow` in `crates/cell-world/src/cell/effects/ammo_dart_cc.rs`. `Stun` has no such guard and leaks the `BSF_MOVEMENT_LOCK` refcount.
- **A `pulse_count = 1` effect never registers an instance,** so its `on_remove` never runs (see the `StatBuff` module doc).
- **NPC AI never reads `BSF_MOVEMENT_LOCK`.** A "stun" does nothing to an NPC. The NPC movement tick scales speed by `MOVEMENT_SPEED_MOD` (`effective_move_speed`, 100 = normal), and the client applies the same factor to players. So a slow that works on both is a `MOVEMENT_SPEED_MOD` delta.
- **Pulse-layer `onTimerUpdate` carries the effect id to the client,** to the target player or to every witness of an NPC target. A new effect id (outside the cooked data) reaches clients this way; flag it for UAT (compare #938).

Related: [[effect-scripts-run-after-the-death-check]], [[owner-pet-effects-and-passives]].
