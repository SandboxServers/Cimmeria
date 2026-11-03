# SP-DCB2-p1 result (CB-U08a)

Verified 2026-11-03 by Sonnet. Driver attempts: 1. Server evidence window 07:40:53Z-07:43:44Z (bookmarks `SPU SP-DCB2-p1 CB-U08a start` 07:40:58Z and `end` 07:43:24Z, player Dcbch, player_id 99, entity 47), read from `dr-lab\logs\server.log`.

| Row | Result | Tier | Server evidence | Client evidence | Looker | Notes |
|---|---|---|---|---|---|---|
| CB-U08a Cast placement | PARTIAL | N1 (boot prop), N3 (door panel) | `instance_populated` 07:40:07Z Castle_CellBlock record_count 49, spawned 49, failed 0. End bookmark lists entities 102403 and 102404 (`Castle_OpCoreSoldier_Unarmed`, tags `Cellblock_Ambient_Prisoner1` and `_Prisoner2`, template 176) near the tester. Friendly and stationary not checked (the entity query result was too large to read); DEBUG spawn lines are not in the file | Driver find: soldiers at (-271,65.45,-135) and (-254,65.45,-135), matching the expected positions | not run | Cast was already present with mission 638 not completed (see Notes below). AoI via `server_witnesses` not run |
| CB-U08a Bark (chain 11010 / screen 200160) | NOT-RUN | N1 | No `Content: npc bark` line anywhere in the window. Region2 entry at 07:43:11.957Z matched 3 actions (`fire_enter_region`, entity 47). Chain 11010 is gated on Region2 entry during mission 639 step 2117 (`db/resources/castle_cellblock_chains.sql:3471`). End bookmark missions: 689 (complete), 638 (step 2114), 622 (step 2113); mission 639 is absent, so the gate was not met | none | n/a | A bark was never expected for this drive, so its absence is not a defect and also proves nothing |
| CB-U08a WARN absent | PASS | - | No WARN or ERROR lines (excluding h2/rmcp) between 07:40:00Z and 07:44:00Z; `screen_not_cached`, `empty_text`, `actor_not_player` absent | none | n/a | |
| CB-U08a Second client | NOT-RUN | - | no second client or witnesses read | none | n/a | Out of this sub-row's drive |
| H-CB-U08-1 name plate | NEEDS_HUMAN | - | n/a | Driver: no Op-CORE figures visible on screen (both off screen) | not run | The shot cannot show the name plate |

## Wrong order, case (c): a brief defect, not a product bug

- Mission 638 is accepted by entering Region2: `Content: accepting mission` 638 (chain 1011, "638 - Region2 entry: accept (non-Jaffa)") at 07:43:11.971Z, step 2114 anchored 07:43:11.972Z, dialog set "Free Prisoner 329" added at 07:43:11.972Z. That happened when the driver's `.10 GOTO -265 65.45 -135` teleport crossed the region.
- The driver's steps .4 to .9 (right-click Prisoner 329, ACCEPT, door button, minigame) all ran BEFORE the Region2 crossing, so there was no dialog set and no mission to accept. "No dialog open", "nothing to accept" and "no Livewire opened" are the expected server behaviour at that point, not a failure.
- The brief orders the steps as if Prisoner 329's talk, the door minigame and the 638 completion come before the cast. They need the player to cross Region2 first, on foot or by a GOTO into Region2, and the brief omits that step.
- The row says the cast appears after mission 638 completes, yet the cast was already spawned at world load (49 of 49 instance NPCs at 07:40:07Z) and 638 was still at step 2114. Either the "after 638" wording in the brief is stale, or the cast is meant to be gated and is not (the second would be a product or doc question, not decided here). Check the Cellblock_Ambient spawn conditions in the seed.
- Other lab problems: a driver deviation (extra `/interact`, `client_target` fallback), `client_minigame_play` returned inconclusive `server_unreachable` even though `smartfox_victory` was true. The 689 win still reached the server (`fire_chain_by_id` 1025, mission 689 completed 07:41:19.927Z, 07:41:19Z "Disable Your Prison Boot"), so the boot win is verified server-side at N1 (a Livewire win via the boot prop).

No product bug found, so no issue bodies were written.

## SIGN-OFF SHEET (NEEDS_HUMAN)

- CB-U08a, name plate `Op-CORE Soldier`, H-CB-U08-1: `C:\Users\Steve\AppData\Local\cimmeria-lab\screenshots\lab_screenshot-20261103-014321.343.png`. Driver: corridor, orange jumpsuit, no Op-CORE visible. Looker not run.

## FINDINGS

None filed. Case (c) only.

## NOT RUN / BLOCKED

- Bark clause: not run. Needs mission 639 at step 2117 (a Stasis Sickness mission stage that the fresh character did not have; only the ability 1372 was launched at zone load), then a Region2 crossing.
- Second-client AoI clause: not run.
- Mission 638 completion: never reached.

## RESUME HINT

A re-run on a fresh character must, in this order:
1. Play the boot Livewire at Prison Boot Control (N1; works).
2. Cross Region2 first. Walk or `.gotolocation` to about (-265, 65.45, -135), which fires the Region2 entry. That is what accepts mission 638 and adds the "Free Prisoner 329" dialog set. Confirm `accepting mission` 638 in the log.
3. Only then go to Prisoner 329 at (-289.81, 65.6, -113.14) for the dialog and ACCEPT, then the door panel at (-287.29, 67.26, -115.04), then find the cast and take the screenshot facing the soldiers.
4. For the bark clause, get mission 639 to step 2117 before the Region2 crossing (check the chain in `castle_cellblock_chains.sql:3471` and the stasis cure steps), then cross once, re-cross, and relog for the once-only checks. Read the log between the `start` and `end` bookmarks from `dr-lab\logs\server.log`, not the tail.
5. For the door button, avoid `rotate_camera` failures: stand 4 to 6 m off and face it; tolerate the harness's `server_unreachable` on `client_minigame_play` and grade the win from `fire_chain_by_id`.
