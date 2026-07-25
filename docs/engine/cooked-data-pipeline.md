---
title: "Cooked Data Pipeline"
type: reference
audience: engineers
last_updated: 2026-07-25
---

# Cooked Data Pipeline

> **Last updated**: 2026-07-25
> **RE Status**: Delivery path verified against `crates/` and `data/cache/`; the DB→PAK
> *cooking* half is historical (original CME server) and no longer runs
> **Sources**: `data/cache/` (21 PAKs, inspected directly), `db/resources/`,
> `crates/services/src/base/cooked_data.rs`, `crates/services/src/base/resources.rs`,
> `deprecated/cpp/src/baseapp/mercury/sgw/resource.cpp`, `deprecated/python/`

> [!WARNING]
> **Two pipelines are described in this document; only one of them runs.**
>
> The *cooking* pipeline (PostgreSQL → Python `toXml()` → ZIP) was the original CME
> server's build step. Its code is now under `deprecated/`. Cimmeria does **not** cook
> PAKs — it ships the already-cooked `.pak` files in `data/cache/` and reads them at
> startup. Sections describing SOCI, Boost.Python and `toXml()` are historical.
>
> The *delivery* pipeline (`versionInfoRequest` / `onVersionInfo` / `elementDataRequest` /
> `BASEMSG_RESOURCE_FRAGMENT`) **is** live and is implemented in Rust at
> `crates/services/src/base/cooked_data.rs`.

---

## Overview

The cooked data pipeline transforms game content into XML documents packaged as PAK archives, which are served to the game client on demand.

## Architecture

### As originally built (CME server — now `deprecated/`)

```
PostgreSQL Database (db/resources/)
        |
        | Python queries via SOCI + Boost.Python
        v
Python Resource Classes (deprecated/python/base/)
        |
        | Each class has a toXml() method
        v
Cooked XML Documents
        |
        | Packaged into ZIP archives
        v
PAK Files (data/cache/*.pak)
```

### As Cimmeria runs today

```
PAK Files (data/cache/*.pak)  — 21 archives, pre-cooked, committed
        |
        | ResourceCache reads ZIP entries at startup
        v
crates/services/src/base/resources.rs  (CategoryData / ResourceCache)
        |
        | plus Cimmeria-authored overrides:
        |   crates/services/src/base/dialog_overrides.rs
        |   crates/services/src/base/item_overrides.rs
        v
crates/services/src/base/cooked_data.rs
        |
        | BASEMSG_RESOURCE_FRAGMENT (0x36), fragmented at MAX_CHUNK
        v
Game Client (CookedDataCache)
```

Cimmeria's database layer is **`sqlx` 0.8**, not SOCI, and there is no Boost.Python
binding — the server is a single Rust process (`crates/server/`).

## PAK File Format

PAK files are **standard ZIP archives** containing XML files. They can be opened with any ZIP tool.

### Inventory

| | Value (measured 2026-07-25) |
|---|---|
| PAK files in `data/cache/` | **21** |
| On-disk total | ~22.4 MB |
| Entries (excluding `MetaData`) | **55,025** |
| Uncompressed entry bytes | ~34.3 MB |
| Compressed entry bytes | ~18.0 MB |

Each PAK is a ZIP archive holding one XML document per record, plus a `MetaData` entry. See
[Cooked Data PAK File Format](cooked-data-pak-format.md) for the per-category breakdown and
the provenance of the committed set.

### XML Schema

Cooked XML is **attribute-centric**, not element-centric — record fields are XML attributes
on a single `COOKED_*` root, with child elements used only for repeated sub-structures.
A real entry, `_34` from `CookedDataAbilities.pak`:

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

The root element name encodes the category (`COOKED_ABILITY`, `COOKED_ITEM`,
`COOKED_MISSION`, `COOKED_DIALOG`, ...). If you are hand-authoring an override entry, match
this shape exactly — the client's gSOAP deserializer binds attributes by name and will fire
`onCookedDataError` on a shape it does not recognise.

## Resource Categories

The wire protocol defines **22 resource categories** (indices 1–22; index 0 is a reserved
empty slot). The authoritative list is the string array in
`deprecated/cpp/src/baseapp/mercury/sgw/resource.cpp:16-38`.

The table below maps each category index to the PAK that serves it and the entry count
measured from `data/cache/` on 2026-07-25. The counts exclude the `MetaData` entry and sum
to exactly 55,025, matching the archive totals above.

| Idx | Category | PAK file | Entries |
|--:|---|---|--:|
| 0 | *(reserved, empty)* | — | — |
| 1 | `kismet_event_sequence` | `CookedDataKismetSeqEvent.pak` | 1,973 |
| 2 | `ability` | `CookedDataAbilities.pak` | 1,886 |
| 3 | `mission` | `CookedDataMissions.pak` | 1,040 |
| 4 | `item` | `CookedDataItems.pak` | 6,059 |
| 5 | `dialog` | `CookedDataDialogs.pak` | 5,405 |
| 6 | `kismet_event_set` | `CookedDataKismetSetEvent.pak` | 675 |
| 7 | `char_creation` | `CookedCharCreation.pak` | 1 |
| 8 | `interaction_set_map` | `CookedInteractionSet.pak` | 4,663 |
| 9 | `effect` | `CookedDataEffects.pak` | 3,216 |
| 10 | `text` | `TextStrings.pak` | 29,126 |
| 11 | `error_text` | `ErrorStrings.pak` | 216 |
| 12 | `world_info` | `CookedWorldInfo.pak` | 91 |
| 13 | `stargate` | `CookedDataStargates.pak` | 28 |
| 14 | `container` | `CookedDataContainers.pak` | 20 |
| 15 | `blueprint` | `CookedBlueprints.pak` | 498 |
| 16 | `applied_science` | `CookedSciences.pak` | 4 |
| 17 | `discipline` | `CookedDisciplines.pak` | 78 |
| 18 | `racial_paradigm` | `CookedParadigm.pak` | 5 |
| 19 | `special_words` | `SpecialWords.pak` | 1 |
| 20 | `interaction` | `CookedInteractions.pak` | 40 |
| 21 | `pet_command` | **no PAK ships** | — |
| 22 | `behavior_event` | `CookedBehaviorEvents.pak` | **0** (stub archive) |
| | | **Total** | **55,025** |

Two categories carry no data. `pet_command` (21) has no PAK file at all, and
`behavior_event` (22) ships only a 120-byte stub archive containing a `MetaData` value of 1
and no entries. Requests for either will miss the cache.

> [!NOTE]
> **Category 7 (`char_creation`) is deliberately not server-pushed.** It is absent from the
> version-negotiation category map, so the client falls back to its local
> `CookedCharCreation.pak`. See `crates/services/src/base/cooked_data.rs:246`. Do not
> "fix" this by adding category 7 to the map without understanding the character-creation
> flow first.

Earlier revisions of this document listed 22 *database tables* here (ability_sets,
entity_templates, behaviors, loot_tables, mob_templates, body_components, nacsi,
store_inventories, ...). That list conflated DB schema with wire categories — several of
those tables have no resource category, and several categories have no single backing
table. The table above is the wire contract; for the database schema see `db/resources/`.

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

### Server-Side (original CME server — historical)

1. **Startup**: Server connects to PostgreSQL via SOCI
2. **Query**: Python resource classes query the database
3. **Serialize**: Each resource class generates XML via `toXml()`
4. **Package**: XML is compressed into PAK format
5. **Cache**: PAK files are stored in `data/cache/`
6. **Serve**: When client requests data, PAK contents are sent via Mercury

### Server-Side (Cimmeria — current)

1. **Startup**: `ResourceCache` opens each `data/cache/*.pak` and indexes its ZIP entries
2. **Override**: Cimmeria-authored replacements are layered on top
   (`dialog_overrides.rs`, `item_overrides.rs`); overridden element IDs are tracked per
   category so the version handshake can force a re-fetch
3. **Negotiate**: `versionInfoRequest` → `onVersionInfo` tells the client which elements to invalidate
4. **Serve**: `elementDataRequest` → one or more `BASEMSG_RESOURCE_FRAGMENT` packets

There is no database round-trip on the cooked-data path in Cimmeria. The PostgreSQL
resource schema under `db/resources/` feeds gameplay systems directly; it is not re-cooked
into PAKs at runtime.

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

> [!WARNING]
> The four steps below describe the **original CME server**. They do not work in Cimmeria,
> which does not cook PAKs from the database.

To change game content on the original server:

1. Edit the database (`db/resources/` seed or live PostgreSQL)
2. Restart the server (Python resource classes reload from database)
3. Clients receive updated cooked data on next connection
4. Optionally use hot-reload for live updates during development

**In Cimmeria**, changing a cooked-data record means adding an override entry in the
relevant module under `crates/services/src/base/` (`dialog_overrides.rs`,
`item_overrides.rs`, or a new sibling following the same pattern). The override layer
replaces the PAK entry for that element ID and marks it in the invalidation set so
`onVersionInfo` tells the client to re-fetch it. Do not edit the `.pak` files in place —
they are the pristine 2008 cook and are the reference against which overrides are diffed.

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

The Cimmeria server codebase contains **no XSD schema files** for cooked data types.

On the original server the cooked XML was generated dynamically by Python `toXml()` methods
in `deprecated/python/base/` resource classes and serialized via `base.createResource()`
(see `deprecated/cpp/src/baseapp/mercury/sgw/resource.cpp:64-65`). The XML structure was
implicitly defined by the Python code, not by formal XSD schemas. Cimmeria serves the
pre-cooked XML verbatim from the PAK archives, so no schema exists on either path.

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

Each `BASEMSG_RESOURCE_FRAGMENT` message (original C++:
`deprecated/cpp/src/baseapp/mercury/sgw/client_handler.cpp:293-382`; Rust:
`crates/services/src/base/cooked_data.rs`):

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

| | Original CME server | Cimmeria (current) |
|---|---|---|
| Max fragment body | 1000 bytes (`FragmentSize`) | **1390 bytes** (`MAX_CHUNK`, `cooked_data.rs`) |
| Throttling | `ResourceTxQueueSize`; overflow deferred to `queuedResources_`, retried each tick | — |
| Reliability | Reliable Mercury bundle | Reliable Mercury bundle |

Cimmeria raised the chunk size because Mercury's `MAX_BODY_LENGTH` is 1411 bytes and the
first fragment spends only 16 of those on headers (`BASEMSG` 1 + `WORD_LEN` 2 + `data_id` 2
+ `chunk_id` 1 + `frag_flags` 1 + `msg_type` 1 + `category_id` 4 + `element_id` 4). The
historical 1000-byte value wasted roughly 28% of every packet. 1390 leaves a 5-byte safety
margin under the tighter first-fragment cap of 1395. This is a pure throughput change —
the fragment *format* is unchanged, so client compatibility is preserved. A guard test in
`crates/services/src/mercury/protocol/tests.rs` pins the constant against the decrypt path;
change both together.

### Resource Category IDs

From `deprecated/cpp/src/baseapp/mercury/sgw/resource.cpp:16-38`, the 22 categories by index
(see the mapping table earlier in this document for the PAK that backs each one):

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
| File timestamps | **Not uniform** — see below | ZIP entry headers, read 2026-07-25 |
| Encryption | None | Standard ZIP, no password |

**Entry timestamps identify each PAK's provenance**, and the committed set is not
homogeneous:

| ZIP entry date | Count | Provenance |
|---|--:|---|
| 2008-12-11 | 17 | QA Build — the original cook |
| 2026-03-16 | 3 | `CookedDataKismetSeqEvent`, `CookedDataKismetSetEvent`, `CookedInteractionSet` — re-packed by Cimmeria to merge Server Build extras into the QA base |
| 2026-02-24 | 1 | `CookedBehaviorEvents` — the Discord Build stub |

An earlier revision of this table asserted a uniform `2014-02-04 19:29` timestamp. That was
read from a Server Build copy of the data, not from the archives in this repository.

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

Measured directly from the committed `data/cache/` archives, 2026-07-25. Entry counts
exclude `MetaData`. "Ratio" is compressed ÷ uncompressed — lower is better compression.

| PAK File | Entries | Uncompressed | Compressed | Ratio |
|----------|--------:|-------------:|-----------:|------:|
| CookedBehaviorEvents.pak | 0 | 4 B | 6 B | *(stub)* |
| CookedBlueprints.pak | 498 | 489,457 | 167,853 | 34% |
| CookedCharCreation.pak | 1 | 168,880 | 10,421 | 6% |
| CookedDataAbilities.pak | 1,886 | 1,611,952 | 890,838 | 55% |
| CookedDataContainers.pak | 20 | 7,145 | 4,300 | 60% |
| CookedDataDialogs.pak | 5,405 | 4,334,409 | 2,239,693 | 52% |
| CookedDataEffects.pak | 3,216 | 2,141,474 | 1,233,006 | 58% |
| CookedDataItems.pak | 6,059 | 6,678,016 | 3,026,956 | 45% |
| CookedDataKismetSeqEvent.pak | 1,973 | 1,054,159 | 581,585 | 55% |
| CookedDataKismetSetEvent.pak | 675 | 679,639 | 236,816 | 35% |
| CookedDataMissions.pak | 1,040 | 2,406,033 | 660,884 | 27% |
| CookedDataStargates.pak | 28 | 16,478 | 9,278 | 56% |
| CookedDisciplines.pak | 78 | 51,700 | 27,306 | 53% |
| CookedInteractionSet.pak | 4,663 | 2,091,004 | 1,227,600 | 59% |
| CookedInteractions.pak | 40 | 25,169 | 12,435 | 49% |
| CookedParadigm.pak | 5 | 1,795 | 1,052 | 59% |
| CookedSciences.pak | 4 | 1,502 | 880 | 59% |
| CookedWorldInfo.pak | 91 | 38,539 | 22,298 | 58% |
| ErrorStrings.pak | 216 | 103,134 | 57,329 | 56% |
| SpecialWords.pak | 1 | 24,735 | 1,361 | 6% |
| TextStrings.pak | 29,126 | 12,349,672 | 7,541,917 | 61% |
| **Total (21 PAKs)** | **55,025** | **~34.3 MB** | **~18.0 MB** | **52%** |

The previous revision of this table reported 20 PAKs, ~55,000 entries, ~19 MB uncompressed
and a "~38%" ratio. Those figures were computed against a different (Server Build) copy of
the data, counted `MetaData` as an entry, and inverted the ratio convention. The numbers
above are from the archives actually committed to this repository.

---

## TODO

- [x] ~~Document the exact XSD schema for each cooked data type~~ → See "XSD Schemas" section above. No XSD files exist server-side. Client uses gSOAP-generated type bindings with 42 CookedData types registered. Full type inventory documented.
- [x] ~~Determine if the client validates XML against XSD at runtime~~ → See "Client XSD Validation" section above. No runtime XSD validation. Client uses gSOAP compile-time deserializers; parse failures trigger `onCookedDataError` events.
- [x] ~~Document the incremental update protocol~~ → ClientCache `versionInfoRequest`/`onVersionInfo`/`elementDataRequest` in `findings/entity-types-wire-formats.md`
- [x] ~~Map the exact Mercury message format for cooked data delivery~~ → See "Mercury Resource Delivery Format" section above. Uses `BASEMSG_RESOURCE_FRAGMENT (0x36)`; original C++ fragmented at 1000 bytes, Cimmeria at 1390. Wire format: dataId, chunkId, flags, then header (msgType, categoryId, elementId) on first fragment only, followed by XML body. 22 resource categories mapped.
- [x] ~~Verify the PAK file compression level and format details~~ → See "PAK File Format Details" section above. Standard ZIP archives with DEFLATE compression (method 8). **21** PAK files containing **55,025** entries totaling ~34.3 MB uncompressed / ~18.0 MB compressed. MetaData entry is a 4-byte little-endian uint32. XML entries named by `_<databaseId>`.
- [ ] Confirm whether category 21 (`pet_command`) was ever cooked. No PAK ships for it in any known build; it may have been abandoned before content authoring began.
