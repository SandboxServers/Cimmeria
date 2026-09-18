--
-- TOC entry 3227 (class 0 OID 63067)
-- Dependencies: 244
-- Data for Name: respawners; Type: TABLE DATA; Schema: resources; Owner: -
--

INSERT INTO respawners (respawner_id, world_id, name, pos_x, pos_y, pos_z) VALUES (1, 8, 'Checkpoint Alpha Respawn', 0, 0, 0);

INSERT INTO respawners (respawner_id, world_id, name, pos_x, pos_y, pos_z) VALUES (2, 8, 'Throne Checkpoint Respawn', 0, 0, 0);

INSERT INTO respawners (respawner_id, world_id, name, pos_x, pos_y, pos_z) VALUES (3, 8, 'Op-Core Triangle Respawn', 0, 0, 0);

INSERT INTO respawners (respawner_id, world_id, name, pos_x, pos_y, pos_z) VALUES (4, 8, 'Armory Respawn', 0, 0, 0);

INSERT INTO respawners (respawner_id, world_id, name, pos_x, pos_y, pos_z) VALUES (5, 12, 'Level 7: Ring Transporters', -79.2139969, 45.1759987, -163.815002);

INSERT INTO respawners (respawner_id, world_id, name, pos_x, pos_y, pos_z) VALUES (6, 23, 'Respawner', 0, 0, 0);

INSERT INTO respawners (respawner_id, world_id, name, pos_x, pos_y, pos_z) VALUES (7, 23, 'Respawner_bet_Infirmary', 0, 0, 0);

INSERT INTO respawners (respawner_id, world_id, name, pos_x, pos_y, pos_z) VALUES (8, 12, 'Stasis Chamber', -334.231, 73.472, -228.026);

-- ── Harset (packet H10) ────────────────────────────────────────────────
-- Ids 20-23 are reserved for the Harset rebuild campaign
-- (docs/analysis/harset-rebuild/work-packets.md "Worker Input And
-- Ownership"). Only row 21 is seeded here.
--
-- Rows 20 (world 57 Harset), 22 (69 Harset_Market) and 23
-- (70 Harset_StorageRm) are DELIBERATELY ABSENT: no coordinate for them
-- exists in any recovered script or seed, and the campaign forbids
-- inventing one. They are seeded after the M0 in-client placement
-- session pins them. Castle CA00's zero-coordinate guard in
-- `resolve_respawn_target` is what keeps the absence honest — a (0,0,0)
-- placeholder would be silently "valid" and is worse than no row.
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

