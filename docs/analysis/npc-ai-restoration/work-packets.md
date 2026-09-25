# NPC AI Restoration Work Packets

> Type: how-to. Audience: the coordinator and packet workers.
> Updated: 2026-09-24. Companions: [launch prompt and decisions](README.md), [audit](audit.md), [telemetry plan](telemetry.md), [testing playbook](../../../TESTING.md), [parity ledger dispatch rules](../legacy-command-parity/work-packets.md#dispatch-rules).

## Dispatch rules

This ledger reuses the dispatch, ownership, worknote/handoff and acceptance rules of the [legacy command parity ledger](../legacy-command-parity/work-packets.md#dispatch-rules).

- Worktrees live under `.claude/worktrees/npcai-<packet>`; branches are `npcai/<packet>-<slug>`.
- Every cargo call goes through the shared build lane, and live-DB tests use the worktree's own `sgw_<worktree>` database (see the build-lane notes in [castle-rebuild/README.md](../castle-rebuild/README.md#implementation-session-record-2026-09-17)).
- Worktrees need the `external/` junction. Remove it with `cmd /c rmdir external` before deleting the worktree.
- Run clippy on the CI toolchain (`cargo +<CI stable> clippy`), not the machine default.
- Initial state: documentation only, against `main` @ `b0b594e9`. No packet has started.

Status vocabulary: **Ready**, **BlockedDependency**, **BlockedDecision** (needs a D-NA answer from [README.md](README.md#decisions)), **BlockedEvidence**, **Writing**, **Review**, **Integrated**, **UATPending**, **Done**.

Writers and advisors run as defined in `.claude/agents/`, with no model overrides. `rust-gameserver-dev` is the default writer. `testing-validation-engineer` reviews every regression strategy, and `documentation-writer` reviews the doc updates each packet owes (see the [CLAUDE.md doc map](../../../CLAUDE.md)).

**Contended files.** Serialise edits to these through the coordinator:

- `npc_ai/fight.rs`: NA00, NA02, NA10, NA11, NA12, NA13, NA15, NA22.
- `ticks/npc_movement.rs`: NA01, NA02, NA10, NA11.
- `crates/entity/src/navigation/mod.rs`: NA01, NA02, NA15.

Land NA00 first, because every later packet uses its `set_ai_state` helper and its naming. `fight.rs` is already large, so any packet that grows it past the 700-line hard cap splits it along the fight-phase seams (target selection / leash / chase / attack / cover) in the same PR.

## Common acceptance

- Every behaviour change ships a regression guard that **fails when the fix is reverted** (TESTING.md). For NPC AI that is usually one of:
  - a unit test on the pure decision function;
  - a `SpaceManager`-level tick test that runs `npc_ai_tick` and `npc_movement_tick` for N ticks over a real navmesh (`crates/entity/tests/castle_navmesh.rs` and the `navigation` unit tests already load the shipped `castle.nav` and `castle_cellblock.nav`);
  - a `LogCapture` negative-log test for each new WARN.
- Every new log target is added to `OTEL_FILTER` with a pinning assertion (telemetry.md §Conventions).
- Every packet ends with a UAT line that names the SigNoz query from [telemetry.md §3](telemetry.md#3-live-session-runbook) the owner runs after playing.

## Phase 0: telemetry first

### NA00

**Status:** Review (branch pushed, d5b9d5f8). **Scope title:** Telemetry plumbing, the AI state-transition helper, and deploy identity. **Depends:** land or rebase PR #726 first (it rewrites the path-fail messages and the `destroy_space` throttle cleanup). **Advisor:** npc-ai-spawn-advisor, testing-validation-engineer.

**Entries:**

- `crates/server/src/logging.rs` (`OTEL_FILTER` and its test) and `otel.rs` (resource attributes);
- the env-var table in `crates/server/src/main.rs`;
- every `ai_state =` write: `aggro.rs`, `fight.rs`, `leash.rs`, `dispatch.rs`, `lifecycle/`, `ticks/npc_respawn/`, and the content executor's `set_npc_poi` / follow;
- `content/executor/world/mod.rs:84-97`.

**Scope:**

- [telemetry.md §1](telemetry.md#1-filter-and-identity-fixes-na00) in full.
- A `set_ai_state(npc, to, reason)` helper, through which all state writes go, emitting `npc_ai.transition` and its counter.
- `npc_ai.aggro event=acquired` with `cause`.
- Rename the misleading `dist_to_spawn` fields (T7).
- `set_aggression_tag_miss`.
- **No behaviour change.**

**Acceptance:**

- A unit test that every `AiState` write in the crate goes through the helper: a grep-style test, or make the field private with a setter.
- LogCapture tests for the transition row and the tag miss.
- The filter test pins every new target.
- UAT: one colo session; SigNoz shows `cimmeria.deploy_env='colo'` and `wire.out.avatar_update` rows.

### NA01

**Status:** Review (branch pushed, 8bbfb905). **Scope title:** Storey-aware navmesh height query (M4). **Depends:** none (it touches the `entity` crate only). **Advisor:** movement-teleport-advisor.

**Entries:** `crates/entity/src/navigation/mod.rs:394-434` (`get_height_at`); its callers `npc_movement.rs:211`, `spawner/npcs.rs`, `console/bookmark.rs:171`, and `grep get_navmesh_height`.

**Scope:**

- Add `get_height_near(x, y_ref, z) -> Option<f32>`: `findNearestPoly` centred at `y_ref` with extents about `[0.5, 2.5, 0.5]`, then `getPolyHeight`.
- Move every caller that has a reference Y to it.
- Keep the old function only where no reference exists, renamed to say what it does (`height_nearest_world_origin`), or delete it.

**Acceptance:** an entity-crate test on `castle_cellblock.nav` at a point with two storeys in the same XZ column (the audit's (-289.5, 68.5, -154.3) region, or any XZ with walkable polys at about 0.2 and 68.6) proves that `y_ref = 68.5` returns about 68.6 and `y_ref = 0.5` returns about 0.2. The test fails against the old function.

### NA02

**Status:** Review (branch pushed, e270c296). **Scope title:** Stuck, float, path, LoS and cover detectors. **Advisor:** npc-ai-spawn-advisor, movement-teleport-advisor, aoi-witness-broadcast (for `wire.out`).

**Entries:** telemetry.md §2.1-2.5; `crates/entity/src/detour_ffi.rs:108-114`; `navigation/mod.rs` `find_path`; `path_failure/mod.rs`; `space_manager/spatial.rs` (LoS); `cover/ai_integration.rs`; `fight.rs:363`; `startup.rs:224-243`.

**Scope:**

- `NavMesh::find_path` returns a typed `PathOutcome` (`Ok`, `Partial`, `NoStartPoly`, `NoEndPoly`, `NoCorridor`, `StraightenFailed`) carrying the snap distances. Callers keep today's behaviour, and each outcome is logged.
- Every event in telemetry.md §2.1-2.5 that is not in NA00. Specifically:
  - aggro_scan rejects, using today's reasons only (`same_faction`, `dead`, `not_player`); NA13 adds the rest;
  - idle-unticked gauge, `idle_parked`, leash enter/snap/loop, `threat cleared_without_exit`;
  - `ground_deviation`, `stale_velocity`, `npc_off_mesh`, `stuck`, `spawn_off_mesh`;
  - `forced_position` and `npc_ai.los blocked` (`wire.out.movement_type` was dropped by NA10: no movement type goes on the wire);
  - `no_cover` reasons, `cover.selection`, `cover.coverage` per space.
- **No behaviour change.** The detectors must fire on today's bugs; they are the before-picture.

**Acceptance:**

- LogCapture tests prove that each WARN fires on a reproduced bug shape: stale velocity after `attack_in_place`, a leash loop, a partial path across two mesh islands, and cover coverage 0 on Cellblock with today's seed.
- Tests prove that the throttle suppresses and counts.
- UAT: owner plays Cellblock through the first two rooms. The NA02 queries show the pre-fix picture (stale_velocity > 0, leash loops, `cover.coverage` WARN, `no_cover reason=no_candidate_in_radius`).

### NA03

**Status:** Review (branch `npcai/na03-signoz-dashboard` pushed 2026-09-25). The dashboard **Cimmeria — NPC AI health** (id `01a0d755-9145-7eed-aedf-29d0efa28e1e`) and nine `NPC AI —` log views exist in the colo SigNoz under the owner's campaign authorization; exports in [operations/signoz/](../../operations/signoz/npc-ai-views.md); runbook [operations/npc-ai-telemetry-runbook.md](../../operations/npc-ai-telemetry-runbook.md). The acceptance render waits on a colo deploy with NA00/NA02: today only `npc_ai_decisions_total`, `npc_path_fail_total` and `npc_respawns_total` have data. **Scope title:** SigNoz saved views, the NPC AI health dashboard and the runbook. **Advisor:** documentation-writer.

**Scope:**

- The views and dashboard in [telemetry.md §3](telemetry.md#3-live-session-runbook), created through the SigNoz MCP. Exported JSON is committed under `docs/operations/signoz/` so it can be re-imported.
- Promote telemetry.md §3 into `docs/operations/` as the operator runbook, and link it from `docs/architecture/observability.md`.

**Acceptance:** the dashboard renders the NA02 UAT session. Confirm with the owner before creating shared dashboards (they are outward-facing).

## Phase 1: behaviour fixes (each lands behind NA02's detectors, then UAT)

### NA10

**Status:** Review (branch `npcai/na10-stop-hygiene` pushed 2026-09-25). Evidence decision: zero velocity only. The client has no movement-type receiver, and the old `setMovementType` broadcast reached witnesses as a truncated `onSequence`, so NA10 removed it ([findings §11](../../reverse-engineering/findings/npc-movement-pathfinding.md#11-correction-2026-09-25-the-client-has-no-movement-type-receiver)). **Scope title:** Movement stop hygiene: stale velocity and movement type (S1, S2, S13, part of S4). **Advisor:** aoi-witness-broadcast, movement-teleport-advisor.

**Scope:**

- One `stop_npc_movement(npc, reason)` that clears `nav_path`, zeroes `velocity` and sets the correct stationary movement type. Replace every mid-leg `nav_path.clear()`: `fight.rs:332, 521, 551`, `aggro.rs:127`, patrol, wander, follow, investigate and the content executor.
- Every position write goes through `write_position`, so the spatial grid stays in sync (the leash included).
- Evidence step first: which `EMobMovementType` or stance the client expects for "stationary in combat". Candidates are Cover (0) only at a node, or no movement-type change plus zero velocity. Check `setMovementType` handling in the client before choosing.

**Acceptance:**

- A tick test: after `attack_in_place` the next AoI `EntityMoved` for the NPC carries zero velocity. Revert-proved.
- The NA02 `stale_velocity` counter reads 0 in UAT.
- UAT: guards stop to shoot without running in place.

### NA11

**Status:** Review (branch `npcai/na11-ground-clamp`, on main after NA10 #779). Per-step and per-arrival ground clamp on the storey nearest the lerp, grounded `vy`, horizontal step budget; backup via `moveAlongSurface`; patrol, investigate and wander endpoints snapped; spawn Y grounded inside the `is_point_valid` band. `DT_STRAIGHTPATH_ALL_CROSSINGS` measured and not adopted: over 60 random Cellblock routes the clamp alone leaves 0 of 34,412 ticks more than 0.3 u off the floor (old lerp: 234, worst 4.7 u under), and crossings only add arrival snaps (3.5% slower). **Scope title:** Server-side ground clamp and safe fallbacks (M1-M3, M5). **Advisor:** movement-teleport-advisor.

**Scope:**

- In `npc_movement_tick`, `new_y = height_near(new_x, lerp_y, new_z).unwrap_or(lerp_y)` on every step and every waypoint arrival.
- Broadcast `vy` derived from the grounded delta, not from the chord.
- Worlds without a mesh keep the lerp and log once.
- Evaluate `DT_STRAIGHTPATH_ALL_CROSSINGS` and adopt it only if the per-tick clamp alone leaves visible corner error.
- Fix the raw endpoints:
  - backup uses a horizontal-only direction and a navmesh raycast or `moveAlongSurface`;
  - investigate, wander and patrol endpoints are snapped to the mesh;
  - unrouted follow is unchanged (it already keeps its own Y).
- Snap spawn Y to the mesh at spawn when within the `is_point_valid` band, and log `spawn_off_mesh` when not.

**Acceptance:**

- A tick test on `castle_cellblock.nav`: an NPC chasing from floor to ramp never deviates more than 0.3 u from `height_near` along the path. It fails on the old lerp.
- A backup-waypoint test with the player 5 u above ends on the mesh.
- UAT: stand at the top of a ramp and let a guard come to you; no climbing through the air. `ground_deviation` about 0.

### NA12

**Status:** Review (branch `npcai/na12-leash-reset` pushed 2026-09-25, feature commit d68b04d3, rebased on main after NA02/#781; wired into the NA02 leash, threat and idle_parked detectors). NPC-to-spawn horizontal leash with a 5 u band and 20 u vertical cap, `entity_templates.leash_distance` (nullable, default 50), lost target (dead / gone / out of AoI 5 s), walk home with evade, reset on arrival, snap fallback (no route or 20 s), player combat drain, 5 s re-aggro window. **Scope title:** Leash, evade and reset rework (S3-S7, S12). **Advisor:** npc-ai-spawn-advisor, combat-systems-advisor.

**Scope:**

- **Leash metric:** NPC-to-spawn beyond `leash_distance` (default 50; a per-template column may follow), **or** the target lost for a grace period. Horizontal distance with a vertical cap, and hysteresis so an NPC at the boundary does not flicker.
- **Walk home:** on Leashing, `find_path(npc, spawn)` and walk it. No movement-type wire exists: the client shows the walk from position and velocity alone (NA10). `MobMovementType::Leash` stays a server-side cache value. Snap and stop through `npc_ai::snap_npc_to` / `stop_movement_on` (NA10) so the grid and velocity stay right.
- **Evade:** ignore threat and damage while returning, and log `damage_ignored`.
- **Arrival:** heal to full, restore `spawn_dir`, clear cooldowns, go Idle.
- **Snap fallback** only when no path exists, as a forced position with facing.
- **Lost target:** the target dies, disconnects or threat drains → Leashing (walk home), not Idle in place.
- **Player combat drain:** on every threat clear, call `exit_player_combat` / `clear_dead_npc_from_all_player_threat` for the players who listed this NPC.
- **Post-reset suppression:** no proximity re-aggro for about 5 s after arriving, to break the loop.
- Cover is released on leash (already done).

**Acceptance:**

- Tick tests: the leash triggers on NPC distance and not on player distance (the tutorial staging spot at 49.9 from spawn must not leash an NPC standing at spawn); the NPC walks home and does not teleport; no stale path after arrival.
- A 60 s tick test proves the loop is gone.
- A player's `threatened_mobs` is empty after a leash, and regen resumes.
- UAT: `leash loop`, `idle_parked` and `cleared_without_exit` all read 0; guards walk home when you die or run.

### NA13

**Status:** Review (branch `npcai/na13-faction-aggro` pushed 2026-09-25, on NA12). Effective aggression = override (`spawnlist.aggression_override`, `set_aggression`, console) else the 2009 `FACTION_REACTION_TABLE` (code constant pinned to `enumerations.xml`; players react as faction 3); only HOSTILE aggroes. Runtime field is now `aggro.override_level: Option<MobAggression>`; the two `set_aggression` rows keep `level 1` (= HOSTILE). Gates: `entity_templates.aggro_radius` (default 18 u), 4 u vertical band, LoS with `Unknown` failing closed where a navmesh exists, dead, GM `.aggro off`. Spawns 20 and 10 seeded NEUTRAL; no other Castle/Harset chain calls `set_aggression`. Wire broadcast NOT shipped: the client consumes the level via `onAggressionOverrideUpdate` (SGWMob, index unverified), not `onEntityProperty` type 6 (open item in [npc-ai.md](../../gameplay/npc-ai.md#wire-not-broadcast-yet-open-item)). **Scope title:** Faction-derived proximity aggro with radius, LoS, vertical band and GM toggle (A1-A6, A8). **Advisor:** npc-ai-spawn-advisor, combat-systems-advisor, server-authority-enforcer (GM toggle), database-persistence (seed columns).

**Scope:**

- **Effective aggression** = runtime override (content `set_aggression`, a spawn seed override) or else the faction reaction of player faction vs NPC faction, as `EMobAggressionLevel`. Only `HOSTILE (1)` aggroes on sight. Port the reaction table from `SGWPlayer.py:999-1009` as data (a seed table or a constant table in code with its source cited).
- **Rename** the Rust runtime `aggression` field so it matches the enum direction (A6), and update every `set_aggression` chain row in the same PR.
- **Admission:** Idle NPCs with effective aggression HOSTILE are ticked.
- **Scan gates:**
  - aggro radius: an `entity_templates.aggro_radius` column, default 18 u (tuned at UAT);
  - a vertical band, `abs(dy) <= 4 u`;
  - LoS required, and `Unknown` does **not** count as clear for aggro, only for attacks;
  - dead players skipped;
  - GMs skipped only while the new server-side `.aggro off` toggle is set (D-NA02). Client `ghost` is invisible to the server.
- **Chain-armed spawns stay passive until their chain fires:** a seeded override on spawnlist rows 20 (`ArmYourself_NIDGuard`) and 10 (`ArmYourself_PrisonerRetrievalUnit`) keeps chains 1008 and 1032 in charge of them. Audit Castle (world 8) and Harset for other `set_aggression` chains the same way.
- **Wire:** broadcast `GENERICPROPERTY_MobAggression` on a change, as Python did. First confirm the property and method ids against `docs/protocol/client-method-dispatch-table.md`.
- Doc updates: `docs/gameplay/` combat/NPC page and `docs/game-systems.md`.

**Acceptance:**

- Unit tests for the effective-aggression derivation.
- Tick tests: a NID Guard aggroes a player at 15 u in LoS; it does not aggro at 25 u, through a wall, or on another storey; it does not aggro a GM with the toggle set.
- Chain 1008 still owns the first guard (chain-replay test). The PRU does not fire before the vial interaction.
- Live-DB test for the new seed columns.
- UAT: walk the Cellblock topside as a GM with the toggle off. Each room's guards engage when you enter the room, not before. `npc_ai.aggro cause=proximity` rows exist for the Hallway, MessHall and Barracks tags.

### NA14

**Status:** BlockedDependency (NA13). **Scope title:** Same-room assist aggro (A7, D-NA04). **Advisor:** npc-ai-spawn-advisor.

**Scope:** when an NPC enters Fighting from damage or proximity, same-faction Idle NPCs within `assist_radius` (a template column, default 10 u) with LoS to the victim and the same vertical band take a threat seed on the same target, with `cause=assist`. No chaining: an assisting NPC does not recruit further. Mark the deviation from legacy in code and in the docs.

**Acceptance:** tick tests for MessHall_Guard1/2 (7 u apart: both engage) and for the Hallway guards (more than 10 u apart: only the one hit engages); a no-chain test. UAT: shoot one MessHall guard and both engage.

### NA15

**Status:** BlockedDependency (NA02, NA11). **Scope title:** Path robustness (S8-S10). **Advisor:** movement-teleport-advisor.

**Scope:**

- **Partial path:** walk to the reachable end. If the target is still out of attack range there and not in LoS, hold as stationary; after a grace period, leash.
- **Start off-mesh:** snap the NPC to the nearest poly within a bounded radius (forced position) before pathing, and log `npc_off_mesh`.
- **`repath_degenerate`:** clear the stale path.
- **Minimum stop distance:** the NPC stops at `max(ability min range, combined radii)`, never inside the target.
- The repath threshold uses horizontal distance plus a vertical term, so a player walking down a ramp triggers a repath.

**Acceptance:** tick tests over the two-island fixture and an off-mesh start; a stop-distance unit test. UAT: `stuck` and `npc_off_mesh` read about 0.

### NA16

**Status:** BlockedEvidence. **Scope title:** Line of sight source and the stationary PRU (S11, S15). **Advisor:** combat-systems-advisor, movement-teleport-advisor.

**Scope:**

- Read NA02's `npc_ai.los blocked` rows for the PRU at (-220.3, 66.7, -121.4).
- Determine the eye heights used and what the ray hits (a mesh edge? the unit's own model origin below the floor?).
- Fix the eye height per being type, or the unit's spawn Y.
- Decide the occlusion source for aggro and fire-time line of sight: the navmesh raycast cannot see floors, ceilings or props (S15). The options are raycasting the extracted collision geometry (the navmesh-extractor already has the triangles), or keeping the navmesh ray plus the vertical band. Whether to add line of sight at ability fire time is combat-systems-advisor's call.

**Acceptance:** a regression test at the recorded geometry. UAT: the drone fires at 12-16 m.

## Phase 2: cover (D-NA05, faithful)

### NA20

**Status:** Done (evidence only). Finding: [`docs/reverse-engineering/findings/cover-world-placement.md`](../../reverse-engineering/findings/cover-world-placement.md). Go/no-go for NA21: **go, smaller scope than estimated** — Castle/Castle_CellBlock's real cover nodes are `ASGWSpecCoverNode` actors and `StaticMeshActor.CoverNodeArray` groups baked directly into the `.umap` chunks (4,024 nodes total), already in absolute world space with no owner-transform composition needed for either pattern found; no new binary-format decoder is required, only a property walk the existing `crates/upk::extract_actors`/`ACTOR_CLASSES` machinery already supports. Q4 (client pose trigger) remains unresolved — UnrealScript bytecode, not natively recoverable — and does not block NA21. **Scope title:** Where the world-space cover data lives (C1, C2, C6). **Advisor:** game-archaeology-specialist.

**Scope:**

1. With `crates/upk-objects`, decode one `SGWCoverNodeComponent` in `CA-Prebuilt.upk`. Does it carry the `CoverNodePrefabData` array itself, or a reference to the pak template by prefab name?
2. Find which Cellblock and Castle map actors (prefab instances / archetypes) own such components. Count them, and compare against the `SGWCoverNodeComponent`, `SGWSpecCoverNode` and `CoverNode` names in the Cellblock chunks.
3. Hand-transform one instance to world space. Check it against the geometry, and against `Castle_CellBlock_MedStationDesk` (set 1381, hand-authored in C05) if that desk is one of them.
4. **Live pose experiment first (cheap):** spawn a test NPC with `use_cover` at the one real world-space node, set 1381 `Castle_CellBlock_MedStationDesk` at about (-234, 66.5, -124.7), with a threat nearby, and watch whether the client crouches. If it does, the pose comes from position plus the node's height and needs no client work. If it does not, the pose needs a claim on the pawn that the server cannot set, which is client-patch territory and needs an owner decision. Also evaluate granting ability 1451 "Cover Stance" on entering cover and revoking it on leaving.
5. In Ghidra, finish `USGWAnim_BlendByCover` and the `CombatStance`/`bCoverFromTarget` flow. What wire state makes an NPC crouch, peek and fire? A stance property? Ability animation? (Movement type 0 is ruled out: no server-to-client movement-type message exists, NA10.)

**Output:** a finding under `docs/reverse-engineering/findings/`, and a go/no-go plus data model for NA21.

### NA21

**Status:** Review (branch `npcai/na21-cover-extractor`). `cover_extract` (`crates/navmesh-extractor`) emits world-space cover from the `.umap` chunks: 236 nodes / 58 sets for Castle_CellBlock (world 12), 3,788 / 481 for Castle (world 8). `cover_sets` gained `world_id` (FK to `worlds`) and `cover_nodes` gained `width`. The 9,346 prefab-local `.pak` rows were dropped. Set 1381 was retired, because the extractor reproduces the desk as set 1200001, and chains 1132/1133 were rekeyed. The cell's cover index is partitioned per world. See [cover-extraction.md](../../engine/cover-extraction.md). **Scope title:** Per-map cover extractor and space-scoped seeds (C1, C2). **Advisor:** game-archaeology-specialist, database-persistence.

**Scope:**

- Extend the navmesh-extractor walker, which already resolves archetypes and mirrored transforms, to emit world-space cover nodes per map: position, orientation, height/width/quality, and the slot grouping per owner.
- Add a world/space scope to `cover_sets` (a seed schema change in `db/resources/`; ask before any `db/scripts` migration) and regenerate the seeds for Castle_CellBlock and Castle first.
- Keep set 1381 or retire it if the extractor reproduces it.
- Retire the global prefab-local rows, or keep them only as templates.

**Acceptance:** extractor tests on synthetic fixtures (not asset-only; see the PR #683 lesson). The NA02 `cover.coverage` WARN clears for worlds 12 and 8. At least N nodes sit on the navmesh in each Cellblock guard room.

### NA22

**Status:** BlockedDependency (NA21, NA10, NA12; NA20 for pose). **Scope title:** Cover behaviour: hold, seek, pose (C3-C5, C7). **Advisor:** npc-ai-spawn-advisor, combat-systems-advisor.

**Scope:**

- A per-space cover index.
- `use_cover` from the template (`useCover`); melee and stationary units are off.
- **Spawn in cover:** reserve the nearest slot within about 1.5 u at spawn, and hold it through combat until flanked, then re-pick.
- **Seek cover in combat:** prefer the best slot within attack range of the target, not only when out of range. Keep the existing scorer and flank hysteresis.
- **Pose:** drive whatever NA20 finds (a stance, a state flag or an ability animation), with peek-and-shoot cadence if the client needs a server cue. It cannot come from movement type 0: NA10 found that no movement type reaches the client (`0x00deb660` is the GM `onShowPath` visualiser).
- Release on leash, death and target loss.
- Write `docs/architecture/cover-system.md`, which `cover/mod.rs` already points to.

**Acceptance:** tick tests (a spawned-in-cover NPC keeps its slot while the target is in range; an NPC under fire moves to a slot; flanking releases the slot); the cover decisions appear in telemetry. UAT: Cellblock guards duck behind room cover and fire from it.

## Phase 3: documentation corrections

### NA30

**Status:** Review (branch pushed, 68732034). **Scope title:** Fix the docs this audit disproved. **Advisor:** documentation-writer.

**Scope:** every row in [audit §6](audit.md#6-documentation-that-is-wrong) that no earlier packet fixed. Annotate the 2026-09-18 playtest appendix rows rather than rewriting them.

**Acceptance:** markdownlint clean; the index entries stay in sync.

## Suggested order

NA00, NA01 and NA20 in parallel. Then:

1. NA02.
2. **UAT-0**, the before-picture session.
3. NA10.
4. NA11 and NA12 in parallel, since they share only `fight.rs`, and the coordinator serialises that file.
5. **UAT-1**: running in place, floating, stuck.
6. NA13, then NA14.
7. **UAT-2**: aggro.
8. NA15 and NA16.
9. NA21, then NA22.
10. **UAT-3**: cover.
11. NA30, which can run at any time.
