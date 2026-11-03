# Soldier damage abilities 718, 725, 1477 refused as no_mechanics: their effects have no damage NVPs

Rows: `armory-abilities-soldier-0`: Soldier-718 (Selective Fire), Soldier-725 (Sustained Fire), Soldier-1477 (All Out Assault).
Runs: `C:\Users\Steve\AppData\Local\cimmeria-lab\uat-runs\2026-11-02-unknown-cgbqjljvxcsrgh`.

Expected: a Soldier with the right weapon equipped presses the ability and the server launches it (like 598, 713, 1476 in the same run).

Observed: the press reaches the server and is refused at the gate. SigNoz (scope `abilities`, `cimmeria.lab_run = 'dr-lab'`, `player_name CONTAINS 'Cgbqjljvxcsrgh'`, window 2026-11-02T19:23Z to 19:39Z):
- 718: `use_ability_recv` then `ability_refused reason=no_mechanics stage=gate` at 19:29:22.834 and 19:29:31.703.
- 725: same at 19:32:08.432 and 19:32:17.297.
- 1477: same at 19:37:29.075 and 19:37:38.081.

Cause (read-only DB): the abilities are `ABILITY_TYPE_DD`, `passive_yn = false`, `event_set_id = 15`, `item_monikers = [3175425141]`, with `effect_ids` 778 / 759, 758, 757 / 5270, 5269, 5268, 5267, 4357, 3702. `resources.effect_nvps` has no row for any of those effect ids, so `ability_effects_have_mechanics` (crates/entity/src/abilities/implemented.rs) is false and `lacks_mechanics` refuses. The launching abilities do have NVPs (effect 660: `FocusDamage 200`, `HealthDamage 20`; 1767, 1770, 1772 likewise). The effects' own `effect_desc` gives the intended numbers: 778 "Single Target -200F / -20H, 1 second pulse, 3 ammo"; 759 "Single Target Channeled: 5 ticks -300F / -30H"; 5270 "F-300 H-30, 5 pulses".

Repro: `lab_uat_run {sections: ["armory-abilities-soldier-0"], rows: ["Soldier-00", "Soldier-718", "Soldier-725", "Soldier-1477"], specs_dir: "C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\dr-lab\docs\guides\uat-specs"}`.

Seed lines: `db/resources/Effects/` (effect_nvps for the ids above), `db/resources/Abilities/Seed/abilities.sql` (abilities 718, 725, 1477). Fix via the `seed-change` skill; no migration.

Area: db/resources (Effects seed), not the server gate (the gate behaved correctly for the data).
Labels: bug, area:seed, abilities (see docs/agents/).
