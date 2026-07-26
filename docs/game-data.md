---
title: "Game Data"
type: reference
audience: engineers
last_updated: 2026-07-25
---

# Game Data

What content data exists for the Stargate Worlds emulator, where it comes from, and what's missing.

## The Good News: We Have Almost Everything

The project contains a complete database dump with **112,626 rows of game data across 65 tables**. This includes every ability, item, mission, effect, dialog, NPC template, stargate address, and more that was in the game. The original feasibility assessment overstated the "missing content data" gap — the data is here.

## Where the Data Lives

### Database (`db/resources/`)

This is the **authoritative source** for all game content — a per-system tree of
PostgreSQL DDL and seed scripts (18 game-system directories plus the shared
`_schema.sql` / `_indexes.sql` / `_foreign_keys.sql` files), loaded alongside
`db/database.sql` and `db/sgw/`. It contains:

| Content Type | Count | Examples |
|-------------|-------|---------|
| Localized text strings | 29,126 | Item names, dialog text, UI labels, error messages |
| Dialog system | 29,007 | Dialog trees, screens, buttons, NPC speakers |
| Visual assets | 13,305 | Mesh references, body components, character visuals |
| Missions | 12,917 | Missions, steps, objectives, tasks, rewards |
| Items | 8,826 | Weapons, armor, consumables, crafting materials |
| Effects & Events | 7,881 | Combat effects, visual sequences, Kismet events |
| Crafting | 3,139 | Blueprints, components, disciplines, sciences |
| Abilities | 2,070 | Combat abilities, ability sets, archetype trees |
| Character creation | 1,962 | Character definitions, customization options |
| Spawning | 681 | Entity templates, spawn sets, spawn points |
| World/Zones | 284 | Zone definitions, stargates, respawn points |
| Economy | 87 | Loot tables, item lists, store prices |

### PAK Files (`data/cache/`)

The PAK files are **standard ZIP archives** containing XML. Each entry is a "cooked" version of database content formatted for the client to consume. You can open them with any ZIP tool and read the XML inside.

The PAK files are generated from the database by the server's Python scripts — they're output, not source. **If you need to change game data, change it in the database, and the PAK files will be regenerated automatically.**

### Entity Definitions (`entities/`)

XML files that define the **structure** of game objects — what properties a player has, what an NPC can do, what interfaces they support. There are 18 entity types (`entities/defs/*.def`) and 18 interfaces (`entities/defs/interfaces/*.def`).

Key entities:
- **SGWPlayer** — The main player entity (58KB of property definitions)
- **SGWMob** — NPCs and monsters (31KB)
- **SGWSpawnSet** — Spawn point management
- **Account** — Account-level data

### Python Scripts (`deprecated/python/`)

164 Python files containing the **game logic** — how combat works, how missions progress, how crafting resolves. This is where the "rules of the game" live, separate from the data. These are the legacy reference implementation; active development is the Rust port under `crates/`.

### Visual Script Files (`deprecated/data-scripts/scripts/`)

XML-based node graphs for effects and missions, created with the **Atrea Script Editor** (part of the ServerEd Qt tool). These are the **source files** for mission and effect scripts.

The compilation pipeline works like this:

```
.script files                                  <-- SOURCE: visual node graphs (XML)
  (deprecated/data-scripts/scripts/)
    |
    |  compiled by scriptcompiler.cpp
    |  (deprecated/cpp-tools/ServerEd/)
    v
.py files                                      <-- OUTPUT: auto-generated Python
  (deprecated/python/cell/missions/ etc.)
```

Key details:
- The `.script` XML files are the authoritative source — they represent visual node graphs from the Atrea Script Editor
- The `.py` files in `deprecated/python/cell/missions/` etc. are **auto-generated output**, not hand-written code
- The compiler is implemented in `deprecated/cpp-tools/ServerEd/scriptcompiler.cpp` (part of the Qt ServerEd tool; the built `tools/ServerEd/` tree keeps only the compiled artifacts)
- Python files are **not** regenerated on server start — they are static compiled output checked into the repo
- To modify a mission script, you should ideally edit the `.script` source and recompile, though direct `.py` edits work too

If you have the ServerEd tool working, you can modify scripts visually instead of writing Python code. A potential future improvement is porting the compiler to Python so the server can recompile scripts on startup.

## The Data Pipeline

Data flows from the database to the client through an automated pipeline:

```
PostgreSQL Database
    |  (Python queries the database)
    v
Python Resource Classes
    |  (Each class has a toXml() method)
    v
Cooked XML
    |  (Packed into ZIP archives)
    v
PAK Files (data/cache/)
    |  (Served to client on demand)
    v
Game Client
```

This pipeline is **confirmed working in our server**. When a client connects, the server generates cooked data from the database and sends it. The 22 resource categories handled include abilities, items, effects, missions, dialogs, stargates, blueprints, world info, text strings, and more.

## Data & Code Status

"Data" = exists in the database or entity definitions. "Code" = Python game logic exists. "Tested" = confirmed working in our emulator with a real client.

| System | Data | Code | Server Status |
|--------|------|------|---------------|
| Combat formulas | In Python code | QR system fully coded | Tested in-game |
| Spawning | 153 NPC templates | Spawn management coded | Tested in-game |
| Missions | 1,040 defined | 16 mission scripts | Tested (Castle Cellblock) |
| Dialogs | 5,406 trees | Full interaction system | Tested in-game |
| Items | 6,060 defined | Inventory system coded | Tested in-game |
| Abilities | 1,887 defined | Full pipeline coded | Not yet tested |
| Crafting | 499 blueprints | Full crafting logic | Not yet tested |
| Chat | 11 channels | Full implementation | Not yet tested |
| Trading | Defined | Full transaction logic | Not yet tested |
| Stargates | 29 gates | Dialing coded, travel stub | Not yet tested |
| Effects | 3,217 defined | Script system + Python | Not yet tested |
| World/Zones | 91 worlds | 11 space scripts | 1 zone tested |
| Character creation | 23 definitions | Data loading only | Barely functional |
| Minigames | Defined | 1 of 8 complete (Livewire) | Not yet tested |

## What's Missing or Incomplete

### Sparse Data (System Works, Needs More Content)

- **Loot tables** — The loot system code is complete, but only 3 items exist in 2 loot tables. This needs to be populated with appropriate drops for each monster type and level range.

### Missing Data (Not Found Anywhere)

- **Level/XP curves** — How much XP is needed per level? Not defined in any data file. Needs to be designed or discovered from other sources.
- **Stat scaling per level** — How do player stats grow as they level up? Base values per level are not explicitly defined.

### Logic Not Yet Implemented

- **Mission reward dispatch** — no chain can award XP. `Action::GrantXP` exists in the content engine's action enum but has no loader arm *and* no executor arm, and all 1,040 mission rows in [db/resources/Missions/Seed/missions.sql](../db/resources/Missions/Seed/missions.sql) carry `reward_naq = 0, reward_xp = 0`. Both halves need doing before missions can pay out. See [content/content-engine.md §3](content/content-engine.md).
- **Chain-driven effects, damage, and movement** — the `apply_effect`, `remove_effect`, `qr_combat_damage`, `move_entity`, `launch_ability`, and `fail_objective` content actions load from seed but have no executor arm, so the 13 seeded rows using them silently no-op.
- **7 of 8 minigames** — only Livewire is implemented ([crates/services/src/minigame/games/](../crates/services/src/minigame/games/)); Hack, Activate, Analyze, Bypass, and the two Converse variants route to a shared placeholder, and Alignment / GoauldCrystals are commented out entirely.

### Recently Implemented (previously listed as missing)

- **AI behavior** — the `AiState` machine now covers Spawning, Idle, Investigating, Fighting, Leashing, Dead, Despawning, Follow, Patrol, Wander, Submit, and Error ([crates/entity/src/cell_entity/mod.rs:163-176](../crates/entity/src/cell_entity/mod.rs#L163-L176)). Content chains can drive Investigating, Follow, and the terminal states via the `set_npc_poi` / `set_follow_target` / `set_npc_ai_state` actions.
- **Organizations/Guilds** — server-side handlers exist at [crates/services/src/cell/cell_methods/organization.rs](../crates/services/src/cell/cell_methods/organization.rs) and [client_methods/organization.rs](../crates/services/src/cell/client_methods/organization.rs), with shared logic in [crates/game/src/social/guilds.rs](../crates/game/src/social/guilds.rs).
- **Mail system** — handlers exist at [crates/services/src/base/world_entry/methods/mail/](../crates/services/src/base/world_entry/methods/mail/), [cell/cell_methods/mail.rs](../crates/services/src/cell/cell_methods/mail.rs), and [cell/client_methods/mail.rs](../crates/services/src/cell/client_methods/mail.rs).
- **Auction house / Black Market** — **not yet on `main`.** Implemented under `crates/services/src/base/black_market/` (create, bid, cancel, search, expiry sweep, plus tests) and reachable in-world through the content engine's `open_black_market` action, but this is phase-1 work on the unmerged `feat/571-black-market-phase1` branch. Treat as in-flight, not shipped.
- **Cover nodes** — the binary format has been parsed. [db/resources/AI/Seed/cover_nodes.sql](../db/resources/AI/Seed/cover_nodes.sql) and [cover_sets.sql](../db/resources/AI/Seed/cover_sets.sql) carry the extracted positions, and the content engine exposes `player_entered_cover` / `player_left_cover` / `player_in_cover_duration` / `npc_flanked` triggers on top of them.

## What We Don't Need

The client installation has ~5GB of `.umap` and `.upk` files (maps, textures, models, animations). **None of this is needed for the server.** These are purely client-side rendering assets. The server only needs to know about zone boundaries (defined in `spaces.xml`) and spawn/cover point positions.

## Modifying Game Data

To change game content:

1. **Edit the seed** — Change the relevant `Seed/*.sql` file under [db/resources/](../db/resources/). The seed files are the single source of truth; never hand-write a `db/scripts/*.sql` migration
2. **Restart the server** — The Python resource classes will reload from the database
3. **Clients get updated data** — The cooked XML is regenerated and served on next connection

For mission and effect scripts, the proper workflow is to edit the `.script` XML node graphs in `data/scripts/` using the Atrea Script Editor (in ServerEd) and recompile. The auto-generated Python in `deprecated/python/cell/` will be updated. Direct edits to the `.py` files also work but will be overwritten if the `.script` source is recompiled. The Python files are static — the server does not recompile them on startup.
