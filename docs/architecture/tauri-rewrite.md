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

---

## Rust Workspace Layout

### Crate Structure

```
cimmeria/
├── Cargo.toml                          # Workspace root
├── crates/
│   ├── common/                         # Shared types, math, config
│   ├── mercury/                        # Network protocol (byte-identical to C++)
│   ├── defs/                           # Entity definition code generator
│   ├── entity/                         # Entity system runtime
│   ├── game/                           # Game logic (replaces ALL Python)
│   ├── content-engine/                 # Data-driven content system
│   ├── services/                       # Service orchestration
│   ├── admin-api/                      # REST + WebSocket admin interface
│   └── commands/                       # GM/admin command registry
├── src-tauri/                          # Tauri application shell
├── frontend/                           # SolidJS admin UI
├── entities/                           # Entity definitions (UNCHANGED)
├── config/                             # Service configuration (UNCHANGED)
├── db/                                 # Database schema (UNCHANGED)
├── data/                               # Game data files (UNCHANGED)
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

---

## Key Technical Designs

### 1. Entity System — Compile-Time Code Generation

A `build.rs` script reads `.def` XML files at **compile time** and generates native Rust structs. Property access is a direct struct field read, wire serialization is derived automatically, and type errors are caught by the compiler.

### 2. Mercury Protocol — Byte-Identical Reimplementation

**Critical Protocol Constants:**

| Constant | Value |
|---|---|
| `PACKET_MAX_SIZE` | 1472 bytes |
| `HEADER_SIZE` | 4 bytes |
| `MAX_BODY` | 1348 bytes |
| `RX_WINDOW_SIZE` | 64 packets |
| `TX_WINDOW_SIZE` | 45 packets |
| `ACK_TIMEOUT` | 700 ms |
| `MAX_RETRIES` | 20 |
| `KEEPALIVE_INTERVAL` | 1000 ms |
| `MAX_FRAGMENTS` | 64 |
| `PROTOCOL_VERSION` | 391 |

**Encryption:** AES-256-CBC + HMAC-MD5 (client requires these specific algorithms).

### 3. Content Engine — Database-Driven Game Logic

Replaces hand-written Python scripts with database-configured trigger → condition → action chains.

**11 triggers**, **7 conditions**, **21 actions** — all configurable through the admin UI.

### 4. Frontend — SolidJS + Three.js

Progressive 3D fidelity: nav mesh wireframe (Phase 1) → heightmap terrain (Phase 2) → extracted UE3 geometry (Phase 3).

---

## Key Technical Decisions

| Decision | Choice | Why |
|---|---|---|
| Async runtime | tokio | Tauri uses it internally |
| HTTP/WS framework | axum | Built on tokio, tower middleware |
| Database | sqlx (PostgreSQL) | Compile-time query checking, async |
| Encryption | `aes` + `cbc` | Client requires AES-256-CBC |
| HMAC | `hmac` + `md5` | Client requires HMAC-MD5 |
| XML parsing | quick-xml | Fast, good for .def files |
| Frontend framework | SolidJS | Smallest bundle, fastest updates |
| 3D engine | Three.js | Industry standard for web 3D |
| Serialization | bytes + manual | Mercury requires exact byte layout |
| Navigation | recast-detour-rs | Same Recast/Detour library |
| Logging | tracing | Structured, async-aware |
| Entity codegen | build.rs | Zero-runtime-cost types from XML |

---

## Milestone Sequence

### Phase 1: "Client Connects" (~4-6 weeks)
Client can connect, authenticate, and see an empty world.

### Phase 2: "Player In World" (~4-6 weeks)
Player can log in, see character in world, move around.

### Phase 3: "Game Systems Online" (~6-8 weeks)
Combat, abilities, inventory, NPCs, missions all functional.

### Phase 4: "Admin Dashboard" (~3-4 weeks)
Full admin UI with real-time monitoring and content editing.

### Phase 5: "3D World Viewer" (~3-4 weeks)
Three.js space viewer with nav meshes and live entity positions.

### Phase 6: "Full Parity + Polish" (~4-6 weeks)
Feature parity with Python implementation. Production-ready.

**Total: ~24-34 weeks for full parity**
