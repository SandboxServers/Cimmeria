---
title: "NPC AI System"
type: reference
audience: engineers
last_updated: 2026-05-27
---

# NPC AI System

> **Last updated**: 2026-05-31
> **Status**: All 12 Atrea AI states are now wired in the Rust runtime. Behavior states (Patrol, Wander, Investigating, Follow) are driven by `npc_ai_tick`; terminal states (Despawning, Submit, Error) are reachable via the `SetNpcAiState` content action. Implementation status detail is in the [summary table](#implementation-status-summary) at the bottom; the historical "Python design" sections below are kept for reference but no longer reflect the runtime.

## Overview

NPC mob behavior is driven by a state machine implemented in the Rust cell service (`crates/services/src/cell/service/npc_ai.rs`). The runtime mirrors the Python `SGWMob` design — every 2 seconds the `npc_ai_tick` snapshot-and-dispatch loop routes each NPC into a per-state handler. Threat events preempt behavior states into `Fighting` with per-state scratch preserved.

The Detour-backed navmesh (via `space_mgr.find_path`) handles pathfinding for all movement states. Movement is interpolated at 100 ms cadence by `npc_movement_tick`.

**Key files (Rust runtime):** `crates/entity/src/cell_entity/mod.rs` (the 12-state `AiState` enum + per-state scratch fields), `crates/services/src/cell/service/npc_ai.rs` (state-machine dispatch + per-state handlers), `crates/services/src/cell/combat/threat.rs` (`generate_threat` preemption), `crates/services/src/cell/service/ticks/npc_respawn.rs` (Dead → Idle promotion). The Python design files referenced in legacy sections (`python/cell/SGWMob.py`, `python/Atrea/enums.py`) are kept for evidence-of-intent only.

---

## AI State Machine

### State Definitions

Defined in `python/Atrea/enums.py` lines 228-239.

| State | Value | Implemented | Notes |
|-------|-------|-------------|-------|
| `AI_STATE_Spawning` | 0 | INERT | Variant preserved for source-enum completeness but the Rust runtime starts every NPC at Idle and never enters this state. Future spawn-VFX hooks (Goa'uld ribbon-device reveal, etc.) can plug in here. |
| `AI_STATE_Idle` | 1 | DONE | Waits for threat or AI tick promotion into Patrol / Wander / auto-aggro. |
| `AI_STATE_Investigating` | 2 | DONE | `npc_ai_investigate` — pathfind to `poi`, dwell 5 s on arrival, return to Idle. Reached via `SetNpcPoi` content action. |
| `AI_STATE_Fighting` | 3 | DONE | Target selection, ability selection, fire. Per-ability range gating + retry-on-launch-failure (see [#329](https://github.com/SandboxServers/Cimmeria/issues/329)). |
| `AI_STATE_Leashing` | 4 | DONE | `npc_ai_leash` snaps NPC to spawn + restores HP when target exceeds `LEASH_DISTANCE = 50`. |
| `AI_STATE_Dead` | 5 | DONE | Set on death via `combat::mark_npc_dead`. `npc_respawn_tick` (1 Hz) promotes back to Idle when `respawn_at` elapses. |
| `AI_STATE_Despawning` | 6 | DONE | `npc_ai_despawn` removes the entity from the space. Reached via `SetNpcAiState`. |
| `AI_STATE_Follow` | 7 | DONE | `npc_ai_follow` maintains distance band `[follow_min_distance, follow_max_distance]` to a target. Reached via `SetFollowTarget`. |
| `AI_STATE_Patrol` | 8 | DONE | `npc_ai_patrol` walks a loop from `entity_templates.patrol_path_id` → `point_set_points`, dwells on arrival at each waypoint. Threat preemption preserves the index. |
| `AI_STATE_Wander` | 9 | DONE | `npc_ai_wander` samples random points within `wander_radius` of spawn, dwells for `[wander_min_dwell_secs, wander_max_dwell_secs]` between hops. |
| `AI_STATE_Submit` | 10 | DONE | `npc_ai_submit` clears combat state and holds. Reached via `SetNpcAiState`. |
| `AI_STATE_Error` | 11 | DONE | `npc_ai_error` is quiescent — diagnostic fallback. Reached via `SetNpcAiState` or the `enterErrorAIState` slash command. |

### State Transitions

`generate_threat` preempts any non-Dead non-Fighting state to Fighting (with per-state scratch preserved so the post-fight return can re-evaluate). Idle promotion priority is **aggression > patrol > wander**.

```
Idle      -->  Fighting    (generate_threat fires + NPC was Idle / Patrol / Wander / Investigating / Follow)
Idle      -->  Patrol      (npc_ai_tick observes patrol_path non-empty)
Idle      -->  Wander      (npc_ai_tick observes wander_radius > 0 and no patrol_path)
Idle      -->  Investigating (SetNpcPoi content action)
Idle      -->  Follow      (SetFollowTarget content action with a valid target)
Fighting  -->  Idle        (threat list drains)
Fighting  -->  Leashing    (target exceeds LEASH_DISTANCE from spawn)
Leashing  -->  Idle        (snap to spawn + HP restore complete)
Any alive -->  Dead        (HP -> 0; combat::mark_npc_dead)
Dead      -->  Idle        (npc_respawn_tick promotes; respawn_at elapsed)
Any alive -->  Despawning / Submit / Error  (SetNpcAiState content action)
```

### Tick Loop

`doAiAction()` is the main AI tick, called on a recurring timer:

```python
def doAiAction(self):
    if self.isDead():
        return
    state = self.AIState
    if state == AI_STATE_Spawning:
        self.doAiSpawnAction()
    elif state == AI_STATE_Idle:
        self.doAiIdleAction()
    elif state == AI_STATE_Fighting:
        self.doAiFightingAction()
    # All other states fall through without action
```

---

## Threat System

### Data Structure

Threat is stored as a plain dict on the mob instance:

```python
self.threat = {}  # entityId (int) -> accumulated threat (float)
```

### Damage-to-Threat Conversion

```python
# In onStatChange() or equivalent damage handler:
threat = -healthChange * 2 - focusChange
self.threatGenerated(attackerEntityId, threat)
```

Health damage is weighted 2x relative to focus damage. The formula is negated because `healthChange` is negative when damage is dealt.

### Threat Accumulation

```python
def threatGenerated(self, entityId, threat):
    if entityId not in self.threat:
        self.threat[entityId] = 0.0
    self.threat[entityId] += threat
    if self.AIState == AI_STATE_Idle and not self.isDead():
        self.AIState = AI_STATE_Fighting
```

### Target Selection

`getTopThreateningEntity()` does a linear scan of the threat dict:

1. Iterates all entries in `self.threat`.
2. Prunes entries where the entity is dead (`entity.isDead()` or entity no longer exists in AoI).
3. Returns the entity ID with the highest accumulated threat value.
4. Returns `None` if the list is empty after pruning, which triggers a transition back to Idle.

**Known issues:**
- Entity ID recycling: a dead entity's ID may be reused by a new entity, which would falsely inherit threat.
- Distance, line-of-sight, and cover are not factored into target selection.
- No threat decay over time.

### Unimplemented Threat Methods

Declared on `SGWMob` but contain no logic: `addDirectToThreatList`, `addBuffToThreatList`, `addHealToThreatList`, `addToThreatList`, `removeFromThreatList`, `onGroupMateEnteredCombat`, `onGroupMateThreatTransfer`.

---

## Aggression System

### Aggression Levels

| Level | Value | Meaning |
|-------|-------|---------|
| `HOSTILE` | 1 | Attacks on sight (proactive aggro — NOT IMPLEMENTED) |
| `SUSPICIOUS` | 2 | Heightened alertness |
| `NEUTRAL` | 3 | Default — ignores players |
| `FRIENDLY` | 4 | Will not attack |
| `DEFAULT` | 5 | Falls back to faction/template default |

The default value in `SGWMob.def` is 3 (Neutral). Mobs do not proactively detect or aggro players at any aggression level — they only enter combat when damage is received.

### Per-Instance Override

The `aggressionOverride` property stores a per-instance aggression level that takes precedence over the template default. The client is notified of changes via the `onAggressionOverrideUpdate` client method.

### Timed Overrides

```python
def overrideAggression(self, level, entityBase, seconds):
    # Sets aggressionOverride, schedules revert after `seconds`
```

This allows scripted events to temporarily change a mob's stance (e.g., a friendly NPC turned hostile during a mission encounter) and automatically revert afterward.

---

## Ability Selection (Combat AI)

### Classification

`classifyHostileAbility(target, ability)` evaluates a single ability and returns one of:

| Result | Value | Condition |
|--------|-------|-----------|
| `ABILITY_Usable` | 1 | Passes all checks |
| `ABILITY_CoolingDown` | 2 | On cooldown |
| `ABILITY_Filtered` | 3 | Heal, buff, or non-single-target mode |
| `ABILITY_NeedsAmmo` | 4 | No ammo remaining |

Classification logic in order:
1. **Filter heals and buffs** — Abilities that restore health or apply positive effects to self are excluded.
2. **Require single-target mode** — Only `TCM_Single` targeting mode is accepted. AoE and cone abilities return `ABILITY_Filtered`.
3. **Check cooldown** — If the ability's cooldown timer is active, returns `ABILITY_CoolingDown`.
4. **Check ammo** — If the ability requires ammo and the mob has none, returns `ABILITY_NeedsAmmo`.

### Selection Loop

`selectHostileAbility(target)` iterates the mob's ability set:

```python
def selectHostileAbility(self, target):
    needs_ammo = []
    for ability in self.getAbilities():
        result = self.classifyHostileAbility(target, ability)
        if result == ABILITY_Usable:
            return ability       # First usable ability wins
        elif result == ABILITY_NeedsAmmo:
            needs_ammo.append(ability)
    if needs_ammo:
        self.triggerReload()    # All blocked by ammo: reload
    return None
```

There is no priority weighting — the first usable ability in iteration order is selected. No distance checks, no cooldown preference, no situational logic (e.g., prefer ranged when target is far).

### Combat Tick

`doAiFightingAction()` runs each combat tick:

1. Call `getTopThreateningEntity()`. If `None`, set `AIState = AI_STATE_Idle` and return.
2. Call `lookAt(target)` to rotate the mob toward the target.
3. Call `selectHostileAbility(target)`. If an ability is returned, launch it.
4. If no ability is available (all on cooldown or no ammo), schedule a 0.5-second retry.

---

## Ammo Management

Mobs use the same `bandolier_items` / `Stat[AMMO_SLOT_1+slot]` model as players in principle. In practice the **Rust port skips the ammo gate for non-players**: the fire-gate in [`crates/services/src/cell/abilities.rs:259-263`](../../crates/services/src/cell/abilities.rs#L259) short-circuits with `entity.is_player && current_ammo < required_ammo`, so NPCs currently fire without consuming rounds and never need to reload. `triggerReload()` is not yet ported.

Legacy accessors and their Rust equivalents:

| Legacy (`SGWMob.py` / `SGWPlayer.py`) | Rust equivalent |
|----------------------------------------|-----------------|
| `getAmmoStat()` — stat ID for current slot | `crate::stats::AMMO_SLOT_1 + entity.active_bandolier_slot` |
| `getClipSize()` — max ammo from equipped weapon | [`CellEntity::active_clip_size()`](../../crates/entity/src/cell_entity.rs#L373) |
| `getAmmoCount()` — current ammo | [`CellEntity::active_ammo()`](../../crates/entity/src/cell_entity.rs#L366) |
| `consumeAmmo(amount)` | [`CellEntity::set_slot_ammo(slot, current - amount)`](../../crates/entity/src/cell_entity.rs#L390) |
| `triggerReload()` | Not ported for NPCs (player path: [`handle_reload`](../../crates/services/src/cell/cell_methods/player/world.rs#L121)) |

Legacy behavior: on spawn (`doAiSpawnAction`), the mob called `getClipSize()` on its equipped weapon and set its ammo stat to that value, representing a full reload at spawn. When `selectHostileAbility` found all abilities blocked by ammo, it called `triggerReload()`. The reload completed after a delay and refilled the clip, allowing the combat loop to resume.

If/when NPC reload is needed, the same machinery applies — but **all three** of the following are required together; partial work will silently leave NPCs stuck mid-reload:

1. Drop the `is_player` short-circuit in the fire-gate ([`abilities.rs`](../../crates/services/src/cell/abilities.rs)).
2. Set `reload_complete_at` from an AI-driven path (an NPC equivalent of `requestReload`).
3. **Widen `reload_completion_tick`** ([`service.rs:610`](../../crates/services/src/cell/service.rs#L610)) — it currently iterates `space_mgr.all_player_entity_ids()` only, so an NPC's deadline would never be promoted. Add an `all_reloadable_entity_ids()` accessor or extend the existing one to include fighting NPCs.

See [weapon-ammo-reload.md](weapon-ammo-reload.md) for the full ammo and reload model.

---

## Tapping System (Kill Credit)

These properties are defined in `SGWMob.def` but have no Python implementation:

| Property | Type | Purpose |
|----------|------|---------|
| `tappedEntity` | INT32 | Entity ID with loot and XP rights |
| `tappedSquad` | INT32 | Squad ID with loot and XP rights |
| `tappedSquadMembers` | ARRAY<INT32> | Individual members of the tapped squad |

Tapping determines who receives loot drops and XP when the mob dies. Currently, loot generation on death runs without any tap check — all loot goes to whoever triggered the death event.

---

## Mob Properties Reference

Key properties from `SGWMob.def` (55 total), grouped by subsystem:

**Controller IDs** (C++ controller handles, stored as INT32):
`navControllerID`, `visionID`, `yawID`, `behaviorTimerID`, `despawnTimerID`, `investigateTimerID`, `grenadeDetectorID`, `trackControllerID`, `targetOverrideTimer`

**AI State:**
`AIState`, `POI` (VECTOR3 — investigate destination), `Home` (VECTOR3 — spawn/leash anchor), `lastNavigate`, `stateLock`, `stateChanges`, `stateHistory`, `disableBehaviorSystem`, `nextWanderTime`

**Combat:**
`MyAbilitySetID`, `LootTableID`, `Aggression`, `minIdealRange`, `maxIdealRange`, `isKillable`, `isTrackable`, `isWorthXP`

**Cover:**
`bCoverFromTarget`, `useCover`, `reservedCoverNode`, `CombatStance`

**Following:**
`currentlyFollowing`, `followTarget`, `followMinDistance`, `followMaxDistance`, `followAngle`, `followMovementType`

**Patrol:**
`patrolPaths` (dict), `currentPatrolPath`, `patrolMovementType`

**Hearing:**
`hearingRadius`

**Despawn:**
`despawnFlag`, `despawnTimerID`, `spawnTime`, `decayTimerID`

**Behavior Events:**
`mobBehaviorEventSet`

---

## Unimplemented States: Reconstruction Notes

### Investigating (State 2)

A mob heard a noise or detected suspicious movement but has not confirmed a threat. It should navigate to `POI`, look around for a set duration, and return to `Home` if nothing is found.

Evidence: `POI` (VECTOR3) property holds the destination, `investigateTimerID` stores a C++ timer controller, `hearingRadius` controls detection range, `onNoise()` is a declared cell method that would set `POI` and transition to this state.

### Leashing (State 4)

The mob's current target has moved beyond pursuit range or out of LOS. The mob abandons the fight and returns to `Home`, clearing its threat list on arrival.

Evidence: `Home` (VECTOR3) stores the spawn anchor, `maxIdealRange` defines engagement distance. Standard MMO pattern: if distance to Home exceeds `maxIdealRange * 2`, cancel pursuit, navigate Home, clear `self.threat`, transition to Idle.

### Patrol (State 8)

The mob follows a scripted waypoint path between spawn locations.

Evidence: `patrolPaths` (dict) stores one or more named path definitions, `currentPatrolPath` tracks the active path index, `patrolMovementType` controls speed/animation. DB columns `patrol_path_id` and `patrol_point_delay` in `entity_templates`. C++ methods `startPatrol(path, delay)` and `cancelPatrol()` are declared on the cell entity.

### Wander (State 9)

Random movement within a radius of `Home`. The mob picks a random nearby point, navigates there, waits a random delay, then picks another point.

Evidence: `Home` property provides the anchor, `nextWanderTime` property stores the timestamp of the next wander move. Recommend: use `findPathTo()` with a randomly offset position from `Home`, bounded by `minIdealRange`.

### Follow (State 7)

The mob maintains a set distance and angle behind a target entity (used by pets and escort NPCs).

Evidence: `currentlyFollowing` (bool), `followTarget` (entity reference), `followMinDistance`, `followMaxDistance`, `followAngle`, `followMovementType` properties all defined in `.def`.

### Submit (State 10)

A controlled shutdown state for mobs that surrender rather than fight to the death (e.g., scripted encounters). The mob stops fighting and signals completion to the mission system before despawning.

Evidence: state is defined in the enum; no supporting properties are uniquely tied to this state.

### Error (State 11)

A diagnostic recovery state for when the AI reaches an inconsistent condition.

Evidence: Cell methods `enterErrorAIState()` and `leaveErrorAIState()` are declared. Properties `errorStateReason`, `errorStateDescription`, `errorAIState`, and `errorTime` are defined for logging the failure context.

### Despawning (State 6)

Controlled removal of the mob from the world, distinct from death. Allows animations and cleanup to complete before the entity is destroyed.

Evidence: `despawnFlag` (bool) property, `despawnTimerID` controller, `DespawnWhenFree()` cell method, `decayTimerID` for corpse removal after death.

---

## Navigation Integration

The C++ cell layer exposes these navigation methods to Python:

| Method | Purpose |
|--------|---------|
| `findPathTo(position)` | Compute and begin moving along a navmesh path |
| `findDetailedPathTo(position)` | Higher-fidelity path with full waypoint list |
| `addWaypoint(position)` | Append a waypoint to the current path |
| `cancelMovement()` | Stop all movement immediately |

None of these are called by the current Python mob AI. The `navControllerID` property is reserved for a C++ navigation controller that is never created. The only movement primitive used is `lookAt(target)`, which rotates the mob's yaw toward a target entity without translating.

The practical result is that all mobs are stationary during combat. They rotate to face their target and fire, but do not close distance, retreat to cover, or reposition.

---

## Cover System (Not Implemented)

Cover nodes are spatial graph nodes placed in the world that provide defensive bonuses. The design supported mobs finding and reserving cover positions before or during combat.

Relevant properties: `useCover` (bool), `bCoverFromTarget` (bool direction flag), `CombatStance` (enum), `reservedCoverNode` (node reference).

Relevant cell methods: `onReserveCoverSlot()`.

None of this is implemented. The `CombatStance` property is set but not acted upon.

---

## Behavior Event System (Not Implemented)

`mobBehaviorEventSet` stores one or more named event sets that define data-driven behavior triggers. The design intent appears to be a table-driven system where events (e.g., "player enters radius", "health drops below 50%") trigger scripted responses (e.g., bark dialog, call for help, switch ability set).

Cell methods `addBehaviorSet(name)` and `removeBehaviorSet(name)` are declared for runtime modification of the active event sets. No behavior set logic is implemented.

---

## Mob Groups (Not Implemented)

`mobGroup` property and `mobJoinGroup()` cell method are defined for coordinating multiple mobs as a unit. This would enable pack behavior (all members assist when one is attacked), coordinated patrol paths, and shared threat lists. No group logic is implemented.

---

## Implementation Status Summary

| Feature | Status | Notes |
|---------|--------|-------|
| State machine tick loop | DONE | `doAiAction()` dispatches by state |
| Spawning state | DONE | Loads ammo, transitions to Idle |
| Idle state | DONE | Waits for `threatGenerated()` |
| Fighting state | DONE | Target selection, ability fire, 0.5s retry |
| Dead state | DONE | Loop exits cleanly |
| Threat accumulation | DONE | Damage -> threat formula, Idle->Fighting transition |
| Top-threat targeting | DONE | Linear scan with dead-entity pruning |
| Ability classification | DONE | Type/targeting/cooldown/ammo checks |
| Ammo management | DONE | Load on spawn, consume per shot, auto-reload |
| Combat exit | DONE | Threat empty -> Idle |
| Loot on death | DONE | Loot table referenced, no tap check |
| Aggression override | DONE | With client broadcast and timer revert |
| lookAt() rotation | DONE | Mob faces target during combat |
| Leashing state | DONE | `npc_ai_leash` snaps NPC to spawn + restores HP on `Fighting → Leashing` transition when target exceeds `LEASH_DISTANCE = 50`. |
| Proactive aggro detection | DONE | `aggression > 0` → `npc_ai_idle_auto_aggro` scans witnesses every 2 s, seeds 1.0 threat on the closest opposing-faction player. Set via `set_aggression` content action. |
| Navigation (findPathTo) | DONE | Detour FFI behind `space_mgr.find_path()` + `npc_movement_tick` consumes `nav_path` waypoints at 100 ms. See [#35](https://github.com/SandboxServers/Cimmeria/issues/35). |
| Per-ability range | DONE | `ability_ranges()` reads each ability's `min_range`/`max_range` from defs; fight tick gates on the chosen ability rather than a flat 30 m. See [#329](https://github.com/SandboxServers/Cimmeria/issues/329). |
| Three-bucket ability selection | DONE | `choose_npc_ability` partitions known abilities into usable / cooling / needs-ammo and picks the first off-cooldown ID. See [#342](https://github.com/SandboxServers/Cimmeria/issues/342). |
| `setMovementType` AoI broadcast | DONE | `broadcast_movement_type` fans the EMobMovementType byte to AoI witnesses on every state transition (CombatAdvance on Fighting entry, Leash on Leashing entry, clear on Idle). Dedup'd against `last_movement_type` so re-entry of same state is a wire no-op. Closes [#270](https://github.com/SandboxServers/Cimmeria/issues/270). |
| NPC respawn | DONE | `npc_respawn_tick` (1 Hz) reads `respawn_secs` (COALESCE `spawnlist`, `entity_templates`, minimum 3s enforced via CHECK). On NPC death the `combat::mark_npc_dead` helper stamps `respawn_at = now + respawn_secs`. Tick promotes Dead → Idle, restores HP / FOCUS / state / interaction-type / facing direction, snaps position to spawn, closes any open loot UIs on still-looting players, and broadcasts in wire order: EntityMoved → INTERACTION_TYPE → ON_STATE_FIELD_UPDATE → ON_STAT_UPDATE. `NULL` columns → one-shot mob (corpse persists). Effect-script-driven HP-to-0 paths that bypass `damage_apply` (e.g., `scripts::MeleeDamage`) also bypass respawn — future content using those paths must call `combat::mark_npc_dead` explicitly. |
| Investigating state | DONE | `npc_ai_investigate` handler routes the NPC to a content-set `poi`, dwells 5s (`INVESTIGATE_DWELL_SECS`), returns to Idle. Reached via the `SetNpcPoi` content action; the `onNoise` cell-method hook for in-game audio is deferred. |
| Patrol state | DONE | `npc_ai_patrol` walks the loop from `entity_templates.patrol_path_id` → `point_set_points`. Dwells `patrol_point_delay` at each waypoint. Threat preemption preserves `patrol_next_index` so the post-fight return resumes the route. |
| Wander state | DONE | `npc_ai_wander` samples a random point within `wander_radius` of `spawn_position`, validates against the navmesh, dwells a random duration in `[wander_min_dwell_secs, wander_max_dwell_secs]`. Off-mesh candidates fall back to `spawn_position`. |
| Follow state | DONE | `npc_ai_follow` maintains a distance band `[follow_min_distance, follow_max_distance]` to the target. Out of band → pathfind toward target; below min → hold (no back-away). Reached via the `SetFollowTarget` content action. |
| Submit state | DONE | `npc_ai_submit` clears combat state (threat_list, BSF_IN_COMBAT, movement-type cache) and holds. Reached via the `SetNpcAiState` content action. |
| Error state | DONE | `npc_ai_error` is a quiescent diagnostic state — handler is a no-op per tick. Reached via the `SetNpcAiState` content action or the `enterErrorAIState` slash command. |
| Despawning state | DONE | `npc_ai_despawn` removes the entity from the space on entry; AoI fires the leave events to witnesses. Reached via the `SetNpcAiState` content action. |
| Cover system | NOT IMPL | Tracked by [#209](https://github.com/SandboxServers/Cimmeria/issues/209) — needs spatial index + reservation lifecycle on top of the state machine. |
| Mob group coordination | NOT IMPL | mobGroup property, mobJoinGroup() declared; deferred. |
| Behavior event sets | NOT IMPL | addBehaviorSet/removeBehaviorSet declared; deferred. |
| Tapping (kill credit) | DONE | Content-engine kill chains supersede the Python tap design. |
| Group mate threat assist | NOT IMPL | Methods declared, no logic. |
