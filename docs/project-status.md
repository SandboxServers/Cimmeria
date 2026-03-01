# Project Status

Where the Cimmeria server emulator stands today and what's ahead.

## Summary

The core **infrastructure** works: build pipeline, authentication, Mercury protocol transport, game data serving, and basic world entry are all functional. A player can log in, enter the game world, and move around.

Beyond that, many game systems have code — some from the original developers' Python scripts, some from our implementation — but most haven't been rigorously tested against the live client, and many have known gaps or reliability issues. Only the login/auth flow has been tested thoroughly enough to call solid.

## System Status

### Legend

| Status | Meaning |
|--------|---------|
| **Solid** | Tested end-to-end, works reliably |
| **Working** | Functional but incomplete, known gaps, or lightly tested |
| **Partial** | Some code exists, core flow may work but significant features are missing or untested |
| **Stub** | Entity definitions and method signatures exist but implementation is empty or placeholder |
| **Not Started** | No implementation beyond what the entity defs provide for free |

### Infrastructure

| System | Status | Notes |
|--------|--------|-------|
| Build & setup | **Solid** | `pwsh setup-dependencies.ps1` builds all 6 projects on VS2026. One-command from clone to running servers. |
| Authentication & login | **Solid** | Full flow: HTTP login → shard select → session key → Mercury connect. Tested, reliable. |
| Mercury protocol | **Solid** | Reliable UDP transport with AES-256 encryption, ACKs, inactivity timeout. Core transport works. |
| Game data pipeline | **Solid** | All 22 resource categories served from database to client (274+ packets). Client receives and processes data. |
| Database | **Solid** | PostgreSQL with 112,626 rows across 65 tables. Schema loads, connections work, data serves. |
| Python embedding | **Solid** | Boost.Python 3.4 embedding works. Entity scripts load and execute. |
| Mercury reliability layer | **Working** | Basic ACKs work, but missing cumulative ACKs, piggyback support, and advanced congestion control that BigWorld has. Sufficient for single-player testing, untested under load. |

### World & Entities

| System | Status | Notes |
|--------|--------|-------|
| World entry | **Working** | createBasePlayer → spaceViewportInfo → createCellPlayer → forcedPosition sequence works. Player enters the world. Occasional desync on reconnect. |
| Entity spawning | **Working** | NPCs and world objects spawn and are visible. Based on space scripts, not dynamic spawning. |
| Movement | **Working** | Player can move around. Server tracks position. No server-side validation of movement speed/teleporting. |
| Area of Interest | **Working** | Entities appear/disappear based on distance. Basic grid-based AoI, no LOD levels. |
| Character creation | **Partial** | Data exists (archetypes, visual options) but server handling is minimal. The flow runs but output is limited. |
| Navigation meshes | **Working** | NavBuilder generates meshes, server loads them. Pathfinding available but NPC AI doesn't use it yet. |

### Game Systems

| System | Status | Notes |
|--------|--------|-------|
| NPC interaction & dialog | **Working** | Right-click triggers interaction scripts, dialog trees display. Tested in Castle Cellblock only. |
| Mission scripting | **Working** | Region enter, interact, kill, use-item objectives all advance. FindAmbernol runs end-to-end. **One zone tested** — other zones need verification. |
| Combat (basic) | **Working** | Abilities fire, effects apply, damage resolves, death/respawn works. But: no diminishing returns, limited stat scaling, combat formulas unvalidated against original. |
| Inventory | **Working** | Items are given and appear in inventory. Bag management works. Move/equip less tested. |
| Stats | **Working** | Base stats sync to client, stat updates display. Derived stat formulas (armor, resistances) need validation. |
| Abilities | **Partial** | 1,887 defined in database, Python pipeline exists (1,090 lines). Basic firing works but many abilities untested. Cooldowns, warmups, and edge cases unverified. |
| Chat | **Partial** | Python logic exists (365 lines) for channels and messaging. **Not tested with live client.** |
| Crafting | **Partial** | 499 blueprints in database, Python logic exists (532 lines). **Not tested with live client.** |
| Stargate travel | **Partial** | 29 gates defined, DHD UI works, dialing sequence coded. Gate transitions (zone travel) are incomplete — GateTravel.py is mostly stub. |
| Trading | **Partial** | Python logic exists (273 lines). **Not tested with live client.** |
| Stores/vendors | **Working** | Store UI opens, item purchase works. Buyback less tested. |
| Loot | **Partial** | Loot system works but loot tables are nearly empty — most mobs drop nothing. |
| XP & leveling | **Stub** | XP awards exist in mission rewards but level-up thresholds and XP curves are not defined. |
| Stat scaling | **Stub** | No per-level stat growth tables, no diminishing returns formulas. |
| Enemy AI | **Stub** | Mobs spawn and enter combat when provoked, but have no patrol paths, wander behavior, aggro leashing, or tactical AI. They stand in place until attacked. |
| Organizations/guilds | **Stub** | Entity definitions exist, all 15 RPC methods are empty stubs. |
| Mail | **Stub** | Read-only header retrieval partially works. Sending, archiving, deleting are all stubs. |
| Contact lists | **Stub** | All 6 methods are empty stubs. |
| Auction house | **Stub** | Entity definitions exist. Both Python files are empty stubs. |
| Dueling | **Stub** | Empty stubs. |
| Pets | **Stub** | Entity definitions exist. No behavior implementation. |
| Minigames | **Stub** | Framework exists (URL-launched web/Flash games). 3 of 9 have full logic, 6 are placeholders. None tested with client. |

### Content Coverage

| Content Type | Total in DB | Tested/Verified | Notes |
|--------------|-------------|-----------------|-------|
| Zones | 91 world definitions | 1 (Castle Cellblock) | Other zones have space scripts but are untested |
| Missions | 1,041 | ~5 in Castle Cellblock | Most missions reference scripts that exist but are unverified |
| Abilities | 1,887 | ~10-20 | Basic attack abilities tested, vast majority untested |
| Items | 6,060 | ~20-30 | Quest items and basic gear tested |
| NPCs | 153 templates | ~10 | Castle Cellblock NPCs tested |
| Dialog trees | 5,406 | ~10 | Castle Cellblock dialogs tested |
| Stargates | 29 | 0 end-to-end | DHD UI works, gate passage does not |
| Crafting blueprints | 499 | 0 | System untested |

## Known Issues

### FindAmbernol Quest — Missing Vial Outline

Mission 639 "Find Ambernol" works end-to-end, but the vial entity is missing its quest interaction outline (the visual glow that tells players "click this"). Entity template 18 has `interaction_type=0` — it lacks the `INT_MissionWorldObject` flag (bit 30, value 1073741824). The `FindAmbernol.py` script never calls `setInteractionType()` on the vial. Fix: add `setInteractionType(entity.interactionType | 1073741824)` when the player reaches step 2145.

### Client Cache Corruption

Using ASEditor to edit a `.umap` file invalidates the client's shader cache and cooked data in `Documents/My Games/Firesky/`. This causes the game client to crash on subsequent launches. Deleting that folder fixes the client.

### Combat Formula Gaps

Combat works at a basic level but many formulas are unvalidated:
- No diminishing returns on stats
- Armor/resistance calculations may not match original
- Ability scaling with level is undefined
- Area-of-effect abilities untested
- Cover system mechanics are minimal

### Mercury Protocol Gaps

The transport layer works but lacks some BigWorld features:
- No cumulative ACKs (individual only)
- No piggyback packet support (explicitly rejected)
- Native little-endian byte ordering (BigWorld uses big-endian) — works because both client and server are x86
- Untested with multiple simultaneous clients

## What We Have to Work With

### Cimmeria Server Source

~18,861 lines of C++ across ~120 source files implementing the full server stack (Auth, Base, Cell, NavBuilder, ServerEd, UnifiedKernel).

### Original Python Scripts

164 Python files from the original developers covering entity behavior, mission scripting, NPC interactions, ability management, crafting, chat, and more. These are the original game logic — not reimplementations.

### Game Data

Everything the original game had in terms of content data:

| Category | Count |
|----------|-------|
| Database rows | 112,626 across 65 tables |
| Abilities | 1,887 |
| Items | 6,060 |
| Missions | 1,041 |
| Effects | 3,217 |
| Dialog trees | 5,406 |
| Crafting blueprints | 499 |
| Stargates | 29 |
| NPC templates | 153 |
| World definitions | 91 |
| Localized strings | 29,126 |
| Entity types | 18 types + 18 interfaces |

### BigWorld Reference Source

Source code for BigWorld Technology 1.9.1 and 2.0.1, the server middleware SGW was built on. This tells us exactly how the protocol, entity system, and server architecture are supposed to work.

### Reverse Engineering Documentation

89 documents covering the complete binary analysis of the SGW client:

| Category | Documents | Highlights |
|----------|-----------|------------|
| Protocol | 5 | Mercury wire format, entity property sync, login handshake, position updates, message catalog (975 events) |
| Gameplay | 19 | Per-system breakdowns for every game feature with wire formats |
| Engine | 8 | BigWorld internals, CME framework, space management, LOD, checkpointing |
| Architecture | 1 | Cimmeria service topology and configuration reference |
| Analysis | 2 | Event-net mapping (~98% coverage), BigWorld source cross-reference index |
| RE findings | 17 | Per-system wire format documents with decompilation evidence |
| Guides | 3 | Evidence standards, reading decompiled code, entity definition guide |
| Technical | 16 | Legacy analysis documents from initial RE investigation |

The Ghidra annotation effort named **101,909 of 168,239 non-thunk functions (60.6%)** in the client binary via 10 automated scripts. Wire formats for every entity RPC method have been derived and cross-validated against Python scripts, `.def` files, and BigWorld reference source.

### Entity Definitions

Complete XML entity type definitions with properties, methods, and client/server/cell split annotations. These are the authoritative source for wire format derivation — all entity method calls serialize according to their `.def` + `alias.xml` type definitions via a universal RPC dispatcher.

## Roadmap

### Near-term: Solidify What Exists

- **Test other zones** — Verify space scripts beyond Castle Cellblock
- **Character creation** — Make the full creation flow work (archetype, appearance, name)
- **Combat validation** — Compare ability/effect/stat formulas against original client behavior
- **Chat testing** — Verify the existing Python chat logic with a live client
- **Stargate travel** — Complete GateTravel.py to enable zone transitions

### Medium-term: Fill in Partial Systems

- **Crafting** — Test and fix the existing 532-line implementation
- **Trading** — Test and fix the existing 273-line implementation
- **Loot tables** — Populate from game data so mobs drop appropriate items
- **Abilities** — Test ability categories beyond basic attacks (heals, buffs, debuffs, AoE)
- **Enemy AI** — Add patrol paths, wander behavior, aggro leashing

### Long-term: Build Missing Systems

- **Organizations/guilds** — Implement the 15 RPC methods (roster, ranks, MOTD, etc.)
- **Mail** — Implement sending, archiving, deleting, item attachments
- **Auction house** — Implement search, listing, bidding
- **XP curves & leveling** — Design and implement level progression
- **Stat scaling** — Define per-level stat growth and diminishing returns
- **Contact lists, dueling, pets** — Implement from stubs
- **Minigames** — Complete the 6 placeholder implementations

### Aspirational

- **Multi-player testing** — Stress test with concurrent connections
- **Server resilience** — Implement BigWorld-style distributed checkpointing
- **Cross-platform** — Linux server builds via CMake migration
- **CI/CD** — Automated build and test pipeline
