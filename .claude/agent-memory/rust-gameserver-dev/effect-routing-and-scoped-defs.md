---
name: effect-routing-and-scoped-defs
description: AB-07 per-effect routing (abilities/effect_routing/): where user/area halves land, why they land after the target part, ground/splash scoped defs, and the generator's routing.py mirror
metadata:
  type: project
---

Since AB-07 (2026-10-03) every cast is split per effect by
`cell-combat/src/cell/abilities/effect_routing/` (`route_effect`, `plan_cast`,
ADR decision 35). User halves (`EF_ResolveOnAbilityUser`, or single effects of
a Self ability with **no** area effect) land on the caster; a beneficial
`TCM_AERadius` effect of a player's non-ground cast fans out to allies; damage
never routes onto the user. `damage_apply` only ever sees `RoutedCast::target_def`.

**Traps that cost time:**

- Off-target halves land **after** the target part in `fire.rs`. Landing them
  first let a user `+Accuracy` buff change the same cast's QR, so a test that
  pre-picked a missing `effect_sequence_id` silently hit.
- A Self ability with an area effect (Whirlwind, Bewilderment) keeps its single
  effects on the target: they are follow-ups of the area hit (flag 64), not
  self-effects. Don't widen rule 2.
- Ground secondaries take `secondary_scope` (target part minus single-target
  damage); seed authors many area debuffs on `TCM_Single` rows with "Radius AE"
  text (Flashbang 937), so "area effects only" would have dropped them.
- `tools/ability_mechanics/routing.py` mirrors `route_effect` for the generator
  scope rules; change both together. Seed pins that moved with it:
  `HAS_MECHANICS_TODAY`, the heal/stat `seed_live_db_tests` unbound lists.

Related: [[beneficial-cast-resolution-and-abilitydef-fields]],
[[timed-effect-ledger-and-stat-routing]], [[damage-apply-miss-gate-and-seeded-rolls]].
