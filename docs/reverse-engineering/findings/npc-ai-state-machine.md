# NPC AI State Machine — Client Binary Analysis

> **Last updated**: 2026-03-08
> **Source**: SGW.exe Ghidra decompilation
> **Confidence**: HIGH — decompiled movement handler with debug strings, entity callbacks, RTTI

---

## Key Finding: 7 Movement Types, Server-Driven FSM

The client does NOT run AI logic. The server drives all state transitions and sends `aMovementType` + `aPath` to the client. The client renders the resulting movement with debug visualization.

**The server currently only implements "Spawning" and "Fighting" — all 7 movement types need server implementation.**

---

## Movement Type Enum (jump table at `0x00dec018`)

Handler: `BigWorld_onRemoteEntityMove_MovementTypeSwitch` at `0x00deb660` (610 instructions)

| Value | Debug String | AI State | Purpose |
|-------|-------------|----------|---------|
| 0 | "Entity: %d is moving to cover" | **CoverAdvance** | Move to cover position |
| 1 | "Entity: %d is making a combat advance" | **CombatAdvance** | Advance toward target in combat |
| 2 | "Entity: %d is leashing" | **Leash/Return** | Return to spawn point |
| 3 | "Entity: %d is patroling" | **Patrol** | Follow patrol waypoints |
| 4 | "Entity: %d is following" | **Follow** | Follow another entity |
| 5 | "Entity: %d is wandering" | **Wander** | Random movement in area |
| 6 | "Entity: %d is avoiding" | **Avoid** | Avoid danger/AoE |
| >6 | "Entity: %d is performing unknown movement" | **Unknown** | Fallback |

Each case parses from the server event:
- `aPath` — waypoint data (list of positions)
- `aMovementType` — the enum value above
- `aEntityId` — which entity is moving

The handler averages path waypoints into entity location fields at `+0xdc`, `+0xe0`, `+0xe4`, then calls vtable `+0x2cc` to update the actor.

---

## Entity Type Hierarchy

Registration at `0x00c67420`:

```
GameEntityBase → GameEntity → GameBeing → GameMob
                                        → GamePet
                            → GamePlayer
```

| Index | Entity Type | C++ Class | Source |
|-------|------------|-----------|--------|
| 0 | Account | GameAccount | GameEntityFactory.cpp |
| 1 | SGWSpawnableEntity | GameEntity | GameEntity.cpp |
| 2 | SGWDuelMarker | GameEntity | GameEntity.cpp |
| 3 | SGWBeing | GameBeing | GameBeing.cpp |
| 4 | SGWMob | GameMob | GameMob.cpp |
| 5 | SGWPet | GamePet | — |
| 6 | SGWPlayer | GamePlayer | GamePlayer.cpp |
| 7 | SGWGmPlayer | GamePlayer | GamePlayer.cpp |

---

## Entity Callback Registration

`SGWBeing` callbacks at `0x00df3ab0`, `SGWMob` callbacks at `0x00df3cc0` (identical set):

| Address | Callback | Purpose |
|---------|----------|---------|
| `0x00dedf30` | TickUpdate | Per-frame update, timing |
| `0x00deaaf0` | onPositionUpdate | Position/movement interpolation |
| `0x00deb660` | MovementTypeSwitch | **7-state AI movement handler** |
| `0x00dec040` | PathDestroy | Path cleanup |
| `0x00df3550` | RegionUpdate | Region/zone change |

---

## Aggro/Threat System

### Aggression Level (GameMob)
- Handler: `GameMob_onAggressionLevelUpdate` at `0x00d31bd0`
- Property: `aAggressionLevel` (int8) stored at `GameMob + 0x16c`
- UI class: `UIAggressionLevel` (RTTI `0x01de972c`)
- Override: `aggressionOverrides` property, `onAggressionOverrideUpdate`/`Cleared` events
- GM command: `gmSetNoAggro` → `Event_NetOut_SetNoAggro`

### Threat System (GamePlayer)
- Handler: `GamePlayer_onHasThreatUpdate` at `0x00e07570`
- Properties: `EntityId` (int32), `HasThreat` (uint8)
- `HasThreat=true` → add threat indicator (`0x00c6bd20`)
- `HasThreat=false` → remove threat indicator (`0x00e083a0`)
- `onThreatenedMobsUpdate` — sends player list of mobs with threat on them

---

## Behavior Event System (NOT Behavior Trees)

SGW uses a **Behavior Event** system — event-driven, data-loaded from cooked paks:

- `BehaviorEventData` class (RTTI `0x01e254a8`)
- `CookedData::BehaviorEventType` (RTTI `0x01e25154`)
- Data file: `CookedBehaviorEvents.pak`
- Properties: `aBehaviorEventId`, `BehaviorEventSetId`, `aBehaviorEventSetId`

### GM Debug Commands
| Command | Purpose |
|---------|---------|
| `gmEmitBehaviorEventOnMob` | Emit behavior event on target mob |
| `gmDebugBehaviorsOnMob` | Debug behavior state |
| `gmDebugPathsOnMob` | Debug pathing |
| `gmDebugAbilityOnMob` | Debug abilities |
| `gmAddBehaviorEventSet` | Add behavior event set |
| `gmRemoveBehaviorEventSet` | Remove behavior event set |
| `loadBehavior` | Load behavior definition |
| `enterErrorAIState` / `exitErrorAIState` | Force error/recovery AI state |

---

## Combat State System

- Property: `CombatState` (strings at `0x01956cbc`, `0x019b5638`)
- UI class: `UICombatantState` (RTTI `0x01de9368`)
- Kismet event: `SeqEvent_CombatStateChanged` (RTTI `0x01e519c0`, factory `0x00dcdc80`)
- Condition: `SeqCond_IsInCombat` (RTTI `0x01dc56d0`)
- Debug: `gmDebugCombat`, `gmDebugCombatVerbose`, `toggleCombatLOS`

---

## Patrol/Waypoint System

- `SGWSpecWayPoints` actor class (string `0x018d773e`)
- `SGWActorFactoryWaypoint` (RTTI `0x01dd5170`)
- Property: `activeWayPoints`
- Debug: `Event_SlashCmd_ShowWaypoints`, `Event_SlashCmd_ShowCommandWaypoints`
- Navigation: `ANavigationPoint` → `GetAllNavInRadius`, `CanTeleport`, `GetReachSpecTo`

---

## Cover System

Deeply integrated with AI movement type 0 (CoverAdvance):

| Class | Purpose |
|-------|---------|
| `ACoverLink` | Cover link actor placed in levels |
| `CoverGroup` | Group of cover links (can be toggled) |
| `CoverSlotMarker` | Individual cover position |
| `SGWCoverNodeComponent` | SGW cover component |
| `SGWSpecCoverNode` | Editor cover node |

### Cover Properties
| Property | Description |
|----------|-------------|
| `defCover` / `aDefCoverWeight` | Defensive cover weight |
| `offCover` / `aOffCoverWeight` | Offensive cover weight |
| `cover` / `aCoverWeight` | General cover weight |
| `CoverHeight` | Cover height |
| `CoverQRModifier` | QR combat modifier when in cover |

NavMesh: `.cdata/navmesh`, debug via `Event_SlashCmd_ShowNavMesh`

---

## State Field System (GameBeing)

- `Event_NetIn_onStateFieldUpdate` / `onStateFieldUpdate` — server pushes state bitmask
- Property: `bStateField` — bitmask of active states
- `Event_Entity_StateFieldChanged` — client event when states change
- Listeners: `GameProxyPlayer`, `USGWTargetIndicator`

---

## Kismet AI Events

| Event | Purpose |
|-------|---------|
| `SeqEvent_EntityDeath` | Entity died |
| `SeqEvent_EntityMakeDead` | Force death |
| `SeqEvent_EntitySpawn` | Entity spawned |
| `SeqEvent_EntityAlert` | Entity alert state |
| `SeqEvent_EntityEnemyCombat` | Enemy combat started |
| `SeqEvent_EntityMission` | Mission-related |
| `SeqEvent_EntityAllyAction` | Ally action triggered |
| `SeqEvent_CombatStateChanged` | Combat state toggled |
| `SeqEvent_SeeDeath` | Witness a death |

---

## Implications for Cimmeria

1. **Implement all 7 movement types.** Server must send `aMovementType` (0-6) + `aPath` (waypoint list) + `aEntityId` to client.

2. **No behavior trees** — use Behavior Events loaded from `CookedBehaviorEvents.pak`. The system is event-driven: emit events → trigger state transitions → send movement updates.

3. **Leash is type 2.** No `LeashDistance` property on client — leash radius is server-only logic.

4. **Cover is type 0.** NPCs need pathfinding to `CoverLink` actors. `CoverQRModifier` affects combat when in cover.

5. **`bStateField` bitmask** is the main state sync channel. Server sets bits → client fires `StateFieldChanged` → UI updates.

6. **AggressionLevel (int8)** controls mob hostility display. Threat system is separate — `HasThreat` per-player, `onThreatenedMobsUpdate` for threat list.
