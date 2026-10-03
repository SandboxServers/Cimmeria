# AM-11c Worknotes

> Type: reference. Audience: the ammo coordinator and reviewers.
> Companions: [README.md](../README.md), [work-packets.md](../work-packets.md), [AM-04 worknotes](AM-04.md), [ADR § 31](../../../architecture/abilities-and-effects-decisions-23-33.md#31-special-ammo-modifies-the-shot-directly-from-resourcesammo_modifiers-ammo-campaign-am-04-d-am07).

## Contract

- **Packet:** AM-11c, the support darts: Stim, Coagulant, Nanites, Antidote and Adrenaline. It is in Wave 2 and was gated on AM-04 (#1047, merged).
- **Decision in force:** D-AM07. The server applies the ammo row directly, and the toggle abilities only record where the numbers came from.
- **Reserved ids:** effects and nvps 9160-9169. This packet uses 9160-9163.

## What shipped

| Piece | Where |
|---|---|
| Four `ammo_modifiers` rows, four on-hit effects and four NVPs | `db/resources/Abilities/Seed/ammo_modifiers_dart_support.sql`, plus one `\ir` line in `db/database.sql` |
| The `RemoveEffects` script, `DART_SUPPORT_DAMAGE_MULT`, and the `EffectCategory` and `RemoveCategories` NVP names | `crates/cell-world/src/cell/effects/ammo_dart_support/mod.rs` |
| One `match` arm | `crates/cell-world/src/cell/effects/registry.rs` |
| Unit and live-DB tests | `crates/cell-world/src/cell/effects/ammo_dart_support/tests.rs` |
| A smoke test of the whole shot through `handle_use_ability` | `crates/cell-combat/tests/ammo_dart_support.rs` (a new integration-test file) |
| The family row, the `EffectCategory` convention and the targeting finding | ADR § 31 |

## How a beneficial dart fits the damage pipeline

**Can a player shoot an ally today? No.** `use_ability`'s #444 gate lets a player's single-target ability through only against a target that `combat::player_may_attack` allows. That is either a hostile-faction NPC that is not a pet, or the player's engaged duel opponent. The launch refuses an ally, a bystander and the player's own entity, and `damage_apply` refuses any other player again at apply time. A player cannot shoot themselves either, because `target_id > 0` runs the same gate.

So a support dart can only land on a hostile target. As the brief asks, the row applies the on-hit effect and zeroes the damage. A Stim dart on a hostile NPC deals no damage and restores that NPC's Focus. Hostility and targeting rules are unchanged, and the shot still generates threat like any other hit.

**How the damage is zeroed.** `ammo_modifiers_mults_positive_chk` requires `damage_mult > 0`, so the rows use 0.0001. `calculate_damage_penetrating` rounds `raw * bonus * (1 - resist) * (1 + qr) * scale`, so any shot under 5000 points of pre-armour damage lands as 0. Every seeded dart ability is far below that. The dart auto attacks (1086, 1090) have no `HealthDamage` NVP at all and already deal 0 today. Follow-up: relax the CHECK to `damage_mult >= 0` and seed 0.

**What a proper friendly-target path needs.** This is a follow-up and was not built:

1. A supportive marker the server can read. `AbilityDef` has none, and `target_type_id` only encodes self, target or ground. For ammo the marker belongs on the row, because the ability that fires is the ordinary weapon shot. It could be a `beneficial` column on `ammo_modifiers`, or a test derived from the row, such as "the on-hit effect is a heal or a cleanse".
2. The reverse of the #444 gate for such a shot: allow a friendly player or yourself, and refuse a hostile one. It has to run at launch, at the warmup re-check and at `damage_apply`'s apply-time player gate. All three call `player_may_attack` today.
3. A damage-free resolve branch that skips the QR damage, the threat and the duel clamp, and runs only the on-hit effect. A friendly shot must not put either player in combat.
4. A telemetry event for the friendly path (`ammo_support_applied`) that carries the correlators.

## Values and sources

| Ammo | Row | On-hit effect | Source |
|---|---|---|---|
| Dart_Stim | damage 0.0001, penetration 1.0, damage type kept; toggle 992 | 9160 `HealFocus`, `HealPercentage` 10 | SOURCE-BACKED: effect 5008 "Direct Heal: Target +10% Focus". 992's own text ("Penetration: Decreased / Damage: Increased") is a copy of Hollow Point 715's, so it is not used. |
| Dart_Antidote | same; toggle 1228 | 9161 `RemoveEffects`, `Poison,Disease,Contagion,Wound,Burning` | The categories are SOURCE-BACKED. They come from 1228 (5057 Contagion, 5005 Poison 50%, 5004 Disease 33%) and 2874 (5007 Wound, 5006 Burning 50%). The row can record only one toggle, so 2874 is named in the seed comment. RECONSTRUCTION: the 50% and 33% chances are not rolled. |
| Dart_Coagulant | same; toggle 3427 | 9162 `RemoveEffects`, `Wound` | RECONSTRUCTION. There is no toggle ability. MS021 template 3427 "DartType:Beneficial:Coagulant" has effect 5161 "Cure Wound Effect" and FX event set 1477. |
| Dart_Adrenaline | same; toggle 1220 | 9163 `HealHealth`, `HealPercentage` 10 | RECONSTRUCTION. 1220 is "Buff: Toggle", with no effects and no numbers. |
| Dart_Nanites | no row | none | NO EVIDENCE. There is no ability, effect, moniker or FX sequence for it anywhere in the seeds or the handoff pack. It fires as a plain dart. |

## Changes from the plan

1. **Adrenaline is a heal, not a `StatBuff`.** A `StatBuff` sends `onTimerUpdate` with its effect id, either to the target's client or to an NPC target's witnesses, and 9163 is not in the client's cooked data. Nobody has checked how the client handles an unknown effect id, and an unknown cooked id has crashed it before (#938). Instead, all four on-hit effects are server-only: they change stats, which are flushed as `onStatUpdate`, and they send no per-effect message. Adrenaline can become a `StatBuff` once we know how the client handles an unknown effect id, or once the effect is pushed as cooked data.
2. **A new script, `RemoveEffects`.** No remove-effect script existed, and there is no effect-category data. The originals keyed their cleanses on `EFFECT_*` monikers, which were never seeded, and `EffectDef` has no name field. The convention is now an `EffectCategory` NVP on any effect that can be removed (ADR § 31). The coordinator made it part of the contract for AM-08 (`Burning`) and AM-11a (`Poison`, `Disease`). No seeded effect carries the NVP yet, so today Antidote and Coagulant remove only the tagged Wave-2 DoTs.
3. **The file became a directory.** With its tests, `ammo_dart_support.rs` reached 607 lines, so it is split into `ammo_dart_support/mod.rs` and `tests.rs` under the file-size rule. `effects/mod.rs` is unchanged.
4. **The pipeline smoke test is an integration test** in `crates/cell-combat/tests/`, because this packet owns no file under `damage_apply`. It is a new file and needs no `mod` line.

## Tests

| Test | Type | Proves |
|---|---|---|
| `ammo_dart_support::tests::antidote_removes_one_effect_of_each_listed_category`, `coagulant_removes_only_wound`, `a_cleanse_without_categories_removes_nothing`, `remove_categories_parses_the_nvp` | unit | the cleanse removes one effect per category, oldest first, matching case-insensitively; untagged and unlisted effects survive |
| `a_removed_effect_runs_its_on_remove` | unit | a cleansed Stun releases its movement lock |
| `the_support_multiplier_rounds_any_seeded_shot_to_zero` | unit (modifier math) | 0.0001 is positive and rounds a 4999-point shot to 0 |
| `a_stim_dart_restores_ten_percent_focus`, `an_adrenaline_dart_restores_ten_percent_health`, `antidote_and_coagulant_darts_cleanse`, `a_nanites_dart_fires_unmodified` | unit (resolution chain) | `shot_ammo` -> `on_hit_effect_id` -> registry dispatch works for each seeded row |
| `live_db_dart_support_seed_rows` | live-DB seed guard | the four rows, the four effects with their scripts and NVPs, no Nanites row, and the names of the provenance abilities |
| `crates/cell-combat/tests/ammo_dart_support.rs::a_stim_dart_heals_focus_and_deals_no_damage` | smoke (whole shot) | through `handle_use_ability`, a 1000-damage shot with Stim loaded leaves the hostile NPC at full health and restores 10% of its Focus; the same shot with default darts does damage |
| `...::a_stim_dart_cannot_target_an_ally_player` | smoke | the #444 gate refuses an ally, and the same fixture commits once the ally is made hostile |

**Revert proof.** Each mutation failed its guard:

- Setting `DART_SUPPORT_DAMAGE_MULT` to 1.0 failed the rounding test and the smoke test ("a support dart deals no damage").
- Removing the `RemoveEffects` registry arm failed `antidote_and_coagulant_darts_cleanse`.
- Removing the `on_remove` dispatch failed `a_removed_effect_runs_its_on_remove`.
- Deleting the four dart rows from the test database failed `live_db_dart_support_seed_rows`.

With everything restored, all of them pass.

## Telemetry

`RemoveEffects` logs two events on target `abilities`:

- `effect_removed_by_cleanse` (INFO), one per removal, with `removed_effect_id`, `removed_invoker_id`, `category` and the account and player correlators.
- `remove_effects_skipped` (WARN), with `reason = no_categories`.

The shot itself logs AM-04's `ammo_damage_applied`. There is no new event on target `ammo`, so the telemetry catalog does not change.

## Known limits

- `MITIGATION` is capped at 0, so penetration does nothing in live play. This does not affect this family, because every row keeps penetration at 1.0.
- A cleansed effect's buff icon stays on the client until its own countdown ends. The script cannot send the zero `onTimerUpdate` that the pulse sweep sends.
- Support darts only help hostile targets until the friendly-target path above exists.

## UAT

This needs `ammo.finite_special` on and a dart weapon whose `ammo_types` include the support types (AM-11a widens them).

1. Load Stim darts and shoot a hostile NPC. Expect no damage number and no health loss, and the NPC's Focus rises by 10%. In SigNoz, look for `heal_focus` on target `abilities` and `ammo_damage_applied` with `on_hit_effect_id = 9160`.
2. Load Adrenaline darts and shoot a damaged hostile NPC. Expect its Health to rise by 10%.
3. Poison an NPC with AM-11a's Poison dart, then shoot it with an Antidote dart. Expect `effect_removed_by_cleanse` with `category = Poison`, and the DoT stops ticking. The debuff icon may stay until its timer runs out (a known limit).
4. Try to shoot a friend with any support dart. Expect the launch to be refused (#444), as with any weapon.
5. **Client risk check:** these effects send no per-effect client message, so the client should never receive an unknown 91xx id. If the client crashes after a support-dart hit, capture its log and look for an `onTimerUpdate` carrying 9160-9163.
