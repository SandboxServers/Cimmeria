--
-- TOC entry 3227 (class 0 OID 63067)
-- Dependencies: 244
-- Data for Name: respawners; Type: TABLE DATA; Schema: resources; Owner: -
--

-- World 8 (Castle) rows 1-4 shipped with pos (0,0,0) in the recovered data:
-- the names survived, the coordinates did not. Dying anywhere in Castle put
-- the player at the world origin, because `resolve_respawn_target`
-- (crates/cell-interactions/src/cell/respawn/mod.rs) found a
-- matching row and returned its zeros — the safe fallbacks below it were
-- unreachable precisely because the rows existed. See
-- docs/analysis/castle-rebuild/audit.md defect B1 and decision D-CA11.
--
-- The coordinates below are authored (D-CA11) from the surrounding world-8
-- seed geometry, NOT recovered. `resolve_respawn_target` now also skips any
-- respawner still sitting at the origin, so a future unauthored row degrades
-- to the fallback instead of stranding the player.

-- RECONSTRUCTION, provisional until in-client UAT.
-- Checkpoint Alpha is the Marsh / Moh'katan plateau (y ~= 55.2). Placed on the
-- line between the two authored groups of world-8 actors there — the Praxis
-- Jaffa guards (spawnlist 121/123/124, x 786-790) and the
-- Marsh / Moh'katan / Jaffa-Lieutenant group (spawnlist 118/120/119,
-- x 807-811) — so the point is inside the footprint those NPCs stand on
-- rather than extrapolated past its edge, where a wall or a drop is as
-- likely as floor (and `unstuck` is still unimplemented). Nearest authored
-- actor is the Castle_DHD prop (spawnlist 2, 806.27/55.10/517.24) at ~7.6
-- units; every NPC is >= 7.8 away. Proximity to the faction-1 Jaffa is safe
-- as seeded: since NA13 an NPC aggroes on sight only when it is HOSTILE to
-- players (its spawn's `aggression_override`, else the faction reaction
-- table), and faction 1 reacts FRIENDLY to players (faction 3).
INSERT INTO respawners (respawner_id, world_id, name, pos_x, pos_y, pos_z) VALUES (1, 8, 'Checkpoint Alpha Respawn', 800, 55.2099991, 513);

-- RECONSTRUCTION, provisional until in-client UAT.
-- Inside point set 2049 `Castle.ThroneRoom` (point_set_points 2314-2317:
-- x 331.4-395.9, z 617.2-683.2). Biased to the west half of the box because
-- that is where the floor height is evidenced: both west corners sit at
-- y 41.13-41.15 and the Castle_AccessPanel spawn (spawnlist 92) sits at
-- 330.49/41.18/653.11 just outside the west edge, while both east corners are
-- at y 48.19 (a dais or ramp of unknown extent). A box-centre point would
-- have to guess which side of that step it lands on.
INSERT INTO respawners (respawner_id, world_id, name, pos_x, pos_y, pos_z) VALUES (2, 8, 'Throne Checkpoint Respawn', 345, 41.2000008, 650);

-- RECONSTRUCTION, PROVISIONAL -- pending CA05 recon.
-- The Op-Core Triangle has not been located in the map data yet: nothing in
-- the seed (point set, spawn, region, sequence) is named for it. This is a
-- placeholder, not a located checkpoint -- a corner point of point set 2051
-- `Castle.Infirmary` (point_set_points 2325), chosen because it is authored
-- region geometry on the y ~= 70.1-70.3 interior level that the Coppleman /
-- PRU corridor NPCs (spawnlist 87/90/114, 20-40 units away) stand on, so it
-- is very likely walkable. Replace with the real position once CA05's
-- sublevel recon lands.
INSERT INTO respawners (respawner_id, world_id, name, pos_x, pos_y, pos_z) VALUES (3, 8, 'Op-Core Triangle Respawn', 371.037994, 70.1490021, 904.648987);

-- RECONSTRUCTION, provisional until in-client UAT.
-- The Armory arrival platform, taken verbatim from two independent authored
-- world-8 rows that already use it as a destination: ring_transport_regions
-- 34 `Castle_ArmoryRingDropZone` (point set 2081) and the cross_world_teleport
-- target of Castle_CellBlock chain 1109 (mission 688's hand-off to Castle).
-- Every player who reaches Castle through the Cellblock arrives standing on
-- exactly this point, so it is the best-evidenced walkable spot in the zone.
INSERT INTO respawners (respawner_id, world_id, name, pos_x, pos_y, pos_z) VALUES (4, 8, 'Armory Respawn', 466.365, 70.397, 991.466);

INSERT INTO respawners (respawner_id, world_id, name, pos_x, pos_y, pos_z) VALUES (5, 12, 'Level 7: Ring Transporters', -79.2139969, 45.1759987, -163.815002);

-- KNOWN GAP (CA00): world 23 `Beta_Site_Evo_1` has the same lost-coordinate
-- problem as the Castle rows above, and no evidence to rebuild from — the
-- recovered data has no Beta Site respawner position, the names give no
-- location ('Respawner_bet_Infirmary' names a room that has no point set,
-- spawn or region in the seed), and no `deprecated/python` script reads or
-- writes them. Deliberately left at the origin rather than invented: the
-- `resolve_respawn_target` origin guard makes them fall through to the
-- in-place fallback, so dying in world 23 respawns the player where they
-- died instead of at (0,0,0). Author them from a `.location` pass if Beta
-- Site ever becomes a playable zone; the live-DB guard
-- `seeded_respawners_are_not_at_the_world_origin` lists them by id as the
-- documented exception.
INSERT INTO respawners (respawner_id, world_id, name, pos_x, pos_y, pos_z) VALUES (6, 23, 'Respawner', 0, 0, 0);

INSERT INTO respawners (respawner_id, world_id, name, pos_x, pos_y, pos_z) VALUES (7, 23, 'Respawner_bet_Infirmary', 0, 0, 0);

INSERT INTO respawners (respawner_id, world_id, name, pos_x, pos_y, pos_z) VALUES (8, 12, 'Stasis Chamber', -334.231, 73.472, -228.026);

-- ── Harset (packets H10, PL-A) ─────────────────────────────────────────
-- Ids 20-23 are reserved for the Harset rebuild campaign
-- (docs/analysis/harset-rebuild/work-packets.md "Worker Input And
-- Ownership"). All four are now seeded.
--
-- Rows 20, 22 and 23 are PLACED FROM MAP DATA, not walked in-client —
-- the owner stopped waiting for the M0 session and asked for labelled
-- estimates instead (docs/analysis/harset-rebuild/placements/METHOD.md).
-- Each row's evidence and confidence is in the ledger at
-- placements/A-arrival-and-travel.md (PL-A-02, PL-A-03, PL-A-04);
-- correct them there and here in one edit after a playtest.
--
-- Row 21: world 68 Harset_CmdCenter. The coordinate is the Command
-- Center door's arrival point, recovered from
-- deprecated/python/cell/spaces/Harset.py:15 (`str2vec('0,0.355,-20')`)
-- — not invented. Two properties worth preserving across any future
-- re-pin:
--   * World 68 has no navmesh, so `resolve_recovery_position`'s
--     `is_point_valid` check on this candidate passes trivially; without
--     this row a death in the council room falls back to respawning in
--     place at the death position with a `warn!`.
--   * The point sits several units clear of point set 2079
--     ('Harset_CmdCenter.HarsetTransition'), so a player who respawns
--     here is NOT standing in the return-door trigger box and is not
--     instantly teleported back to Harset.
INSERT INTO respawners (respawner_id, world_id, name, pos_x, pos_y, pos_z) VALUES (21, 68, 'Command Center Respawn', 0, 0.355, -20);

-- PLACEMENT PL-A-02, provisional until a playtest.
-- Row 20: world 57 Harset. The gate plaza, 3 m west of where the gate
-- arrival pin (stargates.stargate_id = 3) stood until NA29 dropped it, so a
-- hub death returns the player beside the gate; travellers now arrive on the
-- gate row itself, about 9 m east on the dais. Evidence: MAP-GEOMETRY + MAP-LANDMARK.
--   * (-8.0, 34.0) is interior to navmesh component 187 — the 24,770 m^2 hub
--     component — and so is every sample on the 0.6 m and 1.2 m rings around
--     it (37/37). That matters more here than for the gate pin: this row is
--     also the recovery candidate `nearest_valid_respawner` hands an off-mesh
--     world-57 arrival, so a row that the mesh rejects is no recovery at all.
--   * y -68.99 is the topmost up-facing surface obj_slab reports in the
--     column at (-8.0, 34.0); a lower floor sheet sits at -69.31.
--   * Clear of point set 1001 'Harset.Stargate' (2.5 m cylinder at
--     (-0.372, -67.364, 37.353)) by 8.33 m and of point set 2078
--     'Harset.CommandCenterTransition' (z -238..-244) by the length of the
--     zone, so respawning triggers neither volume.
-- NOT placed at the zone's busiest point: (1.0, -68.92, 2.9) — the plaza
-- gateway between the two GA-GuardPost00 props — carries 6,049 of the 28,988
-- reject rows that quote a real accepted position, the largest of the 38
-- distinct ones, with four more clean anchors within 5 m. (It survives the
-- synthetic-point filter: only (0,0,0) and (1,1,1) are excluded for Harset,
-- so the round x = 1.00 is a coincidence.) It is in navmesh component 1028,
-- one of the 1,939 fragments the H53 defect leaves behind, so it would not
-- serve as an arrival-recovery candidate and nothing could path to it. On the
-- rebuilt mse13 mesh that whole cluster joins the hub component, so this is
-- the obvious re-pin the day harset.nav is rebuilt.
INSERT INTO respawners (respawner_id, world_id, name, pos_x, pos_y, pos_z) VALUES (20, 57, 'Harset Gate Plaza Respawn', -8, -68.989999999999995, 34);

-- PLACEMENT PL-A-03, provisional until a playtest.
-- Row 22: world 69 Harset_Market. Evidence: MAP-GEOMETRY + MAP-LANDMARK,
-- floor only — **world 69 has no navmesh file at all**, so no on-mesh or
-- reachability check was possible and none is claimed.
--   * y 3.61 is the interior floor of the market building: obj_slab reports it
--     as the topmost up-facing surface in the columns at (48, 78), (47, 77)
--     and (49, 79), on a 16 m^2 / 34-triangle patch spanning x[46, 50]
--     z[76, 80], with nothing overhead. The same 3.61 floor reads across
--     x 12..96, z 6..96 wherever the roof sheet (y 17.3-19.2) does not
--     obscure the column.
--   * 3.0 m from the authored `EM-StandingLight05` floor lamp at
--     (47.96, 3.48, 81.04) — a floor-standing prop, so its base height
--     independently corroborates the 3.61 floor, and a lit spot is a walkable
--     one.
-- The doors between the hub and this world are not seeded yet (H14), so this
-- row is not positioned relative to an entrance; re-pin it near the door once
-- the door exists.
INSERT INTO respawners (respawner_id, world_id, name, pos_x, pos_y, pos_z) VALUES (22, 69, 'Harset Market Respawn', 48, 3.6099999999999999, 78);

-- PLACEMENT PL-A-04, provisional until a playtest.
-- Row 23: world 70 Harset_StorageRm. Evidence: MAP-GEOMETRY, with an on-mesh
-- check — world 70 *does* have a navmesh (`data/spaces/harset_storagerm.nav`)
-- and, unlike world 57, it is left at the default `enforce` mode, so a row the
-- mesh rejects would be filtered straight back out by
-- `nearest_valid_respawner`.
--   * (50.0, 44.0) is interior to component 36 of harset_storagerm.nav —
--     3,055 m^2 / 314 polys at y 0.2..1.2, x[16.1, 87.5] z[34.1, 99.2], which
--     is the storage room's own floor rather than the 82,249 m^2 ground sheet
--     (component 0) that spans the whole map.
--   * y 0.00 exactly: obj_slab's column at (50, 44) reports the up-facing
--     floor at 0.00, and all nineteen authored GA-Fence00/GA-Fence03 props in
--     this map sit at y 0.000, so the fences and the floor agree.
--   * z 44 is inside the fence-free band: every fence prop has z >= 51.2, so
--     the row is not inside the crate maze that fills the south half.
INSERT INTO respawners (respawner_id, world_id, name, pos_x, pos_y, pos_z) VALUES (23, 70, 'Harset Storage Room Respawn', 50, 0, 44);

-- ── Interior and story rooms (2026-09-27) ─────────────────────────────
-- These worlds had no stargate, ring pad, spawn or respawner, so
-- `.gotolocation <world>` had nowhere to put a GM and dying inside one
-- respawned the player in place. Every cooked map below was searched for
-- PlayerStart / SGWTeleporter / SGWStargate / RingTransporter actors and
-- has none, so each row is PLACED FROM MAP DATA, provisional until a
-- playtest (same method as the Harset rows above). Conversion
-- BigWorld = (UE.y, UE.z, UE.x) / 100 (`ue3_to_bw`); floor heights from
-- `obj_slab`. Each point passes `nav_inspect` at the default tolerances,
-- as do 36 samples on rings 0.6 / 1.2 / 2.4 m around it, all in the same
-- component. All six worlds are navmesh_mode 'advisory'.

-- Row 24: world 20 Agnos_Library. MEDIUM. Main hall on the building's
-- centre line just south of the central dome; floor y 0.32. Component 20
-- of agnos_library.nav (7,606 m^2), which holds 230 of the map's 276 cover
-- nodes; the mesh's largest component is the outside terrain sheet. The
-- likely front entrance is the small south hall near (1.7, 0.2, -45),
-- component 18, not directly joined to the main hall.
INSERT INTO respawners (respawner_id, world_id, name, pos_x, pos_y, pos_z) VALUES (24, 20, 'Agnos Library Hall Respawn', 1.7, 0.37, 45);

-- Row 25: world 62 Dakara_E1_StoryRm. MEDIUM. Centre of the tent interior
-- (x 57..84, z 17..43); floor y 0.00; component 3 of dakara_e1_storyrm.nav
-- (381 m^2), about 5 m clear of the floor props (braziers, the skeletal
-- meshes near (70..72.6, 24..25)).
INSERT INTO respawners (respawner_id, world_id, name, pos_x, pos_y, pos_z) VALUES (25, 62, 'Dakara Story Room Respawn', 71, 0.05, 30);

-- Row 26: world 80 Omega_Site_CmdCenter. HIGH. 3.9 m east of the ring pad
-- a traveller from Omega Site arrives on (ring_transport_regions 17,
-- `OmegaSiteCmdCenterRegion`, at (6.121, 35.23, 9.987)), outside its 2.53 m
-- trigger cylinder so arriving does not stand the player in the pad.
-- Component 11 of omega_site_cmdcenter.nav (9,144 m^2), same as the pad.
INSERT INTO respawners (respawner_id, world_id, name, pos_x, pos_y, pos_z) VALUES (26, 80, 'Omega Command Center Respawn', 10, 35.23, 9.987);

-- Row 27: world 88 Tollana_Curia. The cooked map is a STUB: one flat
-- 100 x 100 m terrain tile at y 0, no actors, no room geometry. The point
-- is the centre of that tile (the mesh's only component) — standable, but
-- a GM lands on an empty plain. Re-place it if the Curia is ever rebuilt.
INSERT INTO respawners (respawner_id, world_id, name, pos_x, pos_y, pos_z) VALUES (27, 88, 'Tollana Curia Respawn', 50, 0.05, 50);

-- Row 28: world 50 Sewer_Falls. LOW-MEDIUM. 9 m inside the map's only
-- doorway marker (`SGW_Weather:DoorwayPrecipitationPlanes` at
-- (-429.12, 12.12, 190.16), beside `LUS-JunkyardHut00`); floor y 12.15;
-- component 1 of sewer_falls.nav (77,937 m^2, the main area).
INSERT INTO respawners (respawner_id, world_id, name, pos_x, pos_y, pos_z) VALUES (28, 50, 'Sewer Falls Respawn', -429.1, 12.2, 181);

-- Row 29: world 2 SandBox. HIGH. SandBox plays the Harset_CmdCenter
-- client map and sandbox.nav is byte-identical to harset_cmdcenter.nav, so
-- this is row 21's Command Center door arrival, reused.
INSERT INTO respawners (respawner_id, world_id, name, pos_x, pos_y, pos_z) VALUES (29, 2, 'SandBox Respawn', 0, 0.355, -20);
