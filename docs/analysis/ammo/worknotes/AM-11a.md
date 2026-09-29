# AM-11a Worknotes

> Type: reference. Audience: the ammo coordinator and the AM-11b / AM-11c workers.
> Companions: [README.md](../README.md), [work-packets.md](../work-packets.md), [AM-04 worknotes](AM-04.md), [ADR § 31](../../../architecture/abilities-and-effects-system.md#31-special-ammo-modifies-the-shot-directly-from-resourcesammo_modifiers-ammo-campaign-am-04-d-am07).

## Contract

- **Packet:** AM-11a, the crowd-control darts: `Dart_Poison`, `Dart_Disease`, `Dart_Tranquilizer`.
- **Also owned by this packet, for all darts:** the dart-weapon widening and the dart rows of the debug-hub crate (the D-AM06 extension). AM-11b and AM-11c must not touch them.
- **Decisions in force:** D-AM06, D-AM07.
- **Id block:** effects, nvps and loot ids 9140-9150.

## Dart type to toggle ability

This mapping is for all ten dart types. AM-11b and AM-11c use it. Source: `db/resources/Abilities/Seed/abilities.sql`. Every dart toggle carries moniker 488944709 (`Scientist_Medical`).

| Ammo type | Toggle ability | Evidence | Confidence |
|---|---|---|---|
| `Dart_Poison` | **990** Dart Type: Hazardous: Poison | Exact name. Its `effect_ids` is empty. Mission copy 3214 (`MS021_081024_DartType:Hazardous:Poison`) has only a damage template, effect 4679 | High |
| `Dart_Disease` | **991** Dart Type: Hazardous: Disease | Exact name. **2876** is a duplicate row with the same name and text, and 3430 is the mission copy (effects 5166-5168: "Add BC" and "Add DOT BC", pulse 5 s, no NVPs) | High |
| `Dart_Tranquilizer` | **998** Dart Type: Hazardous: Disorient | No ability is named Tranquilizer. The only in-game text for it is `DN_Ob_Ms_Tollana_BelethsOtherLackey_DartGun` = "Tranquilizer Dart Gun". Disorient is the closest hazardous toggle to a sedative. RECONSTRUCTION | Low |
| `Dart_EMP` | **999** Dart Type: Hazardous: EMP | Exact name. Mission copy 3431, effects 5169-5171 | High |
| `Dart_Radioactive` | none | No dart toggle names it. "Radioactive Soil - Gathering" (2307) is unrelated. Candidates if AM-11b wants one: 1227 Contagion or 1215 Interruption, both of which have no enum type of their own | None |
| `Dart_Stim` | **992** Dart Type: Beneficial: Stim | Exact name. Effect 5008 "Direct Heal: +10% Focus". Mission copy 3428 | High |
| `Dart_Coagulant` | none in the player set | Only the mission copy 3427 (`MS021_081024_DartType:Beneficial:Coagulant`, effect 5161) | Medium (use 3427) |
| `Dart_Nanites` | none | No ability names it | None |
| `Dart_Antidote` | **1228** Dart Type: Beneficial: Antidote | Exact name. Effects 5057 / 5005 / 5004 remove `EFFECT_Contagion`, `EFFECT_Poison` (50%) and `EFFECT_Disease` (33%). **2874** is the second Antidote: effects 5007 / 5006 remove `EFFECT_Wound` and `EFFECT_Burning` (50%) | High |
| `Dart_Adrenaline` | **1220** Dart Type: Beneficial: Adrenaline | Exact name. `effect_ids` empty | High |

The four abilities left over from the brief's list, none with an `EAmmoType` of its own:

- **1215** Hazardous: Interruption: "Damage Type: Physical / Interruption: Increased".
- **1226** Hazardous: Confusion: no text.
- **1227** Hazardous: Contagion: no text. It is a separate effect class from Disease, since Antidote 1228 removes `EFFECT_Contagion` on its own.
- **2876**: the duplicate Disease row described above.

**The toggle text is boilerplate.** "Damage Type: Physical / Penetration: Decreased / Damage: Increased" appears on 990, 991, 998 and 999, and also on 992 Stim, which is a heal dart. It says nothing about any one dart type, so AM-11a leaves `damage_mult` and `penetration_mult` at 1.0 and puts each dart's value into its on-hit effect.

## What shipped

| Piece | Where |
|---|---|
| Three `ammo_modifiers` rows, on-hit effects 9140-9142, and nvps 9140-9144 | `db/resources/Abilities/Seed/ammo_modifiers_dart_cc.sql` |
| `MovementSlow`, the Tranquilizer slow | `crates/cell-world/src/cell/effects/ammo_dart_cc.rs`, one arm in `registry.rs` |
| The dart widening: 19 guns take all 10 dart specials | `db/resources/Items/Seed/ammo_dart_widening.sql` |
| Debug-hub crate: gun 3584 plus a 500-round stack each of 9005-9014 | `db/resources/Loot/Seed/ammo_dart_loot.sql` (loot ids 9140-9150) |
| Pipeline tests | `crates/cell-combat/src/cell/abilities/damage_apply/ammo_dart_cc_tests.rs` |
| The ADR's family rows | `docs/architecture/abilities-and-effects-system.md` § 31 |

## Rows (RECONSTRUCTION)

| Type | Row | On-hit effect | Script | NVPs |
|---|---|---|---|---|
| `Dart_Poison` | 1.0 / 1.0, `DT_Physical`, toggle 990 | 9140: 5 pulses, 2 s apart | `Suppression` | `HealthDamage` 4, `EffectCategory` Poison |
| `Dart_Disease` | 1.0 / 1.0, `DT_Physical`, toggle 991 | 9141: 10 pulses, 2 s apart | `Suppression` | `HealthDamage` 2, `EffectCategory` Disease |
| `Dart_Tranquilizer` | 1.0 / 1.0, `DT_Physical`, toggle 998 | 9142: 4 pulses, 2 s apart | `MovementSlow` | `SpeedReduction` 40 |

- **Poison** does 20 Health over 8 s: 4 on the hit plus 4 on each of the 4 remaining pulses.
- **Disease** does the same 20 over 18 s.
- **Tranquilizer** drops the target to 60% movement speed for 6 s.

The dart auto attack (1086, effect 1237) does -100 Focus / -10 Health, so either DoT is worth about two extra shots of Health.

The `EffectCategory` nvps on 9140 and 9141 are a contract with AM-11c: its Antidote `RemoveEffects` script matches them. The seed guard asserts both.

## Changes from the plan

1. **Tranquilizer is a slow, not a stun, and it needs a new script.** `Stun` sets `BSF_MOVEMENT_LOCK`, which has two faults. The NPC AI never reads that bit. And when the effect pulses, `on_apply` runs on every pulse and on every same-shooter refresh, but `on_remove` runs once, so the ref-counted flag never clears. Both faults are filed as **#1049**, not fixed here. The NPC movement tick and the client both scale speed by `MOVEMENT_SPEED_MOD`, so `MovementSlow` lowers that stat and puts it back when the effect ends. It is guarded to one slow per effect per target:
   - `on_apply` does nothing if an instance of the effect is already on the target (a pulse or a refresh).
   - `on_remove` restores the speed only when no instance of the effect is left. Every removal path drops the instance before it calls `on_remove`.
2. **Poison and Disease reuse `Suppression`,** the existing per-pulse Health chip, with no new script. **Watch:** Suppression's doc says a movement-speed half may be added to it later. If that happens, Poison and Disease darts would start to slow too, and should then move to a script of their own.
3. **One extra shared line:** `#[cfg(test)] mod ammo_dart_cc_tests;` in `damage_apply/mod.rs`. The coordinator approved it.
4. **Loot ids 9140-9150** match the AM-11a block. This keeps them clear of AM-05's `ammo_loot.sql`. The coordinator approved it.

## Known limits

- **Suppression writes Health directly.** The DoT ignores armour and absorb pools. Mitigation is capped at 0 anyway, and penetration is 1.0 on every row here, so the cap changes nothing for this family.
- **Speed restore arithmetic.** `MovementSlow` restores by adding back the reduction, clamped to the stat's range. Suppose something else lowered `MOVEMENT_SPEED_MOD` below 40 while the dart slow was on (a GM `.speed`, for example). The restore can then leave the target faster than it was before the dart hit.
- **Logout while slowed.** A slowed player who logs out before the slow expires drops the instance (per-session state) without an `on_remove`. Whether the slowed stat persists depends on whether `MOVEMENT_SPEED_MOD` is saved. It is not checked here.
- **The slow is invisible on the target's client when the target is an NPC.** The NPC simply moves slower, since the server steps its path.

## UAT risk: unknown effect ids reach the client

`register_active_effect` sends `onTimerUpdate(TIMER_DURATION_EFFECT)` for the effect's id, 9140, 9141 or 9142. It sends it again with a zero duration when the effect expires.

- For an NPC target, `send_entity_method` fans the message out to **every witness** of the NPC.
- For a player target (a duel), it goes to that player.

None of these effect ids exists in the client's cooked data. Nobody has checked what the client does with a duration timer for an unknown effect id. The client's `EffectSet` handler may look the id up and draw nothing, or it may fault, as the unknown Dialog overrides did in #938. **The first live UAT step for AM-11a:** shoot an NPC with Poison darts while a second client watches, and confirm that neither client crashes and what, if anything, is drawn.

If the client does fault, the fix is not in this packet. Choose one of these:

- stop the effect-duration timer for effects the client does not know (the pulse layer), or
- reuse existing cooked effect ids (for example 5164 / 5167 "Add DOT BC").

## Tests

| Test | Type | Proves |
|---|---|---|
| `ammo_dart_cc::tests::slow_applies_once_per_instance_and_restores_on_expiry` | unit | The hit slows. Pulses and refreshes do not slow again. Expiry restores exactly the starting speed |
| `ammo_dart_cc::tests::two_shooters_share_one_slow_until_the_last_instance_goes` | unit | A second shooter does not stack. The speed comes back only with the last instance |
| `ammo_dart_cc::tests::missing_or_zero_reduction_changes_nothing`, `missing_target_is_a_no_op`, `registry_resolves_movement_slow` | unit | Edge cases and the registry arm |
| `ammo_dart_cc::tests::live_db_dart_cc_seed_rows` | live-DB seed guard | Through the startup loaders: the three rows, the effects' scripts, pulses and nvps (including `EffectCategory`), the provenance ability names, the 19 widened guns (each holds all 10 specials, no duplicates), and table 3 (gun and 10 stacks at probability 1, never over the stack cap, #1045) |
| `damage_apply::ammo_dart_cc_tests::poison_dart_chips_on_the_hit_and_on_each_pulse` | pipeline smoke | A Poison shot takes exactly 4 more than a default-dart shot on the same roll, registers a 4-pulse DoT, and one pulse chips 4 more |
| `damage_apply::ammo_dart_cc_tests::disease_dart_is_a_weaker_longer_dot` | pipeline smoke | 2 on the hit, 9 pulses left, 2 per pulse |
| `damage_apply::ammo_dart_cc_tests::tranquilizer_dart_slows_until_the_effect_expires` | pipeline smoke | A shot slows the NPC to 60, a second hit and two pulses leave it at 60, and the third pulse expires it back to 100 |
| `damage_apply::ammo_dart_cc_tests::default_darts_run_no_on_hit_effect` | pipeline smoke | Default darts register nothing and slow nothing |

**Revert proof.** Each change was reverted on its own, and the named tests failed:

| Change reverted | Tests that failed |
|---|---|
| The `on_apply` guard in `MovementSlow` | both slow unit tests, and the Tranquilizer pipeline test |
| The `MovementSlow` registry arm | the Tranquilizer pipeline test |
| The `\ir` line of `ammo_modifiers_dart_cc.sql`, reloading the worktree database | `live_db_dart_cc_seed_rows` |
| The `\ir` line of `ammo_dart_widening.sql`, the same way | `live_db_dart_cc_seed_rows` |
| The `\ir` line of `ammo_dart_loot.sql`, the same way | `live_db_dart_cc_seed_rows` |

All pass once the change is restored. The existing `ammo`, `loot`, `debug_hub` and `effect` live-DB tests (287) pass against the new seed.
