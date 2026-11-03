PACKET SP-DCA3-p2 VERIFIED 2026-11-03 by Sonnet; driver attempts: 1; server evidence window 12:48:02Z-12:49:50Z (SigNoz cimmeria-server, cimmeria.lab_run='dr-lab'; DEBUG rows were present)

| Row | Result | Tier | Server evidence (time, scope, event, key fields) | Client evidence | Looker | Notes |
|---|---|---|---|---|---|---|
| D6c | PASS (server) with a client-read discrepancy | N3 | 12:48:02 `lab fixture staged` (player 106); 12:48:17 `Mission state persisted` 1520 status 1; 12:48:24.500 `Restored missions from the saved journal` (1520 status 1); 12:48:24.504 `fire_player_loaded: matched`; 12:48:24.504 `Content: completing mission` chain_id 1383 mission_id 1520; 12:48:24.505 `Mission completed directly` (event mission_completed, path direct); 12:48:24.519 `Mission state persisted` 1520 status 2. No dialog 4985 or 4984 display row. No WARN/ERROR, no `mission_frame_dropped`, no `frames suppressed` row. DB now: 1520 status 2 (completed), current_step_id null, completed_step_ids [4452,4453,4454]. | Driver S2 tracker still listed "Infirmary" with step text "Hack the Med Dispenser..." (step 4453 text, not 4454's "Use your radio to report to Capt. Copplemann."); no chat line because of the capture gap. | NOT RUN: screenshot file not found on disk, so no looker | See note 1 |
| D7 | NOT-RUN | n/a | `resources.entity_templates` 412 `static_mesh` = `Ga-Props.GA-PuzzleStation00`, so the swap was not staged | none | none | Optional row; never fails the packet |

FINDINGS:
1. Chain 1383 did fire on the fixture login (login was 12:48:24Z, not 12:49:00Z; the driver's client read came about 44 s later). It completed mission 1520 directly and the DB holds status 2. The product behaviour for D6c is correct. The "chain didn't fire" hypothesis is refuted.
2. The tracker text the driver saw ("Hack the Med Dispenser", step 4453) matches neither the staged step (4454, "Use your radio...") nor a completed mission. The server restored 1520 at status 1 and completed it 5 ms later in the same player_init tick, then queued onObjectiveUpdate, onStepUpdate and onMissionUpdate frames (complete_direct.rs; no drop or suppression log). The tracker most likely held stale state from the earlier session of this character (player_loaded also fired at 12:46:02 before the fixture was staged), or the client did not apply the frames sent during world entry. This is a harness or ordering artefact until proven otherwise: the packet tap was not running, so frame delivery and client handling cannot be checked, and the chat capture gap hides the login-time lines. No issue filed. If a clean re-run (fresh client, tap on, frames after UI ready) still shows a stale tracker, file a login-time completion-frame delivery issue.
3. The relog-with-4984-open case is not covered by this part.

NOT RUN / BLOCKED: D7 (mesh swap not staged). Looker for D6c (screenshot `lab_screenshot-20261103-064923.278.png` not located on disk).

SIGN-OFF SHEET (NEEDS_HUMAN):
| row | clause | screenshot | driver note | looker answer |
|---|---|---|---|---|
| D6c | journal shows 1520 completed or absent after login at 4454 | lab_screenshot-20261103-064923.278.png | Tracker still lists Infirmary with the Med Dispenser hack step; no dialog window | not run (file not found) |

RESUME HINT: re-run D6c once on a fresh client with the packet tap started before login (mcp lab-server packet_tap_start) and read the tracker after the world is ready; expect no Infirmary entry. D7 needs the staged mesh swap first.
