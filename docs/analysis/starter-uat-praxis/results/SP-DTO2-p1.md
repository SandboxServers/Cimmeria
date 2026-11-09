# SP-DTO2-p1 result (verifier)

Run deviations: instance p3 stood in for default (character Fxtoltwothree, player_id 112, account 12); lane p2 played Fxdltwo (player_id 109, account 11). Bookmark `SPU SP-DTO2-p1 P6 start` about 08:33:29Z 2026-11-09 (driver UTC; not seen in the log, see Findings 3). Tier of the whole packet: G for `.bug` and `.gotolocation`, client reads N3.

| Row | Grade | Tier (N1/N2/N3/G) | Server evidence (query or log event, time) | Screenshot | Issue |
|---|---|---|---|---|---|
| P6 | PASS +NEEDS_HUMAN | G (jumps), reads N3 | `server_sessions` after the run: two in_world sessions, player_id 109 (entity 1) and 112 (entity 39), world Tollana. `server_witnesses` 100430 (template 420): witnessed_by entity 1 and entity 39; entities 39 and 1 each list the other plus 100004, 100065, 100430-100433. `server_entity_query` 420/421: 4 spawns 19001-19004 at the seed positions. Out of range (driver 6.5, count 0 at 400 m, own avatar y -765.97 at the pad) and the 6.3 sighting are client-read only: the witness state is a post-run snapshot, the 6.3-time and pad-time witness sets are not available. Return (6.7): the post-run witness lists contain all four again and both avatars sit at the fixture point (211.04, 0.01, -547.64). | DTO2-P6-p2-return (sign-off) | none |
| P7 | PASS +NEEDS_HUMAN | N3 reads | Server now: 420 at (206.00, -0.58, -543.00); 421 at (199.50, -0.93, -540.00), (195.50, -1.01, -540.00), (197.50, -1.01, -536.54); all equal to the seed to 0.01 m, is_on_ground true. Client: READ_A1 (about 08:33:3x) vs READ_A2 08:35:32 report identical positions for all four, rendered in both; gap more than 120 s (at least 60 s). | DTO2-P7-final (sign-off) | none |
| P9 | PARTIAL | n/a (verifier log read) | The log ring buffer holds 500 entries, about 14 s of DEBUG chatter, ending after the run; the packet window (08:33:09Z to about 08:35:55Z) is not covered. No `CorrectionSuppressed`, navmesh reject or spawn WARN/ERROR in what is held; this proves nothing about the window. Indirect: no player drift (both avatars at the fixture point), spawns intact. | none | none |

## Not run
- P8 and P10: part SP-DTO2-p2.
- Real walk back from the pad (570, -1.04, -530) to the gate on foot: the return was a `.gotolocation` jump (tier G). A re-run needs a walk of about 359 m from the pad; not exercised.
- P9 log checks (CorrectionSuppressed, navmesh reject/invalid/off-mesh, spawn/npc/aoi WARN/ERROR for template 420/421, and the `dialog_set_maps cache miss` WARN): the log tail covers about 14 s after the run only. A re-run must call `server_log_tail` during or right after the part.
- `lab fixture applied` lines for both characters: not in the buffer. Fixture is inferred (see Findings 3).
- Live phasing U-PH1 to U-PH5 and PH-01a/b (mission 621): not runnable today (PH-04 verb missing); no rows. Walk back from the pad on foot is the resume path.

## Resume hint
Both characters (Fxdltwo entity 1 player 109, Fxtoltwothree entity 39 player 112) are still in_world on Tollana at (211.04, 0.01, -547.64) with mission 568 absent (no `sgw_mission` row, i.e. not_active). A re-run starts from that state; for P9 run the log read inside the window.

## Recipe
- `server_sessions`; `server_db_query` for `public.sgw_player WHERE player_id IN (109,112)` and `public.sgw_mission WHERE mission_id=568`.
- `server_entity_query template_id=420` and `421`; `server_entity_get entity_id=1` and `39`.
- `server_witnesses entity_id=100430`, `39`, `1` (the argument is `entity_id`, required).
- `server_log_tail lines=5000` (the `filter` argument is ignored; `contains` and `limit` exist; buffer is 500 entries).
- Looker: Haiku general-purpose agents on the two screenshot paths.

## Findings
1. entity_find "player" oddity (lab tool, not a product bug). In each lane the "player" entry carried the other character's name (lane A: Fxdltwo id 1; lane B: Fxtoltwothree id 39). `server_witnesses` shows the two players did witness each other (entity 39 witnesses include entity 1; entity 1 witnesses include entity 39; each is in witnessed_by of the other), and `server_entity_get` gives entity 1 = Fxdltwo, 39 = Fxtoltwothree. So the remote avatar occupies the "player" key and the local avatar is not listed while the remote one is in range; at the pad (remote out of range) lane B's "player" is its own avatar (id 1). Count 6 in READ_A1/B1 and B3 therefore includes the remote player, not the local one: a lane that wants the own entity must use player_state. Row P6 (6.3, 6.7).
2. Pad position (observation). After `.gotolocation Tollana 570 -1.04 -530` the lane B avatar reads server y -765.97 (driver step 5) instead of -1.04: no ground at the pad, so the vertical was not clamped. No row depends on it; the avatar was jumped back afterwards. Row P6 (6.5). Not filed: lab/recipe matter (c), choose a pad with ground for a re-run.
3. Evidence gap. The log ring buffer (500 entries, about 14 s) does not reach the window: no `lab fixture applied` lines, no `SPU SP-DTO2-p1 P6 start` bookmark, no P9 data. Fixture is inferred from the avatars at the fixture point (211.04, 0.01, -547.64) and no `sgw_mission` row for 568 for either player. P6/P7 rest on the post-run server snapshot plus the driver's client reads. `server_witnesses` is available after the run (clients stopped did not remove the sessions).

No issue bodies written: no product bug or doc mismatch found.

## Sign-off sheet
| Row | What to look at | Screenshot path | Driver's words | Looker pre-screen | Your read of the numbers |
|---|---|---|---|---|---|
| P6 | Return after the jump: standing figure and the machines in view, stone plaza | C:\Users\Steve\AppData\Local\cimmeria-lab\screenshots\lab_screenshot-20261109-023424.602.png | "driver cannot view"; READ_B3 count 6, all four listed, rendered | Q1 YES: standing figure in an orange coat, facing away. Q2 UNSURE: large metal ring on the right, second machine not clear. Q3 YES: outdoors under trees on a tiled plaza with benches. | All four spawns witnessed by both players post-run, positions at seed; the shot is the visual check of the machines |
| P7 | Final shot: nothing sunk or rotated | C:\Users\Steve\AppData\Local\cimmeria-lab\screenshots\lab_screenshot-20261109-023533.473.png | READ_A2 same as READ_A1, all rendered | Q1 YES: man stands upright, seen from behind. Q2 UNSURE: only a cropped gadget at the right edge. | Server positions equal the seed to 0.01 m; the ground contact of the machines is not visible in the shot |
