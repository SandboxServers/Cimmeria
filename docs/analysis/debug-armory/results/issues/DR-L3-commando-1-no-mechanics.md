# Seed: 7 Commando tree-1 abilities refused no_mechanics (no effect NVPs, or no effects)

Rows: armory-abilities-commando-1: Commando-649, 860, 872, 874, 875, 1244, 1253. Group with [DR-L3-commando-0-no-mechanics.md](DR-L3-commando-0-no-mechanics.md); likely also 1250 and 880 once they can run.

Runs: `C:\Users\Steve\AppData\Local\cimmeria-lab\uat-runs\2026-11-02-unknown-cgbqqpcwwamins`

Expected: the press leaves the client and the client applies the ability (reveal, fade, mark, sensor effects).

Observed: SigNoz scope `abilities`, `cimmeria.lab_run = 'dr-lab'`, `player_name CONTAINS 'Cgbqqpcwwamins'`, 22:52Z to 23:04Z 2026-11-02: each ability got 2x `use_ability_recv` then `ability_refused` reason `no_mechanics`. Seed (`resources.abilities`, `resources.effect_nvps`): 649 (effects 3947, 713), 860 (923), 872 (942), 874 (3948, 945), 875 (4249, 946), 1244 (4250, 1405) have effects but zero NVPs; 1253 Pulse Sensor has empty `effect_ids`. The runner PASSed 6 of them (860 failed) because only the `sent` clause counted; its `applied` clause failed.

Repro: lab_uat_run {sections: ["armory-abilities-commando-1"], specs_dir: "C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\dr-lab\docs\guides\uat-specs", rows: ["Commando-00", "Commando-649", "Commando-1253"]}

Area: db/resources/ (abilities, effects, effect_nvps seeds). Fix via the seed-change skill.

Labels: bug, seed-data, debug-armory
