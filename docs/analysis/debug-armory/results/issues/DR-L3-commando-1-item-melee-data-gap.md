# Data: no shipped item carries ITEM_Melee, so Stealthed Strike (1246) and Lethal Strike (1355) cannot be used

Rows: armory-abilities-commando-1: Commando-1246, Commando-1355 (runner BLOCKED: "no shipped weapon carries ITEM_Melee (weapon-requirement audit FAIL)").

Runs: `C:\Users\Steve\AppData\Local\cimmeria-lab\uat-runs\2026-11-02-unknown-cgbqqpcwwamins`

Expected: the abilities' required weapon moniker is carried by at least one shipped weapon, so a player can equip it and press the ability.

Observed: `resources.abilities` 1246 and 1355 have `item_monikers = [3756430796]`, which `resources.monikers` names `ITEM_Melee`. `resources.items` has 0 rows with that moniker in `moniker_ids`. The blade abilities (641, 651, 1356, 1357) require `ITEM_Blade` (830336901), carried by 53 items, and launch. Both abilities have effects and NVPs (1246: 2, 1355: 4). Same class as GW-3128 in the lab run plan (an item missing its moniker).

Decision needed: add `ITEM_Melee` to the melee weapons in the items seed, or correct the two abilities' requirement to `ITEM_Blade` (or the moniker the client expects). Check the client data before choosing.

Repro: lab_uat_run {sections: ["armory-abilities-commando-1"], specs_dir: "C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\dr-lab\docs\guides\uat-specs", rows: ["Commando-1246"]}

Area: db/resources/ (items, abilities seeds). Fix via the seed-change skill.

Labels: bug, seed-data, debug-armory
