# Archeologist abilities refused no_mechanics: effects without NVPs or scripts (seed gap)

Rows: armory-abilities-archeologist-1 Archeologist-643, 718, 751, 752, 753, 754, 777, 1178, 1179, 1180, 1181, 1182 (12 rows).
Runs: C:\Users\Steve\AppData\Local\cimmeria-lab\uat-runs\2026-11-03-unknown-cgbqzhbkqryynk (character Cgbqzhbkqryynksb, window 2026-11-03 03:06Z to 03:21Z)
Expected: each press launches or applies its effect.
Observed: SigNoz scope abilities, cimmeria.lab_run = dr-lab, player_name Cgbqzhbkqryynksb, start 1793675190000 end 1793676080000: every row shows ability_refused reason no_mechanics (and no_mechanics_refused), no ability_launched. Seed (resources.abilities, resources.effects, resources.effect_nvps, live DB): no effect has a mechanic NVP or script_name; the only NVP is EffectCategory=Mental on effects 1228, 1474, 1479, 1486. Effects: 643 [1474, 707]; 751 [1427, 797]; 752 [2475, 798]; 753 [1486, 799]; 754 [1480, 1479, 1228, 800]; 777 [2776, 2775, 2774, 821]; 1178 [2773, 1341]; 1179 [2777, 1342]; 1180 [4566, 1343]; 1181 [2781, 2780, 2779, 1344]; 1182 [2782, 1345]; 718 [778 Selective Fire Target Damage]. Working comparison: 598 Quick Burst (effect 660, 2 NVPs) and 742/743 Field Medic launched in the same run. Seed file and line: UNKNOWN.
Same class as DR-L5-archeologist-0-no-mechanics.md, DR-L4-scientist-no-mechanics.md and the DR-L3 no-mechanics issues; fix in one seed-change pass (mental-resist, suppression, fear, confusion, disorient and threat-wipe mechanics, plus damage NVPs for 778).
Repro: lab_uat_run {sections: ["armory-abilities-archeologist-1"], rows: ["Archeologist-718", "Archeologist-1182"], specs_dir: "C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\dr-lab\docs\guides\uat-specs"}
Area: db/resources (effect_nvps, effects.script_name); unknown file.
Labels: seed, data, debug-armory
