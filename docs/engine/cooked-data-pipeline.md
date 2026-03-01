# Cooked Data Pipeline

> **Last updated**: 2026-03-01
> **RE Status**: Fully documented -- pipeline is working
> **Sources**: `data/cache/`, `db/resources.sql`, `python/`, `docs/game-data.md`

---

## Overview

The cooked data pipeline transforms game content from the PostgreSQL database into XML files packaged as PAK archives. These are served to the game client on demand. The pipeline is fully implemented and functional in Cimmeria.

## Architecture

```
PostgreSQL Database (db/resources.sql)
        |
        | Python queries via SOCI + Boost.Python
        v
Python Resource Classes (python/base/)
        |
        | Each class has a toXml() method
        v
Cooked XML Documents
        |
        | Packaged into ZIP archives
        v
PAK Files (data/cache/*.pak)
        |
        | Served to client via Mercury
        v
Game Client (CookedDataCache)
```

## PAK File Format

PAK files are **standard ZIP archives** containing XML files. They can be opened with any ZIP tool.

### Inventory

| PAK File | Size | Contents |
|----------|------|----------|
| `data/cache/` | ~16 MB total | All cooked game data |

Each PAK file contains one or more XML documents corresponding to a resource category (abilities, items, missions, etc.).

### XML Schema

The cooked XML follows schemas defined by XSD files in the client data. Each element is a "cooked" representation of a database record:

```xml
<!-- Example: cooked ability -->
<Ability>
    <Id>12345</Id>
    <Name>Staff Strike</Name>
    <Description>A basic melee attack</Description>
    <CooldownTime>1.5</CooldownTime>
    <Range>3.0</Range>
    <!-- ... -->
</Ability>
```

## Resource Categories

The pipeline handles 22 resource categories:

| Category | DB Table(s) | Record Count | Description |
|----------|-------------|--------------|-------------|
| Abilities | abilities | 1,887 | Combat abilities and trees |
| Ability Sets | ability_sets | ~200 | Grouped ability collections |
| Items | items | 6,060 | Weapons, armor, consumables |
| Effects | effects | 3,217 | Combat and visual effects |
| Missions | missions | 1,041 | Quest definitions |
| Dialogs | dialog_trees | 5,406 | NPC dialog trees |
| Crafting Blueprints | blueprints | 499 | Crafting recipes |
| Stargates | stargates | 29 | Gate addresses and connections |
| Character Defs | character_defs | 23 | Character creation options |
| Spawn Templates | entity_templates | 153 | NPC spawn configurations |
| Behaviors | behaviors | ~100 | NPC AI behaviors |
| Interaction Sets | interaction_sets | ~200 | NPC interaction menus |
| Loot Tables | loot_tables | 2 | Item drop tables |
| World Info | zones | 91 | Zone definitions |
| Text Strings | localized_text | 29,126 | Localized UI text |
| MOBs | mob_templates | ~150 | Monster/NPC templates |
| Body Components | body_components | ~5,000 | Visual mesh references |
| NACSI | nacsi | ~100 | Animation/cutscene info |
| Sciences | sciences | ~50 | Crafting science trees |
| Disciplines | disciplines | ~30 | Crafting disciplines |
| Store Inventories | store_inventories | ~50 | NPC store contents |
| Racial Paradigms | racial_paradigms | ~10 | Racial progression |

## CookedElementBase System

On the client side, the `CookedElementBase` class hierarchy manages deserialization:

```
CookedElementBase
    |
    +-- CookedAbility
    +-- CookedItem
    +-- CookedMission
    +-- CookedEffect
    +-- CookedDialog
    +-- ... (one per resource type)
```

Each subclass knows how to parse its corresponding XML format and populate in-memory structures for the client.

## Data Loading Flow

### Server-Side (Cimmeria)

1. **Startup**: Server connects to PostgreSQL via SOCI
2. **Query**: Python resource classes query the database
3. **Serialize**: Each resource class generates XML via `toXml()`
4. **Package**: XML is compressed into PAK format
5. **Cache**: PAK files are stored in `data/cache/`
6. **Serve**: When client requests data, PAK contents are sent via Mercury

### Client-Side (SGW.exe)

1. **Request**: Client sends `LoadConstants`, `LoadAbility`, `LoadItem`, etc. messages
2. **Receive**: Server responds with cooked XML data
3. **Parse**: `CookedElementBase` subclasses deserialize the XML
4. **Cache**: `CookedDataCache` stores parsed data in memory
5. **Validate**: Cache stamps track data version for incremental updates

### Hot-Reload

The client supports hot-reloading of cooked data without restarting:

| Message | Direction | Description |
|---------|-----------|-------------|
| `RequestReload` | Client -> Server | Request fresh data |
| `onCookedDataError` | Server -> Client | Data validation error |

Debug/GM commands can trigger reloads:
- `LoadAbility`, `LoadAbilitySet`, `LoadBehavior`, `LoadMOB`
- `LoadInteractionSet`, `LoadItem`, `LoadMission`, `LoadNACSI`

## Modifying Game Data

To change game content:

1. Edit the database (`db/resources.sql` or live PostgreSQL)
2. Restart the server (Python resource classes reload from database)
3. Clients receive updated cooked data on next connection
4. Optionally use hot-reload for live updates during development

## Data Integrity

The cooked data system includes version tracking:

- **Protocol digest**: MD5 hash of entity definitions ensures client/server agreement
- **Cache stamps**: Per-entity version numbers allow incremental updates
- **Error reporting**: `onCookedDataError` notifies the client of validation failures

## Related Documents

- [Game Data](../game-data.md) -- Complete data inventory
- [CME Framework](cme-framework.md) -- Atrea Script Editor and other CME systems
- [BigWorld Architecture](bigworld-architecture.md) -- Entity system context
- [Service Architecture](../architecture/service-architecture.md) -- Server configuration

## TODO

- [ ] Document the exact XSD schema for each cooked data type
- [ ] Determine if the client validates XML against XSD at runtime
- [ ] Document the incremental update protocol (cache stamp comparison)
- [ ] Map the exact Mercury message format for cooked data delivery
- [ ] Verify the PAK file compression level and format details
