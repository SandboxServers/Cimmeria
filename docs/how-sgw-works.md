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

- **EventSignal Framework** — A publish-subscribe event system that connects all the game pieces together (954 unique event types)
- **CookedData Pipeline** — A system for baking game data (abilities, items, missions) from a database into XML files the client can consume
- **Python Scripting** — Game logic written in Python 3.4, embedded via Boost.Python. This is where combat formulas, mission scripts, NPC behavior, and most gameplay code lives
- **Visual Script Editor (Atrea Script Editor)** — A node-graph editor for creating effects and mission logic without writing code. The `.script` XML files in `data/scripts/` are the source; the compiler in `tools/ServerEd/scriptcompiler.cpp` generates the Python files in `python/cell/missions/` etc. The compiled Python is static and not regenerated at server startup
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

To connect the client to our server, the **AtreaRL loader** patches the login URL at runtime so the client talks to localhost instead of `stargateworlds.com`.

## Key Numbers

| Metric | Count |
|--------|-------|
| Total functions in sgw.exe | ~169,420 |
| C++ classes identified | 4,943 |
| Network message types | 421 (167 server-to-client + 254 client-to-server) |
| Slash commands | 256 |
| Event signal types | 954 |
| Python game scripts | 164 files |
| Entity types | 17 |
| Source files referenced in binary | 608 |

## Reference Sources Available

| Component | Version | Source Available |
|-----------|---------|----------------|
| Unreal Engine 3 | Build 3717 | Internet Archive |
| BigWorld Technology | 1.9.1 + 2.0.1 | GitHub (open-sourced) |
| CEGUI | 0.6.x | GitHub (open-source) |
| Scaleform/GFx | ~4.0.7 | Internet Archive |

Having reference source for all major middleware components means ~60-70% of the binary's functions can be identified and understood without pure reverse engineering.
