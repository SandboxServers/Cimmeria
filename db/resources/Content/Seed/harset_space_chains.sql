-- Harset space chains — ring switches and the Command Center doors
-- Worlds: 57 Harset, 68 Harset_CmdCenter
--
-- Every chain in this file is a line-for-line port of one of the two
-- surviving 2009 Atrea-generated space scripts. Nothing here is new
-- authoring; each chain's comment names the Python source line it came
-- from. The two scripts are the ONLY surviving Harset space logic —
-- they hold ring switches and one door, nothing else (see
-- docs/analysis/harset-rebuild/README.md "Purpose And Evidence
-- Boundary").
--
--   deprecated/python/cell/spaces/Harset.py          (70 lines)
--   deprecated/python/cell/spaces/Harset_CmdCenter.py (31 lines)
--
-- Chain ID ranges (allocated statically in
-- docs/analysis/harset-rebuild/work-packets.md to avoid seed-order
-- sensitivity; this file owns 6001-6099):
--   Ring switches:        6001-6005  (all five used)
--   Command Center doors: 6006-6007  (both used)
--   Relog restore:        6008-6020  (RESERVED, deliberately unused —
--     see "No relog-restore chains" below)
--
-- Evidence class: RECOVERED_SCRIPT for every row in this file — the seven
-- chains, their triggers and actions, the five tag/region-id pairings and
-- both arrival coordinates all come from the two Python scripts named
-- above. Nothing here is RECONSTRUCTION or NEW CONTENT.
-- Chain 6007 shipped `enabled = false` as H10's own safety call; placement
-- PL-A-06 flipped it to true after measuring the recovered coordinate
-- against the cooked map. The coordinate itself was never changed. The
-- reasoning is inline at that chain.
--
-- Packet: Harset H10, amended by placement PL-A-06. Base: main @ 3c1fed6c.

SET search_path = resources, pg_catalog;

-- ============================================================
-- RING SWITCHES — Harset.py lines 17-51
-- ============================================================
--
-- Five right-clickable ring consoles on the Harset plaza and its
-- outlying pads. The 2009 script subscribed each spawn tag to a
-- callback that looked the transporter up by region id and called
-- `region.interact(self.owner)`:
--
--     def interactCb(args):
--         region = self.owner.space.transporters.get(4)
--         if region is not None:
--             region.interact(self.owner)
--         return True
--     self.n2_lvar_TagSubscriptionId = self.owner.subscribe(
--         "entity.interact.tag::" + 'HarsetRingLeftBottom', interactCb, once = False)
--
-- `trigger_transporter` is the exact Rust equivalent: the executor arm
-- reaches `ring_transport::handle_interact`, which is what the Python's
-- `region.interact()` called. Cellblock chain 1043 is the shipped
-- precedent (castle_cellblock_chains.sql:830-846).
--
-- Tag → region id mapping verified three ways:
--   * Harset.py:19,26,33,40,47 (`transporters.get(N)`)
--   * spawnlist.sql spawns 4/127/128/129/130 (the tags, all template 3,
--     all world 57)
--   * ring_transport_regions.sql:29-53 (regions 4-8, world 57)
--
-- Region 8's `ring_transport_regions.tag` column reads
-- 'HarsetinRingRightRegion' — a typo in the shipped 2009 data. It is
-- COSMETIC and deliberately not "fixed": nothing matches on that
-- column. The ring FSM keys on `region_id`; these chains key on the
-- *spawn* tag ('HarsetRingRight'), which is spelled correctly.
--
-- `params` key is `regionId` — camelCase. The loader reads exactly that
-- key and falls back to `unwrap_or(0)` rather than dropping the action
-- (crates/content-engine/src/loader/action.rs:225-228), so a snake_case
-- typo here produces a silently dead switch with no authoring-time
-- signal. Do not rename it.
--
-- No conditions. The Python had no gate either — these are permanent
-- space furniture, not mission content. `ring_transport_regions.
-- required_mission_id` is NULL on all five, so the cell-runtime mission
-- backstop is never consulted. Cellblock chain 1008 is the shipped
-- precedent for a zero-condition space chain.
--
-- No `set_interaction_type` action: entity template 3 ("Ring
-- Transporter Switch", entity_templates.sql:49) already carries
-- `interaction_type = 32` = INT_RingNetwork as a TEMPLATE DEFAULT, and
-- the spawner reads that column onto every spawned entity. Seeding the
-- bit again would imply it needs setting and invite a later "fix" of a
-- non-bug. The linter's five allowlist entries
-- (crates/content-engine/tests/it/interact_tag_linter.rs) record this.
--
-- Behavioural note for UAT: a right-click OPENS THE DESTINATION LIST.
-- It does not teleport. The hop happens on the follow-up
-- `setRingTransporterDestination`. All five regions list the other four
-- as destinations, so every ring reaches every other ring.

-- Chain 6001: HarsetRingLeftBottom → transporter region 4. Harset.py:17-23.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (6001, 'Harset - Ring switch LeftBottom: open destination list for region 4', 'space', 57, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (6001, 'interact_tag', 'HarsetRingLeftBottom', 'player', false, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (6001, 'trigger_transporter', NULL, NULL, '{"regionId": 4}', 0, 0);

-- Chain 6002: HarsetRingRightBottom → transporter region 5. Harset.py:24-30.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (6002, 'Harset - Ring switch RightBottom: open destination list for region 5', 'space', 57, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (6002, 'interact_tag', 'HarsetRingRightBottom', 'player', false, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (6002, 'trigger_transporter', NULL, NULL, '{"regionId": 5}', 0, 0);

-- Chain 6003: HarsetRingLeft → transporter region 6. Harset.py:31-37.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (6003, 'Harset - Ring switch Left: open destination list for region 6', 'space', 57, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (6003, 'interact_tag', 'HarsetRingLeft', 'player', false, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (6003, 'trigger_transporter', NULL, NULL, '{"regionId": 6}', 0, 0);

-- Chain 6004: HarsetRingLeftTop → transporter region 7. Harset.py:38-44.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (6004, 'Harset - Ring switch LeftTop: open destination list for region 7', 'space', 57, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (6004, 'interact_tag', 'HarsetRingLeftTop', 'player', false, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (6004, 'trigger_transporter', NULL, NULL, '{"regionId": 7}', 0, 0);

-- Chain 6005: HarsetRingRight → transporter region 8. Harset.py:45-51.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (6005, 'Harset - Ring switch Right: open destination list for region 8', 'space', 57, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (6005, 'interact_tag', 'HarsetRingRight', 'player', false, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (6005, 'trigger_transporter', NULL, NULL, '{"regionId": 8}', 0, 0);

-- ============================================================
-- COMMAND CENTER DOORS — Harset.py:52-60, Harset_CmdCenter.py:17-25
-- ============================================================
--
-- Two `client_hinted_region` subscriptions that moved the player across
-- worlds. Both point sets carry `flags = 1` (REGION_FLAG_CLIENT_HINTED,
-- point_sets.sql:67,69), so the client is actually told about them.
--
-- The Python guarded on `args['entering']` with an empty `else`, so
-- `enter_region` (not `exit_region`, not both) is the faithful port —
-- RegionEnter and RegionExit are distinct trigger types with distinct
-- fire sites, so the guard is honored by construction.
--
-- WORLD GATING — a guardrail that is documented rather than enforced.
-- docs/analysis/harset-rebuild/README.md "Architecture Guardrails" says
-- these chains should carry a world condition because OnRegionEnter
-- ignores world. When this file was authored they could not: only six
-- condition types were authorable (mission_status, step_status,
-- archetype, objective_status, counter, stat_below_max). Packet H07 added
-- the seventh, `world`, and both door chains now carry it (rows below).
-- `Condition::PropertyEquals` still has no loader arm, and
-- `content_chains.scope_type`/`scope_id` remain documentary only —
-- nothing reads them at resolve time.
--
-- For legitimate play this is safe: the two region keys are
-- world-prefixed and byte-distinct, and each is unique across the whole
-- of point_sets.sql, so a player in one world cannot trip the other
-- world's chain. The residual hole is not name collision but that
-- `SpaceManager::get_region` is a world-global HashMap keyed on a
-- CLIENT-SUPPLIED region id — a crafted packet can hint the other
-- world's region from anywhere. Here that buys nothing (both doors are
-- unconditionally open to everyone), but the next `enter_region` chain
-- that grants or completes something will need a real gate.
--
-- Packet H07 (same PR) added the authorable `world` condition, so both
-- door chains below carry one: 6006 -> world eq 57 (Harset), 6007 ->
-- world eq 68 (Harset_CmdCenter). Shape: (chain_id, 'world', <world_id>,
-- NULL, 'eq', NULL, sort_order); `target_id` is resources.worlds.world_id.
-- Note a condition alone does NOT close the client-supplied-region-id
-- hole described above — `fire_interact_tag` never populates a world
-- param at all, so the dispatch-site check is still wanted (H06). See
-- docs/analysis/harset-rebuild/worknotes/H10.md, IR-1.
--
-- `cross_world_teleport` cannot set yaw (the executor sends
-- rotation [0,0,0]), so the player arrives facing world-north. The
-- Python's `moveTo` could not set yaw either — this is faithful, not a
-- regression.
--
-- No ping-pong by construction: each arrival point sits several units
-- outside the opposing world's trigger box (audit.md). A future M0
-- re-pin MUST preserve that clearance.

-- Chain 6006: Harset → Harset_CmdCenter. Harset.py:52-60, destination
-- from Harset.py:15 (`str2vec('0,0.355,-20')`).
--
-- ENABLED. World 68 has no navmesh file at all, so the movement
-- validator fails open on arrival — `is_position_valid` returns true
-- unconditionally with no mesh loaded, and bounds fall back to the
-- 20 km box. There is therefore no off-mesh failure mode to guard
-- against here, and the raw 2009 coordinate ships as recovered.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (6006, 'Harset - Command Center door: cross-world teleport to Harset_CmdCenter', 'space', 57, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (6006, 'enter_region', 'Harset.CommandCenterTransition', 'player', false, 0);

-- World gate (H07): only a player standing in Harset (57) may trip the
-- outbound door, whatever region id the client hints.
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (6006, 'world', 57, NULL, 'eq', NULL, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (6006, 'cross_world_teleport', NULL, 'Harset_CmdCenter',
        '{"x": 0.0, "y": 0.355, "z": -20.0}', 0, 0);

-- Chain 6007: Harset_CmdCenter → Harset. Harset_CmdCenter.py:17-25,
-- destination from Harset_CmdCenter.py:15 (`str2vec('0,-67.600,-231')`).
--
-- *** ENABLED by placement PL-A-06. The coordinate is UNCHANGED. ***
--
-- H10 shipped this disabled and asked M0 for a new coordinate. M0 was
-- cancelled; the door was unblocked instead by measuring the recovered
-- coordinate against the cooked map rather than replacing it
-- (docs/analysis/harset-rebuild/placements/A-arrival-and-travel.md,
-- row PL-A-06). Three things were open and all three are now answered:
--
-- 1. IS THERE A FLOOR THERE? Yes. `obj_slab` on the cooked Harset chunks
--    reports an up-facing surface at y -67.64 in the columns at
--    (0, -231), (-1, -231), (+1, -231) and (0, -230), with a second
--    floor sheet at -68.92 below it and the nearest ceiling at -61.13 —
--    6.5 m of headroom. The seeded y of -67.600 sits 0.04 m above that
--    floor. The point is neither inside geometry nor above a fall, which
--    was the only question the M0 walk was still needed for.
--    Independently confirmed afterwards by a second tool and method: the
--    Castle-nav session's rebuilt Harset mesh puts a walkable polygon
--    0.03 m under this exact point, in its main hub component. The 2009
--    coordinate is correct and always was.
-- 2. DOES IT PING-PONG? No. Point set 2078
--    ('Harset.CommandCenterTransition', the outbound door in world 57)
--    is an AABB spanning z -243.52..-238.41; the arrival at z -231 is
--    7.41 m north of its nearest face, so a returning player is not
--    standing in the outbound trigger and has to walk back into it
--    deliberately. Pinned by
--    `door_arrivals_and_respawner_sit_outside_the_opposing_trigger_box`.
-- 3. WHAT ABOUT THE NAVMESH? It is still off-mesh, and that is now a
--    navmesh defect rather than a reason to keep the door shut. Nothing
--    within ~20 m of this door is on-mesh at the real floor height:
--    `harset.nav`'s nearest polygon to the arrival is 28.6 m above it,
--    and at the door threshold itself 51.7 m above it. There is no
--    on-mesh alternative to move to, so "wait for an on-mesh
--    coordinate" was waiting on a mesh rebuild (H53 / GH1), not on a
--    playtest.
--
--    Off-mesh is survivable here for two independent reasons. World 57
--    is `navmesh_mode = 'advisory'` (H53), so the movement validator
--    fails open and the silent `CorrectionSuppressed` freeze H10 feared
--    cannot happen. And `respawners` row 20 now exists (placement
--    PL-A-02), so even if world 57 were flipped back to `enforce` the
--    failure mode is "arrive at the gate plaza" rather than "ghosted
--    player". Both halves are pinned by
--    `harset_return_arrival_is_offmesh_but_survivable`.
--
-- Enabling this also unparks mission 1361's acceptance trio (chains
-- 6511-6513 in harset_opcore_chains.sql) and its abandon twin 6528 — a
-- biconditional pinned by `praxis_acceptance_is_enabled_iff_the_return_
-- door_is`. Disabling 6007 again means disabling those four too.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (6007, 'Harset_CmdCenter - Harset door: cross-world teleport to Harset', 'space', 68, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (6007, 'enter_region', 'Harset_CmdCenter.HarsetTransition', 'player', false, 0);

-- World gate (H07): only a player standing in Harset_CmdCenter (68) may
-- trip the return door.
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (6007, 'world', 68, NULL, 'eq', NULL, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (6007, 'cross_world_teleport', NULL, 'Harset',
        '{"x": 0.0, "y": -67.6, "z": -231.0}', 0, 0);

-- ============================================================
-- No relog-restore chains (range 6008-6020 stays unused)
-- ============================================================
--
-- The campaign rule is "every chain that sets an interaction bit or
-- binds a dialog set has a `player_loaded` restore chain gated on the
-- active step" — needed because chain-set `interaction_type` bits live
-- in memory only and do not survive a server restart (see
-- castle_cellblock_chains.sql chains 1045/1046).
--
-- No chain in this file sets a bit or binds a dialog set. The ring
-- switches' INT_RingNetwork bit comes from entity template 3's
-- `interaction_type` column, which the spawner re-reads from the DB on
-- every spawn — it survives restart by construction. So H10 needs zero
-- restore chains, and the reserved 6008-6020 block is left free for a
-- later packet in this file's range.
