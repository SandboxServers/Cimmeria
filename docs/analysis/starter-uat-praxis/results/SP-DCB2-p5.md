PACKET SP-DCB2-p5 VERIFIED 2026-11-03 by Sonnet; driver attempts: 1; server evidence window 09:10:20Z-09:13:15Z (dr-lab server.log; player_id 100, Dcbch)

| Row | Result | Tier | Server evidence (time, scope, event, key fields) | Client evidence | Looker | Notes |
|---|---|---|---|---|---|---|
| CB-U06 | BLOCKED (case c, driver recipe) | G | Precondition: `sgw_mission` 686 status=1, step 2353, objectives 2730/2731 active. 09:10:38Z gmMissionAssign 686 refused `already_active` (WARN mission_accept_refused, expected). 09:11:22Z gmKillTarget killed ONE guard, 102622 (Hallway05_Guard2); Guard1 102621 stayed alive and kept firing at the player (09:11:22-09:11:28Z). No `complete mission` 686, no `accepting mission` 687, no Matinee 1751, no `displaying dialog` 2516, no blurb 5859, no Marsh despawn, no chain 1093 kill-increment row anywhere 09:10-09:13Z. After relog (09:12:05Z, entity 53): no 1751/2516/5859; chain 1178 re-barks Marsh at 09:12:06Z (Marsh present, so no despawn happened). | Driver saw nothing; both guards alive after relog | Both shots: 1 NO (normal third-person camera), 2 NO (crafting panel with tabs, not a dialog) | Chains 1092/1093 each increment `hallway05_kills`; chain 1094 completes 686 and accepts 687 when the counter is reached, and chain 1188 needs both guards dead. One kill cannot complete the mission, so the timing clauses never ran. Scene/dialog/blurb/687/late-entry clauses BLOCKED. The relog "no replay" clause held only vacuously. Tier-G setup (see context): Dcbch held pistol item 55 and abilities 592/594/597/1218 via GM grant because the Guard body is not interactable (results\issues\SP-DCB2-p2-1.md). |

SIGN-OFF SHEET (NEEDS_HUMAN):
- H-T16-1 | CB-U06 | NID Guard window (General/Mission/Crafting tabs, empty grid) open over the centre | C:\Users\Steve\AppData\Local\cimmeria-lab\screenshots\lab_screenshot-20261103-031125.911.png | driver: window open after kill, chat shows gmKillTarget line | looker: normal third-person camera, a crafting panel with tabs, no dialog.
- H-CB-U06-2 | CB-U06 | no blurb text; same window still open | C:\Users\Steve\AppData\Local\cimmeria-lab\screenshots\lab_screenshot-20261103-031146.691.png | driver: looks the same | looker: normal camera, crafting panel, no dialog. The scene never played, so H-CB-U06-1 (camera and interval) is not reachable.

FINDINGS:
1. CB-U06: driver killed only Hallway05_Guard2; 686 needs both Hallway05 guards (chains 1092/1093/1094, 1188). Case (c), a driver recipe error, not a product bug. No issue file written. Open product question to check on the rerun: whether gmKillTarget raises the `entity_dead_tag` event the kill chains listen on. No kill-increment row was logged for the one kill (the chain may log nothing on increment, so this is unproven). Check it once both guards are dead.
2. Info: the driver saw 686 missing from the tracker while `sgw_mission` holds it active at step 2353. That is a client display question, not a server fault.
3. No unexpected WARN or ERROR in 09:10-09:13Z other than the expected `already_active` refusal and `GM cmd: target id out of u32 range` (gmSetHealth with target -1, a driver typo).

NOT RUN / BLOCKED: Scene, dialog 2516, blurb 5859, 687 accept and relog late-entry replay check: BLOCKED (case c). Recipe for the next driver: from the carry-over state (686 active, step 2353), kill BOTH Hallway05_Guard1 and Hallway05_Guard2 (`/gmkilltarget 0` on each, retarget between; god mode on, and re-send it after any relog). Note t=0 at the second kill. Then read `complete mission` 686, the Matinee 1751 start, Marsh despawn, dialog 2516 (about +10.1 s), blurb 5859 (about +26.1 s) and `accepting mission` 687 once. Then relog and confirm no replay.

RESUME HINT: Redo CB-U06 with both guards killed. Guard entity ids change per relog (102621/102622 before it, 102670/102671 after), so re-FIND them.


---

# ATTEMPT 2

PACKET SP-DCB2-p5 VERIFIED 2026-11-03 by Sonnet; driver attempts: 2; server evidence window 09:16:00Z-09:18:46Z (dr-lab server.log; player_id 100, Dcbch)

| Row | Result | Tier | Server evidence (time, scope, event, key fields) | Client evidence | Looker | Notes |
|---|---|---|---|---|---|---|
| CB-U06 | FAIL (product, case a) | G | Precondition held: `sgw_mission` 686 status=1, step 2353, active objectives [2730, 2731], completed [2731] (flank objective 2731 completed 09:16:04.29Z, chain 1142). Both guards killed by the right tags: 09:16:32.09Z `gmKillTarget: killing NPC` 102671 (Hallway05_Guard2); 09:17:08.89Z the same for 102670 (Hallway05_Guard1); both `NPC death: respawn not scheduled`. NO `fire_entity_death` row, no `complete mission` 686, no `accepting mission` 687, no Matinee 1751, no `displaying dialog` 2516, no blurb 5859, no Marsh despawn in 09:16-09:19Z. After the relog (09:17:52Z, entity 54, guards respawned as 102719/102720): no 1751/2516/5859; chain 1178 re-barks Marsh at 09:17:53Z (Marsh present). Guard 102719 killed the player at 09:18:12Z (expected, god mode lost on relog). | Driver saw no dialog or blurb, 686 not on tracker | Screenshots not opened by driver; not looked at (NEEDS_HUMAN, scene never played) | Root cause by code reading: gmKillTarget -> kill_npc_out_of_band -> resolve_death never calls the content `entity_death`; only the combat paths (use_ability/kill_credit.rs, pulsing/pulse.rs) do. So `entity_dead_tag` never fires and chains 1092/1093/1094 do not run. Issue body: results\issues\SP-DCB2-p5-1.md. Timing clauses (dialog 2516, blurb 5859) and 687 accept: BLOCKED by this. Late-entry clause held only vacuously. |

SIGN-OFF SHEET (NEEDS_HUMAN):
- H-T16-1 | CB-U06 | state shortly after the second kill (expect plain world, no scene) | C:\Users\Steve\AppData\Local\cimmeria-lab\screenshots\lab_screenshot-20261103-031713.008.png | driver: not opened | looker: not run (server shows no scene).
- H-CB-U06-2 | CB-U06 | about 21 s after the second kill, no dialog or blurb | C:\Users\Steve\AppData\Local\cimmeria-lab\screenshots\lab_screenshot-20261103-031731.013.png | driver: not opened | looker: not run.

FINDINGS:
1. CB-U06: gmKillTarget (tier G) does not raise `entity_dead_tag`, so killing both Hallway05 guards does not complete 686. Product (a). Issue body: docs\analysis\starter-uat-praxis\results\issues\SP-DCB2-p5-1.md.
2. Driver deviation (killed both guards) was correct and is what exposed finding 1. Attempt 1's open question is answered.
3. Only WARN in the window: the expected `gmSetHealth target id out of u32 range` (target -1, driver typo, 09:16:00Z). No ERROR.

NOT RUN / BLOCKED: Scene, dialog 2516 and blurb 5859 timings, 687 accept, real Marsh despawn and late-entry no-replay: BLOCKED behind finding 1. Case (c) does not apply this time.
RESUME HINT: Two options once the issue is decided. Fix route: re-drive with `/gmkilltarget` on both guards. Workaround route: have a player kill both guards with a real weapon (pistol #1667 / bypassing SP-DCB2-p2-1) or drive chain 1094 another way; t=0 at the `complete mission` 686 row. Guard ids change per relog; re-FIND them and re-send god mode after a relog.
