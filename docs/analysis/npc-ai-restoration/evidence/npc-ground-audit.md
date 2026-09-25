> Evidence pass, 2026-09-24. Read-only research against `main` @ 192d4216 and 7 days of colo SigNoz data (2026-09-17 to 2026-09-24). Kept verbatim as the evidence record; the ledger in [../audit.md](../audit.md) supersedes it where they differ.

# NPC grounding and stuck-movement audit (symptoms 3 and 4)

Author: movement-teleport-advisor. 2026-09-24. Tree: `main` @ 192d4216. Read-only: no repo edits, no cargo.
Evidence sources: code (file:line), vendored Detour (`external/recast/Detour/Source/DetourNavMeshQuery.cpp`),
Ghidra on SGW.exe, and colo SigNoz, 7 days (2026-09-19 00:15Z to 2026-09-21 16:52Z; all rows Castle_CellBlock).

Confidence scale: **H** means verified in code plus telemetry or the binary. **M** means verified in code with
indirect telemetry. **L** means inference.

---

## TL;DR

1. **Symptom 3 (air-walking) has a single root cause: Detour straight paths are 2D, and our tick lerps Y along them.**
   `findStraightPath` emits a corner only where the path turns in XZ. A path that runs straight across flat floor and
   then up a ramp or stairs is **one segment**, from the start on the floor to the end on the ramp. `npc_movement_tick`
   interpolates Y linearly along that segment (`ticks/npc_movement.rs:183`), so the NPC climbs an invisible slope
   over the flat floor before it reaches the ramp. It then sinks into the upper landing when the stairs are steeper than
   the chord. Telemetry shows both effects. **H**
2. **The NPC is then frozen in the air.** When it comes into range, the fight handler calls `nav_path.clear()`
   (`fight.rs:550-552`) wherever the lerp left the NPC. The NPC fights from that height for the rest of the engagement.
   Colo data: Cellblock Guards attacked from Y 69.47 to 69.64 at an XZ where the same guard also stood at Y 68.6,
   so they hover about 1.0u. **H**
3. **Sending the OnGround variant (0x18) is NOT a fix, which corrects an earlier memory note.** In SGW's
   `EntityManager::onEntityMove`, the -13000 sentinel that OnGround writes into Y is replaced with the actor's
   **current client Location height**. There is no ray-cast. Switching to 0x18 would pin an NPC to its creation height.
   **H (binary)**
4. **Server-side grounding is the fix, but `get_navmesh_height` is broken on multi-storey maps.** It centres its search
   at `y = 0` with a ±500 box (`navigation/mod.rs:394-434`), so it returns the storey nearest world Y=0, not the storey
   under the NPC. The existing `ground_y` and `y_offset_from_ground` step telemetry is therefore wrong. Of 855 samples,
   397 read an offset of at least 3u. The false offsets are exact storey heights: 68.35, 39.37, 34.39, 24.5, 14.8, 12.0.
   Fix the query before building a clamp or any dashboard on it. **H**
5. **Symptom 4 (stuck some distance from spawn) is mainly leash, not the navmesh.** The leash check is the 3D distance
   from spawn to the **target**, with a global value of 50 (`fight.rs:193-196`, `aggro.rs:11`). The tutorial player
   stands at about (-316, 73.5, -195), which is 49.6 to 49.95 from the Cellblock Guard spawn (-289.5, 68.5, -154.3).
   The guard chases 10 to 30u, then leashes. Leash is a bare teleport that does **not clear `nav_path`**. The movement
   tick then walks the stale chase path from spawn in a straight line (confirmed in telemetry). Idle auto-aggro then
   re-seeds threat, Fighting leashes again immediately, and this repeats every 6 s. One guard logged 120 `leashed`
   rows at `dist_to_spawn = 0`. Leashes outnumber chases 353 to 120 over the week. **H**
6. **The hypothesis "air-walk, then off-mesh start, then no path forever" is refuted in the colo data. The mechanism
   is still latent.** There were zero `no start poly` warnings in 7 days, one `no end poly`, and two `path_fail` rows.
   Hovering guards still found a start poly, because Detour's candidate test uses poly AABBs and these guards stand
   near stairs, where the AABBs are tall. On a flat floor, a hover of more than 0.5u would fail `START_EXTENTS`. **M**

---

## A. Where each NPC mover gets its Y

The shared movement tick is `crates/services/src/cell/service/ticks/npc_movement.rs`:

- `:96-99`: distance is **3D**, so `dy` consumes step budget. On steep legs the NPC's horizontal speed drops.
- `:104-107`: on arrival the NPC snaps to `next_wp.y`. The comment claims this Y is "already on navmesh surface".
  That is only true for the first and last corners (detail-mesh projection). Intermediate corners are poly-mesh
  portal vertices, which is the ~0.15u sawtooth recorded in an earlier note.
- `:180-183`: **mid-leg Y is a linear lerp from the NPC's current Y to the waypoint Y.** There is no surface query.
- `:190-194`: the broadcast velocity includes `vy = dy/dist*speed`. `USGWAvatarFilter` extrapolates along it between
  updates, so the climb continues on the client's screen past the last packet.
- `:211`: `get_navmesh_height` is called **for logging only**, and on the wrong storey (see D1). Nothing clamps Y.

### Per-state sources of the destination and its Y

| Mover | Destination and its Y | Straight-line (non-mesh) fallback? | Evidence | Conf |
|---|---|---|---|---|
| **Fight chase** | `nav_target_pos` = target position (player Y) or a cover slot. `find_path(npc_pos, target)`. End Y is the detail-mesh projection of the player's position within `DEST_EXTENTS` ±3. | **No** fallback. On `None` it reports `no_path` and keeps the **stale** path. A degenerate path (≤1) also keeps the stale path. | `fight.rs:411-495` | H |
| Fight repath trigger | Repath only when `last_wp.distance_to(target) > 5.0` (3D). A player walking down a ramp toward the NPC often stays inside 5u of the old endpoint, so the NPC keeps walking the old, higher-ending leg. | n/a | `fight.rs:411-425` | H |
| **Fight in-range stop** | `nav_path.clear()` at whatever lerped Y the NPC holds. | Freezes the NPC mid-air, with no re-grounding. | `fight.rs:550-552` | H |
| **Fight min-range backup** | `compute_backup_waypoint` extrapolates along the 3D vector from target to NPC. Y = `target.y + (npc.y - target.y)*(min_range+1)/dist`. Player above means the backup goes below the floor; player below means it goes above. | **Yes**: a raw waypoint that never touches the navmesh. Through walls and off-mesh. | `ability_select.rs:190-208`, `fight.rs:517-523` | H |
| **Follow** | `dest` = 3D lerp toward the leader (includes leader Y). The routed case uses Detour, and the end projection has ±3 extents, so a lerped-into-the-air `dest` can snap to another storey. | Yes. Unrouted pushes `dest` with the **NPC's own Y** (fixed in #677). It is still straight through walls. | `follow.rs:108-165` | H |
| **Investigate** | `poi` comes from content (`SetNpcPoi` x, y, z; `content/executor/world/mod.rs:123`). | **Yes**: raw `poi_pos`, including authored Y. | `investigate.rs:121-150` | H |
| **Wander** | Candidate at spawn Y; `is_position_valid`, else spawn. | Yes: raw target on path failure. | `wander.rs:151-202` | H |
| **Patrol** | Authored patrol waypoint. | Yes: raw waypoint. | `patrol.rs:182-219` | H |
| **Leash** | `npc.position = spawn_pos`, a bare field write. **`nav_path` is not cleared, velocity is not zeroed, and `write_position` is not called** (grid desync). | The stale chase path is walked from spawn **in a straight lerp** by the movement tick. | `leash.rs:48-50`; the transition at `fight.rs:193-229` also does not clear it | H (telemetry below) |
| Escort (GC1) | Follow state (`SetFollowTarget`). | Same as follow. | `content/executor/world/mod.rs:136-199` | H |
| move_waypoint and others | Content actions only clear `nav_path` (`world/mod.rs:128,197,230`). No other direct `nav_path` writers exist in production code. | | grep of `nav_path.push/=` | H |

### The ramp case, end to end (NPC below, player above on a ramp)

1. The fight tick sets `nav_target_pos = player.position` (Y on the ramp, for example 73.47).
2. `NavMesh::find_path` (`navigation/mod.rs:441`). The start poly uses `START_EXTENTS` ±0.5 around the NPC. The end
   poly uses `DEST_EXTENTS` ±3 around the player, which finds the ramp poly. Detour `findNearestPoly` in this vendored
   version returns the detail-surface point, and prefers a poly directly under the point within climb height.
   A different storey is chosen only if the player's Y is more than 3u off every surface, or a closer storey overlaps
   the box. That is not the common case here, so **the endpoint is right**.
3. `findPath` then `findStraightPath` with `options = 0` (`detour_wrapper.cpp:172-178`). The string-pulling runs in
   **XZ**, and a vertex is added only at an XZ turn. Floor-then-ramp in a straight XZ line becomes
   **`[start(floor), end(ramp)]`**.
4. Telemetry confirms single long rising legs. NPC 100813: one leg of 20.8u from Y 34.8 to 39.6. NPC 100574: one
   45u leg from Y 68.5 to 73.6, toward the player at (-315.6, 73.6, -191.4).
5. The tick lerps Y along the chord. Over the floor section the NPC rises in the air. On the stairs the chord can fall
   below the surface (100813 measured -0.64u under the stairs at x = -70.7). Over the flat top it stays below the
   landing.
6. The NPC enters `max_range` (3D, and elevation shortens it), so `nav_path.clear()` runs and the NPC hovers for the
   rest of the fight. The player walking down the ramp keeps the NPC in range, so nothing ever moves it again. This is
   "especially when the player is above me on a ramp".
7. The client renders our Y exactly (0x10, see B) and extrapolates `vy`.

Also relevant: the straight-line fallbacks (backup, patrol, wander, investigate, unrouted follow) apply the same lerp to
**unvalidated** endpoints, so they can also leave the mesh horizontally.

---

## B. How the client renders NPC Y (Ghidra)

- We send only `0x10 NoAlias FullPos YPR`. The constant is at `mercury/aoi/mod.rs:43`, built in `aoi/update.rs:33-47`
  and `aoi/create.rs:94-101`. The physics byte is hardcoded `0x01`. **H**
- OnGround handler `FUN_00ddb830`: copies X and Z and sets **Y := `DAT_019d1a44`**. Bytes `00 20 4b c6` decode to
  **-13000.0f**, which is BigWorld's classic "on ground" sentinel, **not FLT_MAX** as the draft says. It then calls the
  handler at vtable+0x28. **H**
- That handler is `BW_client_entity_manager_6` (entity_manager.cpp, `0x00dd1859`+). It converts to UE units and
  checks each component against -13000. **If a component equals the sentinel, it substitutes the actor's current
  `Location` component** (`BW__unknown_00e685c0` returns `actor+0xdc`, the same field `FUN_00e68a30` writes as
  Location). The result goes to `FUN_00e68a30`, which is the filter input (vtable +0x10c on the filter at
  `actor[0xe5]`). **There is no ray-cast and no terrain query on this path.** **H**
- Therefore the claim in `docs/drafts/spec/position-updates.md:124-125` ("OnChunk from height map, OnGround from terrain
  ray-cast") is **wrong for SGW**: both keep the current client height. The same claim in the
  `npc-broadcast-facing-and-grounding` agent memory is also wrong.
- The wire physics byte (`param_1+0x15`) goes to `actor+0x58`, plausibly `AActor::Physics` (1 = PHYS_Walking). I did not
  verify whether UE physics ever grounds a `BigWorldEntity`. The memory note says collision is disabled on it. **L**

**Recommendation:** do **server-side ground clamping**, and do not use 0x18. With 0x18 the NPC keeps its create-time
height on every client and clips through ramps. Keep 0x10.

Proposed clamp:

1. Fix the height query to be storey-aware. Add `get_height_near(x, y_ref, z)`: `findNearestPoly` centred at
   `[x, y_ref, z]` with extents about `[0.5, 2.5, 0.5]`, then `getPolyHeight` on that ref. Or use the
   `isOverPoly`-aware overload. `y_ref` is the lerped Y. Use it in `npc_movement_tick` for every step and every snap:
   `new_y = height.unwrap_or(lerp_y)`.
2. Optionally pass `DT_STRAIGHTPATH_ALL_CROSSINGS` to `findStraightPath`, or better `moveAlongSurface` or
   `getPolyHeight` per tick on the corridor poly. Crossings add a vertex at every poly edge, which bounds chord error to
   one poly but still uses poly-mesh Y. The per-tick query is what fixes it.
3. Zero `vy` in the broadcast velocity when grounded, or derive it from the grounded delta, so the client filter does
   not extrapolate into the air.
4. Worlds without a mesh keep the lerp. Log it once per NPC (see D).

This does not touch client wire format and needs no client patch. The owner confirmed that preference.

---

## C. Symptom 4: why an NPC stops at a distance and stays stuck

### C1. Leash (dominant, **H**)

- `fight.rs:193-196`: `spawn.distance_to(&target_pos) > LEASH_DISTANCE (50.0)`, measured in 3D from spawn to the
  **target**. The player's elevation counts toward it.
- Colo: Cellblock Guard spawn (-289.5, 68.5, -154.3), across session instances 100299, 100574, 100630 and 100908.
  First chase at `dist_to_target` 49.6 to 49.95. The player stands right at the leash boundary.
- The guard leashes after moving 10 to 30u toward the player (`dist_to_spawn` 10.5, 21.3, 30.7 at the leash tick).
  This is "moves only so far from spawn".
- Leash loop: Idle auto-aggro (`fight.rs:25-78`, which seeds threat on any opposing player **in AoI**, with no range)
  goes to Fighting, the leash check fails at once, the state goes to Leashing, the NPC snaps and goes Idle. The period
  is 6 s (three AI ticks). NPC 100630: 120 consecutive `leashed` rows at `dist_to_spawn = 0`, 16:40 to 16:52Z. The guard
  looks "stuck" and never moves while the player stays more than 50 from spawn. Stepping back to 45 re-enables a chase,
  but a player hovering at 49 to 51 (tutorial staging spot, ramp elevation) flips it every tick (100574 at 16:16:45
  chased, then leashed 4 s later).
- **The leash does not clear `nav_path`.** NPC 100908: leashed at 03:43:31 with `nav_path_len = 1` (waypoint
  (-286.35, 65.6, -120.4)). At 03:43:37 it is at (-288.4, 67.5, -142.4), which is exactly 35% along the straight line
  from spawn to that waypoint: x -289.5→-288.4, y 68.5→67.48, z -154.3→-142.4. The movement tick is walking the stale
  path from spawn **with the lerp and no mesh**. Then the next leash snaps it again. The client sees a glide out, a
  glide back, and repeats.
- The leash snap is a bare field write (`leash.rs:49`), so the stale chase `velocity` is kept and extrapolated by the
  client filter, per the existing memory note. The AoI relay still sends the new position every tick (per
  `console/placement.rs:42`, **M**).

Fix direction (ownership: npc-ai-spawn-advisor for leash policy, movement for the write):

- On the Fighting→Leashing transition and in `npc_ai_leash`: `nav_path.clear()`, `velocity = [0;3]`, and route through
  `update_position_preserving_facing` (or a walk-home path instead of a snap).
- Add a post-leash re-aggro suppression window (the original SGW behaviour needs checking), otherwise the 6 s loop
  continues.
- Reconsider the leash metric: NPC distance from spawn, or horizontal distance, or a per-template leash radius.

### C2. Hovering stop, then stale state (**H** for the hover, **M** for the consequence)

The fight in-range stop freezes the NPC mid-chord (A, step 6). It does not stop it pathing later: no `no start poly`
occurred, because Detour's candidate test uses poly **AABB** overlap, and near stairs the AABB is tall enough. Detour
`findNearestPoly` then returns `closestPointOnPoly`, and `find_path` starts from that projected point. So on stairs and
ramps the NPC recovers when it next moves. On a **flat** floor a hover above 0.5u plus the BV quantization slack fails
`START_EXTENTS` (`navigation/mod.rs:51`, `:448-460`). Fight then logs `no_path` and keeps the stale path. Once the path is
consumed or cleared, the NPC is permanently path-less, and returning the player does not help because the failure is at
the **start**. Latent, not seen in 7 days.

### C3. Other movement-side stuck causes (latent, **M**)

- **Partial paths are invisible.** Detour `findPath` returns `DT_SUCCESS | DT_PARTIAL_RESULT` when the target is in
  another mesh component. `dt_status_failed` only checks `DT_FAILURE` (`detour_ffi.rs:112`). We accept a path to the
  island edge as a normal `chase`, and the NPC parks at the edge of its component (a doorway gap in the rebuilt
  Cellblock mesh would do it) with `decision_outcome = chase` every tick and no warning. It recovers if the player
  re-enters the same component.
- **Off-mesh spawn or snap targets.** Spawn Y authored as a model origin, or XZ within about 1.1u of a wall (0.6 erosion
  plus 0.5 extent), fails the start poly forever. Leash snaps back onto the same bad spawn. `spawner/npcs.rs:390` logs
  `on_navmesh` at DEBUG only, and its `ground_y` has the D1 bug.
- **Straight-line fallbacks** (backup, patrol, wander, investigate, unrouted follow) can end off-mesh horizontally or
  under the floor (backup with the player above). The next `find_path` from there fails the ±0.5 start box, so the
  state is sticky.
- **`hold_no_repath` with the target near the old endpoint but unreachable in LOS**: the NPC stands at its path end. It
  recovers when the target moves more than 5u.

---

## D. Telemetry audit

### Existing

| Event | Target and level | Has | Gaps |
|---|---|---|---|
| `step` (sampled 1/10 plus the first 5 of each leg) | `movement.npc` DEBUG, `npc_movement.rs:209-228` | cur/new/wp xyz, `ground_y`, `y_offset_from_ground`, `y_source = lerp`, leg_step | **`ground_y` is from the wrong storey (D1)**. No world or navmesh_hash. No leg length or leg dy. No ai_state. |
| `waypoint_reached` | `movement.npc` DEBUG `:163-173` | wp xyz, remaining | No ground_y, no corner kind (intermediate or final) |
| `NPC AI tick` | `npc_ai.tick` DEBUG `dispatch.rs:310-356` | pos, nav, dest, target_pos, dist_to_target, has_los, dist_to_spawn | **No spawn→target distance** (the leash metric), no leash radius, no world, no `on_mesh` or ground dy for the NPC |
| `decision=leashed` | `npc_ai` INFO `fight.rs:200-209` | `dist_to_spawn` (actually spawn→**target**, a misleading name) | No npc pos, no target pos, no stale `nav_path_len`, no leash-loop count |
| leash complete | default target INFO `leash.rs:67-70` | npc_id | No from/to, no path cleared, no world |
| `path_fail` | `npc_ai.path_fail` WARN throttled 5 s, `path_failure/mod.rs:161-187`; counter `npc_path_fail_total{world,state,reason}` | from/to, dy, hash | Cannot say **which stage** failed (start poly, end poly, corridor, straighten). The fight `no_path` message wrongly says "falling back to a straight line". Partial results are never reported. |
| `NavMesh::find_path: no start/end poly` | module-path target WARN, **unthrottled**, `navigation/mod.rs:458,475` | `?start` or `?end` | No npc_id, world, or hash. `debug` for no corridor. Not correlated with the AI row. |
| spawn behaviour | `spawner.npc_behaviour` DEBUG `npcs.rs:379-400` | on_navmesh, ground_y | ground_y is the wrong storey; on_navmesh only at DEBUG |
| wire | always 0x10 | n/a | Variant is constant, so no need to log it now. Add `variant` to `aoi.update` only if a variant switch ever lands. |

### D1. Prerequisite bug

`NavMesh::get_height_at` (`crates/entity/src/navigation/mod.rs:394-434`) searches centred at `[x, 0, z]`,
`HEIGHT_EXTENTS = [2, 500, 2]`, so it picks the poly nearest **world Y = 0**. Colo proof: NPC 100131 walking at Y 68.5
reads `ground_y = 0.2`. NPC 100033 at a constant Y of 55.39 has `ground_y` flip 55.38 → 43.38 between two adjacent
samples. 397 of 855 samples show offsets that equal storey heights. Every `ground_y` or dy field built on it, including
the `.bookmark` console at `console/bookmark.rs:171`, is unreliable on Castle_CellBlock.

### Missing instrumentation, proposed

Following instrumentation-discipline (Rule 2: `event=` on debug transitions; Rule 4: `world` is the only approved
label; ids are fields) and negative-logging (WARN for a player-visible, non-self-correcting condition, throttled per NPC).

1. **`movement.npc` event=`ground_deviation`**, WARN, throttled per NPC at 5 s with `suppressed = N`. Emit in
   `npc_movement_tick` on every step and snap (not only sampled ones) when `|y - ground_y_storey_aware| > 0.3`.
   Fields: npc_id, world, navmesh_hash, ai_state, x, y, z, ground_y, dy, `y_source` (lerp, snap or clamp), leg_len,
   leg_dy, wp xyz, leg_step. Counter `npc_ground_deviation_total{world, dir=above|below}`. It requires D1 fixed. After a
   clamp lands it becomes the regression tripwire.
2. **`npc_ai` event=`path_request`**, DEBUG, one per `find_path` call from the AI. It needs `NavMesh::find_path` to
   return a `PathOutcome { status: Ok | Partial | NoStartPoly | NoEndPoly | NoCorridor | StraightFail, start_snap,
   end_snap, waypoints }` instead of `Option`. Fields: npc_id, state, from, to, start_snap_dy, end_snap_dist, status,
   n_waypoints, max_leg_dy, max_leg_len, end_to_target_dist. Promote `Partial`, `NoStartPoly` and `NoEndPoly` to the
   existing `npc_ai.path_fail` WARN (new `reason` labels: `partial`, `no_start_poly`, `no_end_poly`, `no_corridor`).
   Remove the two unthrottled module-path warns, or demote them to debug.
3. **`npc_ai` event=`npc_off_mesh`**, WARN, throttled per NPC at 30 s. In the AI tick (2 s cadence), when the space has a
   mesh and `diagnose_point(npc.position)` is invalid. Fields: npc_id, world, hash, ai_state, xyz, `gate`,
   `horizontal_dist`, `dy`, `last_move_source` (path, fallback, leash, backup). This is the direct detector for "NPC
   drifted off-mesh and will never path again".
4. **Leash**: add `npc_x, npc_y, npc_z`, `target_x, target_y, target_z`, `spawn_to_target` (rename from
   `dist_to_spawn`), `npc_to_spawn`, `leash_distance`, `stale_nav_path_len` to `decision=leashed`. In `npc_ai_leash`:
   `event=leash_snap` with from, to and `nav_path_cleared`. Add a **`leash_loop` WARN** when one NPC leashes 3 or more
   times in 60 s without the target entering range. Counter `npc_leash_total{world}`.
5. **`npc_ai.tick`**: add `spawn_to_target`, `on_mesh` (from `diagnose_point().valid`, only when a mesh is loaded), and
   `ground_dy` (storey-aware) so a single saved view answers both symptoms.
6. **Stop sites** (`fight.rs:550`, `attack_in_place`, and `follow_band`, which holds): include `ground_dy` on the
   decision row so "hovering while attacking" can be queried.
7. **Spawn**: raise `spawner.npc_behaviour` `on_navmesh = false` on a meshed world to a WARN
   (`event=spawn_off_mesh`) once per spawn_id, with `PointVerdict` gate, horizontal_dist and dy.

SigNoz queries for today's data: `body = 'NPC AI tick' AND decision_outcome = 'leashed'` grouped by npc_name;
`attack_in_place` rows' `y` against the floor at the same XZ; `movement.npc` step rows (ignore `ground_y` until D1 is
fixed).

---

## Out of scope / handoffs

- Leash policy (radius, metric, re-aggro suppression, walk-home) goes to **npc-ai-spawn-advisor**. Auto-aggro seeding
  from AoI witnesses with no range goes to **ai-aggro-audit** / combat.
- AoI relay of the leash snap and any velocity reset go to **aoi-witness-broadcast**.
- Doc fixes needed, not made (read-only task): `docs/drafts/spec/position-updates.md:124-125` (OnChunk and OnGround
  keep the current height, and the sentinel is -13000, not FLT_MAX). Agent memory
  `npc-broadcast-facing-and-grounding.md` defect 2 ("0x18 fixes floating") is wrong. `npc-leg-boundary-y-sawtooth.md`
  and `castle-has-no-navmesh.md` cite `get_navmesh_height` as a working ground truth, but it returns the wrong storey
  on multi-level meshes.

---

## Addendum (owner follow-up): "frozen but still attacking" and "runs in place" (colo, GM account)

### What the client is told when an NPC stops

Two independent channels drive what the client shows:

> **Corrected 2026-09-25 (NA10):** item 1 is wrong. No server-to-client movement-type message exists, and witness method 1 is `onSequence`. The `setMovementType` broadcast reached clients as a truncated `onSequence` and has been removed. Only item 2, velocity, drives what the client shows. See [npc-movement-pathfinding.md §11](../../../reverse-engineering/findings/npc-movement-pathfinding.md#11-correction-2026-09-25-the-client-has-no-movement-type-receiver).

1. **`setMovementType`** (SGWBeing method 1) selects the mob animation set. `broadcast_movement_type`
   (`abilities/messaging.rs:201-257`) is compare-and-set, so a value goes on the wire **only when it changes**.
   `None` clears the cache and sends **nothing** (`:243-256`). `EMobMovementType` has no idle or stop value
   (`cell_entity/mod.rs:197-205`). **No code path ever tells the client that the NPC stopped moving.**
2. **UPDATE_AVATAR 0x10.** Every AoI tick relays each entity's stored `position` **and `velocity`**
   (`space_manager/aoi.rs:229-240`). The client decodes the packed velocity (`FUN_00ddb830` → `FUN_00de1850`) and
   passes it into the filter/actor input (`FUN_00e68a30`). `velocity` is written only by `write_position` (the
   movement tick), and zeroed only on death, despawn and respawn (`combat/state.rs:121`, `lifecycle/mod.rs:144`,
   `npc_respawn/mod.rs:227`). **No stop site zeroes it.**

Colo, 7 days, `npc_ai.tick` rows where `nav_path_len = 0`:

| ai_state / outcome | last_movement_type | rows |
|---|---|---|
| Fighting / attack_in_place | CombatAdvance | 578 |
| Leashing / leashed | Leash | 351 |
| Fighting / stationary_holds | CombatAdvance | 113 |
| Fighting / no_ability, repath_degenerate, no_path | CombatAdvance | 22 |
| Idle (after leash) | None (cache cleared, **no wire**) | 1081 |

Every path-less fighting NPC is still flagged CombatAdvance on the client. Every NPC that leashed home is still
flagged **Leash** on the client, because the post-leash `None` sent nothing.

### Shape 1: "runs in place" (H in code and telemetry; M on the client-side rendering)

- **Leash-home case (dominant).** The leash loop sends CombatAdvance, then Leash. The NPC snaps to spawn and goes
  Idle, and `None` puts nothing on the wire. The client keeps playing the Leash (trot) set at a position that never
  changes. The stale-path walk after the snap (see C1) is interrupted by the next leash, so `velocity` is also left at
  walking speed and relayed every 100 ms. This loops every 6 s while the player stays more than 50 from spawn
  (NPC 100630: 120 cycles). The result looks like a run cycle with no translation.
- **Fighting with no route.** `no_path`, `repath_degenerate`, `stationary_holds` and `no_ability` all keep
  CombatAdvance, and there is no path to translate along. For a GM this is more likely: containment is warn-only for
  GMs (`navmesh_mode.rs` doc), so a GM can stand or fly where `DEST_EXTENTS` ±3 finds no poly. The row at 03:45:33 on
  09-20 has the target at Y 97.0, 28.5u above the guard, and logged `no end poly`. `find_path` then returns `None`
  every tick and the NPC holds the advance animation. Only 2 such rows this week, so this is secondary.
- **Stale velocity treadmill.** The chase path ends at the player's position, so the in-range stop always happens
  **mid-leg**. `velocity` is left at the chase velocity (`npc_movement.rs:190-194`, never reset by `fight.rs:550`). The
  client gets a non-zero velocity at a fixed position ten times a second. If its locomotion blend or filter
  extrapolation uses that velocity, the NPC animates forward and is pulled back every update. I verified the decode
  path, but not whether SGW's AnimTree reads it (**M/L**).

Fix direction, all server-side:

- At every stop site, zero `velocity` and route through `write_position`: `fight.rs:550` (in range), `no_path` with an
  empty path, leash transition and handler, and the final waypoint (already zeroed at `npc_movement.rs:139-141`).
- Send a real movement type on stop. Either re-send `CombatAdvance` only while `nav_path` is non-empty, or find
  the value the 2009 server used for "standing in combat" (probably Cover = 0 or another enum value). **Needs RE of
  the `FUN_00deb660` FSM before choosing**, which is npc-ai-spawn-advisor's call. Stop sending `None` as "stop",
  because it is not a wire message.
- After a leash, send a movement type the client renders as idle, for the same reason.

### Shape 2: "frozen but still attacking" (H)

Why it does not move: in the fight branch where the target is in range and LoS is not Blocked, `nav_path.clear()` runs
**every AI tick** (`fight.rs:550`). The NPC resumes moving only when the target leaves range or LoS reads `Blocked`.
The freeze point is wherever the mid-leg lerp left it, often about 1u in the air (see A6).

Which checks let it keep shooting:

1. **Fight gate, range** (`fight.rs:240-262`): `in_range = dist_to_target <= max_range`, a **3D** distance, where
   `max_range` is the ability's value or `NPC_ATTACK_RANGE = 30` (`aggro.rs:20`). This is not an issue by itself.
2. **Fight gate, LoS** (`fight.rs:263`): `has_line_of_sight` means `is_clear_or_unknown()`
   (`line_of_sight.rs`, `spatial.rs:21-23`). It is **Unknown, which counts as clear**, when either endpoint cannot be
   projected onto the mesh: the NPC with ±0.5 then ±3, the target with ±3. A GM flying, noclipping, or standing on
   geometry the mesh does not cover gets Unknown and is always "visible".
3. **The LoS that does run is a Detour navmesh raycast** along the walkable surface. It only reports `Blocked` where
   the ray crosses a navmesh boundary edge. It cannot see ceilings or floors between storeys, ramps overhead, railings,
   glass, or props that do not cut the mesh. A guard below a ramp therefore has "LoS" to a player standing on the ramp
   above it. That is the ramp scenario from symptom 3, and it keeps the guard frozen and firing.
4. **Ability pre-consume guard** (`use_ability/handle.rs:238-256`): a **range-only** 3D check (ability `max_range`,
   default 30). There is **no LoS check** at fire time.
5. Stationary mobs (`fight.rs:370-400`) now face the target but never fire out of range. That is fine.

So "frozen and attacking" is the correct in-range branch, combined with (a) a freeze point in the air, (b) LoS that
fails open for off-mesh targets such as a GM, and (c) a navmesh raycast that cannot see through-floor or overhead
occlusion. The combat side (whether fire-time LoS should exist, which occlusion source to use) belongs to
**combat-systems-advisor**. I recommend it at least gate on `LineOfSight::Blocked | Unknown` for targets that are
**not** GMs-in-noclip, or use a geometry raycast.

### Instrumentation for these two shapes

- `npc_ai.tick`: add `vx, vy, vz` (the stored velocity actually relayed) and `wire_movement_type`. That is the last
  value **sent**, as opposed to `last_movement_type`, which is the cache and becomes None on clear with no wire. Also
  add `los` (clear, blocked or unknown) instead of the boolean `has_los`.
- `movement.movement_type` "cleared" (`messaging.rs:249`): raise it to INFO while it still means "the client is stuck on
  the prior animation", with the fields `prior_kind` and `npc_moving` (from `nav_path_len`).
- A new `npc_ai` WARN, `event=animating_without_path`, throttled at 30 s per NPC: last wire movement type is not
  None, `nav_path` is empty, and `|velocity| > 0` for 2 or more AI ticks. This is the direct "runs in place" detector.
- `wire.out.avatar_update` (the sampled log at `base/world_entry/cell_dispatch/aoi.rs:198-219`) already carries
  vx/vy/vz, but **it returned zero rows on colo in 7 days**. Either the deploy predates it or its level is filtered.
  Verify it is exported.
