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

## Unblock probe (runner-DCA-v, 2026-11-09, Fxdcathree p2, coordinator Sonnet, 12 calls)
Cause found: the BLOCKED result was a fixture/brief positioning defect, not a product defect. Mission 638 is accepted by chains 1011 (non-Jaffa) / 1012 (Jaffa) on `enter_region Castle_Cellblock.Region2` (db/resources/Content/Seed/castle_cellblock_chains.sql:613-647), which also adds the prisoner dialog set (slot 17). Region2 is a BoundingBox (db/resources/Events/Seed/point_set_points.sql set 2033): x -311.19..-241.99, z -141.13..-115.19. The brief/fixture position (-287, 65.5, -113) and the prisoner (-289.81, 65.47, -113.14) are 2 m OUTSIDE it (z -113 > -115.19), and a fixture stage applies no enter event, so 638 was never accepted and the prisoner had no dialog.
Evidence (tier G for moves, N1 for the click): stage 689 completed + 638 not_active at (-300,.,-160); .gotolocation to (-287,65.5,-113) gave no 638 row; .gotolocation to (-287,65.5,-125) (inside Region2) gave sgw_mission 638 status 1 current_step 2114; .gotolocation to (-289.8,65.5,-109.8), client_target (N1) then right-click (N1) on Prisoner 329 (id 100914) opened DialogWin.
Recipe for a p8 rerun: after login, GOTO a point inside Region2 (e.g. -287 65.5 -125) first, wait ~3 s, then GOTO (-289.8 65.5 -109.8) and target/right-click the prisoner (coordinator does client_target). Grade stays PARTIAL until the rest of CB-U12a (accept, door button, minigame, 62060 rewards) is run; p9..p13 carry-over then is 638 accepted via this route.
