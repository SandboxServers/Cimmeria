---
title: "How Stargate Worlds Works"
type: explanation
audience: new hires, engineers
last_updated: 2026-07-25
---

# How Stargate Worlds Works

Stargate Worlds is built from three major technology layers stacked together, plus custom code by Cheyenne Mountain Entertainment (CME).

## The Technology Stack

### Unreal Engine 3 (Graphics & Client)

The game client (`sgw.exe`) is built on **Unreal Engine 3** — the same engine behind Gears of War, Mass Effect, and many other games from that era. UE3 handles:

- 3D rendering, lighting, and visual effects
- Character models and animations
- Map/level loading (the `.umap` files in CookedPC)
- Physics (via PhysX/Novodex)
- Audio (via FMOD)
- The Kismet visual scripting system for in-game events
- UnrealScript for gameplay hooks

SGW uses **UE3 Build 3717** from early 2008. The full UE3 source for this build is available as a reference.

### BigWorld Technology (Networking & MMO Infrastructure)

For the MMO-specific parts — networking, entity management, server architecture — CME licensed **BigWorld Technology**, an Australian MMO middleware platform. BigWorld provides:

- **Mercury Protocol** — A custom reliable UDP protocol for fast client-server communication
- **Entity System** — A distributed object model where game objects (players, NPCs, items) have synchronized state across client and server
- **Server Architecture** — Multiple server roles working together:
  - **LoginApp** — Handles initial authentication
  - **BaseAppMgr** — Assigns players to BaseApp instances
  - **BaseApp** — Manages persistent player state and client connections
  - **CellApp** — Runs the spatial simulation (movement, combat, AI)
  - **DBMgr** — Database operations

SGW uses **BigWorld ~1.9.x** (between versions 1.9.1 and 2.0.1), with several custom modifications by CME. The BigWorld source code was later open-sourced after the company went bankrupt, and both 1.9.1 and 2.0.1 are available on GitHub as reference.

### CME Custom Layer (Game-Specific)

On top of UE3 and BigWorld, CME built their own systems:

- **EventSignal Framework** — A publish-subscribe event system that connects all the game pieces together (750 unique event types)
- **CookedData Pipeline** — A system for baking game data (abilities, items, missions) from a database into XML files the client can consume
- **Python Scripting** — Game logic written in Python 3.4, embedded via Boost.Python. This is where combat formulas, mission scripts, NPC behavior, and most gameplay code lives
- **Visual Script Editor (Atrea Script Editor)** — A node-graph editor for creating effects and mission logic without writing code. The `.script` XML files are the source; a compiler generates the Python. The compiled Python is static and not regenerated at server startup. All three now sit under `deprecated/` in this repo: sources in `deprecated/data-scripts/scripts/` (30 files), compiler at `deprecated/cpp-tools/ServerEd/scriptcompiler.cpp`, output in `deprecated/python/cell/missions/`
- **SpaceViewport System** — A custom extension to BigWorld for handling Stargate zone transitions (not present in any public BigWorld release)

### UI Systems

SGW ships with **two** UI rendering systems:
- **CEGUI** — An open-source UI library with Lua scripting (438 classes in the binary)
- **Scaleform/GFx** — A Flash-based UI system for rich animated interfaces (271 classes)

This dual system likely reflects a transition partway through development.

## How the Pieces Connect

```
[Player's Computer]                    [Server Cluster]

  SGW.exe                               AuthenticationServer
  ├── Unreal Engine 3                    ├── SOAP Login (HTTP)
  │   ├── Rendering                      └── Session Key Generation
  │   ├── Audio (FMOD)
  │   ├── Physics (PhysX)               BaseApp
  │   └── Kismet Events                 ├── Mercury Protocol (UDP)
  │                                      ├── Player Sessions
  ├── BigWorld Client                    ├── Entity Management
  │   ├── Mercury Protocol ◄──────────► ├── Python Game Logic
  │   ├── Entity Manager                 └── Database (PostgreSQL)
  │   └── ServerConnection
  │                                      CellApp
  ├── CME Framework                      ├── Spatial Simulation
  │   ├── EventSignal Bus                ├── Movement & Physics
  │   ├── CookedData Cache               ├── Combat & AI
  │   └── Network Manager               └── Area of Interest
  │
  └── UI (CEGUI + Scaleform)
```

## The Emulator Approach

Cimmeria reimplements the **server side** — AuthenticationServer, BaseApp, and CellApp — while using the **original unmodified client**. The client doesn't know (or care) that it's talking to our server instead of the original CME servers.

To connect the client to our server, the hardcoded SOAP login hostname `www.stargateworlds.com` has to be redirected. Cimmeria ships its own launcher (`crates/launcher/`) which does this as a one-time on-disk `.rdata` byte patch of SGW.exe — the replacement host is written into the original 22-byte slot, zero-padded, so no surrounding strings or pointers shift (`crates/launcher/src/patch_rdata.rs`). Data-section edits avoid ASLR and PE-checksum recalculation, which is why this was chosen over the runtime DLL injection the original **AtreaRL** loader used. AtreaRL remains an RE reference only; it is not part of the build.

## Key Numbers

| Metric | Count |
|--------|-------|
| Total functions in sgw.exe | ~173,225 |
| C++ classes identified | 4,943 |
| Network message types | 420 (167 server-to-client + 253 client-to-server) |
| Slash commands | 256 |
| Event signal types | 750 |
| Python game scripts | 164 files |
| Entity types | 18 |
| Source files referenced in binary | 608 |

Counts are distinct names, not string occurrences — the same `Event_*` name is emitted at several addresses, so raw occurrence counts run three to six times higher. The function count tracks the Ghidra database and creeps upward as analysis proceeds.

## Reference Sources Available

| Component | Version | Source Available |
|-----------|---------|----------------|
| Unreal Engine 3 | Build 3717 | Internet Archive |
| BigWorld Technology | 1.9.1 + 2.0.1 | GitHub (open-sourced) |
| CEGUI | 0.6.x | GitHub (open-source) |
| Scaleform/GFx | ~4.0.7 | Internet Archive |

Having reference source for all major middleware components means ~60-70% of the binary's functions can be identified and understood without pure reverse engineering.
