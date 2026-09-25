# NPC AI Telemetry Plan

> Type: reference and how-to. Audience: packet workers (NA00-NA03) and whoever debugs a live session.
> Updated: 2026-09-24. Companions: [audit](audit.md) section 5, [work packets](work-packets.md), [instrumentation discipline](../../architecture/instrumentation-discipline.md), [negative-logging convention](../../architecture/negative-logging-convention.md), [movement validation](../../architecture/movement-validation.md) (and `movement-telemetry.md` once PR #726 lands), [observability ADR](../../architecture/observability.md).

## Goal

After the NA0x telemetry packets, the owner can play one colo session and answer each of these from SigNoz alone, with no debugger and no code reading:

- Why did this NPC not aggro? Was it ticked, what did it see, and why was each candidate rejected?
- Why is this NPC standing still? Is its velocity stale, is its path empty, is it off the mesh, did its path fail and at which stage, or is it parked Idle?
- Is this NPC in the air? How far is it from the floor under it right now?
- Did this NPC consider cover? Which node did it pick, and if none, why not?
- What did the client actually receive for this NPC (position and velocity; no movement type exists on the wire, see NA10)?

The telemetry packets ship **before** the behaviour packets, so every behaviour fix can be compared before and after against live data.

## Conventions every new event follows

- Use a dotted `target:` under `npc_ai.*`, `movement.npc`, `cover.*` or `wire.out.*`. **Add each new target to `OTEL_FILTER` in `crates/server/src/logging.rs` at the level it emits, and extend `otel_filter_exports_the_debug_level_aoi_seams` to pin it.** A target left out of the filter inherits `info` and its DEBUG rows never reach SigNoz. That is gap T1, and it has already happened once with `aoi.create_emit`.
- Every NPC row carries `npc_id, tag, template_id, world, space_id`. `world` is the only approved metric label; ids stay fields (instrumentation discipline, rule 4).
- Rows for a player-visible stuck or wrong state are WARN, throttled per `(npc_id, kind)` with a `suppressed = N` count. An **unthrottled** counter sits beside each one, as the negative-logging convention requires.
- Transitions are DEBUG with an `event=` field. Steady-state samples are DEBUG and rate-limited.
- Distances are named by their endpoints: `npc_to_spawn`, `target_to_spawn`, `npc_to_target`. Never use a bare `dist_to_spawn` (T7).

## 1. Filter and identity fixes (NA00)

| Change | Where | Fixes |
|---|---|---|
| Add `wire.out.avatar_update=debug`, `movement.navmesh=debug`, `cover=debug`, `spawner=debug`, plus every new target below, to `OTEL_FILTER` | `crates/server/src/logging.rs:26` | T1 |
| Resource attributes `host.name`, `cimmeria.deploy_env` (from `CIMMERIA_DEPLOY_ENV`, already set on the colo) and `service.version` (git SHA baked in at build time) | OTLP resource builder in `crates/server/src/otel*.rs`; env-var table in `crates/server/src/main.rs` | T2 |
| Log the navmesh fingerprint in the first NPC-AI row per space (already in `movement.navmesh navmesh_loaded`), so a session joins to its mesh | existing | |

Check the SigNoz volume budget before raising `wire.out.avatar_update`: it samples 1 in 100 sends today, which is fine at colo scale.

## 2. New and changed events

### 2.1 State and aggro (NA00, NA02)

| Target / event | Level | When | Fields beyond the common set |
|---|---|---|---|
| `npc_ai.transition` `event=state_change` | DEBUG | Every AI state change, emitted from **one** `set_ai_state(npc, to, reason)` helper that replaces every raw `ai_state =` write | `from, to, reason` (`threat_preempt, auto_aggro, assist, threat_empty, target_dead, target_gone, leash_out, leash_arrived, leash_snap_fallback, respawn, content`), `npc_to_spawn, threat_count, nav_path_len`. Counter `npc_ai_transitions_total{world,from,to,reason}` |
| `npc_ai.aggro` `event=acquired` | INFO | Every entry into Fighting | `cause` (`proximity, damage, assist, content_threat`), `player_id, account_id, npc_to_target, dy, has_los, effective_aggression` |
| `npc_ai.aggro_scan` `event=candidate_rejected` | DEBUG, per (npc, player) at most once per 10 s | Idle scan rejects a witness | `reason` (`same_faction, not_hostile, dead, gm_ignored, out_of_radius, out_of_vertical_band, no_los, post_reset_suppressed`), `player_id, npc_to_target, dy, aggro_radius` |
| `npc_ai.aggro_scan` `event=no_candidates` | DEBUG, sampled once per NPC per 30 s | Scan found nobody | `witness_count` |
| `npc_ai.idle` gauge `npc_ai_idle_unticked{world}` | metric | Each AI tick | count of Idle NPCs not admitted to the tick |
| `npc_ai.idle_parked` | INFO | Any change to Idle with `npc_to_spawn > 2.0` where the NPC will not be ticked | `reason, npc_to_spawn, x, y, z`. Counter `npc_idle_parked_total{world,reason}`. After NA12 this should read zero. |
| `npc_ai.leash` `event=enter` | INFO | Fighting to Leashing | `npc_to_spawn, target_to_spawn, leash_distance, npc_xyz, target_xyz, nav_path_len` |
| `npc_ai.leash` `event=arrived` / `event=snap_fallback` | INFO | Leash completes | `from_xyz, snap_dist` (snap only), `walk_secs, path_ok, spawn_on_mesh` |
| `npc_ai.leash` `event=loop` | WARN, 60 s per NPC | 3 or more leashes in 60 s without the target entering range | `leash_count, target_id` |
| `npc_ai.leash` `event=damage_ignored` | DEBUG | Damage while evading | `attacker_id, amount` |
| `threat` `event=cleared_without_exit` | WARN, per player-mob pair | NPC clears its threat while a player still lists it in `threatened_mobs` | `player_id, mob_id, reason`. Should read zero after NA12. |
| `content` `event=set_aggression_tag_miss` | WARN | `set_aggression` matches no entity | `tag, chain_id` |

### 2.2 Movement and grounding (NA01, NA02)

| Target / event | Level | When | Fields |
|---|---|---|---|
| `movement.npc` `event=step` (existing) | DEBUG sampled | Fix `ground_y` to use the storey-aware query from NA01 | add `y_source` (`lerp, clamp, clamp_miss`), `leg_len, leg_dy, ai_state` |
| `movement.npc` `event=ground_deviation` | WARN, 5 s per NPC | Any step or snap with `abs(y - ground_y) > 0.3` on a meshed world | `ground_y, dy, y_source, leg_len, leg_dy, wp_xyz`. Counter `npc_ground_deviation_total{world,dir}`. It becomes the regression tripwire once NA11 lands. |
| `npc_ai.tick` (existing) | DEBUG | Add `vx, vy, vz` and three-state `los` (`clear, blocked, unknown`) in place of `has_los`. **As built (NA02):** no `movement_type_sent` — see the next row | |
| ~~`movement.npc` `event=animating_without_path`~~ | — | **Folded into `stale_velocity` (NA02).** NA10's Ghidra pass showed there is no server-to-client movement-type message: `broadcast_movement_type` was sending a truncated `onSequence`, and the client animates NPC movement from velocity alone. Re-based on velocity this event is "non-zero velocity, empty path, not moving", which is `stale_velocity` with `path_state = empty`. Query `event = 'stale_velocity' AND path_state = 'empty'` | |
| `movement.npc` `event=stale_velocity` | WARN, 10 s per NPC | Velocity non-zero while the position has not changed for 3 movement ticks (300 ms) | `vx, vy, vz, speed, still_ticks, path_state` (`empty`, `stalled`), `nav_path_len, ai_state, movement_type` (the dedup cache, for NA10's before/after). This is the running-in-place detector (S1). **As built:** the counter counts episodes, not ticks. |
| `npc_ai` `event=npc_off_mesh` | WARN, 30 s per NPC | AI tick on a meshed world where `diagnose_point(npc.position)` is invalid | `gate, horizontal_dist, dy, last_move_source` (`path, fallback, leash, backup, content, spawn`) |
| `npc_ai.path` `event=request` | DEBUG | Every AI `find_path` | `state, from, to, target_is_gm, status` (`ok, partial, no_start_poly, no_end_poly, no_corridor, straighten_failed`), `start_snap_dy, end_snap_dist, n_waypoints, max_leg_dy, end_to_target_dist`. **As built:** non-`ok` statuses are always logged; `ok` is sampled once per NPC per 10 s (`suppressed`); the counter is unthrottled |
| `npc_ai.path_fail` (existing) | WARN | Add the reasons `partial, no_start_poly, no_end_poly, no_corridor`; fix the fight message that claims a straight-line fallback | **As built:** `partial` rows have their own throttle window and their own counter, `npc_path_partial_total{world,state}`; `npc_path_fail_total` counts routes with no usable path only |
| `npc_ai` `event=stuck` | WARN, 15 s per NPC | Fighting and chasing (a path exists and the target is out of range) while `npc_to_target` has not shrunk for 3 AI ticks | `npc_to_target_history, nav_path_len, los, next_wp, decision_outcome`. **As built:** "chasing" is the fight's out-of-range branch (`chase, hold_no_repath, repath_degenerate, no_path`) with a non-empty path, except `no_path`, which counts with or without one; "shrunk" means by at least 0.5 |
| `spawner.npc_behaviour` `event=spawn_off_mesh` | WARN, once per spawn_id | Spawn on a meshed world fails the find_path start box (±0.5), not just `is_point_valid` | `gate, horizontal_dist, dy, snapped_y` |
| Remove or demote the two unthrottled `NavMesh::find_path: no start/end poly` warnings | | They are superseded by `npc_ai.path` | **Done (NA02): removed.** `NavMesh::find_path` now returns a typed `PathOutcome` and logs nothing itself |

### 2.3 What the client received (NA02)

| Target / event | Level | Fields |
|---|---|---|
| `wire.out.avatar_update` (existing, now exported) | DEBUG, 1 in 100 | add `npc_moved_since_last` (bool; absent for players). **As built (NA02):** no `movement_type` field — the client animates from velocity, and no movement-type message exists |
| ~~`wire.out.movement_type`~~ | — | **Dropped (NA02).** There is no server-to-client movement-type message (NA10 Ghidra evidence); NA10 suppresses the truncated `onSequence` `broadcast_movement_type` was sending; movement-type changes are logged server-side as `movement.movement_type outcome=suppressed\|cleared` |
| `wire.out.forced_position` | DEBUG | Every forced position sent. **As built (NA02):** logged at the one send site (`teleport.rs`); every row today is a player snap, because no NPC snap — the leash included — is sent as a forced position |

### 2.4 Line of sight (NA02)

`npc_ai.los` `event=blocked`, DEBUG and sampled once per (npc, target) per 5 s: `from_xyz` and `to_xyz` including the eye heights used, `result` (`clear, blocked, unknown_off_mesh`), `hit_xyz`. This closes T9 and gives S11 its evidence.

**As built (NA02):** emitted for every result that is not `clear` (so `result` is `blocked` or `unknown_off_mesh`), with `ray_from` / `ray_to` (the projected points the ray was cast between) and `eye_height_used = 0.0` — the navmesh ray adds no eye height at all. It replaces the unsampled `movement.navmesh reason=los_unknown_off_mesh` row.

### 2.5 Cover (NA02; extended by NA22)

| Target / event | Level | When | Fields |
|---|---|---|---|
| `npc_ai` `decision_outcome=no_cover` | DEBUG | Replaces the silent `NoCover => {}` arm | **As built:** sampled once per NPC per 10 s; `use_cover_false` and `stationary` are not logged (the spawn row records both). `reason` (`no_candidate_in_radius, reserve_lost, in_range_no_better_slot`, plus `no_world` (the space has no `resources.worlds` id, so no cover index) and the defensive `index_miss`), `candidates_scanned, reserved_skipped, search_radius, cover_nodes_loaded` |
| `cover.selection` `event=picked` / `event=rejected` | DEBUG, sampled | Scoring | `chunk_id, node_id, score` and score components; top 3 rejected with their reasons. **As built:** components are `move_dist, threat_dist` (the two inputs that vary); rejected `reason` is `reserved` or `lower_score`; ≤ 1 set of rows / 10 s per NPC |
| `cover.coverage` `event=space_summary` | INFO once the space has its NPCs; WARN when unusable | Per space: the cover nodes of its world and how many stand on its navmesh | **As built (NA02, after NA21):** `world_id, nodes_in_world, nodes_on_mesh, sets_in_world, cover_npcs`. On the mesh means `get_height_near` around the node's own Y finds a floor within 1.0. WARN `reason = no_usable_cover` when a meshed space has cover-seeking NPCs (`use_cover`, not stationary) and no usable node. NA21 replaced the prefab-local seed with extracted world-space nodes, so the planned "bounds + on mesh" test and NA02's interim prefab-local heuristic are gone; Castle_CellBlock now reads 236 nodes, 211 on the mesh, 58 sets, INFO |
| `cover.state` `event=enter` / `event=leave` | DEBUG | NPC reaches or leaves a reserved slot | `chunk_id, node_id, pose, reason` (`arrived, flanked, target_lost, leash, death`) |

## 3. Live-session runbook

Run these in SigNoz (logs explorer, `service.name = 'cimmeria-server'`, and `cimmeria.deploy_env = 'colo'` once NA00 lands) after a play session. Start from a `.bug <note>` bookmark: its `playtest.bookmark.entity` rows give the NPC ids near the player at that moment.

| Question | Query |
|---|---|
| Which NPCs aggroed, and how? | `target = 'npc_ai.aggro'`, group by `cause, tag` |
| Why did this guard ignore me? | `target = 'npc_ai.aggro_scan' AND npc_id = N`. No rows at all means it was never ticked: check `npc_ai_idle_unticked` |
| Timeline for one NPC | `npc_id = N AND target IN ('npc_ai.transition','npc_ai.leash','npc_ai.aggro','npc_ai.path','movement.npc')`, ordered by time |
| Who is running in place? | `target = 'movement.npc' AND event = 'stale_velocity'` |
| Who is floating? | `event = 'ground_deviation'`, group by `tag`, sort by `abs(dy)` |
| Who is stuck? | `event IN ('stuck','npc_off_mesh','idle_parked')` and `target = 'npc_ai.leash' AND event = 'loop'` |
| Does this map have usable cover? | `target = 'cover.coverage'` at startup |
| Why no cover in this fight? | `decision_outcome = 'no_cover' AND npc_id = N`, group by `reason` |
| What did the client see? | `target = 'wire.out.avatar_update' AND entity_id = N` (the avatar sample keys on `entity_id`, not `npc_id`); `npc_moved_since_last = false` beside a non-zero `vx`/`vz` is running in place |

The helper scripts in [evidence/signoz/](evidence/signoz/) condense SigNoz JSON pulls into one line per event. SigNoz MCP results over about 25k tokens land in a tool-results file. Run the scripts over that file; do not read it raw.

NA03 turns the table above into saved views and one "NPC AI health" dashboard with these panels:

- aggro by cause;
- transitions per reason;
- `stale_velocity`, `ground_deviation`, `stuck`, `idle_parked` and leash-loop counts per world;
- path status mix;
- cover coverage per space;
- `no_cover` reasons.
