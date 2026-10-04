---
name: pulsing-script-reapply-and-npc-cc
description: A pulsing script's on_apply runs per hit/pulse/refresh but on_remove once; stateful scripts belong on the timed-effect ledger (state_flags payload since AB-09). Interrupts from scripts queue on SpaceManager for combat to resolve.
metadata:
  type: project
---

Found 2026-09-28 (AM-11a, #1049); fixed by AB-09 on 2026-10-03.

- **on_apply runs many times per instance.** `damage_apply` dispatches the script on the hit, before `register_active_effect`; `pulsing/tick.rs::fire_pulse` re-dispatches it every pulse; a same-source re-hit refreshes and still dispatches. `on_remove` runs once, after the instance is dropped. A `pulse_count = 1` effect never registers an instance, so its `on_remove` never runs.
- **The fix pattern is the timed-effect ledger, not an instance guard.** `apply_timed_effect` is keyed `(effect, invoker)`: a re-run replaces its own entry (release, then retake), so re-apply is idempotent and expiry is the stat-buff tick's. Stat deltas: `stats`; `state_field` bits: `TimedEffectSpec::state_flags` (one counted ref per entry, `entity/cell_entity/stat_buff_flags.rs`). `Stun`/`Knockdown` (`cell-effect-scripts/.../crowd_control.rs`) and `MovementSlow` (`TimedStacking::PerEffect`) use it. `flush_stat_buff_timers` sends the owed `onStateFieldUpdate` (self + witnesses) only when the value changed.
- **`clear_all_state_flags` forfeits ledger holds** (zeroes entries' `state_flags`); keep that if you add another hard reset.
- **NPCs honour `BSF_MOVEMENT_LOCK` since AB-09:** `npc_ai_fight` holds (`decision_outcome = stunned`, before the leash check) and `npc_movement_tick` zeroes velocity and keeps the route. The AoI tick resends velocity every 100 ms, so a frozen NPC must have zero velocity or it runs in place.
- **Scripts cannot reach async combat:** to interrupt, a script calls `crowd_control::queue_interrupt` (`SpaceManager::pending_interrupts`); `cell-combat effects::interrupt` resolves it inside `flush_stat_buff_timers` (same burst) and on the stat-buff tick. Use the same queue-then-flush shape for any other script-triggered async action.
- **Test trap:** effect-script tests and `install_effect_scripts` share one registry; a fixture effect whose QR roll can miss lands nothing since AB-06, so CC pipeline fixtures add `EF_DONT_USE_QR`.
- **Pulse-layer `onTimerUpdate` carries the effect id** to the target player or every witness of an NPC; the ledger's timers go to player targets only.

Related: [[timed-effect-ledger-and-stat-routing]], [[effect-scripts-run-after-the-death-check]], [[damage-apply-miss-gate-and-seeded-rolls]].
