# Six soldier-2 abilities refused no_mechanics: effects missing or without NVPs in the seed

Rows: armory-abilities-soldier-2: Soldier-854 (Smoke Grenade), Soldier-858 (Leadership), Soldier-1455 (Quick Launch), Soldier-1891 (Stance: Rally), Soldier-1329 (Incendiary Grenade), Soldier-2864 (EMP Grenade)

Runs: C:\Users\Steve\AppData\Local\cimmeria-lab\uat-runs\2026-11-02-unknown-cgbqnmmsdfkvui

Expected: the press launches the ability and applies its effects (spec: the press leaves the client; the client applies the ability).

Observed: the runner graded 854, 858, 1455, 1891 PASS with the `applied` clause [FAILED]. SigNoz (`cimmeria.lab_run = 'dr-lab'`, scope `abilities`, player Cgbqnmmsdfkvuijp): `ability_refused reason=no_mechanics` and `no_mechanics_refused` at 21:25:39 (858), 21:26:11 (1455), 21:30:40 (1891), 21:32:00 (854), 21:35:30 (1329), 21:36:55 (2864). Seed: 854, 1329, 1455 have `effect_ids = {}`; 858 (effects 1211, 921), 1891 (5309, 4354, 4353, 4352, 2430) and 2864 (4200-4206) have effects but zero `resources.effect_nvps`. Same family as the soldier-1 issues (656, 1483, 1456, 1882, 2102).

Repro: lab_uat_run {sections: ["armory-abilities-soldier-2"], rows: ["Soldier-854", "Soldier-858", "Soldier-1455", "Soldier-1891"], specs_dir: "C:\\Users\\Steve\\source\\projects\\Cimmeria\\.claude\\worktrees\\dr-lab\\docs\\guides\\uat-specs"}

Area: db/resources/ (abilities, effects, effect_nvps seeds); fix via the seed-change skill.

Labels: bug, seed, debug-armory
