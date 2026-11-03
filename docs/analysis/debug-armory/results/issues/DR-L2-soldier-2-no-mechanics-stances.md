# Soldier-2 Phase Grenade and Stance: Commander refused no_mechanics: effects missing or without NVPs in the seed

Rows: armory-abilities-soldier-2: Soldier-1894 (Phase Grenade), Soldier-1893 (Stance: Commander)

Runs: C:\Users\Steve\AppData\Local\cimmeria-lab\uat-runs\2026-11-02-unknown-cgbqokntohiosu

Expected: the press launches the ability and applies its effects (spec: the press leaves the client; the client applies the ability).

Observed: the runner graded both PASS with the `applied` clause [FAILED]. SigNoz (`service.name = 'cimmeria-server' AND cimmeria.lab_run = 'dr-lab' AND scope_name = 'abilities' AND player_name CONTAINS 'Cgbqokntohiosu'`, 1793656080000 to 1793656500000): 1894 `use_ability_on_ground_recv` then `ability_refused reason=no_mechanics`, `no_mechanics_refused effect_count=0`, at 21:53:05 and 21:53:29; 1893 `use_ability_recv` then `ability_refused reason=no_mechanics`, `effect_count=2`, at 21:53:52 and 21:54:01. Seed (`server_db_query`): 1894 has no effect_ids; 1893 has 2 effects and 0 rows in `resources.effect_nvps`. Both were BLOCKED in the first run (hotbar capacity, fixed by DR-07n), so this is the first time the server saw them. Same family as [DR-L2-soldier-2-no-mechanics.md](DR-L2-soldier-2-no-mechanics.md).

Repro: lab_uat_run {sections: ["armory-abilities-soldier-2"], rows: ["Soldier-1894", "Soldier-1893"], specs_dir: "C:\\Users\\Steve\\source\\projects\\Cimmeria\\.claude\\worktrees\\dr-lab\\docs\\guides\\uat-specs"}

Area: db/resources/ (abilities, effects, effect_nvps seeds); fix via the seed-change skill.

Labels: bug, seed, debug-armory
