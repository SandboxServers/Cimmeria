# Reverse Engineering Progress Tracker

> **Last updated**: 2026-03-01
> **Current phase**: Phase 1 — Foundation (annotation scripts running)
> **Next phase**: Phase 2 — Combat & Core Systems RE
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

## Phase 2 Plan — Combat & Core Systems RE

**Goal**: Decompile the client-side handlers for combat, inventory, and entity property sync messages. Cross-reference with our server Python implementation to validate correctness and identify gaps. Every finding gets documented with evidence per `docs/guides/evidence-standards.md`.

### 2a. Combat System (server is ~70%)

The combat system is the highest-priority RE target. AbilityManager.py (1,091 lines) is the most complex Python file and has several TODO stubs that need client-side validation before implementation.

**What's already implemented (Python):**
- Ability launch with warmup/cooldown timers
- Full QR (Quality Rating) damage pipeline: QR calc → beta distribution → result code → armor → absorption
- Effect application: pulsing intervals, stat modifications, removal conditions
- TargetSelf and TargetTarget targeting with range/facing validation
- Auto-cycling (auto-repeat ability on cooldown)
- Stat system with 70+ stats, min/cur/max tracking, dirty flag sync
- Death handling, effect clearing, kismet event integration

**What's stubbed or missing (TODO comments in AbilityManager.py):**
- `TargetGround` (TCM value 3) — location-based targeting (line 575-576)
- `TCM_AERadius` — Area of Effect radius targeting (line 812-813)
- `TCM_AECone` — Cone AoE targeting (line 694-695)
- `TCM_Group` — Group-wide targeting (value 4)
- `TCM_Aura` — Aura effects (value 5)
- Line-of-sight checks (line 572)
- Tracking/Stabilization stats in QR formula (line 206)
- Cover QR modifiers: coverQR, coverAcc, coverDef (line 206)
- Stealth system integration (flags defined, no logic)
- Threat/aggro table management
- Diminishing returns on repeated effects
- Channeled ability interruption

**Ghidra targets — decompile these to understand wire format and client expectations:**

| Function | Type | Why |
|----------|------|-----|
| `Event_NetOut_UseAbility` registration func | NetOut | Client ability request serialization — what data does the client send when firing an ability? |
| `Event_NetOut_useAbilityOnGroundTarget` registration func | NetOut | Ground-target ability format — needed for TargetGround implementation |
| `Event_NetOut_SetAutoCycle` registration func | NetOut | Auto-attack toggle — what fields? |
| `Event_NetOut_TestLOS` registration func | NetOut | Client LOS check request format |
| `Event_NetOut_ToggleCombatLOS` registration func | NetOut | Combat LOS toggle format |
| `Event_NetIn_onEffectResults` registration func | NetIn | Effect result notification — what struct does the client expect? |
| `Event_NetIn_TimerUpdate` registration func | NetIn | Cooldown/warmup timer sync — timer type enum, duration format |
| `Event_NetIn_onStatUpdate` registration func | NetIn | Stat change format — {StatId, Min, Current, Max} dict layout |
| `Event_NetIn_onStatBaseUpdate` registration func | NetIn | Base stat change format |
| `Event_NetIn_onLOSResult` registration func | NetIn | LOS check result format |
| `Event_NetIn_onMeleeRangeUpdate` registration func | NetIn | Melee range sync format |
| `Event_NetIn_onThreatenedMobsUpdate` registration func | NetIn | Aggro/threat list format — needed for threat system |

**Cross-reference sources:**
- `python/cell/AbilityManager.py` (1,091 lines) — primary combat logic
- `python/cell/SGWBeing.py` (~850 lines) — stat tracking, stat sync, death/respawn
- `entities/defs/interfaces/SGWCombatant.def` — 28 properties, 19 cell methods, 6 client methods
- `entities/defs/interfaces/SGWAbilityManager.def` — ability method declarations
- Client data: `Ability.xsd`, `Effect.xsd`, `enumerations.xml` (damage types, targeting modes, result codes)
- BW reference: `external/engines/BigWorld-Engine-2.0.1/src/lib/entitydef/` — property serialization format

**Deliverables:**
- [ ] Wire format tables for each decompiled event (bytes, types, field names)
- [ ] Updated `docs/gameplay/combat-system.md` with decompiled evidence
- [ ] Updated `docs/gameplay/ability-system.md` with targeting mode wire formats
- [ ] Updated `docs/gameplay/effect-system.md` with effect result struct
- [ ] Findings doc: `docs/reverse-engineering/findings/combat-wire-formats.md`
- [ ] Validate AbilityManager.py's `onEffectResults` call matches client expectation
- [ ] Validate stat sync format matches `onStatUpdate` client handler

### 2b. Inventory System (server is ~80%)

Inventory.py (600 lines) is mostly functional but has gaps in partial updates and stack operations.

**What's already implemented (Python):**
- Full container structure: 12 bag types, equipment slots, bandolier
- Item CRUD: add, remove, move with stack combine/swap
- Item persistence: database save/load with dirty tracking
- Cash (naquadah) tracking
- Active slot management with equipment callbacks
- Discipline validation on item moves
- Buyback system for vendors
- Client sync: `onBagInfo()`, `onUpdateItem()`, `onRemoveItem()`, `onCashChanged()`

**What's stubbed or missing:**
- Partial inventory updates — currently sends full inventory on any change (TODO line 25)
- Stack splitting to occupied slots — `NotImplementedError` at line 413
- Stat recalculation on equip/unequip (equip callbacks exist but stat application unclear)
- Item repair mechanics
- Organization vault integration
- Loot distribution (group loot modes defined in .def but not connected)

**Ghidra targets:**

| Function | Type | Why |
|----------|------|-----|
| `Event_NetIn_onContainerInfo` registration func | NetIn | Full container data format — bag structure, slot layout |
| `Event_NetIn_onUpdateItem` registration func | NetIn | Item update format — which fields, partial vs full |
| `Event_NetIn_onRemoveItem` registration func | NetIn | Item removal notification format |
| `Event_NetIn_onRefreshItem` registration func | NetIn | Item refresh vs update — when is each used? |
| `Event_NetIn_CashChanged` registration func | NetIn | Currency update format |
| `Event_NetIn_onActiveSlotUpdate` registration func | NetIn | Bandolier slot switch format |
| `Event_NetOut_MoveItem` registration func | NetOut | Client item move request — source/dest format |
| `Event_NetOut_RepairItem` registration func | NetOut | Repair request format — single vs batch |
| `Event_NetOut_UseItem` registration func | NetOut | Item use request format |
| `Event_NetOut_LootItem` registration func | NetOut | Loot pickup format |

**Cross-reference sources:**
- `python/cell/Inventory.py` (600 lines) — container management
- `python/cell/Item.py` (~150 lines) — item model
- `python/cell/Bag.py` (~250 lines) — container model
- `entities/defs/interfaces/SGWInventoryManager.def` — inventory method declarations
- Client data: `enumerations.xml` (bag types, equipment slots, item categories)

**Deliverables:**
- [ ] Wire format tables for container/item events
- [ ] Updated `docs/gameplay/inventory-system.md` with decompiled evidence
- [ ] Findings doc: `docs/reverse-engineering/findings/inventory-wire-formats.md`
- [ ] Validate Inventory.py's `onBagInfo` format matches client expectation
- [ ] Determine partial update protocol (avoid sending full inventory every time)

### 2c. Entity Property Sync Protocol (critical — wrong IDs = crash)

This is the most technically critical RE target. If property IDs don't match between server and client, the client will desync, misinterpret data, or crash. The BigWorld entity property system uses ordered property IDs that must match the .def file exactly.

**What we know:**
- Properties are defined in .def files with sync flags: `OWN_CLIENT`, `OTHER_CLIENTS`, `ALL_CLIENTS`, `CELL_PUBLIC`, `CELL_PRIVATE`, `BASE`, `BASE_AND_CLIENT`
- BigWorld assigns property IDs based on ordering within the .def file
- The client reads properties by ID, not by name
- Script 03 found and named functions from `entity_manager.cpp` (7 functions) and `servconn.cpp` (5 functions) — these are the core property sync handlers
- `SGWBeing.py` has explicit stat sync with client/witness split and dirty flags

**What we need to verify:**
- Property ID assignment algorithm — is it sequential within each flag category, or global?
- Ghost entity property format — what subset goes to non-owning clients (witnesses)?
- Volatile vs non-volatile update format differences
- The `onEntityProperty(propertyId, value)` generic setter — how does the client dispatch this?

**Ghidra targets:**

| Function | Type | Why |
|----------|------|-----|
| `BW_client_entity_manager` at 00dd3330 | Named (script 03) | Entity creation/property initialization |
| `BW_client_entity_manager_2` at 00dd0b00 | Named (script 03) | Property update handling |
| `BW_client_entity_manager_3` at 00dd3150 | Named (script 03) | Entity manager function |
| `BW_client_entity_manager_4-7` | Named (script 03) | Additional entity manager functions |
| `BW_common_servconn` at 00de04c0 | Named (script 03) | Server connection — message dispatch |
| `BW_common_servconn_2-5` | Named (script 03) | Server connection functions |
| `Event_NetIn_EntityProperty` registration func | NetIn | Generic property update format |
| `Event_NetIn_EntityFlags` registration func | NetIn | Entity flag sync format |

**Cross-reference sources:**
- BW 2.0.1 `src/lib/connection/server_connection.cpp` — client-side connection handler
- BW 2.0.1 `src/lib/entitydef/entity_description.cpp` — property ID assignment
- BW 2.0.1 `src/lib/entitydef/data_description.cpp` — property serialization
- BW 2.0.1 `src/lib/connection/entity_def_constants.hpp` — property flag definitions
- `entities/defs/SGWPlayer.def` (65KB) — the largest entity definition, 11 interfaces
- `python/cell/SGWBeing.py` — stat sync implementation

**Deliverables:**
- [ ] Property ID assignment algorithm documented with evidence
- [ ] Property serialization format per sync flag type
- [ ] Ghost entity property subset documented
- [ ] Updated `docs/protocol/entity-property-sync.md` with decompiled evidence
- [ ] Findings doc: `docs/reverse-engineering/findings/property-sync-protocol.md`
- [ ] Validate that our .def property ordering matches the client's expected IDs
- [ ] Test: send a property update, verify client doesn't crash or show wrong data

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
