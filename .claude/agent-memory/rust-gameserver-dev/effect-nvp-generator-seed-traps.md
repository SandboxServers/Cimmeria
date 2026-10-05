---
name: effect-nvp-generator-seed-traps
description: Effect seed load order (effect_nvps.sql before effects.sql), heal NVP per-pulse math, and which heal effects must stay unbound until routing lands (AB-02)
metadata:
  type: project
---

Facts found while building `tools/ability_mechanics/effect_nvps_from_desc.py` (AB-02, 2026-10-03):

- `db/database.sql` loads `Effects/Seed/effect_nvps.sql` BEFORE `Effects/Seed/effects.sql`. An `UPDATE effects SET script_name ...` placed in the NVP seed matches zero rows silently. Script bindings go in the effect's own INSERT in `effects.sql`.
- Heal per-pulse math: `HealHealth`/`HealFocus` run once per pulse; the pulsing layer fires exactly `pulse_count` pulses (first synchronously in `damage_apply`, then `pulse_count - 1` from `register_active_effect`). A total "over N seconds" is `value / pulse_count`; a "per second" rate is the value as is (1383: 3.00 x 25 = 75%).
- Effects that read like heals but must not get `HealHealth`/`HealFocus` yet, because `apply_hit` / `fire_beneficial` land every effect on the one target: AE/Group halves (1215 Morale Boost AE, 3372, 3374), deployable pulses (3371, 3373: 994/1225 have no `resources.deployables` row), the heal half of DD ability 2848 (4140), Goldam's Gift revive (1357/1358), and bare "+N% Focus" on Buff abilities (2134 is Stance: Courage's max-Focus buff; 5008 is the Stim dart toggle 992, whose real heal is server-only effect 9160).
- Field Medic II (743): effect 789 says +20%, tooltip says 10%; the effect row wins (pet 3211 precedent).
- `nvp_id` ranges per family are fixed by the ability-mechanics ledger; the sequence setval stays at 462, far below 20000.

**Why:** these traps are invisible until a playtest heals a hostile or a seed edit silently no-ops.
**How to apply:** AB-03/AB-04/AB-07/AB-10 extend the generator; read `tools/ability_mechanics/README.md`, rerun `--report`, and lift a scope rejection only in the packet that adds the routing it waits for. See [[deployable-pulse-and-seed-traps]].
