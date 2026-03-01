# Game Data Analysis

Comprehensive analysis of all game content data available for the Cimmeria server emulator: PAK files, entity definitions, Python scripts, database schema, and client assets.

## Summary

The project has virtually all game content data needed for a server emulator. The `resources.sql` database dump contains 112,626 rows across 65 tables. The PAK files are standard ZIP archives with XML -- no proprietary format to reverse-engineer. A complete data pipeline (DB -> Python -> Cooked XML -> Client) is already implemented and functional.

## 1. Client SourceCache (PAK Files)

**Location**: Client install `SGWGame/SourceCache.en-US/`
**Date**: June 30, 2009 (all files)
**Total size**: ~22 MB across 20 pak files
**Format**: Standard ZIP archives (magic `PK\x03\x04`, DEFLATE compression)

Each ZIP entry is named by numeric ID (e.g., `_1000`) and contains well-formed XML with SOAP-style namespace declarations and a `COOKED_*` root element.

### File Inventory

| Pak File | Size | Entries | Content |
|----------|------|---------|---------|
| CookedDataAbilities.pak | 1,052,598 | 1,887 | Ability definitions |
| CookedDataItems.pak | 3,548,002 | 6,060 | Item definitions |
| CookedDataEffects.pak | 1,509,266 | 3,217 | Effect definitions |
| CookedDataMissions.pak | 749,028 | 1,041 | Mission definitions |
| CookedDataDialogs.pak | 2,702,739 | 5,406 | Dialog trees |
| CookedInteractionSet.pak | 1,627,165 | 4,662 | NPC interaction topic maps |
| CookedInteractions.pak | 15,811 | 41 | Interaction type definitions |
| CookedBlueprints.pak | 209,713 | 499 | Crafting blueprints |
| CookedCharCreation.pak | 10,615 | 2 | Character creation definitions |
| CookedDataContainers.pak | 6,036 | 21 | Inventory container types |
| CookedDataStargates.pak | 11,670 | 29 | Stargate addresses and positions |
| CookedDisciplines.pak | 33,812 | 79 | Crafting disciplines |
| CookedDataKismetSeqEvent.pak | 679,258 | 1,773 | Kismet event sequences |
| CookedDataKismetSetEvent.pak | 287,554 | 661 | Kismet event sets |
| CookedParadigm.pak | 1,566 | 6 | Racial paradigms |
| CookedSciences.pak | 1,314 | 5 | Applied sciences |
| CookedWorldInfo.pak | 29,856 | 92 | World/zone definitions |
| ErrorStrings.pak | 75,407 | 217 | Error message strings |
| SpecialWords.pak | 1,555 | 2 | Profanity filter |
| TextStrings.pak | 10,095,453 | 29,127 | All localized text strings |

### Repo Cache (`data/cache/`)

The repo contains the same pak files (dated Feb 4, 2014) with identical entry counts. Two minor differences:
1. Repo paks include a `MetaData` entry (4-byte uint32 version number)
2. Slightly different XML attribute ordering

The repo already contains the complete game content database.

## 2. PAK Format Details

### Not BigWorld PackedSection

The BigWorld engine has a `PackedSection` binary format (`src/lib/resmgr/packed_section.hpp`) using binary trees with string tables. **SGW does NOT use this format for game data.** SGW paks use standard ZIP + XML instead.

### ZIP Structure

```
PK\x03\x04  (local file header)
  filename: "_<numeric_id>" (e.g., "_1000")
  compression: DEFLATE (type 8)
  content: UTF-8 XML with COOKED_* root element
```

Python's `zipfile.ZipFile()` reads them natively. No custom tooling needed.

## 3. Database: The Authoritative Data Source

**Location**: `db/resources.sql`
**Size**: 126,378 lines
**Total rows**: 112,626 across 65 tables

### Data Volume by Category

| Category | Key Tables | Rows |
|----------|-----------|------|
| Localization | texts | 29,126 |
| Dialog system | dialog_screens, dialog_screen_buttons, dialogs, dialog_sets, dialog_set_maps, speakers | 29,007 |
| Items | items, items_event_sets | 8,826 |
| Visual assets | static_meshes, skeletal_meshes, body_components, body_component_visuals, body_sets | 13,305 |
| Missions | missions, mission_steps, mission_objectives, mission_tasks, mission_rewards, mission_reward_groups | 12,917 |
| Effects/Events | effects, effect_nvps, event_sets, event_sets_sequences, sequences, sequences_nvp | 7,881 |
| Abilities | abilities, ability_moniker_groups, ability_sets, ability_set_abilities, archetypes, archetype_ability_tree | 2,070 |
| Crafting | blueprints, blueprints_components, disciplines, racial_paradigm, applied_science | 3,139 |
| Character creation | char_creation, char_creation_choices, char_creation_visgroups, char_creation_abilities | 1,962 |
| Spawning | entity_templates (153), spawnlist, spawn_sets, point_sets, point_set_points, spawn_points | 681 |
| World | worlds (91), stargates (28), respawners, generic_regions, ring_transport_regions | 284 |
| Economy | loot_tables, loot, item_lists, item_list_items, item_list_prices, containers, monikers | 87 |
| System | resource_types, resource_versions, interactions, special_words, error_texts | 783 |

### The PAK files are derived from this data

The Python `common/defs/` classes load from these tables and generate the same cooked XML that appears in the pak files. No reverse extraction is needed.

## 4. Data Pipeline (Already Working)

```
PostgreSQL (resources.sql)
    |
    v
Python Resource Classes (common/defs/*.py)
    - Ability.py, Item.py, Effect.py, Mission.py, etc.
    - Each has loadAll() and toXml() methods
    |
    v
Cooked XML (COOKED_* root elements)
    |
    v
PAK Files (ZIP archives via DefMgr.makePaks())
    |
    v
C++ ResourceManager -> serves to client on demand
```

22 resource categories handled:
Abilities, Items, Effects, Missions, Dialogs, InteractionSets, Interactions, Blueprints, CharCreation, Containers, Stargates, Disciplines, Paradigms, Sciences, WorldInfo, ErrorStrings, TextStrings, SpecialWords, KismetSequences, KismetSetEvents, BodySets, BodyComponentVisuals

## 5. Entity Definitions (`entities/`)

### Entity Types (17 total, from `entities.xml`)

| Entity | Def Size | Purpose |
|--------|----------|---------|
| `SGWPlayer` | 58 KB | Main player entity |
| `SGWGmPlayer` | 27 KB | GM player with extra commands |
| `SGWMob` | 31 KB | NPC/monster entity |
| `Account` | 4.5 KB | Account management entity |
| `SGWPet` | 4.5 KB | Pet/companion entity |
| `SGWSpawnableEntity` | 9.6 KB | Generic spawnable entity |
| `SGWSpawnRegion` | 7.5 KB | Spawn region manager |
| `SGWSpawnSet` | 5.5 KB | Spawn set with population tracking |
| `SGWSpaceCreator` | 5.9 KB | Space/zone initializer |
| `SGWCoverSet` | 2.5 KB | Cover system data |
| `SGWBlackMarket` | 2.0 KB | Auction house entity |
| `SGWBeing` | 0.7 KB | Base being (shared properties) |
| `SGWDuelMarker` | 0.8 KB | Duel area marker |
| Others | Various | PlayerGroupAuthority, PlayerRespawner, Entity, Escrow, ChannelManager |

### Interfaces (20 total, in `entities/defs/interfaces/`)

| Interface | Size | Purpose |
|-----------|------|---------|
| `MinigamePlayer` | 26.2 KB | Full minigame system |
| `OrganizationMember` | 23.2 KB | Guild/organization system |
| `SGWAbilityManager` | 12.6 KB | Ability launching and cooldowns |
| `SGWCombatant` | 12.2 KB | Combat stats (health, focus, energy) |
| `SGWBeing` | 11.5 KB | Base being properties |
| `SGWInventoryManager` | 9.7 KB | Full inventory system |
| `Communicator` | 8.5 KB | Chat channels and messaging |
| `Missionary` | 7.6 KB | Mission tracking |
| `SGWBlackMarketManager` | 4.5 KB | Auction house |
| `SGWMailManager` | 4.3 KB | In-game mail |
| `ContactListManager` | 3.6 KB | Friends/contacts |
| `DistributionGroupMember` | 3.6 KB | Group/party system |
| `Lootable` | 3.5 KB | Loot generation |
| `GateTravel` | 3.4 KB | Stargate travel |
| `GroupAuthority` | 2.7 KB | Authority management |
| `ClientCache` | 1.4 KB | Client-side caching |
| Others | Various | EventParticipant, SGWPoller |

### Supporting Files

| File | Size | Purpose |
|------|------|---------|
| `enumerations.xml` | 142 KB | All game system enumerations |
| `alias.xml` | 33 KB | Type aliases for entity properties |
| `spaces.xml` | 2.5 KB | 28 world/zone definitions with boundaries |
| `cell_spaces.xml` | 0.8 KB | Cell spatial partitioning config |
| `editor/Nodes.xml` | 136 KB | Visual script editor node definitions |

## 6. Python Game Logic (`python/`)

164 files across 4 major directories implementing the game systems.

### Implemented Game Systems

#### Combat System -- CONFIRMED WORKING IN-GAME

- QR (Quality Rating) system with beta distribution
- 5 damage types: Untyped, Energy, Hazmat, Physical, Psionic
- Each type has own armor factor and absorption stats
- Stat-based resistance: Health (fortitude), Focus (intelligence), Energy (engagement)
- Config-driven thresholds: Miss < 0.07, Glancing < 0.20, Critical > 0.80, DoubleCrit > 0.93
- Formula: `base_damage * QR * multiplier * stat_resistance * armor_factor * mitigation * absorption`

#### Ability System -- FULLY IMPLEMENTED

- Full pipeline: cooldowns, warmup, channeling
- Target methods: Single, AoE Radius, AoE Cone, Group
- Position requirements: Front, Flank, Rear, Above, Below
- Weapon requirements and ammo consumption
- Effect sequencing and Kismet visual events

#### Effect/Script System -- PARTIALLY IMPLEMENTED

- XML node-graph visual scripting (`.script` files in `data/scripts/`)
- Auto-generated Python from script graphs (compiled output in `python/cell/missions/` etc.)
- Compiler: `tools/ServerEd/scriptcompiler.cpp` (part of the Qt ServerEd tool)
- 4 effect scripts + Debug.script (60KB)
- `TestEffect.py` (22.6KB) as comprehensive Python handler

**Compilation pipeline:** The `.script` XML files are the **source** (visual node graphs from the Atrea Script Editor). The `.py` files are **compiled output**. The Python files are static -- they are not regenerated on server start. To modify a script properly, edit the `.script` source and recompile via ServerEd.

#### Mission System -- CONFIRMED WORKING IN-GAME

- Multi-step with objectives and tasks
- Accept/complete/fail/abandon lifecycle
- Reward groups with item rewards
- 17 mission scripts for 4 zones
- FindAmbernol quest in Castle Cellblock runs end-to-end: region enter, interact, kill, and use-item objectives all advance correctly
- **Known issue:** Some entity templates have `interaction_type=0` and need scripts to set `INT_MissionWorldObject` (bit 30) dynamically via `setInteractionType()`

#### Inventory System -- CONFIRMED WORKING IN-GAME

- Multiple container types (Main, Mission, Crafting, Equipment)
- Stacking, charges, durability
- Equipment with visual components
- Ammo and clip management
- Items successfully given to players during quest progression

#### Crafting System -- IMPLEMENTED

- 499 blueprints, 79 disciplines, 5 applied sciences, 6 racial paradigms
- Full crafting logic in `Crafter.py` (20.7KB)

#### NPC/AI System -- BASIC

- Threat-based target selection
- FSM: Spawning -> Idle -> Fighting -> Dead
- Ability selection, cooldown checking, ammo/reload
- Loot generation on death
- **Missing**: Pathfinding, leashing, patrol, wander, investigate

#### Spawn System -- CONFIRMED WORKING IN-GAME

- SpawnSet with weighted selection
- Population tracking and cooldowns
- 153 entity templates from database
- NPCs and world objects spawn visibly and are interactable in Castle Cellblock

#### Dialog/Interaction -- CONFIRMED WORKING IN-GAME

- 5,406 dialog trees, 4,662 interaction set maps
- Dynamic interaction based on mission state
- Vendor, trainer, loot, DHD handlers
- Dialog trees display correctly during NPC interactions and mission sequences
- Right-clicking world objects and NPCs triggers correct interaction scripts

#### Minigames -- PARTIALLY IMPLEMENTED

- 10 types total, 3 fully coded (Alignment, GoauldCrystals, Livewire)
- 7 with placeholder stubs

#### Stargate Travel -- IMPLEMENTED

- 29 stargates with 7-symbol addresses
- Position/rotation data, world-to-world travel

#### Character Creation -- IMPLEMENTED

- 23 definitions across archetypes and genders
- Visual customization, starting abilities

## 7. What's Missing

### Data Gaps

| Gap | Status | Notes |
|-----|--------|-------|
| Loot table data | Schema complete, nearly empty | Only 3 items in 2 tables |
| Level/XP curves | Not found anywhere | Must be designed/recreated |
| Stat scaling per level | Not explicitly defined | Baseline stats per level missing |
| Cover node data | Binary format in covernodes_*.pak | 1,332 entries, needs RE |

### Logic Gaps

| Gap | Status | Notes |
|-----|--------|-------|
| AI behavior (patrol, leash, flee) | Not coded | Basic threat+FSM only |
| Organization/Guild logic | Entity def only (23KB) | No Python implementation |
| Mail system logic | DB table exists | No Python handlers |
| Auction house logic | Entity def only | No implementation |
| 7 minigame types | Stubs only | 3 of 10 fully coded |

### Not Needed for Server

| Data | Location | Why Not Needed |
|------|----------|---------------|
| UE3 map geometry | CookedPC/ (5 GB of .umap/.upk) | Client-side rendering only |
| Flash/CEGUI UI assets | CookedPC/UI/ (22 MB) | Client-side UI only |
| Character meshes/textures | CookedPC/Packages/Character/ | Client-side rendering |

## 8. Automation Assessment

### No New Tools Needed

The existing data pipeline handles everything:
- `resources.sql` is the authoritative source for all game data
- Python `common/defs/` classes already map DB rows to client XML
- `DefMgr.makePaks()` generates ZIP pak files
- C++ `ResourceManager` serves cooked data to clients at runtime

### Reverse Pipeline (Client -> DB) Not Needed

Entry counts are identical between client paks and repo paks. The database dump already contains everything the client has.

### What Could Be Automated

1. **Cover node parser** -- Write a Python script to read the binary float data from `covernodes_*.pak` ZIP entries
2. **Loot table population** -- Script to generate loot table entries based on mob level ranges and item categories
3. **XP curve generation** -- Script to compute XP requirements per level based on common MMO formulas

## 9. Client Installation Game Data

For reference, the full client install contains:

| Directory | Size | Content |
|-----------|------|---------|
| `CookedPC/Maps/` | ~3.8 GB | 24 UE3 map files (.umap) |
| `CookedPC/Packages/` | ~2.3 GB | UE3 asset packages (.upk) |
| `CookedPC/UI/` | ~22 MB | CEGUI + Flash UI assets |
| `SourceCache.en-US/` | ~22 MB | Game data paks (analyzed above) |
| `Cache/` | ~700 KB | Cover node paks (binary) |

Only the SourceCache and Cache directories contain server-relevant data. The CookedPC content is entirely client-side rendering assets.
