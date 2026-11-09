PACKET SP-DHA3-p3 VERIFIED 2026-11-09 by Sonnet; driver attempts: 2; server evidence window 06:30:52Z-06:33:22Z (bookmarks `SPU SP-DHA3-p3 FZ-U2c start/end`, source: dr-lab server.log)

| Row | Result | Tier | Server evidence (time, event or query, key fields) | Client evidence | Looker | Notes |
|---|---|---|---|---|---|---|
| FZ-U2c fixture | PASS | G | Start bookmark 06:30:52Z, player 106: mission 1664 status 1, step 5226, 6061/6062 status 0 (6059/6060/6063 carried from earlier use, accepted) | MLIST `#1664 active step 5226` (driver shows obj 3/5, accepted baseline) | skipped | Baseline accepted per coordinator |
| FZ-U2c Marsh report | PASS | N1-ish (native_camera, not a confirmed real click) | 06:31:31.348Z bark screen 200180 chain 6018; 06:31:31.350Z `Content: complete objective` 6061 (count in window: 1); dialog set 120302 removed | Chat "Report received. Good work out there." | skipped | Native level for Marsh click 1: native_camera, inconclusive verdict |
| FZ-U2c Re-click | PASS | native/real_input | Second `interaction.pose` 06:31:41Z target 100199 (route fallback); no second complete_objective 6061; 0 lines mentioning 6512 or 4457 in window | No further line | skipped | |
| FZ-U2c Relog restore (6033/6034) | PASS | Typed (lab_logout reported failure #1680) | Server DID relog: `SGWPlayer.logOff` 06:31:49Z (logoff_character_select), world entry 06:31:56-06:32:08Z, entity 8 -> 9; `fire_player_loaded` actions=2: chain 6034 (Moh'katan cue added 06:32:08.523Z) and 6504; chain 6033 never ran (0 hits). missions_restored saved_count 8 | Driver reported no relog happened | skipped | Driver report is wrong on this point: the logout was applied server-side despite the harness error. Relog clause EXERCISED, not BLOCKED |
| FZ-U2c Moh'katan report | PASS | native_camera | 06:33:15.039Z bark screen 200181 chain 6019; 06:33:15.040Z complete objective 6062 (once); `mission_completed` path auto | Chat "Your report is filed." | skipped | |
| FZ-U2c Completion | PASS | server | DB sgw_mission 1664 player 106: status 2, completed_objective_ids [6063,6059,6060,6061,6062], completed_step_ids [5224,5225,5226], current_step_id null; end bookmark shows status 2 | Last MLIST no longer lists 1664 | skipped | |
| FZ-U2c overall | NEEDS_HUMAN | | All server clauses PASS; only cue/look-and-feel clauses remain | | | server clauses PASS |

FINDINGS:
1. Driver report said the relog did not happen. Server shows a full logoff and world re-entry at 06:31:49-06:32:08Z (harness error -32603 was a late bridge timeout; logoff was applied). Lab gap #1680 is a reporting issue, not a missing relog.
2. WARN (unexpected, not mission logic): `Inbound client message dropped` reason unhandled_account_method, msg_id 221 baseMethod, 06:31:54Z (during the relog). Known-class client noise; no issue filed.
3. WARN: slow `INSERT INTO sgw_mission` plus `cell_dispatch_slow` mission_update elapsed 600 ms at 06:33:15.6Z (completion persist). Local lab load; no issue filed.
4. No WARN/ERROR in mission or content targets besides the above.

NOT RUN / BLOCKED: none. (Looker step skipped by instruction.)

RESUME HINT: none needed for server clauses. Human sign-off only. Next driver should not trust lab_logout failure text; check server logOff.

SIGN-OFF SHEET:
FZ-U2c | C-MARSH-CUE | lab_screenshot-20261109-003118.098.png | Marsh has a mission cue before the report | not run
FZ-U2c | C-MARSH-RELOG | lab_screenshot-20261109-003258.929.png | No marker over Marsh after report and relog | not run
FZ-U2c | C-KATAN-CUE | lab_screenshot-20261109-003306.094.png | "?" or marker over Moh'katan after relog | not run
FZ-U2c | C-DONE | lab_screenshot-20261109-003327.872.png | No cues, mission 1664 gone from tracker | not run
