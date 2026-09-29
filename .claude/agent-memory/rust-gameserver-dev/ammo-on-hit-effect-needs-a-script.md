---
name: ammo-on-hit-effect-needs-a-script
description: An ammo_modifiers on_hit_effect_id with no script_name never fires its first pulse; the pipeline reads NVP damage only from the ability's own effects. Also: no Dart_Radioactive toggle ability exists.
metadata:
  type: project
---

`damage_apply::apply_damage_to_target` (cell-combat) sums `HealthDamage` /
`FocusDamage` NVPs only over `ability_def.effect_ids`. The ammo on-hit effect
(AM-04) is appended to the *script* list and to the pulsing registration, but
not to the NVP sum. So an on-hit effect with `script_name = NULL` does nothing
on the hit, and if it pulses, the tick delivers only `pulse_count - 1` pulses.

**Why:** found while building AM-11b's radiation dose (2026-09-28).

**How to apply:** every ammo on-hit effect needs a `script_name`. Reuse
`RangedEnergyDamage` for a one-shot Focus/Health payload (`FocusDamage` only =
Focus drain); `RadiationDamage` (`effects/ammo_dart_tech.rs`) is a per-pulse
HEALTH DoT. Do not use `Stun` for pulsed or refreshed effects: its refcount is
bumped every `on_apply` but cleared once.

Also: the client has no Radioactive dart toggle ability (17 toggles in the
handoff pack § 07); `Dart_Radioactive`'s row cites 1227 Contagion. Unassigned
hazardous dart toggles: 998 Disorient, 1215 Interruption, 1226 Confusion.
