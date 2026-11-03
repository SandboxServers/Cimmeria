# Battle Avoidance (815) refused no_mechanics: effects without NVPs or scripts (seed gap)

Rows: armory-abilities-archeologist-2 Archeologist-815 (1 row).
Runs: C:\Users\Steve\AppData\Local\cimmeria-lab\uat-runs\2026-11-03-unknown-cgbqztyvkwlksr (character Cgbqztyvkwlksrjk, instance p2, window 2026-11-03 03:21Z to 03:35Z)
Expected: the press launches or applies the stance (an active ability, passive_yn false).
Observed: SigNoz scope abilities, cimmeria.lab_run = dr-lab, player_name Cgbqztyvkwlksrjk, ability_id 815, start 1793676080000 end 1793676930000: use_ability_recv x2, ability_refused reason no_mechanics x2 (stage gate), no_mechanics_refused x2 (passive false, effect_count 3), no ability_launched. Seed (resources.abilities, resources.effects, resources.effect_nvps, live DB): effects 4383 Battle Avoidance, 4382 Remove Stance, 861 Check for Mini-Game stance; 0 NVPs, no script_name. Runner graded the row PASS (press left the client); the client-applied clause failed inside it. Seed file and line: UNKNOWN.
Same class as DR-L5-archeologist-1-no-mechanics.md and the other no-mechanics issues; fix in the same seed-change pass.
Repro: lab_uat_run {sections: ["armory-abilities-archeologist-2"], rows: ["Archeologist-815"], specs_dir: "C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\dr-lab\docs\guides\uat-specs"}
Area: db/resources (effect_nvps, effects.script_name); unknown file.
Labels: seed, data, debug-armory
