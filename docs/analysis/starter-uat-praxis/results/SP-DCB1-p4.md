PACKET SP-DCB1-p4 VERIFIED 2026-11-03 by Sonnet; driver attempts: 3 (attempts 1-2 at 06:20-06:24Z did the row; attempt 3 at 06:43-06:45Z re-drove a spent row); server evidence window 06:20:39Z-06:45:15Z (SigNoz cimmeria-server, lab_run dr-lab; player 98 Dcbbh; entity 39 then 41 after relog)

| Row | Result | Tier | Server evidence (time, scope, event, key fields) | Client evidence | Looker | Notes |
|---|---|---|---|---|---|---|
| CB-U02a Frost search | PASS (server) | N1 right-click (earlier attempts); attempt 3 clicks failed | 06:21:09Z `interaction_pose` dialog "Search Cpl. Frost's Corpse" tag ArmYourself_FrostBody; chain "622 - Dialog 3995 (loot Frost)"; `Content: advancing step` mission 622 2113 -> 80623; granting Frost's Letter; one search. Bookmark 06:24:32Z `step_advance 2113->80623`. | not graded from the driver report | not run | One search only. Tier of the 06:21 click comes from the earlier driver report, not this one. |
| CB-U02a Guard body dialog | PASS (server) | N1 (earlier attempt) | 06:24:22Z `interact` template "Cellblock guard corpse", dialog "Search the Guard's Corpse" (3996) via per-player dialog set; chain "622 - Dialog 3996 (loot Guard)"; `advancing step` 80623 -> 80622 "Equip the pistol from your inventory."; tutorial=5882 chain=1005 shown once. | none | not run | Dialog once. |
| CB-U02a Grant, once | PASS | server | 06:24:22Z `Content: granting item` "SI 3 9mm Pistol" once in the whole window; `content_grant_ability` [592, 594, 597, 1218] once; DB `sgw_player_ability_grants` player 98: exactly four rows, one per ability (source tutorial 622, granted 06:24:22.476Z). DB `sgw_inventory` character 98: exactly one type_id 55 (item_id 10567, now container 3 bandolier slot 0 after a later equip). | driver INV read: pistol in Bandolier as item 10567 | n/a | Driver "FAIL" on INV is a reading error: it looked for instance id 55; 55 is the template type_id. Exactly one exists. |
| Attempt 3 clicks (CB-U02a.3, .5) | case (c) lab/driver | N1 | Interacts DID reach the server: 06:44:11Z and 06:44:24Z (Corporal Frost), 06:44:58Z (Cellblock guard corpse). Each logged "target has no static interaction type", "no per-player interactions for template", "fire_interact_template/tag: no chains matched", no dialog, no step change. Bookmark 06:45:14Z shows mission 622 status 2 (completed), steps 80623+80622 done, offered_dialog_ids [] | windows_opened [] | n/a | The fixture was already past the row (mission completed, dialog sets removed), so no dialog could open. Not a product bug and not a click failure: server received all three interacts and correctly ignored them. |
| H-CB-U02-0 | NEEDS_HUMAN | N1 | 06:24:32Z bookmark: `offered_dialog_ids [5882]`, event `tutorial=5882 chain=1005` (server showed it once) | shot 20261103-004509.864 taken at 06:45:09Z, after the tutorial was gone | Q1 tutorial window on screen? NO (only quest tracker and chat). Q2 dialog still open? NO. | The 06:45 shot cannot show the tutorial; the 06:24 shot lacks a looker. |

SIGN-OFF SHEET (NEEDS_HUMAN):
- CB-U02a, H-CB-U02-0 tutorial 5882 on screen: C:\Users\Steve\AppData\Local\cimmeria-lab\screenshots\lab_screenshot-20261103-004509.864.png (driver did not judge; looker saw no tutorial and no dialog, expected at this late time). The relevant shot is the earlier C:\Users\Steve\AppData\Local\cimmeria-lab\screenshots\lab_screenshot-20261103-002428.646.png. H-CB-U02-1 not produced here.

FINDINGS:
none. No clause is contradicted; item 55 and each ability were granted once.

NOT RUN / BLOCKED:
NOT-RUN here, see SP-DCB1-p5: equip, reload, two shots, Focus before the first shot, relog duplicate check, H-CB-U02-1. CB-U02a.4 LOOT_ALL not run (Frost search already done). Attempt 3 did not need to run: the row was already complete from attempts 1-2.

RESUME HINT:
Do not re-search Frost or the guard body; the dialogs are spent and mission 622 is complete. The pistol (item 10567) is already in the bandolier. Go on to SP-DCB1-p5 from the current state.