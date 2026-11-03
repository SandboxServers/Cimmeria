# SP-DCB1-p8 result (CB-U03c, Dcbbh)

PACKET SP-DCB1-p8 VERIFIED 2026-11-03 by Sonnet; driver attempts: 1; server evidence window 07:23:18Z-07:26:02Z (bookmarks `SPU SP-DCB1-p8 CB-U03c start` 07:23:23Z, `end` 07:25:42Z, player_id 98; source: `dr-lab` logs/server.log).

| Row | Grade | Tier (N1/N2/N3/G) | Server evidence | Screenshot | Issue |
|---|---|---|---|---|---|
| S1 login/WAIT | PASS | G | start bookmark 07:23:23Z for player 98 in Castle_CellBlock | none | none |
| S2 bookmark start | PASS | G | `playtest bookmark` CB-U03c start 07:23:23.955Z. Snapshot: mission 638 at step 2115 (objective 2453 done, step 2114 done); no 639 in the visible part | none | none |
| CB-U03c.1 GOTO drone | PARTIAL | G | no server row for the teleport itself; only the driver's positions | none | none |
| CB-U03c.2 FIND drone | PARTIAL | N1 | id 102261 appears among the bookmark's nearby entities; no other server row | none | none |
| CB-U03c.3 TCLICK drone | BLOCKED | N1 | no `interact`/target row for 102261 in the window; client harness failed (read_mouse_over, then hover_timeout). Case (c) | lab_screenshot-20261103-012401.713.png | none |
| CB-U03c.4 CHATX /gmkilltarget | NOT-RUN | G | no target selected; no kill, no `complete objective` row in window | none | none |
| CB-U03c.5 GOTO vial | PARTIAL | G | no server row for the teleport itself | none | none |
| CB-U03c.6 FIND vial | PARTIAL | N1 | id 102263 "Ambernol" (template 18) resolves on the server | none | none |
| CB-U03c.7 RCLICK vial | BLOCKED | N1 | 07:24:33.678Z `interact` entity 45 -> 102263 "Ambernol"; `interaction_type: None`, "no per-player interactions for template" (18), "no static interaction type". The click reached the server but the pickup step (2145, chain 1036) was never active: no drone kill, and 638 sat at step 2115. Precondition unmet (case c), not a product bug | lab_screenshot-20261103-012436.780.png | none |
| CB-U03c.8 LOOT_ALL | NOT-RUN | N1 | no `granting item` row in window | none | none |
| CB-U03c.9 INV before relog | NOT-RUN | N1 | none | none | none |
| CB-U03c.10 RELOG / restore chain 2114 | PASS | G | 07:24:54.394Z `missions_restored` (entity 46, saved_count 5); 07:24:54.398Z `Content: adding dialog set` map 2794 "Free Prisoner 329", chain 1026, exactly once; no duplicate set. No WARN or ERROR in the window | none | none |
| CB-U03c.11 INV after relog | PARTIAL | N1 | no `granting item` row in window, consistent with no vial; client-side read only | none | none |
| CB-U03c.12 UIS tracker | PARTIAL | N1 | client-side read only; the server snapshot agrees 638 is open | none | none |
| CB-U03c.13 READ c8 channel 9 | NOT-RUN | G | driver got 0 lines (hook installed late, gap true) | none | none |
| CB-U03c.14 SHOT-H H-CB-U03-2 | NEEDS_HUMAN | N1 | none; looker not run | lab_screenshot-20261103-012540.730.png | none |
| CB-U03c.15 bookmark end | PASS | G | CB-U03c end 07:25:42.873Z | none | none |

Packet-level clauses from the brief:

- Restore chain 2114: PASS for the dialog-set add (once, chain 1026). The first-press `displaying dialog` 2300 was not pressed, so it is untested.
- 638 accepted once, steps 2115/2116/2144/2145: not exercised. No `accepting mission`, `advancing step` or `complete objective` row in the window. Steps 2144/2145 FAIL nothing; they are BLOCKED.
- Overall CB-U03c (steps 5-7): BLOCKED (precondition and harness), with the relog restore clause PASS. Screenshots are paths only; no looker was run.

## Not run

- .3 to .9: the drone was never targeted (camera and hover failures at the harness, case c), so the kill, the vial pickup and the pre-relog inventory never ran.
- The start snapshot shows 638 still at step 2115 (door panel not done) and no 639, so the character was not at step 2144/2145 when the packet began. Carry-over from SP-DCB1-p7 did not reach step 2116.
- .13: empty chat capture (hook installed after the UI Lua state began).
- H-CB-U03-2 sign-off: `C:\Users\Steve\AppData\Local\cimmeria-lab\screenshots\lab_screenshot-20261103-012540.730.png`. Is a vial icon visible in the med-station area? The driver did not judge it.

## Resume hint

Before re-driving: bring Dcbbh to mission 639 step 2144 (finish the 2115 door panel and the 2116 dialogs 2299/5020; confirm `accepting mission` 639 and `Content: advancing step` rows in the server log). Then bookmark, go within 4-6 m of the drone, clear dialogs, left-click it and confirm a target row. Run `/gmkilltarget`, wait for `complete objective`, then right-click the vial from 3-5 m. Re-check the camera turn; if `on_screen` or `hover_timeout` repeats, add a `.gotolocation` closer to a facing position first. Relog and check `adding dialog set` once and `displaying dialog` 2300 on the first press of Prisoner 329.

## Recipe

1. `lab_ensure_in_world` as Dcbbh; bookmark `.bug SPU SP-DCB1-p8 CB-U03c start`.
2. `.gotolocation` next to the drone (about -224, 65, -121 server); `client_entity_find`; `client_world_click` left; `.gmkilltarget` via chat.
3. `.gotolocation` to the vial (about -231, 66, -124); `client_world_click` right on the vial; `client_inventory`.
4. `lab_logout` then `lab_ensure_in_world`; check `client_inventory` and the tracker; bookmark `end`.
5. Grade from `logs/server.log` (or SigNoz with filter `attribute.note CONTAINS 'SPU SP-DCB1-p8'`); `server_log_tail` is too short.