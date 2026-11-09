# SP-DTO4-p1 result (instance p2, character Fxdcatwo, player_id 105)

| Row | Grade | Tier (N1/N2/N3/G) | Server evidence (query or log event, time) | Screenshot | Issue |
|---|---|---|---|---|---|
| T18 | PASS | N1 (client_world_click); setup G | Fixture applied 08:01:21Z; offer bind 2656 added 08:01:27Z (chain 9007). Two `Content: displaying dialog` 2153 (chain 9001) at 08:02:40.50Z and 08:02:45.85Z (gap 5.3 s, gate exercised). One `fire_dialog_choice` 2153 at 08:02:54.197Z, one `Content: accepting mission` 08:02:54.198Z, one `Mission accepted` 08:02:54.198Z, one `Mission step advanced` 1963->1964 (chain 9004). `removing dialog set` 2656 08:02:54.197Z then `adding dialog set` 4959 08:02:54.199Z (chain 9006), once each. DB: step 1964, status 1, completed_step_ids [1963], repeats 0. No WARN/ERROR, no `dialog_set_maps cache miss`. | none | none |
| U7 | BLOCKED (journal not opened; tracker half pre-screened) | N1 key (posted_only) | DB: step 1964 status 1. Seed `resources.mission_steps` mission 568: step_enabled is false for 1963-1966 (1964 included). Journal text not read; key `j` opened CraftingWin. | lab_screenshot-20261109-020304.363.png | none |

Notes on T18: the driver's second click was refused client-side ("did not produce the expected Any within 1200 ms") yet the server logged a second interact and a second display of 2153, so the second click did reach the server (the tool retried the view once). Both clicks are 5.3 s apart, slower than a human double-click (D-TW11 caveat). The driver closed the dialog from page 1 of 5; the accept fires on close of 2153 (chain 9004), which is the single accept.

## Not run
- U7 journal clause: the journal window never opened (`j` toggled CraftingWin or is unbound to the journal); no journal objective text for 2310 was read. Next driver: open the journal from the HUD button or find the real key via client_ui_clickables, and close CraftingWin first (it may still be open).
- U5a, U5b, U6: parts SP-DTO4-p2 and p3. Abandon, turn-in, silent completed state, first-offer dialog text, prop dialogs, relog rows (SP-DTO3), phasing U-PH1..U-PH5: known gaps.

## Resume hint
Mission 568 is active at step 1964 (status 1, completed [1963]) as of the end of this part. The live DB has since moved: a later part staged step 1965 at 08:03:47Z. Re-run T18 from fixture `state: not_active`, position near Copplemann (211.0, 0.6, -547.6).

## Recipe
- Logs: server_log_tail is empty/ring-limited; used `Select-String` on `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\dr-lab\logs\server.log` (UTC) for `SPU SP-DTO4-p1`, `displaying dialog`, `dialog set`, `Mission accepted`, `fixture`.
- `server_db_query`: `select ... from public.sgw_mission where player_id=105 and mission_id=568`; `select * from resources.mission_steps where mission_id=568 order by step_id`.
- Looker (Haiku) on the screenshot path.

## Findings
1. U7 (observation, harness): `j` opened CraftingWin, not a journal. `docs/analysis/union-beta-e1-continuation/uat/_driver-common.md` line 70 marks "J toggles the mission journal" as an ASSUMPTION; no code or doc supports a product issue. Treat as a harness/key-binding gap; no issue body written.
2. U7 (observation): the HUD mission tracker shows "Search the nearby Repair Facility for any traces of advanced technology." at step 1964 while seed step_enabled is false (looker, tracker only, not the journal).
3. T18 (observation): driver reported click 2 as FAILED client-side while the server received it; the driver's raw verdict is not evidence of a missing click.
No WARN or ERROR in the window.

## Sign-off sheet

| Row | What to look at | Screenshot path | Driver's words | Looker pre-screen | Your read of the numbers |
|---|---|---|---|---|---|
| U7 | Journal shows objective for 2310 though step_enabled is false | C:\Users\Steve\AppData\Local\cimmeria-lab\screenshots\lab_screenshot-20261109-020304.363.png | "journal key j posted (posted_only, inconclusive); no journal window but CraftingWin appeared" | Q1 NO (crafting window open, not journal); Q2 YES (objective line under "Finding Tollon Technology"); Q3 "Search the nearby Repair Facility for any traces of advanced technology." | Step 1964 active; seed step_enabled false for all four 568 steps. The visible text is the HUD tracker, not the journal. |
| T18 | Human double-click vs two tool calls | none | two clicks 5.3 s apart | n/a | One accept, one advance; a true human double-click may differ. |