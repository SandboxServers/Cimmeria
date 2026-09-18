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
-- The one thing NOT recovered is chain 6007's `enabled = false`, which is
-- this packet's own safety call and is justified inline at that chain.
--
-- Packet: Harset H10. Base: main @ 3c1fed6c.

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
-- (crates/content-engine/tests/interact_tag_linter.rs) record this.
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
-- ignores world. They do not, because they CANNOT: only six condition
-- types are authorable (mission_status, step_status, archetype,
-- objective_status, counter, stat_below_max —
-- crates/content-engine/src/loader/condition.rs:12-69).
-- `Condition::PropertyEquals` exists in the enum but has no loader arm,
-- so a `content_conditions` row naming it is dropped with a `warn!`,
-- and `content_chains.scope_type`/`scope_id` below are documentary only
-- — nothing reads them at resolve time.
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
-- *** FOLLOW-UP: packet H07 is adding an authorable world condition. ***
-- When it lands, both chains below should gain one:
--   6006 -> world eq 'Harset'
--   6007 -> world eq 'Harset_CmdCenter'
-- (exact condition_type / value spelling per H07's loader arm). Note a
-- condition alone does NOT close the client-supplied-region-id hole
-- described above — `fire_interact_tag` never populates a world param at
-- all, so the dispatch-site check is still wanted. See
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

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (6006, 'cross_world_teleport', NULL, 'Harset_CmdCenter',
        '{"x": 0.0, "y": 0.355, "z": -20.0}', 0, 0);

-- Chain 6007: Harset_CmdCenter → Harset. Harset_CmdCenter.py:17-25,
-- destination from Harset_CmdCenter.py:15 (`str2vec('0,-67.600,-231')`).
--
-- *** DISABLED — awaiting an M0 in-client coordinate pin. ***
--
-- Unlike 6006's destination, this one lands in world 57, which DOES
-- have a navmesh (data/spaces/harset.nav). `NavMesh::is_point_valid`
-- returns FALSE for (0, -67.600, -231) — verified by the
-- `harset_return_coordinate_is_off_mesh` guard in
-- crates/services/src/cell/content/chain_replay_tests/harset_space.rs,
-- which loads the real .nav and asserts the verdict. The audit had
-- measured ~27 units off-mesh by vertex proximity; a Detour query
-- agrees.
--
-- Shipping this enabled would strand the player: `resolve_recovery_
-- position` has nothing to offer in world 57 (reprojection fails at
-- ±3 unit search extents, the AABB clamp declines because the point is
-- inside the mesh bounds, and no world-57 respawner row is seeded —
-- respawners row 20 also waits on M0). The result is a silent
-- `CorrectionSuppressed`: the player moves on their own screen, never
-- moves for witnesses, and gets no error. A visibly dead one-way door
-- is a strictly better failure than a silently ghosted player.
--
-- The coordinate is NOT adjusted here. Campaign rule: every Harset
-- coordinate is either recovered from the Python or pinned in-game;
-- inventing one is out of scope for this packet. M0 must replace the
-- x/y/z below AND flip `enabled` to true in the same change, and must
-- keep the ~8 units of clearance from point set 2078's trigger box so
-- the door does not ping-pong. The replay guard asserts BOTH the
-- off-mesh verdict and `enabled = false`, so the two cannot drift.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (6007, 'Harset_CmdCenter - Harset door: cross-world teleport to Harset (DISABLED: arrival off-mesh, awaiting M0 pin)', 'space', 68, false, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (6007, 'enter_region', 'Harset_CmdCenter.HarsetTransition', 'player', false, 0);

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
