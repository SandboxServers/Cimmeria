# Cimmeria Reverse Engineering Documentation

Server emulator for **Stargate Worlds** (SGW), the cancelled MMO built on BigWorld Technology and the Cheyenne Mountain Entertainment (CME) game framework. The reverse engineering effort targets `sgw.exe` -- the game client binary -- loaded in Ghidra, with the goal of understanding the client-server protocol and game mechanics well enough to build a faithful server replacement.

The emulator is **playable today**: players can log in, enter the world, interact with NPCs, run quests, and engage in combat. These docs capture everything learned during the RE process and track what remains.


## Quick Stats

| Metric | Count |
|--------|-------|
| Event_NetOut messages (client to server) | 253 |
| Event_NetIn messages (server to client) | 167 |
| Total cataloged network messages | 420 |
| Event-to-.def mapping coverage | ~98% |
| Entity types | 18 |
| Interfaces | 18 |
| Named functions in sgw.exe | 101,909 (60.6%) |
| Remaining unnamed functions | ~66,330 |
| Python game logic scripts | 164 |
| Database rows (game data) | 112,626 |
| Abilities / Items / Missions / Effects | 1,887 / 6,060 / 1,041 / 3,217 |
| Documentation files | 114 |


## Document Map

### Top-Level Documents

| Document | Description |
|----------|-------------|
| [How SGW Works](how-sgw-works.md) | Technology overview -- BigWorld, CME, and how the pieces fit together |
| [Client Tools](client-tools.md) | Launcher, editor mode, debug tools available in the client |
| [Operator Guide](operator-guide.md) | Setup, configure, run, and connect a game client -- everything from git clone to playing |
| [Developer Guide](developer-guide.md) | How to extend the server: entities, properties, methods, commands, interactions, missions |
| [Building the Server](building.md) | How to build and run the Cimmeria server emulator |
| [Game Systems](game-systems.md) | Survey of every game feature: combat, abilities, stargates, missions, crafting |
| [Game Data](game-data.md) | What game content exists (items, abilities, missions) and what is missing |
| [Commands Reference](commands.md) | Player commands, chat, GM tools, and debug commands |
| [Connection Flow](connection-flow.md) | End-to-end login and world entry sequence |
| [Network Messages](network-messages.md) | High-level catalog of client-server messages |
| [Project Status](project-status.md) | What works, what is left, and the roadmap |
| [Gap Analysis](gap-analysis.md) | Comprehensive system-by-system gap analysis with per-feature status tracking |

---

### `content/` -- Game Content Data Audit

Content-level audit of all game data: zone completeness, mission chains, cross-references, orphaned content, and reconstruction potential. 7 documents covering 1,040 missions, 6,059 items, 3,216 effects, 153 NPCs, and 24 zones.

| Document | Description | Status |
|----------|-------------|--------|
| [README.md](content/README.md) | **HUB** -- Playability matrix, content summary, zone progression, reconstruction priority | Complete |
| [content-inventory.md](content/content-inventory.md) | Statistical inventory of all content types with completeness metrics | Complete |
| [zone-audit.md](content/zone-audit.md) | Per-zone completeness scorecard (2 PLAYABLE, 2 PARTIAL, 5 transport-only, 14 SHELL, 1 DATA-ONLY) | Complete |
| [mission-chains.md](content/mission-chains.md) | All 1,040 missions: scripted chains, unscripted analysis, inferred reconstruction | Complete |
| [association-map.md](content/association-map.md) | The crazy wall: cross-references, broken FKs, orphaned content, reconstruction web | Complete |
| [archetype-content-map.md](content/archetype-content-map.md) | Per-archetype content availability (2 implemented, 6 placeholder) | Complete |
| [reconstruction-map.md](content/reconstruction-map.md) | What can be rebuilt vs holes vs never-built, priority recommendations | Complete |
| [external-data-analysis.md](content/external-data-analysis.md) | Analysis of 11 external dev team spreadsheets and text files | Complete |

See also: [gap-analysis.md](gap-analysis.md), [game-data.md](game-data.md)

---

### `protocol/` -- Wire Formats and Messaging

Network protocol internals: packet structures, Mercury reliable messaging, entity property synchronization, and specific message flows.

| Document | Description | Status |
|----------|-------------|--------|
| [message-catalog.md](protocol/message-catalog.md) | **HUB** -- Complete catalog of all 420 network messages with IDs, directions, and payload structures | Complete |
| [mercury-wire-format.md](protocol/mercury-wire-format.md) | Mercury packet header, reliable sequencing, AES-256 encryption, ACK/NACK handling | Complete |
| [entity-property-sync.md](protocol/entity-property-sync.md) | How entity properties are serialized, delta-compressed, and synchronized client/server | Complete |
| [login-handshake.md](protocol/login-handshake.md) | HTTP auth, SOAP schemas, session key exchange, baseAppLogin binary format, error recovery | Complete |
| [position-updates.md](protocol/position-updates.md) | 32 avatarUpdate variants, packed formats, SVID aliasing, client prediction/reconciliation | Complete |

See also: [technical/mercury-protocol.md](technical/mercury-protocol.md), [technical/mercury-audit.md](technical/mercury-audit.md), [technical/login-auth-flow.md](technical/login-auth-flow.md), [technical/post-auth-sequence.md](technical/post-auth-sequence.md), [technical/network-messages.md](technical/network-messages.md)

---

### `gameplay/` -- Game System Documentation

Per-system breakdowns of game mechanics, derived from RE analysis, entity definitions, and Python scripts. 24 per-system documents covering combat, abilities, effects, stats, inventory, crafting, missions, travel, cinematics, minigames, social systems, NPC AI, spawning, loot, progression, and character creation.

| Document | Description | Status |
|----------|-------------|--------|
| [README.md](gameplay/README.md) | **HUB** -- System dashboard with status and cross-references for every gameplay system | Complete |
| [combat-system.md](gameplay/combat-system.md) | Cover mechanics, threat/aggro, damage resolution, death/respawn | Complete |
| [ability-system.md](gameplay/ability-system.md) | Ability activation, cooldowns, targeting, channeling, combos | Complete |
| [effect-system.md](gameplay/effect-system.md) | Buffs, debuffs, DoTs, HoTs, effect stacking and priority | Complete |
| [stat-system.md](gameplay/stat-system.md) | Base stats, derived stats, level scaling, equipment modifiers | Complete |
| [inventory-system.md](gameplay/inventory-system.md) | Item slots, stacking, equipment, bag management | Complete |
| [crafting-system.md](gameplay/crafting-system.md) | Blueprints, material requirements, crafting stations, 499 recipes | Complete |
| [mission-system.md](gameplay/mission-system.md) | Quest objectives, step advancement, rewards, mission scripts | Complete |
| [gate-travel.md](gameplay/gate-travel.md) | Stargate dialing, 29 defined gates, zone transitions | Complete |
| [minigame-system.md](gameplay/minigame-system.md) | 10 minigames (3 implemented, 7 remaining), lockpicking, hacking, etc. | Complete |
| [organization-system.md](gameplay/organization-system.md) | Guilds/organizations, ranks, permissions, roster management | Complete |
| [group-system.md](gameplay/group-system.md) | Party formation, loot rules, group chat | Complete |
| [contact-list.md](gameplay/contact-list.md) | Contact list management, named lists, flags, event notifications | Complete |
| [chat-system.md](gameplay/chat-system.md) | Chat channels, whispers, emotes, chat commands | Complete |
| [mail-system.md](gameplay/mail-system.md) | In-game mail, attachments, delivery | Complete |
| [trade-system.md](gameplay/trade-system.md) | Player-to-player trade, trade windows, confirmation | Complete |
| [black-market.md](gameplay/black-market.md) | Auction house, listings, bidding, buyout | Complete |
| [pet-system.md](gameplay/pet-system.md) | Pet summoning, commands, abilities | Complete |
| [duel-system.md](gameplay/duel-system.md) | PvP duel requests, rules, resolution | Complete |
| [npc-ai.md](gameplay/npc-ai.md) | NPC AI state machine, threat system, aggro, ability selection | Complete |
| [spawn-system.md](gameplay/spawn-system.md) | Spawn regions, spawn sets, population management, respawn timers | Complete |
| [loot-system.md](gameplay/loot-system.md) | Loot generation algorithm, loot tables, eligibility rules | Complete |
| [progression-system.md](gameplay/progression-system.md) | XP curves, leveling, stat growth, training points, applied science | Complete |
| [cinematic-system.md](gameplay/cinematic-system.md) | Kismet/Matinee sequences: stargate animations, ring transport, ability VFX, console activations, camera control | Complete |
| [character-creation.md](gameplay/character-creation.md) | Character creation flow, archetypes, visual choices, starting loadout | Complete |

See also: [game-systems.md](game-systems.md), [technical/game-systems.md](technical/game-systems.md), [technical/game-data-analysis.md](technical/game-data-analysis.md)

---

### `engine/` -- BigWorld and CME Internals

How the underlying BigWorld engine and CME game framework operate inside sgw.exe.

| Document | Description | Status |
|----------|-------------|--------|
| [entity-type-catalog.md](engine/entity-type-catalog.md) | **HUB** -- All 18 entity types with properties, methods, and interface bindings | Complete |
| [bigworld-architecture.md](engine/bigworld-architecture.md) | BigWorld 1.9.1 client internals: entity manager, space manager, connection layer | Complete |
| [cme-framework.md](engine/cme-framework.md) | CME framework: PropertyNode, EventSignal (750 types), Atrea scripts, SpaceViewport | Complete |
| [cooked-data-pipeline.md](engine/cooked-data-pipeline.md) | PAK/ZIP format, XML cooking, gSOAP deserialization, Mercury resource delivery | Complete |
| [watcher-system.md](engine/watcher-system.md) | BigWorld watcher/debug variable system (not used by SGW) | Complete |
| [space-management.md](engine/space-management.md) | WorldGrid AoI, 24 spaces, BSP trees, ghost entities, load balancing | Complete |
| [entity-lod-system.md](engine/entity-lod-system.md) | BigWorld entity property LOD and volatile info (not used by SGW) | Complete |
| [distributed-checkpointing.md](engine/distributed-checkpointing.md) | BigWorld backup/recovery system, Cimmeria gap analysis | Complete |

See also: [technical/bigworld-version-analysis.md](technical/bigworld-version-analysis.md), [technical/sgw-binary-overview.md](technical/sgw-binary-overview.md)

---

### `architecture/` -- Cimmeria Server Architecture

How the Cimmeria emulator itself is structured. 7 documents.

| Document | Description | Status |
|----------|-------------|--------|
| [service-architecture.md](architecture/service-architecture.md) | Auth, Base, Cell service topology, inter-service protocol, developer mode, console commands | Complete |
| [python-console.md](architecture/python-console.md) | Python console deep-dive: wire format, reference client, Atrea API, GM command table, security | Complete |
| [server-systems.md](architecture/server-systems.md) | Server-only infrastructure: session management, rate limiting, anti-cheat, economy, world state, scheduling, admin tools, metrics | Complete |
| [scaling-analysis.md](architecture/scaling-analysis.md) | Scaling strategy: current single-instance reality, BigWorld vs Cimmeria comparison, 5-tier scaling roadmap, capacity estimates | Complete |
| [tech-stack-replacement.md](architecture/tech-stack-replacement.md) | Tech stack replacement analysis: 5 options (incremental upgrade through full C# rewrite), codebase audit, protocol feasibility, phased recommendation | Complete |
| [data-driven-content-engine.md](architecture/data-driven-content-engine.md) | Data-driven content engine: replace per-script Python with DB-driven trigger/condition/action chains, full schema, worked examples, runtime implementation, migration path | Complete |
| [server-internals.md](architecture/server-internals.md) | C++ server architecture deep-dive: Mercury networking, entity system, BaseApp/CellApp internals, threading model, design patterns | Complete |

See also: [building.md](building.md), [connection-flow.md](connection-flow.md), [operator-guide.md](operator-guide.md), [developer-guide.md](developer-guide.md)

---

### `client/` -- Game Client Analysis

Analysis of game client binaries and launcher tools.

| Document | Description | Status |
|----------|-------------|--------|
| [sgw-launcher.md](client/sgw-launcher.md) | Custom launcher design: install pipeline, login redirect, implementation tech stacks | Complete |

See also: [client-tools.md](client-tools.md), [technical/launcher-exe.md](technical/launcher-exe.md), [technical/ateraloader-exe.md](technical/ateraloader-exe.md)

---

### `tools/` -- Development Tools

Design documents for development and administration tools.

| Document | Description | Status |
|----------|-------------|--------|
| [admin-panel.md](tools/admin-panel.md) | Web admin dashboard: architecture, Flask+React stack, py_client protocol bridge, DB queries, API routes | Complete |

---

### `analysis/` -- Investigation Logs

Working notes and cross-reference indexes from ongoing RE sessions.

| Document | Description | Status |
|----------|-------------|--------|
| [event-net-mapping.md](analysis/event-net-mapping.md) | 420 Event_NetIn/NetOut mapped to .def methods, Ghidra addresses, handler chains (~98% coverage) | Complete |
| [bigworld-reference-index.md](analysis/bigworld-reference-index.md) | Cross-reference: BigWorld 2.0.1 source symbols to sgw.exe addresses | Complete |

---

### `guides/` -- How-To Guides

Practical guidance for contributors working on the RE effort or the emulator.

| Document | Description | Status |
|----------|-------------|--------|
| [evidence-standards.md](guides/evidence-standards.md) | Confidence levels, citation format, how to document findings | Complete |
| [reading-decompiled-code.md](guides/reading-decompiled-code.md) | Tips for reading Ghidra decompiler output, common patterns, pitfalls | Complete |
| [entity-def-guide.md](guides/entity-def-guide.md) | How to read and write entity XML definitions and connect them to Python scripts | Complete |

---

### `reverse-engineering/` -- Ghidra Work

Annotation scripts, function naming progress, and per-system RE findings.

| Document | Description | Status |
|----------|-------------|--------|
| [PLAN.md](reverse-engineering/PLAN.md) | RE plan: phases, targets, methodology | Complete |
| [STATUS.md](reverse-engineering/STATUS.md) | RE status: 101,909/168,239 functions named (60.6%), all 5 phases complete | Complete |
| [function-naming-progress.md](reverse-engineering/function-naming-progress.md) | Naming conventions, coverage metrics, per-script results | Complete |
| [address-map.md](reverse-engineering/address-map.md) | Key address table: vtables, global objects, critical functions in sgw.exe | Complete |

#### `binaries/` -- Binary Analysis

| Document | Description | Status |
|----------|-------------|--------|
| [launcher-exe.md](reverse-engineering/binaries/launcher-exe.md) | CME Launcher.exe RE analysis: patch client internals, SOAP protocol | Complete |

#### `annotation-scripts/` -- Ghidra Jython Scripts (Phase 1 — all run)

| Script | Functions Named | Status |
|--------|----------------|--------|
| `01_label_rtti_typeinfo.py` | 5,063 | Complete |
| `02_label_defined_strings.py` | 1,006 | Complete |
| `03_label_entity_types.py` | 1,476 | Complete |
| `04_label_event_handlers.py` | 4,085 | Complete |
| `05_label_bw_common.py` | 1,367 | Complete |
| `06_label_boost_python.py` | 1,127 | Complete |
| `07_label_vtable_methods.py` | 34,367 | Complete |
| `08_label_import_callers.py` | 12,553 | Complete |
| `09_label_pattern_match.py` | 16,423 | Complete |
| `10_export_stats.py` | (export) | Complete |

#### `findings/` -- Per-System Wire Format Findings (Phases 2–4)

| Document | Messages | Description | Confidence |
|----------|----------|-------------|------------|
| [combat-wire-formats.md](reverse-engineering/findings/combat-wire-formats.md) | 29 | Universal RPC dispatcher, abilities, effects, stats, timers | HIGH |
| [inventory-wire-formats.md](reverse-engineering/findings/inventory-wire-formats.md) | 22 | Bags, items, equipment, repair, salvage | HIGH |
| [entity-types-wire-formats.md](reverse-engineering/findings/entity-types-wire-formats.md) | 14 | Entity creation, cache stamps, version info | HIGH |
| [entity-property-sync.md](reverse-engineering/findings/entity-property-sync.md) | — | Property ID assignment, delta encoding, create/update formats | HIGH |
| [mission-wire-formats.md](reverse-engineering/findings/mission-wire-formats.md) | 11 | Mission accept, advance, abandon, rewards | HIGH |
| [organization-wire-formats.md](reverse-engineering/findings/organization-wire-formats.md) | 18 | Guild create, invite, ranks, permissions, roster | HIGH |
| [crafting-wire-formats.md](reverse-engineering/findings/crafting-wire-formats.md) | 8 | Blueprints, crafting stations, research | HIGH |
| [gate-travel-wire-formats.md](reverse-engineering/findings/gate-travel-wire-formats.md) | 6 | DHD, gate activation, zone transitions | HIGH |
| [group-wire-formats.md](reverse-engineering/findings/group-wire-formats.md) | 4 | Group authority, member coordination, mob groups | HIGH |
| [minigame-wire-formats.md](reverse-engineering/findings/minigame-wire-formats.md) | 9 | Lockpick, hacking, fishing minigames | HIGH |
| [chat-wire-formats.md](reverse-engineering/findings/chat-wire-formats.md) | 6 | Channels, whispers, emotes | HIGH |
| [mail-wire-formats.md](reverse-engineering/findings/mail-wire-formats.md) | 5 | Send, receive, attachments, delete | HIGH |
| [black-market-wire-formats.md](reverse-engineering/findings/black-market-wire-formats.md) | 6 | Auction listings, bids, buyout | HIGH |
| [contact-list-wire-formats.md](reverse-engineering/findings/contact-list-wire-formats.md) | 4 | Named lists, flags, event notifications | HIGH |
| [trade-wire-formats.md](reverse-engineering/findings/trade-wire-formats.md) | 5 | Trade initiate, offer, accept/cancel | HIGH |
| [duel-wire-formats.md](reverse-engineering/findings/duel-wire-formats.md) | 3 | Duel request, accept, resolution | HIGH |
| [pet-wire-formats.md](reverse-engineering/findings/pet-wire-formats.md) | 2 | Pet summon, commands | HIGH |

---

### `technical/` -- Legacy Technical Documents

Detailed RE analysis documents created during initial investigation. These predate the reorganized directory structure above and contain the foundational analysis that drives everything else.

| Document | Description |
|----------|-------------|
| [sgw-binary-overview.md](technical/sgw-binary-overview.md) | Binary structure, sections, RTTI analysis, function counts |
| [network-messages.md](technical/network-messages.md) | Complete 420-message catalog with IDs, names, directions |
| [mercury-protocol.md](technical/mercury-protocol.md) | Mercury reliable UDP protocol analysis |
| [mercury-audit.md](technical/mercury-audit.md) | Audit of Mercury implementation against BigWorld reference |
| [login-auth-flow.md](technical/login-auth-flow.md) | Full login and authentication flow analysis |
| [post-auth-sequence.md](technical/post-auth-sequence.md) | What happens after auth: entity creation, world entry |
| [bigworld-version-analysis.md](technical/bigworld-version-analysis.md) | BigWorld version identification (1.9.1 client) |
| [game-systems.md](technical/game-systems.md) | Game systems analysis from the binary |
| [game-data-analysis.md](technical/game-data-analysis.md) | Analysis of game data content and coverage |
| [slash-commands.md](technical/slash-commands.md) | Client slash command handler analysis |
| [server-feasibility.md](technical/server-feasibility.md) | Server emulation feasibility assessment |
| [source-reconstruction-feasibility.md](technical/source-reconstruction-feasibility.md) | Source code reconstruction feasibility |
| [building.md](technical/building.md) | Build process technical details |
| [launcher-exe.md](technical/launcher-exe.md) | Launcher binary analysis |
| [ateraloader-exe.md](technical/ateraloader-exe.md) | AtreaLoader binary analysis |
| [atrealoader-config.md](technical/atrealoader-config.md) | AtreaLoader configuration format |
| [atrearl-loader.md](technical/atrearl-loader.md) | AtreaRL loader binary analysis |


## Key Data Sources

The most important files and directories for RE work, located relative to the project root.

| Source | Path | What It Contains |
|--------|------|------------------|
| Entity definitions | `entities/defs/*.def` | XML property/method definitions for all 18 entity types |
| Entity registry | `entities/entities.xml` | Master entity type list with type IDs |
| Interface definitions | `entities/defs/interfaces/*.def` | Shared property/method sets used across entity types |
| Custom type aliases | `entities/custom_alias.xml` | Type mappings for entity property serialization |
| Python game scripts | `python/` | 164 files: entity behavior, missions, combat, interactions |
| Database schema | `db/database.sql`, `db/sgw/`, `db/resources/` | PostgreSQL schema (split into per-domain directories) |
| Cooked game data | `data/cache/*.pak` | Client resource packs (items, abilities, effects, etc.) |
| Effect/mission scripts | `data/scripts/` | Source .script XML files (visual node graphs) |
| Space definitions | `entities/cell_spaces.xml` | Zone/cell partitioning configuration |
| Server configs | `config/*.config` | Service configuration (ports, DB, tuning parameters) |
| BigWorld reference | *(external)* | BigWorld 1.9.1 + 2.0.1 source for protocol/architecture reference |
| sgw.exe Ghidra project | *(external)* | The primary RE target, loaded in Ghidra |


## Phase Roadmap

| Phase | Focus | Status |
|-------|-------|--------|
| **Phase 1** | World Entry -- Login, auth, shard selection, world entry, movement | COMPLETE |
| **Phase 2** | Expanding Content -- More zones, stargate travel, chat, crafting | In Progress |
| **Phase 3** | Social Systems -- Organizations, mail, auction house, minigames | Planned |
| **Phase 4** | Polish -- AI patrols, loot tables, XP curves, stat scaling | Planned |
| **Phase 5** | Modernization -- Dependency upgrades, CMake build, CI/CD, Docker | Planned |

See [Project Status](project-status.md) for detailed breakdown of each phase.


## Evidence Standards

All RE documentation uses a three-tier confidence system to distinguish verified facts from educated guesses.

| Level | Label | Meaning |
|-------|-------|---------|
| **HIGH** | Confirmed | Directly verified in Ghidra disassembly, confirmed by BigWorld reference source, or tested in live emulator |
| **MEDIUM** | Probable | Strong indirect evidence: consistent string references, RTTI matches, behavioral correlation with reference source |
| **LOW** | Speculative | Inferred from naming patterns, partial decompilation, or analogy with similar systems; needs further verification |

When documenting findings, always state the confidence level and cite the evidence basis (address, function name, reference source file, or test observation). See [Evidence Standards Guide](guides/evidence-standards.md) for full details.


## Contributing

When adding or updating documentation:

1. Place documents in the correct subdirectory per the map above.
2. Use relative links for all cross-references within `docs/`.
3. Tag every claim with a confidence level (HIGH / MEDIUM / LOW).
4. Include Ghidra addresses or source references where applicable.
5. Update this README when adding new documents.
