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

**Status:** UATPending (merged 2026-09-25, PR #776 32d2faa5; earlier: branch pushed, d5b9d5f8). **Scope title:** Telemetry plumbing, the AI state-transition helper, and deploy identity. **Depends:** land or rebase PR #726 first (it rewrites the path-fail messages and the `destroy_space` throttle cleanup). **Advisor:** npc-ai-spawn-advisor, testing-validation-engineer.

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

**Status:** UATPending (merged 2026-09-25, PR #774 6e6ec5c3; earlier: branch pushed, 8bbfb905). **Scope title:** Storey-aware navmesh height query (M4). **Depends:** none (it touches the `entity` crate only). **Advisor:** movement-teleport-advisor.

**Entries:** `crates/entity/src/navigation/mod.rs:394-434` (`get_height_at`); its callers `npc_movement.rs:211`, `spawner/npcs.rs`, `console/bookmark.rs:171`, and `grep get_navmesh_height`.

**Scope:**

- Add `get_height_near(x, y_ref, z) -> Option<f32>`: `findNearestPoly` centred at `y_ref` with extents about `[0.5, 2.5, 0.5]`, then `getPolyHeight`.
- Move every caller that has a reference Y to it.
- Keep the old function only where no reference exists, renamed to say what it does (`height_nearest_world_origin`), or delete it.

**Acceptance:** an entity-crate test on `castle_cellblock.nav` at a point with two storeys in the same XZ column (the audit's (-289.5, 68.5, -154.3) region, or any XZ with walkable polys at about 0.2 and 68.6) proves that `y_ref = 68.5` returns about 68.6 and `y_ref = 0.5` returns about 0.2. The test fails against the old function.

### NA02

**Status:** UATPending (merged 2026-09-25, PR #781 1e283f61; earlier: branch pushed, e270c296). **Scope title:** Stuck, float, path, LoS and cover detectors. **Advisor:** npc-ai-spawn-advisor, movement-teleport-advisor, aoi-witness-broadcast (for `wire.out`).

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

**Status:** UATPending (merged 2026-09-25, PR #782 bcfa23e3; earlier: branch `npcai/na03-signoz-dashboard` pushed 2026-09-25). The dashboard **Cimmeria — NPC AI health** (id `01a0d755-9145-7eed-aedf-29d0efa28e1e`) and nine `NPC AI —` log views exist in the colo SigNoz under the owner's campaign authorization; exports in [operations/signoz/](../../operations/signoz/npc-ai-views.md); runbook [operations/npc-ai-telemetry-runbook.md](../../operations/npc-ai-telemetry-runbook.md). The acceptance render waits on a colo deploy with NA00/NA02: today only `npc_ai_decisions_total`, `npc_path_fail_total` and `npc_respawns_total` have data. **Scope title:** SigNoz saved views, the NPC AI health dashboard and the runbook. **Advisor:** documentation-writer.

**Scope:**

- The views and dashboard in [telemetry.md §3](telemetry.md#3-live-session-runbook), created through the SigNoz MCP. Exported JSON is committed under `docs/operations/signoz/` so it can be re-imported.
- Promote telemetry.md §3 into `docs/operations/` as the operator runbook, and link it from `docs/architecture/observability.md`.

**Acceptance:** the dashboard renders the NA02 UAT session. Confirm with the owner before creating shared dashboards (they are outward-facing).

## Phase 1: behaviour fixes (each lands behind NA02's detectors, then UAT)

### NA10

**Status:** UATPending (merged 2026-09-25, PR #779 bde45a9c; earlier: branch `npcai/na10-stop-hygiene` pushed 2026-09-25). Evidence decision: zero velocity only. The client has no movement-type receiver, and the old `setMovementType` broadcast reached witnesses as a truncated `onSequence`, so NA10 removed it ([findings §11](../../reverse-engineering/findings/npc-movement-pathfinding.md#11-correction-2026-09-25-the-client-has-no-movement-type-receiver)). **Scope title:** Movement stop hygiene: stale velocity and movement type (S1, S2, S13, part of S4). **Advisor:** aoi-witness-broadcast, movement-teleport-advisor.

**Scope:**

- One `stop_npc_movement(npc, reason)` that clears `nav_path`, zeroes `velocity` and sets the correct stationary movement type. Replace every mid-leg `nav_path.clear()`: `fight.rs:332, 521, 551`, `aggro.rs:127`, patrol, wander, follow, investigate and the content executor.
- Every position write goes through `write_position`, so the spatial grid stays in sync (the leash included).
- Evidence step first: which `EMobMovementType` or stance the client expects for "stationary in combat". Candidates are Cover (0) only at a node, or no movement-type change plus zero velocity. Check `setMovementType` handling in the client before choosing.

**Acceptance:**

- A tick test: after `attack_in_place` the next AoI `EntityMoved` for the NPC carries zero velocity. Revert-proved.
- The NA02 `stale_velocity` counter reads 0 in UAT.
- UAT: guards stop to shoot without running in place.

### NA11

**Status:** UATPending (merged 2026-09-25, PR #783 02e163fa; earlier: branch `npcai/na11-ground-clamp`, on main after NA10 #779). Per-step and per-arrival ground clamp on the storey nearest the lerp, grounded `vy`, horizontal step budget; backup via `moveAlongSurface`; patrol, investigate and wander endpoints snapped; spawn Y grounded inside the `is_point_valid` band. `DT_STRAIGHTPATH_ALL_CROSSINGS` measured and not adopted: over 60 random Cellblock routes the clamp alone leaves 0 of 34,412 ticks more than 0.3 u off the floor (old lerp: 234, worst 4.7 u under), and crossings only add arrival snaps (3.5% slower). **Scope title:** Server-side ground clamp and safe fallbacks (M1-M3, M5). **Advisor:** movement-teleport-advisor.

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

**Status:** UATPending (merged 2026-09-25, PR #785 83dfefa2; earlier: branch `npcai/na12-leash-reset` pushed 2026-09-25, feature commit d68b04d3, rebased on main after NA02/#781; wired into the NA02 leash, threat and idle_parked detectors). NPC-to-spawn horizontal leash with a 5 u band and 20 u vertical cap, `entity_templates.leash_distance` (nullable, default 50), lost target (dead / gone / out of AoI 5 s), walk home with evade, reset on arrival, snap fallback (no route or 20 s), player combat drain, 5 s re-aggro window. **Scope title:** Leash, evade and reset rework (S3-S7, S12). **Advisor:** npc-ai-spawn-advisor, combat-systems-advisor.

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

**Status:** UATPending (merged 2026-09-25, PR #787 b66fd426; earlier: branch `npcai/na13-faction-aggro` pushed 2026-09-25, on NA12). Effective aggression = override (`spawnlist.aggression_override`, `set_aggression`, console) else the 2009 `FACTION_REACTION_TABLE` (code constant pinned to `enumerations.xml`; players react as faction 3); only HOSTILE aggroes. Runtime field is now `aggro.override_level: Option<MobAggression>`; the two `set_aggression` rows keep `level 1` (= HOSTILE). Gates: `entity_templates.aggro_radius` (default 18 u), 4 u vertical band, LoS with `Unknown` failing closed where a navmesh exists, dead, GM `.aggro off`. Spawns 20 and 10 seeded NEUTRAL; no other Castle/Harset chain calls `set_aggression`. Wire broadcast NOT shipped: the client consumes the level via `onAggressionOverrideUpdate` (SGWMob, index unverified), not `onEntityProperty` type 6 (open item in [npc-ai.md](../../gameplay/npc-ai.md#wire-not-broadcast-yet-open-item)). **Scope title:** Faction-derived proximity aggro with radius, LoS, vertical band and GM toggle (A1-A6, A8). **Advisor:** npc-ai-spawn-advisor, combat-systems-advisor, server-authority-enforcer (GM toggle), database-persistence (seed columns).

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

**Status:** UATPending (merged 2026-09-25, PR #789 26bd65e3; earlier: branch `npcai/na14-assist-aggro` pushed 2026-09-25, on main after NA13/#787). `combat::generate_threat` calls `npc_ai::recruit_assisters` when an NPC enters Fighting from damage or proximity; same-faction NPCs that are HOSTILE themselves, Idle/Patrol/Wander, within their `entity_templates.assist_radius` (new nullable column, CHECK > 0, default 10 u) of the victim, within the 4 u band and in navmesh LoS (Unknown fails closed) take a 1.0 seed with `cause=assist` (transition `reason=assist`). Assist and content threat do not recruit (no chaining); a GM with `.aggro off` pulls no assisters. Telemetry: `npc_ai.aggro event=acquired cause=assist`, `npc_ai.aggro_scan event=assist_joined` / `assist_rejected` (new reason `not_idle`). Tests: meshless tick tests (join, no chain, NEUTRAL, busy states incl. Leashing, patrol/wander, faction/band/template radius, content threat, proximity, GM) and `castle_cellblock.nav` tests (MessHall pair assists, Hallway guards 18.6 u apart do not), live-DB guards for the column; revert-checked. UAT: shoot one MessHall guard, both engage; SigNoz `npc_ai.aggro` grouped by `cause` shows `assist`. **Scope title:** Same-room assist aggro (A7, D-NA04). **Advisor:** npc-ai-spawn-advisor.

**Scope:** when an NPC enters Fighting from damage or proximity, same-faction Idle NPCs within `assist_radius` (a template column, default 10 u) with LoS to the victim and the same vertical band take a threat seed on the same target, with `cause=assist`. No chaining: an assisting NPC does not recruit further. Mark the deviation from legacy in code and in the docs.

**Acceptance:** tick tests for MessHall_Guard1/2 (7 u apart: both engage) and for the Hallway guards (more than 10 u apart: only the one hit engages); a no-chain test. UAT: shoot one MessHall guard and both engage.

### NA15

**Status:** UATPending (merged 2026-09-25, PR #788 0da904cd; earlier: branch `npcai/na15-path-robustness` pushed 2026-09-25, on main after NA12 #785). The chase moved out of `fight.rs` into `npc_ai/chase/`. A partial route is walked to its end, then the NPC holds with zero velocity and no new route requests, and walks home after 8 s (`reason=unreachable`). A partial route home walks to its end and snaps (`arrival=snap_partial_route`) instead of waiting out the 20 s timeout. An off-mesh start is snapped onto the nearest polygon within 2 u / ±4 u and retried once, or the NPC goes home. A degenerate repath clears the stale route. Chases stop `max(min_range, 1.0)` short of the target. The repath test is 5 u horizontal or 1.5 u vertical. An off-mesh target (S14) is routed to the nearest on-mesh point within 8 u / ±4 u. Tick tests: `service/tests/npc_ai/path_robustness.rs`, each revert-proven. **Scope title:** Path robustness (S8-S10, S14). **Advisor:** movement-teleport-advisor.

**Scope:**

- **Partial path:** walk to the reachable end. If the target is still out of attack range there and not in LoS, hold as stationary; after a grace period, leash.
- **Start off-mesh:** snap the NPC to the nearest poly within a bounded radius (forced position) before pathing, and log `npc_off_mesh`.
- **`repath_degenerate`:** clear the stale path.
- **Minimum stop distance:** the NPC stops at `max(ability min range, combined radii)`, never inside the target.
- The repath threshold uses horizontal distance plus a vertical term, so a player walking down a ramp triggers a repath.

**Acceptance:** tick tests over the two-island fixture and an off-mesh start; a stop-distance unit test. UAT: `stuck` and `npc_off_mesh` read about 0.

### NA16

**Status:** UATPending (merged 2026-09-25, PR #786 4233273f; earlier: branch `npcai/na16-line-of-sight`). The S11 cause is the med-station desk, not eye height. The desk is a navmesh hole between the drone and the vial, and the ray reads `Blocked` over a 1 m desk. A stationary NPC's attack line of sight now ignores a same-storey (4 u band) navmesh `Blocked`. Mobile NPCs and aggro keep the strict verdict. S15: 45% of same-storey `Blocked` verdicts are false against the collision geometry, and the navmesh-only heuristics were rejected on numbers. Approved as D-NA11. The collision-geometry occluder is follow-up #784 (new data artifact). No fire-time line-of-sight check: the fight tick covers NPCs, and a navmesh check would give players false "no line of sight" errors. See audit S11/S15 and [npc-ai.md](../../gameplay/npc-ai.md#rust-line-of-sight-in-the-fight-tick). **Scope title:** Line of sight source and the stationary PRU (S11, S15). **Advisor:** combat-systems-advisor, movement-teleport-advisor.

**Scope:**

- Read NA02's `npc_ai.los blocked` rows for the PRU at (-220.3, 66.7, -121.4).
- Determine the eye heights used and what the ray hits (a mesh edge? the unit's own model origin below the floor?).
- Fix the eye height per being type, or the unit's spawn Y.
- Decide the occlusion source for aggro and fire-time line of sight: the navmesh raycast cannot see floors, ceilings or props (S15). The options are raycasting the extracted collision geometry (the navmesh-extractor already has the triangles), or keeping the navmesh ray plus the vertical band. Whether to add line of sight at ability fire time is combat-systems-advisor's call.

**Acceptance:** a regression test at the recorded geometry. UAT: the drone fires at 12-16 m.

## Phase 2: cover (D-NA05, faithful)

### NA20

**Status:** Done (evidence only; merged PR #777 731eadc7). Finding: [`docs/reverse-engineering/findings/cover-world-placement.md`](../../reverse-engineering/findings/cover-world-placement.md). Go/no-go for NA21: **go, smaller scope than estimated** — Castle/Castle_CellBlock's real cover nodes are `ASGWSpecCoverNode` actors and `StaticMeshActor.CoverNodeArray` groups baked directly into the `.umap` chunks (4,024 nodes total), already in absolute world space with no owner-transform composition needed for either pattern found; no new binary-format decoder is required, only a property walk the existing `crates/upk::extract_actors`/`ACTOR_CLASSES` machinery already supports. Q4 (client pose trigger) remains unresolved — UnrealScript bytecode, not natively recoverable — and does not block NA21. **Scope title:** Where the world-space cover data lives (C1, C2, C6). **Advisor:** game-archaeology-specialist.

**Scope:**

1. With `crates/upk-objects`, decode one `SGWCoverNodeComponent` in `CA-Prebuilt.upk`. Does it carry the `CoverNodePrefabData` array itself, or a reference to the pak template by prefab name?
2. Find which Cellblock and Castle map actors (prefab instances / archetypes) own such components. Count them, and compare against the `SGWCoverNodeComponent`, `SGWSpecCoverNode` and `CoverNode` names in the Cellblock chunks.
3. Hand-transform one instance to world space. Check it against the geometry, and against `Castle_CellBlock_MedStationDesk` (set 1381, hand-authored in C05) if that desk is one of them.
4. **Live pose experiment first (cheap):** spawn a test NPC with `use_cover` at the one real world-space node, set 1381 `Castle_CellBlock_MedStationDesk` at about (-234, 66.5, -124.7), with a threat nearby, and watch whether the client crouches. If it does, the pose comes from position plus the node's height and needs no client work. If it does not, the pose needs a claim on the pawn that the server cannot set, which is client-patch territory and needs an owner decision. Also evaluate granting ability 1451 "Cover Stance" on entering cover and revoking it on leaving.
5. In Ghidra, finish `USGWAnim_BlendByCover` and the `CombatStance`/`bCoverFromTarget` flow. What wire state makes an NPC crouch, peek and fire? A stance property? Ability animation? (Movement type 0 is ruled out: no server-to-client movement-type message exists, NA10.)

**Output:** a finding under `docs/reverse-engineering/findings/`, and a go/no-go plus data model for NA21.

### NA21

**Status:** UATPending (merged 2026-09-25, PR #780 e9653a73; earlier: branch `npcai/na21-cover-extractor`). `cover_extract` (`crates/navmesh-extractor`) emits world-space cover from the `.umap` chunks: 236 nodes / 58 sets for Castle_CellBlock (world 12), 3,788 / 481 for Castle (world 8). `cover_sets` gained `world_id` (FK to `worlds`) and `cover_nodes` gained `width`. The 9,346 prefab-local `.pak` rows were dropped. Set 1381 was retired, because the extractor reproduces the desk as set 1200001, and chains 1132/1133 were rekeyed. The cell's cover index is partitioned per world. See [cover-extraction.md](../../engine/cover-extraction.md). **Scope title:** Per-map cover extractor and space-scoped seeds (C1, C2). **Advisor:** game-archaeology-specialist, database-persistence.

**Scope:**

- Extend the navmesh-extractor walker, which already resolves archetypes and mirrored transforms, to emit world-space cover nodes per map: position, orientation, height/width/quality, and the slot grouping per owner.
- Add a world/space scope to `cover_sets` (a seed schema change in `db/resources/`; ask before any `db/scripts` migration) and regenerate the seeds for Castle_CellBlock and Castle first.
- Keep set 1381 or retire it if the extractor reproduces it.
- Retire the global prefab-local rows, or keep them only as templates.

**Acceptance:** extractor tests on synthetic fixtures (not asset-only; see the PR #683 lesson). The NA02 `cover.coverage` WARN clears for worlds 12 and 8. At least N nodes sit on the navmesh in each Cellblock guard room.

### NA22

**Status:** UATPending (the last packet; merged in the PR that carries this close-out; earlier: branch `npcai/na22-cover-behaviour` pushed 2026-09-25). Cover is a firing position: the fight tick seeks the best free slot within attack range less 2 u in range too (10 u walk, 4 s seek retry), holds it until flanked or out of range, stops at it with zero velocity and fires without chasing. Spawn hold within 1.5 u (spawn, startup sweep, respawn, leash home). Cover Stance (ability 1451) through new `CoverStance` / `RemoveCoverStance` scripts on effects 4565 / 1742, removed on leave, leash, death and surrender. `entity_templates.use_cover` (NULL = hostile faction 10; stationary, props and melee-only never). Squad affinity by distance (2 u), not by set. No pose wire (D-NA10); the Q4 owner experiment decides the crouch. See [architecture/cover-system.md](../../architecture/cover-system.md). **Scope title:** Cover behaviour: hold, seek, pose (C3-C5, C7). **Advisor:** npc-ai-spawn-advisor, combat-systems-advisor.

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

**Status:** Done (docs only; merged 2026-09-25, PR #775 507cc572; earlier: branch pushed, 68732034). **Scope title:** Fix the docs this audit disproved. **Advisor:** documentation-writer.

**Scope:** every row in [audit §6](audit.md#6-documentation-that-is-wrong) that no earlier packet fixed. Annotate the 2026-09-18 playtest appendix rows rather than rewriting them.

**Acceptance:** markdownlint clean; the index entries stay in sync.

## Phase 4: UAT-1 follow-ups

UAT-1 ran on the colo on 2026-09-25 against build `059d6038`. Every finding, with its evidence, is in [worknotes/uat-1.md](worknotes/uat-1.md). The owner approved the follow-up work the same day ("do the follow up work"). NA23 and NA24 ran in parallel with disjoint files; NA23 touched `npc_ai/dispatch.rs` only for the tick row's line-of-sight source.

### NA23

**Status:** Review (branch `npcai/na23-cover-los` pushed; code commit 3cb044fd, rebased on NA24 #791). **Scope title:** Cover-aware line of sight and flank churn (UAT-1 findings 1-3, D-NA12). **Advisor:** npc-ai-spawn-advisor.

**Scope:**

- **Peek origin.** `cover::find_peek` gives a cover slot a peek point on the navmesh: over the prop along the node's facing (0.5-3.5 u, with 1 u of clear floor ahead), else round either end of the marker (a walk of at most 6 u). `SpaceManager::npc_line_of_sight` uses it for an NPC standing within 1.5 u of the slot it holds. The NPC sees a target when the peek ray or its own ray is clear.
- **One rule for aggro and attack.** The Idle aggro scan, the assist check, the attack check and the `npc_ai.tick` row share that rule. `AttackLosPolicy::InCoverSlot` (no check at all) becomes `CoverPeek` (strict from the peek point, `los_policy=cover_peek`).
- **Holding fire.** An NPC with no shot from its slot holds fire (`cover_no_shot`) and gives the slot up after 3 s (`cover_released_no_shot`). A new pick needs a shot from the slot.
- **Flank churn.** A held slot is released 20 degrees past side-on (NA22 used 5), and a slot is picked only with the threat in front of side-on. A slot given up as flanked, blind or unreachable cannot be picked again by the same NPC for 6 s. A flanked NPC with a line from where it stands fires in place.

**Acceptance:** tests on `castle_cellblock.nav` and the world-12 cover seed at the UAT-1 positions, each revert-proven:

- `Hallway01_Guard` aggroes the player 8.1 u in front of its counter;
- `Hallway02_Guard` holds fire at the spot where it killed the player through the walls;
- the mess-hall strafe that flanked `MessHall_Guard2` keeps its slot;
- a flanked guard fires in place and does not re-pick its slot;
- no slot is picked that has no shot.

The drone's `stationary_relaxed` shot is still pinned by NA16's `stationary_los.rs`. UAT: Hallway01 engages on sight from its counter; no guard fires through a wall (`los_policy=cover_peek` rows read `los=clear` whenever the NPC fires).

### NA24

**Status:** UATPending (merged 2026-09-25, PR #791 73486cb2; ran as a parallel worker to NA23). **Scope title:** UAT-1 findings 4-8. **Advisor:** npc-ai-spawn-advisor, items-systems-advisor.

**Scope:**

- **Dead player (finding 4).** The dead gate on item use: the cell `inventory` methods and the base `world_entry` inventory use. `npc_ai/fight_target.rs` `select_target` treats a `BSF_DEAD` target as dead. `abilities/death` purges a dying player from every threat list.
- **Bookmarks (finding 5).** Real witness fields in `console/bookmark.rs`.
- **Being-class followers (finding 6).** NPC admission in `space_manager/queries.rs` ticks Col Marsh's Follow.
- **Off-mesh spawn (finding 7).** The spawnlist seed for `Castle_BravoOfficer3` moves it onto the floor.
- **Tick volume (finding 8).** `npc_ai/dispatch.rs` samples the tick row for unwitnessed Idle NPCs.

### NA26

**Status:** Review (branch `npcai/na26-navmesh-all-worlds` pushed, no PR; ran as a parallel worker to NA27). **Scope title:** Rebuild the navmesh for every world. **Advisor:** movement-teleport-advisor. **Owner request (2026-09-25):** "make sure we update all the worlds navmeshes too", server-authoritative, no client patch.

**Scope:**

- **Build.** Every `entities/spaces.xml` world has a `data/spaces/*.nav` from the cooked client maps: 18 new files (17 maps plus `sandbox.nav`, a copy of `harset_cmdcenter.nav` because SandBox plays on that map) and four replaced 2012 meshes (`harset`, `harset_storagerm`, `sgc_w1`, `agnos`). `castle.nav` and `castle_cellblock.nav` were rebuilt, measured and kept: the rebuild changed nothing measurable. Parameters, sizes, components and validation per world are in `data/spaces/README.md`; the method is `docs/engine/navmesh-build-pipeline.md` §9.
- **Caps.** A single-tile XRC mesh cannot hold the big exteriors at `cs=0.3`. Dakara_E1 and both Menfa maps ship whole at `cs=0.6`; Agnos, Lucia, Tollana and Beta_Site_Evo_1 ship cropped. A tiled Detour mesh is the follow-up.
- **NavBuilder.** A fifth unchecked Recast limit (13-bit span heights) flattened Tollana onto one sheet at y -90. NavBuilder now exits 3 on it, pinned by `a_vertical_extent_past_the_13_bit_span_height_is_refused`.
- **Containment.** Every NA26 world is seeded `navmesh_mode = 'advisory'` (21 rows); Castle_CellBlock is the one meshed world left on `enforce`. The live-DB guard `only_the_documented_worlds_are_seeded_advisory` pins the list.
- **Tests.** The Harset fixtures (`line_of_sight_tests`, `off_mesh_sentry`, `advisory.rs`, `arrival.rs`, `harset_placement_tests`, `world57_placement`, `interior_regions`) re-derived on the new meshes, same bug shapes.

**Acceptance:** entity, navmesh-extractor and services suites green, live-DB suite green on the reloaded seed; the telemetry comparison in `data/spaces/README.md`. **Owner decisions left open:** whether PL-A-01's Harset arrival pin should be dropped now that the gate row is on-mesh, and whether the six Harset rows that moved onto the mesh (303, 304, 306, 307, 308, 313) should stop being `is_stationary`.

### NA29

**Status:** Review (branch `npcai/na29-harset-arrival` pushed, no PR). **Scope title:** Harset gate arrival on the gate row; five world-57 spawns made mobile. **Advisor:** movement-teleport-advisor. **Owner decisions (2026-09-25):** "As long as the arrival is on the navmesh and doesn't cause issues put it in the original location", and yes to un-stationarying the rows NA26 moved onto the mesh. These close the two decisions NA26 left open.

**Scope:**

- **Gate arrival.** The four `arrival_*` values on the `'Harset'` stargates row are `NULL` again, so travellers arrive on the gate row (-0.076, -67.274, 38.011), yaw 3.141, as the 2009 server did. On the NA26 mesh the row is on the gate dais (dy -0.04). The agent-radius disc is on-mesh at r 0.6 and 1.2 (13/13 each). `find_path` is `Ok` to the plaza, respawner 20, the DHD, all five ring pads and the Command Center door. `validate_gate_arrival` returns `Validated`, and the yaw faces out of the gate.
- **The gate volume.** The row is inside point set 1001 `Harset.Stargate`, 0.72 m off the axis of the 2.5 m cylinder. Landing there fires nothing. The dial is per entity, and a traveller's dial is cancelled and scrubbed before the move, so its enter hint is a no-op even while another player has the gate open. No chain triggers on the tag, and 1001 is not a ring pad. Ledger: [PL-A-01, dropped](../harset-rebuild/placements/A-arrival-and-travel.md#pl-a-01--the-pin-was-dropped-na29).
- **Spawns.** 303, 304, 306, 307 and 313 are `is_stationary = false`. Each has a 13/13 disc at r 0.6 and an `Ok` path to the gate. 308 stays stationary: it is 3.44 m above the mesh surface and passes only on the 4.0 jump tolerance, probably standing on a raised platform the extractor does not decode. Ledger: [NA29: five rows walk](../harset-rebuild/placements/B-world57-population-and-regions.md#na29-five-rows-walk).

**Acceptance:** each new or changed guard is revert-proven:

- `harset_gate_arrival_is_the_gate_row_on_the_mesh_and_inert_in_the_gate_volume` fails when the pin is re-added, when the row is moved off the dais or out of 1001, and when an `enter_region` chain is keyed on `Harset.Stargate`.
- `a_traveller_arriving_while_another_player_holds_an_open_dial_is_not_crossed` fails when the crossing looks up any open dial instead of the traveller's own.
- `world57_mobile_placements_can_walk` fails when a mobile row's disc leaves the mesh or 308 is re-pinned to the surface.
- `world57_placement_rows_are_seeded_with_their_tags_and_templates` fails when a mobile row is set stationary again.

The full services live-DB suite is green (3,227 tests). UAT: dial Harset from Castle and cross. You land on the gate dais facing the plaza, can walk off at once, and are not sent back through the gate. The two Lan'toc Jaffa, the two listening-device baskets and Petbe's quarters search object still stand where they were seeded.

### NA28

**Status:** Review (branch `npcai/na28-tiled-navmesh` pushed, no PR). **Scope title:** Tiled navmeshes so the large exteriors get full coverage. **Advisor:** movement-teleport-advisor. **Owner approval (2026-09-25):** "Yes" to the tiled-mesh follow-up NA26 left open.

**Scope:**

- **NavBuilder.** `tile=<cells>` builds one `rcPolyMesh` per tile (RecastDemo's tile border, per-tile logging, `threads=`), filters the small islands Recast leaves on tile seams, checks the 22-bit poly-ref budget, and writes a tiled `XRCT` `.nav`. `tile=0`, the default, is byte-identical to the old builder. The Recast pipeline moved to `recast_pipeline.cpp` so both modes share it. Method: `docs/engine/navmesh-build-pipeline.md` §10.
- **Loader.** `crates/entity/src/navigation/load.rs` detects the layout by its magic; `load_tiled.rs` adds every tile to one `dtNavMesh`, under per-tile header caps, with the fingerprint hashing the whole file as before. `nav_inspect` reads both layouts and links portals with Detour's test.
- **Meshes.** Agnos, Lucia, Tollana and Beta_Site_Evo_1 rebuilt at full extent (no crop), Dakara_E1 and both Menfa maps at `cs=0.3`. All stay `navmesh_mode = 'advisory'`; no seed change. Numbers and the old-vs-new comparison are in `data/spaces/README.md`.
- **Tests.** Synthetic two-tile loader tests (path, height, sight line, slide and recovery across the border; unlinked tiles stay islands; hostile tiled headers), `nav_tiled` round-trip, `NavGraph::from_tiled` portal linking, `nav_inspect` on a tiled file, and `tests/navbuilder_tiled.rs` against a tree-built NavBuilder (seams rejoin, output independent of the thread count, the seam filter is what removes a straddling island). No test was pinned to the seven meshes.

**Acceptance:** entity, navmesh-extractor and services suites green; the probe and coverage comparison in `data/spaces/README.md`. **Owner decisions left open:** none new. Promoting the tree-built NavBuilder to `bin64\NavBuilder.exe` is a local step outside the branch.

### NA27

**Status:** Review (branch `npcai/na27-occluder` pushed, no PR). Phase 1 was no-go under the first size budget. The owner then raised the budget and asked for paging (2026-09-25, D-NA13), and phase 2 ships an `.occ` for all 23 worlds. **Scope title:** Line of sight from collision geometry ([#784](https://github.com/SandboxServers/Cimmeria/issues/784)). **Advisor:** npc-ai-spawn-advisor.

**Scope:**

- Phase 1: an occluder format (a column grid of solid Y spans with sub-cell rectangles, plus an exact terrain heightfield), built for all 23 maps at 0.25, 0.5 and 1.0 m. Measure size, RAM, build time and accuracy on NA16's sweep.
- Phase 2, only on a go: load `data/spaces/<world>.occ` beside the `.nav`, and let it replace the navmesh ray for aggro, attack and cover sight.

**Result:** see [worknotes/na27-occluder-phase1.md](worknotes/na27-occluder-phase1.md) (phase 1 and phase 2) and the per-world table in [data/spaces/README.md](../../../data/spaces/README.md#occluders-occ-na27).

- **Accuracy.** At 0.5 m there are no false clears on either sweep. False blocks are 1.24% (Castle_CellBlock) and 1.05% (Castle) of truly clear pairs, and most of them are rays grazing within 0.1 m of a wall edge. The navmesh is wrong on 38% (Castle_CellBlock) and 49% (Castle) of its `Blocked` answers.
- **Query cost.** 1.9 µs per segment.
- **Why no-go.** The owner's rule was that the biggest world must fit in about 10 MB on disk and 50 MB of RAM. Agnos needs 57 MB and 288 MB at 0.5 m, and 22 MB and 98 MB at 1.0 m. Fifteen of the 23 worlds fit, including every Castle and Harset world.
- **What shipped (phase 1).** `crates/occluder` (format, builder, segment test) and `occluder_extract` (build, measure, probe), with synthetic tests.
- **Phase 2.**
  - **Paging.** 64 m pages, each compressed. Only the pages within 132 m of a player are resident (`refresh_occluder_residency`, 1 Hz), and a query on a packed page unpacks it on the spot, in 0.2-1 ms.
  - **Trim.** Coverage is the navmesh components that hold a real entry point, plus 15 m. That also clips Omega_Site_CmdCenter's giant triangles.
  - **Integration.** The occluder replaces the navmesh ray for aggro, attack and cover sight (`los_policy=occluder`, `npc_ai.los source=occluder`), and D-NA11 and D-NA12 no longer apply where a world has one.
  - **Numbers.** The 23 files total 131.4 MB after NA28's tiled meshes (the seven rebuilt worlds are marked in the data README). Agnos is 30.5 MB on disk, 168.0 MB with every page unpacked and 4.0 MB with one player. A query costs 1-5 µs.
  - **Tests.** They pin the drone at the desk, `Hallway02_Guard` at the wall, `Hallway01_Guard` at all three of Lomiada's spots (it sees her at 13.6 u too), guard-room walls, storeys, the off-area fallback and residency, and each is revert-proven.

## Phase 5: shared-world player visibility

### NA34

**Status:** Review (branch `npcai/na34-player-visibility` pushed, no PR). **Scope title:** Two players in a shared world (Castle, Harset) not reliably seeing each other. **Advisor:** aoi-witness-broadcast. **Owner report (2026-09-25):** "multiple players on the same map like castle can't reliably see each other... I think it's the first player on the map can see the second but the second can't see the first? Just a guess."

**Scope:**

- **SigNoz evidence.** 30 days of `cimmeria-server`/`cimmeria-trace` logs show every player-to-player `aoi.entity_enter` pair after PR #737 (2026-09-19) landing bidirectionally and `is_player=true` on both legs, with zero `aoi.player_ghost_incomplete`, `aoi.entered_no_witness_addr` or `aoi.create_send_failed` occurrences in that window. The one asymmetric, `is_player=false` pair found (space 65544, 2026-09-17) predates #737 and is Root Cause 2 from [player-ghost-aoi-cascade.md](../../architecture/player-ghost-aoi-cascade.md) firing on the pre-fix build, not a live regression. No telemetry exists for the owner's 2026-09-25 report itself — the dev SigNoz overlay has no player-to-player AoI activity after 2026-09-21.
- **Code audit.** `SpaceManager::compute_aoi_changes` (`cell/space_manager/aoi.rs`) iterates every player in `space.players` symmetrically every 100 ms tick; `CellEntity::is_introducible()` and the `entity_to_addr` identity-stamp-at-`CreateEntity` path (`cell/service/base_messages/lifecycle.rs`) are both synchronous with no `.await` gap that could race the AoI tick. No server-side bug was found that reproduces a *permanent* one-direction failure on current `main`.
- **New regression test.** `base::world_entry::cell_dispatch::tests_dispatch_arms::two_player_visibility::both_arrival_directions_deliver_the_observee_identity` drives two real `ConnectedClientState` sessions sharing one `connected`/`entity_to_addr` map — A already ready, B mid-load — through both `EnteredAoI` directions in the same tick, then flushes B's deferred buffer on its `onClientReady`. It asserts each witness's wire cascade decodes to the *other* player's real identity (name, level, archetype, appearance), not the bare NPC-shaped cascade. This is the "arrival order B" case [Known gaps](../../architecture/player-ghost-aoi-cascade.md#known-gaps) flagged as never validated end to end; it currently **passes** on `main`, narrowing (without disproving) a live server-side bug.
- **Observability.** Two new DEBUG rows at `target: "aoi.introduce"` (`aoi_dispatch.rs`'s buffering branch and `deferred_flush.rs`'s flush branch), each carrying `witness_id`, `entity_id`, `is_player`, `outcome` (`deferred_not_ready` / `flushed_on_ready`). A SigNoz query on `(witness_id, entity_id)` now shows the full hold duration for one introduction — the exact pair this investigation needed and didn't have.

**Acceptance:** the new fan-out byte test passes and is revert-proven (reverting the deferred-buffer join in `player_ghost::compose_cascade_body` trips it, matching the existing `player_ghost.rs` guard style). `aoi.introduce` rows appear in SigNoz on the next real two-player session.

**Outstanding:** the owner's report has no matching telemetry and the SpaceManager/base-dispatch level cannot reproduce it. The next real two-client session should watch for `aoi.introduce` (`outcome=deferred_not_ready` with no matching `flushed_on_ready`, or the reverse) and grep for `aoi.player_ghost_incomplete` / `aoi.entered_no_witness_addr` — either would pin a live bug this packet's audit could not find on paper.

### NA37

**Status:** Done (2026-09-25, two rounds). **Scope title:** Reproduce shared-world player visibility end-to-end with two real wire clients, following up on NA34's in-process investigation; round 2 adds real-network chaos (packet loss, reorder, jitter, latency) per the owner's colo-vs-localhost concern. **Advisor:** aoi-witness-broadcast.

**Round 1 — real wire path, lossless localhost:**

- **`cimmeria-wireclient` grew a real UDP socket loop** (`src/session.rs`'s `GameSession`), closing wireclient's Phase 1.5 gap for this scenario by reusing `cimmeria_mercury::test_harness::LoopbackPeer` — the Tier 2 loopback harness's Channel driver — against a real `BaseService` socket instead of a paired test peer. No second reliable-delivery/reassembly implementation was written. A structural server→client bundle decoder (`src/bundle.rs`) was ported from `tools/pcap_dissect.py`'s `SERVER_MSG_FORMAT` table (msg_id/entity_id/class_id/method_index; full per-argument semantics remain wireclient Phase 3).
- **`crates/wireclient/tests/two_client_castle_visibility.rs`** (live-DB): two real `GameSession`s enter a real spawned `Orchestrator`'s Castle world (world 8) via sentinel `sgw_player` rows this test inserts and cleans up. Asserts, in **both** arrival orders, GM + non-GM: each witness's decoded wire bytes carry the other's `CREATE_ENTITY`, a `BEING_APPEARANCE` cascade entry, a movement relay, and a `leaveAoI`/`entityInvisible` after disconnect. A companion test asserts a character >100m away (`CellEntity::aoi_radius`) is **not** introduced — the negative control against a harness bug that would make the positive assertions pass trivially.
- **A real test-infrastructure bug found and fixed along the way, initially indistinguishable from the reported one.** The first runs showed **zero** cross-player introduction in either direction — exactly the owner's symptom. Root cause: `CellService::start()` loads `entities/spaces.xml` from the process's *current working directory*-relative path `"entities"`, and `cargo test`'s CWD for an integration test binary is the package directory, not the repo root — so `CreateEntity` logged `"Unknown world: Castle"` and neither player entity was ever registered in a real `SpaceManager` space. Fixed by `chdir`-ing the test process to the repo root before starting the server. A trap for *any* future test that spins up a real `Orchestrator`/`CellService` from outside `cimmeria-services` itself.
- **Confirmed, not a bug: every player-ghost introduction is class-flattened to `SGWPlayer` (0x02),** even for a GM observee — `connect_entity` stamps `class_id = 0x02` for every player's cell identity, matching `player-ghost-aoi-cascade.md`'s documented "GMs are introduced as plain players" gap. The test asserts the documented behavior rather than flagging it.
- **Result: full pass, both directions, both arrival orders, GM and non-GM** — agreeing with NA34's in-process finding, but ruling out a wire encode/decode bug specifically, which NA34's level could not.

**Round 2 — real wire path, under injected network chaos:**

- **Added `crates/wireclient/tests/two_client_castle_visibility_chaos.rs`**, using the project's network-chaos primitives (TESTING.md type 10 / `docs/architecture/network-chaos-testing.md`) on the real `BaseService` transport via a new `chaos-testing` feature + `BaseService::set_transport_override` seam: ~7% loss + jitter during world entry, a deterministic destination- and size-filtered targeted burst-drop (`LossyTransport::drop_next_sends_to` with `min_len`, added this round so a test can hit "the next *substantial* packet to this witness" without an incidental tickSync keepalive consuming the armed drop), and elevated latency. Also added jitter and a per-destination send-side reorder buffer to `crates/mercury/src/lossy_transport.rs` (bucketed by address — a shared buffer broke the Mercury phase-3 handshake's positional two-packet parsing when multiple clients' traffic interleaved in it; documented as a harness limitation, not exercised end to end for that reason).
- **Confirmed the ordering-hazard hypothesis NA37 round 1 flagged as the leading remaining explanation.** With the burst-drop precisely targeting a witness's `CREATE_ENTITY` for a peer (confirmed via `RUST_LOG=mercury.lossy_transport=debug,aoi.create_emit=debug` while tuning the targeting), the peer's full appearance/stat cascade and repeated position updates arrive at the witness **before** the retransmitted `CREATE_ENTITY` does — dozens of messages for an entity the witness was never told exists.
- **Root cause: `Channel::receive_packet`'s in-order RX-window delivery gate is fully implemented and unit-tested but never wired into any live receive path**, harness or production. Confirmed by grep: it is called from exactly one place in the whole codebase, its own test (`crates/mercury/src/channel/tests/channel_lifecycle.rs`). `Channel::reassemble_parsed` (what both the harness and, implicitly, production actually use) returns a non-fragmented packet's body immediately with no ordering check, and `FragmentAssembler`'s `pending` map has no cross-bundle ordering either. Neither NA34 nor NA37 round 1 could have surfaced this — both ran with zero packet loss, so no retransmit (and therefore no reordering opportunity) ever occurred.
- **Attempted a server-side fix and reverted it.** Made the AoI cascade send in `aoi.rs::entered_aoi` wait for `CREATE_ENTITY`'s ACK (polling the witness's Channel TX window for the packet's `seq` to clear) before firing the cascade. This closes the hole for the *cascade* specifically, but broke the lossless tests: Mercury only piggybacks ACKs on the peer's own next outbound send, so a witness that hasn't sent anything since receiving `CREATE_ENTITY` doesn't ACK promptly even with zero packet loss — the wait stalled *every* entity introduction (all 5+ NPCs a fresh player discovers on login, plus the peer player) for up to the wait's timeout, an unacceptable universal latency regression traded for a narrow packet-loss fix. It also wouldn't have closed the *movement*-update leak (`entity_moved` fires from an independent per-tick path with no equivalent gating).
- **What a safe fix needs (not shipped this round):** either genuine cross-packet in-order delivery — wire `receive_packet`'s existing gate into the live receive path on both the server (for client→server traffic; today `handle_encrypted_datagram` calls `parse_incoming` directly with no Channel-level ordering at all) and the client/harness side (`LoopbackPeer`'s recv pump, which `GameSession` depends on) — or a *reactive* per-entity hold that only engages once Mercury's own retransmit driver actually resends `CREATE_ENTITY` (`TxEntry::retransmit_count > 0`), never on the happy path. Either is a genuine protocol-layer change needing its own design review, RE verification of whether the real 2009 client itself relies on in-order delivery, and broad regression coverage — out of scope for this packet.
- **The reproduction is preserved as an `#[ignore]`d regression test** (`burst_drop_of_peer_create_entity_recovers_via_retransmit`, run with `cargo test -- --ignored`) rather than shipped failing or silently dropped, so whoever picks up the real fix has a ready, precise repro. The other three round-2 scenarios (lossy network, latency) pass and are not ignored.

**Combined conclusion:** the owner's report is very likely this exact mechanism — a real player over the internet (packet loss NA34/NA37-round-1 never modeled) hitting the ordering hazard. Neither in-process dispatch (NA34) nor the lossless wire path (NA37 round 1) could have found it; injecting real loss (NA37 round 2) did, on the first deterministic attempt.

**Acceptance:** `cimmeria-wireclient` unit tests pass (`bundle.rs`'s 6 decoder tests; `crates/mercury`'s `lossy_transport` unit tests, 13 total including the new jitter/reorder/targeted-drop/min-len cases). `two_client_castle_visibility.rs`'s two tests and `two_client_castle_visibility_chaos.rs`'s three non-ignored tests pass locally against a live DB, stable across repeated runs (not yet wired into CI — see `docs/architecture/wireclient.md` Phase 7). `cargo fmt --check` clean; `cargo +1.98.1 clippy` clean for the touched crates.

**Outstanding:** wire the live-DB wireclient tests into a `wireclient-e2e` nextest profile / CI job (wireclient.md Phase 7); design and implement the real ordering fix (see "What a safe fix needs" above) and un-ignore `burst_drop_of_peer_create_entity_recovers_via_retransmit` once it lands; capture a real SigNoz `aoi.introduce` trace from the owner's *next* live session for corroborating evidence; verify against the real 2009 client binary (Ghidra) whether it actually relies on Mercury-level in-order delivery, which would confirm real players are exposed to this and not just a theoretical wire-protocol gap.

### NA31

**Status:** Review (branch `npcai/na31-fire-los-eye-height` pushed, no PR; ran in parallel with NA32 and NA33). **Scope title:** Player fire-time line of sight and per-being eye heights (D-NA14). **Advisor:** combat-systems-advisor. **Owner approval (2026-09-25):** "work on all still open items".

**Scope:**

- **Fire-time check.** `use_ability/fire_los.rs`: a player ability aimed at another entity, in a world with an occluder, is refused with `onErrorCode(0, ability, 39)` when the eye ray and every tolerance ray are blocked. The tolerance rays are the target one tick back, the shooter one tick ahead, and the target's eye 0.35 m to each side. The check runs straight after the range check, before the holster queue, so a refused shot draws no weapon and starts no cooldown. No occluder, or `Unknown`, never refuses. NPC launches are left alone, because the fight tick checks them in the same tick.
- **Auto-cycle.** One error when the loop's target goes behind a wall, then armed and silent until the line clears (`AbilityManager::auto_cycle_los_notified`). `ticks/auto_cycle.rs` was already over 700 lines on main, so its tests moved to `auto_cycle_tests.rs`.
- **Eye heights.** `resources.body_sets.eye_height` is seeded from each body set's reference-mesh bounds in the cooked client, and `BS_JaffaMale` gains a missing row. The cell loads the table at startup. `SpaceManager::eye_height_of` feeds NPC line of sight, cover `slot_has_shot` and the player check. `InitPlayerState` carries the character's `bodyset` to the cell. `npc_ai.los` gains `target_eye_height_used`.
- **Telemetry.** `abilities` DEBUG `event=los_refused` (source, both eyes, ray, hit, rays tried) and `abilities_los_refused_total{world}`.

**Evidence:** [being-eye-heights.md](../../reverse-engineering/findings/being-eye-heights.md) (error strings, pawn defaults, mesh bounds).

**Acceptance:** each guard fails when its piece is reverted:

- `a_shot_through_the_hallway_walls_is_refused_with_error_39`, on the real `castle_cellblock.occ` at UAT-1's through-the-walls spot, checks the exact bytes `00 07 00 00 00 27 00` and that no cooldown started. `the_refusal_is_logged_with_its_ray` covers the log row. Both fail with the gate removed.
- `a_clear_shot_over_the_counter_fires` is Lomiada at 13.6 u from `Hallway01_Guard`. `a_world_without_an_occluder_never_refuses` covers no occluder. `an_endpoint_off_the_occluder_grid_is_allowed` fails when `Unknown` refuses.
- `a_target_running_into_cover_is_hit_where_the_client_still_sees_it`, `a_shooter_stepping_out_of_cover_is_ahead_on_its_own_client`, `a_target_half_behind_the_corner_is_hit_on_its_body_edge` and `the_tolerance_rays_do_not_see_round_a_real_corner` all fail with the tolerance rays removed.
- `auto_cycle_tick_tells_the_player_once_when_the_target_is_behind_a_wall` fails with the gate removed and with the one-shot notice removed.
- `a_low_wall_hides_a_rat_but_not_a_jaffa`, `across_the_med_station_desk_a_rat_is_hidden_and_a_human_is_not` (real geometry), `npc_line_of_sight_casts_between_body_set_eyes` and the `InitPlayerState` body-set assertion all fail when eye heights revert to 1.5 m.
- `occluder_los_eye_heights` re-checks NA27's Castle_CellBlock pins at the seeded eyes: human guards, the floating drone at 3.31 m, and human and Jaffa players. The drone still sees over the desk, `Hallway02_Guard` stays blind, `Hallway01_Guard` sees Lomiada, and walls and storeys still block.
- Live-DB: `seeded_eye_heights_load_by_body_set`, `every_spawned_being_body_set_has_an_eye_height` and `a_seeded_jaffa_looks_from_its_measured_eye` fail on a seed with no `BS_JaffaMale` row and the human value set back to 1.5.

**UAT:** in Castle_CellBlock, shoot a guard from behind a hallway wall. The client shows its line-of-sight message and fires nothing. Step out, and the shot fires. With auto-attack on, the message appears once and the loop resumes by itself.
### NA32

**Status:** Review (branch `npcai/na32-cover-defense-stepback` pushed, no PR). **Scope title:** Cover Stance in the hit roll, and the ranged step-back. **Advisor:** combat-systems-advisor. **Owner approval (2026-09-25):** "Work on all still open items", which covers NA22's unwired stance and the step-back NA15 left for a decision. Recorded as D-NA15.

**Scope and result:**

- **Evidence.** SGW.exe resolves no hits; its only cover stat string is the `CoverQRModifier` Lua label. The units come from the client's `alias.xml` stat comments: `coverDefense` -0.01 QR per point (line 235), `coverAccuracy` +0.01 against a target in cover (234), `coverQRModifier` 1 QR per point for whichever side is behind cover (216). The cooked text agrees that 100 points = 1 QR (1729 vs 4299, 1450 vs 4995). "Cover Penetration" is `coverAccuracy`. Magnitude: the effect row's +100 (4565, like 1746, 1747 and 2003), not the ability text's +200. See [combat-system.md](../../gameplay/combat-system.md#cover-as-damage-reduction-na32).
- **Formula (round 2, D-NA15a).** `combat::cover_reduction` (`combat/damage/cover_damage.rs`): in unflanked cover the hit loses `clamp(COVER_RATING[quality, height] + 0.1 x coverDefense + 10 x coverQRModifier - 0.1 x attacker coverAccuracy, 10, 60)` percent, applied after `(1 + qr)` by `calculate_damage_scaled`; flanked or away from a node, 0. An attacker in cover still gains its `coverQRModifier` as QR. Round 1's -1 QR penalty is gone. `SpaceManager::cover_standing` (`space_manager/cover_hit.rs`) decides "in cover": an NPC within 1.5 u of its held slot, a player within 1.5 u / 2 u of the nearest node, and the attacker not flanking (`is_flanked`, 20 degrees past side-on). `abilities/damage_apply/cover_roll.rs` resolves it and writes `abilities.qr event=cover_resolved` (`cover_quality`, `cover_height`, `base_pct`, `stance_pct`, `penetration_pct`, `final_pct`, `flanked`). No wire field changes layout.
- **The roll's direction.** The python beta branches pulled the mean *down* as QR rose, so a covered target showed more criticals and took the same damage. The branches are swapped (`combat/damage/qr.rs`); at QR 0 nothing changes.
- **Step-back.** `npc_ai/step_back.rs`: a mobile, not-in-cover NPC whose ability has a `min_range` or whose abilities are all ranged steps back from a target inside `max(min_range, 2 u)` to that range plus 3 u, slid along the navmesh, at most once every 3 s. It holds inside a hard `min_range` during the cooldown, finishes a step in flight, and fires instead when cornered. It replaces the fight tick's min-range backup, whose label it keeps for that case. Melee (or mixed-set), stationary and in-cover NPCs never step back; a flanked one has released its slot and does.
- **Tests (each revert-proven).** `cover_damage` units (lunch table 10-20%, wall 50-60%, typical slot 25/35%, the band for every node and stat stack, penetration floor, flanked 0); `cover_hit` standing (slot, walk-to, flank band, player floor); `damage_apply/cover_tests.rs` through `apply_damage_to_target` (the guard slot takes exactly 35% off, wall 60% vs table 15%, flanked and away-from-cover take the exposed hit, a covered guard always takes at least 40%, covered player, the telemetry row); `qr` distribution tests (`expected_damage_rises_with_qr` and the band tests fail on the python order); `tests/npc_ai/step_back.rs` (steps back, fires outside 2 u, melee, stationary, cooldown, in flight, dead-zone hold, in cover, flanked) and two on the real `castle_cellblock.nav` (the step lands on the mess-hall floor; cornered against a wall it fires). Two `ability_range` tests moved their player from 1 u to 4 u, outside the comfort range.

**Acceptance:** UAT: shoot a Cellblock guard in its cover slot from in front, then from its flank; SigNoz `abilities.qr event=cover_resolved` shows `defender_cover=in_cover final_pct=35.0` (a Mid/Better slot) then `flanked=true final_pct=0.0`. Walk into a guard's face: it backs off about 3 u (`decision_outcome=step_back`) and no more than once per 3 s.
### NA33

**Status:** Done (merged 2026-09-25, branch `npcai/na33-aggression-broadcast`). **Scope title:** Broadcast the NPC aggression level to clients (D-NA16). **Advisor:** npc-ai-spawn-advisor, combat-systems-advisor.

**Scope:**

- Verified the SGWMob flat `ClientMethod` indices Ghidra-side: `onAggressionOverrideUpdate` = 27, `onAggressionOverrideCleared` = 28. SGWMob and SGWPlayer share indices 0-26 (the `SGWSpawnableEntity` → `SGWBeing` prefix is structurally identical for both); SGWMob's own `<Implements>` is just `Lootable`, whose `<ClientMethods>` is empty, so its own two methods begin right after the shared prefix. Client registers both through `MemberCallback<GameMob, ...>` at `0x00d31cd0`; the Update handler at `0x00d31bd0` reads the `aAggressionLevel` INT8 and stores it at `GameMob + 0x16c`. See [findings/npc-aggression-broadcast.md](../../reverse-engineering/findings/npc-aggression-broadcast.md).
- Added `mercury::method_idx::ON_AGGRESSION_OVERRIDE_UPDATE` / `ON_AGGRESSION_OVERRIDE_CLEARED` and the SGWMob rows in `docs/protocol/client-method-dispatch-table.md`.
- Wired the broadcast into all three runtime-change call sites (content `set_aggression` action, the `.aggression` GM console command, the surrender/`npc_ai_submit` disarm) and the AoI-entry replay (mirrors python `SGWMob.createOnClient`'s conditional send — only when an override is active). Deliberately does **not** replicate legacy `setAggression`'s `onEntityProperty(GENERICPROPERTY_MobAggression)` broadcast: NA13 found no client consumer for that property type, so the literal legacy wire call was dead code in 2009. `onAggressionOverrideUpdate`/`Cleared` are the ClientMethods the client actually has a live, working handler for — same intent `createOnClient` already had, extended to the runtime-change path python never wired it for. No client patch.
- `UIAggressionLevel` (RTTI `0x01de972c`) is registered alongside `UIArchetype`/`UIStatType`/`TargetType`/`UIDamageType` as a Lua-scriptable enum type (`0x00ab1a5e`), not a CEGUI widget — the family the UI/reticle Lua layer reads for nameplate and interaction-verb display. The specific consuming Lua script was not located (no client Lua source in this tree); this is inferred with medium confidence, not directly observed.

**Acceptance:** byte-exact wire-format tests (`aggression_wire_tests.rs`) and fan-out tests, each revert-proven; AoI-entry replay tests (`aoi_entry_replays_active_aggression_override`, and the negative `aoi_entry_sends_nothing_for_a_faction_derived_mob`); a surrender-broadcast test (`submit_broadcasts_the_disarm_to_witnesses`); a console wire test (`na13_console_aggression_broadcasts_update_then_cleared`). Full `cimmeria-services` suite green (3,242 tests); `cimmeria-server` logging target-scan guard green (no new custom tracing target introduced).

### NA36

**Status:** Done (branch `npcai/na36-extractor-gaps`, 2026-09-25). **Scope title:** Navmesh/occluder extractor geometry gaps (raised platforms with no decoded collision). **Advisor:** movement-teleport-advisor.

**Scope:**

- `staticmesh::MESH_ACTOR_CLASSES` widens the walker's class filter from exactly `StaticMeshActor` to also include `KActor` and `FracturedStaticMeshActor`, unconditionally — both derive from `AStaticMeshActor` and share its placement + `StaticMeshComponent` shape, so the same resolver walks them unchanged (collision-flag gating is untouched). `InterpActor` is **opt-in**, default off (`staticmesh::OPT_IN_MESH_ACTOR_CLASSES`, `ExtractOptions::include_interp_actors`, CLI `--include-interp-actors`): a same-day follow-up walked back the first pass's unconditional inclusion after a review flagged that `InterpActor` is disproportionately doors, gates, lifts and elevators in this content, and baking a mover's cooked (often closed) pose into a `.nav`/`.occ` risks sealing a doorway or blocking sight through an opening a player can actually use. `coverage::decode_status` now takes the run's flag as a parameter so the coverage report correctly flags `InterpActor` as a risk whenever a rebuild forgets to opt in. Full method and evidence: [navmesh-build-pipeline.md §11](../../engine/navmesh-build-pipeline.md#11-mesh-actor-class-gap-interpactor--kactor--fracturedstaticmeshactor-na36-2026-09-25).
- 23-map class census after the fix: `InterpActor` is present in 15 of 23 maps (965 exports total, Harset alone has 31). `KActor`, `FracturedStaticMeshActor` and `StaticMeshCollectionActor` (the one class deliberately left unimplemented — its array-of-components shape needs its own walk) have **zero** exports anywhere in the shipped 2009 client content.
- Classified all 754 successfully-resolving `InterpActor`s by mesh name (23-map scratch diff, not shipped): 77% are load-bearing static-shaped props (332 `GLB-RingTransporter00` ring-transport platforms alone, plus lamps, antennas, parked vehicles), 7% are literal doors or a Stargate's rotating chevron mechanism — the exact risk that motivated the opt-in — and 16% are ambiguous security-camera heads. Full table in navmesh-build-pipeline.md §11; a future packet enabling `InterpActor` per-map should trust the static-shaped majority and individually review or exclude the door/mechanism minority.
- `harset.nav` rebuilt from the fixed extraction **with `--include-interp-actors` on** — Harset's 31 `InterpActor`s were checked by hand and are all console platforms and other static dressing, none a door (29,768→29,772 verts, 15,287→15,289 polys, 42,379→42,385 edges) and `harset.occ` alongside it (self-check `paged == unpaged` passes, +635 bytes). `harset_cmdcenter.nav` (source for `sandbox.nav` too) rebuilds byte-identical regardless of the flag and was left untouched; `harset_market.nav` / `harset_storagerm.nav` carry zero instances of any of the three classes and were not rebuilt. No other map has been rebuilt with the flag on — all 14 other `InterpActor`-carrying maps default off until individually checked the same way.
- Regression check: `nav_inspect --probes` against both the 67-row seeded NA26 probe set and the 41-row telemetry-derived `harset_lastvalid_probes.txt` gives an **identical** pass/fail set before and after — a strict non-regression. The specific "6 lost positions" the packet was framed around (five clustered 8-10 m above ground near `x[9,47] z[-91,59]`, plus spawn 308's known 3.45 m gap) are **not** resolved by this fix: exhaustive `obj_slab` / actor-proximity checks against the fixed extraction found no export of any class near the five clustered points at the target height, and confirmed spawn 308's tower prefab is already decoded and present — its problem is Y-calibration on a hillside compound mesh (NA29's existing conclusion), not a missing class. Both are left open; see the finding for the leading unconfirmed hypothesis (an animated mover whose cooked pose is not its runtime position).
- Seeded-spawn Y audit (Harset only, the world with concrete telemetry evidence): of the still-open off-mesh `spawnlist` rows, none met the "seed Y wrong, high confidence" bar — 308 needs a live `.location` reading, 310's gap is a navmesh-connectivity issue (real terrain exists almost exactly at its seeded Y), and 311 is already flagged LOW-confidence/INFERRED with its own deletion candidate note. No `db/resources` seed changes shipped. Full table in navmesh-build-pipeline.md §11.

**Acceptance:** `cimmeria-navmesh-extractor` 424/424 (including `mesh_actor_class_tests.rs` cases proving `KActor`/`FracturedStaticMeshActor` resolve identically to `StaticMeshActor` regardless of the flag, `InterpActor` is invisible with the flag off and resolves with it on, and an out-of-family class like `Pawn` is ignored either way; plus CLI-parsing tests for the bare `--include-interp-actors` flag and its occluder-side `true`/`false` equivalent); `cimmeria-entity` and `cimmeria-services` full suites green against the rebuilt `harset.nav` (`harset_placement_tests`, `world57_placement`, `off_mesh_sentry`, `movement_validation`, `line_of_sight` all pass unchanged); `cimmeria-server` green. `harset.occ`'s own self-check passed.
### NA35

**Status:** Review (branch `npcai/na35-gate-travel-cinematic` pushed, no PR; code shipped once the owner lifted the session's usage restriction). **Scope title:** Stargate dial timing and gate-travel cinematic. **Advisor:** game-archaeology-specialist. **Owner request (2026-09-25):** "Is there a gate travel animation/cinematic we can put up for gate travel before putting up the loading screen for the transition between maps?" Tester Lomiada: "the dialing is quite fast/done when i leave the dhd is supposed do be?"

**Correction applied mid-session:** the coordinator relayed the owner's clarification that the deprecated legacy server never had working gate travel end to end, so `deprecated/python/cell/SGWPlayer.py` is not behavioural evidence — ground truth is the client binary alone. This packet's findings and D-CA20 (`docs/analysis/castle-rebuild/README.md`) are grounded entirely in `SGW.exe` Ghidra evidence, not the legacy Python.

**Findings** (full detail in [`stargate-dial-and-travel-sequences.md`](../../reverse-engineering/findings/stargate-dial-and-travel-sequences.md)):

- The DHD dial UI collects all 7 glyphs client-side and reports the finished address to the server exactly once (`FUN_005682d0`) — there is no wire-level signal for in-progress chevron selection, so server-driven chevron broadcast (6106-6112) is impossible without a client patch, not merely unattested. **Not implemented** — this remains out of scope pending a client patch decision.
- `GATE_DIAL_DURATION` had zero client-binary support (its only source was the disavowed Python); the client's own DHD window closes on its own timeline as soon as dialling finishes, independent of the server — directly explaining Lomiada's "done when I leave the DHD" report.
- `Stargate_CrossGate` (6113) is a real, per-gate-instance Kismet trigger (`FUN_00e2c810`/`FUN_00d2de90`); Cimmeria's crossing path raced it against the world-transition teardown with no scheduled gap.
- `onStargatePassage` (client method 68) was declared (`ON_STARGATE_PASSAGE` constant) with a confirmed real client subscriber but zero production send call sites.

**Implemented** (once the owner lifted the session's usage restriction):

- `GATE_DIAL_DURATION` retimed from 4s to 100ms — the minimum the 100ms cell-tick-drain architecture can express; no confirmed client-binary duration exists to replace it with, so this removes the disavowed number rather than guessing a new one.
- `onStargatePassage` (68, 4-byte LE `addressId`) now sent to the crossing player only, immediately after `Stargate_CrossGate`.
- A `PendingCrossing` hold (`CROSSING_CINEMATIC_HOLD`, 1.5s, explicitly provisional) defers the `RESET_ENTITIES` teardown behind a `BSF_MovementLock`-locked wait, drained by a new `crossing_tick` on the existing 100ms cell tick. The lock is released if the deferred travel fails (destination vanished, unrecoverable arrival, closed base channel); the hold itself is cancelled by `destroy_entity`/`disconnect_entity` like the dial timer already was.
- `crates/services/src/cell/gate_travel/` (`mod.rs`, `sequences.rs`, `tick.rs`) and `crates/services/src/cell/space_manager/gate_dial_state.rs` / `crossing_hold_state.rs`.

**Acceptance:** fake-clock timer tests for both the retimed dial and the new crossing hold (`gate_dial_state` unit tests, each revert-proven — e.g. `dial_opens_almost_immediately_not_after_a_multi_second_hold` asserts `GATE_DIAL_DURATION < 500ms`); byte-exact wire tests for `Stargate_CrossGate`, `onStargatePassage`, and the `onStateFieldUpdate` movement-lock frame (`gate_travel::tests::dial_timer::crossing_an_open_gate_sends_cross_gate_then_onstargatepassage_and_defers_travel`); a hold-then-travel sequence test (`crossing_hold_elapsing_runs_the_deferred_travel`); a disconnect-during-hold test (`disconnect_during_the_crossing_hold_cancels_the_deferred_travel`); and a deferred-travel-failure test proving the movement lock is released, not left stuck (`deferred_travel_failure_clears_the_movement_lock`). Two pre-existing tests that asserted a synchronous `GateTravel` on crossing (`cell::cell_methods::player::world::tests::entering_a_stargate_region_with_an_open_dial_travels`, `cell::gate_travel::tests::arrival::crossing_into_an_unrecoverable_arrival_sends_no_transfer`) were updated to expire and drain the new hold first — without that they would have passed vacuously regardless of the arrival-refusal logic under test. `cimmeria-services` gate-related suite green (187 tests).

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
