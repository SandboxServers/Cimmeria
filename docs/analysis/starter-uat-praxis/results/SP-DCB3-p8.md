PACKET SP-DCB3-p8 RUN 2026-11-09 by runner-DCA-u (Haiku driver a1 + coordinator takeover; no Sonnet verify); grade BLOCKED/PARTIAL
Local lab p2, Fxdcathree (Soldier), staged Castle_CellBlock (-287,65.5,-113), 689 completed, 638/639/62050/62060 not_active.

| Row | Grade | Tier | Evidence | Issue |
|---|---|---|---|---|
| CB-U12a (Boot, 638 play) | BLOCKED | N1 for click | Driver native right-click on Prisoner 329 (100865) failed at observe (avatar covered the target). Takeover: moved player to (-289.8,65.5,-109.8) by .gotolocation (G), client_target left-click confirmed target 100865 (N1), client_world_click right confirmed (N1), but no dialog opened (DialogWin and BlurbWin hidden). public.sgw_mission for the player shows 689 status 2 and 622 active at step 2113 (Zone load: accept mission), 567 active; no 638 row. Prisoner offered nothing, so 638 was never accepted. Door button click not run; minigame not run. | none filed |
| 62060 reward rows | NOT-RUN | | nothing reached | |

## Brief and harness defects
- Verify file covers 62060 but brief describes 638 (recorded as brief defect).
- Fixture with 689 completed and 638 not_active does not make the prisoner offer 638 (no dialog); the real 638 gating (chain binding replayed during play, per the brief) is not reproduced by the fixture. Next try: stage 638 at its first step, or play the real route (boot Livewire at spawn).
- Avatar covers the target when standing in line with the camera; stand 3.3 m to the side (north of prisoner) before client_target/world_click.
- lab-driver tool list lacks client_target and client_minigame_play; Haiku driver cannot run the minigame steps.
- lab_ensure_in_world returned immediately after staging (fixture applied; 622 active as at fresh play).
