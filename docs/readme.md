# Cimmeria Reverse Engineering Documentation

Server emulator for **Stargate Worlds** (SGW), the cancelled MMO built on BigWorld Technology and the Cheyenne Mountain Entertainment (CME) game framework. The reverse engineering effort targets `sgw.exe` -- the game client binary -- loaded in Ghidra, with the goal of understanding the client-server protocol and game mechanics well enough to build a faithful server replacement.

The emulator is **playable today**: players can log in, enter the world, interact with NPCs, run quests, and engage in combat. These docs capture everything learned during the RE process and track what remains.


## Quick Stats

| Metric | Count |
|--------|-------|
| Event_NetOut messages (client to server) | 253 |
| Event_NetIn messages (server to client) | 167 |
| Total cataloged network messages | 420 |
| Entity types | 18 |
| Interfaces | 18 |
| Unnamed functions in sgw.exe | ~85,000 |
| Python game logic scripts | 164 |
| Database rows (game data) | 112,626 |
| Abilities / Items / Missions / Effects | 1,887 / 6,060 / 1,041 / 3,217 |


## Document Map

### Top-Level Documents

| Document | Description |
|----------|-------------|
| [How SGW Works](how-sgw-works.md) | Technology overview -- BigWorld, CME, and how the pieces fit together |
| [Client Tools](client-tools.md) | Launcher, editor mode, debug tools available in the client |
| [Building the Server](building.md) | How to build and run the Cimmeria server emulator |
| [Game Systems](game-systems.md) | Survey of every game feature: combat, abilities, stargates, missions, crafting |
| [Game Data](game-data.md) | What game content exists (items, abilities, missions) and what is missing |
| [Commands Reference](commands.md) | Player commands, chat, GM tools, and debug commands |
| [Connection Flow](connection-flow.md) | End-to-end login and world entry sequence |
| [Network Messages](network-messages.md) | High-level catalog of client-server messages |
| [Project Status](project-status.md) | What works, what is left, and the roadmap |

---

### `protocol/` -- Wire Formats and Messaging

Network protocol internals: packet structures, Mercury reliable messaging, entity property synchronization, and specific message flows.

| Document | Description | Status |
|----------|-------------|--------|
| [message-catalog.md](protocol/message-catalog.md) | **HUB** -- Complete catalog of all 420 network messages with IDs, directions, and payload structures | Planned |
| [mercury-wire-format.md](protocol/mercury-wire-format.md) | Mercury packet header, reliable sequencing, AES-256 encryption, ACK/NACK handling | Planned |
| [entity-property-sync.md](protocol/entity-property-sync.md) | How entity properties are serialized, delta-compressed, and synchronized client/server | Planned |
| [login-handshake.md](protocol/login-handshake.md) | HTTP auth request, session key exchange, shard selection, Mercury connection setup | Planned |
| [position-updates.md](protocol/position-updates.md) | Movement packets, coordinate encoding, velocity/orientation fields, server reconciliation | Planned |

See also: [technical/mercury-protocol.md](technical/mercury-protocol.md), [technical/mercury-audit.md](technical/mercury-audit.md), [technical/login-auth-flow.md](technical/login-auth-flow.md), [technical/post-auth-sequence.md](technical/post-auth-sequence.md), [technical/network-messages.md](technical/network-messages.md)

---

### `gameplay/` -- Game System Documentation

Per-system breakdowns of game mechanics, derived from RE analysis, entity definitions, and Python scripts.

| Document | Description | Status |
|----------|-------------|--------|
| [README.md](gameplay/README.md) | **HUB** -- System dashboard with status and cross-references for every gameplay system | Planned |
| [combat-system.md](gameplay/combat-system.md) | Cover mechanics, threat/aggro, damage resolution, death/respawn | Planned |
| [ability-system.md](gameplay/ability-system.md) | Ability activation, cooldowns, targeting, channeling, combos | Planned |
| [effect-system.md](gameplay/effect-system.md) | Buffs, debuffs, DoTs, HoTs, effect stacking and priority | Planned |
| [stat-system.md](gameplay/stat-system.md) | Base stats, derived stats, level scaling, equipment modifiers | Planned |
| [inventory-system.md](gameplay/inventory-system.md) | Item slots, stacking, equipment, bag management | Planned |
| [crafting-system.md](gameplay/crafting-system.md) | Blueprints, material requirements, crafting stations, 499 recipes | Planned |
| [mission-system.md](gameplay/mission-system.md) | Quest objectives, step advancement, rewards, mission scripts | Planned |
| [gate-travel.md](gameplay/gate-travel.md) | Stargate dialing, 29 defined gates, zone transitions | Planned |
| [minigame-system.md](gameplay/minigame-system.md) | 10 minigames (3 implemented, 7 remaining), lockpicking, hacking, etc. | Planned |
| [organization-system.md](gameplay/organization-system.md) | Guilds/organizations, ranks, permissions, roster management | Planned |
| [group-system.md](gameplay/group-system.md) | Party formation, loot rules, group chat | Planned |
| [chat-system.md](gameplay/chat-system.md) | Chat channels, whispers, emotes, chat commands | Planned |
| [mail-system.md](gameplay/mail-system.md) | In-game mail, attachments, delivery | Planned |
| [trade-system.md](gameplay/trade-system.md) | Player-to-player trade, trade windows, confirmation | Planned |
| [black-market.md](gameplay/black-market.md) | Auction house, listings, bidding, buyout | Planned |
| [pet-system.md](gameplay/pet-system.md) | Pet summoning, commands, abilities | Planned |
| [duel-system.md](gameplay/duel-system.md) | PvP duel requests, rules, resolution | Planned |

See also: [game-systems.md](game-systems.md), [technical/game-systems.md](technical/game-systems.md), [technical/game-data-analysis.md](technical/game-data-analysis.md)

---

### `engine/` -- BigWorld and CME Internals

How the underlying BigWorld engine and CME game framework operate inside sgw.exe.

| Document | Description | Status |
|----------|-------------|--------|
| [entity-type-catalog.md](engine/entity-type-catalog.md) | **HUB** -- All 18 entity types with properties, methods, and interface bindings | Planned |
| [bigworld-architecture.md](engine/bigworld-architecture.md) | BigWorld 1.9.1 client internals: entity manager, space manager, connection layer | Planned |
| [cme-framework.md](engine/cme-framework.md) | CME game layer on top of BigWorld: UI framework, game state, HUD systems | Planned |
| [cooked-data-pipeline.md](engine/cooked-data-pipeline.md) | .pak file format, XML cooking, resource loading from cooked data | Planned |
| [watcher-system.md](engine/watcher-system.md) | BigWorld watcher/debug variable system, runtime inspection | Planned |
| [space-management.md](engine/space-management.md) | Space/zone loading, cell partitioning, entity AoI | Planned |

See also: [technical/bigworld-version-analysis.md](technical/bigworld-version-analysis.md), [technical/sgw-binary-overview.md](technical/sgw-binary-overview.md)

---

### `architecture/` -- Cimmeria Server Architecture

How the Cimmeria emulator itself is structured.

| Document | Description | Status |
|----------|-------------|--------|
| [service-architecture.md](architecture/service-architecture.md) | Auth, Base, Cell service topology, Mercury inter-service messaging, config | Planned |

See also: [building.md](building.md), [connection-flow.md](connection-flow.md)

---

### `analysis/` -- Investigation Logs

Working notes and cross-reference indexes from ongoing RE sessions.

| Document | Description | Status |
|----------|-------------|--------|
| [event-net-mapping.md](analysis/event-net-mapping.md) | Mapping between Event_NetIn/NetOut IDs and server handler functions | Planned |
| [bigworld-reference-index.md](analysis/bigworld-reference-index.md) | Cross-reference: BigWorld 1.9.1/2.0.1 source symbols to sgw.exe addresses | Planned |

---

### `guides/` -- How-To Guides

Practical guidance for contributors working on the RE effort or the emulator.

| Document | Description | Status |
|----------|-------------|--------|
| [evidence-standards.md](guides/evidence-standards.md) | Confidence levels, citation format, how to document findings | Planned |
| [reading-decompiled-code.md](guides/reading-decompiled-code.md) | Tips for reading Ghidra decompiler output, common patterns, pitfalls | Planned |
| [entity-def-guide.md](guides/entity-def-guide.md) | How to read and write entity XML definitions and connect them to Python scripts | Planned |

---

### `reverse-engineering/` -- Ghidra Work

Annotation scripts, function naming progress, and per-system RE findings.

| Document | Description | Status |
|----------|-------------|--------|
| [function-naming-progress.md](reverse-engineering/function-naming-progress.md) | Tracking named vs unnamed functions, naming conventions, coverage metrics | Planned |
| [address-map.md](reverse-engineering/address-map.md) | Key address table: vtables, global objects, critical functions in sgw.exe | Planned |

#### `annotation-scripts/` -- Ghidra Jython Scripts

| Script | Purpose | Status |
|--------|---------|--------|
| `annotate_event_net_handlers.py` | Label Event_NetIn/NetOut handler functions from vtable analysis | Planned |
| `annotate_entity_types.py` | Label entity type factory functions and type ID constants | Planned |
| `annotate_mercury_messages.py` | Label Mercury message handler dispatch functions | Planned |
| `annotate_rtti.py` | Extract and apply RTTI class names from .rdata | Planned |
| `annotate_python_methods.py` | Label C++ functions exposed to Python via Boost.Python | Planned |
| `annotate_string_refs.py` | Propagate meaningful names from string literals to referencing functions | Planned |
| `annotate_vtables.py` | Identify and label vtable structures and their owning classes | Planned |
| `annotate_property_sync.py` | Label entity property getter/setter/sync functions | Planned |
| `annotate_ui_events.py` | Label UI event handler registrations and callbacks | Planned |
| `export_function_list.py` | Export current function names and addresses for external analysis | Planned |

#### `findings/` -- Per-System RE Findings

Raw analysis notes organized by system, with memory addresses and decompiler excerpts. Documents are created here as individual systems are investigated in Ghidra.

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
| Database schema | `db/sgw.sql` | PostgreSQL schema for all persistent game data |
| Resource data | `db/resources.sql` | 112,626 rows of abilities, items, missions, effects, etc. |
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
