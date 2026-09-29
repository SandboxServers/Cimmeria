# AM-11b Worknotes

> Type: reference. Audience: the ammo coordinator and the AM-12 close-out.
> Companions: [README.md](../README.md), [work-packets.md](../work-packets.md), [AM-04 worknotes](AM-04.md), [ADR § 31](../../../architecture/abilities-and-effects-system.md#31-special-ammo-modifies-the-shot-directly-from-resourcesammo_modifiers-ammo-campaign-am-04-d-am07).

## Contract

- **Packet:** AM-11b, tech-disable darts: `Dart_EMP` and `Dart_Radioactive`.
- **Decisions in force:** D-AM07 (the server applies the row directly; toggle abilities are provenance only).
- **Base:** `ammo/am-04-damage-framework` (PR #1047).
- **Out of scope:** the dart weapon `ammo_types` widening and the debug-crate dart rows (AM-11a). The tests stub the bandolier slot.

## Toggle mapping

| Ammo type | Toggle | Cooked text | Notes |
|---|---|---|---|
| `Dart_EMP` | 999 Dart Type: Hazardous: EMP | "Buff: / Toggle / Damage Type: Physical / Penetration: Decreased / Damage: Increased" | `effect_ids` empty; no numbers |
| `Dart_Radioactive` | none | none | No Radioactive or Radiation dart ability exists in `abilities.sql`, the texts seed, or the handoff pack (§ 07 lists 17 toggles). The enum value is a legacy-server name. `toggle_ability_id` is NOT NULL, so the row cites 1227 Dart Type: Hazardous: Contagion ("Buff: / Toggle", no text). Its name matches no other dart ammo type, and Antidote dart 1228's effect 5057 cures `EFFECT_Contagion`, so it was a status that stays on the target. |

The other unassigned hazardous toggles are 998 Disorient, 1215 Interruption and 1226 Confusion. AM-11a may use one of them for Tranquilizer.

## What shipped

| Piece | Where |
|---|---|
| Two `ammo_modifiers` rows, effects 9150 and 9151, NVPs 9150 and 9151 | `db/resources/Abilities/Seed/ammo_modifiers_dart_tech.sql`, one `\ir` line in `db/database.sql` |
| `RadiationDamage` script and the seeded-value constants | `crates/cell-world/src/cell/effects/ammo_dart_tech.rs`, one arm in `registry.rs` |
| Pipeline smoke tests | `crates/cell-combat/src/cell/abilities/damage_apply/ammo_dart_tech_tests.rs`, one `mod` line in `damage_apply/mod.rs` |
| A row per dart in the family table under ADR § 31 | `docs/architecture/abilities-and-effects-system.md` |

## Rows (RECONSTRUCTION)

| Type | `damage_mult` | `penetration_mult` | `damage_type` | On hit |
|---|---|---|---|---|
| `Dart_EMP` | 1.1 | 0.75 | `DT_Physical` | 9150: `RangedEnergyDamage`, `FocusDamage` 50, one shot |
| `Dart_Radioactive` | 1.0 | 1.0 | NULL (the ability's own) | 9151: `RadiationDamage`, `HealthDamage` 3, 5 pulses, 2 s apart |

The dart auto attack (1086, effect 1237) is "-100F / -10H", so the payloads are sized against a 10-health, 100-focus hit. EMP follows 999's directions, but milder than Hollow Point's 1.25 / 0.5 because the dart's value is its payload. The Radioactive dose is 15 HEALTH over 8 seconds; a same-shooter hit refreshes it (pulsing's same-source rule), so it tops out at 3 HEALTH every 2 seconds per shooter.

## Behaviour choices

- **EMP is a FOCUS drain, not an interrupt.** FOCUS is the shield, and draining it is the reading the campaign gives the EMP bullet (AM-09). An interrupt would have to cancel the target's channel or pending warmup, which is async combat code (`pulsing::cancel_channels_from_attacker`, the pending-cast table) that a synchronous `EffectScript` cannot reach. The drain reuses `RangedEnergyDamage`, so EMP needs no code.
- **Radioactive is a DoT, not a debuff.** It needs a new script because an on-hit effect with no `script_name` never fires its first pulse: `apply_damage_to_target` reads NVP damage only from the ability's own effects, so an NVP-only DoT would lose a pulse. `MeleeDamage` does the arithmetic but logs each tick as `melee_damage`; `Suppression` is documented as the future home of a movement slow, which a radiation dose should not inherit. `RadiationDamage` logs `radiation_pulse` with `health_before` and `health_after`.
- **No stun and no movement lock.** Following the coordinator's AM-11a finding: `Stun` re-runs `on_apply` on every pulse and same-shooter refresh but clears the `BSF_MOVEMENT_LOCK` refcount once, and NPC AI never reads that flag, so neither dart uses it.

## Known limits

- Both payloads write the stat directly, like every effect script: armour and MITIGATION never reduce them.
- `MITIGATION` is capped at 0 in the default stats, so EMP's `penetration_mult` of 0.75 does nothing in live play; the EMP shot is a plain 10% damage increase until mitigation is populated.
- The dose's pulses after the first run through the pulse tick, which skips dead targets, floors surrendered NPCs at 1 and credits a DoT kill. The first pulse runs in `apply_damage_to_target`'s script pass, before its death sweep.

## Client exposure and UAT

Effects 9150 and 9151 exist only in the server seed, not in the client's cooked data. What each dart sends the client:

| Dart | Per-effect client message | Risk |
|---|---|---|
| `Dart_EMP` | None. Effect 9150 is single-shot, so it never registers an active effect; the client sees only the target's `onStatUpdate` (FOCUS) that follows every scripted hit. | None known. |
| `Dart_Radioactive` | Yes. Effect 9151 pulses, so `pulsing::register_active_effect` sends `onTimerUpdate` type 5 (`EffectSet`) with `ID` and `SecondaryId` 9151 addressed to the target entity on every hit, and a zero timer when the dose ends. | **Unverified.** `EffectSet_HandleOnTimerUpdate` (`0x00e09160`) stores the id in its active-effect list; nobody has checked whether the effect bar then looks 9151 up in cooked data, or what it does when the lookup misses. An unknown cooked id has crashed the client before (dialog overrides, #938). |

Nothing reaches a client until AM-12 turns `ammo.finite_special` on. Before then, one UAT step should cover it: fire a Radioactive dart at an NPC with the target frame open, and watch for a crash, a missing icon, or a blank icon for the 8 seconds of the dose. If the client mishandles the id, the fix is in the shared pulsing layer (a per-effect "server-only, no timer" switch), which every Wave-2 family with a pulsing on-hit effect (AM-08's 9110 too) would need, not in this packet.

## Telemetry

| Event | Level | Target | Fields |
|---|---|---|---|
| `radiation_pulse` | INFO | `abilities` | `source_id`, `target_id`, `effect_id`, `damage`, `health_before`, `health_after` |
| `radiation_pulse_skipped` | DEBUG | `abilities` | `reason` (`target_missing`, `target_dead`), `source_id`, `target_id`, `effect_id` |

The EMP drain logs through `RangedEnergyDamage`'s existing `ranged_energy_damage` event, and both shots log AM-04's `ammo_damage_applied` with the `on_hit_effect_id`.

## Tests

| Test | Type | Proves |
|---|---|---|
| `ammo_dart_tech::tests::emp_dart_scales_up_lowers_penetration_and_lands_physical`, `radioactive_dart_leaves_the_shot_itself_alone` | unit (modifier math) | the rows resolve with the seeded factors and damage type |
| `ammo_dart_tech::tests::emp_dart_hit_drains_focus_only`, `radioactive_dart_hit_starts_a_pulsing_dose`, `default_dart_has_no_payload` | unit | the shot's on-hit effect resolves and its script does the payload |
| `ammo_dart_tech::tests::` the four `RadiationDamage` tests | unit, negative-log | per-pulse damage, the log fields, the floor at 0, the dead-target skip, bad NVPs, a missing target |
| `ammo_dart_tech::tests::live_db_dart_tech_seed_rows` | live-DB seed guard | both rows, both effects (script, pulses, NVPs) and both provenance abilities load as the constants say, and each effect names a registered script |
| `damage_apply::ammo_dart_tech_tests::emp_dart_hit_drains_the_targets_focus` | pipeline smoke | same roll: an EMP dart hit costs the NPC exactly 50 more FOCUS than a default dart |
| `damage_apply::ammo_dart_tech_tests::radioactive_dart_hit_starts_a_dose_the_tick_keeps_delivering` | pipeline smoke | the hit takes the first pulse, registers 4 more on the NPC, and one pulse tick takes the next |

**Revert proof.** Removing the `registry.rs` arm failed `radiation_is_registered`, `radioactive_dart_hit_starts_a_pulsing_dose`, the live-DB guard and the Radioactive smoke test. Making `RadiationDamage` skip its HEALTH write failed the three script tests that check HEALTH and the Radioactive smoke test. Removing the `\ir` line and reloading the database failed `live_db_dart_tech_seed_rows`. Restored, all pass.
