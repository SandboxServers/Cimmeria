---
name: timed-effect-ledger
description: AB-04 (PR #1159) timed effect ledger — PerSource vs ReplaceSameStat stacking, widening bounds, what clears it, API gaps for AB-08/09/10
metadata:
  type: project
---

AB-04 (PR #1159, reviewed 2026-10-03) generalised `CellEntity::stat_buffs` into a `TimedEffect` ledger keyed `(effect_id, invoker_id)`.

- Stacking: `TimedStat` = PerSource (same caster refreshes to now+duration, other caster stacks); stimpack `StatBuff` = ReplaceSameStat (decision 28).
- Bounds widen instead of clamping (`shift_stat_widening`), revert is exact `unshift_stat`. Safe only while other writers of the same stat do not clamp against a widened bound. MOVEMENT_SPEED_MOD is 0/100/500 so the slow-dart `Stat::change` writer does not collide; re-check if a new clamping writer touches Accuracy/Defense/Response/Cover*.
- Clears: `EF_ClearOnDeath` (flag 4) at `resolve_death`; all 21 generated rows carry it. `InitPlayerState` resets the ledger on world entry. Entity removal drops it (no leak).
- NPC targets: `send_timer_update` drops non-player timers, so NPC debuffs have no icon by design.
- API gaps flagged for later packets: no state-flag/refcount payload (AB-09a stun lock), no mutable absorb amount (AB-10 shields), held entries send no start timer (AB-08 toggle icons unresolved), moniker 1470900795 is shared by most combat abilities so remove-by-moniker on it would strip nearly everything.

Related: [[effect-script-registry-seam]], [[qr-direction-and-cover]].
