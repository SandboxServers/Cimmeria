# CME Framework Layer

> **Last updated**: 2026-03-01
> **RE Status**: Partially documented from Ghidra analysis + codebase references
> **Sources**: Ghidra string analysis (STATUS.md), `docs/how-sgw-works.md`, `data/scripts/`

---

## Overview

Cheyenne Mountain Entertainment (CME) built a custom framework layer on top of BigWorld Technology and Unreal Engine 3. This layer bridges the two engines and adds game-specific systems not present in either middleware platform.

The CME framework is primarily client-side code in `sgw.exe`, but several systems have server-side implications.

## EventSignal Framework

The EventSignal system is CME's central event bus. It connects all subsystems using a publish-subscribe pattern.

### Architecture

```
[EventSignal Bus]
    |
    +-- Event_NetOut_*     (Client -> Server messages)
    +-- Event_NetIn_*      (Server -> Client messages)
    +-- Event_SlashCmd_*   (Slash command dispatch)
    +-- Event_Action_*     (Input/movement actions)
    +-- Event_Editor_*     (Editor/PIE mode events)
    +-- Event_Option_*     (Settings changes)
    +-- Event_UI_*         (UI dispatch events)
    +-- Event_Property_*   (Property change notifications)
```

### Scale

| Category | Count | Description |
|----------|-------|-------------|
| Total unique event types | 750 | Unique `Event_*` strings in binary |
| `Event_NetOut_*` | 253 | Client-to-server messages |
| `Event_NetIn_*` | 167 | Server-to-client messages |
| `Event_SlashCmd_*` | 256 | Slash command dispatch events |
| `Event_Action_*` | 33 | Player input/movement actions |
| `Event_Editor_*` | 29 | Editor and PIE mode events |
| `Event_Option_*` | 8 | Graphics/audio option changes |
| `Event_UI_*` | 2 | UI system dispatch |
| `Event_Property_*` | 2 | Property change notifications |

Note: the original "954" count included duplicate string references at different addresses. After deduplication, 750 unique event types exist.

### Event_NetOut / Event_NetIn

These events bridge the CME EventSignal system to BigWorld's Mercury protocol:

- `Event_NetOut_*` -- When the client UI triggers an action, an EventSignal is fired. A listener serializes the data and sends it as a Mercury message to the server.
- `Event_NetIn_*` -- When the server sends a Mercury message, a handler deserializes it and fires the corresponding EventSignal for the client to process.

See [Event-Net Mapping](../analysis/event-net-mapping.md) for the cross-reference between event names and .def methods.

### Non-Network EventSignal Categories

The 330 non-network event types break down as follows:

#### Event_SlashCmd (256 types)

The largest non-network category. Each slash command typed by the player fires an `Event_SlashCmd_*` signal, which typically dispatches to the corresponding `Event_NetOut_*` to send it to the server. Subcategories:

| Subcategory | Count | Examples |
|-------------|-------|---------|
| Chat/Social | 37 | `Say`, `Yell`, `Tell`, `SayTeam`, `SaySquad`, `SayCommand`, `ChatJoin`, `ChatLeave`, `ChatIgnore`, `ChatFriend` |
| Mission/Quest | 27 | `MissionAssign`, `MissionAdvance`, `MissionComplete`, `MissionReset`, `MissionAbandon`, `ShareMission` |
| GM/Debug | 52 | `SetGodMode`, `SetInvulnerable`, `Kill`, `Spawn`, `Despawn`, `DebugTarget`, `ForceClientCrash`, `DumpObjects` |
| Give/Set | 38 | `GiveXp`, `GiveItem`, `GiveAbility`, `GiveNaqahdah`, `SetLevel`, `SetHealth`, `SetSpeed`, `SetFlag` |
| Inventory | 15 | `EquipItem`, `UnequipItem`, `UseItem`, `DeleteItem`, `MoveItem`, `RepairItem`, `LootItem`, `RechargeItem` |
| Squad/Team/Org | 25 | `SquadInvite`, `TeamInvite`, `CommandInvite`, `SquadKick`, `TeamLeave`, `CommandPromote` |
| Show/Debug Info | 25 | `ShowFPS`, `ShowNavMesh`, `ShowPosition`, `ShowInventory`, `ShowSpawns`, `ShowMobPaths` |
| Navigation | 6 | `Goto`, `GotoLocation`, `GotoXYZ`, `Teleport` |
| Combat | 8 | `CombatDebug`, `CombatDebugVerbose`, `HealDebug`, `AbilityDebug`, `ToggleCombatLOS`, `TestLOS` |
| Minigame | 8 | `StartMinigame`, `DebugStartMinigame`, `DebugJoinMinigame`, `MinigameComplete` |
| Data Reload | 10 | `Reload`, `LoadConstants`, `LoadAbility`, `LoadMOB`, `LoadMission`, `LoadBehavior` |
| Misc | 5 | `Exit`, `LogOff`, `Unstuck`, `Who`, `DHD` |

#### Event_Action (33 types)

Input-bound player actions. These fire when the player presses keybinds.

| Subcategory | Types |
|-------------|-------|
| Movement | `MoveForward`, `MoveBack`, `StepLeft`, `StepRight`, `TurnLeft`, `TurnRight`, `Jump`, `Walk`, `Crouch`, `ToggleAutorun` |
| Camera | `MouseClick`, `MouseLook`, `ZoomIn`, `ZoomOut` |
| Targeting | `TargetSelf`, `CancelTarget`, `CycleTargetForward`, `CycleTargetBackward`, `CycleFriendlyTargetForward`, `CycleFriendlyTargetBackward`, `TargetPet`, `TargetSquadMember1`-`6`, `TargetPartyPet1`-`6` |

#### Event_Editor (29 types)

Development editor events for the PIE (Play-In-Editor) mode built into the client.

| Subcategory | Types |
|-------------|-------|
| Camera modes | `CameraDefault`, `Camera1stPerson`, `Camera3rdPerson`, `CameraFixed`, `CameraFixedTracking`, `CameraFree` |
| Rendering | `ViewWireframe`, `ViewUnlit`, `ViewLit`, `ShowPerformance`, `ShowFPS`, `ShadowStats`, `ScreenShot` |
| PIE script | `PIEScriptLoad`, `PIEScriptStart`, `PIEScriptTogglePause`, `PIEScriptStep`, `SetPIEScript1Active`, `SetPIEScript2Active` |
| Sequences | `TestSequence`, `SequenceBegin`, `SequenceEnd`, `SequenceInterrupt` |
| Debug | `TogglePhysicsMode`, `ToggleCombat`, `Ghost`, `Walk`, `Use`, `Close` |

#### Event_Option (8 types)

Settings change events: `MasterVolume`, `MusicVolume`, `Resolution`, `WindowedMode`, `DevWindowedMode`, `Rendering`, `CamOptionChanged`, `ShowButtonBinds`.

#### Event_UI (2 types)

High-level UI dispatch: `Event_UI_SlashCommand` (routes text to slash command parser), `Event_UI_BindableAction` (routes keybinds to actions).

#### Event_Property (2 types)

Property observation: `Event_Property_AppearanceLogging`, `Event_Property_SctTextLevel`.

## CME::PropertyNode

The PropertyNode system provides a hierarchical property tree for the client. It extends BigWorld's flat entity property model with a tree structure used for UI binding.

### Known Usage

- Character stats display
- Inventory item properties
- Mission objective tracking
- Organization roster data

### Architecture (from Ghidra RTTI)

PropertyNode is a template-based system in `CME::Detail::PropertyNode` that supports a fixed set of value types via C++ template specialization. The class hierarchy:

```
CME::CountedBaseTempl<int, DefaultCountTypeTraits<int,int>>
  |
  +-- CME::Detail::PropertyNode::PropertyBase      (abstract base, vtable at various addrs)
        |
        +-- CME::Detail::PropertyNode::Property<T>  (typed leaf nodes)
        |     T = char, unsigned char, short, unsigned short,
        |         int, unsigned int, long, unsigned long,
        |         __int64, unsigned __int64, float,
        |         wstring, string, Vector2, Vector3, Vector4,
        |         CME::TextCmd const*
        |
        +-- CME::Detail::PropertyNode::Property<BasicPropertyList<TypeList>>   (ordered list)
        +-- CME::Detail::PropertyNode::Property<BasicPropertyTree<TypeList>>   (named tree)
```

The `TypeList` template parameter encodes all supported types via a recursive type list pattern (`CME::TypeList<T, TypeList<...>>`) terminated by `CME::NullType`.

### Supported Value Types

| MSVC Type Code | C++ Type | Size | Description |
|----------------|----------|------|-------------|
| `D` | `char` | 1 | Signed byte |
| `E` | `unsigned char` | 1 | Unsigned byte |
| `F` | `short` | 2 | Signed 16-bit |
| `G` | `unsigned short` | 2 | Unsigned 16-bit |
| `H` | `int` | 4 | Signed 32-bit |
| `J` | `long` | 4 | Signed 32-bit |
| `K` | `unsigned long` | 4 | Unsigned 32-bit |
| `_J` | `__int64` | 8 | Signed 64-bit |
| `_K` | `unsigned __int64` | 8 | Unsigned 64-bit |
| `_N` | `bool` | 1 | Boolean |
| `M` | `float` | 4 | 32-bit float |
| `VVector2` | `Vector2` | 8 | 2D vector |
| `VVector3` | `Vector3` | 12 | 3D vector |
| `VVector4` | `Vector4` | 16 | 4D vector |
| `V...basic_string<wchar_t>` | `wstring` | var | Wide string (TBB allocator) |
| `V...basic_string<char>` | `string` | var | Narrow string (TBB allocator) |
| `PBVTextCmd@CME` | `CME::TextCmd const*` | 4 | Text command pointer |

### Serialization Format

PropertyNode does **not** have its own wire serialization format. The PropertyNode tree is a client-side UI binding layer. The server populates entity properties via standard BigWorld `DataType::addToStream` serialization (confirmed at `0x00c6fc40`). The client-side PropertyNode tree observes these properties and reflects them into the UI.

Reference counting uses `CME::CountedBaseTempl` with `scalable_free` (TBB allocator) for deallocation. The vtable slot 0 (decompiled at `0x0042e380`, `0x00433c50`, `0x00a4e8f0`, etc.) is the destructor, which chains through PropertyBase and CountedBaseTempl vtables.

### Server Implications

The server does not need to serialize PropertyNode structures directly. It sends standard BigWorld entity properties; the client's PropertyNode layer maps these into the UI tree. No custom wire format is needed.

## CME::SoapLibrary

CME replaced BigWorld's LoginApp with a SOAP/HTTP authentication system. The `SoapLibrary` class handles:

- XML parsing for SOAP requests/responses
- HTTP client/server implementation
- Session token management
- Shard list serialization

The server-side equivalent is the `AuthenticationServer` (`src/authentication/`), which serves SOAP endpoints for `/SGWLogin/UserAuth` and `/SGWLogin/ServerSelection`.

## SpaceViewport System

A CME extension to BigWorld's space system. BigWorld normally sends space data via `spaceData` messages. CME added a `spaceViewportInfo` message that must be sent between `createBasePlayer` and `createCellPlayer` during login.

### Purpose

The SpaceViewport tells the client:
- Which map/zone to load
- Which space the player's viewport tracks
- The viewport index (for multi-viewport support)

This was likely added to support Stargate zone transitions, where players move between distinct worlds (planets, space stations, etc.) that each have different environmental settings.

### Wire Format (from Ghidra at `0x00dda6c0`)

The `spaceViewportInfo` message (Client Interface ID `0x08`) has a 13-byte fixed payload:

```
struct spaceViewportInfoArgs {
    uint32  entityID;       // Player's entity ID
    uint32  entityID2;      // Same as entityID (viewport entity)
    uint32  spaceID;        // Space identifier to load
    uint8   viewportID;     // Viewport index (always 0 for primary)
};
```

### Handler Behavior (`ServerConnection::spaceViewportInfo` at `0x00dda6c0`)

The client handler performs the following logic:

1. Logs `"ServerConnection::spaceViewportInfo: space %d svid %d"` with `spaceID` and `viewportID`
2. Looks up the viewport entry in the viewport table at `this+0xf84` using `viewportID` as key
3. **New viewport** (entry's spaceID is 0):
   - Stores `entityID`, `entityID2`, and `spaceID` in the viewport entry
   - Increments a reference count in the space map at `this+0xf9c`
   - If `entityID2` matches the player's own entity ID (`this+0x16c`), updates the active space chain at `this+0xf90`
   - Inserts `entityID2` into the space-entity map at `this+0xfac`
   - Sets a "space changed" flag at `entry+0xc`
4. **Existing viewport** (entry already has a spaceID):
   - If `entityID2` is 0: **close viewport** -- decrements reference count, calls cleanup callback via vtable at `this+0x168 + 0x30`, clears the entry
   - If `entityID2` is nonzero: **re-use viewport** -- logs a warning if spaceID changed, updates the entry
5. **Close nonexistent viewport**: Logs error `"Server wants us to close nonexistent viewport %d"`

### Related Messages

| Message | Direction | Purpose |
|---------|-----------|---------|
| `spaceViewportInfo` (0x08) | Server -> Client | Set/change/close a viewport |
| `setSpaceViewport` | Client -> Server | Client acknowledges viewport |
| `setSpaceViewportAck` | Server -> Client | Server confirms viewport set |

### Server Implementation

The BaseApp sends `spaceViewportInfo` during the player creation sequence. See [Login Handshake](../protocol/login-handshake.md) Step 4 and [Post-Auth Sequence](../technical/post-auth-sequence.md) Phase 5.

## CookedData System

CME built a data pipeline that converts database content into XML "cooked" data files served to the client. See [Cooked Data Pipeline](cooked-data-pipeline.md) for details.

Key classes (from Ghidra):
- `CookedElementBase` -- Base class for cooked data elements
- `CookedDataCache` -- Client-side cache for received data
- `CookedDataManager` -- Manages loading and versioning

Cooked data types (from `CME::RefCountedObj` RTTI templates):
- `CookedKismetEventSetData` -- Kismet event set definitions
- `CookedKismetEventSequenceData` -- Kismet event sequences
- `CookedCharCreationData` -- Character creation options
- `CookedTextType` -- Localized text strings
- `CookedErrorTextType` -- Error message text
- `CookedSpecialWordsType` -- Chat filter / special word list

## Atrea Script Editor

CME built a visual scripting tool called the "Atrea Script Editor" for creating mission and effect scripts. This is part of the ServerEd Qt tool.

### Pipeline

```
.script files (data/scripts/)    -- Visual node graphs (XML source)
        |
        | compiled by scriptcompiler.cpp
        v
.py files (python/cell/)         -- Auto-generated Python
```

### Script File Format

Each `.script` file is XML with root element `<Script>` containing these attributes:

| Attribute | Description | Example |
|-----------|-------------|---------|
| `Version` | Script format version | `"1.4"` |
| `DatasetVersion` | Game data version | `"1.2"` |
| `Module` | Python module path | `"Castle_CellBlock.HackTheRings"` |
| `Type` | Script category | `"Mission"`, `"Effect"`, `"Level"` |
| `NextId` | Next available node ID | Auto-incremented integer |

Child elements:
- `<Node>` -- A script node with `Id`, `Ref` (type), `X`/`Y` (editor position)
  - `<Property>` -- Node configuration with `Name` and `Value` attributes
  - `<Port>` -- Connection point with `Name` and `Flags` attributes
- `<Connection>` -- Edge between ports: `OutNode`, `OutPort`, `InNode`, `InPort`
- `<Comment>` -- Editor annotation with `Id`, `Text`, `X`, `Y`, `Width`, `Height`, `Color`

Port Flags: `0` = output/data, `1` = input (constant), `2` = input (connected)

### Script Categories

| Type | Directory | Count | Purpose |
|------|-----------|-------|---------|
| Mission | `data/scripts/missions/` | 16 | Mission logic and quest flow |
| Level | `data/scripts/spaces/` | 10 | Space/zone initialization and events |
| Effect | `data/scripts/effects/` | 4 | Combat effect behavior |

Total: 30 scripts, 615 nodes, 906 connections.

### Node Types (78 unique types across 5 categories)

#### Event Trigger Nodes (16 types)

These are entry points that fire when game events occur. All have an `Out` port for execution flow and contextual data ports.

| Node Type | Uses | Description | Key Ports |
|-----------|------|-------------|-----------|
| `Event_EntityInteract` | 44 | Player interacts with entity | `Out`, `Player`, `Target`, `Tag`, `Template Name` |
| `Event_DialogChoice` | 22 | Player clicks dialog button | `Out`, `Player`, `Button Id`, `Dialog Id` |
| `Event_GenericRegion` | 20 | Player enters/exits region trigger | `Out`, `Player` |
| `Event_Dead` | 10 | Entity dies | `Out`, `Player` |
| `Event_Loaded` | 5 | Script/space loaded for player | `Out`, `Player` |
| `Event_Effect` | 4 | Effect lifecycle events | `Effect Init`, `Effect Removed`, `Pulse Begin`, `Pulse End` |
| `Event_Item` | 4 | Item-related trigger | `Out`, `Player` |
| `Event_Teleport` | 2 | Teleport event | `Out`, `Player` |
| `Event_ScriptLoaded` | 2 | Script initialization | `Out` |
| `Event_MissionUpdate` | 2 | Mission state change | `Out`, `Player` |
| `Event_DialogSetMap` | 2 | Dialog set mapping | `Out` |
| `Event_Dialog` | 1 | Dialog opened/closed | `Out`, `Player` |
| `Event_Stargate` | 1 | Stargate activation | `Out`, `Player` |
| `Event_MissionStepUpdate` | 1 | Mission step state change | `Out`, `Player` |
| `Event_MissionObjectiveUpdate` | 1 | Objective state change | `Out`, `Player` |
| `Event_MissionTaskUpdate` | 1 | Task state change | `Out`, `Player` |

#### Action Nodes (50 types)

Perform game actions when triggered via their `In` port. Most have an `Out` port for flow continuation.

| Node Type | Uses | Description | Key Properties/Ports |
|-----------|------|-------------|---------------------|
| `Act_Log` | 52 | Debug log message | `In`, `Out` |
| `Act_UpdateMission` | 37 | Change mission state | `Mission Id`, `Accept`/`Complete`/`Fail`/`Abandon` |
| `Act_GetMissionStep` | 31 | Check mission step status | `Mission Id`, `Step Id`, `Active`/`Completed`/`Not Started` |
| `Act_RingTransportDlg` | 27 | Show ring transport UI | `Region Id`, `Player`, `Successful`/`Failed` |
| `Act_GetMission` | 26 | Check mission status | `Mission Id`, `Active`/`Completed`/`Not Accepted`/`Failed` |
| `Act_Dialog` | 25 | Show NPC dialog | `Dialog Set`, `Player`, `NPC` |
| `Act_AdvanceMission` | 24 | Advance mission step | `Mission Id`, `Step Id` |
| `Act_GetEntity` | 20 | Find entity by tag | `Tag`, `Template Tag`, `Entity`/`Entity Id`/`Found`/`Failed` |
| `Act_GiveItems` | 18 | Give item to player | `Design`, `Quantity`, `Bag`, `Player` |
| `Act_UpdateInteraction` | 14 | Set/unset interaction type on entity | `Entity`, `Interaction Type`, `Set`/`Unset` |
| `Act_AddDialog` | 12 | Add dialog to NPC | Properties similar to `Act_Dialog` |
| `Act_Sequence` | 11 | Play animation sequence | `Sequence Id`, `Source`, `Target`, `Impact Time`, `View Type` |
| `Act_SysMsg` | 9 | Display system message | `In`, `Out` |
| `Act_Gate` | 7 | Flow gate (conditional pass-through) | `Active`, `In`, `Out` |
| `Act_UpdateMissionObjective` | 7 | Complete/fail objective | `Mission Id`, `Objective Id`, `Complete`/`Fail` |
| `Act_GetProperty` | 7 | Read entity property | `In`, `Out`, `Value` |
| `Act_RemoveDialog` | 7 | Remove dialog from NPC | |
| `Act_QRDamage` | 6 | Apply QR damage | `Base Damage`, `Damage Type`, `Stat Id`, `Ranged`, `Remaining Damage` |
| `Act_Teleport` | 5 | Teleport entity | `World Name`, `Destination`, `Target` |
| `Act_RemoveItems` | 5 | Remove items from player | `Design`, `Quantity` |
| `Act_Minigame` | 5 | Start minigame | `Minigame`, `Difficulty`, `Display Name`, `Won`/`Lost`/`Failed` |
| `Act_Timer` | 4 | Delayed execution | `In`, `Out` |
| `Act_MoveTo` | 4 | Move entity to position | `Entity`, `Position`, `Completed`/`Failed` |
| `Act_UpdateProperty` | 4 | Set entity property | `In`, `Out` |
| `Act_GetMissionObj` | 8 | Get mission objective info | `Mission Id`, `Objective Id` |
| `Act_MulInt` | 3 | Multiply integers | `A`, `B`, `Output` |
| `Act_DivInt` | 3 | Divide integers | `A`, `B`, `Output` |
| `Act_GetLocation` | 3 | Get entity position | `Entity`, `Position`, `Rotation` |
| `Act_FindItems` | 3 | Search player inventory | `Player`, `Design` |
| `Act_AddInt` | 2 | Add integers | `A`, `B`, `Output` |
| `Act_Spawn` | 2 | Spawn entity | |
| `Act_GetStat` | 2 | Read entity stat | `Stat Id`, `Entity`, `Current`/`Max`/`Min`/`Base` |
| `Act_Despawn` | 2 | Remove entity | |
| `Act_Ability` | 2 | Invoke ability | |
| `Act_LockMovement` | 2 | Lock/unlock player movement | |
| `Act_GenerateThreat` | 2 | Add threat to entity | |
| Others | 15 | `SubInt`, `Kill`, `AddEffect`, `RemoveEffect`, `CheckEffect`, `SetCombatState`, `ChangeStat`, `SetStat`, `GetAmmoStat`, `SetActiveSlot`, `UpdateMissionStep`, `GetDistance`, `AddComponent`, `DelComponent` |

#### Variable Nodes (7 types)

Data providers that output constant or stored values.

| Node Type | Uses | Properties |
|-----------|------|-----------|
| `Var_Int` | 26 | `Value`, `Variable Name`, `Global Variable`, `Persistent` |
| `Var_Player` | 11 | Outputs the current player entity |
| `Var_Vec3` | 8 | `Value` (format: `"x,y,z"`) |
| `Var_String` | 4 | `Value`, `Variable Name`, `Global Variable`, `Persistent` |
| `Var_Entity` | 1 | Entity reference |
| `Var_Float` | 1 | `Value` |
| `Var_Bool` | 1 | `Value` |

#### Comparison Nodes (3 types)

| Node Type | Uses | Ports |
|-----------|------|-------|
| `Cmp_Int` | 14 | `A`, `B`, `In`, `A == B`, `A != B`, `A < B`, `A > B` |
| `Cmp_Str` | 4 | `A`, `B`, `In`, `A == B`, `A != B` |
| `Cmp_Float` | 1 | `A`, `B`, `In`, `A == B`, `A != B`, `A < B`, `A > B` |

#### Other Nodes (2 types)

| Node Type | Uses | Description |
|-----------|------|-------------|
| `Counter_Int` | 3 | Integer counter with increment/decrement |
| `EffectParam_Int` | 4 | Read effect parameter by name (`Parameter Name` property) |

### Connection Model

Connections are directed edges from output ports to input ports:

```xml
<Connection OutNode="1" OutPort="Out" InNode="2" InPort="In"/>
```

- **Execution flow**: `Out` -> `In` ports chain node execution order
- **Data flow**: Named output ports (e.g., `Player`, `Entity`, `Value`) feed into named input ports
- **Branch flow**: Conditional outputs (e.g., `Active`, `Completed`, `Won`, `Failed`) route execution based on state
- A single output port can connect to multiple input ports (fan-out)
- An input port typically receives one connection

### Example: Mission Script Flow

From `HackTheRings.script`:
```
Event_EntityInteract ("Ring switch used")
    |
    +--Out--> Act_GetMissionStep (Mission 640, Step 2120)
    |              |
    |              +--Active--> Act_Minigame ("Livewire", Difficulty 1)
    |              |                |
    |              |                +--Won--> Act_AdvanceMission (Step 2215)
    |              |
    |              +--Completed--> Act_RingTransportDlg (Region 1)
```

## Dual UI System

SGW ships with two UI rendering systems, likely reflecting a mid-development transition:

| System | Technology | Classes | Usage |
|--------|-----------|---------|-------|
| CEGUI | Open-source C++ UI + Lua | 438 classes | Original UI framework |
| Scaleform/GFx | Flash/ActionScript | 271 classes | Newer animated UI |

### Server Implications

The server doesn't need to know which UI system the client uses. Both systems receive the same EventSignal events and BigWorld entity data.

## CME Network Extensions

CME modified BigWorld's networking in several ways:

| Extension | Standard BigWorld | CME/SGW |
|-----------|------------------|---------|
| Encryption | Blowfish | AES-256-CBC + HMAC-MD5 |
| Authentication | Mercury LoginApp | HTTP/SOAP |
| Session key | 4-byte SessionKey | 64-char hex (256-bit) |
| SpaceViewport | Not present | Custom message for zone loading |
| Minigame server | Not present | Separate TCP server for minigames |

## CME:: Class Catalog (from RTTI)

### Framework Classes (28 unique)

Classes in the `CME::` namespace identified from RTTI type descriptors (`.?AV...@CME@@` strings) in `sgw.exe`:

| Class | Category | Description |
|-------|----------|-------------|
| `CME::BasicPropertyList` | Property System | Ordered list container for PropertyNode |
| `CME::BasicPropertyTree` | Property System | Named-key tree container for PropertyNode |
| `CME::CountedBaseTempl` | Memory | Reference-counted base (template, int-based) |
| `CME::Detail::DefaultCountTypeTraits` | Memory | Count type traits for CountedBaseTempl |
| `CME::Detail::PropertyNode` | Property System | PropertyNode namespace/container |
| `CME::Detail::PropertyNode::Property<T>` | Property System | Typed leaf node (16 type specializations) |
| `CME::Detail::PropertyNode::PropertyBase` | Property System | Abstract base class for property nodes |
| `CME::ErrorType` | Core | Error result type (used by mutex, thread operations) |
| `CME::EventSignal` | Events | Namespace for event signal system |
| `CME::EventSignal::Callback` | Events | Base callback interface |
| `CME::EventSignal::CallbackImpl<E>` | Events | Template callback for specific event type |
| `CME::EventSignal::MemberCallback` | Events | Member function callback binding |
| `CME::EventSignal::NoSubject` | Events | Subject-less event source |
| `CME::EventSignal::SubjectEvent` | Events | Subject-bound event type |
| `CME::Exception` | Core | Base exception class |
| `CME::FMallocCME` | Memory | Custom memory allocator (Unreal FMalloc override) |
| `CME::GenericEvent` | Events | Type-erased event |
| `CME::InternalTextCmd` | Text Commands | Internal text command implementation |
| `CME::NullType` | Meta | TypeList terminator |
| `CME::RefCountedObj<T>` | Memory | Reference-counted wrapper for data objects |
| `CME::ServerSource<N,K,V,R,Z>` | CookedData | Server data source (template: category, key, value, ref, zip) |
| `CME::SoapLibrary` | Auth | SOAP/HTTP client/server |
| `CME::TextCmd` | Text Commands | Text command base class |
| `CME::TextCommandManager` | Text Commands | Slash command registry |
| `CME::ThreadInfo` | Threading | Thread metadata for Win32ThreadEx |
| `CME::TypeList` | Meta | Compile-time type list (recursive template) |
| `CME::Win32ReadWriteMutex` | Threading | Reader-writer lock with recursion tracking |
| `CME::Win32ThreadEx` | Threading | Extended Win32 thread wrapper |
| `CME::ZipStorage<N,K,V,R>` | CookedData | Compressed data storage |

### CME-Managed Data Types (23 unique)

Game data types wrapped in `CME::RefCountedObj` and managed by `CME::ServerSource`/`CME::ZipStorage`:

| Data Type | Category | Description |
|-----------|----------|-------------|
| `Ability` | Combat | Ability definitions |
| `BehaviorEventData` | AI | NPC behavior event data |
| `SGW::Blueprint` | Crafting | Crafting blueprint definitions |
| `CookedCharCreationData` | Character | Character creation options |
| `CookedErrorTextType` | Localization | Error message text |
| `CookedKismetEventSequenceData` | Scripting | Kismet event sequence data |
| `CookedKismetEventSetData` | Scripting | Kismet event set definitions |
| `CookedSpecialWordsType` | Chat | Chat filter / special words |
| `CookedTextType` | Localization | Localized text strings |
| `CoverNodeUserCacheData` | Combat | Cover system user data cache |
| `DBEffect` | Combat | Effect definitions from database |
| `DBGateInfo` | World | Stargate address/connection info |
| `DBInvContainer` | Inventory | Inventory container definitions |
| `DBInvItem` | Inventory | Inventory item definitions |
| `Dialog` | Quest | NPC dialog definitions |
| `InteractionSet` | Interaction | NPC interaction set definitions |
| `Mission` | Quest | Mission definitions |
| `SGW::AppliedScience` | Progression | Applied science skill data |
| `SGW::Discipline` | Progression | Discipline/class data |
| `SGW::InteractionData` | Interaction | Interaction data |
| `SGW::RacialParadigm` | Progression | Racial paradigm data |
| `SGW::WorldInfo` | World | World/zone metadata |
| `SGWDeferredRTCanvas` | Rendering | Deferred render target canvas |

## Known CME String Prefixes in Binary

From Ghidra analysis, the following CME-specific string prefixes have been identified:

| Prefix | Count | Description |
|--------|-------|-------------|
| `CME::` | 28 | CME namespace framework classes |
| `Atrea` | ~20+ | Atrea-branded components |
| `SGW` | ~100+ | Game-specific classes |
| `Event_NetOut_` | 253 | Client-to-server event signals |
| `Event_NetIn_` | 167 | Server-to-client event signals |
| `Event_SlashCmd_` | 256 | Slash command event signals |
| `Event_Action_` | 33 | Input action event signals |
| `Event_Editor_` | 29 | Editor/PIE event signals |
| `Event_Option_` | 8 | Option change event signals |
| `Event_UI_` | 2 | UI dispatch event signals |
| `Event_Property_` | 2 | Property change event signals |

## Related Documents

- [How SGW Works](../how-sgw-works.md) -- Technology stack overview
- [Cooked Data Pipeline](cooked-data-pipeline.md) -- Data baking system
- [BigWorld Architecture](bigworld-architecture.md) -- Base engine architecture
- [Event-Net Mapping](../analysis/event-net-mapping.md) -- Event to .def method mapping
- [Post-Auth Sequence](../technical/post-auth-sequence.md) -- SpaceViewport wire format details

## TODO

- [x] Reverse engineer CME::PropertyNode serialization format -> Client-side UI binding layer only; no custom wire format. Uses standard BW `DataType::addToStream`. Template-based type system with 16 value types + List/Tree containers. See PropertyNode section above.
- [x] Catalog all CME:: class names from RTTI in Ghidra -> 28 framework classes + 23 data types identified from RTTI `.?AV...@CME@@` strings. See CME:: Class Catalog section above.
- [x] Document the Atrea Script Editor node types and connections -> 78 node types across 5 categories (16 events, 50 actions, 7 variables, 3 comparisons, 2 other). 30 scripts with 615 nodes and 906 connections. See Atrea Script Editor section above.
- [x] Identify all SpaceViewport parameters from Ghidra -> 13-byte message: `{uint32 entityID, uint32 entityID2, uint32 spaceID, uint8 viewportID}`. Handler at `0x00dda6c0` supports create/update/close operations. See SpaceViewport section above.
- [ ] Document the minigame server protocol (TCP-based)
- [x] Map the ~534 non-network EventSignal types -> Actual count is 330 unique non-network types (750 total, not 954). Largest category: `Event_SlashCmd_` (256), then `Event_Action_` (33), `Event_Editor_` (29). See EventSignal Framework section above.
- [x] ~~Determine if CME modified BigWorld's entity serialization~~ -> NO. Universal RPC dispatcher at `0x00c6fc40` uses standard BW `DataType::addToStream`. See `findings/combat-wire-formats.md`
