PACKET SP-DCB1-p7 VERIFIED 2026-11-03 by Sonnet; driver attempts: 1; server evidence window 07:16:59Z-07:20:09Z

| Row | Grade | Tier (N1/N2/N3/G) | Server evidence (query or log event, time) | Screenshot | Issue |
|---|---|---|---|---|---|
| CB-U03b | PARTIAL (driver claimed FAIL; downgraded: lab/driver click problem, not a product contradiction) | N1 (G for .gotolocation and .bug) | Restore chain 2114: chain 1026 "Content: adding dialog set" for entity 44 at 07:16:15 and, after relog, entity 45 at 07:18:28 (one set each, Free Prisoner 329). Dialog 2300 (Human) shown via "interact: per-player dialog set -> onDialogDisplay" at 07:17:29 (target 102160) and again after relog at 07:19:36 (target 102209), first press each time; dialogButtonChoice 2300 button -1 at 07:17:40 and 07:19:49. Mission 638 state at start bookmark (07:17:04) already step 2115 with 2114 complete; no "accepting mission" row in window, so the accept count (1) is not provable here. No WARN/ERROR in window. Door button (step 2115, chain 1028 Livewire), 2116, 2144, 2145: no server rows (click never reached server). | H-CB-U03-1: C:\Users\Steve\AppData\Local\cimmeria-lab\screenshots\lab_screenshot-20261103-011941.049.png | none (lab/driver) |

Looker (Haiku) on H-CB-U03-1: Prisoner 329 dialog open with "If you help me first. Get me out of here." YES; med station or cell door visible NO (walls, railing, floor only); image blank NO. So the med-station area clause is NEEDS_HUMAN.

SIGN-OFF SHEET (NEEDS_HUMAN): CB-U03b, med-station area visual, H-CB-U03-1, path above. Driver saw the Prisoner 329 dialog; looker saw the same dialog and no med station.

## Not run
- Door button click (step 2115, Livewire chain 1028): client_world_click failed twice at hover, "off screen or covered" for entity 102171 (2.06 m, no name, candidate_ids empty). Lab/driver problem, not a product bug.
- Steps 2116 (dialogs 2299/5020, 639 accepted), 2144 (drone chain 1037), 2145 (vial chain 1036): not reached. Jaffa (chain 1027, dialog 5021) not driven.
- 638 accepted-once count: accept happened before this window (p6 carry-over); not checkable here.

## Resume hint
Next driver: after relog, stand where the door button (about 2 m, y 67.26, no name) is on screen: lower the camera pitch or stand 4-6 m back facing it, then click with client_world_click (or force=true if it still reports covered). Verify a Livewire start row (chain 1028) at the first press, then play the minigame and continue to steps 2116, 2144, 2145. Check the 638 accept count with signoz_aggregate_logs on body CONTAINS 'accepting mission' for player Dcbbh over the whole p6+p7 run.

## Recipe
1. Carry-over char Dcbbh (player_id 98) at the Cellblock, mission 638 step 2115. Send `.bug SPU SP-DCB1-p7 CB-U03b start`.
2. RCLICK Prisoner 329, ACCEPT through Next x4 then Done (server: dialogButtonChoice 2300).
3. RCLICK the door button; expect Livewire start (chain 1028). This step failed to reach the server.
4. Relog; RCLICK Prisoner 329 again; expect one dialog set (chain 1026) and dialog 2300 on the first press.
5. Send `.bug ... end`; verify via SigNoz filter resource.cimmeria.lab_run = 'dr-lab' AND attribute.note CONTAINS 'SPU SP-DCB1-p7'.