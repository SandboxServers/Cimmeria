# SP-DTO2-p2 result (verifier)

Deviation: reused fixture character Fxdlone (player id 108), client instance p2, not Uat Fxtoltwodefa. Evidence source: dr-lab logs\server.log (UTC), lab-server MCP.

| Row | Grade | Tier (N1/N2/N3/G) | Server evidence (query or log event, time) | Screenshot | Issue |
|---|---|---|---|---|---|
| P8 | PARTIAL | N3 chat bookmark / N1 client_move_to | INFO `lab fixture applied` for player 108 (world Tollana) 07:32:39Z. Bookmark `SPU SP-DTO2-p2 P8 start` 07:33:57Z, player at (211.04, 0.01, -547.64), entity 24. Bookmark P10 07:34:13Z, player at (205.31, -1.01, -545.42). No WARN/ERROR, no `CorrectionSuppressed`, no `dialog_set_maps`, no removal or TriggerVolume rows in 07:33:40-07:34:35Z. Live `server_entity_query` template 421: spawns 19002/19003/19004 (entities 100431/2/3) still at seed positions (199.5,-0.93,-540), (195.5,-1.01,-540), (197.5,-1.01,-536.54). Walk through (214.0,-547.7) is NOT server-evidenced: only bookmark positions exist (per-step movement log empty), driver's own end position was about (209,-0.7,-547.1). Copplemann (template 420) not re-queried live. | none | none |
| P10 | NEEDS_HUMAN | n/a (observation) | Numeric half: `server_entity_query` template 421 returns the three spawns (19002-19004) at seed positions, class 1, has_static_mesh true. | lab_screenshot-20261109-013420.162.png | none |

## Not run

- P8 waypoint at (214.0,-547.7) not confirmed by server; a re-run needs a position trace (SigNoz movement events) or a bookmark at the waypoint.
- P10 visual: the driver did not view the shot, and the looker saw only the player's back, so the prop mesh is unassessed.
- U-PH1 to U-PH5 and PH-01a/b (mission 621): not runnable today (PH-04 verb missing); no rows.

## Resume hint

Fixture Fxdlone is on Tollana near (205.3,-1.0,-545.4), mission 568 not_active. For P10, stand 4-6 m from 100432 facing it, with the camera on the prop, then screenshot.

## Recipe

- `server_db_query`: `SELECT player_id,account_id,player_name FROM public.sgw_player WHERE player_name ILIKE '%Fxdlone%'` gave 108.
- Select-String on dr-lab logs\server.log for `SPU SP-DTO2-p2|lab fixture applied` (07:32:39Z; bookmarks 07:33:57Z and 07:34:13Z).
- `server_entity_query {template_id: 421}`.
- Looker (Haiku) on the screenshot in the lab screenshots folder.

## Findings

1. P8: the trigger volume (208.2,2.1,-548.0) produced no log row; its purpose stays UNRESOLVED (not a failure).
2. P10: the driver never viewed the screenshot. The looker saw the player character from behind (orange vest), not the prop.

## Sign-off sheet

| Row | What to look at | Screenshot path | Driver's words | Looker pre-screen | Your read of the numbers |
|---|---|---|---|---|---|
| P10 | Template 421 prop mesh: shape, colour, on floor or not | lab_screenshot-20261109-013420.162.png (lab screenshots folder) | "not viewed by driver" | Q1: player character in orange vest, seen from behind. Q2: UNSURE, feet cropped. Q3: NO, orange vest, dark blue shoulder, pale grey sleeves. (Prop not visible; looker answers are about the player.) | Three 421 spawns exist at seed positions, has_static_mesh true; no mesh description available. |