---
title: "Entity Definition (.def) File Guide"
type: reference
audience: engineers
last_updated: 2026-07-25
---

# Entity Definition (.def) File Guide

> [!NOTE]
> **Verification basis (2026-07-25).** Every claim in this document about *what SGW's
> `.def` files actually contain* has been checked against the 18 entity defs in
> `entities/defs/` and the 18 interface defs in `entities/defs/interfaces/`. Claims about
> *what the BigWorld engine supports in general* are historical and cannot be verified in
> this checkout — the BigWorld 2.0.1 reference tree (`external/engines/BigWorld-Engine-2.0.1/`)
> is not in git and no script in `setup.ps1` or `bootstrap/` fetches it. Where SGW uses only
> a subset of an engine feature, that is now called out inline.

Entity definition files are XML documents that define the complete client-server contract for each entity type in the BigWorld engine. They specify what properties an entity has, what methods can be called on it, and which parts of the entity are visible to which components of the distributed system (cell, base, client). For the Cimmeria project, these files are the authoritative source of truth for the entity system -- they dictate what the server must implement and what the client expects to receive.

---

## Why Entity Definitions Matter

In BigWorld's distributed architecture, an entity can exist simultaneously on multiple components:

- **Cell** -- The authoritative simulation on the CellApp. Handles physics, combat, AI, and spatial logic.
- **Base** -- The persistent identity on the BaseApp. Handles login, persistence, mailbox routing.
- **Client** -- The visual representation on the player's machine. Handles rendering, UI, and input.

The .def file defines the contract between these components. It specifies:

1. **Properties**: What data the entity holds, which components can see it, and whether it persists to the database.
2. **Methods**: What remote procedure calls each component can make on each other, and what arguments they take.
3. **Interfaces**: Reusable bundles of properties and methods that can be mixed into multiple entity types.

If the server sends a property update with the wrong type or calls a client method with the wrong arguments, the client will crash or disconnect. The .def file prevents this by making the contract explicit and enforced at both ends.

---

## File Locations

| Path | Contents |
|------|----------|
| `entities/defs/*.def` | Top-level entity definitions (SGWPlayer.def, SGWMob.def, etc.) |
| `entities/defs/interfaces/*.def` | Interface definitions (SGWCombatant.def, Communicator.def, etc.) |
| `entities/entities.xml` | Entity type registry -- lists all entity types known to the engine |
| `entities/defs/alias.xml` | Custom type aliases (FIXED_DICT, ARRAY, and simple type aliases) |
| `entities/defs/enumerations.xml` | Game-specific enumeration types |
| `entities/custom_alias.xml` | Cimmeria-added type aliases (kept separate from the shipped `alias.xml`) |
| `entities/custom_enumerations.xml` | Cimmeria-added enumerations |
| `entities/spaces.xml` | Space registry — 24 spaces with extents and instancing flags |
| `entities/cell_spaces.xml` | The 16 non-instanced spaces the cell creates at startup |

---

## XML Structure Overview

Every .def file has a `<root>` element containing some combination of the following sections:

```xml
<root>
    <Parent>ParentEntityName</Parent>       <!-- optional: inherits from this entity -->
    <ServerOnly />                          <!-- optional: entity has no client representation -->

    <Implements>                            <!-- optional: list of interfaces -->
        <Interface>InterfaceName</Interface>
    </Implements>

    <Volatile>                              <!-- optional: volatile (non-replicated) spatial data -->
        <position/>
        <yaw/>
        <pitch/>
        <roll/>
    </Volatile>

    <Properties>                            <!-- entity properties -->
        <propertyName>
            <Type>TYPE</Type>
            <Flags>FLAG</Flags>
            <Default>value</Default>
            <Persistent>true</Persistent>
            <DatabaseLength>64</DatabaseLength>
            <Identifier>true</Identifier>
            <DetailLevel>FAR</DetailLevel>   <!-- engine feature; NEVER used in SGW's defs -->
        </propertyName>
    </Properties>

    <CellMethods>                           <!-- methods callable on the cell entity -->
        <methodName>
            <Exposed/>                      <!-- optional: client can call this -->
            <Arg>TYPE <ArgName>name</ArgName></Arg>
        </methodName>
    </CellMethods>

    <BaseMethods>                           <!-- methods callable on the base entity -->
        <methodName>
            <Arg>TYPE</Arg>
        </methodName>
    </BaseMethods>

    <ClientMethods>                         <!-- methods callable on the client proxy -->
        <methodName>
            <Arg>TYPE <ArgName>name</ArgName></Arg>
        </methodName>
    </ClientMethods>

    <LoDLevels>                             <!-- Level-of-Detail definitions -->
    </LoDLevels>                            <!-- all 22 LoDLevels blocks in SGW are EMPTY -->
</root>
```

**Which of these SGW actually uses.** Counted across all 36 def files (18 entity + 18 interface):

| Element | Occurrences in `entities/defs/` |
|---|---|
| `<Flags>` | 436 (every property) — but only three distinct values, see below |
| `<Type>` | 436 (every property) |
| `<Exposed/>` | 262 |
| `<ArgName>` | 1,098 |
| `<Volatile>` | 7 files (`SGWEntity.def` plus 6 interfaces, most of them empty) |
| `<ServerOnly />` | 10 entity defs |
| `<LoDLevels>` | 22 blocks, **all empty** |
| `<Persistent>` | **2** (`SGWPlayer.playerName`, `SGWSpaceCreator.areaKey`) |
| `<DatabaseLength>` | **2** (same two properties) |
| `<Identifier>` | **2** (same two properties) |
| `<DetailLevel>` | **0** — never used |

The takeaway: SGW leans on `<Flags>`, `<Type>`, `<Exposed/>` and almost nothing else.
Persistence is **not** driven by `<Persistent>` in this game — only two properties carry
it. Character state is persisted by explicit server-side database code, not by the
BigWorld auto-persistence path.

---

## The <Parent> Element

Entities can inherit from other entities. The `<Parent>` tag specifies the parent entity type.

```xml
<Parent>SGWSpawnableEntity</Parent>
```

A child entity inherits all properties, methods, and interfaces from its parent. The child can add new properties and methods but cannot remove inherited ones. Interfaces from the parent are also inherited.

The Cimmeria entity hierarchy is:

```
SGWEntity (root entity)
  |-- <Implements> DistributionGroupMember, EventParticipant
  |-- Properties: dbID, nextRequestID, pendingRequests, timers, createOnCell
  |
  +-- SGWSpawnableEntity
        |-- Properties: kismetEventSetId, SpawnSetID, staticMeshName, bodySet, mobId, ...
        |
        +-- SGWBeing
        |     |-- <Implements> SGWBeing(interface), SGWAbilityManager, SGWCombatant
        |     |-- Properties: (none directly, all from interfaces)
        |     |
        |     +-- SGWMob
        |     |     |-- <Implements> Lootable
        |     |     |-- Properties: navControllerID, AIState, threatList, LootTableID, ...
        |     |     |
        |     |     +-- SGWPet
        |     |
        |     +-- SGWPlayer
        |           |-- <Implements> Communicator, OrganizationMember, MinigamePlayer,
        |           |                 GateTravel, SGWInventoryManager, SGWMailManager,
        |           |                 Missionary, SGWPoller, ContactListManager,
        |           |                 SGWBlackMarketManager, ClientCache
        |           |-- Properties: playerName, account, level, experience, ...
        |           |
        |           +-- SGWGmPlayer
        |
        +-- SGWDuelMarker
```

Six further entities descend directly from `SGWEntity` rather than from
`SGWSpawnableEntity`, and are easy to miss because they sit outside the combat branch:

```
SGWEntity
  +-- SGWCoverSet
  +-- SGWEscrow
  +-- SGWPlayerGroupAuthority   <Implements> GroupAuthority
  +-- SGWPlayerRespawner
  +-- SGWSpaceCreator
  +-- SGWSpawnRegion
  +-- SGWSpawnSet               <Implements> GroupAuthority
```

**Entities with genuinely no `<Parent>` — there are only three:**

| Entity | Note |
|---|---|
| `SGWEntity` | The root of the hierarchy. |
| `SGWBlackMarket` | Auction-house singleton. `<ServerOnly />`. |
| `SGWChannelManager` | Chat-channel singleton. `<ServerOnly />`. |

`Account` is a common trap. It *looks* parentless in the sense that it has no BigWorld
parent, but it does contain a `<Parent>` tag:

```xml
<UnrealProperties>
    <Parent>GamePawn</Parent>
</UnrealProperties>
```

That `GamePawn` is the **Unreal Engine 3 Actor class** used for the player controller, not
a BigWorld entity parent. A naive grep for `<Parent>` across `entities/defs/` will report
`Account.def -> GamePawn` and mislead you. Scope the search to tags outside
`<UnrealProperties>`.

`Account` also implements the `ClientCache` interface — the same interface `SGWPlayer`
uses — because the cooked-data cache negotiation happens before character selection, while
the player is still an Account entity. The def file's own comment flags this as a
technical rather than logical grouping.

---

## The <Implements> Section

Interfaces are reusable bundles of properties and methods defined in `entities/defs/interfaces/`. An entity that implements an interface gains all of its properties, methods, and sub-interfaces.

```xml
<Implements>
    <Interface>SGWCombatant</Interface>
    <Interface>SGWAbilityManager</Interface>
    <Interface>Communicator</Interface>
</Implements>
```

Interface .def files have the same structure as entity .def files (Properties, CellMethods, BaseMethods, ClientMethods) but cannot have a `<Parent>` tag or be registered in `entities.xml`. They exist only to be included by entities.

**Example:** SGWPlayer implements 11 interfaces, giving it hundreds of properties and methods across combat, inventory, missions, chat, mail, organizations, gate travel, crafting, and minigames.

The full list of interfaces in Cimmeria:

| Interface | Purpose | Key Properties/Methods |
|-----------|---------|----------------------|
| ClientCache | Client-side data caching | Proxy message handling |
| Communicator | Chat and communication | Chat channels, DND/AFK, tells |
| ContactListManager | Friends/ignore lists | Contact list CRUD |
| DistributionGroupMember | Distribution group membership | Group callbacks |
| EventParticipant | Event system participation | Event callbacks |
| GateTravel | Stargate/ring transporter travel | Gate dialing, destination selection |
| GroupAuthority | Squad/team management | Squad formation, loot rules |
| Lootable | Loot drop capability | Loot table, loot generation |
| MinigamePlayer | Minigame participation | Minigame start/end/scoring |
| Missionary | Mission/quest system | Mission accept/complete/status |
| OrganizationMember | Guild membership | Org join/leave/rank/vault |
| SGWAbilityManager | Ability/effect system | Cooldowns, warmups, effects, channels |
| SGWBeing | Being properties and methods | Name, level, target, appearance, state |
| SGWBlackMarketManager | Auction house | Search, bid, create listing |
| SGWCombatant | Combat system | Stats, health, alignment, combat state |
| SGWInventoryManager | Inventory/equipment | Bags, slots, equip/unequip |
| SGWMailManager | In-game mail | Send/receive/attachments |
| SGWPoller | Polling/survey system | Poll responses |

---

## The <Properties> Section

Properties define the data an entity holds. Each property has a name (the XML tag), a type, visibility flags, and optional attributes.

### Property Definition

```xml
<playerName>
    <Type>          WSTRING         </Type>
    <Flags>         CELL_PUBLIC     </Flags>
    <Identifier>    true            </Identifier>
    <Persistent>    true            </Persistent>
    <DatabaseLength>    64          </DatabaseLength>
</playerName>
```

(Verbatim from `entities/defs/SGWPlayer.def:25-31`. Note there is no `<Default>` — the two
`<Identifier>`/`<Persistent>`/`<DatabaseLength>` properties in the whole game are this one
and `SGWSpaceCreator.areaKey`.)

### Property Flags

Flags control which components of the distributed system can see the property and how changes are propagated. This is the most important aspect of property definitions for server implementation.

> [!IMPORTANT]
> **SGW uses exactly three of these flags.** Across all 436 properties in the 36 def files,
> the only values that ever appear are:
>
> | Flag | Count | Share |
> |---|--:|--:|
> | `CELL_PRIVATE` | 310 | 71% |
> | `BASE` | 64 | 15% |
> | `CELL_PUBLIC` | 62 | 14% |
>
> `OWN_CLIENT`, `OTHER_CLIENTS`, `ALL_CLIENTS`, `BASE_AND_CLIENT` and
> `CELL_PUBLIC_AND_OWN` appear **zero times** anywhere in `entities/defs/` or the
> `entities/*.xml` files. Verified by exhaustive grep, 2026-07-25.
>
> **This is the single most protocol-relevant fact in this document.** SGW does not use
> BigWorld's automatic client property replication at all. *Every* piece of state the
> client sees arrives through an explicit `<ClientMethods>` RPC that server code chooses to
> call — `onStatUpdate`, `onStateFieldUpdate`, `onEffectResults`, and so on. There is no
> "set the property and the engine syncs it" path in this game. If you are implementing a
> feature and looking for the property flag that will push a value to the client, it does
> not exist; find or add the ClientMethod instead.

The table below documents the full BigWorld flag set for reference. The three rows SGW
actually uses are marked; the rest are engine capability that this game left on the table.

| Flag | Used in SGW? | Cell Sees? | Base Sees? | Own Client? | Other Clients? | Description |
|------|:---:|-----------|-----------|-------------|---------------|-------------|
| `CELL_PRIVATE` | **yes (310)** | Yes | No | No | No | Only the CellApp can read/write. No network sync. |
| `CELL_PUBLIC` | **yes (62)** | Yes | No | No | No | CellApp can read/write. Other CellApps in range can read. |
| `BASE` | **yes (64)** | No | Yes | No | No | Only the BaseApp can read/write. Not on the cell. |
| `OWN_CLIENT` | no | Yes | No | Yes | No | Synced to the owning player's client only. |
| `OTHER_CLIENTS` | no | Yes | No | No | Yes | Synced to other players' clients but NOT the owner. |
| `ALL_CLIENTS` | no | Yes | No | Yes | Yes | Synced to all clients in range (owner and others). |
| `BASE_AND_CLIENT` | no | No | Yes | Yes | No | BaseApp and owning client. Not on the cell at all. |
| `CELL_PUBLIC_AND_OWN` | no | Yes | No | Yes | No | Like CELL_PUBLIC but also synced to owner's client. |

**Key implications for server implementation:**

- `CELL_PRIVATE` properties are internal state. They never cross the network. Examples: AI timers, internal cooldown tracking, threat lists. At 71% of all properties, this is the default in SGW.
- `CELL_PUBLIC` properties are, in stock BigWorld, shared between CellApps for Area of Interest. Cimmeria runs a single cell, so in practice these behave identically to `CELL_PRIVATE` — the distinction only matters if multi-CellApp support is ever added. Examples: `Alignment`, `playerName`.
- `BASE` properties exist only on the BaseApp and are never on the cell. Examples: `Account.characterList`, account reference mailboxes.
- Properties with `<Persistent>true</Persistent>` would be saved to the database by the engine — but only two properties in SGW carry the tag, so this is not how the game persists state. See the element-usage table above.

### Property Types

BigWorld defines a set of primitive and composite types for properties and method arguments.

**Types actually used by SGW's 436 properties**, counted 2026-07-25:

| Type | Count | | Type | Count |
|---|--:|---|---|--:|
| `INT32` | 107 | | `MAILBOX` | 10 |
| `PYTHON` | 91 | | `VECTOR3` | 7 |
| `INT8` | 51 | | `StatList` | 6 |
| `FLOAT` | 36 | | `STRING` | 6 |
| `CONTROLLER_ID` | 29 | | `INT16` | 1 |
| `UINT8` | 22 | | `DBID` | 1 |
| `WSTRING` | 15 | | `CharacterInfoList` | 1 |
| `UINT32` | 13 | | `LootItemDefinitionList` | 1 |
| | | | `EscrowRecordList` | 1 |

Notably **absent from property declarations**: `INT64`, `UINT16`, `UINT64`, `FLOAT32`,
`FLOAT64`, `UNICODE_STRING`, `TUPLE`, and inline `ARRAY`/`FIXED_DICT`. SGW always spells
32-bit float as `FLOAT` (never `FLOAT32`) and always reaches an array or dictionary through
a named alias from `alias.xml` rather than declaring one inline. This matters when writing
a `.def` parser: the property-type vocabulary you must handle is the 17 names above, not
the full BigWorld type set. (Method `<Arg>` types draw from a wider pool of aliases — see
the alias section below.)

**Primitive types:**

| Type | Size | Description |
|------|------|-------------|
| `INT8` | 1 byte | Signed 8-bit integer (-128 to 127) |
| `INT16` | 2 bytes | Signed 16-bit integer |
| `INT32` | 4 bytes | Signed 32-bit integer |
| `INT64` | 8 bytes | Signed 64-bit integer |
| `UINT8` | 1 byte | Unsigned 8-bit integer (0 to 255) |
| `UINT16` | 2 bytes | Unsigned 16-bit integer |
| `UINT32` | 4 bytes | Unsigned 32-bit integer |
| `UINT64` | 8 bytes | Unsigned 64-bit integer |
| `FLOAT32` / `FLOAT` | 4 bytes | 32-bit IEEE 754 float |
| `FLOAT64` | 8 bytes | 64-bit IEEE 754 double |
| `STRING` | Variable | Length-prefixed UTF-8 string |
| `WSTRING` / `UNICODE_STRING` | Variable | Length-prefixed UTF-16 string |
| `VECTOR3` | 12 bytes | Three FLOAT32 values (x, y, z) |
| `PYTHON` | Variable | Python-pickled object (arbitrary Python data) |
| `MAILBOX` | Variable | BigWorld entity mailbox reference (used for inter-entity RPC) |

**Composite types:**

| Type | Description |
|------|-------------|
| `ARRAY <of> T </of>` | Variable-length array of elements of type T |
| `TUPLE` | Fixed-length sequence (rarely used in SGW) |
| `FIXED_DICT` | Named-field structure (like a C struct) |

**Special types (defined in alias.xml):**

| Alias | Resolves To | Description |
|-------|-------------|-------------|
| `CONTROLLER_ID` | `INT32` | ID of a BigWorld cell controller |
| `DBID` | `INT64` | Database entity ID |
| `CharacterInfo` | `FIXED_DICT` | Character selection screen data |
| `StatList` | `FIXED_DICT` | 70+ combat stats dictionary |
| `StatUpdate` | `FIXED_DICT` | Single stat change notification |
| `StatUpdateList` | `ARRAY<of>StatUpdate</of>` | Batch stat change list |
| `InvItem` | `FIXED_DICT` | Inventory item representation |
| `MissionStatus` | `FIXED_DICT` | Mission progress tracking |
| `LootItemQuantity` | `FIXED_DICT` | Loot drop entry |
| `NameValuePair` | `FIXED_DICT` | Generic key-value pair |

See `entities/defs/alias.xml` for the complete list of custom type aliases and their FIXED_DICT field definitions.

### Other Property Attributes

| Attribute | Description |
|-----------|-------------|
| `<Default>value</Default>` | Default value when entity is created. For PYTHON types, this can be `{}`, `[]`, `None`, `set()`, etc. |
| `<Persistent>true</Persistent>` | Property is saved to and loaded from the database. |
| `<DatabaseLength>64</DatabaseLength>` | Maximum length for string properties in the database. |
| `<Identifier>true</Identifier>` | Property serves as a unique identifier (like playerName). Used for lookups. Used **twice** in SGW. |
| `<DetailLevel>FAR</DetailLevel>` | Level-of-Detail tier. Properties at lower detail levels are not sent to distant clients. **Never used in SGW** (0 occurrences) — consistent with all 22 `<LoDLevels>` blocks being empty. The LOD code is still linked into the client binary; SGW simply configures it off. |

---

## Method Sections

Methods are remote procedure calls between the distributed components of an entity. Each section defines methods callable on a specific component.

### <CellMethods>

Methods that can be called on the entity's cell component. These run on the CellApp.

```xml
<CellMethods>
    <setTargetID>
        <Exposed/>
        <Arg>  INT32  <ArgName>aTargetID</ArgName>  </Arg>
    </setTargetID>

    <onAttacked>
        <Arg>  MAILBOX  </Arg>  <!-- Cell mailbox of attacker -->
        <Arg>  INT32    </Arg>  <!-- Health change -->
        <Arg>  INT32    </Arg>  <!-- Focus change -->
        <Arg>  UINT8    </Arg>  <!-- EDamageType -->
    </onAttacked>
</CellMethods>
```

**Who can call CellMethods:**
- Other server components (BaseApp, other CellApps) via mailbox calls.
- The owning client, but ONLY if the method has the `<Exposed/>` tag.

### <BaseMethods>

Methods that can be called on the entity's base component. These run on the BaseApp.

```xml
<BaseMethods>
    <DespawnMe>
    </DespawnMe>

    <onDeath/>
</BaseMethods>
```

**Who can call BaseMethods:**
- Other server components via the entity's base mailbox.
- The CellApp hosting the entity's cell component.
- NOT the client (BaseMethods are never exposed to the client).

### <ClientMethods>

Methods that can be called on the entity's client proxy. These execute on the player's machine.

```xml
<ClientMethods>
    <onStatUpdate>
        <Arg> StatUpdateList <ArgName>Stats</ArgName></Arg>
    </onStatUpdate>

    <onEffectResults>
        <Arg> INT32                  <ArgName>SourceID</ArgName>           </Arg>
        <Arg> INT32                  <ArgName>AbilityID</ArgName>          </Arg>
        <Arg> INT32                  <ArgName>EffectID</ArgName>           </Arg>
        <Arg> INT32                  <ArgName>TargetID</ArgName>           </Arg>
        <Arg> UINT8                  <ArgName>ResultCode</ArgName>         </Arg>
        <Arg> ClientEffectResultList <ArgName>ClientEffectResultList</ArgName></Arg>
    </onEffectResults>
</ClientMethods>
```

**Who can call ClientMethods:**
- The server (CellApp or BaseApp). The server serializes the method call and sends it over the network to the client.
- NOT the client itself (client methods are receive-only from the client's perspective).

### The <Exposed/> Tag

The `<Exposed/>` tag on a CellMethod means the client is permitted to call this method directly. When the client invokes an exposed method, it serializes the method index and arguments into a Mercury message and sends it to the CellApp.

```xml
<setCrouched>
    <Exposed/>
    <Arg>  INT8  <ArgName>enabled</ArgName>  </Arg>
</setCrouched>
```

**Security implication:** Exposed methods are the client's attack surface. The server must validate all arguments to exposed methods, because a malicious client can send arbitrary values. Never trust exposed method arguments without validation.

Methods WITHOUT `<Exposed/>` cannot be called by the client. They are server-internal RPC only. If the client tries to call a non-exposed method, the BigWorld engine will reject the call.

### Method Arguments

Method arguments are listed as `<Arg>` elements in declaration order. Arguments can optionally have `<ArgName>` tags for documentation purposes, but the wire protocol uses positional ordering, not names.

```xml
<Arg> INT32   <ArgName>aAbilityId</ArgName>           </Arg>
<Arg> INT32   <ArgName>aTargetId</ArgName>             </Arg>
<Arg> INT32   <ArgName>originatingSubsystemID</ArgName></Arg>
<Arg> PYTHON  <ArgName>aLocation</ArgName>             </Arg>
<Arg> PYTHON  <ArgName>userData</ArgName>              </Arg>
```

The argument order in the .def file is the serialization order on the wire. Arguments are serialized sequentially with no padding, alignment, or field separators -- the receiver must know the types to deserialize correctly.

Methods with no arguments have an empty body:

```xml
<DespawnMe>
</DespawnMe>
```

Or use self-closing syntax:

```xml
<onDeath/>
```

---

## Custom Type Aliases (alias.xml)

The `entities/defs/alias.xml` file defines custom type names that can be used anywhere a type is expected. There are two kinds of aliases:

### Simple Aliases

Map a custom name to a built-in type:

```xml
<CONTROLLER_ID>  INT32  </CONTROLLER_ID>
<DBID>           INT64  </DBID>
<ItemID>         INT32  </ItemID>
```

These are pure documentation aids -- `CONTROLLER_ID` and `INT32` serialize identically.

### FIXED_DICT Aliases

Define structured types with named fields:

```xml
<CharacterInfo> FIXED_DICT
    <Properties>
        <playerId><Type>    INT32    </Type></playerId>
        <name><Type>        WSTRING  </Type></name>
        <extraName><Type>   WSTRING  </Type></extraName>
        <alignment><Type>   INT8     </Type></alignment>
        <level><Type>       INT8     </Type></level>
        <gender><Type>      INT8     </Type></gender>
        <worldLocation><Type> WSTRING </Type></worldLocation>
        <archetype><Type>   INT8     </Type></archetype>
        <title><Type>       INT8     </Type></title>
        <playerType><Type>  INT32    </Type></playerType>
        <playable><Type>    INT8     </Type></playable>
    </Properties>
</CharacterInfo>
```

FIXED_DICT types serialize their fields in declaration order. When a property or method argument uses `CharacterInfo` as its type, the wire format is the concatenation of all the fields' serialized forms, in the order listed.

### ARRAY Aliases

Wrap ARRAY types for convenience:

```xml
<CharacterInfoList>  ARRAY <of> CharacterInfo </of>  </CharacterInfoList>
<StatUpdateList>     ARRAY <of> StatUpdate    </of>  </StatUpdateList>
```

### Nested Types

FIXED_DICT fields can themselves be FIXED_DICT or ARRAY types, creating nested structures:

```xml
<MissionStatus>FIXED_DICT
    <Properties>
        <missionID><Type>   INT32  </Type></missionID>
        <status><Type>      INT8   </Type></status>
        <currentStep><Type> INT8   </Type></currentStep>
        <recievedBy><Type>  INT32  </Type></recievedBy>
        <steps><Type>       ARRAY <of> MissionStepStatus </of>  </Type></steps>
    </Properties>
</MissionStatus>
```

Here `MissionStepStatus` is itself a FIXED_DICT containing an ARRAY of `MissionObjectiveStatus`, which contains an ARRAY of `MissionTaskStatus`. This nesting creates complex but well-defined wire formats.

---

## Enumerations (enumerations.xml)

The `entities/defs/enumerations.xml` file defines named constants used throughout the game:

```xml
<EAbilityFlags>ENUMERATION
    <Type>UINT64</Type>
    <Tokens>
        <Token><Name>AbilityNone</Name>       <Value>0</Value></Token>
        <Token><Name>WeaponBar</Name>         <Value>1</Value></Token>
        <Token><Name>DeploymentBar</Name>     <Value>2</Value></Token>
        <Token><Name>UseWeaponRange</Name>    <Value>4</Value></Token>
        <Token><Name>Toggled</Name>           <Value>8</Value></Token>
    </Tokens>
</EAbilityFlags>
```

Enumerations are used in the Python scripting layer via `Atrea.enums.TokenName` (e.g., `Atrea.enums.WeaponBar`). In the .def files, enum values are serialized as their underlying type (UINT8, INT32, UINT64, etc.) -- the names are for human reference only.

Notable enumerations:
- `EAbilityFlags` (UINT64 bitfield) -- ability capability flags
- `EGender` (UINT8) -- Male=1, Female=2, Other=3
- `ETargetType` (UINT8) -- None, Self, Target, Ground
- `ETargetCollectionParams` (INT32) -- AoE radius and cone parameters
- `ELocales` (UINT32) -- locale IDs for internationalization

---

## How .def Files Map to Python Scripts

Each entity type has corresponding Python scripts that implement the methods declared in its .def file.

### Directory Mapping

> [!WARNING]
> The Python scripting layer described in this section is the **original CME server's**, not
> Cimmeria's. It now lives under `deprecated/python/` and is reference material only —
> active development is Rust under `crates/`. The mapping is documented because the `.def`
> files were authored against it and the argument-ordering conventions carry over, but do
> not expect these scripts to run.

| Component | Python Directory (deprecated) | Script Naming |
|-----------|-----------------|---------------|
| Cell methods | `deprecated/python/cell/` | `EntityName.py` |
| Base methods | `deprecated/python/base/` | `EntityName.py` |
| Common utilities | `deprecated/python/common/` | Various |

For example, `SGWMob.def` declares cell methods like `onGroupMateEnteredCombat` and `addToThreatList`. The original implementations live in `deprecated/python/cell/SGWMob.py`; the Rust equivalents are under `crates/services/src/cell/`.

### Class Structure

Python scripts define a class named after the entity type. The class inherits from `Atrea.CellEntity` (for cell scripts) or `Atrea.BaseEntity` (for base scripts), plus any mixin classes for interfaces.

```python
# deprecated/python/cell/SGWEntity.py
import Atrea
from common.Event import EventParticipant

class SGWEntity(Atrea.CellEntity, EventParticipant):
    def __init__(self):
        Atrea.CellEntity.__init__(self)
        EventParticipant.__init__(self)
```

### Interface Method Implementation

Methods declared in interfaces are implemented in the entity class that uses the interface, or in a dedicated mixin module. For example, `SGWAbilityManager.def` declares `invokeAbility`, but the implementation lives in `deprecated/python/cell/AbilityManager.py` as a class that `SGWBeing` (and by extension `SGWPlayer` and `SGWMob`) inherits from.

### Property Access

Properties declared in .def files are accessible as attributes on `self` in Python:

```python
# Accessing a property declared in SGWCombatant.def
current_health = self.statsCurrent['health']

# Accessing a property declared in SGWBeing.def (interface)
target = self.targetID

# Setting a property triggers network sync if flags require it
self.level = new_level  # If CELL_PUBLIC, other CellApps will see this change
```

### Method Definitions

Methods declared in the .def file must have corresponding Python method definitions with matching argument counts:

```python
# SGWCombatant.def declares:
#   <onAttacked>
#       <Arg>MAILBOX</Arg>  <!-- attacker -->
#       <Arg>INT32</Arg>    <!-- health change -->
#       <Arg>INT32</Arg>    <!-- focus change -->
#       <Arg>UINT8</Arg>    <!-- damage type -->
#   </onAttacked>

# deprecated/python/cell/SGWMob.py (or a combat mixin):
def onAttacked(self, attackerMailbox, healthChange, focusChange, damageType):
    # self is implicit (not counted in .def args)
    # Arguments map positionally to the <Arg> declarations
    ...
```

The `self` parameter is not counted in the .def argument list -- it is implicit. A method with 4 `<Arg>` elements in the .def file has 5 parameters in Python (self + 4 args).

---

## Practical Example: Reading SGWCombatant.def

Let us walk through the `SGWCombatant.def` interface file to understand how a real .def file works.

### Properties Section

```xml
<Alignment>
    <Type>          UINT8           </Type>
    <Default>       0               </Default>
    <Flags>         CELL_PUBLIC     </Flags>
</Alignment>
```

This declares a property called `Alignment`:
- **Type UINT8:** A single unsigned byte (0-255).
- **Default 0:** New entities start with alignment 0.
- **Flags CELL_PUBLIC:** Stored on the CellApp. Other CellApps can read it (for AoI calculations). NOT automatically synced to clients -- the server must use a ClientMethod to notify clients of changes.

```xml
<statsBaseCurrent>
    <Type>          StatList                </Type>
    <Default>       {}                      </Default>
    <Flags>         CELL_PUBLIC             </Flags>
</statsBaseCurrent>
```

This property uses the custom `StatList` type from alias.xml, which is a FIXED_DICT with 70+ fields (coordination, engagement, fortitude, morale, health, focus, etc.). It defaults to an empty dict `{}`. Each field in the StatList is an INT32.

```xml
<entitiesDetectedStealth>
    <Type>          PYTHON          </Type>
    <Flags>         CELL_PRIVATE    </Flags>
    <Default>       {}              </Default>
</entitiesDetectedStealth>
```

This uses `PYTHON` type -- an arbitrary Python object serialized via pickle. It is `CELL_PRIVATE`, so only the CellApp ever sees it. This is common for internal bookkeeping data that does not need a rigid wire format.

### CellMethods Section

```xml
<onAttacked>
    <Arg>       MAILBOX     </Arg>  <!-- Cell mailbox of the entity who attacked us -->
    <Arg>       INT32       </Arg>  <!-- Health change -->
    <Arg>       INT32       </Arg>  <!-- Focus change -->
    <Arg>       UINT8       </Arg>  <!-- EDamageType -->
</onAttacked>
```

This is a CellMethod with 4 arguments. It does NOT have `<Exposed/>`, so the client cannot call it. Only server-side code can invoke `onAttacked` via a mailbox call:

```python
# Server-side Python calling onAttacked on another entity's cell
target.cell.onAttacked(self.cell, healthDelta, focusDelta, damageType)
```

```xml
<setCrouched>
    <Exposed/>
    <Arg>  INT8  <ArgName>enabled</ArgName>  </Arg>
</setCrouched>
```

This IS `<Exposed/>`, so the client can call it. When the player presses the crouch key, the client serializes a method call with one INT8 argument and sends it to the CellApp. The server must validate the argument (is the player in a state where crouching is allowed?) before applying the state change.

### ClientMethods Section

```xml
<onStatUpdate>
    <Arg> StatUpdateList <ArgName>Stats</ArgName></Arg>
</onStatUpdate>
```

This is a ClientMethod -- the server calls it to notify the client. When the server changes a combatant's stats, it invokes `onStatUpdate` with a `StatUpdateList` (an array of `StatUpdate` FIXED_DICTs, each containing StatId, Min, Current, Max). The BigWorld engine serializes this and sends it to the client.

### Tracing to Python

The `setCrouched` method declared in `SGWCombatant.def` is implemented in the entity's Python cell script. Since SGWCombatant is an interface used by SGWBeing (which is used by SGWPlayer and SGWMob), the implementation might be in:

1. `deprecated/python/cell/SGWPlayer.py` -- if it is player-specific
2. `deprecated/python/cell/SGWMob.py` -- if it is mob-specific
3. A combat mixin class -- if shared between players and mobs

The Python implementation would look like:

```python
def setCrouched(self, enabled):
    # Validate: is the player alive? Not in a vehicle? Not stunned?
    if not self._canChangeCrouch():
        return

    # Apply the state change
    if enabled:
        self.setStateField(STATE_CROUCHED, True)
    else:
        self.setStateField(STATE_CROUCHED, False)

    # Notify all clients via the SGWBeing interface's ClientMethod
    self.allClients.onStateFieldUpdate(self.bStateField)
```

The `self.allClients.onStateFieldUpdate(...)` call invokes the ClientMethod declared in the SGWBeing interface's `<ClientMethods>` section, which sends the updated state field to all clients that have this entity in their Area of Interest.

---

## The <Volatile> Section

The `<Volatile>` section declares which spatial properties (position, yaw, pitch, roll) are managed by BigWorld's volatile update system rather than regular property updates.

```xml
<Volatile>
    <position/>
    <yaw/>
    <pitch/>
    <roll/>
</Volatile>
```

Volatile properties use BigWorld's optimized position update protocol with configurable update rates and interpolation, rather than the standard property sync mechanism. They are handled automatically by the engine and do not appear in the `<Properties>` section.

In SGWEntity.def:

```xml
<Volatile>
    <position/>
    <yaw/>
    <pitch/>
    <roll/>
</Volatile>
```

This means position and orientation are volatile -- the engine sends updates at a rate determined by distance (Level of Detail) rather than only when explicitly changed.

**Seven files carry a `<Volatile>` block**, not one. Besides `SGWEntity.def` (the full
position/yaw/pitch/roll set above), the interfaces `Communicator`, `ContactListManager`,
`DistributionGroupMember`, `Lootable` and `OrganizationMember` each declare an *empty*
`<Volatile></Volatile>`, and `interfaces/SGWBeing.def` declares `<yaw/>` alone. The empty
blocks are inert boilerplate. The `SGWBeing` yaw-only block is the one worth knowing about
if you are reconciling orientation handling between entity types.

---

## The <ServerOnly /> Tag

```xml
<ServerOnly />
```

When present at the top level, this tag indicates the entity has no client-side representation. The client never receives create/update messages for this entity type.

**Exactly 10 of the 18 entity defs are `<ServerOnly />`:**

`SGWEntity` (the root, always subclassed), `SGWBlackMarket`, `SGWChannelManager`,
`SGWCoverSet`, `SGWEscrow`, `SGWPlayerGroupAuthority`, `SGWPlayerRespawner`,
`SGWSpaceCreator`, `SGWSpawnRegion`, `SGWSpawnSet`.

The eight that are **not** server-only — and therefore the only types the client can ever
be asked to instantiate — are `Account`, `SGWSpawnableEntity`, `SGWBeing`, `SGWPlayer`,
`SGWGmPlayer`, `SGWMob`, `SGWPet`, and `SGWDuelMarker`. That is a useful bound when
debugging entity-creation messages: a create for anything outside this set is a server bug.

---

## The entities.xml Registry

The `entities/entities.xml` file registers all entity types known to the engine:

```xml
<root>
    <SGWSpawnableEntity/>
    <SGWBeing/>
    <SGWPlayer/>
    <SGWGmPlayer/>
    <SGWMob/>
    <SGWPet/>
    <SGWDuelMarker/>
    <SGWBlackMarket/>
    <Account/>
    <SGWEntity/>
    <SGWPlayerGroupAuthority/>
    <SGWSpaceCreator/>
    <SGWSpawnRegion/>
    <SGWSpawnSet/>
    <SGWPlayerRespawner/>
    <SGWCoverSet/>
    <SGWEscrow/>
    <SGWChannelManager/>
</root>
```

Each tag corresponds to a `.def` file in `entities/defs/`. The order in this file determines the **entity type index** used in the wire protocol. When the server sends a "create entity" message, it includes the type index, and the client uses this index to look up the .def file and instantiate the correct entity class.

**Critical:** If the order in `entities.xml` does not match the order the client expects, entity creation will use the wrong type, causing crashes or bizarre behavior.

---

## Common Patterns and Conventions

### Boolean Properties

BigWorld does not have a native boolean type. Booleans are represented as `INT8` (or occasionally `UINT8`) with values 0 (false) and 1 (true). Some .def files use `FALSE` and `TRUE` as default values:

```xml
<bIsWarmingUp>
    <Type>          INT8                </Type>
    <Flags>         CELL_PRIVATE        </Flags>
    <Default>       FALSE               </Default>
</bIsWarmingUp>
```

The `b` prefix is a naming convention indicating a boolean property.

### PYTHON Type for Flexible Data

When a property needs to hold data whose structure varies at runtime (dictionaries, lists, mixed types), the `PYTHON` type is used. This serializes the data using Python's pickle protocol. It is flexible but less efficient and harder to validate than typed alternatives.

```xml
<pendingRequests>
    <Type>          PYTHON          </Type>
    <Default>       {}              </Default>
    <Flags>         CELL_PRIVATE    </Flags>
</pendingRequests>
```

### MAILBOX for Inter-Entity References

`MAILBOX` properties hold references to other entities' base or cell components. They enable RPC calls between entities:

```xml
<account>
    <Type>          MAILBOX         </Type>
    <Flags>         BASE            </Flags>
</account>
```

A mailbox is not a raw entity ID -- it is a routing address that the BigWorld engine uses to deliver method calls to the correct component on the correct server.

### CONTROLLER_ID for Engine Controllers

`CONTROLLER_ID` (aliased to `INT32`) identifies BigWorld cell controllers -- engine-level objects that manage timers, navigation, visibility, and other recurring behaviors. They are not entity IDs and are not meaningful outside the CellApp that created them.

```xml
<navControllerID>
    <Type>          CONTROLLER_ID       </Type>
    <Flags>         CELL_PRIVATE        </Flags>
    <Default>       0                   </Default>
</navControllerID>
```

A value of 0 means no controller is active.

---

## Reading .def Files Effectively

1. **Start with entities.xml** to see the full list of entity types.
2. **Check the `<Parent>` tag** to understand the inheritance chain. An SGWPlayer inherits everything from SGWBeing, which inherits from SGWSpawnableEntity, which inherits from SGWEntity.
3. **Check `<Implements>`** to find which interfaces are mixed in. SGWPlayer has 11 interfaces -- its effective property and method count is the sum of its own definitions plus all inherited and interfaced definitions.
4. **Read Properties bottom-up.** Start with the interfaces (SGWCombatant, SGWAbilityManager, SGWBeing) to understand the foundational data, then read the entity-specific properties.
5. **Pay attention to Flags.** The flag on a property tells you everything about its network behavior and visibility. A `CELL_PRIVATE` property is irrelevant to the client protocol; an `ALL_CLIENTS` property must be synced to every nearby client.
6. **Look for `<Exposed/>`** on CellMethods. These are the methods the client can call, and they define the attack surface for input validation.
7. **Cross-reference with alias.xml** for any type you do not recognize. If a property uses `StatList` or `CharacterInfo` or `InvItem`, look up the FIXED_DICT definition to understand the actual fields and their types.
8. **Check enumerations.xml** for integer constants. A property of type `UINT8` with values 0-4 probably maps to an enumeration that gives meaningful names to each value.
