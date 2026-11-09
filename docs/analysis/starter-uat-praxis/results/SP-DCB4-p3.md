PACKET SP-DCB4-p3 VERIFIED 2026-11-09 by Sonnet; driver attempts: 1 (coordinator-driven, no Haiku); server evidence window 09:23:45-09:25:15 UTC (dr-lab `logs\server.log`; the `server_log_tail` ring was not used) plus DB state
Local lab, Jaffa "Uat Fxdhajaffa" (player_id 110, archetype 8), fixture-staged (not Dcbej, not fresh). Bookmarks: T03/T04c start 09:23:50.679, end 09:24:54.721.

| Row | Result | Tier | Server evidence | Client evidence | Looker | Notes |
|---|---|---|---|---|---|---|
| T03/T04c Frost | PASS | N1 (driver: client_world_click right) | 09:24:01.673 `interact: per-player dialog set -> onDialogDisplay` target 101070 dialog 3995 "Search Cpl. Frost's Corpse"; `fire_dialog_open: matched` 3995; `Content: granting item` 3730 "Frost's Letter" (chain 1003); `Content: accepting mission` 1360 (chain 1121); `advancing step` 622 -> 80623. DB: sgw_mission 1360 status 1, current_step 4037. `dialogButtonChoice` 3995 at 09:24:05.5. | DialogWin (driver) | not run | No `displaying dialog` rows in this build; interact row stands in. Item 3730 `grant_container_chosen`; no instance table found to read the container, so the grant row is the evidence. |
| T03/T04c Guard | PASS | N1 | 09:24:42.770 interact on 101067 dialog 3996 "Search the Guard's Corpse"; `granting item` 55 "SI 3 9mm Pistol" (chain 1005); `advancing step` 622 -> 80622; four `content ability grant: learned` rows (09:24:42.7916-.7925) for abilities 592, 594, 597, 1218. DB sgw_player_ability_grants has exactly those four (source tutorial, mission 622). | Four "You have learned" lines in chat (driver) | not run | Fixture started with none known, so four grants is correct. First right-click failed before the camera yaw shift (driver); no server row for it. K5 not applicable. |
| T03/T04c Tutorial 5882 | PASS | server | `RecordTutorialShown: first time ... recorded` and `show_tutorial: ... shown` at 09:24:42.793-.794. DB sgw_player_tutorials: 5882 once (09:24:42.79), no 5883 (no NPC attack in window). | TutorialWin (driver) | not run | |
| T03/T04c Equip | NATIVE_SHORTFALL (server held) | driver reports N1 via client_item_action; brief rates that tool N3 | 09:24:49.719 `fire_item_equipped: matched` item 55; `Content: playing sequence` StasisBlockDoubleDoors (chain 1004); `Content: completing mission` 622; `Mission completed directly`. DB: 622 status 2, completed_step_ids {2113,80623,80622}. | "Mission Complete: Arm Yourself!" (driver) | not run | Not a drag_drop N1 equip; the server cannot tell the tier. Server clauses all held. |

SIGN-OFF SHEET (NEEDS_HUMAN):
- T03/T04c, H-T03-2 (inventory, splash and tutorial windows after the equip): C:\Users\Steve\AppData\Local\cimmeria-lab\screenshots\lab_screenshot-20261109-032453.195.png. The driver listed InventoryWin, SplashTextWin and TutorialWin in the window list but did not view it. Looker not run (not requested). H-T03-1 had no screenshot.

FINDINGS:
None filed. WARN `tree_missing` / `archetype_has_no_tree` for Jaffa (archetype 8, player 110) at 09:24:49.724, .731 and 09:25:00.417, on equip and holster appearance refreshes. This is the known Jaffa ability-tree gap, also seen in SP-DCA5, DCA7-p1 and p2 and not filed there; outside this row. No issue bodies written.

NOT RUN / BLOCKED:
- Fresh-character start and BOOT were skipped (fixture, tier G start), so the clauses about item 55 and 3730 absent beforehand rest on the fixture note, not a server row.
- Container of items 3730 and 55 not read from the DB (no item instance table found by name).

RESUME HINT:
Nothing server-side to redo. If tier N1 on the equip matters, redo it with client_drag_drop. View the H-T03-2 screenshot for the sign-off.
