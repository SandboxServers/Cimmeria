# Gameplay Systems Gap Analysis

> **Last updated**: 2026-03-01
> **Purpose**: Map every gameplay system's server code against what's needed for a complete implementation
> **Status**: Source of truth for project completion tracking

---

## Status Taxonomy

| Status | Symbol | Meaning |
|--------|--------|---------|
| **Confirmed Working** | `CW` | Tested end-to-end with the game client and verified correct |
| **Needs Test** | `NT` | Code exists, looks reasonable, but hasn't been verified with a live client |
| **Implemented** | `IM` | Code written but may be incomplete or have known issues |
| **Known / Missing** | `KM` | We know this needs to exist (from .def files, docs, or game design) but no code exists |
| **Needed / Unknown** | `NU` | Server-only system we infer must exist but have no direct evidence for |

---

## Infrastructure Systems (Solid)

### 1. Authentication and Login --- CW

- **Confidence**: HIGH
- **Documentation**: [connection-flow.md](connection-flow.md), [login-handshake.md](protocol/login-handshake.md)
- **Server code**: Full C++ implementation in `src/server/AuthenticationServer/`. SOAP/HTTP login, shard key exchange, session establishment.
- **Server-only notes**: Password hashing (SHA1), shard key generation, session token management
- **Path forward**: Working. Consider upgrading SHA1 to bcrypt/scrypt for production.

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| SOAP login endpoint | CW | -- | C++ frontend_connection | HTTP POST /login tested |
| Password validation | CW | -- | C++ logon_connection | SHA1 hash compare |
| Shard list response | CW | -- | C++ service_main | Returns shard name + key |
| Shard key exchange | CW | -- | C++ base_client | Symmetric key for Mercury |
| Session establishment | CW | -- | C++ base_client | BaseApp accepts after auth |
| Duplicate login prevention | IM | -- | Account.py | Only checked at char select, not continuous |
| Client challenge | IM | -- | C++ auth | Post-login only, no runtime validation |
| Developer mode bypass | CW | -- | BaseService.config | Allows duplicate logins, max access |

### 2. Mercury Protocol --- CW

- **Confidence**: HIGH
- **Documentation**: [mercury-wire-format.md](protocol/mercury-wire-format.md), [message-catalog.md](protocol/message-catalog.md)
- **Server code**: Full C++ reliable UDP in `src/lib/mercury/`. Packet framing, reliability, ordering, encryption.
- **Server-only notes**: Nub tick rate (25ms), send/receive buffers (512KB each)
- **Path forward**: Working. Missing cumulative ACKs and piggyback optimization (noted in docs).

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Reliable UDP transport | CW | -- | C++ mercury/ | Sequence numbers, ACK/NAK |
| Packet encryption | CW | -- | C++ mercury/ | Blowfish, key from auth |
| Message framing | CW | -- | C++ mercury/ | Header + variable-length body |
| Ordered delivery | CW | -- | C++ mercury/ | Per-channel sequence |
| Fragmentation | CW | -- | C++ mercury/ | Large message splitting |
| Cumulative ACKs | KM | -- | -- | Documented as missing |
| Piggyback ACKs | KM | -- | -- | Documented as missing |

### 3. Game Data Pipeline --- CW

- **Confidence**: HIGH
- **Documentation**: [cooked-data-pipeline.md](engine/cooked-data-pipeline.md), [game-data.md](game-data.md)
- **Server code**: DefMgr system loads 22 resource categories from PostgreSQL. Client version sync via `versionInfoRequest()`.
- **Server-only notes**: Server loads all definitions at startup; client receives deltas
- **Path forward**: Working. 112,626 DB rows across all resource tables.

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Resource loading from DB | CW | -- | DefMgr, dbQuery | 22 resource categories |
| Client version sync | CW | -- | Account.py | versionInfoRequest() |
| Cooked data (.pak) serving | CW | -- | C++ BaseApp | Binary pak files in data/cache/ |
| Hot reload | KM | -- | -- | No runtime reload capability |

### 4. Database Persistence --- CW

- **Confidence**: HIGH
- **Documentation**: [service-architecture.md](architecture/service-architecture.md)
- **Server code**: SOCI 3.2.1 ORM layer. PostgreSQL 9.2.3. Player state, inventory, missions persisted.
- **Server-only notes**: Entity backup/restore for cross-cell migration
- **Path forward**: Working. Schema in `db/sgw.sql` + `db/resources.sql`.

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Player state persistence | CW | -- | SGWPlayer.persist() | Level, XP, position, stats |
| Inventory persistence | CW | -- | Inventory.py | sgw_inventory table |
| Mission persistence | CW | -- | MissionManager.py | sgw_mission table |
| Entity backup/restore | CW | -- | SGWPlayer.backup() | Cross-cell migration |
| Connection pooling | IM | -- | SOCI | Single connection per service |

---

## Core Gameplay Systems

### 5. Character Creation --- NT

- **Confidence**: HIGH
- **Documentation**: [character-creation.md](gameplay/character-creation.md)
- **Server code**: `python/base/Account.py` ~300 lines. Full creation flow with validation.
- **Server-only notes**: Name uniqueness check, starting equipment assignment, archetype selection
- **Path forward**: Code is fairly complete. Needs end-to-end client test.

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Character list display | NT | -- | Account.py | SELECT from sgw_player |
| Character visual preview | NT | -- | Account.py | Lazy-loads from sgw_inventory |
| Name validation | NT | -- | Account.py | SQL uniqueness check |
| Visual choice validation | NT | -- | Account.py | charDef.getAllChoices() |
| Archetype selection | NT | -- | Account.py | 8 archetypes from resources |
| Starting equipment | NT | -- | Account.py | BagFillOrder insertion |
| Starting abilities | NT | -- | Account.py | From charDef ability list |
| Character deletion | NT | -- | Account.py | CASCADE to inventory, missions |
| GM character creation | NT | -- | Account.py | SGWGmPlayer entity |
| Name filtering | KM | -- | -- | No profanity/reserved name check |
| Character slot limit | KM | -- | -- | No per-account limit |

### 6. World Entry and Spaces --- CW

- **Confidence**: MEDIUM
- **Documentation**: [space-management.md](engine/space-management.md), [connection-flow.md](connection-flow.md)
- **Server code**: C++ space management + 11 Python space scripts. `SGWPlayer.mapLoaded()` is the full world entry sequence.
- **Server-only notes**: Space loading, entity promotion to player, stat sync, ability tree sync
- **Path forward**: 1 zone (P2C-257 Castle) tested end-to-end. Other zones need testing.

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Space loading | CW | -- | C++ space.cpp | NavMesh + entity loading |
| Player entity creation | CW | -- | Account.playCharacter() | Creates SGWPlayer entity |
| Map load sequence | CW | -- | SGWPlayer.mapLoaded() | 30+ client setup calls |
| Stat sync to client | CW | -- | SGWPlayer.mapLoaded() | All stats sent on entry |
| Ability tree sync | CW | -- | SGWPlayer.mapLoaded() | 3 trees per archetype |
| Zone transition | IM | -- | moveToWorld() | Works but no multi-player sync |
| Space scripts | IM | -- | python/cell/spaces/ | 11 scripts, 1 tested |
| Level scripts | IM | -- | python/cell/levels/ | Per-player space logic |

### 7. Movement and Navigation --- IM

- **Confidence**: MEDIUM
- **Documentation**: [position-updates.md](protocol/position-updates.md)
- **Server code**: C++ `PlayerController` (client-driven), `WaypointController` (NPC), NavMesh pathfinding via Recast/Detour.
- **Server-only notes**: No server-side movement validation. Client is authoritative.
- **Path forward**: Player movement works. NPC movement (pathfinding) exists in C++ but Python never calls it.

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Client position updates | CW | -- | C++ PlayerController | playerUpdate() accepts position |
| Dead reckoning | IM | -- | C++ PlayerController | Extrapolates between updates |
| Space bounds check | IM | -- | C++ PlayerController | Validates within space |
| NavMesh loading | IM | -- | C++ navigation.cpp | Loads .nav files |
| NPC pathfinding | IM | -- | C++ navigation.cpp | findPathTo(), findDetailedPathTo() |
| NPC waypoint movement | IM | -- | C++ WaypointController | addWaypoint(), cancelMovement() |
| NPC patrol | KM | Pathfinding | C++ startPatrol() | Method exists, never called from Python |
| Speed validation | KM | -- | -- | No server-side speed check |
| Teleport detection | KM | -- | -- | No anomaly detection |

### 8. Entity Lifecycle (AoI) --- CW

- **Confidence**: MEDIUM
- **Documentation**: [entity-lod-system.md](engine/entity-lod-system.md), [entity-type-catalog.md](engine/entity-type-catalog.md)
- **Server code**: C++ `CellEntity`, grid-based Area of Interest. `grid_chunk_size=50`, `grid_vision_distance=3`, `grid_hysteresis=1`.
- **Server-only notes**: Entity enters/leaves witness sets based on grid position
- **Path forward**: Core AoI works. No LOD (level of detail) system.

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Entity creation | CW | -- | C++ cell_entity.cpp | From template or dynamic |
| Entity destruction | CW | -- | C++ cell_entity.cpp | Cleanup and notification |
| Grid-based AoI | CW | -- | C++ grid system | Chunk-based witness management |
| Witness enter/leave | CW | -- | C++ cell_entity.cpp | onWitnessEnter/Leave events |
| Entity sync to client | CW | -- | C++ cell_entity.cpp | Property + position updates |
| LOD system | KM | -- | -- | No entity detail levels |
| Entity pooling | KM | -- | -- | No reuse of entity objects |

### 9. Combat and Abilities --- IM

- **Confidence**: HIGH for basics, LOW for edge cases
- **Documentation**: [combat-system.md](gameplay/combat-system.md), [ability-system.md](gameplay/ability-system.md), [combat-wire-formats.md](reverse-engineering/findings/combat-wire-formats.md)
- **Server code**: `AbilityManager.py` 1,090 lines. Complete QR/damage pipeline, effect application, cooldowns.
- **Server-only notes**: DamageCalc algorithm, stat resistance formulas, armor factor computation
- **Path forward**: Single-target combat works. AoE, deploy, and prerequisites need implementation.

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| QR calculation | IM | -- | DamageCalc.calculateQR() | Beta distribution, stat-based |
| Damage calculation | IM | -- | DamageCalc.calculateDamage() | Resist -> AF -> absorb pipeline |
| Stat resistance | IM | -- | calculateStatResistance() | fort/intel/engage based |
| Armor factor | IM | -- | calculateArmorFactor() | Per-damage-type AF stats |
| Absorption | IM | -- | calculateAbsorbedDamage() | 15 absorption stats |
| Single-target abilities | IM | -- | AbilityInstance.launch() | TCM_Single only |
| Ability warmup | IM | -- | AbilityInstance.launch() | Speed modifiers applied |
| Ability cooldowns | IM | -- | AbilityManager | Per-ability + per-moniker |
| Auto-cycle | IM | -- | AbilityManager | Re-fire on cooldown complete |
| Position/facing checks | IM | -- | AbilityInstance.canUse() | Front/flank/rear/above/below |
| Weapon range checks | IM | -- | AbilityInstance.canUse() | Min/max range validation |
| Ammo consumption | IM | -- | AbilityInstance.afterWarmup() | Decrements ammo stat |
| AoE abilities (radius) | KM | -- | -- | TCM_AERadius returns Deprecated |
| AoE abilities (cone) | KM | -- | -- | TCM_AECone returns Deprecated |
| Group targeting | KM | Groups | -- | TCM_Group not implemented |
| Aura targeting | KM | -- | -- | TCM_Aura not implemented |
| Prerequisite monikers | KM | -- | Loaded, not checked | canUseWithMonikers() exists but uncalled |
| LOS checks | KM | -- | -- | No line-of-sight validation |
| Deploy abilities | IM | -- | Flags handled | Warmup speed applied, full logic unclear |
| Health regen tick | KM | -- | -- | healthRegen stat exists, no tick |
| Focus regen tick | KM | -- | -- | focusRegen stat exists, no tick |

### 10. Effects and Buffs --- IM

- **Confidence**: HIGH for framework, NONE for content coverage
- **Documentation**: [effect-system.md](gameplay/effect-system.md)
- **Server code**: Effect management within `AbilityManager.py`. `EffectInstance` handles init/pulse/remove cycle. 4 of 3,217 effects have scripts.
- **Server-only notes**: Stat change tracking with auto-revert on effect removal. Shared QR within same pulse.
- **Path forward**: Framework is solid. Need `tools/generate_effect_stubs.py` to create 3,213 missing scripts.

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Effect application | IM | -- | AbilityManager.addEffect() | Auto-replace same effect |
| Effect pulse/tick | IM | -- | EffectInstance.pulse() | Timer-driven, skip if dead |
| Effect removal | IM | -- | EffectInstance.remove() | Reverts non-permanent stat changes |
| Stat change tracking | IM | -- | EffectInstance.changeStat() | Permanent vs non-permanent |
| Shared QR per pulse | IM | -- | EffectInstance.qrCombatDamage() | One roll shared across effects |
| Clear on death | IM | -- | EF_ClearOnDeath | Implemented |
| Clear on damage | IM | -- | EF_ClearOnDamage | Implemented |
| Clear on revive | IM | -- | EF_ClearOnRez | Implemented |
| Clear on bandolier swap | IM | -- | EF_RemoveOnBandolierSlotChange | Implemented |
| Effect scripts (content) | KM | -- | 4 of 3,217 | RangedEnergyDamage, RangedPhysicalDamage, Reload, TestEffect |
| Effect persistence | KM | -- | -- | EF_AlwaysPersist flag exists, not implemented |
| Channeled effects | KM | -- | -- | is_channeled column exists, no handler |
| Stealth-related flags | KM | -- | -- | EF_RemoveOnStealthZeroed etc. unhandled |

### 11. Stats --- IM

- **Confidence**: MEDIUM for infrastructure, LOW for formulas
- **Documentation**: [stat-system.md](gameplay/stat-system.md), [progression-system.md](gameplay/progression-system.md)
- **Server code**: `Stat` class in `SGWBeing.py`. 60+ stats with dirty sync. Template-based initialization.
- **Server-only notes**: Public stats broadcast to witnesses, private stats to owning client only
- **Path forward**: Stat infrastructure works. Missing: derived formulas, per-level growth, diminishing returns.

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Stat class (min/cur/max) | IM | -- | SGWBeing.Stat | 6 values + dirty flags |
| Dirty stat sync | IM | -- | sendDirtyStats() | Incremental updates |
| Public/private split | IM | -- | publicStats list | 11 public, rest private |
| Archetype base stats | IM | -- | setupPlayer() | Applied on first load |
| Per-level stat growth | KM | Leveling | -- | healthPerLevel/focusPerLevel loaded but unused |
| Derived stat formulas | KM | -- | -- | No stat derivation system |
| Stat soft caps | NU | -- | -- | No diminishing returns |
| Item stat bonuses | KM | Inventory | -- | Equipment doesn't modify stats |

### 12. Inventory and Items --- NT

- **Confidence**: HIGH
- **Documentation**: [inventory-system.md](gameplay/inventory-system.md), [inventory-wire-formats.md](reverse-engineering/findings/inventory-wire-formats.md)
- **Server code**: `Inventory.py` ~21K lines, `Bag.py`, `Item.py`. Complete bag/slot system with 20 bag types.
- **Server-only notes**: Temp ID management (1-9999), visual dirty tracking, bandolier slot management
- **Path forward**: Looks complete. Needs end-to-end test with client.

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Bag system (20 types) | NT | -- | Inventory.py, Bag.py | Main, mission, equipment, bank, etc. |
| Item add/remove | NT | -- | Inventory.py | pickedUpItem(), removeItem() |
| Item stacking | NT | -- | Inventory.py | Stack size management |
| Equipment slots | NT | -- | Inventory.py | Head through Artifact2 |
| Bandolier (4 weapon sets) | NT | -- | Inventory.py | Active slot switching |
| Buyback bag | NT | -- | Inventory.py | 12-slot vendor buyback |
| Cash (naquadah) | NT | -- | Inventory.py | addCash(), removeCash() |
| Visual sync | NT | -- | Inventory.py | Equipment visual updates |
| DB persistence | CW | -- | SGWPlayer.persist() | sgw_inventory table |
| Item durability | KM | -- | Column exists | No wear/break mechanics |
| Item binding | KM | -- | Column exists | bound column unused |

### 13. Missions --- IM

- **Confidence**: HIGH for framework, LOW for content coverage
- **Documentation**: [mission-system.md](gameplay/mission-system.md), [mission-wire-formats.md](reverse-engineering/findings/mission-wire-formats.md)
- **Server code**: `MissionManager.py` ~29K lines. `MissionInstance` class. 20 mission scripts in `python/cell/missions/`.
- **Server-only notes**: Step/objective tracking, reward distribution, space event integration
- **Path forward**: Framework tested in Castle zone. Need more zone scripts.

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Mission accept | IM | -- | MissionManager.py | From NPC dialog |
| Mission tracking | IM | -- | MissionInstance | Steps, objectives, status |
| Objective completion | IM | -- | MissionManager.py | completeObjective() |
| Step advancement | IM | -- | MissionManager.py | advance() |
| Mission completion | IM | -- | MissionManager.py | Rewards XP + naquadah |
| Mission failure | IM | -- | MissionManager.py | failObjective() |
| Mission abandon | IM | -- | MissionManager.py | abandon() |
| DB persistence | CW | -- | sgw_mission table | Status, steps, objectives |
| Mission scripts | IM | -- | 20 scripts | Castle zone tested |
| Mission sharing | KM | Groups | pass | shareMission() is stub |
| Repeatable missions | IM | -- | MissionInstance.repeats | Column exists, logic unclear |
| Mission gated loot | KM | Loot | -- | TODO in Lootable.py |

### 14. Loot --- NT

- **Confidence**: HIGH for code, LOW for content
- **Documentation**: [loot-system.md](gameplay/loot-system.md)
- **Server code**: `Lootable.py` 221 lines. Complete loot generation algorithm. Interaction handler for looting.
- **Server-only notes**: Independent probability roll per entry (not "pick one"). Cash vs item drops.
- **Path forward**: Code works. Loot tables need content population.

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Loot table definitions | NT | -- | resources.loot_tables | DB schema complete |
| Loot generation algorithm | NT | -- | Lootable.randomizeLoot() | Per-item probability roll |
| Item drops | NT | -- | Lootable.onLootItem() | Transfers to inventory |
| Cash drops | NT | -- | Lootable.onLootItem() | addCash() for NULL design_id |
| Loot window display | NT | -- | Lootable.sendLootList() | Sends to client |
| Per-player eligibility | IM | -- | LootableItem.eligiblePlayerList | List exists, group assignment unwired |
| Group loot modes | KM | Groups | -- | RoundRobin/FreeForAll enums, no logic |
| Mission-gated loot | KM | Missions | -- | TODO: missionId filtering |
| Loot table content | KM | -- | -- | Tables mostly empty |

### 15. Stores / Vendors --- NT

- **Confidence**: HIGH
- **Documentation**: [inventory-system.md](gameplay/inventory-system.md)
- **Server code**: `python/cell/interactions/Vendor.py`. Complete buy/sell/repair/recharge/buyback flow.
- **Server-only notes**: Price validation, stock from DB item lists, buyback bag (12 slots)
- **Path forward**: Code complete. Needs client test.

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Buy items | NT | -- | Vendor.onBuyItem() | Validates cash, creates item |
| Sell items | NT | -- | Vendor.onSellItem() | Validates ownership, adds cash |
| Repair items | NT | -- | Vendor.onRepairItem() | Cost calculation |
| Recharge items | NT | -- | Vendor.onRechargeItem() | Ammo recharge |
| Buyback | NT | -- | Vendor.onBuybackItem() | 12-slot buyback bag |
| Vendor stock from DB | NT | -- | entity_templates | buy/sell/repair item lists |
| Transactional safety | KM | -- | -- | TODO: Make transactional |

---

## NPC Systems

### 16. NPC AI and Behavior --- IM

- **Confidence**: MEDIUM for combat AI, NOT STARTED for world behavior
- **Documentation**: [npc-ai.md](gameplay/npc-ai.md)
- **Server code**: `SGWMob.py` 397 lines. 2 of 12 AI states implemented. Threat system, ability selection work.
- **Server-only notes**: Entire AI runs server-side. Client only sees state transitions and ability use.
- **Path forward**: Combat AI loop works. Need patrol, wander, leash, investigation states.

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| AI state machine | IM | -- | SGWMob.doAiAction() | 12 states defined, 2 dispatched |
| Spawning state | IM | -- | doAiSpawnAction() | Loads ammo, transitions to Idle |
| Idle state | IM | -- | doAiAction() | Waits for threat |
| Fighting state | IM | -- | doAiFightingAction() | Target + ability + fire loop |
| Threat accumulation | IM | -- | threatGenerated() | -healthChange*2 - focusChange |
| Top-threat targeting | IM | -- | getTopThreateningEntity() | Linear scan, dead pruning |
| Ability classification | IM | -- | classifyHostileAbility() | Type, TCM, cooldown, ammo |
| Ability selection | IM | -- | selectHostileAbility() | First usable from set |
| Auto-reload | IM | -- | selectHostileAbility() | Triggers reload ability |
| Loot on death | IM | -- | SGWMob.onDead() | Generates loot, sets interaction |
| Aggression override | IM | -- | setAggression() | Per-instance + timed |
| Investigating state | KM | Navigation | -- | POI, investigateTimerID defined |
| Leashing state | KM | Navigation | -- | Home property, no logic |
| Patrol state | KM | Navigation | -- | patrolPaths defined, startPatrol() in C++ |
| Wander state | KM | Navigation | -- | Home + nextWanderTime defined |
| Follow state | KM | Navigation | -- | followTarget + min/max distance defined |
| Despawning state | KM | Spawning | -- | despawnFlag + DespawnWhenFree() defined |
| Error state | KM | -- | -- | enterErrorAIState() defined |
| Proactive aggro | KM | -- | -- | No detection radius, only reacts to damage |
| Navigation to target | KM | Navigation | -- | navControllerID defined, never created |
| Cover system | KM | Navigation | -- | useCover, reservedCoverNode defined |
| Hearing system | KM | -- | -- | hearingRadius, onNoise() defined |
| Behavior events | KM | -- | -- | mobBehaviorEventSet defined |
| Mob groups | KM | -- | -- | mobGroup, mobJoinGroup() defined |
| Tapping / kill credit | KM | -- | -- | tappedEntity/Squad defined, unused |
| XP on kill | KM | Progression | -- | isWorthXP flag defined, no giveExperience() call |

### 17. Spawn System --- KM

- **Confidence**: NOT STARTED
- **Documentation**: [spawn-system.md](gameplay/spawn-system.md)
- **Server code**: `SGWSpawnRegion.py` + `SGWSpawnSet.py` = empty stubs (only `__init__`). Rich .def files define all properties and methods.
- **Server-only notes**: Entirely server-side population management. Client never sees spawn regions.
- **Path forward**: Reconstruct from .def properties. Property names clearly indicate design intent.

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| SpawnRegion entity | KM | -- | Empty stub | .def has 26 properties |
| SpawnSet entity | KM | -- | Empty stub | .def has 22 properties |
| Region activation | KM | -- | -- | Activated flag, ActivateMe() method |
| Set activation | KM | Region | -- | ActivateMe()/DeactivateMe() methods |
| Mob spawning | KM | Set activation | -- | MySpawnPoints, entity creation |
| Mob registration | KM | Spawning | -- | RegisterMobBase() method |
| Population tracking | KM | Spawning | -- | CurrentPopulation, reportPopulation() |
| Mob death notification | KM | NPC AI | -- | BeingDeath(), onMobDeath() methods |
| Respawn timers | KM | Death notification | -- | Min/MaxRespawnSeconds, StartRespawnTimer() |
| Set cooldowns | KM | Timers | -- | min/maxCooldownSeconds properties |
| Max active sets | KM | Activation | -- | MaxActiveSets property |
| Spawn tables (weighted) | KM | -- | -- | spawnTableIDs = [(id, weight)] |
| Spawn point randomization | KM | -- | -- | bRandomizeSpawnPoints flag |
| Level range filtering | KM | -- | -- | minMOBLevel, maxMOBLevel |
| Player detection | KM | -- | -- | detectionRadius, detectionController |
| Time-of-day spawns | KM | -- | -- | onTimeOfDayTick() method |
| Mission integration | KM | Missions | -- | missionData, entityMissionStepUpdate() |
| Linked sets | KM | -- | -- | bLinked flag, semantics unknown |
| Population scaling | NU | -- | -- | timerReduction suggests dynamic spawn rates |

---

## Secondary Gameplay Systems

### 18. XP and Leveling --- IM

- **Confidence**: HIGH (Rust implementation with Python-accurate XP table)
- **Documentation**: [progression-system.md](gameplay/progression-system.md), [xp-leveling-design.md](plans/2026-03-08-xp-leveling-design.md)
- **Server code**: Python: `giveExperience()` in `SGWPlayer.py`. Rust: `PlayerState::grant_xp()` in `crates/game/src/player.rs`, `handle_grant_xp()` in `crates/services/src/base/world_entry.rs`.
- **Server-only notes**: XP accumulation and level-up are purely server-side calculations. Kill XP flows Cell→Base via `CellToBaseMsg::GrantXP`.
- **Path forward**: Core XP/leveling pipeline complete. Needs mission XP integration, ASP grants, and DB persistence wiring.

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| XP accumulation | IM | -- | PlayerState::grant_xp() | Additive, sends onExpUpdate (Rust) |
| Level-up detection | IM | -- | while loop | Multi-level-up supported, 16 tests |
| Client notification | IM | -- | handle_grant_xp() | 5 wire messages: onExpUpdate, giveXPForLevel, onMaxExpUpdate, onLevelUpdate, onEntityProperty(TP) |
| XP from missions | IM | Missions | mission.rewardXp | Python path only |
| Level cap (20) | IM | -- | MAX_LEVEL | Enforced in grant_xp() |
| DB persistence | IM | -- | sgw_player | level + exp columns (Python path) |
| XP from mob kills | IM | -- | kill_xp() | 10 × mob_level, Cell→Base pipeline |
| XP curve | IM | -- | LEVEL_XP[21] | Ported from Python Constants.LEVEL_EXP |
| Stat scaling on level-up | IM | -- | StatList::scale_for_level() | health_per_level/focus_per_level, full heal on level-up |
| Training points on level-up | IM | -- | TRAINING_POINTS_PER_LEVEL | 2 TP per level, 38 total by level 20 |
| ASP on level-up | KM | -- | -- | No ASP grant on level |

### 19. Crafting --- NT

- **Confidence**: MEDIUM
- **Documentation**: [crafting-system.md](gameplay/crafting-system.md), [crafting-wire-formats.md](reverse-engineering/findings/crafting-wire-formats.md)
- **Server code**: `Crafter.py` 575 lines. ~95% implemented. Craft, research, reverse engineer, alloy.
- **Server-only notes**: 3-second crafting timer, expertise gain, racial paradigm gating
- **Path forward**: Code looks complete. Needs client test.

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Craft from blueprint | NT | Inventory | Crafter.craft() | Validates, consumes, 3s timer |
| Research item | NT | Inventory | Crafter.research() | Expertise chance roll |
| Reverse engineer | NT | Inventory | Crafter.reverseEngineer() | Random blueprint + components |
| Alloy | NT | Inventory | Crafter.alloy() | Tier + elementary components |
| Discipline learning | NT | -- | spendAppliedSciencePoints() | 1 ASP, prereq check |
| Expertise system (0-100) | NT | -- | gainExpertise() | Capped at 100 |
| Racial paradigm gating | NT | -- | Crafter | Paradigm level prereqs |
| Blueprint management | NT | -- | giveBlueprints() | Deduplicated list |
| Crafting respec | KM | -- | trace() only | respecCrafting() is stub |

### 20. Stargate Travel --- IM

- **Confidence**: MEDIUM
- **Documentation**: [gate-travel.md](gameplay/gate-travel.md), [gate-travel-wire-formats.md](reverse-engineering/findings/gate-travel-wire-formats.md)
- **Server code**: ~90 lines in `SGWPlayer.py`. DHD interaction in `interactions/DHD.py`.
- **Server-only notes**: Dial timer (4 seconds), gate state machine, zone transition
- **Path forward**: Single-player gate travel works. Multi-player sync missing.

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| DHD interaction | IM | -- | DHD.py | Opens DHD with origin address |
| Gate dialing | IM | -- | beginDialing() | 4-second timer |
| Gate cancel | IM | -- | cancelDialing() | Timer cancel + event |
| Gate passage | IM | World Entry | stargatePassed() | moveTo() destination |
| Known address tracking | IM | -- | addStargateAddress() | Known + hidden lists |
| Address discovery | IM | -- | addStargateAddress() | Added by missions/exploration |
| Multi-player gate sync | KM | AoI | -- | Other players don't see gate events |
| Return trips | KM | -- | -- | No bidirectional gate state |
| Gate cooldown | KM | -- | -- | No use-after-dial cooldown |

### 21. Chat --- NT

- **Confidence**: MEDIUM for messaging, STUB for moderation
- **Documentation**: [chat-system.md](gameplay/chat-system.md), [chat-wire-formats.md](reverse-engineering/findings/chat-wire-formats.md)
- **Server code**: `Chat.py` 352 lines. Channel system with join/leave/speak. Direct tells.
- **Server-only notes**: In-memory channel state. Cell channels (say/emote/yell) use AoI broadcast.
- **Path forward**: Core messaging works. Admin/moderation tools are stubs.

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Channel system | NT | -- | ChatChannelManager | Create, join, leave, delete |
| Say/emote/yell (AoI) | NT | AoI | processPlayerCommunication | Witness broadcast |
| Direct tells | NT | -- | sendPlayerMessage() | By player name |
| User channels | NT | -- | requestCreateChannel() | Password-protected, op system |
| Pre-defined channels | NT | -- | Chat.py | team, squad, command, officer, server |
| Channel ops | NT | -- | setPlayerOp() | Creator is auto-op |
| Chat flood protection | KM | -- | -- | No rate limiting |
| Profanity filter | KM | -- | -- | No filtering |
| Mute system | KM | -- | -- | No per-player muting |
| GM broadcast | KM | Admin | -- | No system-wide message tool |

### 22. Trading --- NT

- **Confidence**: MEDIUM
- **Documentation**: [trade-system.md](gameplay/trade-system.md), [trade-wire-formats.md](reverse-engineering/findings/trade-wire-formats.md)
- **Server code**: `Trade.py` 244 lines. Full P2P trading: propose, lock, confirm, execute.
- **Server-only notes**: Version-sequenced proposals, atomic item/cash swap, inventory space validation
- **Path forward**: Code complete. Needs client test.

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Trade initiation | NT | -- | tradeRequest() | By entity ID |
| Proposal update | NT | -- | updateProposal() | Version-sequenced |
| Lock state machine | NT | -- | updateLockState() | None -> Locked -> LockedAndConfirmed |
| Confirmation | NT | -- | confirm() | Validates both proposals |
| Item swap | NT | Inventory | confirm() | Atomic transfer |
| Cash swap | NT | -- | confirm() | Validates balances |
| Cancel | NT | -- | cancel() | Either party can cancel |
| Trade spam protection | KM | -- | -- | No request throttle |

---

## Stub-Only Systems

### 23. Organizations / Guilds --- KM

- **Confidence**: STUB
- **Documentation**: [organization-system.md](gameplay/organization-system.md), [organization-wire-formats.md](reverse-engineering/findings/organization-wire-formats.md)
- **Server code**: 13 methods in `SGWPlayer.py`, all `pass`. `SGWPlayerGroupAuthority.py` is empty.
- **Server-only notes**: Rich enum support: 3 org types (Squad/Team/Command), 9 ranks, 26 permissions
- **Path forward**: Implement from .def + enum definitions. DB schema needed.

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Organization creation | KM | DB schema | pass | onOrganizationCreation() |
| Invite/accept | KM | Creation | pass | organizationInviteResponse() |
| Leave organization | KM | -- | pass | organizationLeave() |
| Rank system (9 ranks) | KM | Creation | -- | EORG_RANK_None through Leader |
| Permission system (26 perms) | KM | Ranks | -- | Full bitmask defined in enums |
| MOTD | KM | Creation | pass | organizationMOTD() |
| Officer notes | KM | Ranks | pass | organizationOfficerNote() |
| Rank name customization | KM | Ranks | pass | organizationSetRankName() |
| Permission editing | KM | Ranks | pass | organizationSetRankPermissions() |
| Cash transfer to bank | KM | Creation | pass | organizationTransferCash() |
| Organization vault | KM | Creation, Inventory | -- | INV_TeamBank(19), INV_CommandBank(20) |
| Squad loot mode | KM | Groups | pass | squadSetLootMode() |
| Minimap ping | KM | -- | pass | BroadcastMinimapPing() |
| Strike teams | KM | -- | pass | strikeTeamResponse() |
| PvP org leave | KM | -- | pass | pvpOrganizationLeaveResponse() |

### 24. Mail --- KM

- **Confidence**: STUB (read-only partial)
- **Documentation**: [mail-system.md](gameplay/mail-system.md), [mail-wire-formats.md](reverse-engineering/findings/mail-wire-formats.md)
- **Server code**: 9 methods in `SGWPlayer.py`. 2 read-only implemented, 7 stubs (`pass`/`print`).
- **Server-only notes**: `sgw_gate_mail` table exists with full schema
- **Path forward**: DB schema exists. Implement CRUD operations.

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Open mailbox (headers) | IM | -- | requestMailHeaders() | Queries sgw_gate_mail |
| Read mail body | IM | -- | requestMailBody() | Queries + debug print |
| Send mail | KM | DB | pass | sendMailMessage() has full signature |
| Delete mail | KM | -- | print only | deleteMailMessage() |
| Archive mail | KM | -- | print only | archiveMailMessage() |
| Attach item | KM | Send, Inventory | -- | itemId param in sendMailMessage() |
| Attach gold | KM | Send | -- | cash param in sendMailMessage() |
| Cash on Delivery | KM | Send, Receive | -- | bCOD param in sendMailMessage() |
| Take item from mail | KM | Inventory | print only | takeItemFromMailMessage() |
| Take cash from mail | KM | -- | print only | takeCashFromMailMessage() |
| Return to sender | KM | Send | print only | returnMailMessage() |
| New mail notification | KM | Send | -- | No event triggered |
| Mail expiry/TTL | NU | DB | -- | No TTL in schema |

### 25. Black Market (Auction House) --- KM

- **Confidence**: STUB
- **Documentation**: [black-market.md](gameplay/black-market.md), [black-market-wire-formats.md](reverse-engineering/findings/black-market-wire-formats.md)
- **Server code**: `SGWBlackMarket.py` is empty shell. All handler methods are stubs.
- **Server-only notes**: Auction bag slot (INV_Auction=18), 5 duration tiers
- **Path forward**: Implement with new DB table for listings.

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Search listings | KM | DB schema | -- | Search sort type enums defined |
| Create listing | KM | DB, Inventory | -- | Duration tiers: VeryShort..VeryLong |
| Place bid | KM | Listing, Economy | -- | Bid tracking needed |
| Buyout | KM | Listing, Economy | -- | Instant purchase |
| Cancel listing | KM | Listing | -- | Return item to seller |
| View my auctions | KM | Listing | -- | MyAuctions enum |
| View my bids | KM | Listing | -- | MyBids enum |
| Auction expiry | KM | Scheduler | -- | Timer-based cleanup |
| Listing fees | NU | Economy | -- | Standard MMO pattern |
| Transaction mail | KM | Mail | -- | Results delivered via mail |

### 26. Contact Lists --- KM

- **Confidence**: STUB
- **Documentation**: [contact-list.md](gameplay/contact-list.md), [contact-list-wire-formats.md](reverse-engineering/findings/contact-list-wire-formats.md)
- **Server code**: 6 methods in `SGWPlayer.py`, all `pass`.
- **Server-only notes**: Event flags: LoggedInStatus, GainLevel, Death, GateTravel
- **Path forward**: Implement with new DB table.

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Create list | KM | DB schema | pass | contactListCreate(name, flags) |
| Delete list | KM | -- | pass | contactListDelete(listId) |
| Rename list | KM | -- | pass | contactListRename(listId, name) |
| Update flags | KM | -- | pass | contactListFlagsUpdate(listId, flags) |
| Add members | KM | -- | pass | contactListAddMembers(listId, names) |
| Remove members | KM | -- | pass | contactListRemoveMembers(listId, names) |
| Online status events | KM | Session | -- | ECONTACT_LIST_EVENT_LoggedInStatus |
| Level-up events | KM | Progression | -- | ECONTACT_LIST_EVENT_GainLevel |

### 27. Dueling --- KM

- **Confidence**: STUB
- **Documentation**: [duel-system.md](gameplay/duel-system.md), [duel-wire-formats.md](reverse-engineering/findings/duel-wire-formats.md)
- **Server code**: 2 methods in `SGWPlayer.py`, all `pass`. `SGWDuelMarker.py` is empty.
- **Server-only notes**: 5-state machine (None/ResponsePending/Challenged/StartPending/Engaged), 7 defeat conditions
- **Path forward**: Implement state machine from enum definitions.

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Duel challenge | KM | -- | -- | State: ResponsePending |
| Duel response | KM | -- | pass | sendDuelResponse() |
| Duel start | KM | Combat | -- | State: StartPending -> Engaged |
| Duel forfeit | KM | -- | pass | duelForfeit() |
| Defeat conditions | KM | Combat | -- | 7 types: health, left squad, disconnect, range, etc. |
| Duel marker entity | KM | -- | Empty shell | SGWDuelMarker.py |

### 28. Pets --- KM

- **Confidence**: STUB
- **Documentation**: [pet-system.md](gameplay/pet-system.md), [pet-wire-formats.md](reverse-engineering/findings/pet-wire-formats.md)
- **Server code**: 3 methods in `SGWPlayer.py`, all `pass`. `SGWPet.py` minimal (sends ability/stance lists).
- **Server-only notes**: Pet extends SGWMob (inherits AI). speedPet stat exists.
- **Path forward**: Pet entity needs AI, command handling, following behavior.

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Pet ability list sync | IM | -- | SGWPet.createOnClient() | Sends to client |
| Pet stance list sync | IM | -- | SGWPet.createOnClient() | Sends to client |
| Invoke pet ability | KM | Combat | pass | petInvokeAbility() |
| Toggle pet ability | KM | -- | pass | petAbilityToggle() |
| Change pet stance | KM | -- | pass | petChangeStance() |
| Pet following | KM | NPC AI (Follow) | -- | Needs Follow AI state |
| Pet combat AI | KM | NPC AI | -- | Inherits SGWMob |

### 29. Minigames --- KM

- **Confidence**: STUB (framework exists)
- **Documentation**: [minigame-system.md](gameplay/minigame-system.md), [minigame-wire-formats.md](reverse-engineering/findings/minigame-wire-formats.md)
- **Server code**: `Minigame.py` 59 lines (request object with callback). 10 game types defined. External game server on port 30000.
- **Server-only notes**: Minigame server is separate HTTP service. Client connects directly.
- **Path forward**: Minigame logic runs externally (Flash/web). Server just manages sessions.

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Minigame session start | IM | -- | Minigame.play() | Sends to client |
| Seed generation | IM | -- | Minigame.setSeed() | 0..0x7FFFFFFF |
| Result callback | IM | -- | onMinigameResult() | Victory/Defeat/Canceled |
| Mission integration | IM | Missions | Castle scripts | HackTheRings, Prisoner_329, etc. |
| 10 game types | KM | External server | Config.MINIGAME_NAMES | Activate through Alignment |
| Spectating | KM | -- | pass | requestSpectateList(), spectateMinigame() |
| Co-op / help | KM | -- | pass | registerToMinigameHelp() |
| Contact system | KM | -- | pass | minigameContactRequest() |
| External server | KM | Infrastructure | -- | Port 30000, separate process |

### 30. Groups / Parties --- KM

- **Confidence**: STUB
- **Documentation**: [group-system.md](gameplay/group-system.md), [group-wire-formats.md](reverse-engineering/findings/group-wire-formats.md)
- **Server code**: `SGWPlayerGroupAuthority.py` empty shell. Some base tracking in Organization enums.
- **Server-only notes**: Squad = org type 0. Member info: position, space, level, health, focus, energy, name, archetype, world.
- **Path forward**: Implement as lightweight version of organization system (Squad type).

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Group creation | KM | -- | -- | EORG_TYPE_Squad = 0 |
| Group invite | KM | -- | -- | Part of organization system |
| Group leave | KM | -- | -- | REAS_requested/kicked/disbanded/logout |
| Member info sync | KM | -- | -- | 9 EMEMBER_INFO types defined |
| Loot mode setting | KM | Loot | pass | squadSetLootMode() |
| Group combat assist | KM | Combat, NPC AI | -- | onGroupMateEnteredCombat() in SGWMob |
| Threat transfer | KM | NPC AI | -- | onGroupMateThreatTransfer() in SGWMob |

---

## Server Infrastructure Gaps

### Session Management --- IM

- **Documentation**: [server-systems.md](architecture/server-systems.md)
- **Priority**: HIGH

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Inactivity timeout | IM | -- | BaseService.config | 300,000ms (5 min) |
| Duplicate login check | IM | -- | Account.py | At char select only |
| Developer mode bypass | IM | -- | BaseService.config | Disables duplicate check |
| Reconnection grace period | KM | -- | -- | Instant disconnect = session lost |
| Session token persistence | KM | -- | -- | No resume after network blip |
| Continuous auth validation | KM | -- | -- | Only checked at login |

### Rate Limiting --- KM

- **Documentation**: [server-systems.md](architecture/server-systems.md)
- **Priority**: MEDIUM

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Ability cooldown enforcement | IM | -- | AbilityManager | Per-ability timers |
| Chat flood protection | KM | -- | -- | No rate limit on messages |
| Action throttling | KM | -- | -- | No per-action rate tracking |
| Trade request spam | KM | -- | -- | No request cooldown |
| Login attempt limiting | KM | -- | -- | No brute-force protection |

### Anti-Cheat Validation --- KM

- **Documentation**: [server-systems.md](architecture/server-systems.md)
- **Priority**: MEDIUM

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Position bounds check | IM | -- | C++ PlayerController | Within space only |
| Ability target validation | IM | -- | AbilityInstance.canUse() | Target exists + alive |
| Inventory ownership check | IM | -- | Trade.py, Vendor.py | Validates item ownership |
| Speed hack detection | KM | -- | -- | Client-authoritative movement |
| Teleport detection | KM | -- | -- | No position delta tracking |
| Damage sanity check | KM | -- | -- | No max-damage cap |
| Action-at-distance exploit | KM | -- | -- | Can fire from any range if client bypassed |

### Economy Sinks / Faucets --- IM

- **Documentation**: [server-systems.md](architecture/server-systems.md)
- **Priority**: LOW

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Vendor buy/sell prices | IM | -- | Vendor.py | Static from DB |
| Mission cash rewards | IM | Missions | MissionManager | mission.rewardNaq |
| Loot cash drops | IM | Loot | Lootable.py | LOOT_Cash type |
| Repair costs | KM | -- | -- | No cost formula |
| AH listing fees | KM | Black Market | -- | Standard MMO sink |
| Cash flow tracking | KM | -- | -- | No monitoring |

### World State Persistence --- KM

- **Documentation**: [server-systems.md](architecture/server-systems.md)
- **Priority**: MEDIUM

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Player position persistence | CW | -- | sgw_player | pos_x/y/z columns |
| Space scripts | IM | -- | python/cell/spaces/ | Reset on restart |
| Gate state persistence | KM | DB | -- | Open/closed not saved |
| Door state persistence | KM | DB | -- | Not saved |
| World state table | KM | DB | -- | No sgw_world_state table |

### Event / Scheduler System --- KM

- **Documentation**: [server-systems.md](architecture/server-systems.md)
- **Priority**: LOW

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Per-entity timers | IM | -- | Atrea.addTimer() | Python timers work |
| Global event scheduler | KM | -- | -- | No cron-like system |
| Daily resets | KM | Scheduler | -- | No daily mission/vendor reset |
| Holiday events | KM | Scheduler | -- | No seasonal content system |

### Admin / GM Tools --- KM

- **Documentation**: [server-systems.md](architecture/server-systems.md)
- **Priority**: HIGH

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Python console | IM | -- | Port 8989 | Disabled (empty password) |
| Access level system | IM | -- | account.accesslevel | DB column exists |
| GM player entity | IM | -- | SGWGmPlayer.py | Subclass with elevated access |
| Console commands | IM | -- | ConsoleCommands.py | Framework exists |
| Player info lookup | KM | Console | -- | 22 GM methods are stubs |
| Ban/mute system | KM | Console | -- | No implementation |
| Teleport command | KM | Console | -- | Stub |
| Item grant | KM | Console | -- | Stub |
| Action logging | KM | DB | -- | No GM log table |
| Announcement broadcast | KM | Chat | -- | No system message |

### Metrics / Telemetry --- KM

- **Documentation**: [server-systems.md](architecture/server-systems.md)
- **Priority**: LOW

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Category logging | IM | -- | Logger.py | LC_Player, LC_Combat, etc. |
| Player count tracking | KM | -- | -- | No counter |
| Area population | KM | -- | -- | No monitoring |
| Gameplay metrics | KM | -- | -- | No event tracking |
| Performance metrics | KM | -- | -- | No tick/entity counters |

---

## Cross-Reference Tables

### Documentation Exists but Code Doesn't

These systems have gameplay docs and wire format docs but their server implementations are stubs.

| System | Gameplay Doc | Wire Format Doc | Server Code Status |
|--------|-------------|----------------|-------------------|
| Organizations | organization-system.md | organization-wire-formats.md | 13 methods, all `pass` |
| Mail | mail-system.md | mail-wire-formats.md | 2 read-only, 7 stubs |
| Black Market | black-market.md | black-market-wire-formats.md | Empty shell |
| Contact Lists | contact-list.md | contact-list-wire-formats.md | 6 methods, all `pass` |
| Dueling | duel-system.md | duel-wire-formats.md | 2 methods, all `pass` |
| Pets | pet-system.md | pet-wire-formats.md | 3 methods, all `pass` |
| Groups | group-system.md | group-wire-formats.md | Empty shell |
| Minigames | minigame-system.md | minigame-wire-formats.md | Framework only, handlers empty |

### Code Exists but Doc Didn't

These systems had working code but no dedicated documentation until this gap analysis.

| System | Code Location | New Doc Created |
|--------|--------------|----------------|
| NPC AI | python/cell/SGWMob.py | gameplay/npc-ai.md |
| Spawn System | python/cell/SGWSpawnRegion.py, SGWSpawnSet.py | gameplay/spawn-system.md |
| Loot System | python/cell/interactions/Lootable.py | gameplay/loot-system.md |
| Progression | SGWPlayer.py, Constants.py | gameplay/progression-system.md |
| Character Creation | python/base/Account.py | gameplay/character-creation.md |
| Server Infrastructure | Config files, C++ services | architecture/server-systems.md |

### Server-Only Blind Spots (Ranked by Gameplay Impact)

Systems that exist only on the server with no client trace, ranked by how much they affect playability.

| Rank | System | Impact | Evidence Source | Confidence |
|------|--------|--------|----------------|------------|
| 1 | Spawn System | CRITICAL -- no mobs without it | .def properties | HIGH (rich property names) |
| 2 | NPC Navigation | HIGH -- mobs can't move | C++ nav methods | HIGH (code exists) |
| 3 | Stat Scaling | HIGH -- no progression feel | Archetype healthPerLevel | MEDIUM (values exist) |
| 4 | XP from Kills | HIGH -- primary XP source | isWorthXP flag | HIGH (flag exists) |
| 5 | Session Management | MEDIUM -- disconnect = lost | config timeout | MEDIUM |
| 6 | Rate Limiting | MEDIUM -- exploitable | absence | LOW (MMO standard) |
| 7 | Economy Balance | LOW -- no economy yet | absence | LOW (MMO standard) |
| 8 | Metrics | LOW -- operational only | absence | LOW (MMO standard) |

---

## Summary Completion Matrix

| # | System | Total | CW | NT | IM | KM | NU |
|---|--------|-------|----|----|----|----|-----|
| 1 | Authentication | 8 | 5 | 0 | 3 | 0 | 0 |
| 2 | Mercury Protocol | 7 | 5 | 0 | 0 | 2 | 0 |
| 3 | Game Data Pipeline | 4 | 3 | 0 | 0 | 1 | 0 |
| 4 | Database Persistence | 5 | 4 | 0 | 1 | 0 | 0 |
| 5 | Character Creation | 11 | 0 | 9 | 0 | 2 | 0 |
| 6 | World Entry | 8 | 5 | 0 | 3 | 0 | 0 |
| 7 | Movement | 9 | 1 | 0 | 5 | 3 | 0 |
| 8 | Entity Lifecycle | 7 | 5 | 0 | 0 | 2 | 0 |
| 9 | Combat & Abilities | 21 | 0 | 0 | 14 | 7 | 0 |
| 10 | Effects & Buffs | 13 | 0 | 0 | 8 | 5 | 0 |
| 11 | Stats | 8 | 0 | 0 | 4 | 3 | 1 |
| 12 | Inventory | 11 | 1 | 8 | 0 | 2 | 0 |
| 13 | Missions | 12 | 1 | 0 | 8 | 3 | 0 |
| 14 | Loot | 9 | 0 | 5 | 1 | 3 | 0 |
| 15 | Vendors | 7 | 0 | 6 | 0 | 1 | 0 |
| 16 | NPC AI | 26 | 0 | 0 | 11 | 15 | 0 |
| 17 | Spawn System | 19 | 0 | 0 | 0 | 18 | 1 |
| 18 | XP & Leveling | 11 | 0 | 0 | 6 | 5 | 0 |
| 19 | Crafting | 9 | 0 | 8 | 0 | 1 | 0 |
| 20 | Stargate Travel | 9 | 0 | 0 | 6 | 3 | 0 |
| 21 | Chat | 10 | 0 | 6 | 0 | 4 | 0 |
| 22 | Trading | 8 | 0 | 7 | 0 | 1 | 0 |
| 23 | Organizations | 15 | 0 | 0 | 0 | 15 | 0 |
| 24 | Mail | 13 | 0 | 0 | 2 | 10 | 1 |
| 25 | Black Market | 10 | 0 | 0 | 0 | 10 | 0 |
| 26 | Contact Lists | 8 | 0 | 0 | 0 | 8 | 0 |
| 27 | Dueling | 6 | 0 | 0 | 0 | 6 | 0 |
| 28 | Pets | 7 | 0 | 0 | 2 | 5 | 0 |
| 29 | Minigames | 9 | 0 | 0 | 4 | 5 | 0 |
| 30 | Groups | 7 | 0 | 0 | 0 | 7 | 0 |
| -- | Session Mgmt | 6 | 0 | 0 | 3 | 3 | 0 |
| -- | Rate Limiting | 5 | 0 | 0 | 1 | 4 | 0 |
| -- | Anti-Cheat | 7 | 0 | 0 | 3 | 4 | 0 |
| -- | Economy | 6 | 0 | 0 | 3 | 3 | 0 |
| -- | World State | 5 | 1 | 0 | 2 | 2 | 0 |
| -- | Scheduler | 4 | 0 | 0 | 1 | 3 | 0 |
| -- | Admin/GM | 10 | 0 | 0 | 4 | 6 | 0 |
| -- | Metrics | 5 | 0 | 0 | 1 | 4 | 0 |
| | **TOTALS** | **369** | **31** | **49** | **95** | **191** | **3** |

### Summary Percentages

| Status | Count | Percentage |
|--------|-------|-----------|
| Confirmed Working (CW) | 31 | 8.4% |
| Needs Test (NT) | 49 | 13.3% |
| Implemented (IM) | 95 | 25.7% |
| Known/Missing (KM) | 191 | 51.8% |
| Needed/Unknown (NU) | 3 | 0.8% |

**Code exists (CW + NT + IM)**: 175 features (47.4%)
**Missing (KM + NU)**: 194 features (52.6%)

### Critical Path for Playability

The minimum viable gameplay loop requires these systems in order:

1. **Spawn System** (KM) -- Without it, no mobs exist in the world
2. **NPC Navigation** (KM) -- Without it, mobs stand still during combat
3. **XP from Kills** (KM) -- Primary progression driver
4. **Stat Scaling** (KM) -- Makes leveling meaningful
5. **Loot Table Content** (KM) -- Makes combat rewarding
6. **Effect Scripts** (KM) -- 4 of 3,217 effects have scripts

Once these are addressed, the core combat->loot->level loop is functional. All other systems (guilds, mail, AH, dueling, pets, minigames) are quality-of-life additions.

---

## Related Documents

- [project-status.md](project-status.md) -- Human-readable summary of this analysis
- [npc-ai.md](gameplay/npc-ai.md) -- NPC AI state machine and threat system
- [spawn-system.md](gameplay/spawn-system.md) -- Spawn region/set architecture
- [loot-system.md](gameplay/loot-system.md) -- Loot generation algorithm
- [progression-system.md](gameplay/progression-system.md) -- XP, leveling, training points
- [character-creation.md](gameplay/character-creation.md) -- Character creation flow
- [server-systems.md](architecture/server-systems.md) -- Server-only infrastructure
