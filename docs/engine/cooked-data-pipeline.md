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

## XSD Schemas

### Server-Side: No XSD Files

The Cimmeria server codebase contains **no XSD schema files** for cooked data types. The only `.xsd` files in the repository are unrelated Boost test infrastructure files under `external/boost/`.

The cooked XML is generated dynamically by Python `toXml()` methods in `python/base/` resource classes and serialized by the server via `base.createResource()`. The XML structure is implicitly defined by the Python code, not by formal XSD schemas.

### Client-Side: XSD Type Information (from Ghidra)

The client binary uses XSD type annotations in its gSOAP-based XML deserializer. Each cooked data type is registered as a `CookedData:*` namespace type in the gSOAP schema system. The client binary contains string references for the following XSD primitive types used in cooked data: `xsd:byte`, `xsd:int`, `xsd:float`, `xsd:boolean`, `xsd:string`, `xsd:QName`, `xsd:dateTime`, `xsd:unsignedByte`, `xsd:unsignedInt`, `xsd:unsignedLong`, `xsd:unsignedShort`, `xsd:short`, `xsd:decimal`, `xsd:hexBinary`, `xsd:token`, `xsd:NMTOKEN`, `xsd:Name`.

The full list of CookedData types registered in the client (from Ghidra string analysis):

| CookedData Type | Description |
|-----------------|-------------|
| `CookedData:AbilityType` | Combat ability definitions |
| `CookedData:EffectType` | Combat/visual effect definitions |
| `CookedData:MissionType` | Quest definitions |
| `CookedData:MissionStepsType` | Quest step sequences |
| `CookedData:MissionObjectiveType` | Quest objective details |
| `CookedData:MissionTaskType` | Quest task details |
| `CookedData:ItemType` | Item definitions |
| `CookedData:ItemRangeSetType` | Item range parameters |
| `CookedData:ItemEventSetType` | Item event triggers |
| `CookedData:ItemRequirementsSetType` | Item requirements |
| `CookedData:ItemInventorySetType` | Item inventory settings |
| `CookedData:DialogType` | Dialog tree roots |
| `CookedData:DialogScreenType` | Dialog screen nodes |
| `CookedData:DialogButtonType` | Dialog button choices |
| `CookedData:ContainerType` | Inventory containers |
| `CookedData:StargateType` | Stargate definitions |
| `CookedData:WorldInfoType` | Zone/world information |
| `CookedData:InteractionSetMapType` | NPC interaction menus |
| `CookedData:InteractionType` | Individual interactions |
| `CookedData:BlueprintType` | Crafting blueprints |
| `CookedData:BlueprintComponentListType` | Blueprint component lists |
| `CookedData:BlueprintComponentType` | Individual blueprint components |
| `CookedData:AppliedScienceType` | Crafting science definitions |
| `CookedData:DisciplineType` | Crafting discipline definitions |
| `CookedData:DisciplineListType` | Discipline list containers |
| `CookedData:RacialParadigmType` | Racial progression data |
| `CookedData:CharCreateCharDefSetType` | Character creation definition sets |
| `CookedData:CharCreateCharDefType` | Character creation definitions |
| `CookedData:CharCreateVisualGroupType` | Character visual group options |
| `CookedData:CharCreateChoiceType` | Character creation choices |
| `CookedData:ObjectTextType` | Localized text strings |
| `CookedData:ErrorTextType` | Error/system text |
| `CookedData:ErrorIDType` | Error string identifiers |
| `CookedData:ObjectTextIDType` | Text string identifiers |
| `CookedData:SpecialWords` | Special word filter container |
| `CookedData:SpecialWordType` | Individual special words |
| `CookedData:BehaviorEventType` | NPC behavior events |
| `CookedData:KismetEventSetType` | Kismet event set definitions |
| `CookedData:KismetEventSequenceType` | Kismet event sequences |
| `CookedData:MonikerType` | Named identifier references |
| `CookedData:NVPType` | Name-value pair parameters |

---

## Client XSD Validation

**The client does NOT validate cooked data XML against XSD schemas at runtime.**

Evidence from Ghidra analysis:

1. **No XSD loading functions found**: Searching for `XSD`, `Validate`, and `schema` in the client function list yields no XSD validation functions. The only `Validate` functions found are `APlayerController_execClientValidate` (unrelated network validation) and internal CRT image validation (`__ValidateImageBase`).

2. **XSD strings are for gSOAP type annotations, not validation**: The `xsd:*` strings in the binary (e.g., `xsd:int`, `xsd:string`, `xsd:boolean`) are gSOAP (Simple Object Access Protocol) type descriptor strings used for XML serialization/deserialization, not for schema validation. They define how to parse XML attributes and elements into C++ types.

3. **XSD references for CEGUI only**: The only `.xsd` file references in the binary are for the CEGUI GUI framework (`Font.xsd`, `GUILayout.xsd`, `CEGUIConfig.xsd`, `Imageset.xsd`, `GUIScheme.xsd`, `Falagard.xsd`), which are unrelated to cooked game data.

4. **Error handling is via `onCookedDataError` events**: The client uses `Event_NetIn_onCookedDataError` signal/callback infrastructure to handle bad cooked data, with per-type `ElementError` and `ElementReady` events. This is a runtime error notification system, not a schema validation system.

**Conclusion**: The client uses gSOAP's code-generated deserializers (compile-time type bindings) to parse cooked data XML. If the XML structure does not match the expected gSOAP-generated class layout, parsing fails and an `onCookedDataError` event is fired. There is no separate XSD validation pass.

---

## Mercury Resource Delivery Format

Cooked data is delivered from the BaseApp to the client via the `BASEMSG_RESOURCE_FRAGMENT` (message ID `0x36`) Mercury message. The delivery uses a fragmentation system to split large XML documents into multiple UDP packets.

### Resource Request Flow

```
Client                          BaseApp
  |                                |
  |  elementDataRequest(cat, key)  |    (Entity RPC via ClientCache.def)
  |------------------------------->|
  |                                |--- ResourceManager.get(cat, key)
  |                                |    (check cache, or call Python
  |                                |     base.createResource(cat, key))
  |                                |
  |  BASEMSG_RESOURCE_FRAGMENT x N |    (one or more fragments)
  |<-------------------------------|
  |                                |
```

### Fragment Wire Format

Each `BASEMSG_RESOURCE_FRAGMENT` message (`src/baseapp/mercury/sgw/client_handler.cpp:293-382`):

```
+--------+--------+-------+--------+--------+--------+-----------+
| uint16 | uint8  | uint8 | uint8  | uint32 | uint32 | uint8[]   |
| dataId | chunkId| flags | msgType| catId  | elemId | xmlBody   |
+--------+--------+-------+--------+--------+--------+-----------+
         ^                 ^--- Only in first fragment (INITIAL)
         |
         +--- Sequential chunk counter
```

| Field | Size | Description |
|-------|------|-------------|
| `dataId` | `uint16` | Unique per-resource transfer identifier (increments per `sendResource` call) |
| `chunkId` | `uint8` | Fragment sequence number (0, 1, 2, ...) |
| `flags` | `uint8` | Bitfield: `RESOURCE_BASE_FLAG (0x40)` always set, `RESOURCE_INITIAL_FRAGMENT (0x01)` on first, `RESOURCE_FINAL_FRAGMENT (0x02)` on last |
| `msgType` | `uint8` | Always `0` (`MESSAGE_CacheData`). Only present in the first fragment. |
| `categoryId` | `uint32` | Resource category index (see table below). Only present in the first fragment. |
| `elementId` | `uint32` | Resource identifier (e.g., ability ID). Only present in the first fragment. |
| `xmlBody` | `uint8[]` | Raw XML document bytes (UTF-8). Fragmented at 1000-byte boundaries. |

### Fragment Size and Throttling

- **Maximum fragment body size**: 1000 bytes (`FragmentSize` constant)
- **Transmit queue limit**: Controlled by `ResourceTxQueueSize`; if the send queue is full, resources are deferred to a `queuedResources_` list and retried each tick
- **Reliability**: Sent via the reliable Mercury bundle (not the unreliable channel)

### Resource Category IDs

From `src/baseapp/mercury/sgw/resource.cpp`, the 22 categories by index:

| Index | Category Name | Description |
|-------|--------------|-------------|
| 0 | *(empty)* | Reserved |
| 1 | `kismet_event_sequence` | Kismet event sequences |
| 2 | `ability` | Combat abilities |
| 3 | `mission` | Quest definitions |
| 4 | `item` | Item definitions |
| 5 | `dialog` | Dialog trees |
| 6 | `kismet_event_set` | Kismet event sets |
| 7 | `char_creation` | Character creation |
| 8 | `interaction_set_map` | Interaction menus |
| 9 | `effect` | Effects |
| 10 | `text` | Localized text |
| 11 | `error_text` | Error strings |
| 12 | `world_info` | Zone data |
| 13 | `stargate` | Stargates |
| 14 | `container` | Inventory containers |
| 15 | `blueprint` | Crafting blueprints |
| 16 | `applied_science` | Applied sciences |
| 17 | `discipline` | Crafting disciplines |
| 18 | `racial_paradigm` | Racial paradigms |
| 19 | `special_words` | Chat filter words |
| 20 | `interaction` | Interactions |
| 21 | `pet_command` | Pet commands |
| 22 | `behavior_event` | NPC behavior events |

### ClientCache Entity Interface

The request side uses the `ClientCache` entity interface (`entities/defs/interfaces/ClientCache.def`):

| Method | Direction | Args | Description |
|--------|-----------|------|-------------|
| `versionInfoRequest` | Client -> Base (Exposed) | `INT32 CategoryId`, `INT32 Version` | Request cache version info for a category |
| `elementDataRequest` | Client -> Base (Exposed) | `INT32 CategoryId`, `INT32 Key` | Request a specific cooked data element |
| `onVersionInfo` | Base -> Client | `INT32 CategoryId`, `INT32 Version`, `INT32 RequiredUpdates`, `INT8 InvalidateAll`, `ARRAY<INT32> InvalidKeys` | Version info response with invalidation data |
| `onCookedDataError` | Base -> Client | `INT32 categoryID`, `INT32 elementKey` | Error notification for failed resource load |

---

## PAK File Format Details

PAK files are **standard ZIP archives** using **DEFLATE compression** (method 8). Verified by direct inspection of files in `data/cache/`.

### Archive Properties

| Property | Value | Source |
|----------|-------|--------|
| Format | ZIP (PK\x03\x04 header) | `file` command output |
| Minimum version | 2.0 to extract | ZIP central directory |
| Compression method | DEFLATE (method 8) | All entries, including MetaData |
| Compression level | Default (zlib level 6) | Inferred from compression ratios |
| File timestamps | 2014-02-04 19:29 | Uniform across all entries |
| Encryption | None | Standard ZIP, no password |

### Entry Naming Convention

- **`MetaData`**: A 4-byte binary file present in every PAK. Contains a little-endian `uint32` that appears to be a record count or version number (e.g., `0x00001f7a` = 8058 in CookedDataAbilities.pak, `0x00001d76` = 7542 in CookedDataItems.pak).
- **`_<id>`**: Individual cooked data elements named by their database primary key (e.g., `_34` for ability ID 34, `_10` for item ID 10).

### Sample XML Content

Each entry contains a single XML document with SOAP namespace declarations:

```xml
<?xml version="1.0" encoding="UTF-8"?>
<COOKED_ABILITY AbilityId="34" AbilityName="Burst-" AbilityDesc="..."
    AbilityTypeId="5" TargetTypeId="2" WarmupSeconds="0.0" CooldownSeconds="1.0"
    PassiveYN="false" IsRanged="true" TrainingCost="0" ...
    xmlns:CookedData1="SGW"
    xmlns:SOAP-ENC="http://schemas.xmlsoap.org/soap/encoding/"
    xmlns:SOAP-ENV="http://schemas.xmlsoap.org/soap/envelope/"
    xmlns:xsd="http://www.w3.org/2001/XMLSchema"
    xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
  <EffectIds>77</EffectIds>
  <Moniker MonikerID="3175425141" />
</COOKED_ABILITY>
```

### Compression Statistics

| PAK File | Entries | Uncompressed | Compressed | Ratio |
|----------|---------|-------------|------------|-------|
| CookedDataAbilities.pak | 1,887 | 1.6 MB | 891 KB | 44% |
| CookedDataItems.pak | 6,060 | 4.7 MB | 2.4 MB | 49% |
| CookedDataDialogs.pak | 5,406 | 3.0 MB | 1.7 MB | 44% |
| CookedDataEffects.pak | 3,217 | 1.3 MB | 890 KB | 33% |
| CookedDataMissions.pak | 1,041 | 2.1 MB | 549 KB | 74% |
| CookedBlueprints.pak | 499 | 310 KB | 115 KB | 63% |
| CookedCharCreation.pak | 24 | 156 KB | 20 KB | 87% |
| CookedInteractionSet.pak | 4,664 | 836 KB | 716 KB | 14% |
| TextStrings.pak | 29,127 | 4.9 MB | 4.4 MB | 12% |
| SpecialWords.pak | 2 | 22 KB | 1.3 KB | 94% |
| **Total (all 20 PAKs)** | **~55,000** | **~19 MB** | **~12 MB** | **~38%** |

---

## TODO

- [x] ~~Document the exact XSD schema for each cooked data type~~ → See "XSD Schemas" section above. No XSD files exist server-side. Client uses gSOAP-generated type bindings with 42 CookedData types registered. Full type inventory documented.
- [x] ~~Determine if the client validates XML against XSD at runtime~~ → See "Client XSD Validation" section above. No runtime XSD validation. Client uses gSOAP compile-time deserializers; parse failures trigger `onCookedDataError` events.
- [x] ~~Document the incremental update protocol~~ → ClientCache `versionInfoRequest`/`onVersionInfo`/`elementDataRequest` in `findings/entity-types-wire-formats.md`
- [x] ~~Map the exact Mercury message format for cooked data delivery~~ → See "Mercury Resource Delivery Format" section above. Uses `BASEMSG_RESOURCE_FRAGMENT (0x36)` with fragmentation at 1000-byte boundaries. Wire format: dataId, chunkId, flags, then header (msgType, categoryId, elementId) on first fragment only, followed by XML body. 22 resource categories mapped.
- [x] ~~Verify the PAK file compression level and format details~~ → See "PAK File Format Details" section above. Standard ZIP archives with DEFLATE compression (method 8). 20 PAK files containing ~55,000 entries totaling ~19 MB uncompressed / ~12 MB compressed. MetaData entry is a 4-byte uint32. XML entries named by `_<databaseId>`.
