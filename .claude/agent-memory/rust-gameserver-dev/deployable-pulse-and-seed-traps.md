---
name: deployable-pulse-and-seed-traps
description: Deployables Phase 0 (2026-09-28) traps: apply_damage_to_target registers every pulsing effect of the def it is given, cooked paks are unzippable SOAP-XML, DeploymentBar flag is not a spawn marker, template blocks up to 409 are taken, fall-through verdicts hide a removed check
metadata:
  type: project
---

Learned building 1012 Deployable: Microwave Emitter (`crates/cell-combat/src/cell/abilities/deployable/`, `crates/cell-world/src/cell/deployables/`).

- **`apply_damage_to_target` registers every pulsing effect of the `AbilityDef` it is handed** on the target (the "Register pulsing effects" block). Re-using it for a secondary hit of an ability that also carries a long "Pulser"/lifetime effect puts that effect on every target. Hand it a clone with `effect_ids = vec![the_one_effect]`. Guard: `a_pulse_registers_no_lifetime_effect_on_its_target`.
- **The cooked caches are plain zips.** `unzip data/cache/CookedDataEffects.pak _5065` gives one SOAP-XML row; the seed matches it field for field. Effect descriptions encode damage as `-100F -10H` = NVPs `FocusDamage 100`, `HealthDamage 10` (rows 641, 656).
- **Ability flag 2 is `DeploymentBar`** (`enumerations.xml` `EAbilityFlags`): 105 abilities, grenades and mines included. It is a bar category, never a "this spawns an object" marker.
- **Effect flags 144 on 5066 = `EF_DontUseQR` (16) + `EF_SequenceOnPulse` (128)** in the client enum; `crates/entity/src/abilities/defs.rs` `EF_*` "category" constants use different numbers (e.g. `EF_DONT_USE_QR = 32`). Check `enumerations.xml` `EEffectFlag` before reading a flag.
- **`entity_templates` id blocks 200-399 are all reserved** (Harset, debug hub, BM, crafting, orgs, pets, bank, social). Deployables took 400-409; that meant raising the sequence floor in the `entity_templates.sql` footer **and** `live_db_seed_sequences.rs` together.
- **Class `being` (0x01) is the "untargetable owned object" shape**: out of `all_npc_entity_ids`/`area_candidates`, refused by `generate_threat`, not AI-ticked while Idle. A pet class would bind into the owner's pet bar.
- **Fall-through verdicts hide a removed check.** Deleting the `OwnerGone` arm still despawned the object on logout (the next arm, `OwnerLeftSpace`, matched because the owner has no space). Guard such chains with tests that assert the *reason* and the id-reuse case (same space, other player), not just "gone".
- `handle.rs` sat at 698 lines; any addition there needs a split first (the not-known feedback went to `use_ability/not_known.rs`).

Related: [[entity-template-seed-authoring]], [[destroy-entity-vs-despawn-npc]], [[npc-class-filter-and-dead-target-traps]], [[revert-proof-mutation-must-be-confirmed]].
