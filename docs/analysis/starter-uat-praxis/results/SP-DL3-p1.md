PACKET SP-DL3-p1 VERIFIED 2026-11-09T07:17Z by Sonnet; driver attempts: 2; server evidence window 07:14:22-07:16:25Z (dr-lab server.log + server_db_query; SigNoz and memory buffer empty)

Deviations: instance p2, reused Commando character Fxdlone (player_id 108, account 11, extra_name "Uat"), not "Uat Fxdlthree". Looker skipped by instruction; screenshots NEEDS_HUMAN.

| Row | Grade | Tier | Server evidence | Screenshot | Issue |
|---|---|---|---|---|---|
| LEG-3a fixture applied | PASS | G | 07:13:21.70 "lab fixture applied" world Harset moved=true; sgw_player 108 world_id=57, world_location Harset | - | - |
| LEG-3a DHD opens on a real click | PASS (a2; a1 was a brief range defect) | N1 | a1 (07:14:27-07:15:11): no interact row for player 108 (driver was 5.74 m from DHD, over the 5 m range; brief defect). a2: bookmark start 07:15:39.11, gotolocation to (7,-68.95,34) 07:15:40.57, then 07:15:52.12 "interact: target resolved" entity 100002 DHD, 07:15:52.124 "interact: DHD -> onDisplayDHD" stargate_id 3 Harset, interaction_pose route fallback; end bookmark 07:16:03.56. Window DHDWin opening is client-read only | L3-harset-dhd-list-a2 | - |
| LEG-3a Tollana dialable by a player | BLOCKED: no playable Harset to Tollana hand-off | N1 (client-read) | Tollana not listed (client-read only: DHDWin lists [] and no gate names). Persisted sgw_player.known_stargates = [2] = The Castle; Tollana is stargate_id 4 (resources.stargates), absent. Matches the expected design: no address held | L3-harset-dhd-list-a2 | - |

Grade: BLOCKED (no playable Harset to Tollana hand-off), as expected by design. Not a FAIL. Combined LEG-3 is composed with SP-DL3-p2.

Findings: no unexpected WARN or ERROR for player 108 in the a2 window. Observation only (not a product bug): DHDWin opens with an empty list for a Harset character holding only The Castle (id 2), so the list shows no entries even though one address is known. Possibly the home gate is excluded from the list; not investigated, not filed.

## Not run
- Looker pre-screen (skipped by coordinator instruction).
- Clause that Tollana would appear in the list when an address is granted (out of scope for this row).

## Resume hint
Nothing to redo for this row. Any rerun must stand within 5 m of DHD 100002 (server position 4.41, -68.96, 31.53); (7,-68.95,34) is 3.58 m and works; (8,-68.95,36) is 5.74 m and fails.

## Recipe
1. ".bug SPU SP-DL3-p1 leg3a start"; ".gotolocation Harset 7 -68.95 34".
2. client_world_click entity 100002 right expect DHDWin (confirmed, N1).
3. client_window_read window DHDWin (title DHD, no lists, no gate names).
4. Verify: server.log lines at 07:15:52 (interact: DHD -> onDisplayDHD); `SELECT known_stargates, world_id FROM public.sgw_player WHERE player_id=108`; `SELECT stargate_id,name FROM resources.stargates`.

NEEDS_HUMAN EVIDENCE FOR SIGN-OFF:
- L3-harset-dhd-list-a2 = lab_screenshot-20261109-011601.255.png: DHD window open; driver/coordinator read says no gate names, Tollana not listed. Question for a person: names legible, none Unknown. Looker: not run.
- (attempt 1, no window) L3-harset-dhd-list-pre = lab_screenshot-20261109-011454.922.png, L3-harset-dhd-list = lab_screenshot-20261109-011507.711.png: taken while out of range; no DHD window expected. Looker: not run.
