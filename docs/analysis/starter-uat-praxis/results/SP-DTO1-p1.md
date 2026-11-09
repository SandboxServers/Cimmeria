# SP-DTO1-p1 results (Tollana placement P1 to P3)

Character Fxdltwo (player 109), instance p2, reused fixture character. Fixture: INFO `lab fixture applied` 07:24:08Z (world Tollana, moved true). Clock: server log is UTC. Bookmarks: P1 start 07:25:23Z, P2 start 07:25:28Z, P3 start 07:25:54Z.

| Row | Grade | Tier (N1/N2/N3/G) | Server evidence (query or log event, time) | Screenshot | Issue |
|---|---|---|---|---|---|
| P1 | PASS +NEEDS_HUMAN | G (fixture arrival), client-read (FIND) | Bookmark P1 start 07:25:23Z: player in Tollana at (211.045, 0.010, -547.644), exactly the staged position. server_entity_query template 420: entity 100430 spawn 19001 at (206.00, -0.58, -543.00), heading 2.315, matches seed (DB row spawnlist 19001 same). Client FIND (driver, client-read only): id 100430, distance 6.86 m, rendered and targetable true; driver gave the server position, not an independent client position. | lab_screenshot-20261109-012528.686.png (DTO1-P1-arrival-view) | none |
| P2 | PASS | N1 | Driver legs: Copplemann, CORE, FUNGUS arrived and confirmed with no stop; NEST already_in_range (no movement, not a native pass for that leg). Server log 07:25:20Z to 07:26:20Z: zero WARN or ERROR lines, no CorrectionSuppressed, position_correction or rejected move rows, no dialog_set_maps cache miss. P3 bookmark position (199.37, -1.008, -539.40) is consistent with the final leg ends. | none | none |
| P3 | PASS +NEEDS_HUMAN | G (.gotolocation to stand near props), client-read (3.2) | server_entity_query template 421: three entities, 100431 Core (199.5, -0.93, -540.0), 100432 Nest (195.5, -1.01, -540.0), 100433 Fungus (197.5, -1.01, -536.54), heading 2.025, is_on_ground true; all match seeds. Client side (driver, client-read only): all three rendered and targetable. | lab_screenshot-20261109-012603.047.png (DTO1-P3-props) | none |

## Not run

- Looker clauses for P1 (standing figure, feet on ground, facing camera) and P3 (props on floor, no flicker): the screenshot files could not be located on disk by name within the time available (a recursive search of the repo and worktrees found none), and the driver did not view them. Looker not run; both go to the sign-off sheet as unscreened.
- P2 NEST leg did not move (already within range from the previous leg), so it proves nothing about walking to Nest. Only three of four legs exercised the walk.
- Independent client-side positions were not captured; the driver reported server positions. Position agreement within 0.5 m is not proven from the client side.

## Resume hint

Fixture state remains: Fxdltwo in Tollana near (197.5, -0.7, -538.85). A re-run starts with a fresh fixture for mission 568 not_active, then lab_ensure_in_world and the P1 bookmark. To finish the visual clauses, locate the two screenshot files in the lab screenshots folder and view them.

## Recipe

- server_db_query: `SELECT player_id, account_id, player_name FROM public.sgw_player WHERE player_name ILIKE '%Fxdltwo%'` (player 109).
- server_entity_query template_id 420 and 421 (no space filter; the space instance id is 65541, so space_id 19 returns nothing).
- server_db_query on resources.spawnlist for seed rows (columns: spawn_id, template_id, world_id, x, y, z, heading).
- Select-String over dr-lab logs\server.log for `SPU SP-DTO1-p1|lab fixture applied`, then WARN/ERROR/correction patterns for 2026-11-09T07:25 to 07:26.

## Findings

1. Process note (all rows): the row text says `server_entity_query` for world 19; live entities carry space_id 65541, so a space_id 19 filter returns empty. Filter by template_id. Not a product bug.
2. No WARN or ERROR in the P1 to P3 window; none expected, none found.

## Sign-off sheet

| Row | What to look at | Screenshot path | Driver's words | Looker pre-screen | Your read of the numbers |
|---|---|---|---|---|---|
| P1 | Standing figure near the arrival, feet on floor, facing the player | lab_screenshot-20261109-012528.686.png | not viewed by driver; Copplemann 6.86 m, rendered, targetable | not run (file not located) | Server position equals seed, heading 2.315, on ground |
| P3 | Three machines on the floor, no float, sink or z-fighting | lab_screenshot-20261109-012603.047.png | not viewed; three units at 2.31 m, rendered, targetable | not run (file not located) | Server positions equal seeds, is_on_ground true |