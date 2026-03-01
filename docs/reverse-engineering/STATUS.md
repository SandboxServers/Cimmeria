# Reverse Engineering Progress Tracker

> **Last updated**: 2026-03-01
> **Current phase**: Phase 2 — Combat & Core Systems RE (COMPLETE)
> **Next phase**: Phase 3 — Missing Systems RE
> **Branch**: `feature/reverse-engineering-docs`

---

## Annotation Script Results

Scripts 01-06 completed, 07-10 pending. Run in order — later scripts build on earlier labels.

| # | Script | Strings Found | Functions Renamed | Errors | Status |
|---|--------|---------------|-------------------|--------|--------|
| 01 | rtti_annotator.py | 9,700 RTTI classes | 4,364 vfuncs + 8,961 vtable labels | 0 | DONE |
| 02 | ue3_exec_annotator.py | 1,275 int* strings | 1,006 | 0 | DONE |
| 03 | bigworld_source_annotator.py | 17 BW paths + 17 Mercury | 23 | 0 | DONE |
| 04 | event_signal_annotator.py | 975 events (479 out + 496 in) | 419 | 0 | DONE |
| 05 | mercury_annotator.py | 120 Mercury strings | 38 + 79 RTTI vtable xrefs | 0 | DONE |
| 06 | cme_framework_annotator.py | 42 CME strings | 28 | 0 | DONE |
| 07 | vtable_annotator.py | 19,030 vtable labels | ~9,600 new vfuncs | 0 | DONE (partial, cancelled) |
| 08 | lua_binding_annotator.py | 72 Lua strings | 0 (no binding tables) | 0 | DONE |
| 09 | string_discovery.py | 1,310 `::` strings | 1,364 | 0 | DONE |
| 10 | xref_propagation.py | 3 passes (69K→68K→67K) | 3,333 (LOW confidence) | 0 | DONE |

**Final tally: 101,909 functions named (60.6% of 168,239 non-thunk functions)**

### Bugs Fixed During Testing

- **Unicode crash** (scripts 01-05): `str(value)` fails on non-ASCII in Jython. Fixed: `unicode(value).encode('utf-8', 'replace')`
- **RTTI discovery** (script 01): `get_all_defined_strings()` misses raw RTTI bytes. Fixed: `memory.findBytes()` API
- **UE3 discovery** (script 02): `findBytes` unreliable for short patterns in Jython. Fixed: reverted to `get_all_defined_strings()`
- **VTable reading** (script 07): Python `bytearray` isn't Java `byte[]`. Fixed: `mem.getInt()`
- **Encoding**: Added `# -*- coding: utf-8 -*-` to all 10 scripts

---

## Phase 1 Checklist — COMPLETE

### 1a. Documentation Structure — DONE
- [x] docs/ directory tree (all subdirectories)
- [x] docs/README.md — Master navigation hub
- [x] docs/protocol/README.md, docs/engine/README.md
- [x] docs/gameplay/README.md (HUB) — System dashboard, 18 systems
- [x] docs/guides/evidence-standards.md (333 lines)
- [x] docs/guides/reading-decompiled-code.md (491 lines)
- [x] docs/guides/entity-def-guide.md (817 lines)

### 1b. Ghidra Annotation Scripts — DONE
- [x] All 10 scripts written, tested, and run
- [x] Scripts 01-06: 5,878 functions renamed (high confidence)
- [x] Script 07: ~9,600 additional vfuncs (cancelled partway, medium confidence)
- [x] Script 08: 0 (Lua vestigial in this binary)
- [x] Script 09: 1,364 functions (medium confidence)
- [x] Script 10: 3,333 functions (low confidence, call graph inference)

### 1c. Hub Documents — DONE
- [x] docs/protocol/message-catalog.md — 975 events mapped
- [x] docs/engine/entity-type-catalog.md — 18 entities + 18 interfaces (953 lines)
- [x] docs/gameplay/README.md — System dashboard

### 1d. Remaining Docs — DONE
- [x] Protocol docs (4): mercury-wire-format, entity-property-sync, login-handshake, position-updates
- [x] Engine docs (5): bigworld-architecture, cme-framework, cooked-data-pipeline, watcher-system, space-management
- [x] Architecture docs (1): service-architecture
- [x] Analysis docs (2): event-net-mapping, bigworld-reference-index
- [x] Gameplay docs (17): combat, abilities, effects, stats, inventory, crafting, missions, gate-travel, minigames, organizations, groups, chat, mail, trade, black-market, pets, dueling
- [x] RE tracking docs (3): function-naming-progress, address-map, findings/README

---

## Phase 2 — Combat & Core Systems RE — COMPLETE

**Goal**: Decompile the client-side handlers for combat, inventory, and entity property sync messages. Cross-reference with our server Python implementation to validate correctness and identify gaps.

### Major Architectural Discovery

**The CME EventSignal system is client-side only** — it routes UI events between client subsystems, NOT network messages. All network serialization goes through a **universal RPC dispatcher** at `0x00c6fc40` that serializes method calls using BigWorld's `DataType::addToStream` virtual methods. Wire formats are entirely driven by `.def` file method signatures + `alias.xml` type definitions.

This means we do NOT need to decompile individual event handlers (100+ functions). Instead, wire formats for every entity method call can be derived directly from the `.def` files.

### 2a. Combat System — DONE

- [x] Discovered universal RPC dispatcher architecture (0x00c6fc40)
- [x] Documented wire formats for all combat messages (client→server and server→client)
- [x] Cross-validated against Python scripts (AbilityManager.py, SGWBeing.py) and .def files
- [x] All formats rated HIGH confidence (3+ sources agree)
- [x] Key discrepancy resolved: `useAbility` has 2 wire params (not 3) — `targetLoc` is server-side only
- [x] Findings: `docs/reverse-engineering/findings/combat-wire-formats.md`

### 2b. Inventory System — DONE

- [x] Documented wire formats for all inventory messages
- [x] InvItem FIXED_DICT is variable-length (ammoTypes ARRAY inside)
- [x] Cross-validated against Inventory.py and SGWInventoryManager.def
- [x] Findings: `docs/reverse-engineering/findings/inventory-wire-formats.md`

### 2c. Entity Property Sync Protocol — DONE

- [x] Property ID assignment algorithm: sequential, in parse order (Parent→Implements→Own)
- [x] Method ID assignment: same parse order, sequential within each category
- [x] createBasePlayer: 4B entityID + 2B typeID + property stream
- [x] createCellPlayer: 4B skip + 4B spaceID + 12B position + property stream
- [x] Property change encoding: IDs 0-59 = 1-byte, 60+ = 2-byte extended
- [x] Client property exclusions identified: 5 property names filtered out
- [x] Property type restrictions for propagation documented
- [x] Findings: `docs/reverse-engineering/findings/entity-property-sync.md`

### Key Addresses Discovered

| Address | Function | Role |
|---------|----------|------|
| `0x00c6fc40` | Universal RPC dispatcher | ALL outgoing entity method calls |
| `0x00dd6a60` | `ServerConnection_startEntityMessage` | Cell method header (`methodID \| 0x80`) |
| `0x00dd6980` | `ServerConnection_startProxyMessage` | Base method header (`methodID \| 0xC0`) |
| `0x00dddca0` | `ServerConnection_createBasePlayer` | Base entity creation |
| `0x00dda2e0` | `ServerConnection_createCellPlayer` | Cell entity creation |
| `0x01593600` | `EntityDescription` parse dispatch | Parse order: Impl→Props→Methods |
| `0x015924a0` | `EntityDescription_parseProperties` | Property ID assignment |
| `0x01594f60` | `MethodDescription_parse` | Method signature parsing |
| `0x015974a0` | `DataDescription_parse_2` | Property flag/type parsing |
| `0x015652d0` | `FNetworkPropertyChange__vfunc_0` | Property change serialization |

---

## Phase 3-5 (Not Started)

See `docs/reverse-engineering/PLAN.md` for full details.

- **Phase 3** — Missing Systems RE: gate travel, missions, crafting, organizations
- **Phase 4** — Secondary Systems RE: minigames, chat, mail, pets, black market, dueling
- **Phase 5** — BigWorld Engine Subsystems: watchers, space slicing, LOD, checkpointing

---

## Data Collected

### Event_NetOut (Client → Server) — 479 signals found

Categories identified:
- **Combat/Ability**: UseAbility, useAbilityOnGroundTarget, SetAutoCycle, ConfirmEffect, RequestReload, RequestAmmoChange, SetCrouched, TestLOS, ToggleCombatLOS
- **GM/Debug**: ~50 commands (GiveItem, SetLevel, Kill, Spawn, Despawn, SetGodMode, etc.)
- **Mission**: MissionAssign, MissionAbandon, MissionAdvance, MissionComplete, MissionReset, ShareMission, ChosenRewards
- **Inventory**: MoveItem, RemoveItem, UseItem, RepairItem, LootItem, GetItemInfo, requestItemData
- **Chat**: ChatJoin, ChatLeave, ChatBan, ChatKick, ChatMute, sendPlayerCommunication, SendGMShout
- **Organization**: OrganizationCreation, OrganizationInvite, OrganizationKick, OrganizationLeave, OrganizationRankChange, etc.
- **Minigame**: StartMinigame, EndMinigame, MinigameComplete, MinigameCallRequest/Accept/Decline/Abort
- **Mail**: SendMailMessage, RequestMailHeaders, RequestMailBody, DeleteMailMessage, TakeItemFromMailMessage
- **Trade**: TradeProposal, TradeLockState, TradeRequestCancel
- **Store**: PurchaseItems, SellItems, BuybackItems
- **Black Market**: BMSearch, BMCreateAuction, BMCancelAuction, BMPlaceBid
- **Gate Travel**: onDialGate, DHD, SetRingTransporterDestination, GiveStargateAddress
- **Pet**: PetInvokeAbility, PetAbilityToggle, PetChangeStance
- **Crafting**: Craft, Research, ReverseEngineer, RespecCraft, SpendAppliedSciencePoint, SetTechSkill
- **Duel**: DuelChallenge, DuelResponse, DuelForfeit
- **Character**: CreateCharacter, DeleteCharacter, PlayCharacter, Respawn, Unstuck
- **Movement/World**: GotoXYZ, GotoLocation, SetMovementType, Physics, ClientReady
- **Data Loading**: LoadConstants, LoadAbility, LoadAbilitySet, LoadBehavior, LoadMOB, LoadItem, LoadMission
- **Contact List**: contactListCreate, contactListDelete, contactListAddMembers, contactListRemoveMembers

### Event_NetIn (Server → Client) — 496 signals found

Categories identified:
- **Combat/Stats**: onEffectResults, TimerUpdate, onStatUpdate, onStatBaseUpdate, onAlignmentUpdate, KnownAbilitiesUpdate, AbilityTreeInfo, onLOSResult, onMeleeRangeUpdate, onThreatenedMobsUpdate
- **Entity**: onEntityMove, onVisible, EntityFlags, EntityProperty, EntityTint, BeingAppearance, onBeingNameUpdate
- **Character**: CharacterList, CharacterVisuals, CharacterCreateFailed, onPlayerDataLoaded, onArchetypeUpdate
- **Mission**: onMissionUpdate, onStepUpdate, onObjectiveUpdate, onTaskUpdate, MissionOffer, MissionRewards
- **Inventory**: onContainerInfo, onUpdateItem, onRemoveItem, onRefreshItem, onActiveSlotUpdate, CashChanged
- **Store**: onStoreOpen, onStoreUpdate, onStoreClose, LootDisplay
- **Chat**: onPlayerCommunication, onSystemCommunication, onChatJoined, onChatLeft, onNickChanged, onTellSent
- **Organization**: 16 events — onOrganizationInvite, onOrganizationJoined, onMemberJoined/Left, onOrganizationRosterInfo, etc.
- **Mail**: onMailHeaderInfo, onMailHeaderRemove, onMailRead, onSendMailResult
- **Black Market**: BMOpen, BMAuctions, BMAuctionUpdate, BMAuctionRemove, BMError
- **Minigame**: onStartMinigame, onEndMinigame, MinigameCallDisplay/Result/Abort, onSpectateList
- **Gate Travel**: setupStargateInfo, updateStargateAddress, onStargatePassage, onDisplayDHD, onDHDReply
- **Crafting**: onUpdateDiscipline, onUpdateKnownCrafts, onUpdateCraftingOptions, onCraftingRespecPrompt
- **Pet**: PetAbilities, PetStances, PetStanceUpdate
- **Duel**: onDuelChallenge, onDuelEntitiesSet, onDuelEntitiesRemove
- **World**: onTimeofDay, setupWorldParameters, onClientMapLoad, MapInfo, onStateFieldUpdate
- **Level/XP**: onLevelUpdate, onExpUpdate, onMaxExpUpdate, onTargetUpdate
- **Auth/Login**: AccountLoginSuccess, ServerSelectSuccess, LoginFailure, onVersionInfo, onClientChallenge

### Entity Type Catalog — 18 entities + 18 interfaces

**Entity Hierarchy:**
```
Account (standalone)
SGWEntity (base)
├── SGWSpawnableEntity
│   ├── SGWBeing
│   │   ├── SGWMob
│   │   │   └── SGWPet
│   │   └── SGWPlayer (11 interfaces)
│   │       └── SGWGmPlayer
│   └── SGWDuelMarker
├── SGWBlackMarket
├── SGWCoverSet
├── SGWEscrow
├── SGWSpaceCreator
├── SGWSpawnRegion
├── SGWSpawnSet
├── SGWPlayerRespawner
├── SGWChannelManager
└── SGWPlayerGroupAuthority
```

**Interfaces:** ClientCache, Communicator, ContactListManager, DistributionGroupMember, EventParticipant, GateTravel, GroupAuthority, Lootable, MinigamePlayer, Missionary, OrganizationMember, SGWAbilityManager, SGWBeing, SGWBlackMarketManager, SGWCombatant, SGWInventoryManager, SGWMailManager, SGWPoller

---

## Session Log

### Session 1 — 2026-03-01

**Phase 1 Foundation — COMPLETED:**
- Created full docs/ directory structure (54 files, ~14,200 lines)
- Wrote all 10 Ghidra annotation scripts
- Ran scripts 01-06 in Ghidra, naming 5,878 functions
- Fixed 5 Jython compatibility bugs during testing
- Created PR #2 on `feature/reverse-engineering-docs` branch
- Fixed script 07 read_pointer bug, ran all 10 scripts
- Fleshed out Phase 2 plan with specific Ghidra targets, cross-references, and deliverables
- All annotation scripts complete: **101,909 functions named (60.6%)**

**Key discoveries:**
- Binary has 9,700 RTTI classes (24x more than estimated 200-400)
- 168,239 total non-thunk functions (not ~85,000 as initially estimated)
- 975 Event signals found (vs 420 originally estimated from defined strings)
- UE3 registration uses `int*` strings with adjacent function pointers in data tables
- `get_all_defined_strings()` works for typed strings, `memory.findBytes()` needed for raw bytes
- Jython `bytearray` is not Java `byte[]` — use `mem.getInt()` instead
- Lua is vestigial (72 strings, no binding tables) — SGW uses Python/Boost.Python
- CEGUI is the UI library (807+ functions from script 09)
- Xref propagation cascades well: pass 1→713, pass 2→1,350, pass 3→1,270

### Session 2 — 2026-03-01

**Phase 2 Combat & Core Systems RE — COMPLETED:**
- Decompiled 15+ functions via Ghidra MCP for targeted analysis
- Discovered universal RPC dispatcher at `0x00c6fc40` — all entity method calls route through one function
- Corrected misconception: CME EventSignal is client-side UI bus, NOT network protocol
- Phase 1 annotations were CORRECT (vfunc_0 = destructor is right), but plan assumptions about which vtable slot has serialization were wrong
- Wire format derivation method: `.def` files + `alias.xml` → complete wire format (no per-handler decompilation needed)
- Decompiled `EntityDescription` parser chain to confirm property/method ID assignment order
- Decompiled `ServerConnection_createBasePlayer` and `createCellPlayer` for entity creation format
- Cross-validated all formats against Python client scripts, .def files, and BW 2.0.1 source
- Wrote 3 findings documents totaling ~1,200 lines of documented wire formats with evidence

**Key discoveries:**
- Universal RPC dispatcher at `0x00c6fc40` routes ALL entity method calls
- Cell method header: `methodID | 0x80`, Base method header: `methodID | 0xC0`
- Arguments serialized via DataType vtable method at offset 0x20 (`addToStream`)
- Entity parse order: Parent→Implements→Properties→ClientMethods→CellMethods→BaseMethods
- Property/method IDs assigned sequentially in parse order
- Property IDs 0-59 use 1-byte encoding, 60+ use 2-byte extended
- `useAbility` wire format has 2 params (not 3) — `targetLoc` is server-side only
- 5 property names excluded from client processing: publicReservationData, publicMissionData, completedMissions, aggressionOverrides, effectMonikers
- createBasePlayer format: 4B entityID + 2B typeID + property stream
- createCellPlayer buffers if no base player yet, replays after createBasePlayer
