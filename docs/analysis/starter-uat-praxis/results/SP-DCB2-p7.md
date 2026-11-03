# SP-DCB2-p7 results (CB-U07b)

PACKET SP-DCB2-p7 VERIFIED 2026-11-03 by Sonnet; driver attempts: 1; server evidence window 12:32:08Z-12:35:42Z (source: dr-lab server.log, JSON lines)

| Row | Result | Tier | Server evidence (time, scope, event, key fields) | Client evidence | Looker | Notes |
|---|---|---|---|---|---|---|
| CB-U07b (Exit) | PASS (+ NEEDS_HUMAN 1) | N1 (client_world_click on Ring Transport Controls 103556); G for goto and bug bookmarks | 12:32:36.982Z `interact` target 103556; `fire_interact_tag: matched` tag Cellblock_ArmoryRingSwitch; `Content: completing mission` mission 688 chain 1109 ("688 - Interact armory ring (step 80688): complete mission, cross-world teleport to Castle"); 12:32:36.983Z `cross-world teleporting entity` world Castle; `interaction_pose` route chain_tag. DB: sgw_mission 688 status 2, completed steps 2356 and 80688. Region Castle.ArmoryRingDropZone entered 12:32:41Z. | Click verdict confirmed; world_id 8 after change | see sheet | Chain 1109 ran from the click itself, no minigame needed. The two client_minigame_play failures are harness noise, not a defect and not a clause of this row. |
| CB-U07b (1360 and Frost's Letter) | PASS | N1 (no step of its own; verified after the transfer and relog) | 12:32:39.768Z and 12:34:18.436Z `missions_restored` (entities 68 and 70, world Castle): active includes 1360:4037. DB now: sgw_mission 1360 status 1, step 4037. Accepted earlier at 09:37:50Z (chain 1121, dialog 3995), so not re-accepted. sgw_inventory: exactly one item type 3730 (item 10587, container 2). No `Content: granting item` for player 101 in the window, so no second reward. | Tracker lists Frost's Letter before and after relog | n/a | Item 3730 was read at verify time; the log has no grant row for it inside this window, which fits "no second reward". |
| CB-U07b (Crate once) | NOT-RUN in this part | - | No `granting item` row in the window | - | - | Belongs to an earlier sub-row; grade there. |
| CB-U07b (Terminal) | NOT-RUN in this part | - | 688 was already at step 80688 per p6 | - | - | Same. |

No WARN or ERROR rows in the window.

SIGN-OFF SHEET (NEEDS_HUMAN):
- H-CB-U07-1, CB-U07b arrival visual, lab_screenshot-20261103-063514.006.png. Driver did not open the PNG. Looker: Q1 outdoor or fortress hall: YES, indoor metal hall with hazard stripes, no outdoor sky visible. Q2 character visible: YES, seen from behind in an orange coat at screen centre. A human should confirm this is the Castle arrival area (server says world Castle, ArmoryRingDropZone, at 12:35Z).

FINDINGS: none.

NOT RUN / BLOCKED: Crate and terminal clauses were not part of this drive. The optional armory kill was skipped (a record, not a grade).

RESUME HINT: none for this part. Crate and terminal clauses need their own sub-row evidence.
