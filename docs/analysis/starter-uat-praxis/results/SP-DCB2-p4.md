PACKET SP-DCB2-p4 VERIFIED 2026-11-03 by Sonnet; driver attempts: 1 (this is the third run; the earlier windows 07:59 and 09:05 are superseded); server evidence window 09:51:50Z-09:54:10Z (server.log, lab_run dr-lab, player_id 101, entity 56)

| Row | Result | Tier | Server evidence | Client evidence | Looker | Notes |
|---|---|---|---|---|---|---|
| CB-U05b | PASS (+ NEEDS_HUMAN 1) | G setup, N3 target, N1 shot | 09:52:20.113 INFO `Mission accepted` 686 step 2353 (gmMissionAssign), before the shot. 09:52:22.268 aggro acquired by Hallway05_Guard2 (102818) and Guard1 (102817); 09:52:22.270 and 09:52:24.266 `picked cover slot` (cover hold). 09:52:52.247 `useAbility: launched` ability 592 (Pistol Shot), target and wire_target 102817 (Hallway05_Guard1); no `no_ammo` row in the window. 09:53:26.275 `cover_released_no_shot`, 09:53:28.275 `move_to_cover`, 09:53:30.275 `cover_released_flanked` (102817, chunk 1200049), 09:53:30.276 `fire_player_flanked_npc` matched, 09:53:30.277 `Content: complete objective` 686 / 2731 (chain 1142), 09:53:32.280 `cover_released_flanked` for 102818 (chunk 1200044). No complete objective 2725, no complete mission 686, no play_sequence 1751, no accepting mission 687. DB sgw_mission player 101 mission 686: status 1, current step 2353, completed [2731], active [2730, 2731]; no 687 row. Entities after: 102817 (-103.49,24.8,-38.2) and 102818 (-101.8,24.8,-52.6), both Fighting, 250/250, at their spawn points inside the corridor. | shot hit_type 1 effect_applied (driver) | see sheet | Both guards alive, none moved out. The shot did not reduce health (250/250 after), not required by the row. Only WARN in window: 09:52:18.65 `GM cmd: target id out of u32 range` for `/gmsethealth 760 0` (target_raw -1), setup only, player in god mode. Guards' repeated ability 559 against the god-mode player is expected. No stuck WARN this run. |

SIGN-OFF SHEET (NEEDS_HUMAN):
- CB-U05b, cover posture (H-CB-U05-1): C:\Users\Steve\AppData\Local\cimmeria-lab\screenshots\lab_screenshot-20261103-035336.501.png. Driver: player frame, "NID Guard" target frame, empty inventory window, tracker with three missions, no guard visible in the 3D view. Looker (Haiku): 3D scene YES; NID guard figure in scene NO (portrait in HUD only); guard crouched or in cover NO (no guard visible); pistol held visible NO. The shot was taken at 09:53:36, after the player had sidestepped 15 m and returned, so it cannot confirm cover posture; a human must look at a shot taken during the cover hold (09:52:24-09:53:26).

FINDINGS:
1. None. No issue body written.

NOT RUN / BLOCKED:
- Nothing blocked. Visual cover-posture clause is NEEDS_HUMAN only.

RESUME HINT:
- None for the server clauses. Character state after: mission 686 active at step 2353 with objective 2731 done and 2730 (kill) open; 687 not assigned; both Hallway05 guards alive at 250/250. CB-U06 (SP-DCB2-p5) can proceed. For the posture look, take the screenshot while the guard is in its cover slot and the player faces it from the stand point.
