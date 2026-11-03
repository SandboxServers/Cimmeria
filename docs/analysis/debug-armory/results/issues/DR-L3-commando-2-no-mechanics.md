# Seed: Commando-640 Aimed Shot: Torso refused no_mechanics (effect 703 has no NVPs)

Rows: armory-abilities-commando-2: Commando-640. Group with [DR-L3-commando-0-no-mechanics.md](DR-L3-commando-0-no-mechanics.md) and [DR-L3-commando-1-no-mechanics.md](DR-L3-commando-1-no-mechanics.md).

Runs: `C:\Users\Steve\AppData\Local\cimmeria-lab\uat-runs\2026-11-02-unknown-cgbqrbvtowfscz`

Expected: the press leaves the client and the client applies the ability.

Observed: SigNoz scope `abilities`, `cimmeria.lab_run = 'dr-lab'`, `player_name CONTAINS 'Cgbqrbvtowfsczqr'`, 23:06Z to 23:20Z 2026-11-02: ability 640 got 2x `use_ability_recv`, `no_mechanics_refused` reason `no_mechanics` and 2x `ability_refused` reason `no_mechanics`. Seed: `resources.abilities` 640 has `effect_ids = [703]`, `resources.effect_nvps` holds 0 rows for it. Runner FAIL, `applied` clause.

Repro: lab_uat_run {sections: ["armory-abilities-commando-2"], specs_dir: "C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\dr-lab\docs\guides\uat-specs", rows: ["Commando-00", "Commando-640"]}

Area: db/resources/ (abilities, effects, effect_nvps seeds). Fix via the seed-change skill.

Labels: bug, seed-data, debug-armory
