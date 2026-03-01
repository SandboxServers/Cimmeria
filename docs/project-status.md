# Project Status

Where the Cimmeria server emulator stands today and what's ahead.

## What Works Right Now

### Build & Setup

The project builds and runs on modern hardware:

- **One-command setup**: `pwsh setup.ps1` downloads all dependencies, patches them for VS2026, builds everything, initializes the database, and starts the servers
- **All 6 projects compile**: Recast, UnifiedKernel, NavBuilder, AuthenticationServer, CellApp, BaseApp
- **Database loads**: PostgreSQL with full game data (112,626 rows)
- **Servers start**: Auth, Base, and Cell servers all launch and accept connections

### Authentication & Connection

- Client connects to the Auth server over HTTP
- Login with username/password works (test account: test/test)
- Shard list is returned and server selection works
- Session key is generated and shared with BaseApp
- Client connects to BaseApp via encrypted Mercury protocol
- Full world entry sequence works: createBasePlayer, spaceViewportInfo, createCellPlayer, forcedPosition
- Player enters the game world with a controllable character

### Mercury Protocol

- Reliable UDP transport works (packets sent, ACKs received)
- AES-256 encryption active on all traffic
- Resource data (abilities, items, etc.) served to client — 274+ packets sent successfully
- Inactivity timeout tuned to 5 minutes

### Game Data Pipeline

- All 22 resource categories served from database to client
- Cooked XML generation from Python resource classes works
- Client receives and processes game data

### In-Game Gameplay (Confirmed Working)

The emulator is playable. The following have been confirmed working in live gameplay in the Castle Cellblock zone:

- **World entry** — Player logs in, selects shard, enters the game world, and can move around
- **Entity spawning** — World objects and NPCs spawn and are visible to the player
- **NPC interaction** — Right-clicking on NPCs and world objects triggers interaction scripts
- **Mission scripts** — Python mission scripts load and execute (FindAmbernol quest runs end-to-end)
- **Mission step advancement** — Region enter, interact, kill, and use-item objectives all advance mission steps correctly
- **Inventory** — Items are given to the player and appear in inventory
- **Dialog display** — Dialog trees display correctly during NPC interactions
- **Sequence playback** — In-game cinematic/animation sequences play
- **Aggression/threat system** — NPCs become hostile and enter combat with the player
- **Combat** — Player can take cover and fight enemies; combat resolves with damage and kills
- **Client-hinted regions** — Entering named regions (e.g., Med Station) triggers step advances

### Script Compilation Pipeline

Mission and effect scripts have a two-stage pipeline:

- **Source**: `.script` XML files in `data/scripts/` — visual node graphs created with the Atrea Script Editor (part of ServerEd)
- **Output**: `.py` files in `python/cell/missions/` etc. — auto-generated Python code
- **Compiler**: `tools/ServerEd/scriptcompiler.cpp` (part of the Qt ServerEd tool)
- Python files are **not** regenerated on server start — they are static compiled output checked into the repo

## Known Issues

### FindAmbernol Quest — Missing Vial Outline

Mission 639 "Find Ambernol" works end-to-end, but the vial entity is missing its quest interaction outline (the visual glow that tells players "click this"). Entity template 18 has `interaction_type=0` — it lacks the `INT_MissionWorldObject` flag (bit 30, value 1073741824). The `FindAmbernol.py` script never calls `setInteractionType()` on the vial. Other Castle Cellblock scripts (Prisoner_329.py, ArmYourself.py, Castle_CellBlock.py space script) set interaction types dynamically, so the fix is to add `setInteractionType(entity.interactionType | 1073741824)` when the player reaches step 2145.

### Client Cache Corruption (Resolved)

Using ASEditor to edit a `.umap` file invalidates the client's shader cache and cooked data in `Documents/My Games/Firesky/`. This causes the game client to crash on subsequent launches, even after reinstalling the game. Deleting that folder fixes the client.

## What's Left to Build

### Phase 1: World Entry — DONE

The player can log in, select a shard, enter the game world, move around, interact with NPCs and objects, run quests, and engage in combat. The core gameplay loop works.

### Phase 2: Expanding Content

- **More zones** — Only Castle Cellblock is tested; other zones need their space scripts verified
- **Stargate travel** — 29 gates defined, travel logic coded, needs end-to-end testing with zone transitions
- **Chat system** — Already mostly implemented, needs integration testing
- **Crafting** — 499 blueprints, full logic coded, needs in-game testing

### Phase 3: Social

- **Organizations/Guilds** — Entity definitions exist (23KB), needs Python implementation
- **Mail system** — Database table exists, needs handlers
- **Auction house** — Entity definitions only
- **Complete minigames** — 3 of 10 done, 7 need implementation

### Phase 4: Polish

- **AI patrol/wander** — NPCs currently just stand in place
- **Loot table population** — System works but tables are nearly empty
- **Level/XP curves** — Need to be designed
- **Stat scaling** — Base stats per level need definition

## What We Have to Work With

### Reference Source Code

We have the source code for the server middleware:

| Component | What It Tells Us |
|-----------|-----------------|
| **BigWorld 1.9.1 + 2.0.1** | Exactly how the Mercury protocol works, how entities are created, how the server is supposed to behave |

### Game Data

Everything the original game had in terms of content data:
- 112,626 rows of game data across 65 database tables
- 1,887 abilities, 6,060 items, 1,041 missions, 3,217 effects
- 5,406 dialog trees, 499 crafting blueprints, 29 stargates
- 153 NPC templates, 91 world definitions
- 29,126 localized text strings
- Complete entity definitions with 17 types and 20 interfaces
- 164 Python game logic scripts

### Reverse Engineering Documentation

16 technical documents covering:
- Complete binary analysis of sgw.exe
- All 421 network messages cataloged
- Full login/authentication flow mapped
- Mercury protocol analyzed
- BigWorld version identified
- All client tools documented
- Source reconstruction feasibility assessed

## Feasibility Rating: PROVEN

The original feasibility assessment rated this project as "MODERATE-HIGH" based on the assumption that content data was largely missing. After analysis and live testing, this has been upgraded to **PROVEN** — the emulator is running and playable. A player can log in, enter the world, move around, interact with NPCs, run quests, fight enemies, and advance through mission objectives.

The remaining work is:

1. **Breadth** — Testing and fixing more zones, missions, and content beyond Castle Cellblock
2. **Social systems** — Organizations, mail, auction house, more minigames
3. **Polish** — AI behavior, loot tables, XP curves, stat scaling
4. **Interaction type bugs** — Some entities missing visual quest indicators (see Known Issues)

The hardest part (reverse engineering the protocol, understanding the architecture, and getting a client into the game world) is done. What remains is expanding content coverage and implementing social features.
