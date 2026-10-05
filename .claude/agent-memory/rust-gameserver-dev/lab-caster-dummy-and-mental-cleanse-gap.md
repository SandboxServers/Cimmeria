---
name: lab-caster-dummy-and-mental-cleanse-gap
description: .dummy caster drives handle_use_ability from the 1 Hz lab sweep (no AI turn); zero cooldown is charged 0.5 s real time; no seeded Mental effect holds, so Clear: Mind (ability 2099, effect 2827) removes nothing.
metadata:
  type: project
---

`.dummy caster <abilityId> [intervalSecs]` (2026-10-04, `cell-console`
`console/abilities/dummy_caster.rs`) is a plain lab dummy plus a `LabCaster`
extension. It stays out of `ai_driven_npc_entity_ids`; `lab_dummy_tick` calls
`dummy_caster::cast_due(now, ..)`, which launches the ability at the owner via
`handle_use_ability`. An NPC launch needs the ability in the caster's
`abilities` (`has_ability`), so placement calls `add_ability`.

**Test traps.**
- Cooldowns run on the real clock and a zero `cooldown` is charged **0.5 s**
  (`handle.rs`), so a second launch in a test with a stepped `now` is refused
  `OnCooldown`; clear it with `abilities.clear_ability_cooldown`.
- An NPC `onSequence` goes out once per witness (`WitnessEntityMethod`), so
  filter on `witness_id` or counts double.
- The stun interrupt (AT-10) is reachable from a console test with
  `request_interrupt(InterruptRequest { cause: Incapacitated, .. })` +
  `cimmeria_cell_combat::cell::effects::interrupt::resolve_interrupts_for`.

**Seed fact (AB-U22).** Every `EffectCategory = Mental` effect is either
unscripted (Suppression/Disorient/Fear/Confuse CC, mechanics UNKNOWN) or the
one-shot 1475 Suppression chip, so nothing Mental is ever held on a target and
Clear: Mind (ability 2099, effect 2827) removes 0. Disabling Shot (1354: 4 s warmup, two Health
`TimedStat`s 4333/4335) + Absolution (4169 `Health:2`) is the working cleanse
pair; pinned by `ab_l2_dummy_caster_uat_picks_live_db`. See
[[absorb-shield-ledger-and-cleanse-categories]].
