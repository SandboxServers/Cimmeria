---
name: per-shot-damage-seam-is-damage-apply
description: To change every weapon shot's damage, hook damage_apply::apply_damage_to_target, not an effect script; the armour term is dead live (MITIGATION capped at 0)
metadata:
  type: project
---

Every ability hit goes through `cell-combat` `abilities::damage_apply::apply_damage_to_target` → `combat::calculate_damage_penetrating` / `_scaled` (pipeline.rs). Effect scripts (`RangedPhysicalDamage` etc. in `cell-world` `effects/scripts.rs`) run only for effects with a `script_name` (8 seeded use RangedPhysicalDamage), and add a Focus-pierce bleed on top of the pipeline damage. A "wrap the script" plan modifies a handful of abilities, not every shot (found in AM-04, 2026-09-28).

- "Weapon shot" test = `AbilityDef.required_ammo > 0` (use_ability's own rule; grenades and melee have 0). NPCs have no bandolier.
- `StatList::new` caps `MITIGATION` at max 0, so `af * (miti - pen)/100` is always 0 live; anything keyed on armour or penetration is inert until mitigation is populated. Tests must raise the cap with `stat.update(0, v, 100)`.
- Audit A-10's `resolve.rs` "placeholder" ammo zeros were test fixtures, not production code.
- Deterministic hit rolls in damage tests: `pseudo_random_seed(entity, ability, effect_seq)` + `calculate_result`; scan effect_seq for an `RC_HIT` rather than hardcoding one.

**Why:** the ammo plan assumed the script seam; the pipeline is the only place that sees every shot.
**How to apply:** any per-shot modifier (ammo families, buffs keyed on the weapon) goes through `ammo_damage::shot_ammo`-style resolution in damage_apply. See [[ability-launch-fire-split]].
