# UB-U7: respawn at respawners 6 and 7

| Field | Value |
|---|---|
| Row | UB-U7: die near the gate, then near the infirmary (GM damage command). You respawn at respawner 6, then 7, never the origin |
| Runnable | Yes |
| Instance | `default` |
| Character | `Uat Fxbetahumdefa` |
| Fixture | F-H5 ((633.0, -59.83, 405.0), 6 m off respawner 6; 1423 completed) |
| Budget | 25 calls |
| Piggyback | none |

**From the seeds.** World 23 has two respawners: 6 'Respawner' at (633, -60.83, 399) by the gate plaza and 7 'Respawner_bet_Infirmary' at (953, 16.35, 287.5) inside the Infirmary region. With no respawner named, the server picks the one nearest the death position. So a death at the fixture point must bring you back at 6, and a death 6 m from 7 at 7.

**The death.** The lab's recipe: god mode off, aggro on, five hostile spawns next to you (`.spawnrandom 74 5 5 3`), then `client_die_and_respawn` with health set to 1 and Release. God mode goes back on right after each respawn.

## Orchestrator before the run

Stage F-H5 and dispatch at once.

## Driver steps

1. `lab_ensure_in_world {"lease_id":L,"character":"Uat Fxbetahumdefa"}`. WAIT17. DIALOG. Expect (633.0, -59.83, 405.0).
2. CHATX(`/gmsetgodmode 0`, `.aggro on`, `.spawnrandom 74 5 5 3`).
3. `client_die_and_respawn {"lease_id":L,"setup_health":1,"respawn":"release","defeat_timeout_ms":60000,"respawn_timeout_ms":120000}`. Record the result, including any position it reports.
4. WAIT17. Record the position. SHOT(`u7-respawn-1`).
5. CHATX(`/gmsetgodmode 1`, `.aggro off`).
6. TP(953.0, 17.35, 293.5) (6 m off respawner 7).
7. CHATX(`/gmsetgodmode 0`, `.aggro on`, `.spawnrandom 74 5 5 3`).
8. Same call as step 3. Record the result.
9. WAIT17. Record the position. SHOT(`u7-respawn-2`).
10. CHATX(`/gmsetgodmode 1`, `.aggro off`).

## Stop conditions

- Step 3 or 8 reports no defeat within its timeout: record it, run CHATX(`/gmsetgodmode 1`, `.aggro off`) and stop. The row is `not-run` (no death), not FAIL.
- The character dies again after a respawn: record it, run step 5's CHATX and go on.

## Expected and pass/fail

| Check | Pass when | Evidence |
|---|---|---|
| A. Gate death | Step 4 position within 3 m of (633, -60.83, 399) | step 4, `u7-respawn-1`, server log |
| B. Infirmary death | Step 9 position within 3 m of (953, 16.35, 287.5) | step 9, `u7-respawn-2`, server log |
| C. Never the origin | Neither respawn is at or near (0, 0, 0) or the world's default spawn | steps 4 and 9 |

The row passes on A, B and C.

## Orchestrator only: verify list

- Two deaths for this character, each followed by `Respawn: no usable respawner was named -- using the one nearest the death position` and the respawner id it chose (6, then 7).
- No `spawn_world_unknown` event.
- The respawn positions in the log match steps 4 and 9.
