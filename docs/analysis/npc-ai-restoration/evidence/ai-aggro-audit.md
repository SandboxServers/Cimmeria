> Evidence pass, 2026-09-24. Read-only research against `main` @ 192d4216 and 7 days of colo SigNoz data (2026-09-17 to 2026-09-24). Kept verbatim as the evidence record; the ledger in [../audit.md](../audit.md) supersedes it where they differ.

# NPC aggro + leash audit: Castle_CellBlock (world 12)

Audited against the working tree at 192d4216. `git diff HEAD origin/main -- crates db` is empty, so every file:line
below is also correct for origin/main (b0b594e9). This was read-only work: no builds and no DB queries.
Confidence tags: **[C]** confirmed in code, **[H]** high (inferred from code, not observed live), **[M]** medium, **[L]** low.

---

## A. The AI state machine as it exists today

### Cadence and admission

- `npc_ai_tick` (`crates/services/src/cell/service/npc_ai/dispatch.rs:238`) runs every 20th AoI tick, about every 2 s.
  `npc_ai_retry_sweep` (`dispatch.rs:384`) runs every 100 ms, but only for Fighting NPCs whose launch failed.
- `npc_movement_tick` (`ticks/npc_movement.rs:25`, file offset ~64) runs every 100 ms. It moves **every** class-0x04 NPC
  that has a non-empty `nav_path`, whatever its `ai_state`. The AI state does not gate movement. **[C]**
- Admission filter, `dispatch.rs:263-278`. Fighting, Leashing, Patrol, Wander, Investigating, Follow, Despawning, Submit
  and Error are always ticked. **Idle is ticked only when `aggression > 0`, the NPC has a patrol path, or
  `wander_radius > 0`.** Dead and Spawning are never ticked. **[C]**
- An Idle NPC that fails that filter gets **no handler call and no `npc_ai.tick` row**. It is invisible to the AI and to
  telemetry. **[C]**

### Transitions (from → to, trigger, file:line, wire effect)

| From → To | Trigger | Where | Side effects |
|---|---|---|---|
| Idle/Patrol/Wander/Investigating/Follow → Fighting | `generate_threat` (damage, content `generate_threat`, auto-aggro seed) | `combat/threat/aggro.rs:106-126` | clears `nav_path`, adds threat, `enter_player_combat` (sets player's `threatened_mobs` / BSF_InCombat). INFO log "NPC aggro: preempt -> Fighting" (untargeted) |
| Leashing → (nothing) | damage while Leashing | `aggro.rs:106-112` | **Leashing is not preemptable**; threat accrues and `leash.rs:58` then throws it away. **[C]** |
| Idle (aggression>0) → Fighting | a player witness of opposing faction is present | `fight.rs:25-79` → `generate_threat(…,1.0)` | no LoS, no distance gate beyond AoI radius 100 (`construction.rs:31`), no GM/invisible exclusion |
| Fighting → Idle | threat list empty | `fight.rs:126-146` | `threat_list.clear()`, cover release, movement-type cache cleared. **Does NOT clear `nav_path`, does NOT go home, does NOT remove the NPC from any player's `threatened_mobs`, records no `decision_outcome`**. The debug log is untargeted |
| Fighting (drop target) | top target HEALTH ≤ 0 | `fight.rs:150-167` | removes the target from threat, returns; the next tick then lands in the row above (Idle **in place**) |
| Fighting (drop target) | top target entity gone | `fight.rs:170-176` | silent removal, no log |
| Fighting → Leashing | **`spawn.distance_to(target_pos) > 50`** | `fight.rs:179-216` | `threat_list.clear()`, `note_outcome("leashed")` + INFO `npc_ai` event=decision, broadcast movement type Leash. **`nav_path` NOT cleared** |
| Leashing → Idle | next natural tick (~2 s later) | `leash.rs:12-90` | raw `npc.position = spawn_pos` (`:48-50`, skipped when `follow_target_id` is set), heal to full, `ai_state = Idle`, clear threat and cooldowns, send methods 20 and 19. No `nav_path` clear, no grid update, no `spawn_dir`, no velocity reset, no player-side threat cleanup. INFO untargeted "leash complete" |
| Dead → Idle | respawn timer | `ticks/npc_respawn/mod.rs:106, 221-228` | the correct reset (clears nav_path and threat, restores position and facing). `aggression` is **kept** |

There is no threat decay, no out-of-AoI disengage, no return-to-spawn walk, and no evade/immune state (grep for
evade/immune in `npc_ai/` finds nothing). **[C]**

### Verdicts on the playtest-appendix claims

| Claim | Verdict | Evidence |
|---|---|---|
| `aggression` is absent from the seed, so Idle NPCs are never ticked and proximity/assist aggro never fires | **CONFIRMED** | `db/resources/Worlds/Tables/spawnlist.sql` has no aggression column, and `Entities/Tables/entity_templates.sql` has none either. `cell_entity/construction.rs:89` sets `aggression: 0`. `dispatch.rs:277` is the filter. The only writers are the `set_aggression` action (`content/executor/world/mod.rs:84-97`), `spawn_entity`'s optional override (`executor/spawn/mod.rs:142`) and the console. No assist/social code exists anywhere: `generate_threat` touches only the NPC that was hit. Nuance: Idle NPCs **are** ticked when they have a patrol path or a wander radius, but no Cellblock row has either (no `wander_radius`/`patrol_path_id` in the seed) |
| The leash predicate is spawn→target, not spawn→NPC | **CONFIRMED** | `fight.rs:180-182`: `spawn.distance_to(&target_pos) > combat::LEASH_DISTANCE` (50.0, `threat/aggro.rs:11`, global) |
| The leash handler raw-teleports, restores no `spawn_dir`, and sends no EntityMoved | **Partly CONFIRMED, partly WRONG** | Raw write CONFIRMED (`leash.rs:48-50`, bypassing `write_position` so the **spatial grid is not updated**: `space_manager/entities.rs:496-528` is the only grid-updating path). No `spawn_dir` restore CONFIRMED. **"No EntityMoved" is WRONG:** `compute_player_aoi` pushes an `EntityMoved` for every entity that stays in a witness's AoI **every AoI tick**, straight from `other.position` (`space_manager/aoi.rs:228-241`), so the snap does reach clients within ~100 ms, as an unreliable UPDATE_AVATAR carrying the stale chase velocity rather than a forced position (0x31). The worse defect the appendix missed is that **`nav_path` is never cleared** (not at the fight.rs:183-185 transition, not in leash.rs), so after the snap the movement tick walks the NPC from spawn back along whatever chase waypoints remain |
| The fight.rs chase repath ignores `Some(path)` with len ≤ 1 | **FIXED-SINCE (instrumentation only)** | `fight.rs:448-468` now records `repath_degenerate` and emits the shared `npc_ai.path_fail` WARN. Behavior is unchanged: the stale path is still left installed |
| (Appendix §C1) fight.rs outcomes never reach the counter | **FIXED-SINCE** | `note_outcome` (`npc_ai/mod.rs:142`) is called before each fight.rs log, and `log_ai_tick` (`dispatch.rs:454-500`) now gives a per-NPC row |

---

## B. Why the non-first Cellblock mobs do not proximity-aggro

### Root cause [C]

Every Cellblock hostile spawns with `aggression = 0`. Only two are ever given aggression, both by content chains:

- **Chain 1008** (`db/resources/Content/Seed/castle_cellblock_chains.sql:473-482`): `enter_region`
  'Castle_CellBlock.Region8' (once) → `set_aggression 1` + `generate_threat 1000` on **`ArmYourself_NIDGuard`**. This
  is the "first mob outside the stasis room" the owner sees aggro. It is **content-scripted, not proximity AI**: the
  threat seed puts it straight into Fighting, and after that its `aggression = 1` also enables the idle auto-aggro scan.
- **Chain 1032** (`castle_cellblock_chains.sql:858-868`): interacting with the Ambernol vial → `set_aggression 1` +
  `generate_threat 1000` on `ArmYourself_PrisonerRetrievalUnit` (spawn 10, `is_stationary = true`).

The python reference only did those two as well (`deprecated/python/cell/spaces/Castle_CellBlock.py:85-134` and
`cell/missions/Castle_CellBlock/FindAmbernol.py:99-103`). The Hallway/MessHall controllers only subscribe to
`entity.dead.tag` (`Hallway0xController.py`, `MessHall.py:54-58`).

Every other hostile never enters `npc_ai_tick`. The first time it acts is when a player damages it
(`generate_threat` → Fighting).

### Cellblock hostile spawns (world 12, `db/resources/Worlds/Seed/spawnlist.sql`)

Common template fields. Template 24 "NID Guard": class `mob`, faction 10, level 1, ability_set 3 (SMG), `move_speed`
NULL → 0.6 u/tick = 6 u/s (`spawner/npcs.rs:148`), `respawn_secs` NULL (one-shot), no wander, no patrol, no aggression
source. Template 15 "Cellblock Guard" is the same with ability_set 1 (pistol). Template 4 "Prisoner retrieval unit" is
faction 10 with ability_set 2.

| spawn_id | tag | template | x, y, z | stationary | aggression source |
|---|---|---|---|---|---|
| 20 | ArmYourself_NIDGuard | 15 | -289.46, 68.54, -154.28 | no | chain 1008 (Region8 enter) |
| 10 | ArmYourself_PrisonerRetrievalUnit | 4 | -220.26, 66.74, -121.38 | **yes** | chain 1032 (vial interact) |
| 29 | MessHall_Guard1 | 24 | -96.25, 34.59, -91.59 | no | **none** |
| 28 | MessHall_Guard2 | 24 | -95.89, 34.59, -98.81 | no | **none** |
| 30 | Hallway01_Guard | 24 | -128.85, 39.55, -73.53 | no | **none** |
| 82 | Hallway02_Guard | 24 | -113.49, 39.55, -63.04 | no | **none** |
| 31 | Hallway03_Guard | 24 | -98.58, 39.55, -77.09 | no | **none** |
| 86 | Hallway04_Guard | 24 | -61.35, 34.59, -69.03 | no | **none** |
| 32 | Hallway05_Guard1 | 24 | -100.16, 24.67, -43.90 | no | **none** |
| 33 | Hallway05_Guard2 | 24 | -101.50, 24.67, -51.30 | no | **none** |
| 25 | Barracks_Guard1 | 24 | -118.41, 24.67, -118.35 | no | **none** |
| 26 | Barracks_Guard2 | 24 | -131.48, 24.67, -116.97 | no | **none** |
| 36 | Barracks_Guard3 | 24 | -136.38, 24.67, -135.67 | no | **none** |
| 27 | Cellblock_ArmoryGuard1 | 24 | -49.69, 24.67, -127.11 | no | **none** |

Non-hostile mob-class rows: Prisoner_329 (template 17, faction 3). `being`/`spawnable` rows (Col Marsh 10, Frost
corpse 14, GuardBody 21, props) are never AI-ticked (`all_npc_entity_ids` admits class 0x04 only).

### Other gates checked (not the cause today, but relevant to any fix)

- **Faction [C].** Players stay at faction 0 (`construction.rs`); the scan rejects only `p.faction == npc_faction`
  (`fight.rs:46`), so 0 ≠ 10 passes.
- **Witness requirement [C].** Candidates are `get_witnesses_of(npc)` (`space_manager/queries.rs:387-406`): players
  whose own AoI set contains the NPC, radius 100 (`construction.rs:31`). Players held out by `is_introducible()`
  (`space_manager/aoi.rs:~90`, the first-login hold) are not "witnesses of" anything until introduced. That only
  affects the *first* seconds.
- **No LoS and no aggro radius [C].** Once aggression > 0, any player in the 100-unit AoI qualifies, through walls and
  across floors (Cellblock stacks floors at y≈24.7 / 34.6 / 39.6 / 46). Turning aggression on for all 12 guards without
  adding a radius + LoS gate would make the whole topside aggro at once.
- **No GM, dead-ghost or cinematic exclusion [C].** Only dead players are skipped (`fight.rs:50`).
- **LoS fails OPEN [C].** `has_line_of_sight` → `is_clear_or_unknown` (`space_manager/spatial.rs:21-23`). An off-mesh
  endpoint is `Unknown` and treated as clear, so LoS cannot fail closed and block aggro.
- **Auto-aggro × leash loop [H], NID guard only.** With aggression 1, a player standing more than 50 from the guard's
  spawn but inside AoI 100 produces a cycle: Idle → seed → Fighting (tick N) → Leashing (tick N+1) → snap and heal →
  Idle (tick N+2) → seed again… The guard never approaches, heals to full every ~6 s, and the player's
  `threatened_mobs` never drains (see C).

### Legacy intent (see E)

In the python fork, `aggressionOverride` is an **`EMobAggressionLevel`** (1 HOSTILE … 5 DEFAULT,
`Atrea/enums.py:276-280`). Without an override, python derives the reaction from `FACTION_REACTION_TABLE[player][mob]`
(`SGWPlayer.py:999-1009`; players are faction 3 at `SGWPlayer.py:439`; `[3][10] = 1` HOSTILE). Rust's `aggression > 0`
treats 2-5 as "aggressive" too, which is a semantic mismatch with the enum.

---

## C. "Stuck at some distance from spawn, never moves again"

Ranked. The common amplifier behind #1 and #2 is the admission filter: **once a template-24 guard drops to Idle it is
never ticked again until someone damages it**, so from the player's side it looks frozen wherever it stopped.

1. **[H, top] Fight ends in place, then passive Idle.** When the target dies (player death), disconnects, or the
   threat list otherwise empties, `fight.rs:126-146` / `150-176` set `ai_state = Idle` **where the NPC stands**. There
   is no return home, and `nav_path` is not cleared, so it finishes any in-flight leg and parks. With aggression 0 it is
   then excluded by `dispatch.rs:277` and does nothing even when the player walks right back up to it. Only
   damage or a respawn moves it again. Matches "stuck ... even if I move back". On a player death/respawn cycle this is
   the default outcome for every guard.
2. **[H] Leash on player distance, plus the stale path after the snap.** The NPC chases only while the **player** is
   within 50 of the NPC's spawn (`fight.rs:180-182`). That produces the "moves only so far from spawn" envelope: the
   NPC can get at most ~50 units out (it stops at its 30-unit ability range). Once the player steps past 50 from spawn:
   Fighting→Leashing (nav_path kept, and it keeps walking for up to 2 s) → `leash.rs:48` teleports it home (relayed
   by the next EntityMoved) → the movement tick then walks it **back out along the leftover chase waypoints**, because
   nothing cleared `nav_path` → it parks as passive Idle (see #1). Visually: it gives up, pops home, walks partway back,
   and freezes. Also: the grid entry is stale after the raw write (`entities.rs:523-527` never runs), velocity is stale,
   and `spawn_dir` is not restored.
3. **[M] NPC sits outside Detour's 0.5-unit start box → `find_path` is None forever.** `NavMesh::find_path` looks up
   the start poly with `START_EXTENTS = [0.5,0.5,0.5]` (`crates/entity/src/navigation/mod.rs:51, 445-460`), while
   `is_point_valid` (which the spawner's `on_navmesh` field reports) tolerates 3.0 horizontally and 4.0 above
   (`mod.rs:53, 67, 247`). Positions that can fall outside the 0.5 box:
   (a) the seeded spawn Y, which is never snapped to the mesh (no height snap in `spawner/` or `space_manager/spawn.rs`),
   and which the leash teleports the NPC back to;
   (b) a mid-leg stop, since `fight.rs:550-552` clears `nav_path` mid-segment and Y is lerped linearly between corners
   (`npc_movement.rs` "new_y = cur_pos.y + dy * t"), which can leave the NPC off the detail surface on stairs/ramps.
   Every repath from there returns None → `no_path` WARN (5 s throttle), and the NPC never moves again while Fighting.
   If the player is inside 30 it still shoots, because LoS is `Unknown` → clear. Only the untargeted
   `NavMesh::find_path: no start poly` WARN (`mod.rs:458`) says *which* endpoint failed, and it carries no npc_id.
4. **[M] Disconnected navmesh components, silently accepted as a partial path.** `castle_cellblock.nav` was rebuilt
   in #694 (2026-09-19) and now has **17 components** (`data/spaces/README.md`; the 50-component map in older notes is
   stale). Detour `findPath` returns `DT_SUCCESS | DT_PARTIAL_RESULT` when the goal is in another island. The wrapper
   only checks `DT_FAILURE` (`crates/entity/src/detour_ffi.rs:108-114`, `navigation/mod.rs:494`), so the NPC walks to
   the nearest reachable point and then repaths each tick into a near-zero path. It stands there logged as a plain
   `chase` (DEBUG), and there is no `path_fail` row.
5. **[L] `repath_degenerate` leaves a stale path installed** (`fight.rs:448-468`). The NPC walks to where the target
   used to be. Self-limiting.
6. **[L] Launch failure loop.** `handle_use_ability` false → WARN "attack tick produced no ability fire" + 500 ms
   retry (`fight.rs:595-627`). The NPC stands and fires nothing. Should be visible in logs if it happens.

Not causes: there is no max-chase-distance or evade/immune flag, and `is_stationary` only affects the PRU (spawn 10).

**Collateral defect [C]:** neither leash nor the fight→Idle path calls `exit_player_combat` /
`clear_dead_npc_from_all_player_threat` (their only callers are death and `lifecycle/`). The player keeps the NPC in
`threatened_mobs`, which blocks regen (`ticks/regen.rs:62`) and the OOC holster, and leaves BSF_InCombat set after
every leash or "target lost".

---

## D. Telemetry audit

### What exists and helps

| Signal | Level / where | Answers |
|---|---|---|
| `spawner.npc_behaviour` | DEBUG, once per spawn, `spawner/npcs.rs:374-406` | aggression (will be 0), faction, is_stationary, move_speed, `on_navmesh`, `ground_y` vs `y`. **Use `y - ground_y > 0.5` to find spawns outside the find_path start box** (hypothesis C3) |
| `npc_ai.tick` | DEBUG, per ticked NPC per ~2 s, `dispatch.rs:454-500` | state_before/after, outcome, pos, nav_path_len, target and dist, has_los, dist_to_spawn. **Absent for Idle aggression-0 NPCs**: that absence is itself the symptom-1 signal |
| `npc_ai` event=decision | `fight.rs` (leashed INFO; chase / hold_no_repath / attack_in_place / no_ability DEBUG; stationary_holds INFO) | fight branch taken |
| `npc_ai_decisions_total{decision_outcome}` | counter, `npc_ai/mod.rs:111-118` | rates |
| `npc_ai.path_fail` + `npc_path_fail_total{world,state,reason}` | WARN (5 s throttle), `path_failure/mod.rs:116-193` | no_path / no_mesh / degenerate |
| `movement.npc` waypoint_reached / step | DEBUG (steps sampled 1/10 plus 5 leg-head steps), `npc_movement.rs` | whether it walks after the leash snap (C2) |
| `movement.navmesh` reason=los_unknown_off_mesh | DEBUG, `spatial.rs:50-59` | NPC or player off mesh |
| `threat` event=enter_combat / exit_combat | INFO, `combat/threat/player_combat.rs:59,110` | player-side combat set (an exit that never comes = collateral defect) |
| `wire.out.avatar_update` | DEBUG sampled, `base/world_entry/cell_dispatch/aoi.rs:196-220` | what position the client got |
| untargeted: "NPC aggro: preempt -> Fighting" (aggro.rs), "aggression-driven auto-aggro" (fight.rs:60), "leash complete" (leash.rs:67), "NavMesh::find_path: no start/end poly" (navigation/mod.rs:458,475) | INFO/WARN | only findable by body text |

### Missing: proposed additions

All follow Rule 2 (debug + `event=`) and the negative-log convention (WARN for a player-visible stuck state, with
throttle + `suppressed` + an unthrottled counter). Fields always include `npc_id, tag, template_id, world, space_id`.

1. **`npc_ai.transition`**, DEBUG, `event="state_change"`, emitted from **one** helper `set_ai_state(npc, to, reason)`
   that replaces the 7+ raw `ai_state =` writes (`aggro.rs:115`, `fight.rs:131,184`, `leash.rs:57`, `dispatch.rs:328,333`,
   `set_npc_poi`, the respawn tick, lifecycle). Fields: `from, to, reason` (enum: `threat_preempt | auto_aggro |
   threat_empty | target_dead | target_gone | leash_out | leash_done | respawn | content`), `dist_to_spawn,
   threat_count, nav_path_len`. Counter `npc_ai_transitions_total{from,to,reason}`, all labels enumerated. Unthrottled:
   transitions are rare.
2. **`npc_ai.idle_parked`**, INFO, `event="parked_off_spawn"`: fires on any →Idle transition where the NPC will
   **not** be ticked (aggression 0, no patrol, no wander) *and* `dist_to_spawn > 2.0`. Fields: `dist_to_spawn,
   reason, x,y,z`. This is the direct symptom-4 detector. Counter `npc_idle_parked_total{world,reason}`.
3. **`npc_ai.aggro_scan`**, DEBUG, `event="candidate_rejected"`, in `npc_ai_idle_auto_aggro`: one row per rejected
   witness, `reason = same_faction | not_player | dead | out_of_radius | no_los | gm` (the last three once those gates
   exist), plus `player_id, account_id, dist`. Throttle per (npc, player) at 10 s. Add `event="no_candidates"` with
   `witness_count` when the scan finds nothing. Promote the existing untargeted INFO at `fight.rs:60` to
   `target: "npc_ai", event = "auto_aggro"` with `player_id, account_id, dist, aggression`.
4. **`npc_ai.idle_skipped`**, DEBUG, sampled: 1 row per Idle-not-admitted NPC per 60 s from the `dispatch.rs:263`
   filter, with `aggression, has_patrol, has_wander, faction`. It is a gauge rather than a stream: better as
   `npc_ai_idle_unticked{world}` set once per tick, and optionally the log.
5. **Leash rows**: `npc_ai` `event="leash_enter"` (INFO, at `fight.rs:184`) adding `npc_dist_to_spawn` alongside
   today's `dist_to_spawn` (which is the *target's*; rename it `target_dist_to_spawn`), plus `nav_path_len`. Then
   `event="leash_complete"` (INFO, replacing `leash.rs:67`) with `from_x/y/z`, `snap_dist`, `nav_path_len_after`
   (should be 0), `spawn_on_navmesh`, `followed` (whether the snap was skipped).
6. **`npc_ai.path_fail` reason split**: expand `PathFailReason::NoPath` into `start_off_mesh | end_off_mesh |
   no_corridor` by returning a typed error from `NavMesh::find_path` rather than `Option`. Add a new
   **`partial_path`** reason from the `DT_PARTIAL_RESULT` bit (`detour_ffi.rs`, `navigation/mod.rs:494`), with
   `partial_gap = dist(path_end, requested_end)`. Also fix the misleading fight message: fight has no straight-line
   fallback, but the message says "falling back to a straight line" (`path_failure/mod.rs:163-166`).
7. **`threat` `event="threat_cleared_without_exit"`**, WARN: when an NPC clears `threat_list` (leash, threat_empty)
   while some player still lists it in `threatened_mobs`. Fields: `player_id, account_id, mob_id, reason`. Counter
   only after the throttle.
8. **Content**: `set_aggression` logs nothing when the tag misses (`world/mod.rs:91-97`). Add
   `target:"content", event="set_aggression_tag_miss"` at WARN (Pattern C), and promote the success log to INFO with
   `from, to`.

---

## E. Legacy reference: what python did (and did not do)

- **Proximity aggro: none.** `SGWMob.py` enters Fighting only via `threatGenerated` (`:72-88`) → `aiBeginCombat`
  (`:157-163`). `setAggression` (`:53-60`) only stores `aggressionOverride` and **broadcasts
  `GENERICPROPERTY_MobAggression` to the client and witnesses**. Rust's `set_aggression` sends nothing to the wire
  (`world/mod.rs:84-97`), so the client's aggression display (`GameMob_onAggressionLevelUpdate` `0x00d31bd0`, RE
  `npc-ai-state-machine.md` §Aggro) is never fed.
- **Aggression semantics.** `EMobAggressionLevel`: 1 HOSTILE, 2 SUSPICIOUS, 3 NEUTRAL, 4 FRIENDLY, 5 DEFAULT. When no
  override exists, the level comes from `FACTION_REACTION_TABLE[player.faction][mob.faction]` (players = 3, and
  `[3][10]` = HOSTILE). This is the closest canonical basis for "faction-10 mobs are hostile on sight" without a new
  column: derive `effective_aggression = override.unwrap_or(reaction_table[player][npc])` and treat only `== 1` as
  aggro-on-sight. The current Rust `> 0` check is wrong for levels 2-5.
- **Assist/social aggro: none.**
- **Leash / evade / return-to-spawn: none.** `AI_STATE_Leashing` is never assigned. When the threat list empties
  (`:288-293`) python goes Idle **and clears BSF_InCombat on the mob**, then stays put, same as Rust. Python also has
  no NPC movement at all: `lookAt` only, mobs stand and shoot. So "parks in place" is legacy parity, but it was only
  tolerable when NPCs never moved.
- **Threat housekeeping python does that Rust skips:** `getTopThreateningEntity` (`:166-197`) calls
  `entity.onRemovedFromThreatList(self.entityId)` on dead targets, which is the player-side cleanup. `onDead`
  (`:118-138`) does the same for all entries. Threat = `-healthChange*2 - focusChange` (`:115`).
- **Original client** (RE `npc-ai-state-machine.md`, `npc-movement-pathfinding.md` §6): the client expects movement
  type 2 (Leash) with a waypath back to spawn. A walk-home leash is therefore canonical client-side intent. The
  current snap is a known and documented gap.

## Suggested fix direction (for the implementing agent, not done here)

1. Fight→Idle and leash must clear `nav_path`, return home (walk via `find_path` with movement type 2, or at minimum
   snap through `update_position_preserving_facing` and restore `spawn_dir`), and drain player-side
   `threatened_mobs`.
2. Leash predicate: `max(npc→spawn, target→spawn) > LEASH_DISTANCE`, or npc→spawn with a hysteresis band, per template
   later. Add Leashing to the preemption list, or keep it immune but log.
3. Proximity aggro: derive hostility from faction (reaction-table style) with an **aggro radius (≈15-20 u) plus a LoS
   gate**, not the 100-unit AoI. Or seed `aggression` per spawn. Either way it is a behavior change for the whole
   Cellblock, so owner sign-off is needed.
4. Snap the spawn Y to the navmesh at spawn (or use a looser start extent on the NPC path) and surface DT_PARTIAL_RESULT.

---

## Addendum (owner answers): colo, GM account in normal play, "frozen but attacking" / "running in place"

### GM status does NOT affect aggro, threat, or targeting [C]

- A grep for `access_level|is_gm|GmPlayer|invuln|no_aggro` across `cell/combat/`, `cell/service/npc_ai/`,
  `cell/abilities/`, `space_manager/aoi.rs` and `space_manager/queries.rs` finds **zero hits**. Auto-aggro
  (`fight.rs:41-57`), `generate_threat`, the threat top-pick and `handle_use_ability` never read access level.
  `gmSetNoAggro` has no server implementation.
- The SGWGmPlayer class flip (0x03) is base/wire only (`base/world_entry/methods/world_entry_db.rs:272`,
  `reanchor_player.rs:293`). The cell entity keeps `is_player = true` and faction 0. **Symptom 1 is not GM-related.**
- **One GM-only difference matters for C:** `space_manager/client_move.rs:195, 283-300`. A GM is **exempt from the
  navmesh-containment snap-back** in enforce-mode Cellblock: the position is WARN-logged (`movement.validation`) and
  kept. Consequences:
  - A GM more than 3.0 from the mesh (`DEST_EXTENTS`) makes `find_path` fail on the end poly (`no_path`), and the
    NPC freezes.
  - Anything off-mesh makes LoS `Unknown`, which is treated as clear, so an NPC within 30 attacks through geometry.
  - An ordinary player would be snapped back onto the mesh.

### Re-ranked for the observed looks

**"Running in place" (run cycle, no displacement)**

1. **[H] Stale non-zero velocity, re-broadcast every AoI tick.** `velocity` is written only by `write_position`
   (`entities.rs:521`) and zeroed only on final-waypoint arrival (`npc_movement.rs`), death (`combat/state.rs:121`),
   respawn (`npc_respawn/mod.rs:227`) and submit (`lifecycle/mod.rs:144`). Every other stop just clears `nav_path`:
   - attack-in-place, `fight.rs:550-552` (every mid-leg stop to shoot);
   - cover release, `fight.rs:331-333`;
   - threat preempt, `aggro.rs:122`;
   - the leash raw write, `leash.rs:48`.

   `compute_player_aoi` re-sends `EntityMoved` with that velocity every 100 ms (`space_manager/aoi.rs:228-241`), and
   it is packed onto the wire (`mercury/aoi/update.rs:40`). The client `USGWAvatarFilter::Output` extrapolates
   `pos = last + vel*dt` (`docs/analysis/playtests/2026-09-18-colo-castle/appendix-npc-movement.md:91-96, 281-284`),
   and the next update snaps it back to the unchanged server position: a run cycle with no progress. The appendix
   flagged this only for leash; **it also hits every attack-in-place stop**, which gives "running in place while
   shooting".
2. **[M-H] `setMovementType` is never reset on the wire.** Fighting entry sends CombatAdvance (1); leash sends Leash
   (2). The exits (`fight.rs:144`, `leash.rs:89`) call `broadcast_movement_type(None)`, which clears only the dedup
   cache and **emits no byte** (`abilities/messaging.rs:189-193`), so the client keeps the combat-advance or leashing
   state on an Idle NPC.
3. **[M] Partial-path or degenerate chase loop at an island edge.** DT_PARTIAL_RESULT is accepted, so the NPC keeps
   installing a near-zero waypoint and arriving. The logs show repeated `chase`.

**"Frozen but attacking" (stays put, keeps shooting)**

1. **[H] It cannot re-path after stopping.** It stops mid-segment at `fight.rs:550` (lerped Y). If that point sits
   outside find_path's 0.5 start box (`navigation/mod.rs:51`), every later chase is `no_path`. It shoots whenever the
   player is back inside 30 and freezes whenever the player is outside. Signature: `npc_ai.path_fail`
   `state = fight reason = no_path` at a constant npc_x/y/z, plus untargeted `NavMesh::find_path: no start poly`.
   A GM target standing off-mesh gives the same pattern with `no end poly`.
2. **[H] Designed attack-in-place at ≤30 with fail-open LoS.** It never closes below ability range and fires from
   wherever it stopped, through walls when either endpoint is off-mesh (`spatial.rs:21-23`).
3. **[M] Leash envelope.** A player more than 50 from spawn means no advance; the aggression-1 NID guard loops
   through re-aggro, leash, heal and fire.
4. **[L] Launch-failure loop.** WARN "attack tick produced no ability fire". Not really "attacking".

The former C1 ("fight ended in place → passive Idle") drops for these visuals, because a passive Idle NPC does not
shoot. It still explains frozen, non-shooting NPCs (for example after the GM died or respawned), and via #2 it can
keep a stale run pose.

### SigNoz checks (colo, last few days)

- `npc_ai.path_fail` grouped by npc_id, reason, state: a constant position with `no_path` confirms frozen #1.
- Body `NavMesh::find_path: no start poly` vs `no end poly`: tells an off-mesh NPC from an off-mesh (GM) target.
- `npc_ai.tick` for a stuck npc_id: `attack_in_place` / `no_path` alternating, `nav_path_len = 0`, a static position,
  `has_los = true`.
- `wire.out.avatar_update` (sampled) for that entity_id: non-zero vx/vz with a constant x/z **proves running #1**.
- `movement.validation` WARN with the GM's account_id: the GM was off-mesh.
- `spawner.npc_behaviour`: `y - ground_y` per guard.

### Additional instrumentation

- `npc_ai.tick`: add `vx, vy, vz` and `path_start_ok` (one `find_nearest_poly` with START_EXTENTS).
- `NavMesh::find_path`: return a typed failure (start/end/corridor/partial) so `npc_ai.path_fail` can carry it,
  replacing the untargeted, npc_id-less WARNs at `navigation/mod.rs:458, 475`.
