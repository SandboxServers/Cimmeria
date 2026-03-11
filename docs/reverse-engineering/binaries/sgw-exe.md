# SGW.exe Binary Overview (Ghidra Analysis)

> **Confidence**: HIGH
> **Date**: 2026-03-08
> **Source**: Ghidra MCP analysis of SGW.exe (QA build)
> **Binary**: `Stargate Worlds-QA/Working/binaries/SGW.exe`

---

## Binary Metadata

| Property | Value |
|----------|-------|
| Format | Portable Executable (PE) |
| Architecture | x86 LE 32-bit |
| Compiler | MSVC (Windows) |
| Image base | `0x00400000` |
| Address range | `0x00400000` - `0xffdfffff` |
| Total memory | 31,694,832 bytes (~30.2 MB) |
| Function count | **173,205** |
| Symbol count | 634,034 |
| Data type count | 2,001 |
| Memory blocks | 8 |

## Memory Segments

| Segment | Start | End | Size | Purpose |
|---------|-------|-----|------|---------|
| Headers | `0x00400000` | `0x004003ff` | 1 KB | PE headers |
| `.text` | `0x00401000` | `0x017ee3ff` | ~20.9 MB | Code |
| `.rdata` | `0x017ef000` | `0x01d8e3ff` | ~5.6 MB | Read-only data (strings, vtables, RTTI) |
| `.data` | `0x01d8f000` | `0x01f159ef` | ~1.5 MB | Initialized globals |
| `.tls` | `0x01f16000` | `0x01f161ff` | 512 B | Thread-local storage |
| `.rsrc` | `0x01f17000` | `0x01fe41ff` | ~820 KB | Resources (icons, manifests) |
| `.reloc` | `0x01fe5000` | `0x0223d5ff` | ~2.5 MB | Relocation table |
| `tdb` | `0xffdff000` | `0xffdfffff` | 4 KB | Thread debug block |

## Build Path Leak

The binary contains full source paths from the original build machine:

```
c:\BUILD\QA\SGW\Working\Development\Src\SGWGame\Inc\GameEntityFactory.h
c:\build\qa\sgw\working\development\src\sgwgame\inc\GameEntity.h
.\Src\BigWorldEntity.cpp
.\Src\GameEntityManager.cpp
.\Src\GameEntityBase.cpp
.\Src\GameEntity.cpp
.\Src\CombatQueue.cpp
.\Src\Ability.cpp
..\..\..\..\Server\bigworld\src\client\entity_manager.cpp
..\..\..\..\Server\bigworld\src\common\servconn.cpp
```

This confirms:
- Build machine path: `c:\BUILD\QA\SGW\Working\Development\`
- SGW game code: `Src\SGWGame\` (separate from BigWorld engine)
- BigWorld client code: `Server\bigworld\src\client\` (4 levels up from SGW source)
- BigWorld common code: `Server\bigworld\src\common\servconn.cpp`

---

## Technology Stack (Client-Side)

### Rendering Engine
- **Unreal Engine 3** (UE3) — evidenced by `AActor`, `APawn`, `APlayerController`, `AGameInfo`, `AHUD`, `ACamera`, `UObject` class hierarchies and Unreal naming conventions (A* for Actors, U* for UObjects)

### Networking
- **BigWorld Mercury** — custom reliable UDP (`Mercury::Nub`, `Mercury::Bundle`, `Mercury::Channel`, `Mercury::Packet`, `Mercury::PacketFilter`, `Mercury::PacketEncrypter`)
- **gSOAP** — SOAP/XML serialization for cooked data delivery and SOAP login

### Cryptography (CryptoPP library)
- **AES-256** (Rijndael): `BlockCipherFinal<Enc/Dec>`, `CBC_Encryption`, `CBC_Decryption`
- **HMAC-MD5**: `HMAC<Weak1::MD5>` — packet authentication
- **SHA-1**: `AlgorithmImpl<SHA1>` — hashing
- **SHA-256**: `AlgorithmImpl<SHA256>` — hashing
- **RSA**: Referenced in OpenSSL strings for TLS/login encryption
- **BlockCipherFilter**: Custom filter wrapper for Mercury packet encryption

### GUI Framework
- **CEGUI** — Crazy Eddie's GUI (game UI framework)
- **wxWidgets** — Editor/tool UI (wxPanel, wxComboBox, wxNotebook, wxScrollBar, etc.)

### Media
- **Bink Video** (`BINKW32.DLL`) — FMV playback
- **SpeedTree** (`CSpeedTreeRT`) — Foliage rendering

### Compression
- **CZipArchive** — ZIP handling for PAK files
- **zlib** — DEFLATE compression (`CDeflateCompressor`)

### Threading
- **Intel TBB** (`tbb::scalable_allocator`, `tbb::concurrent_queue`) — Concurrent data structures
- **CME Threading** (`CME::Win32ThreadEx`, `CME::ThreadInfo`) — Custom thread management

---

## CME Framework Classes (Client Architecture)

The client uses the **CME (Cheyenne Mountain Entertainment) Framework**, a custom middleware layer sitting between BigWorld networking and Unreal Engine rendering:

### Event System
The `EventSignal` namespace contains a comprehensive publish-subscribe event system:

- `EventSignal::CME::CallbackImpl<T>` — Event handler registration
- `EventSignal::CME::MemberCallback<T>` — Member function callbacks
- `EventSignal::CME::TypedEmitInfo<T>` — Event emission metadata

### Event Categories (from RTTI)

| Prefix | Count | Examples |
|--------|-------|---------|
| `Event_Action_*` | 30+ | MoveForward, Jump, CycleTargetForward, TargetSelf, ZoomIn |
| `Event_Editor_*` | 25+ | ToggleCombat, Ghost, ScreenShot, Camera1stPerson, ShowFPS |
| `Event_NetIn_*` | 120+ | onMissionUpdate, onEffectResults, onStatUpdate, onStargatePassage |
| `Event_NetOut_*` | 80+ | UseAbility, CombatDebug, ShareMission, GiveStargateAddress |
| `Event_SlashCmd_*` | 50+ | MissionAssign, GiveAbility, CombatDebug, ToggleCombatLOS |
| `Event_UI_*` | 30+ | InventoryUpdateSlot, InventoryCooldown, UnitCombat, CombatState |
| `Event_Entity_*` | 5+ | StatUpdate, PropertyUpdate (indices 4, 5, 8) |
| `Event_World_*` | 5+ | DialStargateAddress, StargateEvent |
| `Event_Sys_*` | 3+ | FrameStart |
| `Event_Net_*` | 5+ | Connected, Disconnected, ProxyData |
| `Event_Cache_*` | 5+ | ElementReady, ElementError |

### Key Singleton Classes
- `SGWNetworkManager` — Routes Event_NetOut → Mercury messages
- `SGWTextCommandMgr` — Routes Event_SlashCmd → Event_NetOut
- `SGWScriptedWindow` — Routes Event_UI + Event_NetIn → CEGUI
- `SGWHomeless` — Catches Event_Editor actions
- `GateTravel` — Handles all stargate-related events
- `CombatQueue` — Manages ability queue, links to `onEffectResults`
- `Inventory` — Handles all inventory events (containers, items, vaults, cash)

---

## Exported Functions (Partial)

SGW.exe exports 170+ ordinal functions, primarily from BigWorld's `GWaitable` threading primitives:

| Export | Address | Purpose |
|--------|---------|---------|
| `~GAcquireInterface` | `0x0041cd70` | Threading primitive destructor |
| `~GDefaultAcquireInterface` | `0x0041cde0` | Threading primitive destructor |
| `GEvent` / `~GEvent` | `0x004525f0` / `0x00452710` | Event signaling |
| `GSemaphore` | `0x00452b60` | Semaphore primitive |
| `GMutex` | via ordinal | Mutex primitive |
| `GThread` | via ordinal | Thread creation |
| `GWaitable::AddWaitHandler` | `0x004516c0` | Wait handler registration |
| `GWaitable::Acquire` | `0x00451b20` | Object acquisition |
| `GWaitable::Wait` | `0x00451b90` | Blocking wait |
| `GAcquireInterface::AcquireMultipleObjects` | `0x00451fd0` | Multi-object wait |

---

## Mercury Networking Classes (from RTTI)

| Class | RTTI Address | Purpose |
|-------|-------------|---------|
| `Mercury::Nub` | `0x01e91b88` | Core network endpoint |
| `Mercury::Bundle` | `0x01e919e8` | Message bundling |
| `Mercury::Channel` | `0x01e9188c` | Reliable connection channel |
| `Mercury::ChannelInternal` | `0x01e91e10` | Internal channel implementation |
| `Mercury::Packet` | `0x01e91dc8` | Raw UDP packet |
| `Mercury::PacketFilter` | `0x01e93b2c` | Packet encryption/decryption filter |
| `Mercury::PacketEncrypter` | `0x01e93b50` | Packet encryption implementation |
| `Mercury::BaseNub` | `0x01e919ac` | Base nub functionality |
| `Mercury::NubException` | `0x01e53394` | Network error handling |
| `Mercury::ClientMessage` | `0x01e91e38` | Base client message |
| `Mercury::ClientNetMessage` | `0x01e91e8c` | Network message subclass |
| `Mercury::ClientOutgoingMessage` | `0x01e91eb4` | Outgoing (C→S) message |
| `Mercury::ClientIncomingMessage` | `0x01e91ee0` | Incoming (S→C) message |
| `Mercury::ClientExceptionMessage` | `0x01e91e5c` | Exception message |
| `Mercury::ClientChannelRegMessage` | `0x01e91f0c` | Channel registration |
| `Mercury::ClientInactivityDetectMessage` | `0x01e91f3c` | Inactivity timeout detection |
| `Mercury::ClientResetMessage` | `0x01e91f70` | Connection reset |
| `Mercury::ClientChannelRequestStatsMessage` | `0x01e91f9c` | Channel statistics request |
| `Mercury::ClientChannelStatMessage` | `0x01e91fd4` | Channel statistics response |
| `Mercury::ReplyMessageHandler` | `0x01e51f24` | Reply handler |
| `Mercury::TimerExpiryHandler` | `0x01e51f50` | Timer expiration |
| `Mercury::BundlePrimer` | `0x01e51f7c` | Bundle initialization |
| `Mercury::InputMessageHandler` | `0x01e51fa0` | Input message processing |
| `Mercury::BaseNub::ProcessMessageHandler` | `0x01e91900` | Message dispatch |
| `Mercury::BaseNub::QueryInterfaceHandler` | `0x01e91934` | Interface query |
| `Mercury::Nub::Connection` | `0x01e91b60` | Per-peer connection state |
| `Mercury::Nub::ReplyHandlerElement` | `0x01e91a84` | Reply handler lookup |

---

## Entity System (Client-Side)

### GameEntityManager
Source: `.\Src\GameEntityManager.cpp`

Key operations discovered from assertion strings:
- `BWClassMap::InitEntityClassTable` — loads `entities.xml` to build type→class mapping
- `lowestClientID()` — entity ID validation (client IDs start at a minimum threshold)
- Entity lifecycle: create → enterWorld → leaveWorld → destroy
- `SGWSpawnableEntity` is the base class for all in-world entities

### ServerConnection
Source: `..\..\..\..\Server\bigworld\src\common\servconn.cpp`

Key operations from debug strings:
- `logOnBegin(server, username, protocol_digest)` — initiates connection
- `logOnComplete` — handles success/failure
- `enableEntities` — enables entity message processing
- `createBasePlayer(id)` — creates player base entity
- `createCellPlayer(id)` — creates player cell entity (can be buffered if arrives before base)
- `spaceData(space, key)` — receives space data
- `spaceViewportInfo(space, svid)` — receives viewport configuration
- `forcedPosition(entity)` — server-mandated position
- `addMove(entity)` — queues movement for sending
- `startEntityMessage` — begins cell method RPC (`msg_id = index | 0x80`)
- `startProxyMessage` — begins base method RPC (`msg_id = index | 0xC0`)
- `authenticate(key)` — validates session key from server
- `bandwidthFromServer` — configurable bandwidth throttling
- `loggedOff(reason)` — server-initiated disconnect
- `processInput` — main packet processing loop (handles corruption, disconnects, timing)
- `resourceFragment` — handles cooked data fragment delivery
- `restoreClient` — reconnection support
- `voiceData` — voice chat data forwarding

### Entity Movement Types (from debug strings)
```
Entity: %d is moving to cover
Entity: %d is making a combat advance
Entity: %d is leashing
Entity: %d is patroling
Entity: %d is following
Entity: %d is wandering
Entity: %d is avoiding
Entity: %d is performing unknown movement
```

---

## Game System Client Classes

### Combat
- `CombatQueue` (`.\Src\CombatQueue.cpp`) — ability queueing and execution
- Ability lifecycle events: `ABILITY_BEGIN`, `ABILITY_END`, `ABILITY_INTERRUPT`, `ABILITY_FAILED`
- `UICombatantState` — UI display for combatant status
- `UnitCombat` / `CombatState` — UI combat events

### Stargate Travel
- `GateTravel` — handles all gate events
- Properties: `worldStargateList`, `knownStargateList`, `hiddenStargateList`
- Events: `setupStargateInfo`, `updateStargateAddress`, `stargateRotationOverride`, `stargateTriggerFailed`, `onStargatePassage`
- DHD (Dial Home Device): `onDisplayDHD`, `onDHDReply`
- Ring Transporters: `onRingTransporterList`

### Inventory
- `Inventory` class listens to: `onContainerInfo`, `onActiveSlotUpdate`, `onRemoveItem`, `onUpdateItem`, `onRefreshItem`, `onClearOrgVaultInventory`, `CashChanged`, `onVaultOpen`, `onTeamVaultOpen`, `onCommandVaultOpen`
- Uses `DBInvItem` and `DBInvContainer` cache types
- UI events: `InventoryUpdateSlot`, `InventoryHideSlot`, `InventoryCooldown`, `InventoryClear`, `InventoryUpdateContainerSize`, `InventoryUpdateContainerActiveSlot`, `InventoryUpdateCash`

### Missions
- Events: `onMissionUpdate`, `onMissionOfferDisplay`, `offerSharedMission`, `onMissionRewardsDisplay`
- Outbound: `abandonMission`, `shareMission`, `shareMissionResponse`
- GM commands: `gmMissionAssign`, `gmMissionClear`, `gmMissionClearActive`, `gmMissionClearHistory`, `gmMissionList`, `gmMissionListFull`, `gmMissionDetails`, `gmMissionAdvance`, `gmMissionReset`, `gmMissionComplete`, `gmMissionSetAvailable`, `gmMissionAbandon`
- Properties: `MissionId`, `MissionDesignId`, `MissionFlags`, `MissionGiverName`
- UI: `MissionLogOpened`, `MissionLogUpdated`, `MissionUpdated`, `MissionReceived`, `MissionRemoved`, `MissionTrackerUpdated`, `MissionRewards`

### Abilities
- Cooked data: `CookedDataAbilities.pak` (1,887 entries)
- Events: `onAbilityTreeInfo`, `onPetAbilityList`, `KnownAbilitiesUpdate`
- Outbound: `useAbility`, `useAbilityOnGroundTarget`, `trainAbility`, `petInvokeAbility`, `petAbilityToggle`, `loadAbility`, `loadAbilitySet`
- GM: `gmGiveAbility`, `gmSetMobAbilitySet`, `gmDebugAbility`, `gmDebugAbilityOnMob`
- Properties: `AbilityID`, `AbilityName`, `AbilityDesc`, `AbilityTypeId`, `AbilityLists`, `Use_Ability_Velocity`
- UI: `AbilityUpdate`, `AbilityCooldownUpdate`, `UnitAbilityStart`, `UnitAbilityEnd`

---

## PAK Files Referenced in Binary

| PAK File | Purpose |
|----------|---------|
| `CookedDataAbilities.pak` | Ability definitions |
| `CookedDataEffects.pak` | Effect definitions |
| `CookedDataMissions.pak` | Mission definitions |
| `CookedDataItems.pak` | Item definitions |
| `CookedDataContainers.pak` | Container definitions |
| `CookedDataDialogs.pak` | Dialog tree definitions |
| `CookedDataStargates.pak` | Stargate definitions |
| `CookedDataKismetSetEvent.pak` | Kismet event sets |
| `CookedDataKismetSeqEvent.pak` | Kismet event sequences |

---

## URLs in Binary

| URL | Purpose |
|-----|---------|
| `http://www.stargateworlds.com/` | Main website |
| `http://www.stargateworlds.com/xml/sgwlogin` | SOAP login endpoint |
| `http://beta.stargateworlds.com/` | Beta website |

---

## Interesting Debug Assertions

These assertions from `servconn.cpp` reveal internal invariants:

```
vehicleID == 0 && "Client only entities should not start already aboard a vehicle."
0 == mPendingAttempts
replyHandler.getObject() != NULL
replyHandler->status() != LogOnStatus::LOGGED_ON
pChannel_  (non-null check, appears twice)
sentPhysics_[ args.id ] == args.physics
entitiesEnabled_
createCellPlayerMsg_.remainingLength() == 0
id != ObjectID( -1 )
mosPtr == mosPtrBeg
this->findRURequest( args.rid, true ) == rur
```

These confirm:
1. Vehicle system exists but client-only entities cannot start on vehicles
2. `sentPhysics_` array tracks physics mode per entity (for client prediction)
3. `createCellPlayer` can arrive before `createBasePlayer` and gets buffered
4. Reply handlers use request IDs (`rid`) for matching
