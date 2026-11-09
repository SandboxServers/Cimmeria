# SP-DTO1-p2 result (verifier)

Character: reused fixture character Fxdltwo (player id 109), client instance p2. Fixture applied 07:28:16Z (`lab fixture applied`, world Tollana, moved true, missions 1). Source: dr-lab logs\server.log (UTC). Windows from bookmarks `SPU SP-DTO1-p2 P4 start` 07:29:33Z and `P5 start` 07:30:32Z.

| Row | Grade | Tier (N1/N2/N3/G) | Server evidence (query or log event, time) | Screenshot | Issue |
|---|---|---|---|---|---|
| P4 | PASS | N1 (client_world_click); setup .gotolocation = G | `interact` for entities 100431 (07:29:44.98Z), 100432 (07:29:54.91Z), 100433 (07:30:11.29Z), each followed by `fire_interact_tag: matched` and `Content: displaying dialog` 4211, 4213, 4214 (chains 9013, 9016, 9018, "other states: dialog only"). Player pose (197.5,-1.01,-538.85) vs spawns = 2.31 m each. No `Content: advancing step`, no `Mission accepted`. No Nest interact in 07:30:18-07:30:32Z (the out_of_range refusal came from the lab tool, client-side). `sgw_mission` player 109 mission 568: 0 rows. No WARN/ERROR in the packet window; no `dialog_set_maps cache miss`. | none | none |
| P5 | PASS (+NEEDS_HUMAN on blocking) | N1 (client_world_click) | `interact` target 100004 at 07:30:50.27Z, then `interact: DHD -> onDisplayDHD` (stargate Tollana, id 4) from pose (207.5,-0.6,-555.5), about 3.4 m. DHD entity 100004 at (209.75,-1.01) (bookmark rows). Ring console: bookmark rows show a "Ring Transporter Switch" spawn 155, entity 100065, tag Tollana_Ring_fffa0002 (driver saw "Ring Transport Controls" targetable at 4.29 m; targetable is client-read only). Dial: NOT-RUN. | DTO1-P5-dhd (lab_screenshot-20261109-013054.614.png) | none |

## Not run

- P4 Core-at-1.5 m click: out of scope for this part.
- P5 dial: no address given; SP-05 proves the Tollana address.

## Resume hint

A re-run starts from the fixture state: Tollana (211.045,0.606,-547.644), mission 568 not_active, no mission 568 row.

## Recipe

- `server_db_query`: `SELECT player_id,account_id,player_name FROM public.sgw_player WHERE player_name ILIKE '%Fxdltwo%'` and `SELECT * FROM public.sgw_mission WHERE player_id=109 AND mission_id=568`.
- Log file grep (dr-lab logs\server.log) for `SPU SP-DTO1-p2`, `lab fixture`, and the 07:29-07:31 window (interact, dialog, WARN/ERROR). The server_log_tail ring is too short for past events.
- Looker: Haiku sub-agent on the screenshot from the lab screenshots folder.

## Findings

1. P4 note: the driver's first Fungus click failed on the client-side on_screen step (camera pitch clamp -78.8); the retry worked. This is a harness quirk, not a product finding.
2. P5 note: the DHD entity has no name on the client (driver); the server names it "DHD". The Escape did not close DHDWin (harness quirk, per coordinator). Neither is a product finding.
3. The out_of_range refusal for the Nest at 5.4 m was made by the lab tool, so no server row was expected. The server never saw it, which is consistent with the row.
4. Fixture applied at 07:28:16Z, earlier than the driver's stated ~07:29. The re-stage was applied before the first bookmark, so it has no effect on the grades.

## Sign-off sheet

| Row | What to look at | Screenshot path | Driver's words | Looker pre-screen | Your read of the numbers |
|---|---|---|---|---|---|
| P5 | Is Capt. Copplemann (or anyone) standing between the camera and the DHD? | DTO1-P5-dhd (lab_screenshot-20261109-013054.614.png, lab screenshots folder) | shot not viewed by the driver | 1 YES (DHD window open, glyph ring and Address Book). 2 UNSURE (the dialog covers the pedestal; a dark shape at the right edge). 3 NO (the DHD dialog covers the scene). | The DHD interact opened the window at 3.4 m with a server row. The blocking question cannot be judged because the window covers the pedestal. |
