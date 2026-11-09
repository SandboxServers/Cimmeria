PACKET SP-DCB4-p2 VERIFIED 2026-11-09 by Sonnet; driver attempts: 1 (coordinator-driven, no Haiku); server evidence window 09:20:00-09:22:19 UTC (dr-lab `logs\server.log`; the `server_log_tail` ring was empty) plus DB state
Local lab p2, Fxdcathree (player_id 106, not Dcbeh), carry-over from DCB4-p1. Deviation: the driver entered Region2 first (brief defect, see SP-DCB3-p8.md). Bookmarks seen: T03/T04b 09:20:00.59 to 09:20:14.13; T05/T06a 09:20:15.68 to 09:22:18.98.

| Row | Result | Tier | Server evidence | Client evidence | Looker | Notes |
|---|---|---|---|---|---|---|
| T03/T04b Equip | PASS (+ NEEDS_HUMAN 1) | N1 (driver reports native_cegui Inventory_Slot2; server shows the equip event, not the click) | 09:20:03.167 `fire_item_equipped: matched` item_type 55 "SI 3 9mm Pistol"; `Content: playing sequence` StasisBlockDoubleDoors (chain 1004); `Content: completing mission` 622 "Arm Yourself!"; `Mission completed directly`. DB: sgw_mission 622 status 2, completed_step_ids [2113,80623,80622]; pistol item 10677 type 55 now container 3 (was container 1 in p1), slot 0. | "Mission Complete: Arm Yourself!" in chat (driver) | not run | Stasis door sequence fired. Frost, Guard and Tutorial 5882 belong to p1; context here: 1360 still at step 4037, item 10676 type 3730 in container 2; tutorial 5882 has exactly one row (09:18:44), no 5883. |
| T05/T06a One topic | PASS | G moves, N1 click | `adding dialog set` 2794 "Free Prisoner 329" once at accept (09:20:17.281, chain 1011) and once after the relog (09:21:34.311, chain 1026 "Restore Prisoner 329 Human"). Never two sets in either session. | DialogWin before and after relog (driver) | not run | No `removing dialog set` seen, which is consistent with a one-set-per-session count. |
| T05/T06a Dialog | PASS | N1 | Both interacts: `interact: per-player dialog set -> onDialogDisplay` on Prisoner 329 (09:20:57 id 100963; 09:22:09 id 101012). `dialogButtonChoice` dialog 2300 "Free Prisoner 329" at 09:21:11 and 09:22:18. Human line 2300, never 5021. | DialogWin text (driver) | not run | There is no `Content: displaying dialog` row; the interact row stands in for it and the dialog id 2300 appears in the choice rows. The second (post-relog) close at 09:22:18 produced no step change, which is correct. |
| T05/T06a 638 | PASS | | `Content: accepting mission` 638 once (09:20:17.279, via fire_enter_region Region2, chain 1011); `Content: advancing step` to 2115 at 09:21:11.705 on dialog close. DB: 638 status 1, current_step_id 2115, completed_step_ids [2114]. No second accept after the relog. | | | Accept came from Region2 entry, not from the prisoner. |

SIGN-OFF SHEET (NEEDS_HUMAN):
- T03/T04b equip, H-T04-1 (inventory/splash/tutorial windows visible): C:\Users\Steve\AppData\Local\cimmeria-lab\screenshots\lab_screenshot-20261109-032008.705.png. The driver did not view it. Looker not run.
- T05/T06a, H-T05-1 (dialog text with "symbiote" or "Free Prisoner"): no screenshot path was reported. The driver quoted the DialogWin text "You feel uncomfotable, don't you? ..." and "If you help me first. Get me out of here.". Looker not run.

FINDINGS:
None. No WARN or ERROR in the window except the verifier's own failed read-only SQL calls. No issue bodies written.
Observation (not a defect): the 5040 system message in chain 1013 is logged as "stub - correct wire format TBD" at 09:20:17.

NOT RUN / BLOCKED:
- Jaffa variant (5021, chain 1027) not run; only the Human archetype was exercised (PARTIAL for the archetype coverage of T05/T06a if the coordinator counts it).
- `displaying dialog` rows do not exist in this build; the interact `onDialogDisplay` row was used.

RESUME HINT:
Nothing to redo for Human. A Jaffa pass needs a Jaffa character, and 638 is only accepted by entering Region2 (-287, 65.5, -125), not at the brief's coordinates. If H-T04-1 or H-T05-1 matter, view the screenshot above and take a new DialogWin shot.
