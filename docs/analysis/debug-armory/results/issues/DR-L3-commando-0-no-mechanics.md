# Seed: 15 Commando tree-0 abilities refused no_mechanics (no effects or no effect NVPs)

Rows: armory-abilities-commando-0: Commando-1259, 1363, 1366, 1868, 642, 886, 888, 890, 883, 884, 901, 887, 2173, 2864, 3358.

Runs: `C:\Users\Steve\AppData\Local\cimmeria-lab\uat-runs\2026-11-02-unknown-cgbqpzpxnjqkjl`

Expected: the press leaves the client and the client applies the ability (damage, buff, mine placement, stance).

Observed: SigNoz scope `abilities`, `cimmeria.lab_run = 'dr-lab'`, `player_name CONTAINS 'Cgbqpzpxnjqkjl'`, 22:34Z to 22:52Z 2026-11-02: each ability got 2x `use_ability_recv` (or `use_ability_on_ground_recv`) then `ability_refused` reason `no_mechanics`. Seed (`resources.abilities`, `resources.effect_nvps`): 642, 886, 888, 890, 1363, 1366, 1868 have empty `effect_ids`; 883, 884, 901, 887, 1259, 2173, 2864, 3358 have effects but zero NVPs. The runner PASSed 14 of them because only the `sent` clause counted; its `applied` clause failed.

Repro: lab_uat_run {sections: ["armory-abilities-commando-0"], specs_dir: "C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\dr-lab\docs\guides\uat-specs", rows: ["Commando-00", "Commando-642", "Commando-883"]}

Area: db/resources/ (abilities, effects, effect_nvps seeds). Fix via the seed-change skill.

Labels: bug, seed-data, debug-armory
