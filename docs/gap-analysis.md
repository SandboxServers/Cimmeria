---
title: "Gameplay Systems Gap Analysis"
type: explanation
audience: engineers
last_updated: 2026-07-25
---

# Gameplay Systems Gap Analysis

> **Last updated**: 2026-07-25 (re-verification pass against `main` — 168 commits landed since the 2026-05-27 edition)
> **Purpose**: Map every gameplay system's Rust implementation against what's needed for a complete server
> **Status**: Source of truth for project completion tracking
> **Measured against**: `main`. Work living only on an unmerged feature branch is called out explicitly in the affected section and is **not** counted as implemented.
> **Workspace scale**: **2,988 tests across 471 files**, **259 live-DB regression guards**, **3 PL/pgSQL end-to-end smokes**, with a **first-class content engine** the original Python codebase did not have.
>
> **Arithmetic note**: the 2026-05-27 edition's `TOTALS` row and headline percentages did not match its own per-system table. The rows summed to 428 / CW 151 / NT 18 / IM 91 / KM 164 / NU 4, while the headline claimed 437 / CW 139 / NT 18 / IM 91 / KM 184 / NU 5. This edition recomputes the totals directly from the rows; anyone quoting the old 437 / 31.8% figures was quoting a number the table never supported.

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
- **Rust code**: [`crates/services/src/auth/`](../crates/services/src/auth/) — 3,118 lines (handlers.rs, login_smoke.rs, mod.rs, service.rs, tls.rs, tls_smoke.rs, cert_watcher.rs) including a live-DB login smoke
- **Recent PRs**: #414 (auth + base + world-entry pipeline instrumentation), #366 (dev-session telemetry HMAC), #566 (auth TLS listener), #577 (cert mtime watcher + hot reload)
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
| TLS auth listener + cert hot-reload | IM | -- | auth/tls.rs, auth/cert_watcher.rs | `TlsCertStore::reload` swaps only on successful rebuild (cert_watcher.rs:91-118); PRs #566/#577 |
| Continuous auth validation | KM | -- | -- | Only checked at login |
| Login rate limiting | KM | -- | -- | No brute-force protection |

### 2. Mercury Protocol --- CW

- **Confidence**: HIGH
- **Documentation**: [drafts/spec/mercury-wire-format.md](drafts/spec/mercury-wire-format.md) (canonical, in-progress bible chapter), [protocol/mercury-wire-format.md](protocol/mercury-wire-format.md) (legacy summary), [architecture/transport-trait.md](architecture/transport-trait.md), [architecture/mercury-bundle.md](architecture/mercury-bundle.md), [architecture/mercury-loopback-harness.md](architecture/mercury-loopback-harness.md), [architecture/network-chaos-testing.md](architecture/network-chaos-testing.md)
- **Rust code**: [`crates/mercury/`](../crates/mercury/) — 14,066 lines across 66 files, **260 tests** (bundle, channel/, channel_bundle, clock, codec, encryption/, instrumentation, lossy_transport, messages, test_harness/, test_transport, transport, unified, unpacker/)
- **Recent PRs**: #358 (Transport trait), #361 (ChannelBundle), #363/#365 (bundle progression), #370 (loopback harness, 22 paired-channel tests), #374 (network chaos), #404 (wire-log capture), #410 (mercury backpressure), #415 (warn! on unhandled dispatch), **#566 (v2 crypto foundation), #575 (v2 session-key rotation)**
- **Path forward**: Piggyback ACK optimization remains. Mercury v2 needs a patched client to verify against — no shipping client speaks it ([`encryption/mod.rs`](../crates/mercury/src/encryption/mod.rs) L125-136). Bible chapter promotion (draft → verified).

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Reliable UDP transport | CW | -- | mercury/channel_bundle.rs | Sequence numbers, ACK/NAK, retransmit |
| AES-256-CBC encryption (v1) | CW | -- | mercury/encryption/mod.rs | Audit was wrong about Blowfish; the stock client speaks AES-256-CBC + HMAC-MD5. v1 is the `#[default]` (encryption/mod.rs:129-136) — **do not change v1 output bytes** |
| Message framing | CW | -- | mercury/bundle.rs | Header + variable-length body, per-channel sequence |
| Ordered delivery | CW | -- | mercury/channel_bundle.rs | Per-channel sequence |
| Fragmentation + reassembly | CW | -- | mercury/lib.rs | Large message splitting + reassembly |
| ChannelBundle accumulator | CW | -- | mercury/channel_bundle.rs | Cross-entity bundling, AoI burst migration |
| Transport trait + TestTransport | CW | -- | mercury/transport.rs | Wire-seam for byte-exact fan-out tests |
| LossyTransport (chaos) | CW | -- | mercury/lossy_transport.rs | Drop / dup / reorder / latency primitives |
| Loopback session harness | CW | -- | mercury/test_harness/ | Tier 2 paired-channel end-to-end tests |
| Pcap replay (wireclient) | KM | Wireclient socket loop | -- | **Corrected 2026-07-25.** No replay engine exists. `crates/wireclient` has no `UdpSocket` anywhere; `client.rs:56-59` defers the socket loop to an unbuilt "Phase 1.5". `tools/pcap_to_session.py` converts pcap→JSONL; nothing replays it. See §34 |
| Observability instrumentation | CW | -- | mercury/instrumentation.rs | Per-packet OTLP spans, SigNoz integration |
| Mercury v2 encryption | IM | -- | mercury/encryption/mod.rs | Per-packet random IV, HKDF-SHA256-split enc/mac keys (`v2_derive_keys`, mod.rs:175), 16-byte-truncated HMAC-SHA256, v1→v2 downgrade defense (mod.rs:5-7). **Untested against a live client** — opt-in only |
| Mercury v2 session-key rotation | IM | v2 | mercury/encryption/mod.rs:64 | Server-initiated rotation, v2 sessions only; 219 lines of harness coverage in test_harness/tests/rotation.rs. Same untested-against-client caveat |
| Cumulative ACKs | IM | -- | mercury/channel/channel_core.rs | **Corrected 2026-07-25.** `process_acks` drains the whole TX window plus the unsent queue in one pass (channel_core.rs:207-236); test_harness/tests/ack.rs:46 and channel/tests/reassembly.rs:321-325 pin it |
| Piggyback ACKs | KM | -- | -- | Still missing — no match for `piggyback` outside doc comments |

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
| GM character creation | IM | -- | mercury/world_data/phases.rs:46 | **Corrected 2026-07-25.** SGWGmPlayer is ported: seed accounts get `access_level`, and a GM enters the world as entity class `0x03` instead of `0x02` (PRs #473 / #516 / #518, merged 2026-06-17). No GM-only *creation* UI |
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

- **Confidence**: HIGH (re-read 2026-07-25 — the Detour FFI is now wired end-to-end and the validation layers are live)
- **Documentation**: [protocol/position-updates.md](protocol/position-updates.md), [drafts/spec/position-updates.md](drafts/spec/position-updates.md) (canonical bible draft)
- **Rust code**: [`crates/entity/src/movement.rs`](../crates/entity/src/movement.rs) (351), [`crates/entity/src/navigation/`](../crates/entity/src/navigation/) (1,392 — mod.rs, xrc.rs, tests.rs), [`crates/entity/src/movement_validation/`](../crates/entity/src/movement_validation/) (780 — mod.rs, bounds.rs), [`crates/entity/src/detour_ffi.rs`](../crates/entity/src/detour_ffi.rs) (114); Detour is compiled from source by [`crates/entity/build.rs`](../crates/entity/build.rs)
- **Recent PRs**: **#437 (bounds-check + snap-back, #63 PR1)**, **#478 (speed / teleport / navmesh / spaceId validation on AVATAR_UPDATE_EXPLICIT)**, **#428 (NPC AI phases 2-7 — the consumers of pathfinding)**, #426/#436 (navmesh-extractor crate), #432 (harden `NavMesh::load` against malicious `.nav`)
- **Path forward**: Promote the speed layer from warn-only to enforcing once SigNoz rejection telemetry calibrates the tolerance ([`movement_validation/mod.rs`](../crates/entity/src/movement_validation/mod.rs) L17-23).

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Client position updates | CW | -- | services/cell/cell_methods | playerUpdate accepted |
| Dead reckoning / interpolation | IM | -- | entity/movement.rs | Extrapolates between updates |
| Space bounds check | IM | -- | entity/movement_validation/bounds.rs | AABB + NaN/infinity + Z-floor-clip; wired at cell/space_manager/entities.rs:288 |
| NavMesh / Detour FFI | IM | -- | entity/detour_ffi.rs | FFI wrapper; `detour_raycast` (detour_ffi.rs:72) consumed by `SpaceManager::has_line_of_sight` (cell/space_manager/spatial.rs:15-36) |
| NPC pathfinding | IM | -- | entity/navigation/mod.rs | 627 lines — path query, `raycast` (mod.rs:443), point validation; off-mesh start projection handled |
| NPC waypoint movement | IM | -- | cell/service/npc_ai/patrol.rs, wander.rs | **Corrected 2026-07-25.** Waypoint stepping is live in the patrol and wander handlers (PR #428) |
| NPC patrol | IM | -- | cell/service/npc_ai/patrol.rs | **Corrected 2026-07-25.** 215 lines; `AiState::Patrol` dispatched at cell/service/npc_ai/dispatch.rs:86 |
| Server-side speed validation | IM | -- | entity/movement_validation/mod.rs | **Corrected 2026-07-25.** `check_kinematics` computes `\|new-last\|/dt` from the server's monotonic clock (never a client timestamp). Deliberately **warn-only** pending tolerance calibration; wired at cell/space_manager/entities.rs:314 (PR #478) |
| Teleport detection | IM | -- | entity/movement_validation/mod.rs | **Corrected 2026-07-25.** Dual gate — distance > `TELEPORT_JUMP_UNITS` **and** implied speed > `top_speed × TELEPORT_SPEED_FACTOR` — hard-rejects with snap-back via `CellToBaseMsg::TeleportPlayer`; authorized teleports are excused by `note_authorized_teleport` (entities.rs:364) |

### 8. Entity Lifecycle (AoI) --- CW

- **Confidence**: MEDIUM — **downgraded 2026-07-25.** The witness-list *discipline* is well established, but there is a known-open delivery defect (below) that the 2026-06-20 colo repro did not explain. Do not plan against "AoI is done".
- **Documentation**: [engine/entity-lod-system.md](engine/entity-lod-system.md), [engine/entity-type-catalog.md](engine/entity-type-catalog.md)
- **Rust code**: [`crates/entity/src/cell_entity/`](../crates/entity/src/cell_entity/) (bandolier, state_flags, system_options, tests, mod), [`crates/entity/src/world_grid.rs`](../crates/entity/src/world_grid.rs), [`crates/entity/src/space.rs`](../crates/entity/src/space.rs), [`crates/services/src/base/world_entry/cell_dispatch/aoi.rs`](../crates/services/src/base/world_entry/cell_dispatch/aoi.rs) (792)
- **Recent PRs**: #279 (BeingAppearance recomposite broadcast — design issue still open), #418 (generate_threat refreshes appearance on first-add), #408/#410 (AoI burst migration), **#580 (player combat + death state fanned out to witnesses — closes #232)**, **#582 (`aoi.create_emit` / `aoi.create_send_failed` observability seams)**

> **Open defect — invisible entity until relog.** In Castle Cellblock a GuardBody corpse is not visible to a player until they relog. The 2026-06-20 colo repro **disproved** the address-gate hypothesis (the warns never fired), which puts the drop downstream in create + appearance delivery. PR #582 added the `aoi.create_emit` (DEBUG) / `aoi.create_send_failed` (WARN) seams at [`cell_dispatch/aoi.rs:26-27`](../crates/services/src/base/world_entry/cell_dispatch/aoi.rs) to localise it on the next repro. Until that lands, treat entity-introduction delivery as unproven.

- **Path forward**: Close the invisible-entity defect; finish the BeingAppearance fanout-helper consolidation (issue #278, parent of #219/#232/#240/#249/#270 — #232 closed by #580).

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Entity creation | CW | -- | entity/manager.rs | From template or dynamic |
| Entity destruction | CW | -- | entity/manager.rs | Cleanup + witness notification |
| Grid-based AoI | CW | -- | entity/world_grid.rs | Chunk-based witness management |
| Witness enter/leave | IM | -- | entity/cell_entity/mod.rs | **Downgraded 2026-07-25.** onEnter/onLeave fire, but entity-introduction delivery to a witness has a known-open drop (invisible GuardBody corpse until relog — see the callout above). #582 instrumentation pending next repro |
| Property synchronization | CW | -- | entity/properties.rs | Per-distribution-flag write paths |
| State flag conventions | CW | -- | entity/cell_entity/state_flags.rs | bStateField, BSF_InCombat lifecycle |
| Bandolier state | CW | -- | entity/cell_entity/bandolier.rs | Slot lifecycle, type_id vs item_id discipline |
| LOD system | KM | -- | -- | No entity detail levels |
| Witness-fanout helper | IM | -- | services/cell/ | PR #580 converted five own-client-only emit paths (onEffectResults, onStatUpdate, BSF_InCombat, death onSequence, corpse flip) to `send_entity_method_to_self_and_witnesses`. Issue #278 consolidation still incomplete elsewhere |

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
| LOS checks | IM | -- | cell/space_manager/spatial.rs:15 | **Corrected 2026-07-25.** `SpaceManager::has_line_of_sight` raycasts the navmesh (spatial.rs:36) and is enforced on the NPC firing path (cell/service/npc_ai/fight.rs:241) and exposed as the `testLOS` GM command (cell_methods/gm/query.rs:123). **Not yet enforced on player `useAbility`** — that call site checks range but not LOS |
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
| Per-player eligibility | KM | Groups | -- | **Corrected 2026-07-25 (downgrade).** The prior "eligiblePlayerList exists" claim does not hold: there is no eligibility list anywhere in `crates/` — `grep -rn "eligible" crates/ --include=*.rs` returns only unrelated Mercury retransmit and bandolier-eligibility hits. Looting is gated on distance only (re-validated per `lootItem` by PR #446) |
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

- **Confidence**: HIGH for the state machine (re-read 2026-07-25), MEDIUM for tuning
- **Documentation**: [gameplay/npc-ai.md](gameplay/npc-ai.md)
- **Rust code**: [`crates/services/src/cell/service/npc_ai/`](../crates/services/src/cell/service/npc_ai/) — **1,818 lines** split per behavior state (dispatch, fight, ability_select, patrol, wander, investigate, follow, leash, lifecycle); [`crates/services/src/cell/cover/`](../crates/services/src/cell/cover/) — **4,095 lines** (detection, scoring, reservation, spatial, ai_integration, loader); [`crates/services/src/cell/spawner/`](../crates/services/src/cell/spawner/) (2,448); [`crates/services/src/cell/content/`](../crates/services/src/cell/content/) (content-engine drives behavior chains)
- **Recent PRs**: #368 (NPC ability buckets + auto-aggro + Castle drone encounter — closes #342), #418 (generate_threat content action refreshes appearance on first-add), **#428 (NPC AI phases 2-7: Patrol + Wander + Investigating + Follow + Despawning/Submit/Error)**, **#429 (server-driven NPC cover + reservation + player UI + flanking + Castle Cellblock demo — closes #209)**
- **Path forward**: Tuning and content authoring for the movement states (hearing radius, mob groups, kill-credit tapping remain unimplemented). The **navigation blocker is closed** — patrol/wander/investigate/follow all run against the live Detour navmesh.

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
| Investigating state | IM | -- | cell/service/npc_ai/investigate.rs | **Corrected 2026-07-25.** 148 lines; `AiState::Investigating` dispatched at npc_ai/dispatch.rs:88; POI set by content action `Action::SetNpcPoi` (PR #428) |
| Leashing state | IM | -- | cell/service/npc_ai/leash.rs | 81 lines — snap home + heal; dispatched at npc_ai/dispatch.rs:85 |
| Patrol state | IM | -- | cell/service/npc_ai/patrol.rs | **Corrected 2026-07-25.** 215 lines; Idle auto-promotes to Patrol when `has_patrol` (npc_ai/dispatch.rs:109) |
| Wander state | IM | -- | cell/service/npc_ai/wander.rs | **Corrected 2026-07-25.** 200 lines; rejects off-mesh candidates via `space_mgr.is_position_valid` (wander.rs:163); Idle auto-promotes on `has_wander` (dispatch.rs:114) |
| Follow state | IM | -- | cell/service/npc_ai/follow.rs | **Corrected 2026-07-25.** 125 lines; target set by content action `Action::SetFollowTarget` |
| Despawning state | IM | -- | cell/service/npc_ai/lifecycle.rs | **Corrected 2026-07-25.** Terminal states (Despawning / Submit / Error) dispatched at npc_ai/dispatch.rs:90-92 |
| Cover system | IM | -- | cell/cover/ | **Corrected 2026-07-25.** 4,095 lines: node loader (loader.rs + live-DB tests), spatial index, scoring, per-node reservation so two NPCs don't claim one slot, flanking, and NPC AI integration (ai_integration.rs, 521 lines). Driven per tick from cell/service/ticks/cover.rs. Castle Cellblock demo chain in content/chain_replay_tests/cover_demo.rs (PR #429) |
| Hearing system | KM | -- | -- | hearingRadius defined; no runtime consumer |
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

### 19. Crafting --- KM (Phase 1 landed; the crafting *verbs* are still stubs)

- **Confidence**: HIGH (code read 2026-07-25)
- **Documentation**: [gameplay/crafting-system.md](gameplay/crafting-system.md), [reverse-engineering/findings/crafting-wire-formats.md](reverse-engineering/findings/crafting-wire-formats.md)
- **Rust code**: [`crates/entity/src/crafting.rs`](../crates/entity/src/crafting.rs) (191 — `CraftingState`), [`crates/services/src/base/crafting/`](../crates/services/src/base/crafting/) (1,103 — handlers.rs, persistence.rs), [`crates/services/src/cell/cell_methods/player/crafting.rs`](../crates/services/src/cell/cell_methods/player/crafting.rs) (232 — method routing), [`crates/services/src/cell/console/crafting.rs`](../crates/services/src/cell/console/crafting.rs) (136)
- **Recent PRs**: **#427 (Phase 1 — `CraftingState` + persistence + ASP dispatch fix, #53)**, #521 (GM crafting grants)
- **State of play**: Phase 1 shipped the *state* layer — `sgw_player.discipline_ids` / `blueprint_ids` / `applied_science_points` / `racial_paradigm_levels` load and save transactionally, and GM grant commands drive expertise. Every player-facing crafting verb still logs `UNIMPLEMENTED` (cell_methods/player/crafting.rs:45-86).
- **Path forward**: Phase 2 — ASP-spend validation (paradigm gate + prerequisite expertise + DB UPDATE), then the craft / research / reverse-engineer / alloy flows.

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Craft from blueprint | KM | -- | stub | `CRAFT` arm parses `craft_id` then logs `UNIMPLEMENTED: craft` (cell_methods/player/crafting.rs:45-57) |
| Research item | KM | -- | stub | `UNIMPLEMENTED: research` (crafting.rs:59-62) |
| Reverse engineer | KM | -- | stub | `UNIMPLEMENTED: reverseEngineer` (crafting.rs:64-67) |
| Alloy | KM | -- | stub | `UNIMPLEMENTED: alloying` (crafting.rs:69-81) |
| Discipline learning | KM | -- | stub | `spendAppliedSciencePoints` routes but does not mutate — "Phase 1: route only" (crafting.rs:23-44) |
| Expertise system (0-100) | IM | -- | entity/crafting.rs, base/crafting/handlers.rs:36 | **Corrected 2026-07-25.** `set_expertise` with an explicit `[0,100]` clamp, first-grant discipline registration, transactional save, and an `onUpdateDiscipline` client push (handlers.rs:86-140) |
| Racial paradigm gating | KM | -- | state only | `racial_paradigm_levels` loads/saves as a `{paradigm_id → level}` map (persistence.rs:132-152) but **no gate function consumes it** |
| Blueprint management | IM | -- | base/crafting/persistence.rs:91 | **Corrected 2026-07-25.** `blueprint_ids` round-trips through `load_crafting_state` / `save_crafting_state`; no acquire/dedupe verbs yet |
| Crafting respec | KM | -- | stub | `UNIMPLEMENTED: respecCrafting` (crafting.rs:83-86) |

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
- **Rust code**: [`crates/services/src/cell/chat.rs`](../crates/services/src/cell/chat.rs) (447), [`crates/services/src/base/dispatch/chat.rs`](../crates/services/src/base/dispatch/chat.rs) (198), [`crates/services/src/base/world_entry_chat.rs`](../crates/services/src/base/world_entry_chat.rs) (237), [`crates/game/src/social/chat.rs`](../crates/game/src/social/chat.rs) (46) — 928 non-test lines
- **Path forward**: Message *routing* on the non-spatial channels (the channels are registered but carry no traffic); direct tells; admin/moderation tools.

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Say/emote/yell (AoI) | NT | AoI | cell/chat.rs | Witness broadcast |
| Direct tells | KM | -- | -- | `sendPlayerCommunication` parses the `target` WSTRING then discards it — every message is forwarded to the cell as a spatial broadcast (base/dispatch/chat.rs:45-108) |
| User channels | KM | -- | -- | requestCreateChannel not ported |
| Pre-defined channels | IM | -- | base/world_entry_chat.rs:20 | **Corrected 2026-07-25.** All 8 canonical channels (say/emote/yell/team/squad/command/server=7/tell=9) are auto-joined on world entry and pushed to the client as `onChatJoined` (world_entry_appearance/builders.rs:87). `chatJoin` is acknowledged as a no-op because of the auto-join (dispatch/chat.rs:113-123). **No cross-player routing on the non-spatial channels yet** |
| AFK / DND status | IM | -- | base/dispatch/chat.rs:87 | **Added 2026-07-25.** `dnd_message` sets `SPEAKER_DND` on outgoing messages, matching `Chat.py::getSpeakerFlags`; `chatSetAFKMessage` is acknowledged but the auto-reply is not implemented (dispatch/chat.rs:133-144) |
| Channel ops | KM | -- | -- | setPlayerOp not ported |
| Chat flood protection | KM | -- | -- | No rate limiting |
| Profanity filter | KM | -- | -- | No filtering |
| Mute system | KM | -- | -- | No per-player muting |
| GM broadcast | KM | Admin | -- | No system-wide message tool |

### 22. Trading --- IM (ported 2026-06; was KM)

- **Confidence**: HIGH (code read 2026-07-25)
- **Documentation**: [gameplay/trade-system.md](gameplay/trade-system.md), [reverse-engineering/findings/trade-wire-formats.md](reverse-engineering/findings/trade-wire-formats.md)
- **Rust code**: [`crates/services/src/cell/cell_methods/player/trade/`](../crates/services/src/cell/cell_methods/player/trade/) — 3,860 lines (handlers, state, handoff, wire + 5 test modules); [`crates/services/src/base/world_entry/methods/trade/`](../crates/services/src/base/world_entry/methods/trade/) — 2,441 lines (execute/mod.rs, execute/swap.rs + commit / whitelist / slot-reservation live-DB guards); [`crates/entity/src/trade.rs`](../crates/entity/src/trade.rs)
- **Recent PRs**: **#438 (player-to-player trading system, closes #54)**
- **Why IM and not CW**: the whole subsystem landed with unit + live-DB + wire coverage but has not been driven through a live client session end-to-end. A two-client smoke would move this to CW.
- **Path forward**: Live two-client smoke; trade-request spam throttle (see Rate Limiting).

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Trade initiation | IM | -- | trade/handlers.rs:36 | `TRADE_REQUEST` → `begin_trading` (state.rs:28); partners re-checked for range each transition (`partners_in_range`, state.rs:275) |
| Proposal update | IM | -- | trade/state.rs:140 | `apply_proposal`; stale-version proposals rejected (handlers.rs:373) |
| Lock state machine | IM | -- | trade/handlers.rs:268-439 | None → Locked → LockedAndConfirmed, with an explicit truth table for which transitions reset the partner's lock (handlers.rs:396-412) |
| Confirmation | IM | -- | trade/handlers.rs:439 | Commit fires only when both sides reach LockedAndConfirmed |
| Item swap | IM | Inventory | trade/execute/swap.rs:41 | `atomic_swap` — advisory lock, `SELECT … FOR UPDATE`, two-phase parked-row move, destination slot reservation |
| Cash swap | IM | -- | trade/execute/swap.rs:79-94 | Balances validated before the delta is applied (swap.rs:182-183) |
| Cancel | IM | -- | trade/state.rs:222 | `cancel_session`; `TRADE_REQUEST_CANCEL` arm at handlers.rs:40 |
| Disconnect cleanup | IM | -- | trade/state.rs:247 | `cancel_trade_on_disconnect`; regression guard at cell/service/base_messages/tests/trade_disconnect.rs |

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

- **Confidence**: HIGH (verified against `main` 2026-07-25)
- **Documentation**: [gameplay/black-market.md](gameplay/black-market.md), [reverse-engineering/findings/black-market-wire-formats.md](reverse-engineering/findings/black-market-wire-formats.md)
- **Rust code on `main`**: [`crates/services/src/cell/cell_methods/black_market.rs`](../crates/services/src/cell/cell_methods/black_market.rs) (80) + [`crates/services/src/cell/client_methods/black_market.rs`](../crates/services/src/cell/client_methods/black_market.rs) (14) — **94 lines of handler stubs, unchanged**

> **Work in flight on an unmerged branch.** `feat/571-black-market-phase1` carries a substantial Phase 1 implementation: the `sgw_auction` schema, wire deserialization, a create/bid/cancel state machine, an expiry sweep, boot-seeded active auctions listed under a reserved system seller, and the search-serve path — laid out as `base/black_market/{create,bid,cancel,search,sweep,seed,send,validate,wire,types,helpers}.rs` plus `world_entry/cell_dispatch/black_market_dispatch.rs` and a content-executor arm. **None of it is on `main`**, so every row below stays `KM` and the totals do not count it. Re-verify this section the day that branch merges.

- **Path forward**: Land `feat/571-black-market-phase1`; then buyout, my-auctions/my-bids views, listing fees, and transaction mail.

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

### 26. Contact Lists --- CW (shipped 2026-06-20; was KM)

- **Confidence**: HIGH — code read 2026-07-25, and the system is owner-confirmed working in-game as of 2026-06-20.
- **Documentation**: [gameplay/contact-list.md](gameplay/contact-list.md), [reverse-engineering/findings/contact-list-wire-formats.md](reverse-engineering/findings/contact-list-wire-formats.md)
- **Rust code**: [`crates/services/src/base/contact_list/`](../crates/services/src/base/contact_list/) — 2,851 lines across `handlers/` (header_ops, member_ops, presence_fanout), `persistence/`, `wire.rs`; dispatch at [`base/world_entry/cell_dispatch/contact_list_dispatch.rs`](../crates/services/src/base/world_entry/cell_dispatch/contact_list_dispatch.rs); cell side at [`cell/cell_methods/contact_list/`](../crates/services/src/cell/cell_methods/contact_list/)
- **Schema**: `sgw_contact_list`, `sgw_contact_list_member`, and the `list_id` sequence — all under [`db/sgw/Social/`](../db/sgw/Social/) with seed data (database.sql:367-398)
- **Recent PRs**: **#572 / #574 (schema, login-push, client methods 55-60, presence), #578 (`eventId` is a bitfield — LoggedInStatus is 1, not 0), #579 (GainLevel / Death / GateTravel), #581 (initial presence to the logging-in player), #583 (light up an already-online contact on add)**
- **Path forward**: Nothing outstanding at the feature level. The prior "86 lines of stubs" description was two months stale.

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Create list | CW | -- | contact_list/handlers/header_ops.rs:22 | `handle_create` → `persistence::create_list` (persistence/mod.rs:126); system lists ensured at login (`ensure_system_lists`, persistence/mod.rs:80) |
| Delete list | CW | -- | header_ops.rs:79 | `persistence::delete_list` (persistence/mod.rs:146); `onContactListDelete` wire builder at wire.rs:33 |
| Rename list | CW | -- | header_ops.rs:144 | `persistence::rename_list` (persistence/mod.rs:161) |
| Update flags | CW | -- | header_ops.rs:231 | `persistence::update_flags` (persistence/mod.rs:179); byte layout pinned by wire.rs:140 |
| Add members | CW | -- | handlers/member_ops.rs:23 | `persistence::add_members` (persistence/mod.rs:217), capped at `MAX_MEMBERS_PER_REQUEST = 100` (wire.rs:88) |
| Remove members | CW | -- | member_ops.rs:124 | `persistence::remove_members` (persistence/mod.rs:258) |
| Online status events | CW | -- | handlers/presence_fanout.rs:151 | `fanout_login_status` fired from base/dispatch/session.rs:88 (logout) and world_entry_appearance/client_ready.rs:398 (login). `EVENT_LOGGED_IN_STATUS = 1` — a **bitfield**, not an ordinal (wire.rs:94, fixed by #578) |
| Level-up events | CW | -- | base/world_entry/methods/progression/mod.rs:208 | `fanout_contact_event` with `EVENT_GAIN_LEVEL = 2` (wire.rs:96) |
| Death events | CW | -- | cell/abilities/death.rs:83-90 | Cell→Base `ContactListPresenceEvent` with `EVENT_DEATH = 4`; **player deaths only** — NPC deaths deliberately skip the fanout to avoid flooding the channel during combat |
| Gate-travel events | CW | -- | base/world_entry/gate_travel/mod.rs:285 | `EVENT_GATE_TRAVEL = 8`; `dataValue` carries the destination world id |

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

- **Confidence**: HIGH (code read 2026-07-25)
- **Documentation**: [gameplay/minigame-system.md](gameplay/minigame-system.md), [reverse-engineering/findings/minigame-wire-formats.md](reverse-engineering/findings/minigame-wire-formats.md)
- **Rust code**: [`crates/services/src/minigame/`](../crates/services/src/minigame/) — **2,262 lines**: `server.rs` (572, SmartFoxServer-compatible TCP listener, one tokio task + tick timer per connection), `protocol.rs` (443, SFS message codec), `session.rs` (218, ticket registry), `game.rs` (45, `MinigameInstance` trait), `games/livewire/` (877); plus [`cell/cell_methods/minigame.rs`](../crates/services/src/cell/cell_methods/minigame.rs) (174) and the cell↔base result hop
- **Correction (2026-07-25)**: the "external SmartFoxServer service is not yet running" note is obsolete. The SmartFox-compatible server is **in-process** — no separate process to deploy.
- **Path forward**: Port Alignment and GoauldCrystals from Python (explicit TODOs at games/mod.rs:13-14); the remaining six game types run on a placeholder that accepts any input.

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Minigame session start | IM | -- | cell/cell_methods/minigame.rs | Ticket issue + session registry (minigame/session.rs) |
| Seed generation | IM | -- | cell/cell_methods/minigame.rs | 0..0x7FFFFFFF |
| Result callback | IM | -- | minigame/server.rs:22-35 | `send_minigame_result` — four call sites (game-driven / tick-driven × victory / failure) share one error path; the prior silent `let _ = send` stalled quest chains |
| Mission integration | IM | Missions | content-engine OnMinigameComplete | `on_victory_chains` carried through the result hop |
| 8 game types | IM | -- | minigame/games/ | **Corrected 2026-07-25.** Livewire is fully implemented (877 lines incl. setup + tests). Hack / Activate / Analyze / Bypass / Converse / ConverseBasicHumanoid resolve to `PlaceholderGame` (accepts any input). Alignment + GoauldCrystals are commented-out TODOs (games/mod.rs:13-14) |
| Spectating | KM | -- | stub | -- |
| Co-op / help | KM | -- | stub | -- |
| Contact system | KM | -- | stub | -- |
| Minigame server (SmartFox 1.x) | IM | -- | minigame/server.rs | **Corrected 2026-07-25.** In-process TCP listener speaking the SFS protocol (`API_VERSION = 154`, `MAX_MESSAGE_LEN = 0x1000`); 250 ms tick loop drives `MinigameInstance::tick` |

### 30. Groups / Parties --- KM

- **Confidence**: STUB
- **Documentation**: [gameplay/group-system.md](gameplay/group-system.md), [reverse-engineering/findings/group-wire-formats.md](reverse-engineering/findings/group-wire-formats.md)
- **Rust code**: No group *runtime*. [`crates/game/src/social/groups.rs`](../crates/game/src/social/groups.rs) (97 lines) defines a `Group` struct and a `LootMode` enum, but as of 2026-07-25 **nothing references it** — `grep -rn "social::groups" crates/` returns no hits outside the file itself. Treat it as an unwired sketch, not a partial implementation.
- **Path forward**: Implement as lightweight Squad-type Organization; either wire `groups.rs` up or delete it.

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

### 34. Wireclient + Network Chaos Testing --- IM

- **Confidence**: HIGH (crate read line-by-line 2026-07-25)
- **Documentation**: [architecture/wireclient.md](architecture/wireclient.md), [architecture/network-chaos-testing.md](architecture/network-chaos-testing.md), [architecture/mercury-loopback-harness.md](architecture/mercury-loopback-harness.md)
- **Rust code**: [`crates/wireclient/`](../crates/wireclient/) — 1,947 lines across 8 files (auth, handshake, session_trace, client, error + 2 test files), **30 tests**; LossyTransport in mercury; loopback harness for Tier 2
- **Recent PRs**: #370 (Tier 2 loopback harness, 22 paired-channel tests), #374 (network chaos L1+L2+L3), #376 (Tier 3 wireclient scaffold)

> **Correction, 2026-07-25 — wireclient cannot send a UDP packet.** The 2026-05-27 edition marked all four wireclient rows `CW` and described Tier 3 as "headless replay against a live server". That is not what the crate does. There is **no `UdpSocket` anywhere in `crates/wireclient`** (the sole textual hit is a doc comment at handshake.rs:92), `Client::connect()` does not exist, and `client.rs:56-59` states plainly that Phase 1 "stops at *produce the bytes*" with "Phase 1.5 wires the socket loop". What genuinely works is the SOAP auth leg, byte builders/parsers for the phase-3 handshake, and a JSONL trace loader with a diff policy. The Tier 2 loopback and chaos rows below are **unaffected** — those live in `crates/mercury` and are real.

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| SOAP auth client (Phase 1+2) | IM | -- | wireclient/src/auth.rs | 357 lines; driven against an in-process `AuthService` over real TCP by tests/auth_smoke.rs. Not "replay" — it is a live SOAP client |
| Mercury phase-3 handshake | IM | Socket loop | wireclient/src/handshake.rs | 546 lines — `build_baseapp_login` + reply parser. Produces and consumes bytes; **cannot perform a handshake** because nothing sends them (client.rs:52-62) |
| Pcap+key replay | KM | Socket loop | tools/pcap_to_session.py only | **Downgraded from CW.** The Python tool converts `.pcap` + `keys.txt` → JSONL. No replay engine exists on either side |
| Session-trace JSONL | IM | -- | wireclient/src/session_trace.rs | 567 lines — `Trace::from_jsonl_path`, c2s/s2c iterators, `Diff` + `DefaultPolicy`. 10 tests + tests/trace_load.rs |
| LossyTransport drop/dup/reorder/latency | CW | -- | mercury/lossy_transport.rs | -- |
| Loopback paired-channel tests | CW | -- | mercury/test_harness/ | 22 tests |
| Network-chaos scenarios | CW | -- | mercury/test_harness/tests/chaos/ | 9 scenarios incl. `replay_lomiada`, `sustained_5pct_loss_60s`, `tx_window_overflow_with_recovery` |

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

### 37. Ring Transport --- IM

**Added 2026-07-25.** A shipped gameplay system the tracker had no row for. Cross-region and cross-world transporter rings: a player steps onto a ring pad, picks a destination, and the server drives a multi-second state machine that plays Kismet sequences at both ends, hides the players, teleports them, and re-shows them.

- **Confidence**: HIGH (module read 2026-07-25)
- **Rust code**: [`crates/services/src/cell/ring_transport/`](../crates/services/src/cell/ring_transport/) — **2,791 lines** across `regions.rs`, `transporter/`, `wire.rs`, `wire_helpers.rs`, `dispatch.rs`, `runtime.rs` + 788 lines of tests. `python/cell/RingTransporter.py` is the spec for the state graph and timings.
- **Why IM and not CW**: no live-client end-to-end run is recorded for the ring flow. The FSM and wire encoders are covered by unit tests only.
- **Path forward**: Live-client smoke; disconnect-recovery path through the transport FSM.

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Ring region loading | IM | -- | ring_transport/regions.rs | `ring_transport_regions` → `RingRegion` (302 lines) |
| Destination list to client | IM | -- | ring_transport/wire.rs | `RegionInfo` + `onRingTransporterList` payload encoding |
| Transport state machine | IM | -- | ring_transport/transporter/mod.rs | 521 lines — the multi-second FSM; manager at transporter/manager.rs |
| Kismet sequence playback | IM | -- | ring_transport/wire_helpers.rs | `onSequence` at both origin and destination |
| Hide / show + movement lock | IM | -- | ring_transport/wire_helpers.rs | `onVisible`, `onStateFieldUpdate`, `BSF_MOVEMENT_LOCK` |
| Region-trigger entry | IM | -- | ring_transport/runtime.rs | `handle_interact`, `handle_region_trigger`, `handle_select_destination` |
| Cross-world ring travel | IM | -- | ring_transport/runtime.rs | Shares the `CrossWorldTeleport` content action path |

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

### Anti-Cheat Validation --- IM (was KM)

Four layers of server-authoritative movement validation landed in PRs #437 and #478; see §7 for the detail. The remaining gap is damage-side sanity checking.

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Position bounds check | IM | -- | entity/movement_validation/bounds.rs | AABB from the loaded navmesh with a generous fallback for navmesh-less spaces; catches NaN / infinity / Z-floor-clip |
| Ability target validation | IM | -- | cell/abilities/use_ability/ | Target exists + alive; faction gate added by #444 |
| Inventory ownership check | CW | -- | base/world_entry/methods/inventory | Live-DB regression guards |
| Speed hack detection | IM | -- | entity/movement_validation/mod.rs | **Corrected 2026-07-25.** Server-monotonic-clock `dt` (never client-supplied), `top_speed × SPEED_WARN_TOLERANCE`. **Warn-only by design** — over-tolerance moves are logged and counted but still accepted until SigNoz telemetry calibrates the threshold |
| Teleport detection | IM | -- | entity/movement_validation/mod.rs | **Corrected 2026-07-25.** Hard reject + snap-back on the dual distance-AND-implied-speed gate; navmesh containment is the fourth layer (space_manager/entities.rs:297-302) |
| Damage sanity check | KM | -- | -- | No max-damage cap |
| Action-at-distance exploit | IM | -- | cell/abilities/use_ability/handle.rs:239-253 | **Corrected 2026-07-25.** `useAbility` rejects targets beyond the ability's `max_range` (30.0 default). LOS is *not* checked on this path — see the LOS row in §9 |

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

### Admin / GM Tools --- IM (GM command surface is CW; admin panel still IM)

**Substantially corrected 2026-07-25.** The prior edition listed teleport, item grant, and ban/mute as `KM` with the note "Python console not ported". Teleport and item grant shipped in June via the client's **native `/` console** — the `SGWGmPlayer` class flip (PR #473, merged in #518 on 2026-06-17) makes a GM enter the world as entity class `0x03`, which unlocks the client's built-in GM command tail. There is no dot-command interception and no server-side `/`-chat parsing; the client dispatches these itself. Owner-confirmed working 2026-06-20. Ban/mute is genuinely still missing.

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Admin API (REST) | IM | -- | crates/admin-api/ | -- |
| Tauri admin panel | IM | -- | tools/ | Per-page features partial |
| Native GM console (SGWGmPlayer) | CW | -- | cell/cell_methods/gm/ | **Added 2026-07-25.** 5,104 lines across give / stats / missions / travel / spawn / query / world / feedback + tests. Class flip at mercury/world_data/phases.rs:46. PRs #473 / #516 / #518 / #521 / #524 |
| Access level system | CW | -- | cell/dispatch/gm_gate.rs | **Upgraded 2026-07-25.** `enforce_gm_gate` (gm_gate.rs:97) resolves `CellEntity::access_level` and refuses the whole gated method range; `entire_sgwgmplayer_tail_is_gated` (gm_gate.rs:192) pins the classification. Plumbed by #475 / #516 |
| Python console | KM | -- | -- | C++ console not ported (intentional security) |
| Console commands | IM | -- | crates/commands/ | Command framework (registry / parser / permissions, 672 lines) |
| Dev/authoring `.`-console | IM | -- | cell/console/ | **Added 2026-07-25.** 4,370 lines for authoring commands that have no native slash equivalent (PR #523/#524) |
| Player info lookup | IM | -- | admin-api/routes/players.rs | Also `gmShowPlayer` / `gmUsers` / `testLOS` (cell_methods/gm/query.rs) |
| Ban/mute system | KM | -- | -- | No `GM_BAN` / `GM_MUTE` index and no handler anywhere in `crates/` |
| Teleport command | CW | -- | cell/cell_methods/gm/travel.rs | **Corrected 2026-07-25.** `gmGotoXYZ` / `gmGoto` / `gmSummon` / `gmGotoLocation` / `gmDHD` dispatched at gm/mod.rs:219-223; 457 lines + 450 lines of tests; non-finite coords guarded |
| Item grant | CW | -- | cell/cell_methods/gm/give.rs | **Corrected 2026-07-25.** `gmGiveItem` at gm/mod.rs:193, alongside give-xp / give-cash / remove-item / give-expertise / give-ASP; 504 lines + 494 lines of tests; base-side confirmation (trust-but-verify) |
| Action logging | IM | -- | tracing + OTLP | Via observability pipeline; per-command chat feedback on `CHAN_FEEDBACK` |
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
| 1 | Authentication | 12 | 6 | 0 | 4 | 2 | 0 |
| 2 | Mercury Protocol | 15 | 10 | 0 | 3 | 2 | 0 |
| 3 | Game Data Pipeline | 7 | 6 | 0 | 0 | 1 | 0 |
| 4 | Database Persistence | 8 | 7 | 0 | 0 | 1 | 0 |
| 5 | Character Creation | 11 | 0 | 8 | 1 | 2 | 0 |
| 6 | World Entry | 9 | 7 | 0 | 2 | 0 | 0 |
| 7 | Movement | 9 | 1 | 0 | 8 | 0 | 0 |
| 8 | Entity Lifecycle (AoI) | 9 | 6 | 0 | 2 | 1 | 0 |
| 9 | Combat & Abilities | 23 | 5 | 0 | 16 | 2 | 0 |
| 10 | Effects & Buffs | 13 | 4 | 0 | 7 | 2 | 0 |
| 11 | Stats | 8 | 5 | 0 | 0 | 2 | 1 |
| 12 | Inventory | 13 | 9 | 2 | 0 | 2 | 0 |
| 13 | Missions | 12 | 8 | 0 | 2 | 2 | 0 |
| 14 | Loot | 9 | 2 | 2 | 1 | 4 | 0 |
| 15 | Vendors | 8 | 2 | 5 | 1 | 0 | 0 |
| 16 | NPC AI | 22 | 6 | 0 | 11 | 5 | 0 |
| 17 | Spawn System | 23 | 7 | 0 | 11 | 4 | 1 |
| 18 | XP & Leveling | 11 | 9 | 0 | 0 | 2 | 0 |
| 19 | Crafting | 9 | 0 | 0 | 2 | 7 | 0 |
| 20 | Stargate Travel | 10 | 2 | 0 | 5 | 3 | 0 |
| 21 | Chat | 10 | 0 | 1 | 2 | 7 | 0 |
| 22 | Trading | 8 | 0 | 0 | 8 | 0 | 0 |
| 23 | Organizations | 15 | 0 | 0 | 0 | 15 | 0 |
| 24 | Mail | 13 | 0 | 0 | 8 | 4 | 1 |
| 25 | Black Market | 10 | 0 | 0 | 0 | 9 | 1 |
| 26 | Contact Lists | 10 | 10 | 0 | 0 | 0 | 0 |
| 27 | Dueling | 6 | 0 | 0 | 0 | 6 | 0 |
| 28 | Pets | 7 | 0 | 0 | 0 | 7 | 0 |
| 29 | Minigames | 9 | 0 | 0 | 6 | 3 | 0 |
| 30 | Groups | 7 | 0 | 0 | 0 | 7 | 0 |
| 31 | Content Engine | 10 | 6 | 0 | 0 | 4 | 0 |
| 32 | Mercury Bundle | 5 | 5 | 0 | 0 | 0 | 0 |
| 33 | Observability | 8 | 8 | 0 | 0 | 0 | 0 |
| 34 | Wireclient + Chaos | 7 | 3 | 0 | 3 | 1 | 0 |
| 35 | Discord Notifications | 6 | 6 | 0 | 0 | 0 | 0 |
| 36 | Tauri Admin App + Tools | 9 | 1 | 0 | 7 | 1 | 0 |
| 37 | Ring Transport | 7 | 0 | 0 | 7 | 0 | 0 |
| -- | Session Mgmt | 6 | 1 | 0 | 2 | 3 | 0 |
| -- | Rate Limiting | 5 | 1 | 0 | 0 | 4 | 0 |
| -- | Anti-Cheat | 7 | 1 | 0 | 5 | 1 | 0 |
| -- | Economy | 7 | 5 | 0 | 0 | 2 | 0 |
| -- | World State | 6 | 2 | 0 | 1 | 3 | 0 |
| -- | Scheduler | 4 | 0 | 0 | 1 | 3 | 0 |
| -- | Admin/GM | 13 | 4 | 0 | 5 | 4 | 0 |
| -- | Metrics / Telemetry | 7 | 4 | 0 | 3 | 0 | 0 |
| | **TOTALS** | **443** | **159** | **18** | **134** | **128** | **4** |

### Summary Percentages

Recomputed 2026-07-25 directly from the rows above; the columns sum to the totals line and the totals line sums to 443.

| Status | Count | Percentage |
|--------|-------|-----------|
| Confirmed Working (CW) | 159 | 35.9% |
| Needs Test (NT) | 18 | 4.1% |
| Implemented (IM) | 134 | 30.2% |
| Known/Missing (KM) | 128 | 28.9% |
| Needed/Unknown (NU) | 4 | 0.9% |

**Code exists (CW + NT + IM)**: 311 features (70.2%)
**Missing (KM + NU)**: 132 features (29.8%)

**Tested end-to-end (CW)**: 159 features (35.9%).

Movement relative to the 2026-05-27 **rows** (428 / CW 151 / NT 18 / IM 91 / KM 164 / NU 4 — *not* the 437 / 139 / 184 / 5 that edition's totals line printed):

| | CW | NT | IM | KM | NU | Total |
|---|---:|---:|---:|---:|---:|---:|
| 2026-05-27 rows | 151 | 18 | 91 | 164 | 4 | 428 |
| 2026-07-25 | 159 | 18 | 134 | 128 | 4 | 443 |
| **Delta** | **+8** | **0** | **+43** | **−36** | **0** | **+15** |

The 17 systems whose counts moved:

- **Gained ground** — Trading `8 KM → 8 IM` (#438); NPC AI `5 IM/11 KM → 11 IM/5 KM` (#428 movement states, #429 cover); Movement `4 IM/4 KM → 8 IM` (#437/#478); Contact Lists `8 KM → 10 CW` (#572–#583); Admin/GM `5 IM/6 KM → 4 CW/5 IM/4 KM` (#473→#518, #521, #523); Anti-Cheat `2 IM/4 KM → 5 IM/1 KM`; Minigames `4 IM/5 KM → 6 IM/3 KM`; Crafting `9 KM → 2 IM/7 KM` (#427); Chat, Combat, Character Creation, Authentication each `+1 IM`; Mercury `+3 IM` (v2 crypto, v2 rotation, cumulative ACKs).
- **Lost ground** — Wireclient `7 CW → 3 CW/3 IM/1 KM` (the crate has no UDP socket); AoI `7 CW → 6 CW` (open entity-introduction defect); Loot `2 IM → 1 IM/+1 KM` (no eligibility list exists in Rust); Mercury `11 CW → 10 CW` (pcap replay does not exist).
- **New rows (+15)** — Ring Transport (7, previously untracked), contact-list Death and GateTravel events (2), Mercury v2 crypto and rotation (2), the native GM console and the `.`-console (2), auth TLS (1), chat AFK/DND (1).

---

## What changed since the previous (deprecated-codebase) gap analysis

| Metric | Audit (Python+C++) | 2026-05-27 rows | This pass (2026-07-25) | Why the change |
|---|---:|---:|---:|---|
| Total features tracked | 369 | 428 | 443 | New systems (content engine, observability, wireclient, discord, Mercury bundle, ring transport), some sub-feature splits |
| Confirmed Working | 31 (8.4%) | 151 (35.3%) | 159 (35.9%) | Contact lists and the GM command surface moved in; four wireclient rows and two others moved out |
| Code exists (CW+NT+IM) | 175 (47.4%) | 260 (60.7%) | 311 (70.2%) | Trading, NPC AI movement states, cover, movement validation, minigames, crafting state, Mercury v2 |
| Missing | 194 (52.6%) | 168 (39.3%) | 132 (29.8%) | Two months of feature work; the port backlog is now mostly *social* (org, duel, pets, groups) rather than *core* |

The 2026-05-27 column is the sum of that edition's own rows, not the headline it printed. Its totals line said 437 / CW 139 / KM 184 / NU 5, which the table never supported.

The shape of "done" as of this pass: Mercury, observability, and the content engine are firmly done; **NPC navigation is no longer the blocker it was** (patrol / wander / investigate / follow / leash and the cover system all run against a live Detour navmesh); trading is ported; crafting is half-ported (state yes, verbs no); and the remaining long-tail social systems (organizations, dueling, pets, groups) are genuinely still "stub handlers exist but nothing works". Contact lists left that group in June; black market is queued behind an unmerged branch.

---

## Critical Path for Playability

Re-ranked 2026-07-25. Two items from the previous list (NPC navigation states, the trading port) are **done** and have been struck.

1. **Effect-script content coverage** — the framework is CW (PR #420) but the long tail of the 3,217 effect rows still needs scripts. `cell/effects/scripts.rs` is up to 1,648 lines from 869
2. **AoI invisible-entity defect** — a witness can miss an entity introduction entirely (Castle Cellblock GuardBody corpse). #582 instrumentation is in place; this needs a repro, not more code
3. **Mission XP** — `mission.reward_xp` is 0 in all seed rows; chain-side authoring + `Action::GrantXP` wiring needed (the executor still has no `GrantXP` arm)
4. **Crafting Phase 2** — state and persistence landed (#427); every player-facing verb still logs `UNIMPLEMENTED`
5. **Multi-zone end-to-end** — only Castle Cellblock is routinely smoked; the other 23 spaces need verification
6. **Live-client verification of the June/July landings** — trading, ring transport, movement validation, cover, and the minigame server all shipped with unit/live-DB coverage but no client smoke. This is what stands between ~134 IM features and CW

Quality-of-life items (organizations, mail polish, black market, dueling, pets, remaining minigame ports, groups) are still gated on the above but each can be picked up independently. GM tooling and contact lists have left this list.

---

## Cross-Reference Tables

### Documentation Exists but Rust Doesn't (port pending)

Corrected 2026-07-25 — trading and contact lists have left this table.

| System | Gameplay Doc | Wire Format Doc | Rust Code Status |
|--------|-------------|----------------|-------------------|
| Crafting | crafting-system.md | crafting-wire-formats.md | State + persistence ported (#427); all crafting verbs still stubs |
| Organizations | organization-system.md | organization-wire-formats.md | 200 lines stubs — unchanged |
| Black Market | black-market.md | black-market-wire-formats.md | 94 lines stubs on `main`; full Phase 1 waiting on `feat/571-black-market-phase1` |
| Dueling | duel-system.md | duel-wire-formats.md | Not ported |
| Pets | pet-system.md | pet-wire-formats.md | Not ported |
| Groups | group-system.md | group-wire-formats.md | Not ported (`game/src/social/groups.rs` is an unwired 97-line sketch) |

### Rust Code Exists but Doc Lags

These have substantial Rust implementations the per-system docs haven't fully caught up on. P3-equivalent doc-refresh pending.

| System | Code Location | Doc Status |
|--------|--------------|-----------|
| Content Engine | crates/services/src/cell/content/ + crates/content-engine/ | docs/content/content-engine.md is the canonical reference but is currently labelled audience: "engineers" — could use a "what's done vs. planned" callout |
| Mercury Bundle | crates/mercury/src/channel_bundle.rs | docs/architecture/mercury-bundle.md is the ADR |
| Observability | crates/server/, crates/mercury/instrumentation.rs | docs/architecture/observability.md + operations/signoz-*.md |
| Wireclient | crates/wireclient/ | docs/architecture/wireclient.md — **verify this doc's Tier 3 claims**; the crate has no UDP socket (see §34) |
| Discord Notifications | crates/discord/ | docs/architecture/discord-notifications.md |
| Trading | crates/services/src/cell/cell_methods/player/trade/ + base/world_entry/methods/trade/ | **Added 2026-07-25.** docs/gameplay/trade-system.md still describes Python `Trade.py` as the implementation |
| Ring Transport | crates/services/src/cell/ring_transport/ | **Added 2026-07-25.** No dedicated doc — 2,791 lines with no reference page |
| Cover system | crates/services/src/cell/cover/ | **Added 2026-07-25.** docs/game-systems.md still says "CoverSet entity is a stub" — corrected in that file on 2026-07-25 |
| NPC AI movement states | crates/services/src/cell/service/npc_ai/ | **Added 2026-07-25.** docs/gameplay/npc-ai.md predates PR #428 |
| GM command surface | crates/services/src/cell/cell_methods/gm/ + cell/console/ | **Added 2026-07-25.** 5,104 + 4,370 lines; no consolidated GM command reference |
| Minigame server | crates/services/src/minigame/ | **Added 2026-07-25.** docs/gameplay/minigame-system.md still describes an external SmartFox process |
| Movement validation | crates/entity/src/movement_validation/ | **Added 2026-07-25.** Four-layer anti-cheat with no ADR |

### Server-Only Blind Spots (Ranked by Gameplay Impact)

Re-ranked 2026-07-25. The old #1 (NPC Navigation) and #3 (Trading) are closed; #5 (speed-hack detection) is implemented but deliberately warn-only.

| Rank | System | Impact | Status | Notes |
|------|--------|--------|--------|-------|
| 1 | Crafting verbs | HIGH — entire skill tree unplayable | KM | Phase 1 state landed (#427); craft / research / RE / alloy / ASP-spend all log `UNIMPLEMENTED` |
| 2 | AoI entity-introduction drop | HIGH — entities silently invisible | IM | Known-open; address-gate hypothesis disproved 2026-06-20; #582 seams await a repro |
| 3 | Organizations / guilds | MEDIUM — no persistent social layer | KM | 200 lines of stubs, no schema |
| 4 | Rate Limiting | MEDIUM — exploitable | KM | No throttle on chat / trade-request / login. Trading shipped without a request cooldown, so this got *worse* |
| 5 | Speed-hack enforcement | MEDIUM — detection lands, action doesn't | IM | Layer is live but warn-only by design pending tolerance calibration from SigNoz |
| 6 | Damage sanity checking | MEDIUM — no max-damage cap | KM | The one anti-cheat layer with no implementation at all |
| 7 | Mission XP integration | LOW — XP works from kills only | KM | Chain authoring + a `GrantXP` executor arm needed |
| 8 | Multi-zone verification | LOW — 1 zone routinely smoked | NT | 23 spaces unchecked |
| 9 | `sequences_nvp` unread | LOW — cinematic sound-bank / params never reach the client | KM | `db/resources/Events/Seed/sequences_nvp.sql` seeds **2,042 rows** (SoundBankName and friends). No Rust code reads the table, and all six `onSequence` emit sites hardcode a NameValuePairs count of 0: abilities/damage_apply/mod.rs:351, abilities/use_ability/handle.rs:540 and :569, cell/console/net.rs:104, content/executor/mod.rs:122, ring_transport/wire_helpers.rs:42 |

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
