---
name: per-effect-nvp-resolution
description: AB-03 (PR #1157) per-effect NVP damage model in damage_apply/nvp_damage.rs — area-collapse rule, which callers pass the full def, entries stop after the first lethal one
metadata:
  type: project
---

Since AB-03 (PR #1157, reviewed 2026-10-03) every landing `TCM_Single` NVP effect resolves on its own
(`damage_apply/nvp_damage.rs`), at the hit's QR or the unrolled midpoint when `EF_DontUseQR`.
Cone/radius NVPs collapse to one entry (last positive per pool) and land on the primary only when no
non-pulsing single-target NVP effect lands.

**Why it matters:** callers differ in what def they pass. Cone fan-out (`cone_aoe/fan_out.rs`) passes a
scoped one-effect def, so secondaries get the cone. The ground path (`dispatch/mod.rs::apply_secondaries`)
and ammo splash pass the FULL def, so every radius target gets the single-target effects and a DoT's
first pulse (splash does not register the DoT). No radius fan-out exists for entity-targeted casts.

**How to apply:** when reviewing damage changes, check all three callers. Known quirks: entries stop once
an earlier one kills the target, so a hit carries one `SRC_MORTAL` (guarded by `a_killing_effect_ends_the_hit_with_one_mortal_entry`; before the review fix it emitted two). Flat armour mitigation is now
subtracted per effect, so small H values (10-30) can round to 0. NPC ability sets (15 abilities) had no
multi-NVP abilities at review time, so NPC damage was unchanged. See [[npc-ability-sets]].
