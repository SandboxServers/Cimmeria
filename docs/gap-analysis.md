---
title: "Gameplay Systems Gap Analysis"
type: explanation
audience: engineers
last_updated: 2026-05-27
---

# Gameplay Systems Gap Analysis

> **Last updated**: 2026-05-27 (full rewrite against the Rust codebase, issue #344)
> **Purpose**: Map every gameplay system's Rust implementation against what's needed for a complete server
> **Status**: Source of truth for project completion tracking
> **Previous version**: Measured against the deprecated Python + C++ implementation. Replaced because the figures it produced (47% / 175-of-369) counted Python `pass` stubs as "code exists" while not counting any Rust ports. The current Rust workspace is **2,012 tests across 305 files**, **155 live-DB regression guards**, **3 PL/pgSQL end-to-end smokes**, with a **first-class content engine** the original Python codebase did not have.

---

## How to read this doc

- **Code paths** cite the active Rust workspace under [`crates/`](../crates/). When a feature exists in the deprecated [`deprecated/python/`](../deprecated/python/) or [`deprecated/cpp/`](../deprecated/cpp/) trees but **not yet** in Rust, the row is marked `KM` (port pending). The Python and C++ trees are reference-only.
- **Confidence** reflects how sure we are about the status — HIGH means the code has been read and judged; MEDIUM means line counts and recent-PR evidence support the status but a deep read hasn't happened; LOW means inference from neighbouring code or .def files.
- **Recent PRs** are listed where they're load-bearing for the status.

## Status Taxonomy

| Status | Symbol | Meaning |
|--------|--------|---------|
| **Confirmed Working** | `CW` | Tested end-to-end with the game client (Castle Cellblock smoke + Lomiada captures) and verified correct |
| **Needs Test** | `NT` | Code exists, looks reasonable, but hasn't been verified with a live client |
| **Implemented** | `IM` | Code written but may be incomplete or have known issues |
| **Known / Missing** | `KM` | We know this needs to exist (from `.def` files, docs, or game design) but no code exists in `crates/` |
| **Needed / Unknown** | `NU` | Server-only system we infer must exist but have no direct evidence for |

---

## Infrastructure Systems (Solid)

### 1. Authentication and Login --- CW

- **Confidence**: HIGH
- **Documentation**: [connection-flow.md](connection-flow.md), [protocol/login-handshake.md](protocol/login-handshake.md)
- **Rust code**: [`crates/services/src/auth/`](../crates/services/src/auth/) — 1,248 lines (handlers.rs, login_smoke.rs, mod.rs, service.rs) + 12 tests including a live-DB login smoke
- **Recent PRs**: #414 (auth + base + world-entry pipeline instrumentation), #366 (dev-session telemetry HMAC)
- **Path forward**: SHA1 → bcrypt/scrypt upgrade for production deployments. Rate limiting on login attempts.

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| SOAP login endpoint | CW | -- | auth/handlers.rs | HTTP POST /SGWLogin/UserAuth |
| Password validation | CW | -- | auth/handlers.rs | SHA1 hash compare against `account` table |
| Shard list response | CW | -- | auth/handlers.rs | Returns shard name + key |
| Shard key exchange | CW | -- | auth/service.rs | Symmetric key for Mercury session |
| Session establishment | CW | -- | base/login | BaseApp accepts after auth, login_smoke verified |
| Login audit logging | CW | -- | db/scripts/add_login_audit.sql | `login_audit` table, 6 outcome types |
| Duplicate login prevention | IM | -- | base/connect_loop | Checked at char-select, not continuous |
| Developer mode bypass | CW | -- | config | Allows duplicate logins, max access level |
| Dev-session telemetry token | CW | -- | server/main.rs | `CIMMERIA_TELEMETRY_HMAC_SECRET` HMAC-signed JWT-ish |
| Continuous auth validation | KM | -- | -- | Only checked at login |
| Login rate limiting | KM | -- | -- | No brute-force protection |

### 2. Mercury Protocol --- CW

- **Confidence**: HIGH
- **Documentation**: [drafts/spec/mercury-wire-format.md](drafts/spec/mercury-wire-format.md) (canonical, in-progress bible chapter), [protocol/mercury-wire-format.md](protocol/mercury-wire-format.md) (legacy summary), [architecture/transport-trait.md](architecture/transport-trait.md), [architecture/mercury-bundle.md](architecture/mercury-bundle.md), [architecture/mercury-loopback-harness.md](architecture/mercury-loopback-harness.md), [architecture/network-chaos-testing.md](architecture/network-chaos-testing.md)
- **Rust code**: [`crates/mercury/`](../crates/mercury/) — 3,529 lines across 12 files, **229 tests** (bundle, channel_bundle, clock, codec, encryption, instrumentation, lossy_transport, messages, test_transport, transport, unified)
- **Recent PRs**: #358 (Transport trait), #361 (ChannelBundle), #363/#365 (bundle progression), #370 (loopback harness, 22 paired-channel tests), #374 (network chaos), #404 (wire-log capture), #410 (mercury backpressure), #415 (warn! on unhandled dispatch)
- **Path forward**: Cumulative ACK + piggyback ACK optimizations remain. Bible chapter promotion (draft → verified).

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Reliable UDP transport | CW | -- | mercury/channel_bundle.rs | Sequence numbers, ACK/NAK, retransmit |
| AES-256-CBC encryption | CW | -- | mercury/encryption.rs | Audit was wrong about Blowfish; SGW uses AES-256-CBC + HMAC-MD5 |
| Message framing | CW | -- | mercury/bundle.rs | Header + variable-length body, per-channel sequence |
| Ordered delivery | CW | -- | mercury/channel_bundle.rs | Per-channel sequence |
| Fragmentation + reassembly | CW | -- | mercury/lib.rs | Large message splitting + reassembly |
| ChannelBundle accumulator | CW | -- | mercury/channel_bundle.rs | Cross-entity bundling, AoI burst migration |
| Transport trait + TestTransport | CW | -- | mercury/transport.rs | Wire-seam for byte-exact fan-out tests |
| LossyTransport (chaos) | CW | -- | mercury/lossy_transport.rs | Drop / dup / reorder / latency primitives |
| Loopback session harness | CW | -- | mercury/test_harness/ | Tier 2 paired-channel end-to-end tests |
| Pcap replay (wireclient) | CW | -- | wireclient/ | Tier 3 headless replay against live server |
| Observability instrumentation | CW | -- | mercury/instrumentation.rs | Per-packet OTLP spans, SigNoz integration |
| Cumulative ACKs | KM | -- | -- | Documented as missing; #311 closed dispatch warnings |
| Piggyback ACKs | KM | -- | -- | Documented as missing |

### 3. Game Data Pipeline (Cooked Data + Resources) --- CW

- **Confidence**: HIGH
- **Documentation**: [engine/cooked-data-pipeline.md](engine/cooked-data-pipeline.md), [engine/cooked-data-pak-format.md](engine/cooked-data-pak-format.md), [game-data.md](game-data.md), [architecture/mission-pak-overrides.md](architecture/mission-pak-overrides.md)
- **Rust code**: [`crates/services/src/base/cooked_data.rs`](../crates/services/src/base/cooked_data.rs), [`crates/services/src/base/resources/`](../crates/services/src/base/resources/) — 1,398 lines, [`crates/services/src/base/mission_overrides.rs`](../crates/services/src/base/mission_overrides.rs), [`crates/services/src/base/item_overrides.rs`](../crates/services/src/base/item_overrides.rs)
- **Recent PRs**: #250 (equip-from-inventory PAK override pattern), #399 / #405 (server-side stacking + Slappack PAK override)
- **Path forward**: Hot-reload of mission/item override caches without restart.

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Resource loading from DB | CW | -- | base/resources/ | 22 resource categories, 112,626 DB rows |
| Client version sync | CW | -- | base/cooked_data.rs | versionInfoRequest() handled |
| Cooked data (.pak) serving | CW | -- | base/cooked_data.rs | Binary pak files in data/cache/ |
| Mission PAK overrides | CW | -- | base/mission_overrides.rs | Injects new steps without reshipping pak |
| Item PAK overrides | CW | -- | base/item_overrides.rs | Health Slappack stack-size override (PR #399/#405) |
| InvalidKeys handshake | CW | -- | base/cooked_data.rs | Content-derived metadata bump |
| Hot reload at runtime | KM | -- | -- | No runtime reload — server restart needed for now |

### 4. Database Persistence --- CW

- **Confidence**: HIGH
- **Documentation**: [architecture/service-architecture.md](architecture/service-architecture.md), [architecture/integration-test-infra.md](architecture/integration-test-infra.md), [../db/README.md](../db/README.md)
- **Rust code**: sqlx 0.8 throughout `crates/services/` — **64 files use `sqlx::query`** + the durable outbox at [`crates/services/src/base/outbox/`](../crates/services/src/base/outbox/)
- **Recent PRs**: #403 (reseed pgdata on container start), #355 (cell_event_outbox infra), #366 (live-DB harness), #422 (cell_dispatch + executor live-DB coverage uplift)
- **Path forward**: Connection-pool tuning under load; no migration framework yet (manual db/scripts/).

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Player state persistence | CW | -- | services/cell/cell_methods/player | Level, XP, position, stats, training points |
| Inventory persistence | CW | -- | base/world_entry/methods/inventory | sgw_inventory + bandolier_slots tables |
| Mission persistence | CW | -- | entity/missions.rs | sgw_mission + per-step state |
| Cell event outbox | CW | -- | base/outbox/ | Durable Base→Cell event delivery |
| Compile-time query checking | CW | -- | sqlx::query! | Live-DB tests verify against postgres:17.9 |
| Live-DB test infrastructure | CW | -- | test_support.rs | `require_db_or_skip!`, 155 regression guards |
| Connection pooling | CW | -- | sqlx::Pool | Audit was wrong about "single connection per service" |
| Migration framework | KM | -- | db/scripts/ | Idempotent manual scripts; no Diesel/sqlx-migrate yet |

---

## Core Gameplay Systems

### 5. Character Creation --- NT

- **Confidence**: HIGH
- **Documentation**: [gameplay/character-creation.md](gameplay/character-creation.md)
- **Rust code**: [`crates/services/src/base/character_create.rs`](../crates/services/src/base/character_create.rs), [`crates/services/src/base/character/`](../crates/services/src/base/character/), [`crates/services/src/base/chardef.rs`](../crates/services/src/base/chardef.rs) — 1,640 lines with delete + request_visuals live-DB tests
- **Path forward**: Full client smoke through character-create → world-entry would move this to CW.

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Character list display | NT | -- | base/character/ | SELECT from sgw_player |
| Character visual preview | NT | -- | base/character/request_visuals_live_db_tests.rs | Lazy-load from sgw_inventory |
| Name validation | NT | -- | base/character_create.rs | SQL uniqueness check |
| Visual choice validation | NT | -- | base/chardef.rs | char_creation_choices table |
| Archetype selection | NT | -- | base/character_create.rs | 8 archetypes from resources |
| Starting equipment | NT | -- | base/character_create.rs | BagFillOrder insertion |
| Starting abilities | NT | -- | base/character_create.rs | From charDef ability list |
| Character deletion | NT | -- | base/character/delete_live_db_tests.rs | CASCADE to inventory, missions |
| GM character creation | KM | -- | -- | SGWGmPlayer subclass not ported |
| Name filtering | KM | -- | -- | No profanity/reserved name check |
| Character slot limit | KM | -- | -- | No per-account limit |

### 6. World Entry and Spaces --- CW

- **Confidence**: HIGH
- **Documentation**: [protocol/world-entry-phases.md](protocol/world-entry-phases.md), [engine/space-management.md](engine/space-management.md), [connection-flow.md](connection-flow.md)
- **Rust code**: [`crates/services/src/base/world_entry/`](../crates/services/src/base/world_entry/) — **64 files, 22,682 lines** (cell_dispatch, gate_travel, methods/{inventory, mail, player_load, progression, vendor})
- **Recent PRs**: #410 (world-entry spans + #408 follow-ups), #414 (pipeline instrumentation), #422 (cell_dispatch + character + executor/world coverage uplift)
- **Path forward**: Verify each of the 24 published spaces end-to-end (currently 1–2 zones routinely smoked).

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Space loading | CW | -- | services/cell/space_manager | NavMesh + entity loading |
| Player entity creation | CW | -- | base/world_entry/methods/player_load | Creates SGWPlayer entity, two-stage base+cell |
| Map load sequence | CW | -- | base/world_entry/ | All 30+ client setup messages |
| Stat sync to client | CW | -- | base/world_entry/methods/player_load | All stats sent on entry |
| Ability tree sync | CW | -- | base/world_entry/methods/player_load | 3 trees per archetype |
| Zone transition | IM | -- | base/world_entry/gate_travel/ | Tested for Castle ↔ neighbor; multi-player sync incomplete |
| Forced position handling | CW | -- | services/cell/cell_methods | BASEMSG_FORCED_POSITION authoritative move |
| World-entry observability | CW | -- | base/world_entry/ | OTLP spans across the whole pipeline |
| Cell dispatch arms | IM | -- | base/world_entry/cell_dispatch/ | tests_dispatch_arms/ has live-DB coverage |

### 7. Movement and Navigation --- IM

- **Confidence**: MEDIUM (player movement works, NPC pathfinding usable but not fully wired)
- **Documentation**: [protocol/position-updates.md](protocol/position-updates.md), [drafts/spec/position-updates.md](drafts/spec/position-updates.md) (canonical bible draft)
- **Rust code**: [`crates/entity/src/movement.rs`](../crates/entity/src/movement.rs), [`crates/entity/src/navigation.rs`](../crates/entity/src/navigation.rs), [`crates/entity/src/detour_ffi.rs`](../crates/entity/src/detour_ffi.rs) — 949 lines combined
- **Recent project memory**: Navmesh integration needed for NPC pathfinding, LoS, attack range (open project item)
- **Path forward**: Server-side movement validation (speed-hack detection), full Detour wiring for NPCs.

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Client position updates | CW | -- | services/cell/cell_methods | playerUpdate accepted |
| Dead reckoning / interpolation | IM | -- | entity/movement.rs | Extrapolates between updates |
| Space bounds check | IM | -- | entity/movement.rs | Validates within space |
| NavMesh / Detour FFI | IM | -- | entity/detour_ffi.rs | FFI wrapper present |
| NPC pathfinding | IM | -- | entity/navigation.rs | findPathTo/findDetailedPathTo |
| NPC waypoint movement | KM | Pathfinding | -- | Wiring pending |
| NPC patrol | KM | Pathfinding | -- | Spawn-set patrolPaths defined, runtime unwired |
| Server-side speed validation | KM | -- | -- | Client is authoritative; no anomaly detection |
| Teleport detection | KM | -- | -- | No position delta tracking |

### 8. Entity Lifecycle (AoI) --- CW

- **Confidence**: HIGH (witness-list discipline well established)
- **Documentation**: [engine/entity-lod-system.md](engine/entity-lod-system.md), [engine/entity-type-catalog.md](engine/entity-type-catalog.md)
- **Rust code**: [`crates/entity/src/cell_entity/`](../crates/entity/src/cell_entity/) — 2,113 lines (bandolier, state_flags, system_options, tests, mod), [`crates/entity/src/world_grid.rs`](../crates/entity/src/world_grid.rs), [`crates/entity/src/space.rs`](../crates/entity/src/space.rs)
- **Recent PRs**: #279 (BeingAppearance recomposite broadcast — design issue still open), #418 (generate_threat refreshes appearance on first-add), #408/#410 (AoI burst migration)
- **Path forward**: BeingAppearance fanout helper (issue #278, parent of #219/#232/#240/#249/#270).

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Entity creation | CW | -- | entity/manager.rs | From template or dynamic |
| Entity destruction | CW | -- | entity/manager.rs | Cleanup + witness notification |
| Grid-based AoI | CW | -- | entity/world_grid.rs | Chunk-based witness management |
| Witness enter/leave | CW | -- | entity/cell_entity/mod.rs | onEnter/onLeave events |
| Property synchronization | CW | -- | entity/properties.rs | Per-distribution-flag write paths |
| State flag conventions | CW | -- | entity/cell_entity/state_flags.rs | bStateField, BSF_InCombat lifecycle |
| Bandolier state | CW | -- | entity/cell_entity/bandolier.rs | Slot lifecycle, type_id vs item_id discipline |
| LOD system | KM | -- | -- | No entity detail levels |
| Witness-fanout helper | IM | -- | services/cell/ | Used inconsistently; issue #278 tracks consolidation |

### 9. Combat and Abilities --- IM

- **Confidence**: HIGH for primitives, MEDIUM for end-to-end coverage
- **Documentation**: [gameplay/combat-system.md](gameplay/combat-system.md), [gameplay/ability-system.md](gameplay/ability-system.md), [reverse-engineering/findings/combat-wire-formats.md](reverse-engineering/findings/combat-wire-formats.md)
- **Rust code**: [`crates/services/src/cell/combat/`](../crates/services/src/cell/combat/) (2,201 lines), [`crates/game/src/combat/`](../crates/game/src/combat/) (463 lines), [`crates/services/src/cell/abilities/`](../crates/services/src/cell/abilities/) (3,254 lines) — **5,918 lines combat-related, 142+ tests**
- **Recent PRs**: **#420 (complete ability + effect system — closes #47, #61, #331, #416, #419)**, #368 (NPC ability buckets + auto-aggro + Castle drone encounter, closes #342), #418 (generate_threat appearance refresh), #394 (autoReload + reloadOnActivate)
- **Path forward**: AoE damage falloff curves, deploy abilities, LOS checks, prerequisite moniker enforcement.

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| QR calculation | IM | -- | cell/combat/damage.rs | Hit/miss/crit beta distribution |
| Damage calculation | IM | -- | cell/combat/damage.rs | Resist → AF → absorb pipeline |
| Stat resistance | IM | -- | game/combat/stats.rs | fort/intel/engage |
| Armor factor | IM | -- | game/combat/stats.rs | Per-damage-type AF |
| Absorption | IM | -- | game/combat/damage.rs | 15 absorption stats |
| Auto-cycle (auto-fire) | CW | -- | cell/combat/auto_cycle.rs | Re-fire on cooldown complete |
| In-combat state lifecycle | CW | -- | cell/combat/state.rs | BSF_InCombat per-player threat tracking |
| Threat list | IM | -- | cell/combat/threat.rs | Linear scan; per-player threatened_mobs |
| Single-target abilities | IM | -- | cell/abilities/use_ability/ | TCM_Single |
| AoE abilities (radius) | IM | -- | cell/abilities/ | PR #420 closed AoE gaps |
| AoE abilities (cone) | IM | -- | cell/abilities/ | PR #420 |
| Group targeting | KM | Groups | -- | TCM_Group |
| Aura targeting | KM | -- | -- | TCM_Aura |
| Ability warmup | IM | -- | cell/abilities/use_ability/ | Speed modifiers applied |
| Ability cooldowns | CW | -- | cell/abilities/ | Per-ability + per-moniker timer |
| Position/facing checks | IM | -- | cell/abilities/use_ability/ | Front/flank/rear |
| Weapon range checks | IM | -- | cell/abilities/use_ability/ | Min/max range |
| Ammo consumption | CW | -- | cell/abilities/ | Decrement via bandolier discipline |
| Auto-reload | CW | -- | cell/abilities/ | PR #394 wired autoReload + reloadOnActivate |
| Damage application | IM | -- | cell/abilities/damage_apply/ | mod.rs handles kill XP via loot_drop |
| LOS checks | KM | -- | -- | No line-of-sight validation |
| Prerequisite monikers | KM | -- | Loaded, not checked | canUseWithMonikers exists but uncalled |
| Deploy abilities | IM | -- | cell/abilities/ | Flags handled, full deploy semantics partial |
| Health/focus regen tick | IM | -- | cell/effects/pulsing.rs | Effect-driven regen now works |

### 10. Effects and Buffs --- IM

- **Confidence**: HIGH for framework, MEDIUM for content coverage
- **Documentation**: [gameplay/effect-system.md](gameplay/effect-system.md)
- **Rust code**: [`crates/services/src/cell/effects/`](../crates/services/src/cell/effects/) — 2,380 lines (registry.rs, pulsing.rs, scripts.rs **869 lines**, mod), **34 tests**
- **Recent PRs**: **#420 (complete ability + effect system — closes #47, #61, #331, #416, #419)** is the headline. Earlier: #418 (generate_threat content action refreshes appearance on first-add).
- **Path forward**: Effect-script coverage for the long tail of the 3,217 effect rows; PAK-driven effect overrides where the script needs server-only logic.

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Effect application | CW | -- | cell/effects/registry.rs | Auto-replace same effect |
| Effect pulse/tick | CW | -- | cell/effects/pulsing.rs | Timer-driven; PR #420 closed gaps |
| Effect removal | CW | -- | cell/effects/registry.rs | Reverts non-permanent stat changes |
| Stat change tracking | CW | -- | cell/effects/pulsing.rs | Permanent vs non-permanent |
| Shared QR per pulse | IM | -- | cell/effects/pulsing.rs | One roll shared across effects |
| Clear on death | IM | -- | cell/effects/ | EF_ClearOnDeath wired |
| Clear on damage | IM | -- | cell/effects/ | EF_ClearOnDamage wired |
| Clear on revive | IM | -- | cell/effects/ | EF_ClearOnRez wired |
| Clear on bandolier swap | IM | -- | cell/effects/ | EF_RemoveOnBandolierSlotChange wired |
| Effect scripts (registry) | IM | -- | cell/effects/scripts.rs | 869 lines, growing — most-common scripts wired |
| Effect persistence | KM | -- | -- | EF_AlwaysPersist flag exists, not honored across logout |
| Channeled effects | IM | -- | cell/effects/ | is_channeled now flows through |
| Stealth-related flags | KM | -- | -- | EF_RemoveOnStealthZeroed etc. unhandled |

### 11. Stats --- IM

- **Confidence**: HIGH (infrastructure), MEDIUM (formula coverage)
- **Documentation**: [gameplay/stat-system.md](gameplay/stat-system.md), [gameplay/progression-system.md](gameplay/progression-system.md)
- **Rust code**: [`crates/entity/src/stats/`](../crates/entity/src/stats/) (1,118 lines), [`crates/game/src/combat/stats.rs`](../crates/game/src/combat/stats.rs) (159 lines)
- **Recent PRs**: 29d46a65 (XP/leveling — added `StatList::scale_for_level()`, health_per_level/focus_per_level, full heal on level-up)
- **Path forward**: Equipment stat bonuses (audit-era "KM" — still missing); derived-stat formulas.

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Stat class (min/cur/max) | CW | -- | entity/stats/ | 6 values + dirty flags |
| Dirty stat sync | CW | -- | entity/stats/ | Incremental updates |
| Public/private split | CW | -- | entity/stats/ | 11 public, rest private |
| Archetype base stats | CW | -- | entity/stats/ | Applied on first load |
| Per-level stat growth | CW | -- | game/player.rs | scale_for_level() applies health/focus per level |
| Derived stat formulas | KM | -- | -- | No general stat derivation system |
| Stat soft caps | NU | -- | -- | No diminishing returns |
| Item stat bonuses | KM | Inventory | -- | Equipment doesn't yet modify stats |

### 12. Inventory and Items --- IM

- **Confidence**: HIGH
- **Documentation**: [gameplay/inventory-system.md](gameplay/inventory-system.md), [reverse-engineering/findings/inventory-wire-formats.md](reverse-engineering/findings/inventory-wire-formats.md), [content/equip-from-inventory-pattern.md](content/equip-from-inventory-pattern.md)
- **Rust code**: [`crates/services/src/base/world_entry/methods/inventory/`](../crates/services/src/base/world_entry/methods/inventory/) — **5,272 lines** with `core/`, `grant/`, `move_/` submodules + live-DB regression guards; [`crates/game/src/inventory/`](../crates/game/src/inventory/) (370 lines); [`crates/entity/src/inventory.rs`](../crates/entity/src/inventory.rs)
- **Recent PRs**: #405 (server-side stacking + Slappack PAK override), #399 (Slappack stacks to 10), #214 (bandolier+content + UI sync + marsh quest loop + ambernol consumption), #250 (equip-from-inventory pattern + per-key PAK invalidation), #409 (full inventory re-init bundle on respawn)
- **Path forward**: Item durability + item binding flags (columns exist, semantics not wired).

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Bag system (20 types) | CW | -- | base/world_entry/methods/inventory/core | Main, mission, equipment, bank |
| Item add/remove | CW | -- | base/world_entry/methods/inventory/grant | Live-DB regression guards |
| Item stacking (server-side) | CW | -- | PR #405 | PR landed full server-side stack semantics |
| Equipment slots | NT | -- | base/world_entry/methods/inventory/ | Head through Artifact2 |
| Bandolier (4 weapon sets) | CW | -- | entity/cell_entity/bandolier.rs | Slot type_id/item_id discipline |
| Buyback bag | CW | -- | base/world_entry/methods/vendor/buyback | 12-slot |
| Cash (naquadah) | CW | -- | base/world_entry/methods/inventory/ | addCash/removeCash |
| Equip-from-inventory pattern | CW | -- | docs/content/equip-from-inventory-pattern.md | Mission 622/641 worked examples (PR #250) |
| Visual sync | NT | -- | base/world_entry_appearance.rs | Equipment visual updates |
| DB persistence | CW | -- | sqlx live-DB tests | sgw_inventory + bandolier tables |
| Item durability | KM | -- | Column exists | No wear/break mechanics |
| Item binding | KM | -- | Column exists | bound column unused |
| Respawn re-init bundle | CW | -- | PR #409 | Full inventory bundle, not just onUpdateItem |

### 13. Missions --- IM

- **Confidence**: HIGH for framework, MEDIUM for content coverage
- **Documentation**: [gameplay/mission-system.md](gameplay/mission-system.md), [reverse-engineering/findings/mission-wire-formats.md](reverse-engineering/findings/mission-wire-formats.md), [content/mission-chains.md](content/mission-chains.md), [architecture/mission-pak-overrides.md](architecture/mission-pak-overrides.md)
- **Rust code**: [`crates/services/src/cell/missions.rs`](../crates/services/src/cell/missions.rs) + per-system mission code under `cell/content/` (3,885 lines total mission-related), [`crates/game/src/missions/`](../crates/game/src/missions/) (364), [`crates/entity/src/missions.rs`](../crates/entity/src/missions.rs) (543), [`crates/services/src/base/mission_overrides.rs`](../crates/services/src/base/mission_overrides.rs)
- **Recent PRs**: #214 (marsh quest loop), #250 (equip-from-inventory PAK + per-key invalidation), Castle Cellblock end-to-end content
- **Path forward**: Mission sharing for groups; mission-gated loot filtering (Lootable TODO).

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Mission accept | CW | -- | cell/missions.rs | From NPC dialog (content-engine driven) |
| Mission tracking | CW | -- | entity/missions.rs | Steps, objectives, status |
| Objective completion | CW | -- | cell/content/executor/ | Content-engine action |
| Step advancement | CW | -- | cell/content/executor/ | Content-engine action |
| Mission completion | CW | -- | cell/content/executor/ | Rewards XP + naquadah |
| Mission failure | IM | -- | entity/missions.rs | failObjective() |
| Mission abandon | IM | -- | cell/missions.rs | abandon() handler |
| DB persistence | CW | -- | sqlx | sgw_mission table |
| Mission PAK overrides | CW | -- | base/mission_overrides.rs | Mid-chain step injection |
| Repeatable missions | IM | -- | entity/missions.rs | MissionInstance.repeats column; condition gating pending |
| Mission sharing | KM | Groups | -- | shareMission stub |
| Mission-gated loot | KM | Loot | -- | TODO in cell/interactions/loot.rs |

### 14. Loot --- IM

- **Confidence**: MEDIUM (logic exists, content sparse)
- **Documentation**: [gameplay/loot-system.md](gameplay/loot-system.md)
- **Rust code**: [`crates/services/src/cell/interactions/loot.rs`](../crates/services/src/cell/interactions/loot.rs) (309 lines), [`crates/game/src/inventory/loot.rs`](../crates/game/src/inventory/loot.rs), loot bag drop in [`crates/services/src/cell/abilities/loot_drop.rs`](../crates/services/src/cell/abilities/loot_drop.rs)
- **Path forward**: Loot table content (database is mostly empty); per-player loot eligibility for groups; group-loot modes.

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Loot table definitions | CW | -- | db/resources/Loot/ | Schema present |
| Loot generation algorithm | NT | -- | cell/interactions/loot.rs | Per-item probability roll |
| Item drops | CW | -- | cell/abilities/loot_drop.rs | Drops + transfer to inventory |
| Cash drops | NT | -- | cell/interactions/loot.rs | LOOT_Cash |
| Loot bag take-all | CW | -- | cell/interactions/loot.rs | Castle Cellblock smoke verified |
| Per-player eligibility | IM | -- | cell/interactions/loot.rs | eligiblePlayerList exists, group assignment unwired |
| Group loot modes | KM | Groups | -- | RoundRobin/FreeForAll enums, no logic |
| Mission-gated loot | KM | Missions | -- | TODO: missionId filtering |
| Loot table content | KM | -- | -- | Tables mostly empty |

### 15. Stores / Vendors --- NT

- **Confidence**: HIGH (most thoroughly worked subsystem after world-entry)
- **Documentation**: [gameplay/inventory-system.md](gameplay/inventory-system.md)
- **Rust code**: [`crates/services/src/base/world_entry/methods/vendor/`](../crates/services/src/base/world_entry/methods/vendor/) — **7,267 lines** across `buyback/`, `paid_recharge/`, `paid_repair/`, `purchase/`, `sell/`, `data/` submodules
- **End-to-end smoke**: [`tools/vendor_store_smoke.sql`](../tools/vendor_store_smoke.sql)
- **Recent PRs**: #214 (vendor sync), live-DB regression guards across each operation
- **Path forward**: Full client smoke through buy + sell + repair + recharge + buyback cycle would move this to CW.

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Buy items | NT | -- | vendor/purchase/ | Validates cash, creates item, live-DB tests |
| Sell items | NT | -- | vendor/sell/ | Validates ownership, adds cash |
| Repair items | NT | -- | vendor/paid_repair/ | Cost calculation |
| Recharge items | NT | -- | vendor/paid_recharge/ | Ammo recharge |
| Buyback | NT | -- | vendor/buyback/ | 12-slot |
| Vendor stock from DB | CW | -- | vendor/data/ | Buy/sell/repair item lists from DB |
| Transactional safety | IM | -- | sqlx transactions | Live-DB tests verify atomicity |
| PL/pgSQL smoke | CW | -- | tools/vendor_store_smoke.sql | End-to-end test |

---

## NPC Systems

### 16. NPC AI and Behavior --- IM

- **Confidence**: MEDIUM (combat AI works, navigation-driven states still pending)
- **Documentation**: [gameplay/npc-ai.md](gameplay/npc-ai.md)
- **Rust code**: AI logic distributed between [`crates/services/src/cell/spawner/`](../crates/services/src/cell/spawner/) (1,983 lines) and [`crates/services/src/cell/content/`](../crates/services/src/cell/content/) (7,906 lines — content-engine drives behavior chains)
- **Recent PRs**: #368 (NPC ability buckets + auto-aggro + Castle drone encounter — closes #342), #418 (generate_threat content action refreshes appearance on first-add)
- **Path forward**: Patrol/wander/leashing states (blocked on Navigation); cover system (1,332 unimplemented Atrea cover nodes); proactive aggro detection radius.

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| AI state machine | IM | -- | spawner/npcs.rs | Idle / Fighting / Dead / Leashing states active |
| Spawning state | CW | -- | spawner/npcs.rs | Loads ammo, transitions to Idle |
| Idle state | CW | -- | spawner/npcs.rs | Waits for threat |
| Fighting state | IM | -- | spawner/npcs.rs | Target + ability + fire loop |
| Threat accumulation | IM | -- | cell/combat/threat.rs | -healthChange*2 - focusChange |
| Top-threat targeting | IM | -- | cell/combat/threat.rs | Linear scan, dead pruning |
| Ability bucket selection | CW | -- | spawner/abilities.rs | PR #368 three-bucket model (usable/cooling/needs-ammo) |
| Auto-reload | CW | -- | cell/abilities/ | PR #394 |
| Loot on death | CW | -- | cell/abilities/loot_drop.rs | Generates loot, sets interaction |
| Aggression override | IM | -- | spawner/npcs.rs | Per-instance + timed |
| Auto-aggro | CW | -- | PR #368 | Castle drone encounter end-to-end verified |
| Investigating state | KM | Navigation | -- | POI / investigateTimerID defined in spawner |
| Leashing state | IM | Navigation | spawner/npcs.rs | Home property set; full unwind partial |
| Patrol state | KM | Navigation | -- | patrolPaths defined; runtime unwired |
| Wander state | KM | Navigation | -- | Home + nextWanderTime defined |
| Follow state | KM | Navigation | -- | followTarget defined |
| Despawning state | KM | Spawning | -- | despawnFlag defined |
| Cover system | KM | Navigation | -- | 1,332 Atrea cover nodes unimplemented |
| Hearing system | KM | -- | -- | hearingRadius defined |
| Mob groups | KM | -- | -- | mobGroup defined |
| Tapping / kill credit | KM | -- | -- | tappedEntity defined |
| XP on kill | CW | -- | cell/abilities/damage_apply/ | kill_xp(), 10×mob_level Cell→Base pipeline |

### 17. Spawn System --- IM

- **Confidence**: HIGH (Castle Cellblock smoke covers full spawn lifecycle)
- **Documentation**: [gameplay/spawn-system.md](gameplay/spawn-system.md)
- **Rust code**: [`crates/services/src/cell/spawner/`](../crates/services/src/cell/spawner/) — **1,983 lines, 23 tests** across regions, respawners, npcs, loot, missions, dialogs, stargates, abilities submodules
- **Path forward**: Time-of-day spawns, population scaling, linked sets, mission-integration polish.

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| SpawnRegion entity | IM | -- | spawner/regions.rs | Activation handled |
| SpawnSet entity | IM | -- | spawner/regions.rs | Activation handled |
| Region activation | CW | -- | spawner/regions.rs | Activated/Deactivated lifecycle |
| Set activation | CW | -- | spawner/regions.rs | Activate/Deactivate hooks |
| Mob spawning | CW | -- | spawner/npcs.rs | MySpawnPoints, entity creation |
| Mob registration | IM | -- | spawner/npcs.rs | RegisterMobBase analog |
| Population tracking | IM | -- | spawner/regions.rs | CurrentPopulation, reportPopulation |
| Mob death notification | CW | -- | spawner/respawners.rs | BeingDeath fires respawn cycle |
| Respawn timers | CW | -- | spawner/respawners.rs | Min/Max RespawnSeconds |
| Set cooldowns | IM | -- | spawner/regions.rs | min/maxCooldownSeconds |
| Max active sets | IM | -- | spawner/regions.rs | MaxActiveSets honored |
| Spawn tables (weighted) | IM | -- | spawner/npcs.rs | (id, weight) tuples |
| Spawn point randomization | IM | -- | spawner/npcs.rs | bRandomizeSpawnPoints |
| Level range filtering | IM | -- | spawner/npcs.rs | minMOBLevel, maxMOBLevel |
| Player detection radius | KM | -- | -- | detectionRadius defined, not wired |
| Time-of-day spawns | KM | -- | -- | onTimeOfDayTick |
| Mission integration | IM | -- | spawner/missions.rs | mission events fire from spawn lifecycle |
| Linked sets | KM | -- | -- | bLinked flag, semantics unknown |
| Population scaling | NU | -- | -- | timerReduction suggests dynamic spawn rates |
| Stargate spawning | CW | -- | spawner/stargates.rs | Castle ↔ neighbor verified |
| Loot-drop integration | CW | -- | spawner/loot.rs | Castle smoke covers loot bag drop |
| Dialog NPC spawning | CW | -- | spawner/dialogs.rs | Castle Cellblock NPCs |
| Ability NPC spawning | CW | -- | spawner/abilities.rs | PR #368 three-bucket |

---

## Secondary Gameplay Systems

### 18. XP and Leveling --- IM

- **Confidence**: HIGH (kill-XP pipeline shipped end-to-end)
- **Documentation**: [gameplay/progression-system.md](gameplay/progression-system.md), [.claude/plans/2026-03-08-xp-leveling-design.md](../.claude/plans/2026-03-08-xp-leveling-design.md)
- **Rust code**: [`crates/game/src/player.rs`](../crates/game/src/player.rs) (`PlayerState::grant_xp`, `kill_xp`, level scaling), [`crates/services/src/base/world_entry/methods/progression/`](../crates/services/src/base/world_entry/methods/progression/), [`crates/services/src/cell/abilities/damage_apply/`](../crates/services/src/cell/abilities/damage_apply/), [`crates/services/src/cell/abilities/loot_drop.rs`](../crates/services/src/cell/abilities/loot_drop.rs)
- **Recent commits**: 29d46a65 (full XP + leveling system implementation)
- **Path forward**: Mission-XP integration (mission `reward_xp` columns are 0 in seed data — chain authoring task); ASP grant on level-up.

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| XP accumulation | CW | -- | game/player.rs | grant_xp() — additive, fires onExpUpdate |
| Level-up detection | CW | -- | game/player.rs | Multi-level-up supported, 16+ tests |
| Client notification | CW | -- | base/world_entry/methods/progression/ | 5 wire messages |
| XP from mob kills | CW | -- | cell/abilities/damage_apply/ | 10×mob_level, Cell→Base pipeline |
| XP curve | CW | -- | game/player.rs | LEVEL_XP[21] ported from Python Constants |
| Level cap (20) | CW | -- | game/player.rs | MAX_LEVEL enforced |
| DB persistence | CW | -- | sqlx | sgw_player.level + .exp |
| Stat scaling on level-up | CW | -- | entity/stats/ | scale_for_level(), full heal on level-up |
| Training points on level-up | CW | -- | game/player.rs | 2 TP/level, 38 by level 20 |
| XP from missions | KM | Content | seed data | mission.reward_xp is 0 in all seed rows |
| ASP on level-up | KM | -- | -- | No ASP grant |

### 19. Crafting --- KM (port pending)

- **Confidence**: HIGH (we know what needs to exist)
- **Documentation**: [gameplay/crafting-system.md](gameplay/crafting-system.md), [reverse-engineering/findings/crafting-wire-formats.md](reverse-engineering/findings/crafting-wire-formats.md)
- **Rust code**: **No `crates/services/src/cell/crafting/` exists yet.** Python `Crafter.py` (575 lines) under [`deprecated/python/`](../deprecated/python/) was never ported.
- **Path forward**: Full port of the Python Crafter — craft / research / reverse-engineer / alloy flows.

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Craft from blueprint | KM | -- | (Python only) | Validates, consumes, 3s timer |
| Research item | KM | -- | (Python only) | Expertise chance roll |
| Reverse engineer | KM | -- | (Python only) | Random blueprint + components |
| Alloy | KM | -- | (Python only) | Tier + elementary components |
| Discipline learning | KM | -- | -- | spendAppliedSciencePoints |
| Expertise system (0-100) | KM | -- | -- | gainExpertise capped at 100 |
| Racial paradigm gating | KM | -- | -- | Paradigm level prereqs |
| Blueprint management | KM | -- | -- | Deduplicated list |
| Crafting respec | KM | -- | -- | respecCrafting stub in Python |

### 20. Stargate Travel --- IM

- **Confidence**: MEDIUM
- **Documentation**: [gameplay/gate-travel.md](gameplay/gate-travel.md), [reverse-engineering/findings/gate-travel-wire-formats.md](reverse-engineering/findings/gate-travel-wire-formats.md)
- **Rust code**: [`crates/services/src/cell/gate_travel.rs`](../crates/services/src/cell/gate_travel.rs), [`crates/services/src/cell/cell_methods/gate_travel.rs`](../crates/services/src/cell/cell_methods/gate_travel.rs), [`crates/services/src/cell/client_methods/gate_travel.rs`](../crates/services/src/cell/client_methods/gate_travel.rs), [`crates/services/src/base/world_entry/gate_travel/`](../crates/services/src/base/world_entry/gate_travel/), [`crates/game/src/interactions/stargate.rs`](../crates/game/src/interactions/stargate.rs) — 935 lines combined
- **Path forward**: Multi-player gate sync; gate cooldown; return-trip state.

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| DHD interaction | IM | -- | cell/interactions/dialog.rs | Opens DHD with origin address |
| Gate dialing | IM | -- | cell/gate_travel.rs | Timer + state machine |
| Gate cancel | IM | -- | cell/gate_travel.rs | Timer cancel + event |
| Gate passage | CW | -- | base/world_entry/gate_travel/ | moveTo destination, end-to-end verified |
| Known address tracking | IM | -- | cell/gate_travel.rs | Known + hidden lists |
| Address discovery | IM | -- | cell/gate_travel.rs | Added by missions/exploration |
| Cell-dispatch AoI defer | CW | -- | base/world_entry/cell_dispatch/tests_dispatch_arms/aoi_defer_gate.rs | Regression test |
| Multi-player gate sync | KM | AoI | -- | Other players don't see gate events |
| Return trips | KM | -- | -- | No bidirectional gate state |
| Gate cooldown | KM | -- | -- | No use-after-dial cooldown |

### 21. Chat --- NT

- **Confidence**: MEDIUM
- **Documentation**: [gameplay/chat-system.md](gameplay/chat-system.md), [reverse-engineering/findings/chat-wire-formats.md](reverse-engineering/findings/chat-wire-formats.md)
- **Rust code**: [`crates/services/src/cell/chat.rs`](../crates/services/src/cell/chat.rs), [`crates/services/src/cell/client_methods/communicator.rs`](../crates/services/src/cell/client_methods/communicator.rs), [`crates/services/src/base/world_entry_chat.rs`](../crates/services/src/base/world_entry_chat.rs) — 355 lines combined
- **Path forward**: Channel system (currently say/emote/yell only); admin/moderation tools.

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Say/emote/yell (AoI) | NT | AoI | cell/chat.rs | Witness broadcast |
| Direct tells | KM | -- | -- | sendPlayerMessage not ported |
| User channels | KM | -- | -- | requestCreateChannel not ported |
| Pre-defined channels | KM | -- | -- | team/squad/command/officer/server |
| Channel ops | KM | -- | -- | setPlayerOp not ported |
| Chat flood protection | KM | -- | -- | No rate limiting |
| Profanity filter | KM | -- | -- | No filtering |
| Mute system | KM | -- | -- | No per-player muting |
| GM broadcast | KM | Admin | -- | No system-wide message tool |

### 22. Trading --- KM (port pending)

- **Confidence**: HIGH
- **Documentation**: [gameplay/trade-system.md](gameplay/trade-system.md), [reverse-engineering/findings/trade-wire-formats.md](reverse-engineering/findings/trade-wire-formats.md)
- **Rust code**: **No dedicated trade module in `crates/`.** Python `Trade.py` (244 lines) was never ported.
- **Path forward**: Full state-machine port (propose → lock → confirm → execute with item/cash atomic swap).

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Trade initiation | KM | -- | (Python only) | By entity ID |
| Proposal update | KM | -- | -- | Version-sequenced |
| Lock state machine | KM | -- | -- | None → Locked → LockedAndConfirmed |
| Confirmation | KM | -- | -- | Validates both proposals |
| Item swap | KM | Inventory | -- | Atomic transfer |
| Cash swap | KM | -- | -- | Validates balances |
| Cancel | KM | -- | -- | Either party can cancel |
| Disconnect cleanup | KM | -- | -- | Item-lock unwind on dc |

---

## Stub-Only Systems

### 23. Organizations / Guilds --- KM

- **Confidence**: STUB
- **Documentation**: [gameplay/organization-system.md](gameplay/organization-system.md), [reverse-engineering/findings/organization-wire-formats.md](reverse-engineering/findings/organization-wire-formats.md)
- **Rust code**: [`crates/services/src/cell/cell_methods/organization.rs`](../crates/services/src/cell/cell_methods/organization.rs), [`crates/services/src/cell/client_methods/organization.rs`](../crates/services/src/cell/client_methods/organization.rs) — 200 lines combined (handler stubs)
- **Path forward**: DB schema (`sgw_organization` table) + full org lifecycle.

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Organization creation | KM | DB schema | stub | onOrganizationCreation handler returns |
| Invite/accept | KM | Creation | stub | -- |
| Leave organization | KM | -- | stub | -- |
| Rank system (9 ranks) | KM | Creation | -- | EORG_RANK_None through Leader |
| Permission system (26 perms) | KM | Ranks | -- | Bitmask in enums |
| MOTD | KM | Creation | stub | -- |
| Officer notes | KM | Ranks | stub | -- |
| Rank name customization | KM | Ranks | stub | -- |
| Permission editing | KM | Ranks | stub | -- |
| Cash transfer to bank | KM | Creation | stub | -- |
| Organization vault | KM | Creation, Inventory | -- | INV_TeamBank, INV_CommandBank |
| Squad loot mode | KM | Groups | stub | -- |
| Minimap ping | KM | -- | stub | -- |
| Strike teams | KM | -- | stub | -- |
| PvP org leave | KM | -- | stub | -- |

### 24. Mail --- IM

- **Confidence**: MEDIUM (more advanced than audit-era claim — full base/world_entry/methods/mail tree)
- **Documentation**: [gameplay/mail-system.md](gameplay/mail-system.md), [reverse-engineering/findings/mail-wire-formats.md](reverse-engineering/findings/mail-wire-formats.md)
- **Rust code**: [`crates/services/src/base/world_entry/methods/mail/`](../crates/services/src/base/world_entry/methods/mail/) (1,456 lines incl. tests), [`crates/services/src/cell/mail.rs`](../crates/services/src/cell/mail.rs), [`crates/services/src/cell/cell_methods/mail.rs`](../crates/services/src/cell/cell_methods/mail.rs), [`crates/services/src/cell/client_methods/mail.rs`](../crates/services/src/cell/client_methods/mail.rs)
- **Path forward**: Attachment item-lock semantics; CoD safety; new-mail notification fanout.

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Open mailbox (headers) | IM | -- | base/world_entry/methods/mail/ | Queries sgw_gate_mail |
| Read mail body | IM | -- | base/world_entry/methods/mail/ | Queries |
| Send mail | IM | -- | base/world_entry/methods/mail/ | Live-DB tests in mail/tests.rs |
| Delete mail | IM | -- | base/world_entry/methods/mail/ | -- |
| Archive mail | IM | -- | base/world_entry/methods/mail/ | -- |
| Attach item | IM | Send, Inventory | -- | itemId param wired |
| Attach gold | IM | Send | -- | cash param wired |
| Cash on Delivery | KM | Send, Receive | -- | bCOD param defined, semantics partial |
| Take item from mail | IM | Inventory | -- | -- |
| Take cash from mail | IM | -- | -- | -- |
| Return to sender | KM | Send | -- | -- |
| New mail notification | KM | Send | -- | No fanout when recipient online |
| Mail expiry/TTL | NU | DB | -- | No TTL in schema |

### 25. Black Market (Auction House) --- KM

- **Confidence**: STUB
- **Documentation**: [gameplay/black-market.md](gameplay/black-market.md), [reverse-engineering/findings/black-market-wire-formats.md](reverse-engineering/findings/black-market-wire-formats.md)
- **Rust code**: [`crates/services/src/cell/cell_methods/black_market.rs`](../crates/services/src/cell/cell_methods/black_market.rs), [`crates/services/src/cell/client_methods/black_market.rs`](../crates/services/src/cell/client_methods/black_market.rs) — 94 lines (handler stubs)
- **Path forward**: New `sgw_auction` table + lifecycle (listing, bid, buyout, expiry).

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Search listings | KM | DB schema | stub | Sort enums defined |
| Create listing | KM | DB, Inventory | stub | 5 duration tiers |
| Place bid | KM | Listing, Economy | stub | -- |
| Buyout | KM | Listing, Economy | stub | -- |
| Cancel listing | KM | Listing | stub | -- |
| View my auctions | KM | Listing | stub | -- |
| View my bids | KM | Listing | stub | -- |
| Auction expiry | KM | Scheduler | -- | Timer-based cleanup |
| Listing fees | NU | Economy | -- | Standard MMO pattern |
| Transaction mail | KM | Mail | -- | Results via mail |

### 26. Contact Lists --- KM

- **Confidence**: STUB
- **Documentation**: [gameplay/contact-list.md](gameplay/contact-list.md), [reverse-engineering/findings/contact-list-wire-formats.md](reverse-engineering/findings/contact-list-wire-formats.md)
- **Rust code**: [`crates/services/src/cell/cell_methods/contact_list.rs`](../crates/services/src/cell/cell_methods/contact_list.rs), [`crates/services/src/cell/client_methods/contact_list.rs`](../crates/services/src/cell/client_methods/contact_list.rs) — 86 lines (handler stubs)
- **Path forward**: New `sgw_contact_list` table + member management + online-status events.

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Create list | KM | DB schema | stub | contactListCreate |
| Delete list | KM | -- | stub | -- |
| Rename list | KM | -- | stub | -- |
| Update flags | KM | -- | stub | issue #275 verifies handler exists |
| Add members | KM | -- | stub | -- |
| Remove members | KM | -- | stub | -- |
| Online status events | KM | Session | -- | ECONTACT_LIST_EVENT_LoggedInStatus |
| Level-up events | KM | Progression | -- | ECONTACT_LIST_EVENT_GainLevel |

### 27. Dueling --- KM

- **Confidence**: STUB
- **Documentation**: [gameplay/duel-system.md](gameplay/duel-system.md), [reverse-engineering/findings/duel-wire-formats.md](reverse-engineering/findings/duel-wire-formats.md)
- **Rust code**: **No dedicated duel module.** Handler stubs in `cell_methods/player/social.rs`.
- **Path forward**: 5-state machine port; 7 defeat-condition enum.

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Duel challenge | KM | -- | -- | State: ResponsePending |
| Duel response | KM | -- | stub | -- |
| Duel start | KM | Combat | -- | StartPending → Engaged |
| Duel forfeit | KM | -- | stub | -- |
| Defeat conditions | KM | Combat | -- | 7 types |
| Duel marker entity | KM | -- | -- | SGWDuelMarker not ported |

### 28. Pets --- KM

- **Confidence**: STUB
- **Documentation**: [gameplay/pet-system.md](gameplay/pet-system.md), [reverse-engineering/findings/pet-wire-formats.md](reverse-engineering/findings/pet-wire-formats.md)
- **Rust code**: No dedicated pet module. Pet entity extends SGWMob in Python; equivalent Rust path not built.
- **Path forward**: Pet entity (extends spawner mob), Follow AI state, command handling.

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Pet ability list sync | KM | -- | -- | -- |
| Pet stance list sync | KM | -- | -- | -- |
| Invoke pet ability | KM | Combat | -- | -- |
| Toggle pet ability | KM | -- | -- | -- |
| Change pet stance | KM | -- | -- | -- |
| Pet following | KM | NPC AI (Follow) | -- | Needs Follow AI state |
| Pet combat AI | KM | NPC AI | -- | Inherits SGWMob |

### 29. Minigames --- KM

- **Confidence**: STUB (framework partial)
- **Documentation**: [gameplay/minigame-system.md](gameplay/minigame-system.md), [reverse-engineering/findings/minigame-wire-formats.md](reverse-engineering/findings/minigame-wire-formats.md)
- **Rust code**: [`crates/services/src/cell/cell_methods/minigame.rs`](../crates/services/src/cell/cell_methods/minigame.rs), [`crates/services/src/cell/client_methods/minigame.rs`](../crates/services/src/cell/client_methods/minigame.rs) — 202 lines (session-routing stubs)
- **External**: External SmartFoxServer minigame service is not yet running (Hack, Bypass, Livewire, GoauldCrystals, Alignment, Activate, Analyze, Converse — 8 games).
- **Path forward**: Port at least one minigame end-to-end (Livewire is the canonical starter); SmartFoxServer 1.x XML protocol.

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Minigame session start | IM | -- | cell/cell_methods/minigame.rs | Session routing scaffold |
| Seed generation | IM | -- | cell/cell_methods/minigame.rs | 0..0x7FFFFFFF |
| Result callback | IM | Mission integration | cell/cell_methods/minigame.rs | Victory/Defeat/Canceled |
| Mission integration | IM | Missions | content-engine OnMinigameComplete | Chain trigger exists |
| 8 game types | KM | External server | -- | All games unimplemented |
| Spectating | KM | -- | stub | -- |
| Co-op / help | KM | -- | stub | -- |
| Contact system | KM | -- | stub | -- |
| External minigame server | KM | Infrastructure | -- | Separate process, SmartFox 1.x |

### 30. Groups / Parties --- KM

- **Confidence**: STUB
- **Documentation**: [gameplay/group-system.md](gameplay/group-system.md), [reverse-engineering/findings/group-wire-formats.md](reverse-engineering/findings/group-wire-formats.md)
- **Rust code**: No dedicated group module. Cross-cutting with Organizations.
- **Path forward**: Implement as lightweight Squad-type Organization.

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Group creation | KM | -- | -- | EORG_TYPE_Squad = 0 |
| Group invite | KM | -- | -- | -- |
| Group leave | KM | -- | -- | -- |
| Member info sync | KM | -- | -- | 9 EMEMBER_INFO types defined |
| Loot mode setting | KM | Loot | stub | squadSetLootMode |
| Group combat assist | KM | Combat, NPC AI | -- | onGroupMateEnteredCombat |
| Threat transfer | KM | NPC AI | -- | onGroupMateThreatTransfer |

---

## Systems New Since the Original Audit (March 2026)

These didn't exist in the deprecated Python codebase and so weren't in the audit. They're substantial in Rust today.

### 31. Content Engine --- CW

- **Confidence**: HIGH
- **Documentation**: [content/content-engine.md](content/content-engine.md), [content/extending-the-engine.md](content/extending-the-engine.md), [architecture/data-driven-content-engine.md](architecture/data-driven-content-engine.md)
- **Rust code**: [`crates/content-engine/`](../crates/content-engine/) (3,561 lines) + [`crates/services/src/cell/content/`](../crates/services/src/cell/content/) (**7,906 lines, 99 tests**)
- **Path forward**: Wire the remaining defined-but-inert actions (apply_effect, remove_effect, start_timer, cancel_timer, roll_loot_table, spawn_entity, grant_xp).

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Trigger / condition / action chain | CW | -- | content-engine/chain.rs | Castle Cellblock missions end-to-end |
| Loader (DB → Action enum) | CW | -- | content-engine/loader/ | Boundary validation here |
| Executor (action → side effects) | CW | -- | cell/content/executor/ | 99 tests cover dispatched arms |
| Event dispatch | CW | -- | cell/content/event_dispatch/ | OnEntityDeath, OnInteract, OnDialog, ... |
| Mission-context populator | CW | -- | cell/content/mission_context.rs | -- |
| Chain replay tests | CW | -- | cell/content/chain_replay_tests/ | Pins observed chain behavior |
| Action::ApplyEffect / RemoveEffect | KM | -- | loader OK, executor missing | Defined but inert |
| Action::StartTimer / CancelTimer | KM | -- | -- | Trigger::OnTimer defined too |
| Action::GrantXP | KM | -- | -- | Variant defined; mission XP zeroed in seed |
| Persistent counters | KM | -- | content_counters table | Schema present, runtime in-memory only |

### 32. Mercury Bundle / ChannelBundle --- CW

- **Confidence**: HIGH
- **Documentation**: [architecture/mercury-bundle.md](architecture/mercury-bundle.md), [architecture/transport-trait.md](architecture/transport-trait.md)
- **Rust code**: [`crates/mercury/src/channel_bundle.rs`](../crates/mercury/src/channel_bundle.rs), [`crates/mercury/src/bundle.rs`](../crates/mercury/src/bundle.rs)
- **Recent PRs**: #361 (ChannelBundle + AoI burst), #363 (bundle onClientReady), #365 (bundle progression + teleport)

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Cross-entity bundling | CW | -- | channel_bundle.rs | -- |
| AoI burst bundling | CW | -- | PR #361 | -- |
| onClientReady appearance/chat bundling | CW | -- | PR #363 | -- |
| Progression + teleport bundling | CW | -- | PR #365 | -- |
| Backpressure handling | CW | -- | PR #410 | -- |

### 33. Observability Pipeline --- CW

- **Confidence**: HIGH
- **Documentation**: [architecture/observability.md](architecture/observability.md), [operations/signoz-deployment.md](operations/signoz-deployment.md), [operations/signoz-remote-access.md](operations/signoz-remote-access.md), [operations/telemetry.md](operations/telemetry.md), [architecture/negative-logging-convention.md](architecture/negative-logging-convention.md)
- **Rust code**: OTLP exporter wired into all services; Mercury packet instrumentation in [`crates/mercury/src/instrumentation.rs`](../crates/mercury/src/instrumentation.rs); negative-logging convention enforced by `LogCapture` test helper
- **Recent PRs**: #396, #398, #400, #402, #404, #410, #414 (full pipeline)

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| OTLP log appender | CW | -- | server/main.rs | PR #398 |
| Hot-path tracing spans | CW | -- | mercury/, base/, cell/ | PR #398 |
| Mercury packet logging | CW | -- | mercury/instrumentation.rs | Per-packet OTLP |
| Wire-log capture stream | CW | -- | PR #404 | Per-message decode to SigNoz |
| SigNoz self-hosted overlay | CW | -- | operations/signoz-deployment.md | ClickHouse-backed |
| Cloudflare Tunnel + Access | CW | -- | operations/signoz-remote-access.md | No inbound ports |
| Dev-session telemetry | CW | -- | architecture/dev-session-telemetry.md | HMAC-signed token |
| Negative-logging convention | CW | -- | architecture/negative-logging-convention.md | LogCapture regression-guard helper |

### 34. Wireclient + Network Chaos Testing --- CW

- **Confidence**: HIGH
- **Documentation**: [architecture/wireclient.md](architecture/wireclient.md), [architecture/network-chaos-testing.md](architecture/network-chaos-testing.md), [architecture/mercury-loopback-harness.md](architecture/mercury-loopback-harness.md)
- **Rust code**: [`crates/wireclient/`](../crates/wireclient/) (1,727 lines) — Tier 3 headless client; LossyTransport in mercury; loopback harness for Tier 2
- **Recent PRs**: #370 (Tier 2 loopback harness, 22 paired-channel tests), #374 (network chaos L1+L2+L3), #376 (Tier 3 wireclient scaffold)

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| SOAP auth replay | CW | -- | wireclient/src/ | -- |
| Mercury phase-3 handshake | CW | -- | wireclient/src/ | -- |
| Pcap+key replay | CW | -- | wireclient/src/ + tools/pcap_to_session.py | -- |
| Session-trace JSONL | CW | -- | wireclient/src/ | -- |
| LossyTransport drop/dup/reorder/latency | CW | -- | mercury/lossy_transport.rs | -- |
| Loopback paired-channel tests | CW | -- | mercury/test_harness/ | 22 tests |
| Network-chaos scenarios | CW | -- | mercury/test_harness/tests/chaos/ | -- |

### 35. Discord Notifications --- CW

- **Confidence**: HIGH
- **Documentation**: [architecture/discord-notifications.md](architecture/discord-notifications.md)
- **Rust code**: [`crates/discord/`](../crates/discord/)
- **Recent PRs**: #397 (notification crate + tracing-layer harvest + panic hook)

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| EventKind enum | CW | -- | discord/src/event.rs | Variant-count pinning test |
| Channel routing | CW | -- | discord/src/router | channel_for() |
| Embed formatting | CW | -- | discord/src/embed | format_event() |
| Panic hook capture | CW | -- | discord/src/ | -- |
| Per-channel toggles | CW | -- | EventToggles | -- |
| Colo deploy wiring | CW | -- | docker/compose.discord.yml | -- |

### 36. Tauri Admin App + Tools --- IM

- **Confidence**: MEDIUM
- **Documentation**: [tools/admin-api.md](tools/admin-api.md), [tools/admin-panel.md](tools/admin-panel.md), [client/sgw-launcher.md](client/sgw-launcher.md)
- **Rust code**: [`crates/admin-api/`](../crates/admin-api/) (axum REST + WS), [`tools/`](../tools/) (Tauri apps: content editor, scene editor, admin panel), [`crates/launcher/`](../crates/launcher/) (sgw-launcher, egui native)
- **Path forward**: Three.js space viewer (Phase 1 of the rewrite plan); per-page feature polish.

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| REST admin API | IM | -- | crates/admin-api/ | Routes for entities, spaces, content, players, config |
| WebSocket entity stream | IM | -- | admin-api/ws/ | -- |
| WebSocket log stream | IM | -- | admin-api/ws/ | -- |
| JWT auth for remote | IM | -- | admin-api/routes/auth.rs | -- |
| Content editor (Tauri) | IM | -- | tools/ | React + xyflow visual chain editor |
| Scene editor (Tauri) | IM | -- | tools/ | -- |
| Admin panel (Tauri) | IM | -- | tools/ | -- |
| SGW launcher (egui) | CW | -- | crates/launcher/ | Seed + patch manifest, Ed25519 signed |
| Three.js space viewer | KM | -- | -- | Phase 2 of the admin UI plan |

---

## Server Infrastructure (Cross-Cutting)

### Session Management --- IM

- **Documentation**: [architecture/server-systems.md](architecture/server-systems.md)
- **Rust code**: `crates/services/src/base/connect_loop/` handles per-client lifecycle

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Inactivity timeout | IM | -- | config | 300,000ms (5 min) |
| Duplicate login check | IM | -- | base/connect_loop/ | At char select |
| Developer mode bypass | CW | -- | config | Disables duplicate check |
| Reconnection grace period | KM | -- | -- | Instant disconnect = session lost |
| Session token persistence | KM | -- | -- | No resume after network blip |
| Continuous auth validation | KM | -- | -- | Only at login |

### Rate Limiting --- KM

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Ability cooldown enforcement | CW | -- | cell/abilities/ | Per-ability timers |
| Chat flood protection | KM | -- | -- | No rate limit on messages |
| Action throttling | KM | -- | -- | No per-action rate tracking |
| Trade request spam | KM | Trade | -- | No request cooldown |
| Login attempt limiting | KM | -- | -- | No brute-force protection |

### Anti-Cheat Validation --- KM

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Position bounds check | IM | -- | entity/movement.rs | Within space only |
| Ability target validation | IM | -- | cell/abilities/use_ability/ | Target exists + alive |
| Inventory ownership check | CW | -- | base/world_entry/methods/inventory | Live-DB regression guards |
| Speed hack detection | KM | -- | -- | Client-authoritative movement |
| Teleport detection | KM | -- | -- | No position delta tracking |
| Damage sanity check | KM | -- | -- | No max-damage cap |
| Action-at-distance exploit | KM | -- | -- | Can fire from any range if client bypassed |

### Economy Sinks / Faucets --- IM

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Vendor buy/sell prices | CW | -- | base/world_entry/methods/vendor/ | Static from DB |
| Mission cash rewards | CW | -- | cell/content/executor/ | content-engine action |
| Loot cash drops | CW | -- | cell/abilities/loot_drop.rs | LOOT_Cash type |
| Repair costs | CW | -- | vendor/paid_repair/ | Cost formula |
| Recharge costs | CW | -- | vendor/paid_recharge/ | Cost formula |
| AH listing fees | KM | Black Market | -- | -- |
| Cash flow tracking | KM | -- | -- | No monitoring beyond logs |

### World State Persistence --- IM

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Player position persistence | CW | -- | sqlx | sgw_player pos columns |
| Cell event outbox | CW | -- | base/outbox/ | Durable Base→Cell |
| Space scripts | IM | -- | content-engine | Reset on restart |
| Gate state persistence | KM | DB | -- | Open/closed not saved |
| Door state persistence | KM | DB | -- | Not saved |
| World state table | KM | DB | -- | No sgw_world_state |

### Event / Scheduler System --- IM

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Per-entity timers | IM | -- | content-engine | Per-chain timers wire through |
| Global event scheduler | KM | -- | -- | No cron-like system |
| Daily resets | KM | Scheduler | -- | -- |
| Holiday events | KM | Scheduler | -- | -- |

### Admin / GM Tools --- IM

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Admin API (REST) | IM | -- | crates/admin-api/ | -- |
| Tauri admin panel | IM | -- | tools/ | Per-page features partial |
| Access level system | IM | -- | account.access_level | DB column exists |
| Python console | KM | -- | -- | C++ console not ported (intentional security) |
| Console commands | IM | -- | crates/commands/ | Command framework exists |
| Player info lookup | IM | -- | admin-api/routes/players.rs | -- |
| Ban/mute system | KM | -- | -- | -- |
| Teleport command | KM | Console | -- | -- |
| Item grant | KM | Console | -- | -- |
| Action logging | IM | -- | tracing + OTLP | Via observability pipeline |
| Announcement broadcast | KM | Chat | -- | -- |

### Metrics / Telemetry --- CW

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Category logging | CW | -- | tracing crate | -- |
| OTLP export | CW | -- | server/main.rs | -- |
| Mercury packet metrics | CW | -- | mercury/instrumentation.rs | -- |
| Player count tracking | IM | -- | admin-api/ | Exposed via REST |
| Performance metrics | IM | -- | tracing + SigNoz | Visible in dashboards |
| Custom dashboards | IM | -- | SigNoz | -- |
| Cloudflare-Access remote ops | CW | -- | operations/signoz-remote-access.md | -- |

---

## Summary Completion Matrix

| # | System | Total | CW | NT | IM | KM | NU |
|---|--------|-------|----|----|----|----|-----|
| 1 | Authentication | 11 | 6 | 0 | 3 | 2 | 0 |
| 2 | Mercury Protocol | 13 | 11 | 0 | 0 | 2 | 0 |
| 3 | Game Data Pipeline | 7 | 6 | 0 | 0 | 1 | 0 |
| 4 | Database Persistence | 8 | 7 | 0 | 0 | 1 | 0 |
| 5 | Character Creation | 11 | 0 | 8 | 0 | 3 | 0 |
| 6 | World Entry | 9 | 7 | 0 | 2 | 0 | 0 |
| 7 | Movement | 9 | 1 | 0 | 4 | 4 | 0 |
| 8 | Entity Lifecycle (AoI) | 9 | 7 | 0 | 1 | 1 | 0 |
| 9 | Combat & Abilities | 23 | 5 | 0 | 15 | 3 | 0 |
| 10 | Effects & Buffs | 13 | 4 | 0 | 7 | 2 | 0 |
| 11 | Stats | 8 | 5 | 0 | 0 | 2 | 1 |
| 12 | Inventory | 13 | 9 | 2 | 0 | 2 | 0 |
| 13 | Missions | 12 | 8 | 0 | 2 | 2 | 0 |
| 14 | Loot | 9 | 2 | 2 | 2 | 3 | 0 |
| 15 | Vendors | 8 | 2 | 5 | 1 | 0 | 0 |
| 16 | NPC AI | 22 | 6 | 0 | 5 | 11 | 0 |
| 17 | Spawn System | 23 | 7 | 0 | 11 | 4 | 1 |
| 18 | XP & Leveling | 11 | 9 | 0 | 0 | 2 | 0 |
| 19 | Crafting | 9 | 0 | 0 | 0 | 9 | 0 |
| 20 | Stargate Travel | 10 | 2 | 0 | 5 | 3 | 0 |
| 21 | Chat | 9 | 0 | 1 | 0 | 8 | 0 |
| 22 | Trading | 8 | 0 | 0 | 0 | 8 | 0 |
| 23 | Organizations | 15 | 0 | 0 | 0 | 15 | 0 |
| 24 | Mail | 13 | 0 | 0 | 8 | 4 | 1 |
| 25 | Black Market | 10 | 0 | 0 | 0 | 9 | 1 |
| 26 | Contact Lists | 8 | 0 | 0 | 0 | 8 | 0 |
| 27 | Dueling | 6 | 0 | 0 | 0 | 6 | 0 |
| 28 | Pets | 7 | 0 | 0 | 0 | 7 | 0 |
| 29 | Minigames | 9 | 0 | 0 | 4 | 5 | 0 |
| 30 | Groups | 7 | 0 | 0 | 0 | 7 | 0 |
| 31 | Content Engine | 10 | 6 | 0 | 0 | 4 | 0 |
| 32 | Mercury Bundle | 5 | 5 | 0 | 0 | 0 | 0 |
| 33 | Observability | 8 | 8 | 0 | 0 | 0 | 0 |
| 34 | Wireclient + Chaos | 7 | 7 | 0 | 0 | 0 | 0 |
| 35 | Discord Notifications | 6 | 6 | 0 | 0 | 0 | 0 |
| 36 | Tauri Admin App + Tools | 9 | 1 | 0 | 7 | 1 | 0 |
| -- | Session Mgmt | 6 | 1 | 0 | 2 | 3 | 0 |
| -- | Rate Limiting | 5 | 1 | 0 | 0 | 4 | 0 |
| -- | Anti-Cheat | 7 | 1 | 0 | 2 | 4 | 0 |
| -- | Economy | 7 | 5 | 0 | 0 | 2 | 0 |
| -- | World State | 6 | 2 | 0 | 1 | 3 | 0 |
| -- | Scheduler | 4 | 0 | 0 | 1 | 3 | 0 |
| -- | Admin/GM | 11 | 0 | 0 | 5 | 6 | 0 |
| -- | Metrics / Telemetry | 7 | 4 | 0 | 3 | 0 | 0 |
| | **TOTALS** | **437** | **139** | **18** | **91** | **184** | **5** |

### Summary Percentages

| Status | Count | Percentage |
|--------|-------|-----------|
| Confirmed Working (CW) | 139 | 31.8% |
| Needs Test (NT) | 18 | 4.1% |
| Implemented (IM) | 91 | 20.8% |
| Known/Missing (KM) | 184 | 42.1% |
| Needed/Unknown (NU) | 5 | 1.1% |

**Code exists (CW + NT + IM)**: 248 features (56.8%)
**Missing (KM + NU)**: 189 features (43.2%)

**Tested end-to-end (CW)**: 139 features (31.8%) — up from 8.4% in the audit-era (deprecated-codebase) measurement, primarily because infrastructure / observability / content engine / wireclient that ship-or-don't-ship are firmly in the CW column.

---

## What changed since the previous (deprecated-codebase) gap analysis

| Metric | Audit (Python+C++) | This rewrite (Rust) | Why the change |
|---|---:|---:|---|
| Total features tracked | 369 | 437 | New systems (content engine, observability, wireclient, discord, Mercury bundle), some sub-feature splits |
| Confirmed Working | 31 (8.4%) | 139 (31.8%) | Mercury, AoI, infrastructure, observability, wireclient now CW; many "NT in Python" features are CW in Rust because live-DB regression guards exist |
| Code exists (CW+NT+IM) | 175 (47.4%) | 248 (56.8%) | Content engine, much of cell/, full vendor and inventory pipelines, full ability+effect system (PR #420), spawn lifecycle |
| Missing | 194 (52.6%) | 189 (43.2%) | Several Python systems weren't ported (crafting, trading); but new systems landed elsewhere; net: roughly same number of gaps, different shape |

The 47% → 57% shift overstates the gap closure on its own. The real story is that **the shape of "done" changed**: Mercury, observability, and the content engine are firmly done; crafting and trading regressed (never ported); and most of the long-tail social systems (org, mail, AH, contacts, duel, pets, groups) are still in the "stub handlers exist but nothing works" state.

---

## Critical Path for Playability

The minimum viable gameplay loop is in much better shape than the audit-era assessment. Remaining critical-path items (in priority order):

1. **Effect-script content coverage** — the framework is CW (PR #420) but the long tail of the 3,217 effect rows still need scripts
2. **Mission XP** — `mission.reward_xp` is 0 in all seed rows; chain-side authoring + `Action::GrantXP` wiring needed
3. **NPC navigation states** — patrol / wander / leash blocked on full Detour wiring
4. **Crafting port** — full subsystem missing from Rust
5. **Trading port** — full subsystem missing from Rust
6. **Multi-zone end-to-end** — only Castle Cellblock is routinely smoked; the other 23 spaces need verification

Quality-of-life items (organizations, mail polish, black market, contact lists, dueling, pets, minigame port, groups, GM tools) are still gated on the above but each can be picked up independently.

---

## Cross-Reference Tables

### Documentation Exists but Rust Doesn't (port pending)

| System | Gameplay Doc | Wire Format Doc | Rust Code Status |
|--------|-------------|----------------|-------------------|
| Crafting | crafting-system.md | crafting-wire-formats.md | Not ported |
| Trading | trade-system.md | trade-wire-formats.md | Not ported |
| Organizations | organization-system.md | organization-wire-formats.md | 200 lines stubs |
| Black Market | black-market.md | black-market-wire-formats.md | 94 lines stubs |
| Contact Lists | contact-list.md | contact-list-wire-formats.md | 86 lines stubs |
| Dueling | duel-system.md | duel-wire-formats.md | Not ported |
| Pets | pet-system.md | pet-wire-formats.md | Not ported |
| Groups | group-system.md | group-wire-formats.md | Not ported |

### Rust Code Exists but Doc Lags

These have substantial Rust implementations the per-system docs haven't fully caught up on. P3-equivalent doc-refresh pending.

| System | Code Location | Doc Status |
|--------|--------------|-----------|
| Content Engine | crates/services/src/cell/content/ + crates/content-engine/ | docs/content/content-engine.md is the canonical reference but is currently labelled audience: "engineers" — could use a "what's done vs. planned" callout |
| Mercury Bundle | crates/mercury/src/channel_bundle.rs | docs/architecture/mercury-bundle.md is the ADR |
| Observability | crates/server/, crates/mercury/instrumentation.rs | docs/architecture/observability.md + operations/signoz-*.md |
| Wireclient | crates/wireclient/ | docs/architecture/wireclient.md |
| Discord Notifications | crates/discord/ | docs/architecture/discord-notifications.md |

### Server-Only Blind Spots (Ranked by Gameplay Impact)

| Rank | System | Impact | Status | Notes |
|------|--------|--------|--------|-------|
| 1 | NPC Navigation | HIGH — mobs can't patrol/leash | KM | Detour FFI exists, runtime unwired |
| 2 | Crafting | HIGH — entire skill tree unplayable | KM | Python `Crafter.py` 575 lines untransplanted |
| 3 | Trading | MEDIUM — players can mail but not trade | KM | Python `Trade.py` 244 lines untransplanted |
| 4 | Rate Limiting | MEDIUM — exploitable | KM | No throttle on chat/trade-request/login |
| 5 | Speed-hack detection | MEDIUM — client-authoritative | KM | -- |
| 6 | Mission XP integration | LOW — XP works from kills only | KM | Chain authoring needed |
| 7 | Multi-zone verification | LOW — 1 zone routinely smoked | NT | 23 spaces unchecked |

---

## Related Documents

- [project-status.md](project-status.md) — human-readable summary of this analysis
- [../README.md](../README.md) — high-level project status
- [gameplay/](gameplay/) — per-system gameplay docs
- [content/](content/) — content audit + content engine
- [protocol/](protocol/) — wire formats
- [architecture/](architecture/) — server architecture and ADRs
- [reverse-engineering/](reverse-engineering/) — RE findings + Ghidra work
- [../CONTRIBUTING.md](../CONTRIBUTING.md) — how to pick a feature and ship it
