# Soldier-1882 and Soldier-2102 refused no_mechanics: abilities have no effects in the seed

Rows: armory-abilities-soldier-1: Soldier-1882 (Launch Grenade: Tear Gas), Soldier-2102 (Incinerate Ground)

Runs: C:\Users\Steve\AppData\Local\cimmeria-lab\uat-runs\2026-11-02-unknown-cgbqmyhigfcmah

Expected: the ground-targeted press launches the ability and applies its effect (spec: the press leaves the client; the client applies the ability).

Observed: the runner graded both PASS, but its `applied` clause was [FAILED]. SigNoz (`cimmeria.lab_run = 'dr-lab'`, scope `abilities`, player Cgbqmyhigfcmahzf): `use_ability_on_ground_recv` then `ability_refused reason=no_mechanics` and `no_mechanics_refused effect_count=0`, at 21:11:07.765 and 21:11:34.022 (1882) and 21:12:24.296 and 21:12:51.504 (2102). Seed: `resources.abilities` rows 1882 and 2102 have `effect_ids = {}` and no `resources.effects` rows. Same family as Soldier-656, 1483, 1456.

Repro: lab_uat_run {sections: ["armory-abilities-soldier-1"], rows: ["Soldier-1882", "Soldier-2102"], specs_dir: "C:\\Users\\Steve\\source\\projects\\Cimmeria\\.claude\\worktrees\\dr-lab\\docs\\guides\\uat-specs"}

Area: db/resources/ (abilities, effects seeds); fix via the seed-change skill.

Labels: bug, seed, debug-armory
