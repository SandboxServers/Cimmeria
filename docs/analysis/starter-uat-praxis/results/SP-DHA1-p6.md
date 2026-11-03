PACKET SP-DHA1-p6 VERIFIED 2026-11-03 by Sonnet; driver attempts: 1; server evidence window 15:23:27Z-15:25:30Z (local lab server log, dr-lab logs/server.log; SigNoz not used)
| Row | Grade | Tier | Server evidence | Screenshot | Issue |
|---|---|---|---|---|---|
| H40b | PASS (server clauses; NEEDS_HUMAN items listed) | N1 (real clicks and world right-clicks; .bug and .gotolocation are setup G) | player 111 Fxdhagoauld. 15:23:32.923 fire_player_loaded matched, chain 6125 added dsm 4817 (tmpl 209, dialog 4452) at login into Harset_CmdCenter (world 68). 15:24:00.43 interact Royal Guard (entity 100196) -> onDialogDisplay 4452. 15:24:13.864 dialogButtonChoice 4452 button 170; fire_dialog_choice matched (chain 6122, 4 actions): removed dsm 4817, advanced mission 1200 3584->3585, added dsm 4751 (Anat, dialog 5435) and 6360 (Ba'al, 5436). 15:24:28.82 interact Anat (entity 100118) -> onDialogDisplay 5435. 15:24:36.531 dialogButtonChoice 5435 button 171; fire_dialog_choice matched (chain 6123, 3 actions): removed 4751 and 6360, completing mission 1200, mission_completed (path direct), fire_mission_completed matched. DB sgw_mission: player 111, mission 1200, status 2, completed_step_ids [3584,3585], current_step null. No WARN/ERROR in window. | H40-guard: lab_screenshot-20261103-092416.771.png (not looked at); H40-complete: none taken | none |

## Not run
- Chain 6124 (optional Ba'al objective): NOT-RUN by design.
- Looker pre-screens skipped per coordinator; H40-complete screenshot and the end bookmark `h40 end` were not taken (driver hit its 28-call limit).
- lab_finish_dialog on the Anat dialog was not sent. The Anat choice (button 171) DID reach the server and chain 6123 fired and completed 1200; only the client dialog tail (paging/finish) after the choice never ran. No server row for a finish after 15:24:36 exists, and none is needed for the chain.

## Resume hint
No retry needed for server clauses. Optional: for the visual NEEDS_HUMAN items, redo from START LEG 4 with a fixture row (1200 at 3585) only if a person wants the H40-complete screenshot.

## Recipe
Fixture login into world 68 (Harset_CmdCenter), 1200 at 3584. .bug start; world right-click template 209 at 3.9 m; page Dialog_NextButton to page 5; click Dialog_GenericButton1 (button 170). .gotolocation Harset_CmdCenter 61.97 1.77 73.2; right-click template 43 at 3.97 m; page to page 2; click Dialog_GenericButton1 (button 171).

## SIGN-OFF SHEET
- H40-guard: is a dialog window visible / armoured NPC visible? Driver: ui_state empty after the choice click. Screenshot lab_screenshot-20261103-092416.771.png. Looker: not run.
- H40-complete: dialog closed / mission notice visible? Not captured. Client-read only; server shows mission completed.

## Findings
1. Harness gap, not a product defect: driver reported the Anat click as "posted_only" and stopped, but the server received the choice and completed the mission. Driver reports undercount what happened; always verify from server rows. No issue file.
