# Reverse Engineering Progress Tracker

> **Last updated**: 2026-03-01
> **Current phase**: Phase 1 — Foundation
> **Branch**: `feature/reverse-engineering-docs`

---

## Session Log

### Session 1 — 2026-03-01

**Completed:**
- [x] Created docs/ directory structure (all subdirectories)
- [x] Gathered all Event_NetOut strings from Ghidra — **253 unique** (more than the 97 originally estimated)
- [x] Gathered all Event_NetIn strings from Ghidra — **167 unique** (more than the 95 originally estimated)
- [x] Cataloged all 18 entity .def files with property/method counts
- [x] Cataloged all 18 interface .def files with property/method counts
- [x] Committed PLAN.md and STATUS.md as starting point
- [x] docs/README.md — Master navigation hub (248 lines)
- [x] Root README.md — Updated with project context, what works/missing
- [x] docs/protocol/message-catalog.md — All 420 events mapped with implementation status (~26% coverage)
- [x] docs/gameplay/README.md — System dashboard with 18 systems, priority matrix
- [x] docs/guides/evidence-standards.md — Three-tier confidence system (333 lines)
- [x] docs/guides/reading-decompiled-code.md — MSVC patterns, BigWorld specifics (491 lines)
- [x] docs/guides/entity-def-guide.md — Complete .def walkthrough (817 lines)
- [x] All 10 Ghidra annotation scripts (01-10) written and committed
- [x] docs/engine/entity-type-catalog.md — All 36 types with full property/method tables (953 lines)
- [x] docs/reverse-engineering/function-naming-progress.md — Script results tracking template
- [x] docs/reverse-engineering/address-map.md — Key address tracking template
- [x] docs/reverse-engineering/findings/README.md — Findings directory structure
- [x] docs/protocol/README.md — Protocol section navigation
- [x] docs/engine/README.md — Engine section navigation

**In Progress:**
- [ ] Remaining protocol/engine/analysis/architecture docs (12 files)
- [ ] Gameplay system docs (17 files: combat, abilities, effects, stats, etc.)

**Not Started:**
- [ ] Phase 2: Combat & Core Systems RE
- [ ] Phase 3: Missing Systems RE
- [ ] Phase 4: Secondary Systems RE
- [ ] Phase 5: BigWorld Engine Subsystems

---

## Data Collected

### Event_NetOut (Client → Server) — 253 unique messages

Saved to Ghidra string search results. Categories:
- **Combat/Ability**: UseAbility, useAbilityOnGroundTarget, SetAutoCycle, ConfirmEffect, RequestReload, RequestAmmoChange, SetCrouched, TestLOS, ToggleCombatLOS
- **GM/Debug**: ~50 commands (GiveItem, SetLevel, Kill, Spawn, Despawn, SetGodMode, etc.)
- **Mission**: MissionAssign, MissionAbandon, MissionAdvance, MissionComplete, MissionReset, ShareMission, ChosenRewards, AbandonMission
- **Inventory**: MoveItem, RemoveItem, UseItem, RepairItem, RepairItems, RechargeItem, LootItem, GetItemInfo, requestItemData, GMRemoveItem
- **Chat**: ChatJoin, ChatLeave, ChatBan, ChatKick, ChatMute, ChatOp, ChatPassword, ChatSetAFKMessage, ChatSetDNDMessage, sendPlayerCommunication, SendGMShout
- **Organization**: OrganizationCreation, OrganizationInvite, OrganizationKick, OrganizationLeave, OrganizationRankChange, OrganizationMOTD, OrganizationTransferCash, etc.
- **Minigame**: StartMinigame, EndMinigame, MinigameComplete, MinigameCallRequest/Accept/Decline/Abort, SpectateMinigame, debugStartMinigame, etc.
- **Mail**: SendMailMessage, RequestMailHeaders, RequestMailBody, DeleteMailMessage, ArchiveMailMessage, TakeItemFromMailMessage, TakeCashFromMailMessage, PayCODForMailMessage, ReturnMailMessage
- **Trade**: TradeProposal, TradeLockState, TradeRequestCancel
- **Store**: PurchaseItems, SellItems, BuybackItems
- **Black Market**: BMSearch, BMCreateAuction, BMCancelAuction, BMPlaceBid
- **Gate Travel**: onDialGate, DHD, SetRingTransporterDestination, GiveStargateAddress, RemoveStargateAddress
- **Pet**: PetInvokeAbility, PetAbilityToggle, PetChangeStance
- **Crafting**: Craft, Research, ReverseEngineer, RespecCraft, SpendAppliedSciencePoint, SetTechSkill, GiveBlueprint, Alloy
- **Duel**: DuelChallenge, DuelResponse, DuelForfeit
- **Character**: CreateCharacter, DeleteCharacter, PlayCharacter, RequestCharacterVisuals, Respawn, Unstuck
- **Movement/World**: GotoXYZ, GotoLocation, SetMovementType, Physics, ClientReady, WorldInstanceReset
- **Data Loading**: LoadConstants, LoadAbility, LoadAbilitySet, LoadBehavior, LoadMOB, LoadInteractionSet, LoadItem, LoadMission, LoadNACSI, RequestReload
- **Contact List**: contactListCreate, contactListDelete, contactListAddMembers, contactListRemoveMembers, contactListRename, contactListFlagsUpdate
- **Misc**: versionInfoRequest, elementDataRequest, Petition, Who, Disconnect, callForAid, onClientVersion, onClientChallengeResponse, SystemOptions, PerfStats, CancelMovie, BroadcastMinimapPing

### Event_NetIn (Server → Client) — 167 unique messages

Saved to Ghidra string search results. Categories:
- **Combat/Stats**: onEffectResults, TimerUpdate, EffectUserData, onStatUpdate, onStatBaseUpdate, onAlignmentUpdate, KnownAbilitiesUpdate, AbilityTreeInfo, onLOSResult, onMeleeRangeUpdate, onThreatenedMobsUpdate, onAggressionOverrideUpdate/Cleared
- **Entity**: onEntityMove, onVisible, EntityFlags, EntityProperty, EntityTint, BeingAppearance, onStaticMeshNameUpdate, onBeingNameUpdate, onBeingNameIDUpdate, onRemoteEntityCreate/Move/Remove
- **Character**: CharacterList, CharacterVisuals, CharacterCreateFailed, onCharacterLoadFailed, onPlayerDataLoaded, onArchetypeUpdate
- **Mission**: onMissionUpdate, onStepUpdate, onObjectiveUpdate, onTaskUpdate, MissionOffer, MissionSharedOffer, MissionRewards
- **Inventory**: onContainerInfo, onUpdateItem, onRemoveItem, onRefreshItem, onActiveSlotUpdate, CashChanged, onClearOrgVaultInventory
- **Store**: onStoreOpen, onStoreUpdate, onStoreClose, LootDisplay
- **Chat**: onPlayerCommunication, onLocalizedCommunication, onSystemCommunication, onChatJoined, onChatLeft, onNickChanged, onTellSent
- **Organization**: onOrganizationInvite, onOrganizationJoined, onOrganizationLeft, onMemberJoined/Left, onOrganizationRosterInfo, onOrganizationCashUpdate, onOrganizationRankUpdate, onLaunchOrganizationCreation, etc. (16 total)
- **Mail**: onMailHeaderInfo, onMailHeaderRemove, onMailRead, onSendMailResult
- **Black Market**: BMOpen, BMAuctions, BMAuctionUpdate, BMAuctionRemove, BMError
- **Minigame**: onStartMinigame, onEndMinigame, onStartMinigameDialog/Close, MinigameCallDisplay/Result/Abort, onSpectateList, onMinigameRegistrationPrompt/Info, AddOrUpdateMinigameHelper, RemoveMinigameHelper, ShowMinigameContact
- **Gate Travel**: setupStargateInfo, updateStargateAddress, onStargatePassage, onDisplayDHD, onDHDReply, StargateRotationOverride, StargateTriggerFailed, onRingTransporterList
- **Crafting**: onUpdateDiscipline, onUpdateRacialParadigmLevel, onUpdateKnownCrafts, onUpdateCraftingOptions, onDisciplineRespec, onCraftingRespecPrompt, onTrainerOpen
- **Pet**: PetAbilities, PetStances, PetStanceUpdate
- **Duel**: onDuelChallenge, onDuelEntitiesSet, onDuelEntitiesRemove, onDuelEntitiesClear
- **World**: onTimeofDay, setupWorldParameters, onClientMapLoad, MapInfo, ResetMapInfo, onStateFieldUpdate, AddClientHintedGenericRegion, ClearClientHintedGenericRegions
- **Level/XP**: onLevelUpdate, onExpUpdate, onMaxExpUpdate, onExtraNameUpdate, onTargetUpdate, onSetTarget
- **Trade**: TradeState, TradeResults
- **Dialog**: DialogDisplay, InitialInteraction
- **Contact List**: onContactListUpdate, onContactListDelete, onContactListAddMembers, onContactListRemoveMembers, onContactListEvent
- **Auth/Login**: AccountLoginSuccess, ServerSelectSuccess, LoginFailure, onVersionInfo, onCookedDataError, onErrorCode, onClientChallenge
- **Misc**: onSequence, onBeginAidWait, onEndAidWait, onOverridePerfStatsRate, PlayMovie, StopMovie, PvPOrganizationLeaveRequest, onStrikeTeamUpdate, onSquadLootType, onShowCommandWaypoints, onShowPath, onDisableShowPath, ShowNavigation, onSpaceQueueReady, onSpaceQueued, InteractionType, onFactionUpdate, onKismetEventSetUpdate

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

## Phase 1 Checklist

### 1a. Documentation Structure
- [x] Directory tree created
- [x] PLAN.md written
- [x] STATUS.md written (this file)
- [x] docs/README.md — Master navigation hub
- [x] docs/protocol/README.md
- [x] docs/gameplay/README.md (HUB)
- [x] docs/engine/README.md
- [x] docs/guides/evidence-standards.md
- [x] docs/guides/reading-decompiled-code.md
- [x] docs/guides/entity-def-guide.md

### 1b. Ghidra Annotation Scripts
- [x] 01_rtti_annotator.py
- [x] 02_ue3_exec_annotator.py
- [x] 03_bigworld_source_annotator.py
- [x] 04_event_signal_annotator.py
- [x] 05_mercury_annotator.py
- [x] 06_cme_framework_annotator.py
- [x] 07_vtable_annotator.py
- [x] 08_lua_binding_annotator.py
- [x] 09_string_discovery.py
- [x] 10_xref_propagation.py

### 1c. Hub Documents
- [x] docs/protocol/message-catalog.md — All 420 events mapped
- [x] docs/engine/entity-type-catalog.md — All 36 types documented
- [x] docs/gameplay/README.md — System dashboard

### 1d. Remaining Docs
- [x] docs/protocol/mercury-wire-format.md
- [x] docs/protocol/entity-property-sync.md
- [x] docs/protocol/login-handshake.md
- [x] docs/protocol/position-updates.md
- [x] docs/engine/bigworld-architecture.md
- [x] docs/engine/cme-framework.md
- [x] docs/engine/cooked-data-pipeline.md
- [x] docs/engine/watcher-system.md
- [x] docs/engine/space-management.md
- [x] docs/architecture/service-architecture.md
- [x] docs/analysis/event-net-mapping.md
- [x] docs/analysis/bigworld-reference-index.md
- [x] docs/reverse-engineering/function-naming-progress.md
- [x] docs/reverse-engineering/address-map.md
- [x] docs/gameplay/combat-system.md
- [x] docs/gameplay/ability-system.md
- [x] docs/gameplay/effect-system.md
- [x] docs/gameplay/stat-system.md
- [x] docs/gameplay/inventory-system.md
- [x] docs/gameplay/crafting-system.md
- [x] docs/gameplay/mission-system.md
- [x] docs/gameplay/gate-travel.md
- [x] docs/gameplay/minigame-system.md
- [x] docs/gameplay/organization-system.md
- [x] docs/gameplay/group-system.md
- [x] docs/gameplay/chat-system.md
- [x] docs/gameplay/mail-system.md
- [x] docs/gameplay/trade-system.md
- [x] docs/gameplay/black-market.md
- [x] docs/gameplay/pet-system.md
- [x] docs/gameplay/duel-system.md
