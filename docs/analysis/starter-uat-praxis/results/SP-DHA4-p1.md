# SP-DHA4-p1 result (FZ-U2d)

PACKET SP-DHA4-p1 VERIFIED 2026-11-09 by Sonnet; driver attempts: 1; server evidence window 06:35:17Z-06:37:29Z (bookmarks `SPU SP-DHA4-p1 FZ-U2d start/end`, lab run dr-lab server.log). Character Fxdcatwo, player_id 105. Mission rows carried leftovers; baseline 1664 at step 5226 with 6061/6062 uncredited.

| Row | Result | Tier | Server evidence | Client evidence | Looker | Notes |
|---|---|---|---|---|---|---|
| FZ-U2d fixture | PASS | G | Driver read 1664 active at step 5226 before the start bookmark; 6033 and 6034 dialog sets added at login (06:34:26Z) | gmMissionList seq 4647049 | skipped | Leftover mission 1361 (active, step 4042) is not a fixture effect |
| FZ-U2d Moh'katan first | PASS | N1/inconclusive native result, typed `.gotolocation` G | 06:35:50.76Z chain 6019: npc bark screen 200181, dialog set 120303 removed, `complete objective` 6062 for mission 1664. Exactly one 6062 credit in window | Chat "Your report is filed." seq 4647789 | skipped | Side effect: mission 1361 advanced 4042 to 4043 at 06:35:45Z (leftover, not graded) |
| FZ-U2d re-click | PASS | N1 | No second chain 6019, bark 200181 or 6062 credit after 06:35:50Z | Driver saw one new filed line (the other was old ring history) | skipped | Judged from server chain logs |
| FZ-U2d relog restore | PASS | N1 | After relog (entity 10 to 11) at 06:36:10Z chain 6033 added the Marsh dialog set 120302; chain 6034 (Moh'katan cue) did NOT run | lab_logout then ensure_in_world confirmed | skipped | |
| FZ-U2d Marsh second | PASS | N1/inconclusive native result | 06:37:23.38Z chain 6018: bark screen 200180, dialog set removed, `complete objective` 6061, `mission_completed` for 1664. DB: status 2, current_step_id null, completed_step_ids 5224-5226, completed_objective_ids include 6061 and 6062 | Chat "Report received. Good work out there." seq 4650273 | skipped | `active_objective_ids` still lists 6061 and 6062 on the completed row (leftover; see Findings) |
| FZ-U2d cues (D-KATAN-CUE, D-KATAN-RELOG, D-MARSH-CUE, D-DONE) | NEEDS_HUMAN | n/a | All server clauses PASS | Screenshots below | skipped | Look-and-feel only |

FINDINGS:
1. No WARN or ERROR lines in the window.
2. The completed 1664 row still shows 6061/6062 in `active_objective_ids`. Probably leftover from earlier use of this character; not filed (cannot separate from the leftover state).

NOT RUN / BLOCKED: none.

RESUME HINT: none; the row is complete. Driver counted two "filed" lines, but one was old ring history.

SIGN-OFF SHEET:
FZ-U2d | D-KATAN-CUE | lab_screenshot-20261109-003537.538.png | Moh'katan debrief cue (name plate, report prompt) at the NPC | not run
FZ-U2d | D-KATAN-RELOG | lab_screenshot-20261109-003656.195.png | After relog, Marsh cue shown and no Moh'katan cue | not run
FZ-U2d | D-MARSH-CUE | lab_screenshot-20261109-003717.710.png | Marsh debrief cue at the NPC | not run
FZ-U2d | D-DONE | lab_screenshot-20261109-003734.861.png | Debriefing completed (journal or tracker line, no cues left) | not run
