# Weapon-requirement audit fails: abilities require an item moniker no shipped item carries (D-DR3)

Rows: armory-abilities-scientist Scientist-997 (this file); DR-L3 adds armory-abilities-commando-1 Commando-1246, 1250, 1355 (see DR-L3-commando-1-item-melee-data-gap.md).
Runs: C:\Users\Steve\AppData\Local\cimmeria-lab\uat-runs\2026-11-02-unknown-cgbqrovhriypya
Expected: Scientist-997 "Full Mag: Dart Pistol" is pressed with a dart weapon equipped and launches.
Observed: runner BLOCKED "no shipped weapon carries ITEM_Dart_Rifle". resources.abilities 997 has item_monikers = [3257416555] = ITEM_Dart_Rifle; resources.items has 0 items with that moniker id. The names look swapped: 1000 "Dart Rifle: Multi-Shot" requires 2389790449 = ITEM_DartPistol (25 items) and launched. Query: server_db_query, count of resources.items whose moniker_ids contain each moniker id. No server events for ability 997 in the run window (no press).
Repro: lab_uat_run {sections: ["armory-abilities-scientist"], rows: ["Scientist-997"], specs_dir: "C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\dr-lab\docs\guides\uat-specs"}
Area: db/resources (abilities item_monikers or items moniker_ids); unknown file.
Labels: data, seed, debug-armory
