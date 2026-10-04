---
name: timed-effect-ledger
description: AB-04 timed effect ledger — PerSource vs ReplaceSameStat stacking, order-independent baseline bounds, one icon per effect_id, what clears it, API gaps for AB-08/09/10
metadata:
  type: project
---

AB-04 (reviewed 2026-10-03) generalised `CellEntity::stat_buffs` into a `TimedEffect` ledger keyed `(effect_id, invoker_id)`.

- Stacking: `TimedStat` = PerSource (same caster refreshes to now+duration, other caster stacks); stimpack `StatBuff` = ReplaceSameStat (decision 28).
- Bounds: `StatBuffLedger::baselines` records a stat's own min/max when the first entry touches it; every apply/removal moves `cur` by the entry's delta and `refit_ledger_stat` widens the baseline just enough to hold `cur`; the last removal restores the baseline and clamps `cur`. Order-independent (a +50 and a -100 on 0/0/0 end at 0/0/0 either way). A baseline goes stale if another system changes the stat's max while entries are live (level-up on a buffed pool): re-check when a writer like that touches a ledger stat.
- Wire: one icon per `effect_id` on a target (the client keys icons by SecondaryId): start = latest live expiry, clear when the last entry of the effect goes.
- Clears: `EF_ClearOnDeath` (flag 4) at `resolve_death`; 20 of the 21 generated rows carry it (2752 Distraction, flags 64, does not). A dead target refuses a new clear-on-death entry (a lethal hit's debuff runs after the death strip). `InitPlayerState` resets the ledger on world entry. Entity removal drops it (no leak).
- NPC targets: `send_timer_update` drops non-player timers, so NPC debuffs have no icon by design.
- API gaps flagged for later packets: ~~no state-flag/refcount payload (AB-09a stun lock)~~ (AB-09 added `TimedEffectSpec::state_flags`, 2026-10-03), no mutable absorb amount (AB-10 shields), held entries send no start timer (AB-08 toggle icons unresolved), moniker 1470900795 is shared by most combat abilities so remove-by-moniker on it would strip nearly everything.

Related: [[effect-script-registry-seam]], [[qr-direction-and-cover]].
