# Cimmeria Server Rewrite — Rust + Tauri 2.x Architecture

## Context

### What We Have Today

Cimmeria is a server emulator for the Stargate Worlds MMO. It's built in **C++ (~22,000 lines)** with **Python 3.4 scripting (~26,000 lines)** and **PostgreSQL 9.2** for persistence. The server runs as three separate executables (AuthenticationServer, BaseApp, CellApp) that communicate over TCP, while game clients connect via a custom reliable UDP protocol called Mercury.

Every dependency is from 2013 and has known security vulnerabilities (especially OpenSSL 0.9.8). The Python scripting layer adds complexity without type safety, and the C++/Python bridge is fragile. There's no built-in admin UI — operators must use a separate Qt desktop tool (ServerEd) or a raw Python console on port 8989.

### What We're Building

A **complete rewrite** of the server in **Rust** using **Tauri 2.x** as the application shell. Instead of three separate C++ executables and a Python scripting layer, we get:

- **One binary** that runs all server services (auth, base, cell) with a built-in web-based admin UI
- **Native Rust game logic** replacing all 26,000 lines of Python — type-safe, fast, no GIL
- **Live admin dashboard** in the browser — monitor players, edit content, view 3D worlds, all against the running server
- **Remote administration** — the admin UI can be hosted as a static website (free Azure/Cloudflare hosting) that connects back to the server's API
- **Data-driven content engine** in Rust replacing hand-written mission/interaction scripts with database-configured trigger/condition/action chains

The game client (`sgw.exe`) is a fixed binary we cannot modify, so the server must speak the exact same Mercury protocol byte-for-byte. This is non-negotiable and drives many design decisions.

### Why Tauri?

**Plain English:** Tauri lets us build a desktop application where the "backend" is Rust and the "frontend" is a web page running in the OS's built-in browser engine. We're using this pattern cleverly — our Rust backend IS the game server, and the web frontend IS the admin panel. One app, two jobs.

**Technical:** Tauri 2.x uses tokio internally for its async runtime. Since our game server also needs tokio for async networking, they share the same runtime — no overhead from running a separate process. The Tauri IPC bridge (`tauri::command`) gives the admin UI direct function-call access to server internals, and WebSocket channels provide real-time entity/log streaming. The webview renders a SolidJS + Three.js frontend for the admin dashboard.

---

## Architecture Overview

### High-Level System Diagram

```
┌─────────────────────────────────────────────────────────────────────┐
│                        Tauri Application                            │
│                                                                     │
│  ┌──────────────────────────────────────────────────────────────┐   │
│  │                    Rust Backend (tokio)                       │   │
│  │                                                              │   │
│  │  ┌─────────┐  ┌─────────┐  ┌──────────┐  ┌──────────────┐  │   │
│  │  │  Auth   │  │  Base   │  │   Cell   │  │  Admin API   │  │   │
│  │  │ Service │  │ Service │  │  Service │  │  (axum)      │  │   │
│  │  │ :13001  │  │ :32832  │  │  :50000  │  │  :8443       │  │   │
│  │  └────┬────┘  └────┬────┘  └────┬─────┘  └──────┬───────┘  │   │
│  │       │            │            │               │           │   │
│  │  ┌────┴────────────┴────────────┴───────────────┴────────┐  │   │
│  │  │              Shared Server State (Arc<RwLock>)         │  │   │
│  │  │  Entities, Spaces, Sessions, Content Engine, DB Pool  │  │   │
│  │  └───────────────────────────────────────────────────────┘  │   │
│  └──────────────────────────────────────────────────────────────┘   │
│                              │ IPC                                   │
│  ┌──────────────────────────────────────────────────────────────┐   │
│  │                  Webview Frontend (SolidJS)                   │   │
│  │                                                              │   │
│  │  ┌───────────┐ ┌──────────┐ ┌────────┐ ┌────────────────┐  │   │
│  │  │ Dashboard │ │ Content  │ │ Chain  │ │  Space Viewer  │  │   │
│  │  │  & Logs   │ │  Editor  │ │ Editor │ │  (Three.js)    │  │   │
│  │  └───────────┘ └──────────┘ └────────┘ └────────────────┘  │   │
│  └──────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────┘
         ▲                    ▲                        ▲
         │ Mercury UDP        │ TCP (inter-service)    │ HTTPS + WSS
         │                    │                        │  (remote admin)
    Game Clients         (internal only)          Remote Browser
    (sgw.exe)
```

**Plain English:** The whole thing is one application. When you launch it, it starts the game server (which game clients connect to) AND opens a window with the admin dashboard. The admin dashboard talks directly to the server internals — no network hop, no API overhead for local use. For remote administration, the same dashboard UI can be hosted on a free static website and connect to the server over the internet.

### Remote Administration

```
┌─────────────────────┐         HTTPS / WSS          ┌──────────────────┐
│   Azure Static      │◄────────────────────────────► │  Cimmeria Server │
│   Web Apps (free)    │    JWT-authenticated API      │  :8443 admin     │
│                      │                               │                  │
│  Same SolidJS app    │    Static files:              │  axum serves:    │
│  as local webview    │    index.html, app.js,        │  REST endpoints  │
│                      │    three.js, assets            │  WebSocket feed  │
└─────────────────────┘                               └──────────────────┘
```

**Plain English:** The admin UI is just a website. When running locally inside Tauri, it talks to the server through a fast internal channel. When hosted on Azure (free tier — no cost), it talks to the server over the internet with login credentials. Same UI, same features, works from anywhere.

---

## Rust Workspace Layout

### Crate Structure

```
cimmeria/
├── Cargo.toml                          # Workspace root
├── crates/
│   ├── common/                         # Shared types, math, config
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── types.rs                # EntityId, SpaceId, Vector3, etc.
│   │   │   ├── config.rs               # XML config loading (quick-xml)
│   │   │   ├── math.rs                 # Game math (vectors, quaternions)
│   │   │   └── error.rs                # Error types
│   │   └── Cargo.toml
│   │
│   ├── mercury/                        # Network protocol (byte-identical to C++)
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── packet.rs               # Packet framing, flags, MTU
│   │   │   ├── channel.rs              # Reliable UDP (ARQ, windows, retransmit)
│   │   │   ├── nub.rs                  # UDP socket manager, channel registry
│   │   │   ├── bundle.rs               # Message aggregation + fragmentation
│   │   │   ├── unpacker.rs             # Fragment reassembly
│   │   │   ├── encryption.rs           # AES-256-CBC + HMAC-MD5
│   │   │   ├── unified.rs              # TCP inter-service framing
│   │   │   ├── codec.rs                # tokio_util codec adapters
│   │   │   └── messages.rs             # Message ID registry
│   │   └── Cargo.toml
│   │
│   ├── defs/                           # Entity definition code generator
│   │   ├── build.rs                    # Reads entities/*.def XML at compile time
│   │   ├── src/
│   │   │   ├── lib.rs                  # Generated entity types (re-exported)
│   │   │   └── registry.rs             # Runtime type lookup by name/ID
│   │   ├── generator/
│   │   │   ├── mod.rs                  # Code generation logic
│   │   │   ├── parser.rs               # XML .def file parser
│   │   │   └── emitter.rs              # Rust source code emitter
│   │   └── Cargo.toml
│   │
│   ├── entity/                         # Entity system runtime
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── base_entity.rs          # Base-side entity (persistence, mailbox)
│   │   │   ├── cell_entity.rs          # Cell-side entity (spatial, movement, AoI)
│   │   │   ├── mailbox.rs              # RPC abstraction (base/cell/client)
│   │   │   ├── manager.rs              # Entity pool, ID allocation, free list
│   │   │   ├── properties.rs           # Property sync, dirty flags, distribution
│   │   │   ├── space.rs                # Space management, entity registry
│   │   │   ├── world_grid.rs           # Spatial partitioning, AoI queries
│   │   │   ├── movement.rs             # Movement controllers (player, waypoint, debug)
│   │   │   └── navigation.rs           # Recast/Detour pathfinding wrapper
│   │   └── Cargo.toml
│   │
│   ├── game/                           # Game logic (replaces ALL Python)
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── player.rs               # SGWPlayer logic (login, persist, level, XP)
│   │   │   ├── being.rs                # SGWBeing shared logic (combat, stats, buffs)
│   │   │   ├── mob.rs                  # SGWMob logic (AI, aggro, patrol, spawning)
│   │   │   ├── npc.rs                  # SGWNpc logic (dialog, vendor, trainer)
│   │   │   ├── combat/
│   │   │   │   ├── mod.rs
│   │   │   │   ├── stats.rs            # Stat calculation, modifiers
│   │   │   │   ├── damage.rs           # Damage pipeline, mitigation
│   │   │   │   ├── abilities.rs        # Ability execution, cooldowns, resources
│   │   │   │   └── effects.rs          # Buff/debuff application, ticking
│   │   │   ├── inventory/
│   │   │   │   ├── mod.rs
│   │   │   │   ├── containers.rs       # Inventory containers, slot management
│   │   │   │   ├── items.rs            # Item creation, stacking, equipping
│   │   │   │   └── loot.rs             # Loot tables, rolls, distribution
│   │   │   ├── missions/
│   │   │   │   ├── mod.rs
│   │   │   │   ├── manager.rs          # Mission lifecycle, tracking
│   │   │   │   ├── objectives.rs       # Kill/collect/interact/escort objectives
│   │   │   │   └── rewards.rs          # XP, items, currency, reputation
│   │   │   ├── social/
│   │   │   │   ├── mod.rs
│   │   │   │   ├── chat.rs             # Chat channels, commands
│   │   │   │   ├── groups.rs           # Group formation, loot rules
│   │   │   │   ├── guilds.rs           # Guild CRUD, permissions
│   │   │   │   └── mail.rs             # In-game mail system
│   │   │   ├── interactions/
│   │   │   │   ├── mod.rs
│   │   │   │   ├── vendor.rs           # Buy/sell/browse
│   │   │   │   ├── trainer.rs          # Ability training
│   │   │   │   ├── lootable.rs         # Corpse looting
│   │   │   │   └── stargate.rs         # World travel
│   │   │   ├── world/
│   │   │   │   ├── mod.rs
│   │   │   │   ├── spawning.rs         # Respawn timers, spawn sets
│   │   │   │   └── regions.rs          # Region triggers, PvP zones
│   │   │   └── commands/
│   │   │       ├── mod.rs
│   │   │       ├── player_cmds.rs      # /stuck, /wave, /sit, etc.
│   │   │       └── gm_cmds.rs          # /spawn, /teleport, /kill, /give, etc.
│   │   └── Cargo.toml
│   │
│   ├── content-engine/                 # Data-driven content system
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── chain.rs               # Trigger → Condition → Action chains
│   │   │   ├── triggers.rs            # 11 event trigger types
│   │   │   ├── conditions.rs          # 7 condition evaluators
│   │   │   ├── actions.rs             # 21 action executors
│   │   │   ├── context.rs             # Execution context (entity, space, etc.)
│   │   │   └── loader.rs              # Load chains from database
│   │   └── Cargo.toml
│   │
│   ├── services/                       # Service orchestration
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── auth.rs                 # Authentication service
│   │   │   ├── base.rs                 # BaseApp service
│   │   │   ├── cell.rs                 # CellApp service
│   │   │   ├── database.rs             # Connection pool (sqlx)
│   │   │   └── orchestrator.rs         # Start/stop/health of all services
│   │   └── Cargo.toml
│   │
│   ├── admin-api/                      # REST + WebSocket admin interface
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── routes/
│   │   │   │   ├── mod.rs
│   │   │   │   ├── entities.rs         # CRUD entities, inspect state
│   │   │   │   ├── spaces.rs           # Space listing, entity positions
│   │   │   │   ├── content.rs          # Content engine chain CRUD
│   │   │   │   ├── players.rs          # Online players, kick, ban
│   │   │   │   ├── config.rs           # Runtime config read/write
│   │   │   │   └── auth.rs             # JWT login, token refresh
│   │   │   ├── ws/
│   │   │   │   ├── mod.rs
│   │   │   │   ├── entity_stream.rs    # Real-time entity position feed
│   │   │   │   ├── log_stream.rs       # Live log tailing
│   │   │   │   └── event_stream.rs     # Server events (login, combat, etc.)
│   │   │   └── middleware.rs           # Auth, CORS, rate limiting
│   │   └── Cargo.toml
│   │
│   └── commands/                       # GM/admin command registry
│       ├── src/
│       │   ├── lib.rs
│       │   ├── registry.rs             # Command registration and dispatch
│       │   ├── parser.rs               # Command argument parsing
│       │   └── permissions.rs          # Access level checks
│       └── Cargo.toml
│
├── src-tauri/                          # Tauri application shell
│   ├── src/
│   │   ├── main.rs                     # Tauri entry point, server bootstrap
│   │   ├── ipc.rs                      # Tauri IPC command handlers
│   │   └── state.rs                    # Shared state between Tauri + server
│   ├── tauri.conf.json
│   ├── Cargo.toml
│   └── icons/
│
├── frontend/                           # SolidJS admin UI
│   ├── src/
│   │   ├── App.tsx
│   │   ├── pages/
│   │   │   ├── Dashboard.tsx           # Server status, player count, uptime
│   │   │   ├── Players.tsx             # Online player list, inspect, kick
│   │   │   ├── ContentEditor.tsx       # Database content editing (tables)
│   │   │   ├── ChainEditor.tsx         # Visual trigger/condition/action editor
│   │   │   ├── SpaceViewer.tsx         # 3D world viewer (Three.js)
│   │   │   ├── Logs.tsx                # Real-time log viewer with filters
│   │   │   └── Config.tsx              # Server configuration editor
│   │   ├── components/
│   │   │   ├── NodeGraph.tsx           # Visual node editor (chains)
│   │   │   ├── EntityInspector.tsx     # Entity property viewer
│   │   │   ├── WorldRenderer.tsx       # Three.js canvas wrapper
│   │   │   └── DataTable.tsx           # Sortable/filterable data grid
│   │   ├── three/
│   │   │   ├── SceneManager.ts         # Three.js scene lifecycle
│   │   │   ├── NavMeshRenderer.ts      # Render navmesh geometry
│   │   │   ├── EntityMarkers.ts        # Entity position markers
│   │   │   ├── TerrainRenderer.ts      # Heightmap terrain (Phase 2)
│   │   │   └── AssetLoader.ts          # glTF model loading (Phase 3+)
│   │   └── lib/
│   │       ├── api.ts                  # REST client (works local + remote)
│   │       ├── ws.ts                   # WebSocket client
│   │       └── tauri.ts                # Tauri IPC bridge (local only)
│   ├── index.html
│   ├── package.json
│   └── vite.config.ts
│
├── entities/                           # Entity definitions (UNCHANGED)
│   ├── entities.xml                    # Entity type registry
│   ├── defs/
│   │   ├── SGWPlayer.def               # Player entity definition
│   │   ├── SGWMob.def                  # NPC mob definition
│   │   ├── SGWNpc.def                  # NPC definition
│   │   └── interfaces/                 # Shared property/method bundles
│   └── cell_spaces.xml                 # Space configuration
│
├── config/                             # Service configuration (UNCHANGED format)
│   ├── AuthenticationService.config
│   ├── BaseService.config
│   └── CellService.config
│
├── db/                                 # Database schema (UNCHANGED)
│   ├── database.sql
│   ├── resources/                      # Game data schema
│   └── sgw/                            # Player state schema
│
├── data/                               # Game data files (UNCHANGED)
│   ├── cache/*.pak
│   ├── scripts/
│   └── spaces/
│
├── tools/
│   └── asset-pipeline/                 # UE3 asset extraction (future)
│       ├── extract.rs                  # UModel wrapper
│       └── convert.rs                  # → glTF conversion
│
└── docs/                               # Documentation (UNCHANGED)
```

### Crate Dependency Graph

```
                    ┌──────────┐
                    │  common  │  (no deps on other crates)
                    └────┬─────┘
                         │
              ┌──────────┼───────────┐
              ▼          ▼           ▼
         ┌────────┐ ┌────────┐ ┌──────────┐
         │mercury │ │  defs  │ │ commands │
         └───┬────┘ └───┬────┘ └────┬─────┘
             │          │           │
             ▼          ▼           │
         ┌───────────────────┐      │
         │      entity       │◄─────┘
         └────────┬──────────┘
                  │
         ┌────────┼──────────────┐
         ▼        ▼              ▼
    ┌────────┐ ┌────────────────┐│
    │  game  │ │content-engine  ││
    └───┬────┘ └───────┬────────┘│
        │              │         │
        ▼              ▼         ▼
    ┌──────────────────────────────┐
    │          services            │
    └──────────────┬───────────────┘
                   │
          ┌────────┼────────┐
          ▼                 ▼
    ┌───────────┐    ┌───────────┐
    │ admin-api │    │ src-tauri │
    └───────────┘    └───────────┘
```

**Plain English:** Each box is a separate Rust library that compiles independently. Arrows point from "depends on" to "provides." The `common` crate at the top has the basic types everything needs. `mercury` handles networking. `defs` generates entity type code from XML. `entity` manages game objects. `game` contains all the gameplay rules. `services` ties it all together. `admin-api` and `src-tauri` are the two ways to interact with the running server.

---

## Key Technical Designs

### 1. Entity System — Compile-Time Code Generation

**The Problem:** The current server reads `.def` XML files at startup, builds Python class definitions at runtime, and marshals data between C++ and Python on every property access. This is slow and error-prone.

**The Solution:** A `build.rs` script reads the same `.def` XML files at **compile time** and generates native Rust structs. The generated code has zero runtime overhead — property access is a direct struct field read, wire serialization is derived automatically, and type errors are caught by the compiler.

**How it works:**

1. The `defs` crate's `build.rs` runs during `cargo build`
2. It reads `entities/entities.xml` to find all entity types
3. For each `.def` file, it parses properties, methods, and interfaces
4. It emits Rust source code into `OUT_DIR` that gets compiled into the binary

**Example — what goes in, what comes out:**

Input (`entities/defs/SGWPlayer.def`):
```xml
<root>
  <Properties>
    <playerName> <Type>STRING</Type> <Flags>ALL_CLIENTS</Flags> </playerName>
    <health>     <Type>INT32</Type>  <Flags>OWN_CLIENT</Flags>  </health>
    <position>   <Type>VECTOR3</Type><Flags>CELL_PUBLIC</Flags> </position>
  </Properties>
  <CellMethods>
    <useAbility>
      <Exposed/>
      <Arg>INT32</Arg>  <!-- ability_id -->
      <Arg>INT32</Arg>  <!-- target_id -->
    </useAbility>
  </CellMethods>
  <ClientMethods>
    <onHealthChanged>
      <Arg>INT32</Arg>  <!-- new_health -->
    </onHealthChanged>
  </ClientMethods>
</root>
```

Generated output (simplified):
```rust
pub struct SGWPlayerProperties {
    pub player_name: String,       // ALL_CLIENTS — sent to all nearby players
    pub health: i32,               // OWN_CLIENT — sent only to the owning player
    pub position: Vector3,         // CELL_PUBLIC — cell-side, visible to witnesses
}

impl SGWPlayerProperties {
    pub fn write_client_update(&self, stream: &mut BytesMut, flags: DistributionFlags) {
        // Only writes properties matching the given distribution flags
        // Byte layout matches what sgw.exe expects
    }
}

// Type-safe method dispatch — compiler catches missing handlers
pub trait SGWPlayerCellMethods {
    fn use_ability(&mut self, ability_id: i32, target_id: i32);
}

pub trait SGWPlayerClientMethods {
    fn on_health_changed(&self, new_health: i32) -> Bundle;
}
```

**Plain English:** Instead of reading XML files every time the server starts and building types at runtime (slow, no error checking), we read those same XML files once during compilation and generate optimized Rust code. If a developer adds a property to the XML but forgets to handle it in the game logic, the compiler catches it immediately — no more runtime crashes from typos.

### 2. Mercury Protocol — Byte-Identical Reimplementation

**The Constraint:** The game client (`sgw.exe`) is a fixed binary. It speaks a specific binary protocol. Every byte must be exactly right or the client disconnects. We cannot change the protocol — we must match it perfectly.

**Critical Protocol Constants** (from C++ source analysis):

| Constant | Value | What It Means |
|---|---|---|
| `PACKET_MAX_SIZE` | 1472 bytes | Maximum UDP packet size (MTU-safe) |
| `HEADER_SIZE` | 4 bytes | Sequence number + flags |
| `MAX_BODY` | 1348 bytes | Payload after header, footer, encryption overhead |
| `RX_WINDOW_SIZE` | 64 packets | How many packets we track for out-of-order delivery |
| `TX_WINDOW_SIZE` | 45 packets | How many unacknowledged packets we allow in flight |
| `ACK_TIMEOUT` | 700 ms | How long before we resend an unacknowledged packet |
| `MAX_RETRIES` | 20 | How many times we retry before disconnecting |
| `KEEPALIVE_INTERVAL` | 1000 ms | How often we send "I'm still here" packets |
| `MAX_FRAGMENTS` | 64 | Maximum fragments for a single large message |
| `PROTOCOL_VERSION` | 391 | Version handshake value the client expects |

**Packet format on the wire:**

```
┌──────────────────────────────────────────────────────────────────────┐
│ Byte 0-3: Sequence Number (uint32 LE)                                │
│   Bits 0-25:  Sequence number (0 – 67,108,863)                      │
│   Bit 26:     RELIABLE flag                                          │
│   Bit 27:     FRAGMENTED (this packet is part of a larger message)   │
│   Bit 28:     HAS_REQUESTS (contains outgoing messages)              │
│   Bit 29:     HAS_PIGGYBACKS (contains piggybacked acks)             │
│   Bit 30:     HAS_ACKS (contains acknowledgment footer)              │
│   Bit 31:     CREATE_CHANNEL (first packet, establish connection)    │
├──────────────────────────────────────────────────────────────────────┤
│ Byte 4+: Message Body (0 – 1348 bytes after encryption)             │
│   Each message: [uint8 msg_id] [payload...]                          │
│   Multiple messages can be packed into one packet (bundling)         │
├──────────────────────────────────────────────────────────────────────┤
│ Footer (if HAS_ACKS): Acknowledgment bitmask                        │
│   [uint8 count] [uint32 first_seq] [uint32 ack_mask]                 │
└──────────────────────────────────────────────────────────────────────┘
```

**Encryption layer:**
- AES-256 in CBC mode (NOT GCM — the client expects CBC)
- HMAC-MD5 for integrity (legacy, but client requires it)
- PKCS#7 padding
- Rust crates: `aes` + `cbc` + `hmac` + `md5` (NOT `ring` — ring doesn't support CBC mode)

**Plain English:** The protocol is like a postal system. Each "packet" is an envelope that fits in a 1472-byte mailbox. Inside the envelope, multiple "messages" can be bundled together (like putting several letters in one envelope). If an envelope gets lost, the sender notices within 700ms and sends it again. The whole thing is encrypted so nobody can read the mail in transit. We must match this system exactly because the game client was built to expect these specific envelope sizes, retry timings, and encryption methods.

### 3. Content Engine — Database-Driven Game Logic

**What it replaces:** Much of the Python scripting layer consists of hand-written "when X happens, check Y, then do Z" patterns. The content engine replaces these with database-configured chains.

**Chain structure:**

```
Chain: "Mob Death Loot Drop"
├── Trigger: ON_ENTITY_DEATH (mob_entity_id)
├── Conditions:
│   ├── IF killer IS player
│   └── AND mob.level WITHIN 5 OF killer.level
└── Actions:
    ├── ROLL_LOOT_TABLE (mob.loot_table_id, luck: killer.luck_stat)
    ├── SPAWN_LOOT_BAG (at: mob.position, items: loot_result)
    └── GRANT_XP (to: killer, amount: mob.xp_value)
```

**Event triggers** (11 types): `OnEntityCreated`, `OnEntityDestroyed`, `OnEntityDeath`, `OnAbilityUsed`, `OnInteraction`, `OnRegionEnter`, `OnRegionExit`, `OnMissionStep`, `OnItemAcquired`, `OnTimer`, `OnCustomEvent`

**Condition evaluators** (7 types): `PropertyEquals`, `PropertyInRange`, `HasItem`, `HasAbility`, `InRegion`, `FactionCheck`, `CustomExpression`

**Action executors** (21 types): `GrantXP`, `GrantItem`, `RemoveItem`, `ApplyEffect`, `RemoveEffect`, `Teleport`, `SpawnEntity`, `DespawnEntity`, `StartDialog`, `AdvanceMission`, `CompleteMission`, `PlayAnimation`, `PlaySound`, `SendMessage`, `ModifyProperty`, `RollLootTable`, `SpawnLootBag`, `StartTimer`, `CancelTimer`, `TriggerChain`, `ExecuteCustom`

**Plain English:** Instead of writing Python code for every game event ("when a mob dies, give the player loot"), operators configure it in the admin UI: pick a trigger, set conditions, choose actions. No code changes, no server restarts. The content engine evaluates these chains in real-time. For complex custom logic that can't be expressed as chains, `ExecuteCustom` calls into native Rust handler functions.

### 4. Frontend Architecture — SolidJS + Three.js

**Why SolidJS over React:** SolidJS compiles away the framework — components become direct DOM updates with no virtual DOM overhead. For a dashboard showing hundreds of entity positions updating 10 times per second, this matters. The bundle size is also ~7KB vs React's ~45KB, which helps for static hosting.

**Admin UI Pages:**

| Page | What It Shows | Data Source |
|---|---|---|
| **Dashboard** | Server uptime, player count, entity count, tick rate, memory usage, service health | REST polling (5s) + WebSocket events |
| **Players** | Online player list with name, level, location, IP. Click to inspect/kick/ban/teleport | REST + WebSocket |
| **Content Editor** | Database table browser and editor. Edit items, abilities, missions, NPCs, loot tables | REST CRUD against PostgreSQL |
| **Chain Editor** | Visual node graph for content engine chains. Drag triggers → conditions → actions | REST CRUD + live preview |
| **Space Viewer** | 3D view of game world. Nav mesh rendered as wireframe. Entities as colored markers moving in real-time | WebSocket entity stream + Three.js |
| **Logs** | Real-time scrolling log viewer with level/category filters | WebSocket log stream |
| **Config** | Server configuration editor with validation | REST read/write |

**3D Space Viewer — Progressive Fidelity:**

```
Phase 1 (Day One):        Nav mesh wireframe + entity dots
                           ┌─────────────────────────┐
                           │    ╱╲  ╱╲  ╱╲           │
                           │   ╱  ╲╱  ╲╱  ╲   • ←mob│
                           │  ╱    ╲    ╲   ╲        │
                           │ ╱  ●   ╲    ╲   ╲       │
                           │╱  player ╲    ╲   ╲     │
                           └─────────────────────────┘

Phase 2 (Milestone 4):    Heightmap terrain under the nav mesh
                           ┌─────────────────────────┐
                           │ ░░▓▓██▓▓░░░░░░▓▓██████  │
                           │ ░░░▓▓██▓▓░░░▓▓████████  │
                           │ ░░░░░▓▓██▓▓▓▓██████████  │
                           │ ░░░░●░░▓▓████████  •    │
                           │ ░░░░░░░░░▓▓██████████   │
                           └─────────────────────────┘

Phase 3 (Milestone 5):    Extracted UE3 static geometry (glTF)
                           ┌─────────────────────────┐
                           │  🏛️  🌳  🏚️   🌳          │
                           │      🌳     🌳  •        │
                           │  ●                🌳     │
                           │     🏛️    🌳              │
                           │  🌳     🏛️     🌳         │
                           └─────────────────────────┘
```

**Plain English:**
- **Phase 1** — You see the walkable ground as a wireframe (the nav mesh the server uses for pathfinding) and colored dots for entities. Players are blue, mobs are red, NPCs are green. Dots move in real-time as entities move in the game. This works from day one because we already have nav mesh data.
- **Phase 2** — We add terrain underneath the wireframe using heightmap data from the game files, so you see actual hills and valleys.
- **Phase 3** — We extract 3D models from the original game files (buildings, trees, rocks) and render them in the viewer. Now it looks like the actual game world.

### 5. Asset Pipeline (Phase 3+)

**How we get game assets into the web viewer:**

```
UE3 .upk files ──► UModel (extract) ──► .psk/.psa + textures
                                              │
                                              ▼
                                    Blender/script (convert)
                                              │
                                              ▼
                                    .glb (glTF binary) ──► Three.js
```

**Plain English:** The game's 3D models are locked inside Unreal Engine 3 package files (.upk). We use a tool called UModel to extract them, then convert them to a web-standard format (glTF) that Three.js can display in the browser. This is a one-time extraction process per game asset — the converted files are served alongside the admin UI.

---

## Migration Strategy — What Gets Ported, What Gets Replaced

### Python Codebase Breakdown (~26,140 lines)

| Category | Lines | % | Strategy |
|---|---|---|---|
| **Auto-generated code** (enums, constants, property maps) | ~10,400 | 40% | **Eliminated** — `build.rs` generates this from .def files |
| **Gameplay mechanics** (combat, abilities, inventory, missions) | ~7,800 | 30% | **Ported to Rust** — module by module into `game` crate |
| **Content scripts** (interactions, spawners, mission scripts) | ~5,200 | 20% | **Replaced** by content engine chains in database |
| **Admin/debug** (GM commands, console, logging) | ~2,600 | 10% | **Replaced** by admin UI + command registry |
| **Python bindings** (C++↔Python bridge code in C++) | ~2,200 | — | **Eliminated** — no Python embedding needed |

**Plain English:** About 40% of the Python code is boilerplate that gets auto-generated (enum definitions, property mappings). The code generator eliminates all of that. Another 20% is scripted content (mission steps, NPC interactions) that the content engine replaces with database configuration. The core 30% — actual game rules like "how does combat damage work" — gets carefully rewritten in Rust with the same logic but better type safety. The remaining 10% (admin tools) is replaced by the web UI.

### C++ Codebase Breakdown (~22,210 lines)

| Component | Lines | Strategy |
|---|---|---|
| Mercury networking (nub, channel, packet, bundle, encryption) | ~5,500 | **Ported to Rust** — byte-identical protocol |
| Entity system (base_entity, cell_entity, manager, properties) | ~4,800 | **Ported to Rust** — with code generation improvements |
| Service framework (auth, base, cell services) | ~3,200 | **Ported to Rust** — using tokio + axum |
| Space & world grid (spatial partitioning, AoI) | ~2,800 | **Ported to Rust** — with generic spatial structures |
| Python integration (Boost.Python bindings, GIL) | ~2,200 | **Eliminated** — no Python |
| Movement & navigation (controllers, Recast/Detour) | ~1,800 | **Ported to Rust** — using `recast-detour-rs` crate |
| Database layer (SOCI wrapper, async queries) | ~1,100 | **Replaced** by `sqlx` (async, compile-time checked queries) |
| Logging, config, utilities | ~800 | **Replaced** by `tracing` + `quick-xml` |

---

## Milestone Sequence

### Phase 1: "Client Connects" (~4-6 weeks)

**Goal:** A game client can connect, authenticate, and see an empty world.

**What gets built:**
- `common` crate (types, config, math)
- `mercury` crate (full protocol — packet, channel, nub, encryption, bundling)
- `services` crate (auth service only)
- `defs` crate (build.rs code generator, entity type registry)
- Basic Tauri shell with placeholder UI

**Verification:** Launch sgw.exe → enter credentials → client reaches character select (even if empty).

**Plain English:** This is the foundation. If the Mercury protocol isn't byte-perfect, nothing else works. This phase is all about getting the handshake right — the client connects, sends login credentials, the server authenticates and responds with the session, and the client proceeds to the next screen.

### Phase 2: "Player In World" (~4-6 weeks)

**Goal:** A player can log in, see their character in a world, and move around.

**What gets built:**
- `entity` crate (base_entity, cell_entity, manager, properties, space, world_grid)
- `game` crate (player.rs — basic login, character creation, enter world)
- `services` crate (base + cell services, inter-service communication)
- Basic entity synchronization (position updates, property sync)

**Verification:** Create character → enter world → walk around → see terrain → log out → log back in at same position.

### Phase 3: "Game Systems Online" (~6-8 weeks)

**Goal:** Core gameplay works — combat, abilities, inventory, NPCs, missions.

**What gets built:**
- `game` crate (combat/, inventory/, missions/, interactions/, social/)
- `content-engine` crate (chains, triggers, conditions, actions)
- `commands` crate (GM commands, admin commands)
- NPC spawning, AI, patrol, aggression

**Verification:** Fight mobs → gain XP → level up → learn abilities → complete a mission → receive rewards → buy from vendor.

### Phase 4: "Admin Dashboard" (~3-4 weeks)

**Goal:** Full admin UI with real-time monitoring and content editing.

**What gets built:**
- `admin-api` crate (REST routes, WebSocket streams)
- Frontend: Dashboard, Players, ContentEditor, ChainEditor, Logs, Config pages
- Tauri IPC integration for local mode
- JWT authentication for remote mode

**Verification:** Open admin UI → see live player list → edit an item in the content editor → see the change take effect in-game without restart.

### Phase 5: "3D World Viewer" (~3-4 weeks)

**Goal:** Three.js space viewer showing nav meshes and live entity positions.

**What gets built:**
- Frontend: SpaceViewer page, NavMeshRenderer, EntityMarkers
- WebSocket entity position streaming
- Nav mesh geometry extraction and conversion
- Camera controls (orbit, fly, follow-entity)

**Verification:** Open Space Viewer → see nav mesh wireframe → see player dots moving in real-time → click an entity to inspect it.

### Phase 6: "Full Parity + Polish" (~4-6 weeks)

**Goal:** Feature parity with current Python implementation. Production-ready.

**What gets built:**
- Remaining game systems (mail, auction house, guilds, PvP)
- Asset pipeline (UE3 extraction → glTF for enhanced 3D viewer)
- Heightmap terrain rendering in Space Viewer
- Remote admin deployment (static hosting configuration)
- Performance tuning, load testing
- Comprehensive logging and error handling

**Verification:** All current Python game features work identically in Rust. Admin UI works remotely from Azure Static Web Apps. Server handles target concurrent player count.

**Total estimated scope: ~24-34 weeks for full parity**

---

## Key Technical Decisions

| Decision | Choice | Why |
|---|---|---|
| **Async runtime** | tokio | Tauri uses it internally; one runtime for everything |
| **HTTP/WS framework** | axum | Built on tokio, excellent WebSocket support, tower middleware |
| **Database** | sqlx (PostgreSQL) | Compile-time query checking, async, connection pooling |
| **Encryption** | `aes` + `cbc` crates | Client requires AES-256-CBC (ring doesn't support CBC) |
| **HMAC** | `hmac` + `md5` crates | Client requires HMAC-MD5 (legacy but non-negotiable) |
| **XML parsing** | quick-xml | Fast, low-allocation, good for .def files and configs |
| **Frontend framework** | SolidJS | Smallest bundle, fastest updates, ideal for real-time dashboards |
| **3D engine** | Three.js | Industry standard for web 3D, huge ecosystem, well-documented |
| **Serialization** | bytes + manual | Mercury protocol requires exact byte layout — no serde |
| **Navigation** | recast-detour-rs | Rust bindings for same Recast/Detour library we use now |
| **Logging** | tracing | Structured, async-aware, subscriber-based (feed to UI + file) |
| **Entity codegen** | build.rs | Zero-runtime-cost types from existing .def XML files |
| **Config format** | Same XML | Existing .config files work unchanged |
| **Database schema** | Same SQL | Existing db/ schema works unchanged |

---

## Key Rust Dependencies

```toml
# Cargo.toml workspace dependencies
[workspace.dependencies]
tokio = { version = "1", features = ["full"] }
axum = { version = "0.8", features = ["ws"] }
sqlx = { version = "0.8", features = ["runtime-tokio", "postgres", "macros"] }
tauri = { version = "2" }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
bytes = "1"
quick-xml = "0.37"
aes = "0.8"
cbc = "0.1"
hmac = "0.12"
md5 = "0.7"                             # For HMAC-MD5 (client requires it)
rand = "0.9"
tracing = "0.1"
tracing-subscriber = "0.3"
jsonwebtoken = "9"                       # JWT for remote admin auth
tower = "0.5"                            # HTTP middleware
tower-http = { version = "0.6", features = ["cors"] }
recast-detour-rs = "0.1"                 # Navigation mesh
thiserror = "2"
anyhow = "1"
```

---

## Files to Create/Modify

| Path | Action | Description |
|---|---|---|
| `Cargo.toml` | **CREATE** | Workspace root with member crates |
| `crates/common/` | **CREATE** | Shared types, config, math, errors |
| `crates/mercury/` | **CREATE** | Mercury protocol (packet, channel, nub, encryption) |
| `crates/defs/` | **CREATE** | Entity definition code generator (build.rs) |
| `crates/entity/` | **CREATE** | Entity runtime (base, cell, space, grid, movement) |
| `crates/game/` | **CREATE** | All game logic (combat, inventory, missions, etc.) |
| `crates/content-engine/` | **CREATE** | Data-driven trigger/condition/action chains |
| `crates/services/` | **CREATE** | Auth, Base, Cell service orchestration |
| `crates/admin-api/` | **CREATE** | REST + WebSocket admin API (axum) |
| `crates/commands/` | **CREATE** | GM/admin command registry |
| `src-tauri/` | **CREATE** | Tauri application entry point and IPC |
| `frontend/` | **CREATE** | SolidJS admin UI + Three.js viewer |
| `entities/` | **UNCHANGED** | Existing .def files used by build.rs |
| `config/` | **UNCHANGED** | Existing XML configs loaded by common crate |
| `db/` | **UNCHANGED** | Existing SQL schema used by sqlx |
| `data/` | **UNCHANGED** | Existing game data files |
| `docs/architecture/tauri-rewrite.md` | **CREATE** | This plan as permanent documentation |

---

## Verification Plan

### Per-Phase Verification

Each phase has a concrete "it works when..." test described in the milestone section above.

### Protocol Verification

1. **Packet capture comparison** — Record network traffic from the C++ server handling a login sequence. Record the same sequence from the Rust server. Byte-compare the packets. They must match.
2. **Client smoke test** — The game client (`sgw.exe`) successfully completes: connect → login → character select → enter world → move → interact → log out.
3. **Stress test** — Multiple simultaneous client connections exercising the reliable UDP windowing, fragmentation, and retransmission.

### Game Logic Verification

1. **Feature parity checklist** — For each Python module, verify the corresponding Rust implementation produces identical game behavior.
2. **Database compatibility** — Same schema, same queries, same results. A database from the C++ server works with the Rust server without migration.
3. **Content engine validation** — Create chains in the admin UI that replicate existing Python script behavior. Verify identical outcomes.

### Admin UI Verification

1. **Local mode** — Dashboard shows real-time data, content editor saves to database, chain editor creates working chains, space viewer shows entity positions.
2. **Remote mode** — Same UI hosted on Azure Static Web Apps connects to server over HTTPS, authenticates with JWT, shows same data with acceptable latency (<200ms for REST, <50ms for WebSocket).
