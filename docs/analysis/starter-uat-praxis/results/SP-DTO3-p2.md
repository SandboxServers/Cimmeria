# SP-DTO3-p2 result (verifier)

Character: Fxdcatwo (player id 105), client instance p2, world Tollana. Fixture `lab fixture applied` at 07:46:59.715Z (missions 1, moved true). Evidence from the dr-lab server.log (UTC), two `.bug SPU SP-DTO3-p2` bookmark lines present, and a DB read. No WARN or ERROR of any kind in 07:46-07:50Z; no `dialog_set_maps cache miss` in that window.

| Row | Grade | Tier (N1/N2/N3/G) | Server evidence (query or log event, time) | Screenshot | Issue |
|---|---|---|---|---|---|
| U2a | PASS +NEEDS_HUMAN | N1 (client_world_click native_camera on both props; dialogs closed by DIALOG op; journal key posted only) | Start: fixture applied 07:46:59.715; world entry 07:47:11; `adding dialog set` 4959 (tmpl 420, chain 9008) at 07:47:11.770; no `Mission accepted` or `accepting mission` in the part. Fungus: `fire_interact_tag` Tollana_FoodUnit_Fungus 07:48:33.667, `Content: advancing step` 1965 (chain 9017) 07:48:33.668, `displaying dialog` 4214 07:48:33.669. Nest: `fire_interact_tag` Tollana_FoodUnit_Nest 07:49:01.181, `Content: complete objective` 2312 (chain 9015) 07:49:01.181, `displaying dialog` 4213 07:49:01.182. Only one `adding dialog set` (4959) in the whole window, no second add and no remove, so one bind throughout. Nest was clicked once on the server (the first failed click was a client camera failure, no server row). DB after (coordinator snapshot, re-read by me at verify time, identical): current_step_id 1965, status 1, completed_step_ids [1963,1964], completed_objective_ids [2309,2310,2312], active_objective_ids [2311,2312], repeats 0. Journal: client-read only. | DTO3-U2a-journal-1964 (lab_screenshot-20261109-014822.654.png), DTO3-U2a-journal-1965 (lab_screenshot-20261109-014842.905.png) | none |

Note on the row text: the brief expected "step stays 1965" after the Nest and an unchanged `active_objective_ids`; 2312 is still listed active while also completed (completed_objective_ids has it). That is how the DB reads at both times; the row asks only that completed has 2312 and the step stays 1965, which holds.

## Not run

- Journal check by a real journal window: the journal key (posted) opened a crafting window in both shots (see looker), so the journal window itself was never viewed. Client-read only, NEEDS_HUMAN.
- Dialog text vs seed first line: the driver recorded the dialog text on screen for both props but dialog ids came from the server log, not the client; text comparison against the seed not done here.
- Core is not clicked in this part by design; the 1966 journal (p3) and turn-in (p4) are out of scope.
- Phasing rows U-PH1 to U-PH5: not runnable today.

## Resume hint

Fxdcatwo is at mission 568 step 1965, status 1, objectives 2309, 2310, 2312 completed, 2311 and 2312 listed active, bind 4959 still on. Part p3 (Core) can start from this state without a re-stage; a re-run of p2 needs a fresh fixture apply (step 1964).

## Recipe

- Ids: given (player_id 105).
- Log: Python json scan of dr-lab `logs\server.log` for 2026-11-09T07:46-07:50Z, keeping WARN/ERROR and messages matching fixture, dialog set, advancing, complete objective, displaying dialog, fire_interact_tag. The in-memory server_log_tail ring is too short for past events.
- DB: `server_db_query` on public.sgw_mission for player 105, mission 568.
- Screenshots resolved in %LOCALAPPDATA%\cimmeria-lab\screenshots; two Haiku lookers run (the third shot, the failure capture, was not needed).

## Findings

1. Observation (not a product finding, per coordinator): the driver saw two buttons on both prop dialogs (Dialog_DoneButton, Dialog_PrevButton2) where the brief expected none. Recorded only.
2. U2a / journal, client-read only: both journal shots show a crafting window (Alloy/Craft/Research/Reverse Engineer) rather than a journal, so the key the driver posted is not the journal key (or toggled something else). The quest tracker on the right shows "Finding Tollan Technology" with the objective "Report to Captain Copplemann who is running the operation on Tollana." in BOTH shots, including the one taken at 07:48:42, nine seconds after the server advanced to 1965 (07:48:33). Either the tracker had not refreshed, or the step 1965 text is not shown. The server side is correct (step 1965 in DB). Cannot be settled from one screenshot; goes to a person. No issue body written: not established as a product bug.
3. Lab/driver: the first Nest click failed client-side on_screen (camera could not turn); retry worked. Not a server finding.

## Sign-off sheet

| Row | What to look at | Screenshot path | Driver's words | Looker pre-screen | Your read of the numbers |
|---|---|---|---|---|---|
| U2a | Journal at step 1964 (title and objective) | %LOCALAPPDATA%\cimmeria-lab\screenshots\lab_screenshot-20261109-014822.654.png | "not viewed"; key posted, inconclusive | Q1 NO: crafting window open, only a small tracker at right. Q2 YES: "Finding Tollan Technology" at right edge. Q3 YES: "Report to Captain Copplemann..." | Server at 07:48:22: step 1964, as the tracker text implies. |
| U2a | Journal at step 1965 (objective text should have changed) | %LOCALAPPDATA%\cimmeria-lab\screenshots\lab_screenshot-20261109-014842.905.png | "not viewed"; key posted, inconclusive | Q1 NO: crafting window open. Q2 YES: title at top right of tracker. Q3 YES: same "Report to Captain Copplemann..." line | Server at 07:48:42: step 1965 since 07:48:33. Tracker text looks unchanged: needs a person to check whether the journal updates. |
