PACKET SP-DHA1-p7 VERIFIED 2026-11-03 by Sonnet; driver attempts: 2 (a1 was a coordinator fixture error, not graded); server evidence window 15:31:38Z-15:32:36Z (local lab server log, dr-lab logs/server.log; SigNoz not used; character Fxdhagoauld, player_id 111, lab instance p2)
| Row | Grade | Tier | Server evidence | Screenshot | Issue |
|---|---|---|---|---|---|
| H41a | PARTIAL (both legs held on the server; leg 2 ran at N3, a NATIVE_SHORTFALL; NEEDS_HUMAN items listed) | Leg 1 N1 (world right-click native_camera, lab_finish_dialog Done; .bug is setup G). Leg 2 N3 (client_item_action fell back to client_ui_lua) | Leg 1: 15:31:38.744 chain 6112 added dsm 3129 (Petbe, tmpl 163) at world entry, mission 742 restored at 2502. 15:32:10.14 interact Petbe (entity 100119) -> per-player dialog set -> onDialogDisplay 2638. 15:32:19.959 dialogButtonChoice 2638 button_id -1 ("closed"); fire_dialog_choice matched (chain 6102, 3 actions): granted item 2819 Jaffa Disguise (item 10659, container set 2 = mission), removed dsm 3129, advanced 742 2502->2503. Leg 2: 15:32:24.369 useItem item 10659 (type 2819); 15:32:24.373 UseInventoryItem fired ItemUsed; fire_item_use matched (chain 6103, 5 actions): advanced 742 2503->2504, displaying dialog 2637 (set_interaction_type actions ran in the same chain; no per-action log rows). A second use at 15:32:27.837 fired ItemUsed with no chain match (step already 2504; expected). DB sgw_mission player 111 mission 742: status 1, completed_step_ids [2502,2503], current_step_id 2504, active objectives [2913,2914,2915]; mission 1200 status 2. No WARN/ERROR in window. | H41-petbe: lab_screenshot-20261103-093203.028.png (not looked at) | none |

## Not run
- Chain 6101 not replayed by design (the 2820 stack is staged by hand). Legs 3 onward (bug baskets) belong to later parts.
- Looker pre-screen skipped per coordinator.

## Resume hint
Nothing to redo for legs 1 and 2. The character is now at 742 step 2504 (not 2503). A later part that needs step 2504 can start from here; for a re-run of leg 2 at N1, restage 742 at 2502 and repeat from START LEG 41.1.

## Recipe
Fixture (server_stage_fixture): Fxdhagoauld, world Harset, position -165.5 -41.27 95.4, mission 1200 completed, 742 at step 2502, three 2820 items in container "mission". lab_ensure_in_world (world 57). .bug "SPU SP-DHA1-p7 h41 start". Right-click template 163 (Petbe) via client_world_click at about 4 m (N1). Page Next four times, lab_finish_dialog (Done on page 5): this sends the close with button -1, which fires chain 6102. Inventory shows Jaffa Disguise in the Mission container slot 2. client_item_action rightclick on it (fell back to N3) reached the server as useItem and fired chain 6103. .bug "SPU SP-DHA1-p7 h41 a end".

## SIGN-OFF SHEET
- H41-petbe: is an NPC standing near the player, and is a dialog open? Driver: Petbe dialog on screen, 5 pages, no choice button. Screenshot lab_screenshot-20261103-093203.028.png. Looker: not run. Server shows Petbe was the target and dialog 2638 was displayed.

## Findings
1. Driver's "no choice button" is explained by the seed, not a fixture error: the disguise came from chain 6102, which fires on `dialog_choice 2638` and was triggered by the dialog close (button_id -1, "closed"), not by a choice click. The chain header says it mirrors the script node subscribing to `dialog.choice::2638`; whether the original dialog had a visible choice button is not established here. Client-read only for the button absence. Treated as an observation, no issue written (no evidence the behaviour is wrong; the mission advanced as designed).
2. Leg 2 "item not used" was the client-side result only: client_item_action reported effect_ok false (N3 Lua fallback), but the server received useItem, fired ItemUsed and matched chain 6103 at 15:32:24Z. The disguise is not consumed by design ("no consumption — chain decides"; chain 6103 has no remove_item). Harness gap (item_action verdict undercounts), not a product defect. No issue file.
3. Grade note: the row text says PARTIAL for an N3 use; the vocabulary term for the same condition is NATIVE_SHORTFALL. Server clauses all held.
