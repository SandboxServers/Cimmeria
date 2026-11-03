# Archeologist abilities refused no_mechanics: effects without NVPs or scripts (seed gap)

Rows: armory-abilities-archeologist-0 Archeologist-746, 781, 784, 791, 1081, 1082, 1083, 1183, 1187, 1188, 1190, 1191, 1192, 2074 (14 rows).
Runs: C:\Users\Steve\AppData\Local\cimmeria-lab\uat-runs\2026-11-03-unknown-cgbqyvjnrgvlvq (character Cgbqyvjnrgvlvqsi, window 2026-11-03 02:53Z to 03:06Z)
Expected: each press launches or applies its effect.
Observed: SigNoz scope abilities, cimmeria.lab_run = dr-lab, player_name Cgbqyvjnrgvlvqsi: every row shows ability_refused reason no_mechanics (and no_mechanics_refused), no ability_launched. Seed (resources.abilities, resources.effects, resources.effect_nvps): every effect of these abilities has 0 NVPs and no script_name. Effects: 746 [792 Engagement State]; 781 [1414, 824]; 791 [4477, 1415, 837]; 784 [830]; 1081 [3652, 1230, 1229]; 1082 [4550, 1232, 1231]; 1083 [4551, 1234, 1233]; 1183 [1346]; 1187 [1352, 1351]; 1188 [1354, 1353]; 1190 [1358, 1357]; 1191 [1359 threat wipe]; 1192 [1360 focus heal]; 2074 [5306, 4479, 4478, 2758, 2759 aura]. Working comparison: 834/835 (HealPercentage + script HealFocus) and 2827 (RemoveCategories, RemovePolarity + script RemoveEffects) launched on 788, 789, 2099.
Same class as DR-L4-scientist-no-mechanics.md and the DR-L3 no-mechanics issues; fix in one seed-change pass. The runner PASSed 8 of these rows because only the "sent" clause counted.
Repro: lab_uat_run {sections: ["armory-abilities-archeologist-0"], rows: ["Archeologist-1191", "Archeologist-1082"], specs_dir: "C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\dr-lab\docs\guides\uat-specs"}
Area: db/resources (effect_nvps, effects.script_name); unknown file.
Labels: seed, data, debug-armory
