# Game Data

What content data exists for the Stargate Worlds emulator, where it comes from, and what's missing.

## The Good News: We Have Almost Everything

The project contains a complete database dump with **112,626 rows of game data across 65 tables**. This includes every ability, item, mission, effect, dialog, NPC template, stargate address, and more that was in the game. The original feasibility assessment overstated the "missing content data" gap — the data is here.

## Where the Data Lives

### Database (`db/resources.sql`)

This is the **authoritative source** for all game content. It's a full PostgreSQL dump containing:

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

XML files that define the **structure** of game objects — what properties a player has, what an NPC can do, what interfaces they support. There are 17 entity types and 20 interfaces.

Key entities:
- **SGWPlayer** — The main player entity (58KB of property definitions)
- **SGWMob** — NPCs and monsters (31KB)
- **SGWSpawnSet** — Spawn point management
- **Account** — Account-level data

### Python Scripts (`python/`)

164 Python files containing the **game logic** — how combat works, how missions progress, how crafting resolves. This is where the "rules of the game" live, separate from the data.

### Visual Script Files (`data/scripts/`)

XML-based node graphs for effects and missions, created with the **Atrea Script Editor** (part of the ServerEd Qt tool). These are the **source files** for mission and effect scripts.

The compilation pipeline works like this:

```
.script files (data/scripts/)          <-- SOURCE: visual node graphs (XML)
    |
    |  compiled by scriptcompiler.cpp (tools/ServerEd/)
    v
.py files (python/cell/missions/ etc.) <-- OUTPUT: auto-generated Python
```

Key details:
- The `.script` XML files are the authoritative source — they represent visual node graphs from the Atrea Script Editor
- The `.py` files in `python/cell/missions/` etc. are **auto-generated output**, not hand-written code
- The compiler is implemented in `tools/ServerEd/scriptcompiler.cpp` (part of the Qt ServerEd tool)
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
| Missions | 1,041 defined | 17 mission scripts | Tested (Castle Cellblock) |
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
| Minigames | Defined | 3 of 9 complete | Not yet tested |

## What's Missing or Incomplete

### Sparse Data (System Works, Needs More Content)

- **Loot tables** — The loot system code is complete, but only 3 items exist in 2 loot tables. This needs to be populated with appropriate drops for each monster type and level range.

### Missing Data (Not Found Anywhere)

- **Level/XP curves** — How much XP is needed per level? Not defined in any data file. Needs to be designed or discovered from other sources.
- **Stat scaling per level** — How do player stats grow as they level up? Base values per level are not explicitly defined.

### Logic Not Yet Implemented

- **AI behavior** — NPCs can fight (threat-based target selection) but can't patrol, wander, leash, flee, or investigate. They just stand in place until aggro'd.
- **Organizations/Guilds** — 23KB of entity property definitions exist, but no server-side code to actually run the guild system.
- **Mail system** — Database table exists but no handlers.
- **Auction house** — Entity definitions only.
- **7 of 10 minigames** — Have placeholder stubs.

### Binary Data Needing Parsing

- **Cover nodes** — 1,332 cover positions across maps, stored in binary format (not XML). Needs a parser to extract the position data.

## What We Don't Need

The client installation has ~5GB of `.umap` and `.upk` files (maps, textures, models, animations). **None of this is needed for the server.** These are purely client-side rendering assets. The server only needs to know about zone boundaries (defined in `spaces.xml`) and spawn/cover point positions.

## Modifying Game Data

To change game content:

1. **Edit the database** — Modify rows in `resources.sql` or update the running PostgreSQL database directly
2. **Restart the server** — The Python resource classes will reload from the database
3. **Clients get updated data** — The cooked XML is regenerated and served on next connection

For mission and effect scripts, the proper workflow is to edit the `.script` XML node graphs in `data/scripts/` using the Atrea Script Editor (in ServerEd) and recompile. The auto-generated Python in `python/cell/` will be updated. Direct edits to the `.py` files also work but will be overwritten if the `.script` source is recompiled. The Python files are static — the server does not recompile them on startup.
