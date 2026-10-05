---
name: absorb-shield-ledger-and-cleanse-categories
description: AB-10 — shields are ledger entries mirrored into absorb* stats and settled after each damage seam; cleanse categories come from co-sequenced resist rolls; shield/purge rows lack the beneficial bit
metadata:
  type: project
---

AB-10 (2026-10-03) design facts that are not obvious from one file:

- **Mirror + settle, not a new drain path.** `AbsorbShield` adds each pool to its `absorb*` stat; the pipeline (`combat/damage/absorb.rs`) keeps draining the stats. Any seam that can drain must then call `SpaceManager::settle_absorb_shields(target)` (damage_apply after NVP+script damage, `fire_pulse` before its flush). Forget it and the ledger pool stays full while the stat fell. Removal of an entry (any reason) releases `remaining` off the stat inside `remove_timed_effect_at`.
- **Shields absorb Focus too** (pipeline `stat_id == HEALTH || FOCUS`), a deliberate break from Python; damage scripts write pools directly, so `absorb_damage_nvps` rewrites their `FocusDamage`/`HealthDamage` first (Focus first).
- **Server `DT_*` (0-4) differ from the client's `EDamageType` (13-18).** `ShieldType` takes names (`Physical,Energy,Hazmat`) or server numbers; "14" is refused.
- **Categories:** every "<Kind> Resist Roll" shares `effect_sequence` with the effects it gates (79/79); alias.xml's `kineticRes/mentalRes/healthRes` "harmful … effects" make Mental/Kinetic/Health the client-defined classes. Generator tags lasting gated effects `EffectCategory`.
- **Routing trap:** 4306 Personal Shield (flags 342) and Absolution's purges (flags 0) have no `EF_Beneficial_Effect`, so a Self cast would take the hostile path. `ability_is_beneficial` now counts `AbsorbShield` and harmful `RemoveEffects` as beneficial on any type.
- **The cleanse script itself refuses** a harmful purge on a hostile and a buff strip on caster/ally (`relation.rs`, the support-dart classify rule rebuilt in cell-world terms).

Related: [[timed-effect-ledger-and-stat-routing]], [[effect-category-and-friendly-target-gaps]].
