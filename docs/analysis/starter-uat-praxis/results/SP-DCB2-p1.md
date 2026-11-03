PACKET SP-DCB2-p1 VERIFIED 2026-11-03 by Sonnet; driver attempts: 1; server evidence window 08:30:02Z-08:33:45Z (player_id 100, entity 50, space 65601; bookmarks 08:30:07 start, 08:33:24 end)

| Row | Result | Tier | Server evidence | Client evidence | Looker | Notes |
|---|---|---|---|---|---|---|
| CB-U08a | NATIVE_SHORTFALL (+ NEEDS_HUMAN 1; AoI second-client clause PARTIAL) | N2 for Prisoner and door (`/interact` typed after target), N3 door target | 638 accept: INFO `Content: accepting mission` mission 638 chain 1011 at 08:30:43.85, plus add_dialog_set for Prisoner 329 (102503) at 08:30:43.85; first `interact` on 102503 at 08:32:20 (after). Region2 entering=true at 08:30:44 (first crossing); no bark then. Prisoner interact 08:32:20 dialog 2300 "Free Prisoner 329", chain 1015 steps 2114->2115 at 08:32:24; door panel 102514 `interact` 08:32:44 (fire_interact_tag matched 329_CellDoorButton); chain 1017 Livewire victory step 2115->2116 at 08:32:50; dialog 2299 then 1019 at 08:33:10: accepting mission 639 "Find Ambernol", 638 complete; end bookmark: 638 status 2, completed_steps 2114/2115/2116, 639 step 2117. Cast: server_entity_query spawn 520 id 102550 at (-271,65.45,-135) tag Cellblock_Ambient_Prisoner1, spawn 521 id 102551 at (-254,65.45,-135), faction 1, idle, velocity 0, DEBUG `Spawned instance NPC from DB` 08:29:08.82. Bark: INFO `Content: npc bark` chain 11010 screen 200160 entity 50 speaker Op-CORE Soldier at 08:33:14.53 (second Region2 entry 08:33:14), exactly one. WARN/ERROR in 08:30:02-08:33:45: none; screen_not_cached / empty_text / actor_not_player: none. server_witnesses 102550: witnessed_by Dcbch (entity 50) only. | Chat/UI lines per driver, not used for grade | Q1 two NPCs in corridor: NO (only one partly visible figure). Q2 name plate above them: NO (names only on HUD frames) | Server clauses held. Tier is below N1 (typed /interact, N3 door target), so NATIVE_SHORTFALL. Only one player in session: second-client witness clause not exercised (PARTIAL); no packet tap taken. Chain 11010 name: "CB-N04a ... enter Region2 during 639 step 2117". |

SIGN-OFF SHEET (NEEDS_HUMAN):
- CB-U08a, H-CB-U08-1 (Op-CORE Soldier name plate above the cast): C:\Users\Steve\AppData\Local\cimmeria-lab\screenshots\lab_screenshot-20261103-023320.655.png. Driver: no Op-CORE soldier models visible; dialog window "Dcbch" with the Ambernol blurb, bark in chat. Looker: only one partly visible orange-clad figure, no second NPC, no name plate over characters. Likely the camera did not frame the cast; a human should look.

FINDINGS:
1. Open question (not graded), for the coordinator: the cast (spawns 520/521, ids 102550/102551) was spawned at world load, 08:29:08.82Z, about 95 s before 638 was accepted (08:30:43.85Z) and 4 min 2 s before 638 completed (08:33:10.65Z). They are not gated on mission 638. Product or doc question; no issue body written.
2. Observation: server_entity_query shows the NPCs with witness_count 0 while server_witnesses lists the player as a witness. Probably the two tools count different directions; not a finding.
No FAIL, no issue files written.

NOT RUN / BLOCKED:
- Second-client AoI clause (both players listed as witnesses, bark to the crossing player only): not exercised, one player only; no packet tap. Next driver needs a second client in Region2 and a packet tap.
- Visual name plate: NEEDS_HUMAN H-CB-U08-1.

RESUME HINT: To reach PASS, redo the Prisoner and door steps with N1 clicks (client_world_click, retry with a camera face first; the driver's mouse-over transport timed out), take a packet tap, bring a second client into Region2, and frame both cast NPCs in the corridor for the name-plate screenshot.
