PACKET SP-DCB4-p1 VERIFIED 2026-11-09 by Sonnet; driver attempts: 1 (Haiku a1 + coordinator takeover of the Guard click); server evidence window 09:16-09:19 UTC (DB state only; log ring empty)
Local lab p2, Fxdcathree (player_id 106, not Dcbeh), staged by fixture (689 completed, 622 at step 2113); no BOOT.

| Row | Result | Tier | Server evidence | Client evidence | Looker | Notes |
|---|---|---|---|---|---|---|
| T03/T04a Frost | PARTIAL | N1 | sgw_mission 1360 status 1, current_step 4037 (clause held). sgw_inventory item 10676 type 3730 in container 2 (mission inventory) (clause held). `displaying dialog` 3995 NOT verifiable: server_log_tail ring returned 0 entries and no SigNoz query was run. | DialogWin "Corporal Frost" opened by right-click (driver claim only) | not run | Server state matches; only the dialog log row is unverified. |
| T03/T04a Guard | PARTIAL | N1 (coordinator takeover) | sgw_mission 622 status 1, current_step 80622, completed_step_ids [2113,80623], completed objectives [2452,3238,90623] (held). sgw_inventory item 10677 type_id 55, container 1 slot 1 (pistol in backpack; design id 55 confirmed, client id 10677 is the item_id) (held). sgw_player_ability_grants: 4 rows (592, 594, 597, 1218), source tutorial/622, all at 09:18:44.585 (four grants held; names and the `You have learned` log lines not verified). `displaying dialog` 3996 not verifiable (empty ring). | SplashTextWin, DialogWin and TutorialWin opened (driver claim) | not run | Grant timestamp 09:18:44 matches the driver's ~09:18:35-09:18:49 click. Only the log rows are unverified. |
| Tutorial 5882 | PASS | server | sgw_player_tutorials: exactly one row, tutorial_id 5882, shown_at 09:18:44.59; no 5883 row (correct, no NPC attack). | TutorialWin open | not run | Once only, held. |
| Equip | NOT-RUN | | none | | | Not driven (per coordinator). 622 stays at step 80622, door not opened. |

SIGN-OFF SHEET (NEEDS_HUMAN):
- T03/T04a Guard, H-T03-1 (bag/pistol window and tutorial window visible): C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\sp-integration\lab_screenshot-20261109-031849.055.png (path as named by the driver; the driver did not view it and said the dialog and tutorial windows may have been closed before the shot). Looker not run.

FINDINGS:
None. No WARN/ERROR scope could be checked (log ring empty). No issue bodies written.

NOT RUN / BLOCKED:
- Equip (622 completion, stasis door): not driven.
- `displaying dialog` 3995/3996 and `You have learned` log lines: unverifiable, ring empty by the time of the check (see verify-server-evidence.md). Re-grade from SigNoz (`resource.cimmeria.lab_run`, filter `attribute.note CONTAINS 'SPU SP-DCB4-p1'`, 09:16-09:19Z) if the coordinator wants those rows closed to PASS.

RESUME HINT:
Nothing to redo for the server state. A rerun only needs the Equip step (drag the pistol to the weapon slot, N1 client_drag_drop) and a SigNoz pass for the dialog and learned-line rows. Recipe for the Guard click: stand at about (-319,73.47,-209.8), client_camera pitch -10 facing the entity, yaw so the avatar does not cover the body, then client_world_click right with rotate_camera false.
