# SP-DTO4-p3 result: Tollana mission 568 turn-in row U6 (runner-DCA-q, instance p2, Fxdcatwo, player 105)

Verified by the coordinator against `dr-lab` server.log (UTC), bookmarks `SPU SP-DTO4-q`. Screenshots not viewed (Haiku drivers cannot view them).

| Row | Grade | Tier | Server evidence | Screenshot | Issue |
|---|---|---|---|---|---|
| U6 | PASS (server) +NEEDS_HUMAN (turn-in glyph, silent look) | N1 clicks, DIALOG close | Fixture 568 step 1964 applied (attempt 2, 08:23:30Z). Core click 08:23:57.7Z: `advancing step` 1965 then 1966 (chain 9011), 4959 removed, 6393 added, dialog 4211 displayed. Copplemann click 08:24:44.9Z: `displaying dialog` 2154, chat text "You find this in a food duplication unit?". Close 08:25:03.96Z (coordinator DIALOG): `removing dialog set` 6393, `Content: completing mission` 568 chain 9005 "turn-in on close of 2154", `Mission completed directly`. Silent state (W2): a click on Copplemann afterwards (08:25:2xZ) opened no window (`windows_opened []`) and logged no new `displaying dialog`. | turn-in glyph lab_screenshot-20261109-022410.941.png; silent lab_screenshot-20261109-022518.147.png (taken after the last click) | none |

## Attempts
- Attempt 1 (08:21Z): FAIL by driver error, clicked the Fungus unit (100433) instead of Core, so 568 stayed at 1964 and Copplemann showed the in-progress dialog. No product finding.
- Attempt 2 (08:23Z): the first two Copplemann clicks (camera turn, then `observe`) opened nothing; the third opened 2154. The driver left 2154 open, so the coordinator closed it (completion) and ran the silent click.

## Brief defects
- `client_entity_find` results are not sorted by distance; the brief's "CORE_ID is the nearest" must pick by position (199.5, -540.0).
- Copplemann is off screen from COP4 (screen x negative); the click turns the camera and needs a retry or two.
- `lab_screenshot` takes no label; record the path against the label.
- The U6 glyph shot at COP4 likely misses Copplemann (screen [514,1470] off screen); the glyph look is NEEDS_HUMAN and probably unobservable from there.

## Sign-off sheet
| Row | What to look at | Screenshot | Read |
|---|---|---|---|
| U6 | Turn-in mark above Copplemann at step 1966 | lab_screenshot-20261109-022410.941.png | Copplemann reported off screen |
| U6 | Silent completed state: no mark, no window | lab_screenshot-20261109-022518.147.png | numbers confirm no dialog |
