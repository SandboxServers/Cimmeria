# SP-DCA7-p1 results: Infirmary barks (HB1, HB2)

Verified 2026-11-03 by Sonnet verifier; driver attempts: 1 (Haiku, instance p2). Server evidence window 14:04:29Z-14:06:57Z, from the dr-lab `logs/server.log` (the 500-entry ring was not used) and read-only DB queries. Characters differ from the brief: Fxdcatwo (player 105, Soldier) and Fxdcafour (player 107, Jaffa, archetype 8).

| Row | Result | Tier | Server evidence | Client evidence | Looker | Notes |
|---|---|---|---|---|---|---|
| HB1 | PASS (look-and-feel NEEDS_HUMAN) | N1 move_to walk, no fallback | Fixture staged 14:03:15Z; `sgw_mission` p105: 704 and 1520 status 2 (completed); archetype 1 Soldier. Entry 1: `fire_enter_region` Castle.Infirmary 14:04:43.276Z then `Content: npc bark` chain 10200, speaker Ogilvie, screen 10900, channel 0, 14:04:43.277Z. Entry 2: `fire_enter_region` matched 14:05:06.557Z with NO bark. One bark total. No dialog event. | One B1 line (say channel, speaker Ogilvie, seq 201334); none after since_seq 201338; DIALOG none | not run (skipped by instruction) | Bookmarks HB1 start 14:04:34Z, end 14:05:15Z present |
| HB2 | PASS (look-and-feel NEEDS_HUMAN) | N1 move_to walk, no fallback | Fixture applied 14:05:25Z at relog; archetype 8 Jaffa; `sgw_mission` p107: 704 and 1521 completed, no 1520 row (not_active) after the window. `fire_enter_region` 14:06:26.333Z then `npc bark` chain 10201, speaker Castle Medic, screen 10901, 14:06:26.334Z. Exactly one bark; none of chain 10200 for this player. No accept_mission or offer for player 107. | One B2 line (seq 207020); "Ambernol" read gave only the old HB1 line (no new line) | not run | Bookmarks HB2 start 14:06:18Z, end 14:06:36Z |

## Findings
1. WARN `tree_missing` / `archetype_has_no_tree` for Jaffa (player 107, archetype 8), 14:05:25Z and 14:05:34Z, outside the bark rows' scope. Known Jaffa-start gap; not filed. No product bug (a)/(b) found, so no issue file was written.
2. Driver nit (c): the HB2 "Ambernol" read omitted since_seq, so it re-returned the HB1 line; server rows settle it.
3. Driver nit (c): HB1 end bookmark was taken before the re-entry wait ended in one batch; server shows re-entry at 14:05:06Z, so the no-repeat clause is still proven server-side.

## NEEDS_HUMAN (sign-off sheet)
HB1 | lab_screenshot-20261103-080454.318.png | not inspected by driver; client read says channel None, colour white, speaker Ogilvie in Inst9Chat | looker not run | cannot tell
HB2 | lab_screenshot-20261103-080641.359.png | not inspected by driver; speaker Castle Medic, colour white | looker not run | cannot tell

## NOT RUN / BLOCKED
Looker skipped by instruction. Nothing blocked.

## Recipe
- Evidence source: `Select-String` on the dr-lab `logs\server.log` for `npc bark|fire_enter_region` with a timestamp prefix filter; `server_log_tail` ring is too short.
- Fixture check: `SELECT ... FROM public.sgw_mission WHERE player_id IN (105,107)`; status 2 = completed; not_active = no row. Archetype from the `Loaded player data for mapLoaded` log line (sgw_player has no archetype_id column).
- Drive: stage fixture, `move_to` (358,70.2,893) walking from (358,70.2,912); re-entry by leaving to z=912 and walking back; chat read with since_seq. Do not relog between barks of the same character (relog would not reset the once-only flag unless the server restarts).
