---
name: held-toggles-and-stance-moniker
description: AB-08 held ledger entries (toggles, passives), the EFFECT_Stance effect moniker (CRC-32, carried by an NVP), the anchor-effect toggle switch, and the held icon horizon
metadata:
  type: project
---

Since AB-08 (2026-10-03) `TimedStat` holds `pulse_duration = 0` entries
(`cell-effect-scripts` `stat_buff/held.rs`) for two kinds only: a toggle
(`AF_TOGGLED` ability) and a passive (`EF_AlwaysPersist`). Held entries land
only when target == invoker (`held_not_self`), or a forged cast would leave
a stat nobody can press off.

- **Toggle switch = the ability's LAST held `TimedStat` effect** in
  `effect_ids` order. Scripts run one effect at a time, so a per-ability
  "any entry on?" check flips mid-press and the second effect undoes the
  first. Reading the last effect's entry is stable for the whole press and
  converges after a partial removal. Relies on `fire_beneficial` iterating
  `def.effect_ids` in order.
- **Moniker ids are CRC-32 (zlib) of the name** (`Soldier_Command` =
  3212632871). `EFFECT_Stance` = 3785086315; no ability carries it. The seed
  has no effect monikers, so the generator writes `EffectMoniker` on stance
  effects and `RemoveMoniker` on "Remove ... EFFECT_Stance" halves
  (`RemoveByMoniker`). Never remove by an ability moniker (1470900795).
- `ability_is_beneficial` must skip `RemoveByMoniker` (flags 0), or every
  stance goes down the hostile path. The generator's `does_something` skips
  it too, or a second generator run stops being idempotent.
- Held toggle icon: start timer with `HELD_ICON_SECS` (86 400) as TotalTime
  and time left; passives send none. Evidence: `Effect.lua` divides by
  TotalTime; no toggled state in `ActionButtons.lua`.
- Passive seams send dirty stats (`base_messages/passive_sync.rs`): the
  base's world-entry burst predates `InitPlayerState`.
- Generator `--report` also rewrites the seed files, like a plain run.

Related: [[timed-effect-ledger-and-stat-routing]].
