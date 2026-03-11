# Address Map — Key Locations in SGW.exe

> **Last updated**: 2026-03-08
> **Binary**: SGW.exe (32-bit x86 PE, MSVC 8.0 / VC80)
> **Image base**: `0x00400000`

---

## Overview

Key virtual addresses, vtables, global variables, and important functions discovered during reverse engineering. All addresses are virtual addresses as loaded by Ghidra.

## Important Globals

| Address | Name | Type | Notes |
|---------|------|------|-------|
| `0x01ef244c` | `g_EntityManager` | `EntityManager*` | Singleton — set in `BW_client_entity_manager` constructor |
| `0x01ea5778` | `GMalloc` | `FMalloc*` | UE3 global allocator — points to FMallocCME instance |
| `0x01ee1254` | `GEngine` | `UEngine*` | UE3 engine singleton (RVA `0x01AE1254`) |
| `0x01ef134c` | `GEditor` | `UEditorEngine*` | Set when GIsEditor=1 (RVA `0x01AF134C`) |
| `0x01ef2e74` | `GApp` | `void*` | Application pointer (RVA `0x01AF2E74`) |
| `0x01eadbc0` | `FName::GNames` | `TArray<FNameEntry*>*` | Global name table (RVA `0x01ACADE0`; add image base `+0x400000` for note below) |
| `0x01edc69c` | `UObject::GObjObjects` | `TArray<UObject*>*` | Global object array (RVA `0x01ADC69C`) |
| `0x01ead7ac` | `GIsEditor` | `UBOOL` | Editor mode flag |
| `0x01ead7b0` | `GIsUCC` | `UBOOL` | Commandlet mode flag |
| `0x01ead7bc` | `GIsClient` | `UBOOL` | Client mode flag |
| `0x01ead7c0` | `GIsServer` | `UBOOL` | Server mode flag |
| `0x01eb0830` | `GIsGame` | `UBOOL` | Game mode flag (CME addition, not in stock UE3 2004) |
| `0x01ee435c` | BigWorld package init guard | `DWORD` | Bit flags: bit 0=package created, bit 1=callback installed |
| TODO | `g_ConnectionModel` | `ServerConnection*` | Main server connection object |
| TODO | `g_ScriptManager` | `ScriptManager*` | Python script engine |

## Key Vtables

Vtables identified via RTTI (script 01) or manual analysis.

| Address | Class | vfunc Count | Notes |
|---------|-------|-------------|-------|
| See `BW_client_entity_manager` | `EntityManager` | — | Dual vtable: ServerMessageHandler + FCallbackEventDevice |
| `0x018014f4` | `UBWNetDriver` | 3 ifaces | Primary + UObject iface (0x018014ec) + FNetObjNotify (0x018014d8) |
| `0x0180167c` | `UBWConnection` | 2 ifaces | Primary + UObject iface (0x01801670) |
| `0x01895e3c` | `ABigWorldEntity` | — | Extends AActor (0x0183c40c), size 0x1C4 |
| `0x018caea4` | `UBigWorldInfo` | — | Extends UObject, size 0x44 |
| TODO | `ServerConnection` | — | Mercury connection to server |
| TODO | `Entity` | — | Base entity class (BigWorld) |
| TODO | `CME::EventSignal` | — | CME event dispatch (client-side UI bus only) |

## Core Architecture Functions — Phase 2 Decompiled

### Universal RPC Dispatcher

| Address | Function | Notes |
|---------|----------|-------|
| `0x00c6fc40` | Universal RPC dispatcher | ALL outgoing entity method calls route here |
| `0x00dd6a60` | `ServerConnection_startEntityMessage` | Writes cell method header: `methodID \| 0x80` |
| `0x00dd6980` | `ServerConnection_startProxyMessage` | Writes base method header: `methodID \| 0xC0` |

### Entity Creation

| Address | Function | Notes |
|---------|----------|-------|
| `0x00dddca0` | `ServerConnection_createBasePlayer` | 4B entityID + 2B typeID + property stream |
| `0x00dda2e0` | `ServerConnection_createCellPlayer` | 4B skip + 4B spaceID + 12B Vec3 pos + property stream |

### Entity Manager (entity_manager.cpp — CME modified BW 1.9.1)

| Address | Function | Notes |
|---------|----------|-------|
| `0x00dd3330` | `EntityManager::EntityManager` | Constructor — dual vtable (ServerMessageHandler + FCallbackEventDevice), singleton at `0x01ef244c` |
| `0x00dd2270` | `EntityManager::onEntityCreate` | Line 480/529: creates BW entity, scales position ×100, calls RadiansToRotator |
| `0x00dd1650` | `EntityManager::onEntityMoveWithError` | Scales pos/vel ×100, converts rotation, handles sentinel "use current" values, delegates to ApplyTransform |
| `0x00dd09e0` | `EntityManager::createEntity` | Line 2619: allocates entity, calls GameEntityBase::Init (0x00e685e0) |
| `0x00dd0b00` | `EntityManager::PostLoadMap` | Callback 0x32: fires Event_Level_PostLoad |
| `0x00dd3150` | `EntityManager::LevelRemovedFromWorld` | Callback 0x30: fires Event_Map_Unloaded |
| `0x00dd0d00` | `BW_client_entity_manager_1` area | Entity method dispatch |
| `0x00dd2900` | `BW_client_entity_manager_5` | Entity leave AoI — decrements refcount, cleanup |
| `0x00dd27f0` | `BW_client_entity_manager_4` area | Entity enter AoI — increments refcount, enterWorld |
| `0x00dd1b10` | `BW_client_entity_manager_6` | Entity position/movement update |
| `0x00dd1d00` | `EntityManager::enterWorld` | Entity enter world callback |

### EntityDescription Parsing

| Address | Function | Notes |
|---------|----------|-------|
| `0x01593cd0` | `EntityDescription_parse` | Opens .def file, handles Parent recursion |
| `0x01593600` | `EntityDescription__unknown_01593600` | Parse dispatch: Implements→Properties→Methods |
| `0x015924a0` | `EntityDescription_parseProperties` | Property ID assignment (sequential, excludes EDITOR_ONLY) |
| `0x01594f60` | `MethodDescription_parse` | Method signature parsing (Args, ArgNames, Exposed) |
| `0x015974a0` | `DataDescription_parse_2` | Property type + flags + default value parsing |
| `0x015959c0` | `DataDescription_parse_1` | Property flag string → bitmask conversion |
| `0x01593420` | FUN_01593420 | ClientMethods parsing (called from parse dispatch) |
| `0x015934c0` | FUN_015934c0 | CellMethods parsing (called from parse dispatch) |
| `0x01593560` | FUN_01593560 | BaseMethods parsing (called from parse dispatch) |

### Property Change

| Address | Function | Notes |
|---------|----------|-------|
| `0x015652d0` | `FNetworkPropertyChange__vfunc_0` | Writes property change to stream (4B header + values) |

### Event Signal Registration (CME — client-side only)

| Address | Event Name | Notes |
|---------|------------|-------|
| `0x00cb7d90` | `register_NetOut_UseAbility` | Returns string "Event_NetOut_UseAbility" |
| `0x00d771e0` | `register_NetIn_onEffectResults` | Returns string "Event_NetIn_onEffectResults" |
| `0x00d7f520` | `register_NetIn_TimerUpdate` | Returns string "Event_NetIn_TimerUpdate" |
| `0x00d86620` | `register_NetIn_onStatUpdate` | Returns string "Event_NetIn_onStatUpdate" |
| `0x00d86c10` | `register_NetIn_onStatBaseUpdate` | Returns string "Event_NetIn_onStatBaseUpdate" |
| `0x00d7d300` | `register_NetIn_onContainerInfo` | Returns string "Event_NetIn_onContainerInfo" |

**Note**: Event registration functions simply return a name string. They do NOT contain serialization logic. The actual network serialization is handled by the universal RPC dispatcher at `0x00c6fc40`. See `docs/reverse-engineering/findings/combat-wire-formats.md` for details.

## FMallocCME — Custom Memory Allocator

CME's 32-bit adaptation of Epic's FMallocTBB (64-bit only in stock UE3).
Wraps Intel TBB `scalable_malloc` with 24-byte overhead for manual 16-byte alignment.
Hard-asserts on OOM (`check(Ptr)` at FMallocCME.h:37) instead of returning NULL.

| Address | Function | Notes |
|---------|----------|-------|
| `0x004198f0` | `FMallocCME::Malloc` | `scalable_malloc(size+0x18)` + 16-byte align, `check(Ptr)` on OOM |
| `0x004198b0` | `FMallocCME::Free` | Reads raw ptr from alignment header → `scalable_free(raw)` |
| `0x00419810` | `FMallocCME::Realloc` | Does `Malloc + memcpy + Free` (doesn't use `scalable_realloc`) |
| `0x00416660` | Allocator bootstrap | Allocates 4B via CRT malloc, stamps FMallocCME vtable, skips thread-safe proxy (TBB is thread-safe) |
| `0x017f8e8c` | FMallocCME vtable | 20 slots: 3 real (Malloc/Free/Realloc), Init returns TRUE, rest stubs |
| `0x00419950` | `FMallocThreadSafeProxy` | Wraps any FMalloc with CRITICAL_SECTION (0x24 bytes, vtable 0x017F8F7C). Skipped for TBB. |
| `0x00419923` | OOM assert location | `check(Ptr)` — patched by Atrea MallocOOMSoftFail to return NULL |

### Allocation Layout (24-byte overhead per alloc)
```
scalable_malloc(size + 0x18) returns [raw_ptr]
                                      |
[  ...padding...  ][orig_size][raw_ptr][  aligned user data  ]
                    at -0x8    at -0x4   <-- returned to caller
                                         (16-byte aligned via AND 0xFFFFFFF0)
```

### TBB Import Thunks
| Address | Function | IAT Slot |
|---------|----------|----------|
| `0x00457e00` | `scalable_malloc` thunk | `JMP [0x017f0394]` (tbbmalloc.dll) |
| `0x00457dfa` | `scalable_free` thunk | `JMP [0x017f0390]` (tbbmalloc.dll) |
| `0x0150c7ec` | `scalable_realloc` thunk | `JMP [0x017f0398]` (tbbmalloc.dll) |
| `0x004162f0` | `scalable_free` trampoline | `JMP 0x00457dfa` (redirect to thunk) |

### CRT Import IAT Slots (for allocator replacement)
| IAT Address | Function | DLL |
|-------------|----------|-----|
| `0x017ef990` | `malloc` | MSVCR80.dll |
| `0x017ef994` | `free` | MSVCR80.dll |
| `0x017efa58` | `realloc` | MSVCR80.dll |

## UE3 Engine Initialization

| Address | Function | Notes |
|---------|----------|-------|
| `0x00416010` | `GuardedMain` | Top-level engine entry, routes to wxWidgets or game loop |
| `0x004185e0` | `LaunchEngineLoop` | GIs* flag init, engine class selection, callback setup |
| `0x00417fe0` | Engine class selection | `if(!GIsEditor)` → GameEngine vs EditorEngine |
| `0x00418af0` | GIs* flag-setting code | 5 consecutive `MOV [addr], reg` instructions |
| `0x00486000` | `appFailAssert` | Assertion handler — called by `check()` macro |

### GIs* Flag Values by Mode
| Mode | GIsClient | GIsServer | GIsEditor | GIsUCC | GIsGame |
|------|-----------|-----------|-----------|--------|---------|
| Game | 1 | 0 | 0 | 0 | 1 |
| Editor | 1 | 1 | 1 | 0 | 0 |
| UCC | 0 | 0 | 0 | 1 | 0 |

## UE3 Core Functions

| Address | Function | Notes |
|---------|----------|-------|
| `0x004e06a0` | `UObject::ProcessEvent` | vtable index 58 (0x3A) |
| `0x0140fe54` | `UObject::VTable` | Base UObject vtable |
| `0x00ad4530` | `WxUnrealEdApp::OnInit` | Editor wxWidgets init |
| `0x00722e60` | `UEditorEngine::Init` | Editor engine init (transaction system, brush builders) |
| `0x000c43a0` | `FFileManager::MoveFile` | File manager — has bug where moving to same path deletes file |

## BWNetDriver / BWConnection — UE3↔BigWorld Network Bridge

CME replaced UE3's standard IpNetDriver with thin wrappers around BigWorld's Mercury networking.
Class hierarchy: `UObject → UNetDriver → UBWNetDriver` (IpDrv), `UObject → UNetConnectionBase → UBWConnection` (Engine).
UBWConnection adds NO extra data members — it's a pure UE3 wrapper; all real state lives in ServerConnection/Mercury::Channel.

### UBWNetDriver
| Address | Function | Notes |
|---------|----------|-------|
| `0x00480510` | `UBWNetDriver::UBWNetDriver` | Ctor: creates UBWConnection, calls htons(0) for Winsock init |
| `0x004807d0` | `UBWNetDriver::~UBWNetDriver` | Dtor chain → `0x004800c0` |
| `0x00480650` | `UBWNetDriver::StaticClass` | UE3 class registration (IpDrv package) |
| `0x00666540` | `UBWNetDriver::InitListen` | Returns FALSE — BW clients never listen |

### UBWConnection
| Address | Function | Notes |
|---------|----------|-------|
| `0x00480ad0` | `UBWConnection::UBWConnection` | Ctor: calls UNetConnectionBase ctor (0x005e0790) |
| `0x00480b30` | `UBWConnection::~UBWConnection` | Dtor chain → `0x0047fbe0` |
| `0x00480a00` | `UBWConnection::StaticClass` | UE3 class registration (Engine package, 0xB8 bytes) |

### ServerConnection (BigWorld native C++ — not a UObject)
| Address | Function | Notes |
|---------|----------|-------|
| `0x00ddf580` | `ServerConnection::logOnBegin` (first) | SOAP/curl to LoginApp: POST /SGWLogin/UserAuth |
| `0x00ddf9f0` | `ServerConnection::logOnBegin` (reconnect) | Mercury-level reconnect via existing channel |
| `0x00dd8ec0` | `ServerConnection::logOn` | Finalizes login: validates BaseApp addr, sends authenticate |
| `0x00dd8930` | `ServerConnection::send` | Sends current bundle through Mercury::Nub |
| `0x00dd86b0` | `ServerConnection::processInput` | Main packet processing (catch at 0x00dd87c6) |
| `0x00dd9280` | `ServerConnection::enableEntities` | Enables entity streaming after login |
| `0x00dd8630` | `ServerConnection::disconnect` | Destroys channel, clears handler |
| `0x00dd8c20` | `ServerConnection::loggedOff` | Handler for server disconnect notification |
| `0x00dd6130` | `ServerConnection::isConnected` | `return pChannel_ != NULL` |

### LoginReplyHandler / BaseAppLoginHandler
| Address | Function | Notes |
|---------|----------|-------|
| `0x00dded60` | LoginReplyHandler ctor (SOAP/curl) | curl_easy_init + curl_multi for async HTTP |
| `0x00dde380` | LoginReplyHandler ctor (Mercury) | Direct Mercury-level login for reconnection |
| `0x00de10b0` | LoginReplyHandler::handleTimeout | Drives curl_multi_perform at 100ms intervals |
| `0x00ddec40` | BaseAppLoginHandler::onBaseAppReply | Channel swap on success, retry on timeout |
| `0x00de4bf0` | BaseAppLoginHandler ctor | Sends "baseAppLogin" via Mercury to BaseApp |

### Login Flow
1. UE3 creates `UNetPendingLevel` (0x008ccc30) → constructs `UBWNetDriver` → constructs `UBWConnection`
2. `ServerConnection::logOnBegin` — curl SOAP POST to `/SGWLogin/UserAuth` (gSOAP namespace: `sgwlogin`)
3. Response parsed: `SGWLoginSuccess` contains BaseApp address, ticket, shard list
4. `BaseAppLoginHandler` creates Mercury::Channel to BaseApp, sends `baseAppLogin`
5. BaseApp replies → channel transferred to `ServerConnection::pChannel_`
6. `ServerConnection::logOn` sends `authenticate` message
7. `ServerConnection::enableEntities` enables entity streaming

### BaseAppExtInterface Message Table (at 0x019d086c)
Messages FROM client TO BaseApp:

| Name | Purpose |
|------|---------|
| `baseAppLogin` | Initial login to BaseApp |
| `authenticate` | Authentication after login |
| `avatarUpdateImplicit/Explicit` | Player position updates |
| `avatarUpdateWardImplicit/Explicit` | Ward entity position updates |
| `switchInterface` | Switch to different interface |
| `requestEntityUpdate` | Request entity data refresh |
| `enableEntities` | Enable entity streaming |
| `setSpaceViewportAck` | Ack space viewport change |
| `setVehicleAck` | Ack vehicle assignment |
| `restoreClientAck` | Ack client restore |
| `disconnectClient` | Client-initiated disconnect |
| `entityMessage` | Generic entity method call |

### ClientInterface Messages (FROM BaseApp TO client)
Key messages: `authenticate`, `bandwidthNotification`, `updateFrequencyNotification`, `setGameTime`, `resetEntities`, `createBasePlayer`, `createCellPlayer`, `spaceData`, `spaceViewportInfo`, `createEntity`, `updateEntity`, `entityInvisible`, `leaveAoI`, `tickSync`, `setSpaceViewport`, `setVehicle`, 24 `avatarUpdate*` variants, `detailedPosition`, `forcedPosition`, `controlEntity`, `loggedOff`, `restoreClient`, `resourceFragment`, `voiceData`.

### SOAP Login Types (gSOAP/curl)
SOAP namespace: `http://www.stargateworlds.com/xml/sgwlogin`
Auth endpoint: `/SGWLogin/UserAuth` | Server select: `/SGWLogin/ServerSelection`

Key RTTI types: `SGWLoginRequest`, `SGWLoginResponse`, `SGWLoginSuccess`, `AccountInfo`, `SGWShardListResp`, `SGWSelectServerRequest`, `SGWServerLocationResponse`, `UserPendingBaseAppMgrRequest/Response`, `TicketType`, `BaseAppAddress`, `ServerNameType`, `SessionKeyType`, `ga__GlobalAuthReq/Res`.

## ABigWorldEntity / UBigWorldInfo — UE3↔BigWorld Entity Bridge

ABigWorldEntity is a custom AActor subclass (size 0x1C4, extends AActor 0x1A8 + 0x1C bytes).
UBigWorldInfo is a UObject holding BigWorld connection parameters (size 0x44).
Both registered in the "Engine" package from `.\Src\BigWorldEntity.cpp`.

### Coordinate Conversion Constants
| Address | Name | Value | Notes |
|---------|------|-------|-------|
| `0x018cad90` | `BW_TO_UE3_SCALE` | `100.0f` | BW meters → UE3 centimeters |
| `0x018cafcc` | `RAD_TO_URU` | `10430.378f` | Radians → UE3 rotation units (65536/2π) |
| `0x018cafd0` | `NEG_RAD_TO_URU` | `-10430.378f` | Negated for axis swap |
| `0x018cae9c` | `URU_TO_RAD` | `9.58738e-05f` | UE3 rotation units → radians (2π/65536) |

### ABigWorldEntity
| Address | Function | Notes |
|---------|----------|-------|
| `0x0077ed90` | Constructor | Sets vtable to 0x01895e3c |
| `0x0077ed20` | Destructor | / `0x0077edf0` (scripted) |
| `0x0084b040` | `ABigWorldEntity::StaticClass` | Registration at BigWorldEntity.cpp line 85 |
| `0x0084adc0` | `ABigWorldEntity::AttachComponent` | **Key override**: disables UE3 collision for BW entities (sets `CollisionResponseFlags = 0xFFFFC004`) then calls `AActor::AttachComponent` (0x006e6c10) |
| `0x00527800` | `execAttachComponent` | UScript native thunk |

### UBigWorldInfo
| Address | Function | Notes |
|---------|----------|-------|
| `0x0084b110` | Constructor | Calls UObject base |
| `0x0084b160` | Destructor | / `0x0084b1b0` |
| `0x0084ad00` | `UBigWorldInfo::StaticClass` | Registration at BigWorldEntity.cpp line 86 |
| `0x0084a880` | `UBigWorldInfo::Init` | Virtual at vtable offset 0x10c — **empty stub** |
| `0x00533E90` | `execInit` | UScript native thunk |

### BigWorld Package Registration (0x0084a8f0)
1. Creates UE3 package "BigWorld" via `FUN_0049e960(L"BigWorld", 1, 1)`
2. Creates `UnrealMessageCallback` (0x0084b210) bridging BW debug→UE3 logging
3. Registers native functions for both classes
4. Hooks into CME EventSignal system

### ABigWorldEntity Collision Behavior
When a `USkeletalMeshComponent` is attached, ABigWorldEntity sets `CollisionResponseFlags = 0xFFFFC004` to disable UE3 collision — collision is handled by BigWorld's spatial system instead.

### GameEntityBase (.\Src\GameEntityBase.cpp) — The Actual Bridge Object

GameEntityBase is the CME class that wraps a BigWorld entity and maintains the connection to its UE3 actor. ABigWorldEntity is a thin AActor; GameEntityBase does the heavy lifting.

**Field layout:**
| Offset | Type | Name | Notes |
|--------|------|------|-------|
| `+0x04` | `CacheData*` | `mCacheData` | Position cache for entities not yet in world |
| `+0x08` | `AActor*` | `mActor` | The UE3 ABigWorldEntity actor |
| `+0x0C` | `EntityID` | `mEntityID` | |
| `+0x10` | `EntityTypeID` | `mEntityType` | |
| `+0x14` | `SmartPointer` | `mAppearance` | |
| `+0x20` | `double` | `mLastUpdateTime` | |
| `+0x24` | `SpaceID` | `mSpaceID` | |
| `+0x28` | `bool` | `mIsVolatile` | |
| `+0x2C` | `EntityID` | `mVehicleID` | |
| `+0x30` | `bool` | `mInWorld` | |

**CacheData layout (0x1C bytes):** Position.XYZ (3×f32), Velocity.XYZ (3×f32), bIsVolatile (u8)

| Address | Function | Notes |
|---------|----------|-------|
| `0x00e68a30` | `GameEntityBase::ApplyTransform` | Writes BW position/rotation to UE3 Actor→Location/Rotation |
| `0x00e68670` | `GameEntityBase::EnterWorld` | |
| `0x00e685e0` | `GameEntityBase::Init` | Allocates CacheData, sets up bridge |
| `0x00e685c0` | `GameEntityBase::GetPosition` | |
| `0x00e69150` | Entity enterWorld handler | Checks appearance readiness, schedules AppearanceJob |

### Coordinate Axis Swap

Position axes are swapped during BW→UE3 conversion (not just scaled):
```
UE3_X = BW_Z * 100.0    BW_Z → UE3_X
UE3_Y = BW_X * 100.0    BW_X → UE3_Y
UE3_Z = BW_Y * 100.0    BW_Y → UE3_Z
```

Rotation conversion functions:
| Address | Function | Notes |
|---------|----------|-------|
| `0x0084a8a0` | `RotatorToRadians` | UE3 FRotator → BW radians (pitch/roll negated for handedness) |
| `0x0084a9d0` | `RadiansToRotator` | BW radians → UE3 FRotator (pitch/roll negated for handedness) |

### Debug HUD
| Address | Function | Notes |
|---------|----------|-------|
| `0x00c739e0` | `DrawBigWorldDebugInfo` | Displays BW position (meters), BW rotation (radians), UE3 position |

### Config
- `Engine.BigWorldInfo.DefaultBigWorld` — INI key for default BW connection parameters (at `0x008f5c21`)
- `Editor.EditorEngine.BWResDirectory` — BigWorld resource directory path (at `0x0084ae70`)

## Mercury Protocol Functions

| Address | Function | Notes |
|---------|----------|-------|
| `0x015841d0` | `Mercury::Nub::Nub` | Constructor: creates "NetworkThread for ExternalNub", two tbb::concurrent_queue |
| `0x01581ab0` | `Mercury::Nub::processPendingEvents` | NetworkThread: recvfrom loop → ClientMessage → concurrent_queue |
| `0x01576f90` | `Mercury::Nub::sendInternal` | Called from ServerConnection::send |
| TODO | `Mercury::Channel::send` | Outgoing message dispatch |
| TODO | `Mercury::Channel::processMessage` | Incoming message processing |
| TODO | `Mercury::Bundle::startMessage` | Begin constructing a message |
| TODO | `Mercury::Bundle::addBlob` | Add raw data to message |

### Mercury::Nub Threading Model
- Background thread ("NetworkThread for ExternalNub") handles raw UDP I/O via `processPendingEvents`
- Received packets wrapped as `Mercury::ClientMessage` pushed to `tbb::concurrent_queue` at Nub offset +0x138
- Game thread pops messages during `UGameEngine::Tick` (0x008f6930) → `TickDispatch`
- Socket error handling: WSAETIMEDOUT (0x274D), WSAECONNRESET (0x2751), WSAECONNREFUSED (0x2746) → NubException

## Entity Property Functions

| Address | Function | Notes |
|---------|----------|-------|
| `0x015652d0` | `FNetworkPropertyChange__vfunc_0` | Property change serialization |
| `0x015924a0` | `EntityDescription_parseProperties` | Property ID assignment |
| `0x015974a0` | `DataDescription_parse_2` | Property flag/type parsing |
| TODO | `Entity::readCellData` | Deserialize cell entity data |
| TODO | `Entity::readBaseData` | Deserialize base entity data |

## UE3 Editor Integration (Atrea patches)

Atrea (`AtreaLoader.config.xml`) applies runtime patches to enable editor/UCC modes.

| Patch | RVA | Description |
|-------|-----|-------------|
| EditorMode | `0x00018AF0` | Rewrites GIs* flags for editor mode |
| EditorCallbacks | `0x000186D2` | Installs editor callback devices (FCallbackEventDeviceEditor) |
| EditorCallbackVMT | `0x0198F52C` | Replaces game VMT pointers with editor VMT pointers |
| EditorSettings | `0x001757BA` | Inverts EDITOR command-line check (`setz` → `setnz`) |
| EditorCurrentPackage | `0x0198F4A0` | Changes current package from L"Launch" to L"UnrealEd" |
| EditorChunkLimit | `0x007FDA41` | Removes 100-chunk limit for streaming maps (`JLE` → `JMP`) |
| EditorMyGamesDir | `0x0008D1E8` | Skips %USERPROFILE%\My Games redirect (`CALL` → `NOP NOP`) |
| DisablePrefabSerialize | `0x001CE8E1` | Skips prefab serialization on load (`JGE` → `NOP+JMP`) |
| MallocOOMSoftFail | `0x00019923` | Returns NULL on OOM instead of asserting |

## Reference Source Code

| Source | Path | Match Quality |
|--------|------|--------------|
| BigWorld 1.9.1 | `F:\Stargate Worlds-QA\Reference\BigWorld-Engine-1.9.1\` | HIGH — BW ≥1.8.1, VC2005, 1:1 Mercury match |
| UE3 Early (2004) | `F:\Stargate Worlds-QA\Reference\UE3-2004\` | MED-HIGH — same era, VC80.sln, core architecture |
| UE3 CodeRed (2013) | `F:\Stargate Worlds-QA\Reference\UE3-CodeRed\` | MEDIUM — has FMallocTBB, later build |

### Key Source File Matches
| SGW.exe Embedded Path | Reference Source |
|----------------------|-----------------|
| `..\..\..\..\Server\bigworld\src\client\entity_manager.cpp` | `BigWorld-1.9.1/bigworld/src/client/entity_manager.cpp` |
| `..\..\..\..\Server\bigworld\src\common\servconn.cpp` | `BigWorld-1.9.1/bigworld/src/common/servconn.cpp` |
| `nub.cpp` | `BigWorld-1.9.1/src/lib/network/nub.cpp` |
| `LaunchEngineLoop.cpp` | `UE3-2004/Development/Src/Launch/Src/LaunchEngineLoop.cpp` |
| `UnObj.cpp` | `UE3-2004/Development/Src/Core/Src/UnObj.cpp` |
| `FMallocCME.h` | CME custom — derived from `UE3-CodeRed/.../FMallocTBB.h` |

### CME Custom Source Files (no reference source available)
| File | Assertion Refs | Domain |
|------|---------------|--------|
| `BWNetDriver.cpp` | 1+ | UE3↔BigWorld network driver |
| `BWConnection.cpp` | 1+ | UE3↔BigWorld connection wrapper |
| `BigWorldEntity.cpp` | 4+ | UE3 actor for BW entities |
| `FMallocCME.h` | 1 (line 37) | TBB allocator (32-bit backport of FMallocTBB) |
| `BaseAppearanceJob.cpp` | 1+ | Character appearance system |
| `DebugCommunication.cpp` | 5 | Debug/telemetry system |
| `SGWTestIpDrv.cpp` | 1+ | Network testing utility |
| `TcpNetDriver.cpp` | 2+ | TCP fallback network driver |
| `ZipStorage.cpp` | 10 | Zip archive handling |

## CME Custom Modules

### BaseAppearanceJob.cpp
| Address | Function | Notes |
|---------|----------|-------|
| `0x00eb7450` | Assertion location | State machine for async character appearance loading |
| — | BaseAppearanceJob | Abstract base: DoWork → PostProcess → Cleanup virtual dispatch |
| — | PawnAppearanceJob | Subclass for character models |
| — | CompositedAppearanceJob | Subclass for composited appearances |
| — | StaticMeshAppearanceJob | Subclass for static meshes |

Fires `Event_AppearanceJob_Completed` consumed by: GameAppearanceManager, GameBeing, GameProxyPlayer, SequenceManager, CharacterCreation, PortraitManager.

### DebugCommunication.cpp
| Address | Function | Notes |
|---------|----------|-------|
| `0x0047c2f0` | Assertion 1 | FDebugReceiver / FDebugSender (FRunnable subclasses) |
| `0x0047c980` | Assertion 2 | Lock-free SPSC ring buffer for inter-thread comms |
| `0x0047c3e0` | Assertion 3 | UDP telemetry on ports 13500/13502 |

Packet format: `'S' 'T' + name + payload` (~4KB max). Configurable via `DebugCommunication.UDP` INI section.

### TcpNetDriver.cpp
| Address | Function | Notes |
|---------|----------|-------|
| `0x0047ec40` | Assertion 1 | UTcpNetDriver (0x14C bytes, extends UNetDriver) |
| `0x0047efb0` | Assertion 2 | UTcpipConnection (0x4F74 bytes / ~20KB, extends UNetConnection) |
| `0x0047b860` | `RegisterIpDrvClasses` | Registers all 5 IpDrv classes |

Custom bool properties: `AllowPlayerPortUnreach`, `LogPortUnreach`.

### ZipStorage.cpp
| Address | Function | Notes |
|---------|----------|-------|
| `0x00479fa0` | Assertion/log 1 | `Detail::ZipStorageBase` with 7 template instantiations |
| `0x00479340` | Assertion/log 2 | Kismet events, missions, inventory, dialogs, interactions |
| `0x00479930` | Assertion/log 3 | Uses CZipArchive for zip I/O, versioned cache invalidation |
| `0x00478f90` | Assertion/log 4 | Subscribes to 6 CME events (Connected, Disconnected, etc.) |

Uses Apache log4cxx for structured logging with source location info.

### SGWTestIpDrv.cpp
| Address | Function | Notes |
|---------|----------|-------|
| `0x0047dd40` | Assertion | UTestIpDrv (0x178 bytes), UObject-derived network testing utility |

## Build Environment

| Field | Value |
|-------|-------|
| QA Build Path | `c:\BUILD\QA\SGW\Working\Development\Src\Core\Inc\FMallocCME.h` |
| Perforce Workspace | `F:\perforce3\SGW\` (from Launcher.exe strings) |
| Compiler | MSVC 8.0 (Visual C++ 2005) |
| Static Libraries | libcurl 7.17.0, gSOAP 2.7, OpenSSL, Intel TBB |
| PE Characteristics | LARGE_ADDRESS_AWARE (0x0122), image base 0x00400000 |

## String Table Locations

| Address Range | Content | Count |
|---------------|---------|-------|
| TODO | Event_NetOut_* strings | 479 |
| TODO | Event_NetIn_* strings | 496 |
| TODO | RTTI type descriptors | ~9,700 |
| TODO | BigWorld source paths | ~200+ |
| TODO | CME:: debug strings | ~100+ |

---

## How to Update

When you identify a key address in Ghidra:
1. Replace the relevant TODO with the actual address (e.g., `0x14001a2b0`)
2. Update the "Decompiled?" column if you've analyzed the function
3. Add new rows for important discoveries
4. Cross-reference with findings in `docs/gameplay/` and `docs/protocol/` docs

## Naming Conventions

- Ghidra function names: `ClassName_methodName` (underscore separator)
- Vtable entries: `ClassName__vfunc_N` (double underscore + index)
- Inferred names: `ClassName__unknown_HEXADDR` (script 10)
- Event handlers: `EventHandler_NetOut_EventName` or `EventHandler_NetIn_EventName`
