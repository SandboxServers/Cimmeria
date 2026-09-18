--
-- TOC entry 3227 (class 0 OID 63067)
-- Dependencies: 244
-- Data for Name: respawners; Type: TABLE DATA; Schema: resources; Owner: -
--

-- World 8 (Castle) rows 1-4 shipped with pos (0,0,0) in the recovered data:
-- the names survived, the coordinates did not. Dying anywhere in Castle put
-- the player at the world origin, because `resolve_respawn_target`
-- (crates/services/src/cell/cell_methods/player/combat/respawn.rs) found a
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
-- as seeded: NPC auto-aggro requires `aggression > 0`, which no seeded
-- template sets (it is only raised by the `set_aggression` content action or
-- the GM console).
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

