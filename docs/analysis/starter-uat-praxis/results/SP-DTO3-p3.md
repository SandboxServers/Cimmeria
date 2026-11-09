# SP-DTO3-p3 results (Praxis starter UAT, Tollana 568 Core, U2b and U4b)

Character Fxdcatwo (player_id 105), instance p2, local lab server (SigNoz `cimmeria.lab_run = dr-lab`). Window 07:51-07:57Z on 2026-11-09. Evidence from SigNoz; `server_log_tail` not used (ring too short, see verify-server-evidence.md).

Fixture: INFO `lab fixture applied` for player 105 at 07:52:16Z (missions 1, moved true). The `U2b start` bookmark (07:53:31Z) shows mission 568 at step 1965, completed objectives [2309,2310], so the fixture state held. No accept log pair seen (expected).

| Row | Grade | Tier (N1/N2/N3/G) | Server evidence (query or log event, time) | Screenshot | Issue |
|---|---|---|---|---|---|
| U2b Starting state | PASS | G (fixture) | bookmark `SPU SP-DTO3-p3 U2b start` 07:53:31Z: mission 568 step_id 1965, status 1, completed objectives [2309,2310] | - | - |
| U2b Core 1965 to 1966 | PASS | N1 (click on Core 100431) | `Content: advancing step` chain 9012, step 1966, 07:53:43.93Z; bookmark recent_events `dialog=4211 chain=9012`, `mission=568 step=Some(1965)->1966`; `U4b start` bookmark 07:53:48Z shows step_id 1966, objective 4491 active; last_interaction_target 100431 | lab_screenshot-20261109-015354.173.png | - |
| U2b Single bind | PASS | N1 | one `removing dialog set` map 4959 and one `adding dialog set` map 6393 (template 420, chain 9012, flags INT_AStoryMissionTurnIn), both 07:53:43.93Z; no other add/remove in the row window | - | - |
| U2b Journal 1966 | NEEDS_HUMAN | N1 (key j posted, no window) | client-read only. Tracker line "Report to Captain Copplemann who is running the operation on Tollana." (objective 4491 active on server) | lab_screenshot-20261109-015354.173.png | - |
| U4b Relog | PASS +NEEDS_HUMAN | N1 | `player session ended` (logoff_character_select) 07:54:42.9Z; `player entered world` 07:55:01.13Z; `Restored missions from the saved journal` active `568:1966`; `Content: adding dialog set` map 6393 (chain 9009 restore) 07:55:01.13Z; coordinator DB read after relog: step 1966, status 1, completed [2309..2312], active [4491] | lab_screenshot-20261109-015557.322.png | - |
| U4b No fixture on relog | PASS | G | `lab fixture applied` rows for player 105: 07:46:59Z, 07:52:16Z (the packet's), then 07:56:50Z (after this part, staging the next one). None between 07:54:42Z and 07:55:57Z | - | - |
| U4b Glyph | NEEDS_HUMAN | N1 | client-read only; Copplemann (100430) at 9.46 m, on_screen false per driver, so the glyph was not expected in the shot | lab_screenshot-20261109-015557.322.png | - |

## Looker answers (Haiku, verbatim)

DTO3-U2b-journal-1966:
1. NO. Crafting window (Alloy/Craft tabs) is open; only a HUD quest tracker is visible.
2. YES. "Finding Tollan Technology" is shown at right.
3. YES. "Report to Captain Copplemann who is running the operation on Tollana."

DTO3-U4b-1966:
1. NO. The "?" icon sits at the torso or waist, not above the head.
2. NO. The symbol is a question mark, not an exclamation mark.
3. YES. A white "?" inside a circle, at torso level.
4. NO. A "?" icon is visible in front of the figure's torso.

## Not run

- Journal window for 1966: the j key opened no window (driver: "journal: no window"); only the HUD tracker was readable. A next driver should open the journal through the stock UI button or `client_ui_click`, then screenshot. Lab or driver gap, no product issue.
- Glyph over Copplemann: the shot was taken with him off screen at 9.46 m. A next driver should `.gotolocation` to the COP4 spot (a few metres from template 420) and face him before the shot. The relog's numeric half is graded on server evidence.
- Phasing rows U-PH1 to U-PH5: not runnable today.
- Out of scope here: offer and U4a (SP-DTO3-p1), Fungus and Nest (SP-DTO3-p2), turn-in (SP-DTO3-p4).

## Resume hint

Character Fxdcatwo is at 568 step 1966 (objective 4491 active), bind 6393 on Copplemann (template 420), standing at the Core position (197.5, -1.0, -538.85). A re-run of the glyph look starts there; walk to Copplemann first. The next part (p4 turn-in) staged a fresh fixture at 07:56:50Z.

## Recipe

- SigNoz: `signoz_search_logs {service:"cimmeria-server", filter:"attribute.note CONTAINS 'SPU SP-DTO3-p3'", timeRange:"2h"}` gives the bookmarks with mission snapshots (lab_run `dr-lab`).
- `filter:"body CONTAINS 'lab fixture' AND body CONTAINS 'applied'"` for fixture rows.
- `filter:"resource.cimmeria.lab_run = 'dr-lab' AND body CONTAINS 'dialog set'"` with start/end in ms for bind add/remove.
- `filter:"resource.cimmeria.lab_run = 'dr-lab' AND attribute.player_id = 105 AND (body CONTAINS 'session' OR ...)"` for logout/login rows.
- Lookers: Haiku general-purpose agents given the PNG paths.

## Findings

1. Observation (not a defect): the prop dialog for the Core had Done and Prev buttons and its text tail differs from the brief wording (first clause matches). The server dialog id is 4211 on chain 9012.
2. Observation: no WARN or ERROR in the packet window. A WARN `inbound_message_dropped` (reason `unhandled_account_method`, msg_id 221) fired at 07:56:43Z for player 105, after the part ended and during the next part's logout; out of window. The `dialog_set_maps cache miss` WARN did not appear.
3. Observation: the U4b looker saw a "?" at torso level of a figure although the driver reported Copplemann off screen. The source of that mark is unclear; this goes to the human sign-off.

No product findings; no issue bodies written.

## Sign-off sheet

| Row | What to look at | Screenshot path | Driver's words | Looker pre-screen | Your read of the numbers |
|---|---|---|---|---|---|
| U2b Journal 1966 | journal entry for "Finding Tollan Technology" with the return-to-Copplemann objective | C:\Users\Steve\AppData\Local\cimmeria-lab\screenshots\lab_screenshot-20261109-015354.173.png | "journal: no window"; tracker text "Report to Captain Copplemann who is running the operation on Tollana." | Journal window NO (a crafting window was open), title YES (tracker), objective line YES (tracker) | Server: step 1966, objective 4491 active; the tracker text matches the "return to Copplemann" objective |
| U4b Glyph | turn-in mark over Copplemann at step 1966 | C:\Users\Steve\AppData\Local\cimmeria-lab\screenshots\lab_screenshot-20261109-015557.322.png | "Copplemann stays off screen at 9.46 m ... glyph is likely not in the shot" | "?" icon, white in a circle, at torso level; not exclamation; not above the head | Server: bind 6393 restored at 07:55:01Z (INT_AStoryMissionTurnIn), so a question-mark turn-in glyph is the expected state |
