# Soldier abilities 656 and 1483 refused as no_mechanics: their effects have no NVPs

Rows: `armory-abilities-soldier-1`: Soldier-656 (Sustained Bursts), Soldier-1483 (Steady Launch).
Run: `C:\Users\Steve\AppData\Local\cimmeria-lab\uat-runs\2026-11-02-unknown-cgbqjzujpscktw`.

Expected: a Soldier with the right weapon equipped presses the ability and the server launches it (like 612, 632, 658, 853 in the same run).

Observed: the press reaches the server and is refused at the gate. SigNoz (scope `abilities`, `cimmeria.lab_run = 'dr-lab'`, `player_name CONTAINS 'Cgbqjzujpscktw'`, window 2026-11-02T19:39Z to 19:52Z):
- 656: `use_ability_recv`, `no_mechanics_refused`, `ability_refused reason=no_mechanics` at 19:43:50.786 and 19:43:59.566.
- 1483: same at 19:45:41.090 and 19:45:49.934.

Cause (read-only DB): 656 is `ABILITY_TYPE_DD`, `passive_yn = false`, `event_set_id = 15`, effect_ids 4336 and 721, with 0 `resources.effect_nvps` rows (description: "-1500F / -150H, 5 ammo per pulse, 50 pulses"). 1483 is `ABILITY_TYPE_Buff`, `passive_yn = false`, effect_ids 1779, 0 NVP rows (description: "+10% Interruption Resistance for 30 seconds"). `ability_effects_have_mechanics` is false for both, so the gate refuses. Same family as `DR-L2-soldier-0-damage-nvps-missing.md`. The runner graded 656 FAIL (client never applied it) and 1483 PASS on the `sent` clause only; its `applied` clause also shows [FAILED].

Repro: `lab_uat_run {sections: ["armory-abilities-soldier-1"], rows: ["Soldier-00", "Soldier-656", "Soldier-1483"], specs_dir: "C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\dr-lab\docs\guides\uat-specs"}`.

Fix: `seed-change` skill, edit `db/resources/` (Effects NVPs for 4336, 721, 1779); no migration.

Area: db/resources (Effects seed), not the server gate.
Labels: bug, area:seed, abilities (see docs/agents/).
