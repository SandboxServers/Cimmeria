# NPC AI System

> **Last updated**: 2026-03-01
> **Status**: ~20% implemented — Spawning, Idle, Fighting, and Dead states work. All movement states (Patrol, Wander, Leashing, Investigating, Follow) are unimplemented. Mobs stand still while fighting.

## Overview

NPC mob behavior is driven by a state machine implemented in Python on the cell app. The `SGWMob` entity type provides the base AI logic, with `SGWPet` extending it for player-controlled companions. Each mob runs a tick loop (`doAiAction()`) that evaluates the current state and dispatches to a per-state handler.

The C++ cell layer provides navigation primitives (`findPathTo`, `addWaypoint`, `cancelMovement`) and controller IDs for navigation, vision, yaw, and timers, but the Python AI currently ignores all of these. The mob's only movement call is `lookAt()` to rotate toward a target during combat.

**Key files:** `python/Atrea/enums.py` (state and ability enums), `python/cell/SGWMob.py` (state machine, threat, ability selection, ammo), `entities/defs/SGWMob.def` (55 properties, cell and client methods).

---

## AI State Machine

### State Definitions

Defined in `python/Atrea/enums.py` lines 228-239.

| State | Value | Implemented | Notes |
|-------|-------|-------------|-------|
| `AI_STATE_Spawning` | 0 | YES | Loads weapon ammo, transitions to Idle |
| `AI_STATE_Idle` | 1 | YES | Waits for threat |
| `AI_STATE_Investigating` | 2 | NO | POI-based investigation loop |
| `AI_STATE_Fighting` | 3 | YES | Target selection, ability selection, fire |
| `AI_STATE_Leashing` | 4 | NO | Return to Home when target out of range |
| `AI_STATE_Dead` | 5 | YES | Checked in `doAiAction()`, terminates loop |
| `AI_STATE_Despawning` | 6 | NO | Controlled despawn sequence |
| `AI_STATE_Follow` | 7 | NO | Follow a target entity |
| `AI_STATE_Patrol` | 8 | NO | Waypoint path traversal |
| `AI_STATE_Wander` | 9 | NO | Random movement near Home |
| `AI_STATE_Submit` | 10 | NO | Surrender/graceful despawn |
| `AI_STATE_Error` | 11 | NO | Recovery/diagnostic state |

### State Transitions (Implemented)

```
Spawning  -->  Idle       (doAiSpawnAction complete, ammo loaded)
Idle      -->  Fighting   (threatGenerated() called while alive)
Fighting  -->  Idle       (threat list empty after target pruning)
Any       -->  Dead       (isDead() returns true, loop exits)
```

All other transitions (Idle -> Investigating, Fighting -> Leashing, etc.) are defined by design but not implemented.

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

Mobs use the same ammo model as players. Relevant accessors:

- `getAmmoStat()` — Returns the stat ID representing current ammo count
- `getClipSize()` — Returns max ammo from the equipped weapon template
- `getAmmoCount()` — Returns current ammo value
- `consumeAmmo(amount)` — Decrements ammo by `amount`

On spawn (`doAiSpawnAction`), the mob calls `getClipSize()` on its equipped weapon and sets its ammo stat to that value. This represents a full reload at spawn.

When `selectHostileAbility` finds that all abilities are blocked by ammo, it calls `triggerReload()`. The reload completes after a delay and refills the clip, allowing the combat loop to resume.

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
| Investigating state | NOT IMPL | POI, timer, hearingRadius wired but idle |
| Leashing state | NOT IMPL | Home property exists, no pursuit abort |
| Patrol state | NOT IMPL | patrolPaths defined, C++ hooks present |
| Wander state | NOT IMPL | nextWanderTime defined, no navigation calls |
| Follow state | NOT IMPL | All follow properties defined |
| Submit state | NOT IMPL | State defined only |
| Error state | NOT IMPL | Properties and hooks defined |
| Despawning state | NOT IMPL | despawnFlag, timer defined |
| Navigation (findPathTo) | NOT IMPL | C++ API available, never called |
| Cover system | NOT IMPL | Properties and node types defined |
| Proactive aggro detection | NOT IMPL | Mobs only enter combat when hit |
| Mob group coordination | NOT IMPL | mobGroup property, mobJoinGroup() declared |
| Behavior event sets | NOT IMPL | addBehaviorSet/removeBehaviorSet declared |
| Tapping (kill credit) | NOT IMPL | Properties defined, no Python logic |
| Group mate threat assist | NOT IMPL | Methods declared, no logic |
