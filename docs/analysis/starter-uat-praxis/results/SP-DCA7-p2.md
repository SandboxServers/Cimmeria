# SP-DCA7-p2 results: Harset to Castle crossing (RQ-R2)

Verified 2026-11-03 by Sonnet verifier; driver attempts: 1 (Haiku, instance p2). Character differs from the brief: Fxdcafour (player 107, Jaffa, entity 90, account lab2) replaced Uat FxDcahcb. Server evidence window 14:08:50Z-14:11:16Z, from the dr-lab `logs\server.log` (timestamps are UTC). The `server_log_tail` ring was empty for these filters; `server_sessions` shows the character in_world in Castle.

| Row | Result | Tier | Server evidence | Client evidence | Looker | Notes |
|---|---|---|---|---|---|---|
| RQ-R2 | PARTIAL (landing distance not provable server-side; Marsh marker NEEDS_HUMAN) | G (`/gmdhd 2` typed as the step) then a real gate crossing; no `.gotolocation` fallback | `gmDHD: dialing stargate` 14:08:54.898Z (WARN: address 2 not held, granted for the session, expected for a GM dial); `Gate travel: dial accepted, gate opens in 4s` from=Harset to=Castle; `Stargate passed - crossing` 14:09:10.104Z; `fire_stargate_crossed: matched` destination_world=Castle; RESET_ENTITIES 14:09:11.68Z; Castle `Enter world` space 65537 14:09:13.74Z, `fire_player_loaded` 14:09:13.75Z. Chain 10130 fired: `Content: accepting mission` mission 10320 "Report from Harset" chain_id 10130, `Mission accepted` 14:09:10.104Z; Castle load showed saved missions count=4 (was 3) at 14:09:13Z; Marsh dialog set chain 10131 added. No `arrival_unrecoverable_off_mesh` anywhere in the window. | World 57 to 8 (driver WAIT 14:10:11Z); client pos (54464.35,74518.79,4723.98) at 14:10:11Z vs (54464.86,74518.69,4723.24) at 14:10:28Z, delta under 1 m in client units: no snap-back. Driver quoted server pos (761.68,61.72,560.65) after the crossing. | not run (skipped by instruction) | see Findings 1 and 2 |

Clause detail
- Crossing happened: PASS (one crossing, 57 to Castle, this player, by gate; not a fallback).
- Landing: PARTIAL. The server logs no landing position. The code contract (gate_travel/tests/arrival.rs) is arrival exactly on the gate row (761.677,63.466,551.716), which matches the brief. The only server position is the one the driver relayed from `client_move_to` (761.68,61.72,560.65): x identical, 1.75 high and 8.9 on z, about 9.1 m from the gate and outside the 5 m limit. It was read about 7.8 s into a harness walk, so it may be walking drift; I cannot separate that from a wrong arrival. A later `server_entity_query` is after a logoff (14:11:16Z) and relog and does not count.
- No snap-back: PASS on client reads only (client-read only); the server has no second position row.
- No warn: PASS for `arrival_unrecoverable_off_mesh`. Other WARNs listed below.
- Return chain: PASS from the log (accept at 14:09:10Z, persisted: count 4 at 14:09:13Z). The DB row for 10320 is gone when queried now (player 107 has 704, 708, 1521 only) because the character logged off 14:11:16Z and a new fixture loaded at 14:11:45Z, after the window; not a failure.

## Findings
1. WARN `movement.speed_warning` x5 at 14:09:29.9Z-14:09:30.2Z (distance 3.1-4.4 per tick, warn-only, accepted) while the harness walked in Castle toward the gate point. Harness movement speed vs server tolerance, class (c); not filed.
2. WARN `archetype_has_no_tree` for Jaffa (player 107) at the Castle load; known Jaffa gap, outside the row; not filed.
3. Driver gaps (c): `client_move_to` attempt 1 failed with "player moved 925.3 m" because the crossing completed during the approach; attempt 2 failed "ledge ahead" at server (744.54,60.44,543.62). The dial plus approach overlapped; the driver did not hold position. No product bug (a)/(b) found, so no issue file was written.
4. Mission 10320 was accepted at "Stargate passed" while still in Harset, 1.5 s before the world transition; fine for the row.

## NEEDS_HUMAN (sign-off sheet)
RQ-R2-marsh | lab_screenshot-20261103-081037.080.png | not viewed by driver; Marsh id 100034 "Colonel Marsh" 71.93 m away at server (810.73,55.42,515.01), rendered true, on_screen false | looker not run | cannot tell (Marsh was off screen per the client read)
RQ-R2-gate | lab_screenshot-20261103-081030.816.png | not viewed by driver | looker not run | cannot tell

## NOT RUN / BLOCKED
Looker skipped by instruction. Nothing blocked.

## Recipe
- Evidence: `Select-String` on `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\dr-lab\logs\server.log` with a `T14:0x` prefix and the terms `gmDHD|Gate travel|Stargate passed|fire_stargate_crossed|accepting mission|speed_warning`; the log is JSON per line (fields.message). The `server_log_tail` ring was empty.
- Re-run to close the landing clause: after `/gmdhd 2` do NOT call `client_move_to`; wait for the Castle `fire_player_loaded` log row, then immediately `server_entity_query` class_id 2 (or `server_entity_get`) once, wait 15 s, query again. Expect within 5 m of (761.68,63.47,551.72) both times.
- Mission check must happen before the next relog or fixture: `SELECT mission_id,status FROM public.sgw_mission WHERE player_id=107`.
