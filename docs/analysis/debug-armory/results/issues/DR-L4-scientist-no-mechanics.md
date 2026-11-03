# Scientist abilities refused no_mechanics: effects without NVPs, or no effects (seed gap)

Rows: armory-abilities-scientist Scientist-943, 944, 945, 962, 963, 964, 965, 966, 971, 972, 994, 1014, 1015, 1019, 1206, 1212, 1214, 1215, 1225, 1226, 1231, 1232, 1236, 2955 (24 rows). Dependents that cannot be tested until the turrets exist: 967, 968, 1207 (owner_pet_refused).
Runs: C:\Users\Steve\AppData\Local\cimmeria-lab\uat-runs\2026-11-02-unknown-cgbqrovhriypya (character Cgbqrovhriypyahi, window 2026-11-02 23:21Z to 2026-11-03 00:01Z)
Expected: each press launches or applies its effect.
Observed: SigNoz scope abilities, cimmeria.lab_run = dr-lab, player_name Cgbqrovhriypyahi: every row shows ability_refused reason no_mechanics (and no_mechanics_refused), no ability_launched. Seed (resources.abilities + resources.effect_nvps):
- effects, 0 NVPs: 943, 944, 945 (Conversation), 962, 963, 964, 966 (Summon Turret), 971, 972, 1206, 1212 (Enhance Turret), 994, 1225 (Deployable stim/health regen), 1019, 1231, 1232 (Shields), 1214 (Repair Restoration), 1236 (Aggression Inducer).
- empty effect_ids: 965, 1014, 1215, 1226, 2955.
- 1015 Gravity Well: 8 effects, only 2 NVPs, still refused.
Same class as DR-L3-commando-1-no-mechanics.md and DR-L3-commando-0-no-mechanics.md; fix in one seed-change pass. The runner PASSed 18 of these rows because only the "sent" clause counted.
Repro: lab_uat_run {sections: ["armory-abilities-scientist"], rows: ["Scientist-962", "Scientist-1019"], specs_dir: "C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\dr-lab\docs\guides\uat-specs"}
Area: db/resources (effect_nvps, abilities.effect_ids); unknown file.
Labels: seed, data, debug-armory
