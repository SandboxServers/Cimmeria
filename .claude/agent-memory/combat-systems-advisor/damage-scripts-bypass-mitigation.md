---
name: damage-scripts-bypass-mitigation
description: Rust damage scripts (RangedPhysicalDamage etc.) write raw stat deltas; python ran them through qrCombatDamage (QR, armour, absorb). After AB-06 the script is the only path.
metadata:
  type: project
---

Rust `RangedPhysicalDamage` / `MeleePhysicalDamage` / `RangedEnergyDamage` / `MeleeDamage`
(`crates/cell-effect-scripts/src/cell/effects/scripts.rs`) subtract the NVP amounts straight
from FOCUS/HEALTH. Python's `deprecated/python/cell/effects/RangedPhysicalDamage.py` calls
`effect.qrCombatDamage(stat, 14, base, True, True)` for both legs, so QR multiplier, DAMAGE bonus,
resist, armour/penetration and ABSORB_* pools all applied.

**Why it matters:** AB-06 (PR #1152, D-AB07) made the damage script the only damage path for its
effect, so crit/graze, armour, special-ammo penetration and AbsorbShield no longer affect Pistol
Shot (592 = NPC_DEFAULT_ABILITY) or Strike. Only cover/ammo/splash `damage_scale` is applied
(pre-scaled NVPs in `damage_apply/effect_scripts.rs`).

**How to apply:** when reviewing damage numbers, shields or crit reports on scripted abilities,
check whether scripts have been routed through `calculate_damage_penetrating` yet. Also: in a
mixed ability (some effects EF_DontUseQR=16, some not; 184 seeded abilities) the hit still rolls,
and the no-QR effect's NVP damage merges into the one `health_base` scaled by the rolled qr_rand.
