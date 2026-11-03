# SP-DCA7-p3 results (nav_path legs A, B, C)

PACKET SP-DCA7-p3 VERIFIED 2026-11-03 by Sonnet; driver attempts: 1; instance p2; character Fxdcafour (player 107, entity 91). Window 14:11Z-14:14Z.

Caveat: the server log ring buffer (500 lines) no longer holds the window and no `SPU SP-DCA7-p3` bookmarks were found, so hop-by-hop server positions could not be replayed. Evidence is the current server entity state plus fresh nav_path calls on space 65537 (space 8 is not loaded; the brief's space_id 8 was replaced by the coordinator).

| Row | Result | Tier | Server evidence | Client evidence | Looker | Notes |
|---|---|---|---|---|---|---|
| RQ-R1-Marsh | PASS | client_move_to (native move), no fallback | nav_path A re-run: status ok, 3 waypoints (761.68,63.58,551.72) -> (807.24,55.18,521.70) -> (808.30,55.24,518.70), matches the coordinator's. Col Marsh (pet) entity 100034, tag Castle_ColMarsh, at (810.73,55.42,515.01), space 65537. Driver end (808.87,55.18,520.03) is 5.35 m from Marsh (recomputed): in the 3-6 m band. | 4 hops arrived at 0.26-1.45 m of waypoints; no fallback flag. Hop positions client-read only (server window logs aged out). | skipped (screenshot lab_screenshot-20261103-081257.359.png) | Marsh pet entity also named "Col Marsh (pet)", so the driver's entity_find "Marsh" matched. |
| RQ-R1-Mohkatan | PASS | client_move_to, no fallback | Leg B nav_path: ok, 2 waypoints (808.30,55.24,518.70)->(807.48,55.24,515.40). Leg C: ok, 2 waypoints (807.48,55.24,515.40)->(806.27,55.22,517.24). Moh'Katan entity 100036 (tag Castle_Mohkatan) at (807.48,55.24,515.40); DHD_Frost entity 100000 (tag Castle_DHD) at (806.27,55.10,517.24). Player's current server position (804.98,55.20,516.69) matches the driver's leg C end, 1.5 m from the DHD, 2.6 m from Moh'Katan. | Leg B end (808.92,514.89) 1.5 m from Moh'Katan; leg C end 1.5 m from DHD. | skipped (lab_screenshot-20261103-081321.160.png) | Walks are trivial: leg B starts 4.8 m and leg C 3.3 m from target (within 6 m). Leg C final position is the only one confirmed against live server state. |

## Why entity_find returned count 0
Server entity names differ from what the driver searched: the NPC is named `Moh'Katan` (apostrophe, capital K; tag `Castle_Mohkatan`), the DHD is `DHD_Frost` (tag `Castle_DHD`). The searches "Mohkatan", "Castle_Mohkatan" (a server tag, not a name) and "Dial" cannot match those names. Driver/recipe problem (a wrong search term), not a product bug. Both entities exist at their expected spawn positions on the server.

## NEEDS_HUMAN
none (no look-and-feel clause in these rows; screenshots listed by name only: lab_screenshot-20261103-081257.359.png, lab_screenshot-20261103-081321.160.png).

## FINDINGS
1. Process: server log buffer too small to verify a window minutes later; verify immediately or via SigNoz (lab_run='dr-lab'). No product issue; no issue filed.
2. The brief's `space_id: 8` is stale; use 65537 for Castle.

## Recipe (next driver)
- Use entity_find names `Moh'Katan` (or template Moh'Katan), `DHD_Frost`, `Col Marsh (pet)`; or query by position.
- Nav space is 65537. Leg A waypoints: (807.24,55.18,521.70) then (808.30,55.24,518.70). Legs B and C are single 2-point hops.
