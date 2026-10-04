---
name: effect-flag-64-marks-sequenced-damage
description: In the effect seed, single-shot EF_SequenceOnFinish (64) damage rows are conditional/chain follow-ups (vs low Focus, chain jumps, barrage shells); the pipeline applies every effect of an ability, so per-effect NVP damage must not include them
metadata:
  type: project
---

`damage_apply` runs every effect of an ability on the hit target; nothing evaluates sequences or conditions. Since AB-03 (2026-10-03) each `TCM_Single` effect's `HealthDamage`/`FocusDamage` resolves on its own (`damage_apply/nvp_damage.rs`), so any alternate-outcome row that gains NVPs stacks on the base hit.

The seed marks those rows with `EF_SequenceOnFinish` (64, `entities/defs/enumerations.xml`) on a single-shot effect: Execution 1604 and Red Mist 1609 ("vs low Focus", after a "Low Focus Check"/"Focus % Check" effect), Energy Cascade cone 2/3 and "Final target" rows, Grenade Barrage's extra shells (2843, 3516), Sticky Bomb 1431/4363, EMP Grenade 4200/4202. Pulsing DoTs carry 64 too (Lethal Shot 1606, 2720, 2731) and are fine. Positional/stance variants (Surprise Attack 1559/1560, Back Slash bonus rows, "Assassin Stance Bonus Damage") have no flag; they are recognised by name.

**Why:** the generator's `damage` family (`tools/ability_mechanics/families/damage.py`) reports both kinds instead of writing rows; flag 64 is the only machine-readable marker.

**How to apply:** when a later packet models sequences or conditions (AB-09d resist rolls, positional damage), these reported effects are the ones to revisit; until then never give a single-shot flag-64 damage effect NVPs. Also: cone/radius NVP damage lands on a hit only when no direct (non-pulsing) `TCM_Single` damage effect lands; ground casts pass the full def to every target, so radius routing proper is AB-07.
