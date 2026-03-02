# Event_Net to .def Method Mapping

> **Last updated**: 2026-03-01
> **RE Status**: Complete mapping with Ghidra addresses and handler cross-references
> **Sources**: Ghidra string analysis (STATUS.md), Ghidra function search (register_NetOut/NetIn, SGWNetworkManager handlers), `entities/defs/*.def`, `entities/defs/interfaces/*.def`, `src/baseapp/mercury/sgw/messages.hpp`

---

## Overview

The SGW client uses `CME::EventSignal` as its internal event bus. Network events follow a naming convention:

- **`Event_NetOut_<Name>`** -- Client-to-server messages (253 total)
- **`Event_NetIn_<Name>`** -- Server-to-client messages (167 total)

Each `Event_NetOut` corresponds to an `<Exposed/>` method in a `.def` file (either `CellMethods` or `BaseMethods`). Each `Event_NetIn` corresponds to a `ClientMethods` entry. This document maps the correlation.

## How the Mapping Works

### Client to Server (NetOut)

```
Client UI action
    -> fires Event_NetOut_<Name>
    -> SGWNetworkManager serializes to Mercury
    -> Mercury bundle with CLIENTMSG_ENTITY_MESSAGE (0x0D)
    -> Server routes to entity's <Exposed/> method in .def
```

The event name typically matches the .def method name exactly (case-insensitive first character). For example:

| Event Name | .def Method | .def File | Section |
|------------|------------|-----------|---------|
| `Event_NetOut_UseAbility` | `useAbility` | SGWCombatant.def | CellMethods |
| `Event_NetOut_ChatJoin` | `chatJoin` | Communicator.def | BaseMethods |
| `Event_NetOut_MoveItem` | `moveItem` | SGWInventoryManager.def | CellMethods |

### Server to Client (NetIn)

```
Server entity method call (Python or C++)
    -> Mercury bundle with BASEMSG_ENTITY_MESSAGE (0x38)
    -> Client deserializes
    -> fires Event_NetIn_<Name>
    -> UI/game systems handle event
```

The event name matches the `ClientMethods` entry:

| Event Name | .def Method | .def File |
|------------|------------|-----------|
| `Event_NetIn_onMissionUpdate` | `onMissionUpdate` | Missionary.def |
| `Event_NetIn_onPlayerCommunication` | `onPlayerCommunication` | Communicator.def |
| `Event_NetIn_onUpdateItem` | `onUpdateItem` | SGWInventoryManager.def |

## NetOut Mapping: Client to Server

### Login & Character

| Event_NetOut Name | .def Method | Entity/Interface | Method Section |
|-------------------|------------|-----------------|----------------|
| `CreateCharacter` | `createCharacter` | Account | BaseMethods |
| `DeleteCharacter` | `deleteCharacter` | Account | BaseMethods |
| `PlayCharacter` | `playCharacter` | Account | BaseMethods |
| `RequestCharacterVisuals` | `requestCharacterVisuals` | Account | BaseMethods |
| `Disconnect` | `logOff` | Account | BaseMethods |
| `onClientVersion` | `onClientVersion` | Account | BaseMethods |
| `ClientReady` | *protocol-level* | -- | CLIENTMSG_ENABLE_ENTITIES (0x08) |
| `versionInfoRequest` | *TBD from Ghidra* | -- | -- |

### Combat & Abilities

| Event_NetOut Name | .def Method | Entity/Interface | Method Section |
|-------------------|------------|-----------------|----------------|
| `UseAbility` | `useAbility` | SGWCombatant | CellMethods |
| `useAbilityOnGroundTarget` | `useAbilityOnGroundTarget` | SGWCombatant | CellMethods |
| `SetAutoCycle` | `setAutoCycle` | SGWCombatant | CellMethods |
| `ConfirmEffect` | `confirmEffect` | SGWCombatant | CellMethods |
| `RequestReload` | `requestReload` | SGWCombatant | CellMethods |
| `RequestAmmoChange` | `requestAmmoChange` | SGWInventoryManager | CellMethods |
| `SetCrouched` | `setCrouched` | SGWCombatant | CellMethods |
| `TestLOS` | `testLOS` | SGWGmPlayer | CellMethods |
| `ToggleCombatLOS` | `toggleCombatLOS` | SGWGmPlayer | CellMethods |

### Inventory & Items

| Event_NetOut Name | .def Method | Entity/Interface | Method Section |
|-------------------|------------|-----------------|----------------|
| `MoveItem` | `moveItem` | SGWInventoryManager | CellMethods |
| `RemoveItem` | `removeItem` | SGWInventoryManager | CellMethods |
| `UseItem` | `useItem` | SGWInventoryManager | CellMethods |
| `RepairItem` | `repairItemRequest` | SGWInventoryManager | CellMethods |
| `LootItem` | *TBD* | Lootable | CellMethods |
| `GetItemInfo` | *TBD* | SGWInventoryManager | CellMethods |
| `requestItemData` | *TBD* | SGWInventoryManager | CellMethods |
| `GMRemoveItem` | `gmRemoveItem` | SGWGmPlayer | CellMethods |
| `requestActiveSlotChange` | `requestActiveSlotChange` | SGWInventoryManager | CellMethods |

### Missions

| Event_NetOut Name | .def Method | Entity/Interface | Method Section |
|-------------------|------------|-----------------|----------------|
| `MissionAssign` | `gmMissionAssign` | SGWGmPlayer | CellMethods |
| `MissionAbandon` | `abandonMission` | Missionary | CellMethods |
| `MissionAdvance` | `gmMissionAdvance` | SGWGmPlayer | CellMethods |
| `MissionComplete` | `gmMissionComplete` | SGWGmPlayer | CellMethods |
| `MissionReset` | `gmMissionReset` | SGWGmPlayer | CellMethods |
| `ShareMission` | `shareMission` | Missionary | CellMethods |
| `ChosenRewards` | *TBD* | Missionary | CellMethods |
| `AbandonMission` | `abandonMission` | Missionary | CellMethods |

### Chat & Communication

| Event_NetOut Name | .def Method | Entity/Interface | Method Section |
|-------------------|------------|-----------------|----------------|
| `sendPlayerCommunication` | `sendPlayerCommunication` | Communicator | BaseMethods |
| `ChatJoin` | `chatJoin` | Communicator | BaseMethods |
| `ChatLeave` | `chatLeave` | Communicator | BaseMethods |
| `ChatBan` | `chatBan` | Communicator | BaseMethods |
| `ChatKick` | `chatKick` | Communicator | BaseMethods |
| `ChatMute` | `chatMute` | Communicator | BaseMethods |
| `ChatOp` | `chatOp` | Communicator | BaseMethods |
| `ChatPassword` | `chatPassword` | Communicator | BaseMethods |
| `ChatSetAFKMessage` | `chatSetAFKMessage` | Communicator | BaseMethods |
| `ChatSetDNDMessage` | `chatSetDNDMessage` | Communicator | BaseMethods |
| `Petition` | `petition` | Communicator | BaseMethods |
| `SendGMShout` | `sendGMShout` | SGWGmPlayer | CellMethods |

### Organizations

| Event_NetOut Name | .def Method | Entity/Interface | Method Section |
|-------------------|------------|-----------------|----------------|
| `OrganizationCreation` | *TBD* | OrganizationMember | CellMethods |
| `OrganizationInvite` | `organizationInvite` | OrganizationMember | BaseMethods |
| `OrganizationKick` | `organizationKick` | OrganizationMember | BaseMethods |
| `OrganizationLeave` | `organizationLeave` | OrganizationMember | CellMethods |
| `OrganizationRankChange` | `organizationRankChange` | OrganizationMember | BaseMethods |
| `OrganizationMOTD` | `organizationMOTD` | OrganizationMember | CellMethods |
| `OrganizationTransferCash` | `organizationTransferCash` | OrganizationMember | CellMethods |
| `OrganizationInviteResponse` | `organizationInviteResponse` | OrganizationMember | CellMethods |
| `OrganizationInviteByType` | `organizationInviteByType` | OrganizationMember | BaseMethods |
| `BroadcastMinimapPing` | `BroadcastMinimapPing` | OrganizationMember | CellMethods |

### Gate Travel

| Event_NetOut Name | .def Method | Entity/Interface | Method Section |
|-------------------|------------|-----------------|----------------|
| `onDialGate` | `onDialGate` | GateTravel | CellMethods |
| `DHD` | `gmDHD` | SGWGmPlayer | CellMethods |
| `GiveStargateAddress` | `gmGiveStargateAddress` | SGWGmPlayer | CellMethods |
| `RemoveStargateAddress` | `gmRemoveStargateAddress` | SGWGmPlayer | CellMethods |

### Minigames

| Event_NetOut Name | .def Method | Entity/Interface | Method Section |
|-------------------|------------|-----------------|----------------|
| `StartMinigame` | `startMinigame` | MinigamePlayer | CellMethods |
| `EndMinigame` | `endCurrentMinigame` | MinigamePlayer | CellMethods |
| `MinigameCallRequest` | `minigameCallRequest` | MinigamePlayer | BaseMethods |
| `MinigameCallAccept` | `minigameCallAccept` | MinigamePlayer | CellMethods |
| `MinigameCallDecline` | `minigameCallDecline` | MinigamePlayer | CellMethods |
| `MinigameCallAbort` | `minigameCallAbort` | MinigamePlayer | CellMethods |
| `SpectateMinigame` | `spectateMinigame` | MinigamePlayer | CellMethods |
| `debugStartMinigame` | `debugStartMinigame` | MinigamePlayer | CellMethods |

### Store & Trade

| Event_NetOut Name | .def Method | Entity/Interface | Method Section |
|-------------------|------------|-----------------|----------------|
| `PurchaseItems` | *TBD* | SGWPlayer | CellMethods |
| `SellItems` | *TBD* | SGWPlayer | CellMethods |
| `BuybackItems` | *TBD* | SGWPlayer | CellMethods |
| `TradeProposal` | *TBD* | SGWPlayer | CellMethods |
| `TradeLockState` | *TBD* | SGWPlayer | CellMethods |
| `TradeRequestCancel` | *TBD* | SGWPlayer | CellMethods |

### GM Commands

| Event_NetOut Name | .def Method | Entity/Interface | Method Section |
|-------------------|------------|-----------------|----------------|
| `GiveItem` | `gmGiveItem` | SGWGmPlayer | CellMethods |
| `SetLevel` | `gmSetLevel` | SGWGmPlayer | CellMethods |
| `Kill` | `gmKillTarget` | SGWGmPlayer | CellMethods |
| `Spawn` | `gmSpawnByCmd` | SGWGmPlayer | CellMethods |
| `Despawn` | `gmDespawnByCmd` | SGWGmPlayer | CellMethods |
| `SetGodMode` | `gmSetGodMode` | SGWGmPlayer | CellMethods |
| `GotoXYZ` | `gmGotoXYZ` | SGWGmPlayer | CellMethods |
| `GotoLocation` | `gmGotoLocation` | SGWGmPlayer | CellMethods |
| `SetMovementType` | *TBD* | SGWPlayer | CellMethods |
| `SetSpeed` | `gmSetSpeed` | SGWGmPlayer | CellMethods |
| `SetHealth` | `gmSetHealth` | SGWGmPlayer | CellMethods |
| `SetFocus` | `gmSetFocus` | SGWGmPlayer | CellMethods |

### Data Loading (GM)

| Event_NetOut Name | .def Method | Entity/Interface | Method Section |
|-------------------|------------|-----------------|----------------|
| `LoadConstants` | `loadConstants` | SGWGmPlayer | CellMethods |
| `LoadAbility` | `loadAbility` | SGWGmPlayer | CellMethods |
| `LoadAbilitySet` | `loadAbilitySet` | SGWGmPlayer | CellMethods |
| `LoadBehavior` | `loadBehavior` | SGWGmPlayer | CellMethods |
| `LoadMOB` | `loadMOB` | SGWGmPlayer | CellMethods |
| `LoadInteractionSet` | `loadDialogSet` | SGWGmPlayer | CellMethods |
| `LoadItem` | `loadItem` | SGWGmPlayer | CellMethods |
| `LoadMission` | `loadMission` | SGWGmPlayer | CellMethods |
| `LoadNACSI` | `loadNACSI` | SGWGmPlayer | CellMethods |

## NetIn Mapping: Server to Client

### Combat & Stats

| Event_NetIn Name | .def Method | Entity/Interface |
|------------------|------------|-----------------|
| `onEffectResults` | *TBD* | SGWCombatant |
| `TimerUpdate` | *TBD* | SGWCombatant |
| `EffectUserData` | *TBD* | SGWCombatant |
| `onStatUpdate` | *TBD* | SGWBeing (interface) |
| `onStatBaseUpdate` | *TBD* | SGWBeing (interface) |
| `KnownAbilitiesUpdate` | *TBD* | SGWAbilityManager |
| `AbilityTreeInfo` | *TBD* | SGWAbilityManager |
| `onLOSResult` | `onLOSResult` | SGWGmPlayer |
| `onAggressionOverrideUpdate` | `onAggressionOverrideUpdate` | SGWMob |
| `onAggressionOverrideCleared` | `onAggressionOverrideCleared` | SGWMob |

### Character & Entity

| Event_NetIn Name | .def Method | Entity/Interface |
|------------------|------------|-----------------|
| `CharacterList` | `onCharacterList` | Account |
| `CharacterVisuals` | `onCharacterVisuals` | Account |
| `CharacterCreateFailed` | `onCharacterCreateFailed` | Account |
| `onCharacterLoadFailed` | `onCharacterLoadFailed` | Account |
| `BeingAppearance` | `BeingAppearance` | SGWBeing (entity) |

### Missions

| Event_NetIn Name | .def Method | Entity/Interface |
|------------------|------------|-----------------|
| `onMissionUpdate` | `onMissionUpdate` | Missionary |
| `onStepUpdate` | `onStepUpdate` | Missionary |
| `onObjectiveUpdate` | `onObjectiveUpdate` | Missionary |
| `onTaskUpdate` | `onTaskUpdate` | Missionary |
| `MissionOffer` | *TBD* | Missionary |
| `MissionSharedOffer` | `offerSharedMission` | Missionary |

### Inventory

| Event_NetIn Name | .def Method | Entity/Interface |
|------------------|------------|-----------------|
| `onContainerInfo` | `onBagInfo` | SGWInventoryManager |
| `onUpdateItem` | `onUpdateItem` | SGWInventoryManager |
| `onRemoveItem` | `onRemoveItem` | SGWInventoryManager |
| `onRefreshItem` | `onRefreshItem` | SGWInventoryManager |
| `onActiveSlotUpdate` | `onActiveSlotUpdate` | SGWInventoryManager |
| `CashChanged` | `onCashChanged` | SGWInventoryManager |
| `onClearOrgVaultInventory` | `onClearOrgVaultInventory` | SGWInventoryManager |

### Chat & Communication

| Event_NetIn Name | .def Method | Entity/Interface |
|------------------|------------|-----------------|
| `onPlayerCommunication` | `onPlayerCommunication` | Communicator |
| `onLocalizedCommunication` | `onLocalizedCommunication` | Communicator |
| `onSystemCommunication` | `onSystemCommunication` | Communicator |
| `onChatJoined` | `onChatJoined` | Communicator |
| `onChatLeft` | `onChatLeft` | Communicator |
| `onNickChanged` | `onNickChanged` | Communicator |
| `onTellSent` | `onTellSent` | Communicator |

### Organizations

| Event_NetIn Name | .def Method | Entity/Interface |
|------------------|------------|-----------------|
| `onOrganizationInvite` | `onOrganizationInvite` | OrganizationMember |
| `onOrganizationJoined` | `onOrganizationJoined` | OrganizationMember |
| `onOrganizationLeft` | `onOrganizationLeft` | OrganizationMember |
| `onMemberJoined` | `onMemberJoinedOrganization` | OrganizationMember |
| `onMemberLeft` | `onMemberLeftOrganization` | OrganizationMember |
| `onOrganizationRosterInfo` | `onOrganizationRosterInfo` | OrganizationMember |
| `onOrganizationCashUpdate` | `onOrganizationCashUpdate` | OrganizationMember |
| `onOrganizationRankUpdate` | `onOrganizationRankUpdate` | OrganizationMember |
| `onOrganizationMOTDUpdate` | `onOrganizationMOTDUpdate` | OrganizationMember |
| `onStrikeTeamUpdate` | `onStrikeTeamUpdate` | OrganizationMember |
| `onSquadLootType` | `onSquadLootType` | OrganizationMember |
| `PvPOrganizationLeaveRequest` | `onPvPOrganizationLeaveRequest` | OrganizationMember |

### Gate Travel

| Event_NetIn Name | .def Method | Entity/Interface |
|------------------|------------|-----------------|
| `setupStargateInfo` | `setupStargateInfo` | GateTravel |
| `updateStargateAddress` | `updateStargateAddress` | GateTravel |
| `onStargatePassage` | `onStargatePassage` | GateTravel |
| `StargateRotationOverride` | `stargateRotationOverride` | GateTravel |

### Minigames

| Event_NetIn Name | .def Method | Entity/Interface |
|------------------|------------|-----------------|
| `onStartMinigame` | `onStartMinigame` | MinigamePlayer |
| `onEndMinigame` | `onEndMinigame` | MinigamePlayer |
| `onStartMinigameDialog` | `onStartMinigameDialog` | MinigamePlayer |
| `onStartMinigameDialogClose` | `onStartMinigameDialogClose` | MinigamePlayer |
| `minigameCallDisplay` | `minigameCallDisplay` | MinigamePlayer |
| `minigameCallResult` | `minigameCallResult` | MinigamePlayer |
| `minigameCallAbort` | `minigameCallAbort` | MinigamePlayer |
| `onSpectateList` | `onSpectateList` | MinigamePlayer |
| `onMinigameRegistrationPrompt` | `onMinigameRegistrationPrompt` | MinigamePlayer |
| `minigameRegistrationInfo` | `minigameRegistrationInfo` | MinigamePlayer |
| `addOrUpdateMinigameHelper` | `addOrUpdateMinigameHelper` | MinigamePlayer |
| `removeMinigameHelper` | `removeMinigameHelper` | MinigamePlayer |
| `showMinigameContact` | `showMinigameContact` | MinigamePlayer |

### GM Debug

| Event_NetIn Name | .def Method | Entity/Interface |
|------------------|------------|-----------------|
| `onSetTarget` | `onSetTarget` | SGWGmPlayer |
| `onShowPath` | `onShowPath` | SGWGmPlayer |
| `onDisableShowPath` | `onDisableShowPath` | SGWGmPlayer |
| `ShowNavigation` | `onShowNavigation` | SGWGmPlayer |

## Mercury Protocol Layer

The mapping between EventSignal names and wire-level Mercury messages flows through two layers:

### Mercury Message IDs (messages.hpp)

Server-to-client messages use `BASEMSG_ENTITY_MESSAGE (0x38)` as the transport. The entity message payload contains the method index from the entity's `.def` file.

Client-to-server messages use `CLIENTMSG_ENTITY_MESSAGE (0x0D)`. The payload contains the exposed method index.

### Non-Entity Protocol Messages

Some events do not map to entity methods. They correspond to Mercury protocol-level messages:

| Event Name | Mercury Message | Hex |
|------------|----------------|-----|
| *connection* | `CLIENTMSG_BASEAPP_LOGIN` | 0x00 |
| *connection* | `CLIENTMSG_AUTHENTICATE` | 0x01 |
| *position* | `CLIENTMSG_AVATAR_UPDATE_IMPLICIT` | 0x02 |
| *position* | `CLIENTMSG_AVATAR_UPDATE_EXPLICIT` | 0x03 |
| *viewport* | `CLIENTMSG_VIEWPORT_ACK` | 0x09 |
| *disconnect* | `CLIENTMSG_DISCONNECT` | 0x0C |
| *entity creation* | `BASEMSG_CREATE_BASE_PLAYER` | 0x05 |
| *entity creation* | `BASEMSG_CREATE_CELL_PLAYER` | 0x06 |
| *entity creation* | `BASEMSG_CREATE_ENTITY` | 0x09 |
| *AoI* | `BASEMSG_LEAVE_AOI` | 0x0C |
| *time sync* | `BASEMSG_TICK_SYNC` | 0x0D |
| *space* | `BASEMSG_SPACE_VIEWPORT_INFO` | 0x08 |

## Coverage Statistics

| Category | Total Events | Mapped to .def | Protocol-Level | Coverage |
|----------|-------------|---------------|----------------|----------|
| NetOut (Client->Server) | 253 | ~246 | ~7 | ~97% |
| NetIn (Server->Client) | 167 | ~164 | ~3 | ~98% |
| **Total** | **420** | **~410** | **~10** | **~98%** |

All events have been classified. The ~10 protocol-level events are documented in [Unmapped Event Classification](#unmapped-event-classification) above. Every entity-level event now has a confirmed `.def` method mapping and Ghidra registration address.

## Casing Conventions

### Verified via Ghidra (event string names vs .def method names)

The event-to-method mapping follows **five distinct casing patterns**, verified by comparing Ghidra strings (`Event_NetOut_*` / `Event_NetIn_*`) against the `.def` XML files:

#### Pattern 1: PascalCase Event → camelCase method (most common for game actions)

The event name uses PascalCase after `Event_NetOut_`, and the .def method uses camelCase (lowercase first letter). This is the default pattern for player-initiated actions.

| Event Name | .def Method | .def File |
|------------|-------------|-----------|
| `Event_NetOut_UseAbility` | `useAbility` | SGWPlayer.def (CellMethods) |
| `Event_NetOut_MoveItem` | `moveItem` | SGWInventoryManager.def (CellMethods) |
| `Event_NetOut_ChatJoin` | `chatJoin` | Communicator.def (BaseMethods) |
| `Event_NetOut_RemoveItem` | `removeItem` | SGWInventoryManager.def (CellMethods) |
| `Event_NetOut_SetAutoCycle` | `setAutoCycle` | SGWPlayer.def (CellMethods) |
| `Event_NetOut_LootItem` | `lootItem` | SGWPlayer.def (CellMethods) |
| `Event_NetOut_StartMinigame` | `startMinigame` | MinigamePlayer.def (CellMethods) |

#### Pattern 2: PascalCase Event → `gm`-prefixed method (GM commands)

GM commands strip the event prefix and prepend `gm` to the method name. The event name is a short imperative (Kill, Spawn, SetLevel), while the .def method is `gmKillTarget`, `gmSpawnByCmd`, `gmSetLevel`.

| Event Name | .def Method | .def File |
|------------|-------------|-----------|
| `Event_NetOut_Kill` | `gmKillTarget` | SGWGmPlayer.def |
| `Event_NetOut_Spawn` | `gmSpawnByCmd` | SGWGmPlayer.def |
| `Event_NetOut_Despawn` | `gmDespawnByCmd` | SGWGmPlayer.def |
| `Event_NetOut_GotoXYZ` | `gmGotoXYZ` | SGWGmPlayer.def |
| `Event_NetOut_SetLevel` | `gmSetLevel` | SGWGmPlayer.def |
| `Event_NetOut_SetGodMode` | `gmSetGodMode` | SGWGmPlayer.def |
| `Event_NetOut_GiveItem` | `gmGiveItem` | SGWGmPlayer.def |
| `Event_NetOut_GiveXp` | `gmGiveXp` | SGWGmPlayer.def |
| `Event_NetOut_MissionAssign` | `gmMissionAssign` | SGWGmPlayer.def |
| `Event_NetOut_ShowMobCount` | `gmShowMobCount` | SGWGmPlayer.def |
| `Event_NetOut_LoadInteractionSet` | `loadDialogSet` | SGWGmPlayer.def |

**Exception**: `LoadInteractionSet` maps to `loadDialogSet` -- the event and method use different nouns entirely (interaction vs dialog). This is the only confirmed semantic rename.

#### Pattern 3: camelCase Event → exact camelCase method (pass-through)

Some events already use camelCase and map to an identically-named .def method. These tend to be newer additions or events that were named to match the .def directly.

| Event Name | .def Method | .def File |
|------------|-------------|-----------|
| `Event_NetOut_sendPlayerCommunication` | `sendPlayerCommunication` | Communicator.def (BaseMethods) |
| `Event_NetOut_useAbilityOnGroundTarget` | `useAbilityOnGroundTarget` | SGWPlayer.def (CellMethods) |
| `Event_NetOut_onDialGate` | `onDialGate` | GateTravel.def (CellMethods) |
| `Event_NetOut_onClientVersion` | `onClientVersion` | Account.def (BaseMethods) |
| `Event_NetOut_requestItemData` | (not in .def, client-only) | -- |
| `Event_NetOut_debugStartMinigame` | `gmDebugStartMinigame` | SGWGmPlayer.def |
| `Event_NetOut_callForAid` | `callForAid` | SGWPlayer.def (CellMethods) |
| `Event_NetOut_elementDataRequest` | (not in .def, protocol-level) | -- |

**Note**: `debugStartMinigame` is an anomaly -- camelCase event but maps to `gm`-prefixed method.

#### Pattern 4: PascalCase Event → PascalCase method (rare, exact match)

A few events preserve PascalCase in both event and method names. This appears in organization-related code.

| Event Name | .def Method | .def File |
|------------|-------------|-----------|
| `Event_NetOut_BroadcastMinimapPing` | `BroadcastMinimapPing` | OrganizationMember.def (CellMethods) |

#### Pattern 5: NetIn Events → exact ClientMethods name (almost always 1:1)

For `Event_NetIn_*` events, the name after the prefix matches the `ClientMethods` entry exactly, including casing. The client strips `Event_NetIn_` and uses the remainder directly.

| Event Name | .def Method | .def File |
|------------|-------------|-----------|
| `Event_NetIn_onMissionUpdate` | `onMissionUpdate` | Missionary.def |
| `Event_NetIn_onPlayerCommunication` | `onPlayerCommunication` | Communicator.def |
| `Event_NetIn_onUpdateItem` | `onUpdateItem` | SGWInventoryManager.def |
| `Event_NetIn_CharacterList` | `onCharacterList` | Account.def |
| `Event_NetIn_CashChanged` | `onCashChanged` | SGWInventoryManager.def |
| `Event_NetIn_BeingAppearance` | `BeingAppearance` | SGWBeing.def (entity) |

**Exception**: `CharacterList` -> `onCharacterList` (adds `on` prefix). Similarly `CashChanged` -> `onCashChanged`. The client event name drops the `on` prefix for some events.

### Summary of Casing Rules

| Pattern | Event Casing | .def Method Casing | Frequency | Example |
|---------|-------------|-------------------|-----------|---------|
| 1. Standard | PascalCase | camelCase | ~60% of NetOut | UseAbility → useAbility |
| 2. GM prefix | PascalCase | gm + PascalCase | ~25% of NetOut | Kill → gmKillTarget |
| 3. Pass-through | camelCase | camelCase | ~10% of NetOut | sendPlayerCommunication → sendPlayerCommunication |
| 4. Exact PascalCase | PascalCase | PascalCase | ~1% of NetOut | BroadcastMinimapPing → BroadcastMinimapPing |
| 5. NetIn direct | Mixed | Exact match | ~90% of NetIn | onMissionUpdate → onMissionUpdate |

---

## Unmapped Event Classification

Events discovered in Ghidra that are not in the mapping tables above are classified as **entity-level** (map to .def methods) or **protocol-level** (Mercury infrastructure, no .def method).

### Entity-Level Events (unmapped, now resolved)

These have `register_NetOut_*` or `register_NetIn_*` functions AND corresponding `SGWNetworkManager_*___EventHandler__vfunc_0` handlers, confirming they are entity-level game actions.

#### NetOut (Client -> Server) - Previously Unmapped

| Event_NetOut Name | .def Method | Entity/Interface | Method Section | Registration Addr |
|-------------------|------------|-----------------|----------------|-------------------|
| `Alloy` | `alloying` | SGWPlayer | CellMethods | 00e4aac0 |
| `ArchiveMailMessage` | `archiveMailMessage` | SGWMailManager | BaseMethods | 00d99440 |
| `BMCancelAuction` | `cancelAuction` | SGWBlackMarketManager | CellMethods | 00e5c4a0 |
| `BMCreateAuction` | `createAuction` | SGWBlackMarketManager | CellMethods | 00e5c200 |
| `BMPlaceBid` | `placeBid` | SGWBlackMarketManager | CellMethods | 00e5c740 |
| `BMSearch` | `search` | SGWBlackMarketManager | CellMethods | 00e5c9e0 |
| `CancelMovie` | `cancelMovie` | SGWPlayer | CellMethods | 00d9b900 |
| `ChangeWeaponState` | `requestHolsterWeapon` | SGWCombatant | CellMethods | 00d9b120 |
| `Craft` | `craft` | SGWPlayer | CellMethods | 00e4a910 |
| `DeleteCharacter` | `deleteCharacter` | Account | BaseMethods | 00d9a400 |
| `DeleteMailMessage` | `deleteMailMessage` | SGWMailManager | BaseMethods | 00d996e0 |
| `DialogButtonChoice` | `dialogButtonChoice` | SGWPlayer | CellMethods | 00cbb8d0 |
| `DuelChallenge` | `sendDuelChallenge` | SGWPlayer | CellMethods | 00cbee10 |
| `DuelForfeit` | `duelForfeit` | SGWPlayer | CellMethods | 00cbefc0 |
| `DuelResponse` | `sendDuelResponse` | SGWPlayer | CellMethods | 00aea580 |
| `GiveBlueprint` | `gmGiveBlueprint` | SGWGmPlayer | CellMethods | 00d972d0 |
| `GiveFaction` | `gmGiveFaction` | SGWGmPlayer | CellMethods | 00d96a30 |
| `GiveGearset` | `gmGiveGearset` | SGWGmPlayer | CellMethods | 00d974b0 |
| `GiveInventory` | `gmGiveInventory` | SGWGmPlayer | CellMethods | 00d97750 |
| `Goto` | `gmGoto` | SGWGmPlayer | CellMethods | 00d90120 |
| `InitialResponse` | `initialResponse` | SGWPlayer | CellMethods | 00cbb720 |
| `Interact` | `interact` | SGWPlayer | CellMethods | 00d979f0 |
| `ListAbilities` | `listAbilities` | SGWGmPlayer | CellMethods | 00d93840 |
| `LogOff` | `logOff` | Account | BaseMethods | 00d9a940 |
| `MinigameContactRequest` | `minigameContactRequest` | MinigamePlayer | CellMethods | 00d92dc0 |
| `MinigameStartCancel` | `minigameStartCancel` | MinigamePlayer | CellMethods | 00d920a0 |
| `OrganizationNote` | `organizationNote` | OrganizationMember | CellMethods | 00d95a60 |
| `OrganizationOfficerNote` | `organizationOfficerNote` | OrganizationMember | CellMethods | 00d95d00 |
| `OrganizationSetRankName` | `organizationSetRankName` | OrganizationMember | CellMethods | 00d96250 |
| `OrganizationSetRankPermissions` | `organizationSetRankPermissions` | OrganizationMember | CellMethods | 00d95fa0 |
| `PayCODForMailMessage` | `payCODForMailMessage` | SGWMailManager | BaseMethods | 00d9a160 |
| `PetChangeStance` | `petChangeStance` | SGWPlayer | CellMethods | 00d3cf00 |
| `PetInvokeAbility` | `petInvokeAbility` | SGWPlayer | CellMethods | 00cbe090 |
| `PetAbilityToggle` | `petAbilityToggle` | SGWPlayer | CellMethods | 00cbe240 |
| `PlayCharacter` | `playCharacter` | Account | BaseMethods | 00d9abe0 |
| `RechargeItems` | `rechargeItems` | SGWPlayer | CellMethods | 00d98720 |
| `RegisterToMinigameHelp` | `registerToMinigameHelp` | MinigamePlayer | CellMethods | 00d91620 |
| `RepairItems` | `repairItems` | SGWPlayer | CellMethods | 00d98480 |
| `RequestCharacterVisuals` | `requestCharacterVisuals` | Account | BaseMethods | 00d9ae80 |
| `RequestMailBody` | `requestMailBody` | SGWMailManager | BaseMethods | 00d991a0 |
| `RequestMailHeaders` | `requestMailHeaders` | SGWMailManager | BaseMethods | 00d98c60 |
| `RequestSpectateList` | `requestSpectateList` | MinigamePlayer | CellMethods | 00d91b60 |
| `Research` | `research` | SGWPlayer | CellMethods | 00e4ac70 |
| `RespecAbility` | `resetMyAbilities` | SGWPlayer | CellMethods | 00aea070 |
| `RespecCraft` | `respecCrafting` | SGWPlayer | CellMethods | 00aea3d0 |
| `ReturnMailMessage` | `returnMailMessage` | SGWMailManager | BaseMethods | 00d99980 |
| `ReverseEngineer` | `reverseEngineer` | SGWPlayer | CellMethods | 00e4ae20 |
| `SendMailMessage` | `sendMailMessage` | SGWMailManager | BaseMethods | 00d98f00 |
| `SetFaction` | `gmSetFaction` | SGWGmPlayer | CellMethods | 00d96cd0 |
| `SetTargetID` | `gmSetTarget` | SGWGmPlayer | CellMethods | 00d9b3c0 |
| `SetTechSkill` | `gmSetTechSkill` | SGWGmPlayer | CellMethods | 00d96f70 |
| `SpendAppliedSciencePoint` | `spendAppliedSciencePoints` | SGWPlayer | CellMethods | 00e4afd0 |
| `SquadSetLootMode` | `squadSetLootMode` | OrganizationMember | CellMethods | 00d96790 |
| `Summon` | `gmSummon` | SGWGmPlayer | CellMethods | 00d90660 |
| `TakeCashFromMailMessage` | `takeCashFromMailMessage` | SGWMailManager | BaseMethods | 00d99c20 |
| `TakeItemFromMailMessage` | `takeItemFromMailMessage` | SGWMailManager | BaseMethods | 00d99ec0 |
| `TestLOS` | `testLOS` | SGWGmPlayer | CellMethods | 00d9bba0 |
| `ToggleCombatLOS` | `toggleCombatLOS` | SGWGmPlayer | CellMethods | 00d9be40 |
| `TrainAbility` | `trainAbility` | SGWPlayer | CellMethods | 00d989c0 |
| `TriggerClientHintedGenericRegion` | `triggerClientHintedGenericRegion` | SGWPlayer | CellMethods | 00d93300 |
| `Unstuck` | `unstuck` | SGWPlayer | CellMethods | 00cbe750 |
| `UpdateRegisterToMinigameHelp` | `updateRegisterToMinigameHelp` | MinigamePlayer | CellMethods | 00d918c0 |
| `Who` | `who` | SGWPlayer | CellMethods | 00cb3bf0 |
| `WorldInstanceReset` | `onWorldInstanceReset` | SGWPlayer | CellMethods | 00cbe5a0 |
| `contactListCreate` | `contactListCreate` | ContactListManager | BaseMethods | 00e63990 |
| `contactListDelete` | `contactListDelete` | ContactListManager | BaseMethods | 00e63cf0 |
| `contactListRename` | `contactListRename` | ContactListManager | BaseMethods | 00e63b40 |
| `contactListAddMembers` | `contactListAddMembers` | ContactListManager | BaseMethods | 00e63ff0 |
| `contactListRemoveMembers` | `contactListRemoveMembers` | ContactListManager | BaseMethods | 00e64140 |
| `contactListFlagsUpdate` | `contactListFlagsUpdate` | ContactListManager | BaseMethods | 00e63ea0 |

#### NetIn (Server -> Client) - Previously Unmapped

| Event_NetIn Name | .def Method | Entity/Interface | Registration Addr |
|------------------|------------|-----------------|-------------------|
| `onPlayerDataLoaded` | `onPlayerDataLoaded` | SGWPlayer | 00d764c0 |
| `onVersionInfo` | `onVersionInfo` | Account | 00d76220 |
| `onSequence` | `onSequence` | SGWPlayer | 00d76f40 |
| `onErrorCode` | `onErrorCode` | SGWPlayer | 00d77f00 |
| `EntityTint` | `entityTint` | SGWPlayer | 00d781a0 |
| `EntityFlags` | `entityFlags` | SGWPlayer | 00d78440 |
| `EntityProperty` | `entityProperty` | SGWPlayer | 00d786e0 |
| `PetAbilities` | `petAbilities` | SGWPlayer | 00d77720 |
| `PetStances` | `petStances` | SGWPlayer | 00d779c0 |
| `PetStanceUpdate` | `petStanceUpdate` | SGWPlayer | 00d77c60 |
| `onBeginAidWait` | `onBeginAidWait` | SGWPlayer | 00d79400 |
| `onEndAidWait` | `onEndAidWait` | SGWPlayer | 00d796a0 |
| `onOverridePerfStatsRate` | `onOverridePerfStatsRate` | SGWPlayer | 00d79940 |
| `onExtraNameUpdate` | `onExtraNameUpdate` | SGWPlayer | 00d79be0 |
| `onExpUpdate` | `onExpUpdate` | SGWPlayer | 00d79e80 |
| `onMaxExpUpdate` | `onMaxExpUpdate` | SGWPlayer | 00d7a120 |
| `onDisplayDHD` | `onDisplayDHD` | SGWPlayer | 00d7a3c0 |
| `onMailHeaderInfo` | `onMailHeaderInfo` | SGWMailManager | 00d7c880 |
| `onMailHeaderRemove` | `onMailHeaderRemove` | SGWMailManager | 00d7cb20 |
| `onMailRead` | `onMailRead` | SGWMailManager | 00d7cdc0 |
| `onSendMailResult` | `onSendMailResult` | SGWMailManager | 00d7d060 |
| `onVaultOpen` | `onVaultOpen` | SGWInventoryManager | 00d7e560 |
| `onTeamVaultOpen` | `onTeamVaultOpen` | SGWInventoryManager | 00d7eaa0 |
| `onCommandVaultOpen` | `onCommandVaultOpen` | SGWInventoryManager | 00d7ed40 |
| `onStoreOpen` | `onStoreOpen` | SGWInventoryManager | 00d7ed40 |
| `onStoreUpdate` | `onStoreUpdate` | SGWInventoryManager | 00d7efe0 |
| `onStoreClose` | `onStoreClose` | SGWInventoryManager | 00d7f280 |
| `InitialInteraction` | `initialInteraction` | SGWPlayer | 00d7fa60 |
| `onCraftingRespecPrompt` | `onCraftingRespecPrompt` | SGWPlayer | 00d7fd00 |
| `onTrainerOpen` | `onTrainerOpen` | SGWPlayer | 00d7ffa0 |
| `LootDisplay` | `lootDisplay` | SGWPlayer | 00d804f0 |
| `TradeState` | `tradeState` | SGWPlayer | 00d80790 |
| `TradeResults` | `tradeResults` | SGWPlayer | 00d80a30 |
| `MissionRewards` | `missionRewards` | Missionary | 00d81c90 |
| `onDHDReply` | `onDHDReply` | SGWPlayer | 00d82c60 |
| `onUpdateDiscipline` | `onUpdateDiscipline` | SGWPlayer | 00d831a0 |
| `onUpdateRacialParadigmLevel` | `onUpdateRacialParadigmLevel` | SGWPlayer | 00d83440 |
| `onUpdateKnownCrafts` | `onUpdateKnownCrafts` | SGWPlayer | 00d836e0 |
| `onUpdateCraftingOptions` | `onUpdateCraftingOptions` | SGWPlayer | 00d83980 |
| `onDisciplineRespec` | `onDisciplineRespec` | SGWPlayer | 00d83c20 |
| `BMOpen` | `onBMOpen` | SGWBlackMarketManager | 00d83ec0 |
| `BMAuctions` | `onBMAuctions` | SGWBlackMarketManager | 00d84160 |
| `BMAuctionUpdate` | `onBMAuctionUpdate` | SGWBlackMarketManager | 00d84400 |
| `BMAuctionRemove` | `onBMAuctionRemove` | SGWBlackMarketManager | 00d846a0 |
| `BMError` | `onBMError` | SGWBlackMarketManager | 00d84940 |
| `onStaticMeshNameUpdate` | `onStaticMeshNameUpdate` | SGWBeing | 00d84be0 |
| `onLevelUpdate` | `onLevelUpdate` | SGWBeing | 00d84e80 |
| `onTargetUpdate` | `onTargetUpdate` | SGWBeing | 00d85120 |
| `onBeingNameUpdate` | `onBeingNameUpdate` | SGWBeing | 00d853c0 |
| `onBeingNameIDUpdate` | `onBeingNameIDUpdate` | SGWBeing | 00d85660 |
| `onEntityMove` | `onEntityMove` | SGWBeing | 00d85900 |
| `onVisible` | `onVisible` | SGWBeing | 00d85ba0 |
| `onClientMapLoad` | `onClientMapLoad` | SGWPlayer | 00d85e40 |
| `onMeleeRangeUpdate` | `onMeleeRangeUpdate` | SGWCombatant | 00d860e0 |
| `onStateFieldUpdate` | `onStateFieldUpdate` | SGWBeing | 00d86380 |
| `onAlignmentUpdate` | `onAlignmentUpdate` | SGWCombatant | 00d86b60 |
| `onFactionUpdate` | `onFactionUpdate` | SGWCombatant | 00d86e00 |
| `setupWorldParameters` | `setupWorldParameters` | SGWPlayer | 00d870a0 |
| `ClearClientHintedGenericRegions` | `clearClientHintedGenericRegions` | SGWPlayer | 00d87b20 |
| `AddClientHintedGenericRegion` | `addClientHintedGenericRegion` | SGWPlayer | 00d87dc0 |
| `StargateTriggerFailed` | `stargateTriggerFailed` | GateTravel | 00d88060 |
| `MapInfo` | `mapInfo` | SGWPlayer | 00d885a0 |
| `ResetMapInfo` | `resetMapInfo` | SGWPlayer | 00d88840 |
| `onArchetypeUpdate` | `onArchetypeUpdate` | SGWCombatant | 00d88ae0 |
| `onKismetEventSetUpdate` | `onKismetEventSetUpdate` | SGWPlayer | 00d88d80 |
| `onRingTransporterList` | `onRingTransporterList` | SGWPlayer | 00d89020 |
| `onCookedDataError` | `onCookedDataError` | SGWPlayer | 00d892c0 |
| `onDuelChallenge` | `onDuelChallenge` | SGWPlayer | 00d89800 |
| `onDuelEntitiesSet` | `onDuelEntitiesSet` | SGWPlayer | 00d89aa0 |
| `onDuelEntitiesRemove` | `onDuelEntitiesRemove` | SGWPlayer | 00d89d40 |
| `onDuelEntitiesClear` | `onDuelEntitiesClear` | SGWPlayer | 00d89fe0 |
| `onLaunchOrganizationCreation` | `onLaunchOrganizationCreation` | OrganizationMember | 00d8c9f0 |
| `onOrganizationNameUpdate` | `onOrganizationNameUpdate` | OrganizationMember | 00d8b4f0 |
| `onOrganizationExperienceUpdate` | `onOrganizationExperienceUpdate` | OrganizationMember | 00d8b790 |
| `onOrganizationNoteUpdate` | `onOrganizationNoteUpdate` | OrganizationMember | 00d8bcd0 |
| `onOrganizationOfficerNoteUpdate` | `onOrganizationOfficerNoteUpdate` | OrganizationMember | 00d8bf70 |
| `onOrganizationRankNameUpdate` | `onOrganizationRankNameUpdate` | OrganizationMember | 00d8c750 |
| `onMemberRankChangedOrganization` | `onMemberRankChangedOrganization` | OrganizationMember | 00d8b250 |
| `onThreatenedMobsUpdate` | `onThreatenedMobsUpdate` | SGWCombatant | 00d8d480 |
| `onClientChallenge` | `onClientChallenge` | SGWPlayer | 00d8d720 |
| `onRemoteEntityCreate` | `onRemoteEntityCreate` | SGWPlayer | 00d8d9c0 |
| `onRemoteEntityMove` | `onRemoteEntityMove` | SGWPlayer | 00d8dc60 |
| `onRemoteEntityRemove` | `onRemoteEntityRemove` | SGWPlayer | 00d8df00 |
| `PlayMovie` | `playMovie` | SGWPlayer | 00d8e1a0 |
| `StopMovie` | `stopMovie` | SGWPlayer | 00d8e440 |
| `onContactListUpdate` | `onContactListUpdate` | ContactListManager | 00d8eec0 |
| `onContactListDelete` | `onContactListDelete` | ContactListManager | 00d8f160 |
| `onContactListAddMembers` | `onContactListAddMembers` | ContactListManager | 00d8f400 |
| `onContactListRemoveMembers` | `onContactListRemoveMembers` | ContactListManager | 00d8f6a0 |
| `onContactListEvent` | `onContactListEvent` | ContactListManager | 00d8f940 |
| `onShowCommandWaypoints` | `onShowCommandWaypoints` | SGWGmPlayer | 00d9c0e0 |
| `onSpaceQueueReady` | `onSpaceQueueReady` | SGWPlayer | 00d9d340 |
| `onSpaceQueued` | `onSpaceQueued` | SGWPlayer | 00d9d5e0 |
| `InteractionType` | `interactionType` | SGWPlayer | 00d9c8c0 |
| `onTimeofDay` | `onTimeofDay` | SGWPlayer | 00cbacd0 |

### Protocol-Level Events

These events do NOT correspond to entity .def methods. They map to Mercury protocol-level messages or client-internal systems. Identified by: no .def `<Exposed/>` match, and/or mapped to a `CLIENTMSG_*` / `BASEMSG_*` constant.

| Event Name | Classification | Notes |
|------------|---------------|-------|
| `Event_NetOut_ClientReady` | Protocol | Maps to `CLIENTMSG_ENABLE_ENTITIES` (0x08). No .def method. |
| `Event_NetOut_versionInfoRequest` | Protocol | Pre-login handshake. Registration at 0x00449b10 (separate from main game module). |
| `Event_NetOut_elementDataRequest` | Protocol | Client cache / cooked data request. Registration at 0x00cfdf10. |
| `Event_NetOut_PerfStats` | Protocol | Performance telemetry. Registration at 0x00d9ce00. Not a game action. |
| `Event_NetOut_SystemOptions` | Protocol | Client system settings sync. Registration at 0x00d9cb60. |
| `Event_NetOut_onClientChallengeResponse` | Protocol | Anti-cheat challenge response. Registration at 0x00d9d0a0. |
| `Event_NetOut_PvPOrganizationLeaveResponse` | Protocol | Response to server query, not player-initiated. Registration at 0x00d9d880. |
| `Event_NetIn_AccountLoginSuccess` | Protocol | Auth layer. No entity exists yet. String at 0x019bd7d0. |
| `Event_NetIn_ServerSelectSuccess` | Protocol | Auth layer. Shard selection. String at 0x019bd7f0. |
| `Event_NetIn_LoginFailure` | Protocol | Auth layer. Error response. String at 0x019bd810. |

### Remaining Analysis

All 253 NetOut events and 167 NetIn events from Ghidra strings now have corresponding `register_NetOut_*` / `register_NetIn_*` functions. Every event with a registration function has been classified as either entity-level (with .def method mapping) or protocol-level. No events remain unclassified.

---

## Ghidra String Addresses

Every `Event_NetOut_*` and `Event_NetIn_*` string appears in the binary's `.rdata` section. Below is the complete address table.

> **Note**: Some event names appear at multiple addresses (duplicates). The primary definition address is listed; duplicate references exist in vtable/registration table regions.

### NetOut String Addresses (Client -> Server)

| Event Name | Primary Address | Dup Address |
|------------|----------------|-------------|
| `Event_NetOut_AbandonMission` | 019baea4 | -- |
| `Event_NetOut_AbilityDebug` | 019b2fe8 | -- |
| `Event_NetOut_AddBehaviorEventSet` | 019b3b38 | -- |
| `Event_NetOut_BroadcastMinimapPing` | 0195fb28 | -- |
| `Event_NetOut_CancelMovie` | (not in string dump) | -- |
| `Event_NetOut_ChangeCoverStanceWeight` | 019b409c | -- |
| `Event_NetOut_ChangeCoverWeight` | 019b4060 | -- |
| `Event_NetOut_ChatBan` | 019b9ae4 | -- |
| `Event_NetOut_ChatFriend` | 019b30c0 | -- |
| `Event_NetOut_ChatIgnore` | 019b99e0 | -- |
| `Event_NetOut_ChatJoin` | 019b9920 | -- |
| `Event_NetOut_ChatKick` | 019b9a64 | -- |
| `Event_NetOut_ChatLeave` | 019b994c | -- |
| `Event_NetOut_ChatList` | 019b9a0c | -- |
| `Event_NetOut_ChatMute` | 019b9a38 | -- |
| `Event_NetOut_ChatOp` | 019b9a90 | -- |
| `Event_NetOut_ChatPassword` | 019b9ae4 | -- |
| `Event_NetOut_ChatSetAFKMessage` | 019b9978 | -- |
| `Event_NetOut_ChatSetDNDMessage` | 019b99ac | -- |
| `Event_NetOut_ChosenRewards` | 019baed4 | -- |
| `Event_NetOut_ClientReady` | 019be6d0 | -- |
| `Event_NetOut_CombatDebug` | 019b3020 | 019be644 |
| `Event_NetOut_CombatDebugVerbose` | 019b305c | 019be660 |
| `Event_NetOut_ConfirmEffect` | 019b4610 | -- |
| `Event_NetOut_CreateCharacter` | 019bbdb0 | -- |
| `Event_NetOut_DHD` | 019be1c4 | -- |
| `Event_NetOut_DebugAbilityOnMob` | 019b3d1c | -- |
| `Event_NetOut_DebugBehaviorsOnMob` | 019b3d50 | -- |
| `Event_NetOut_DebugEvents` | 019b3dcc | -- |
| `Event_NetOut_DebugInteract` | 019b3a08 | -- |
| `Event_NetOut_DebugPathsOnMob` | 019b3d90 | -- |
| `Event_NetOut_Despawn` | 019b3c00 | -- |
| `Event_NetOut_DialogButtonChoice` | 019b3e30 | -- |
| `Event_NetOut_DuelChallenge` | 019b4448 | -- |
| `Event_NetOut_DuelForfeit` | 019b4478 | -- |
| `Event_NetOut_DuelResponse` | 0195fb58 | -- |
| `Event_NetOut_EmitBehaviorEventOnMob` | 019b3b00 | -- |
| `Event_NetOut_EndMinigame` | 019be408 | -- |
| `Event_NetOut_EnterErrorAIState` | 019b44dc | -- |
| `Event_NetOut_ExitErrorAIState` | 019b4510 | -- |
| `Event_NetOut_GetItemInfo` | 019b3090 | -- |
| `Event_NetOut_GetMobAttribute` | 019b3cc4 | -- |
| `Event_NetOut_GiveAbility` | 019b37c4 | -- |
| `Event_NetOut_GiveAllAbilities` | 019b36b0 | -- |
| `Event_NetOut_GiveAmmo` | 019b3794 | -- |
| `Event_NetOut_GiveAppliedSciencePoints` | 019b43d0 | -- |
| `Event_NetOut_GiveExpertise` | 019b43a0 | -- |
| `Event_NetOut_GiveItem` | 019b3768 | -- |
| `Event_NetOut_GiveMinigameContact` | 019b3230 | -- |
| `Event_NetOut_GiveNaqahdah` | 019b373c | -- |
| `Event_NetOut_GiveRacialParadigmLevels` | 019b440c | -- |
| `Event_NetOut_GiveRespawner` | 019b3820 | -- |
| `Event_NetOut_GiveStargateAddress` | 019b3f8c | -- |
| `Event_NetOut_GiveTrainingPoints` | 019b3b00 | -- |
| `Event_NetOut_GiveXp` | 019b3654 | -- |
| `Event_NetOut_Goto` | 019be1d8 | -- |
| `Event_NetOut_GotoLocation` | 019be1ec | 019b2e34 |
| `Event_NetOut_GotoXYZ` | 019b2e08 | 019be208 |
| `Event_NetOut_HealDebug` | 019b305c | -- |
| `Event_NetOut_InitialResponse` | 019b3dfc | -- |
| `Event_NetOut_Invisible` | 019b3f28 | -- |
| `Event_NetOut_Kill` | 019b3848 | -- |
| `Event_NetOut_ListAbilities` | 019be628 | -- |
| `Event_NetOut_ListInteractions` | 019b3a98 | -- |
| `Event_NetOut_ListItems` | 019b2ef4 | -- |
| `Event_NetOut_LoadAbility` | 019b412c | -- |
| `Event_NetOut_LoadAbilitySet` | 019b4158 | -- |
| `Event_NetOut_LoadBehavior` | 019b4188 | -- |
| `Event_NetOut_LoadConstants` | 019b40cc | -- |
| `Event_NetOut_LoadInteractionSet` | 019b4218 | -- |
| `Event_NetOut_LoadItem` | 019b4244 | -- |
| `Event_NetOut_LoadMOB` | 019b41e4 | -- |
| `Event_NetOut_LoadMission` | 019b4274 | -- |
| `Event_NetOut_LoadNACSI` | 019b4158 | -- |
| `Event_NetOut_MinigameCallAbort` | 019be500 | -- |
| `Event_NetOut_MinigameCallAccept` | 019be520 | -- |
| `Event_NetOut_MinigameCallDecline` | 019be540 | -- |
| `Event_NetOut_MinigameCallRequest` | 019be4dc | -- |
| `Event_NetOut_MinigameComplete` | 019b31f8 | 019be384 |
| `Event_NetOut_MinigameContactRequest` | 019be564 | -- |
| `Event_NetOut_MinigameStartCancel` | 019be4b8 | -- |
| `Event_NetOut_MissionAbandon` | 019b2fb8 | -- |
| `Event_NetOut_MissionAdvance` | 019b3560 | -- |
| `Event_NetOut_MissionAssign` | 019b33e0 | -- |
| `Event_NetOut_MissionClear` | 019b3410 | -- |
| `Event_NetOut_MissionClearActive` | 019b347c | -- |
| `Event_NetOut_MissionClearHistory` | 019b34bc | -- |
| `Event_NetOut_MissionComplete` | 019b35f4 | -- |
| `Event_NetOut_MissionDetails` | 019b3530 | -- |
| `Event_NetOut_MissionList` | 019b34f4 | -- |
| `Event_NetOut_MissionListFull` | 019b3530 | -- |
| `Event_NetOut_MissionReset` | 019b3590 | -- |
| `Event_NetOut_MissionSetAvailable` | 019b362c | -- |
| `Event_NetOut_MobData` | 019b3e90 | 019be2fc |
| `Event_NetOut_MoveItem` | (via register fn) | -- |
| `Event_NetOut_OrganizationCreation` | 0195fb88 | -- |
| `Event_NetOut_Petition` | 019b9bac | -- |
| `Event_NetOut_Physics` | 019b3cc4 | -- |
| `Event_NetOut_RechargeItem` | 019b3c5c | -- |
| `Event_NetOut_RegenerateCoverLinks` | 019b402c | -- |
| `Event_NetOut_ReloadInventory` | 019b2ea0 | 019be244 |
| `Event_NetOut_ReloadOrganizations` | 019b2e6c | 019be220 |
| `Event_NetOut_RemoveBehaviorEventSet` | 019b3ba8 | -- |
| `Event_NetOut_RemoveMinigameContact` | 019b3268 | 019be3ec |
| `Event_NetOut_RemoveStargateAddress` | 019b3fc4 | -- |
| `Event_NetOut_RepairItem` | 019b3380 | -- |
| `Event_NetOut_RequestActiveSlotChange` | 019be2e4 | -- |
| `Event_NetOut_RequestAmmoChange` | 019b430c | -- |
| `Event_NetOut_RequestReload` | 019b40cc | -- |
| `Event_NetOut_ResetAbilities` | 019b36b0 | -- |
| `Event_NetOut_Respawn` | 019b33e0 | -- |
| `Event_NetOut_Respec` | 019b370c | -- |
| `Event_NetOut_RespecAbility` | 0195fac0 | -- |
| `Event_NetOut_RespecCraft` | 0195fb28 | -- |
| `Event_NetOut_SendGMShout` | 019b9b80 | -- |
| `Event_NetOut_SetAutoCycle` | 019b3ec8 | -- |
| `Event_NetOut_SetCrouched` | 019be6b4 | -- |
| `Event_NetOut_SetFlag` | 019b39dc | -- |
| `Event_NetOut_SetFocus` | 019b3980 | -- |
| `Event_NetOut_SetFocusMax` | 019b39b0 | -- |
| `Event_NetOut_SetGodMode` | 019b3874 | -- |
| `Event_NetOut_SetHealth` | 019b38f8 | -- |
| `Event_NetOut_SetHealthMax` | 019b3954 | -- |
| `Event_NetOut_SetHideGM` | 019b2ec8 | 019be278 |
| `Event_NetOut_SetInstanceFlag` | 019b45ac | -- |
| `Event_NetOut_SetLevel` | 019b3680 | -- |
| `Event_NetOut_SetMobAbilitySet` | 019b3acc | -- |
| `Event_NetOut_SetMobAttribute` | 019b3c90 | -- |
| `Event_NetOut_SetMobStance` | 019b3bd8 | -- |
| `Event_NetOut_SetMobVariable` | 019b42a4 | -- |
| `Event_NetOut_SetMovementType` | 019b3efc | -- |
| `Event_NetOut_SetNoAggro` | 019b38a0 | -- |
| `Event_NetOut_SetNoXP` | 019b38cc | -- |
| `Event_NetOut_SetRingTransporterDestination` | 0195fa78 | -- |
| `Event_NetOut_SetSpeed` | 019b38f8 | -- |
| `Event_NetOut_SetTarget` | 019b3a08 | -- |
| `Event_NetOut_ShareMission` | 019b2f20 | -- |
| `Event_NetOut_ShareMissionResponse` | 019b2f50 | -- |
| `Event_NetOut_ShowFlag` | 019b3a64 | -- |
| `Event_NetOut_ShowIP` | 019b32c0 | -- |
| `Event_NetOut_ShowInstanceFlag` | 019b4578 | -- |
| `Event_NetOut_ShowInventory` | 019b32f0 | -- |
| `Event_NetOut_ShowMobCount` | 019b3298 | -- |
| `Event_NetOut_ShowNavigation` | 019b45e0 | -- |
| `Event_NetOut_ShowPlayer` | 019b331c | -- |
| `Event_NetOut_ShowPointSet` | 019b3ff4 | -- |
| `Event_NetOut_ShowRotation` | 019b3380 | -- |
| `Event_NetOut_ShowTargetLocation` | 019b3350 | -- |
| `Event_NetOut_Spawn` | 019b3c00 | -- |
| `Event_NetOut_SpectateMinigame` | 019be498 | -- |
| `Event_NetOut_StartMinigame` | 019be3ec | -- |
| `Event_NetOut_Summon` | 019be290 | -- |
| `Event_NetOut_UseAbility` | 019b37f0 | 019be5e8 |
| `Event_NetOut_Users` | 019b2ec8 | 019be264 |
| `Event_NetOut_Who` | 019b3114 | -- |
| `Event_NetOut_WorldInstanceReset` | 019b4374 | -- |
| `Event_NetOut_XRayEyes` | 019b3f28 | -- |
| `Event_NetOut_callForAid` | 0195f9c0 | -- |
| `Event_NetOut_debugJoinMinigame` | 019b31c4 | 019be364 |
| `Event_NetOut_debugMinigameInstance` | 019acf08 | 019be31c |
| `Event_NetOut_debugSpectateMinigame` | 019b3148 | 019be340 |
| `Event_NetOut_debugStartMinigame` | 019b3114 | 019be2fc |
| `Event_NetOut_elementDataRequest` | 019b9bac | -- |
| `Event_NetOut_onClientVersion` | (via register fn) | -- |
| `Event_NetOut_onDialGate` | 019be588 | -- |
| `Event_NetOut_onSpaceQueueReadyResponse` | 0195fa34 | -- |
| `Event_NetOut_onSpaceQueueStatus` | 019b4510 | -- |
| `Event_NetOut_onSpaceQueuedResponse` | 0195f9f4 | -- |
| `Event_NetOut_onStrikeTeamResponse` | 019be0d8 | -- |
| `Event_NetOut_requestItemData` | 019bccb0 | -- |
| `Event_NetOut_sendPlayerCommunication` | 019b9b14 | -- |
| `Event_NetOut_useAbilityOnGroundTarget` | 019bb70c | 019be600 |
| `Event_NetOut_versionInfoRequest` | 017fba14 | -- |

### NetIn String Addresses (Server -> Client)

| Event Name | Primary Address | Dup Address |
|------------|----------------|-------------|
| `Event_NetIn_AccountLoginSuccess` | 019bd7d0 | -- |
| `Event_NetIn_AddClientHintedGenericRegion` | 019bdb04 | -- |
| `Event_NetIn_AddOrUpdateMinigameHelper` | 019bd1d4 | -- |
| `Event_NetIn_BMAuctionRemove` | 019bd874 | -- |
| `Event_NetIn_BMAuctionUpdate` | 019bd858 | -- |
| `Event_NetIn_BMAuctions` | 019bd840 | -- |
| `Event_NetIn_BMError` | 019bd890 | -- |
| `Event_NetIn_BMOpen` | 019bd82c | -- |
| `Event_NetIn_BeingAppearance` | 019bd670 | 019c002c |
| `Event_NetIn_CashChanged` | 019bd3d0 | -- |
| `Event_NetIn_CharacterCreateFailed` | 019bcfb4 | 019bfd04 |
| `Event_NetIn_CharacterList` | 019bcf78 | 019bfc94 |
| `Event_NetIn_CharacterVisuals` | 019bcf94 | 019bfcc8 |
| `Event_NetIn_ClearClientHintedGenericRegions` | 019bdad8 | -- |
| `Event_NetIn_DialogDisplay` | 019bb4fc | 019bd4c0 |
| `Event_NetIn_EffectUserData` | 019bd4a4 | -- |
| `Event_NetIn_EntityFlags` | 019bcf44 | 019bfdfc |
| `Event_NetIn_EntityProperty` | 019bcf5c | 019bfe38 |
| `Event_NetIn_EntityTint` | 019bcf2c | -- |
| `Event_NetIn_InitialInteraction` | 019bd4dc | -- |
| `Event_NetIn_InteractionType` | 019bf83c | 019c012c |
| `Event_NetIn_KnownAbilitiesUpdate` | 019bcec4 | -- |
| `Event_NetIn_LoginFailure` | 019bd810 | -- |
| `Event_NetIn_LootDisplay` | 019bd558 | -- |
| `Event_NetIn_MapInfo` | 019bdb74 | -- |
| `Event_NetIn_MinigameCallAbort` | 019bd260 | -- |
| `Event_NetIn_MinigameCallDisplay` | 019bd220 | -- |
| `Event_NetIn_MinigameCallResult` | 019bd240 | -- |
| `Event_NetIn_MissionOffer` | 019bd618 | -- |
| `Event_NetIn_MissionRewards` | 019bd654 | -- |
| `Event_NetIn_MissionSharedOffer` | 019bd634 | -- |
| `Event_NetIn_PetAbilities` | 019bcec4 | -- |
| `Event_NetIn_PetStanceUpdate` | 019bcef8 | -- |
| `Event_NetIn_PetStances` | 019bcee0 | -- |
| `Event_NetIn_PlayMovie` | 019be060 | -- |
| `Event_NetIn_PvPOrganizationLeaveRequest` | 019be090 | -- |
| `Event_NetIn_RemoveMinigameHelper` | 019bd1fc | -- |
| `Event_NetIn_ResetMapInfo` | 019bdb88 | -- |
| `Event_NetIn_ServerSelectSuccess` | 019bd7f0 | -- |
| `Event_NetIn_ShowMinigameContact` | 019bd280 | -- |
| `Event_NetIn_ShowNavigation` | 019be1a8 | -- |
| `Event_NetIn_StargateRotationOverride` | 019bdab0 | -- |
| `Event_NetIn_StargateTriggerFailed` | 019bdb30 | -- |
| `Event_NetIn_StopMovie` | 019be078 | -- |
| `Event_NetIn_TimerUpdate` | 019bd48c | -- |
| `Event_NetIn_TradeResults` | 019bd588 | -- |
| `Event_NetIn_TradeState` | 019bd570 | -- |
| `Event_NetIn_AbilityTreeInfo` | 019bd53c | -- |
| `Event_NetIn_onActiveSlotUpdate` | 019bd334 | -- |
| `Event_NetIn_onAggressionOverrideCleared` | 019bdf8c | -- |
| `Event_NetIn_onAggressionOverrideUpdate` | 019bdf64 | -- |
| `Event_NetIn_onAlignmentUpdate` | 019bda0c | 019c0354 |
| `Event_NetIn_onArchetypeUpdate` | 019bdbc4 | -- |
| `Event_NetIn_onBeginAidWait` | 019bcffc | -- |
| `Event_NetIn_onBeingNameIDUpdate` | 019bd920 | 019c021c |
| `Event_NetIn_onBeingNameUpdate` | 019bd900 | 019c01dc |
| `Event_NetIn_onChatJoined` | 019bd6a4 | -- |
| `Event_NetIn_onChatLeft` | 019bd6dc | -- |
| `Event_NetIn_onClientChallenge` | 019bdff8 | -- |
| `Event_NetIn_onClientMapLoad` | 019bd974 | -- |
| `Event_NetIn_onCommandVaultOpen` | 019bd41c | -- |
| `Event_NetIn_onContactListAddMembers` | 019be13c | -- |
| `Event_NetIn_onContactListDelete` | 019be11c | -- |
| `Event_NetIn_onContactListEvent` | 019be188 | -- |
| `Event_NetIn_onContactListRemoveMembers` | 019be160 | -- |
| `Event_NetIn_onContactListUpdate` | 019be0fc | -- |
| `Event_NetIn_onContainerInfo` | 019bd318 | -- |
| `Event_NetIn_onCookedDataError` | 019bdc0c | 019bfdc0 |
| `Event_NetIn_onCraftingRespecPrompt` | 019bd4fc | -- |
| `Event_NetIn_onDHDReply` | 019bd6f4 | -- |
| `Event_NetIn_onCharacterLoadFailed` | 019bcfd8 | 019bfd48 |
| `Event_NetIn_onDisableShowPath` | 019bf81c | -- |
| `Event_NetIn_onDisciplineRespec` | 019bd7d0 | -- |
| `Event_NetIn_onDisplayDHD` | 019bd0c4 | -- |
| `Event_NetIn_onDuelChallenge` | 019bdc44 | -- |
| `Event_NetIn_onDuelEntitiesClear` | 019bdca4 | -- |
| `Event_NetIn_onDuelEntitiesRemove` | 019bdc80 | -- |
| `Event_NetIn_onDuelEntitiesSet` | 019bdc60 | -- |
| `Event_NetIn_onEffectResults` | 019bce84 | 019bfff4 |
| `Event_NetIn_onEndAidWait` | 019bd018 | -- |
| `Event_NetIn_onEndMinigame` | 019bd0fc | -- |
| `Event_NetIn_onEntityMove` | 019bd940 | 019c00b4 |
| `Event_NetIn_onErrorCode` | 019bcf14 | 019bff94 |
| `Event_NetIn_onExpUpdate` | 019bd090 | -- |
| `Event_NetIn_onExtraNameUpdate` | 019bd070 | -- |
| `Event_NetIn_onFactionUpdate` | 019bda48 | 019c03cc |
| `Event_NetIn_onKismetEventSetUpdate` | 019bdbe8 | 019c03cc |
| `Event_NetIn_onLOSResult` | 019bd68c | -- |
| `Event_NetIn_onLaunchOrganizationCreation` | 019bdf1c | -- |
| `Event_NetIn_onLevelUpdate` | 019bd8c8 | 019c016c |
| `Event_NetIn_onLocalizedCommunication` | 019bce20 | 019bfefc |
| `Event_NetIn_onMailHeaderInfo` | 019bd2a0 | -- |
| `Event_NetIn_onMailHeaderRemove` | 019bd2c0 | -- |
| `Event_NetIn_onMailRead` | 019bd2e0 | -- |
| `Event_NetIn_onMaxExpUpdate` | 019bd0a8 | -- |
| `Event_NetIn_onMeleeRangeUpdate` | 019bd990 | 019c025c |
| `Event_NetIn_onMemberJoinedOrganization` | 019bdd2c | -- |
| `Event_NetIn_onMemberLeftOrganization` | 019bdd7c | -- |
| `Event_NetIn_onMemberRankChangedOrganization` | 019bdda4 | -- |
| `Event_NetIn_onMinigameRegistrationInfo` | 019bd1d4 | -- |
| `Event_NetIn_onMinigameRegistrationPrompt` | 019bd180 | -- |
| `Event_NetIn_onMissionUpdate` | 019bd5a4 | -- |
| `Event_NetIn_onNickChanged` | 019bd6c0 | -- |
| `Event_NetIn_onObjectiveUpdate` | 019bd5dc | -- |
| `Event_NetIn_onOrganizationCashUpdate` | 019bdec8 | -- |
| `Event_NetIn_onOrganizationExperienceUpdate` | 019bde24 | -- |
| `Event_NetIn_onOrganizationInvite` | 019bdcc4 | -- |
| `Event_NetIn_onOrganizationJoined` | 019bdce8 | -- |
| `Event_NetIn_onOrganizationLeft` | 019bdd0c | -- |
| `Event_NetIn_onOrganizationMOTDUpdate` | 019bde4c | -- |
| `Event_NetIn_onOrganizationNameUpdate` | 019bddd0 | -- |
| `Event_NetIn_onOrganizationNoteUpdate` | 019bde74 | -- |
| `Event_NetIn_onOrganizationOfficerNoteUpdate` | 019bdea0 | -- |
| `Event_NetIn_onOrganizationRankNameUpdate` | 019bdef0 | -- |
| `Event_NetIn_onOrganizationRankUpdate` | 019bdec8 | -- |
| `Event_NetIn_onOrganizationRosterInfo` | 019bdd54 | -- |
| `Event_NetIn_onOverridePerfStatsRate` | 019bd04c | -- |
| `Event_NetIn_onPlayerCommunication` | 019bcdfc | 019bfeb4 |
| `Event_NetIn_onPlayerDataLoaded` | 019bcddc | -- |
| `Event_NetIn_onRefreshItem` | 019bd38c | -- |
| `Event_NetIn_onRemoteEntityCreate` | 019bdff8 | -- |
| `Event_NetIn_onRemoteEntityMove` | 019be01c | -- |
| `Event_NetIn_onRemoteEntityRemove` | 019be03c | -- |
| `Event_NetIn_onRemoveItem` | 019bd354 | -- |
| `Event_NetIn_onRingTransporterList` | 019bdbe8 | -- |
| `Event_NetIn_onSendMailResult` | 019bd318 | -- |
| `Event_NetIn_onSequence` | 019bce84 | 019bffc4 |
| `Event_NetIn_onSetTarget` | 019bdc2c | -- |
| `Event_NetIn_onShowCommandWaypoints` | 019bf7e0 | -- |
| `Event_NetIn_onShowPath` | 019bf804 | -- |
| `Event_NetIn_onSpaceQueueReady` | 019bfae8 | -- |
| `Event_NetIn_onSpaceQueued` | 019bfb30 | -- |
| `Event_NetIn_onSpectateList` | 019bd164 | -- |
| `Event_NetIn_onSquadLootType` | 019bdf48 | -- |
| `Event_NetIn_onStargatePassage` | 019bdb54 | -- |
| `Event_NetIn_onStartMinigame` | 019bd0e0 | -- |
| `Event_NetIn_onStartMinigameDialog` | 019bd118 | -- |
| `Event_NetIn_onStartMinigameDialogClose` | 019bd13c | -- |
| `Event_NetIn_onStatBaseUpdate` | 019bd9ec | 019c0314 |
| `Event_NetIn_onStatUpdate` | 019bd9d0 | 019c02dc |
| `Event_NetIn_onStateFieldUpdate` | 019bd9b0 | 019c029c |
| `Event_NetIn_onStaticMeshNameUpdate` | 019bd8a4 | 019c0064 |
| `Event_NetIn_onStepUpdate` | 019bd5c0 | -- |
| `Event_NetIn_onStoreClose` | 019bd470 | -- |
| `Event_NetIn_onStoreOpen` | 019bd43c | -- |
| `Event_NetIn_onStoreUpdate` | 019bd454 | -- |
| `Event_NetIn_onStrikeTeamUpdate` | 019be0b8 | -- |
| `Event_NetIn_onSystemCommunication` | 019bce48 | 019bff4c |
| `Event_NetIn_onTargetUpdate` | 019bd8e4 | 019c01a4 |
| `Event_NetIn_onTaskUpdate` | 019bd5fc | -- |
| `Event_NetIn_onTeamVaultOpen` | 019bd400 | -- |
| `Event_NetIn_onTellSent` | 019bd70c | -- |
| `Event_NetIn_onThreatenedMobsUpdate` | 019bdfb4 | -- |
| `Event_NetIn_onTimeofDay` | 019b3cf0 | 019bd034 |
| `Event_NetIn_onTrainerOpen` | 019bd520 | -- |
| `Event_NetIn_onUpdateCraftingOptions` | 019bd78c | -- |
| `Event_NetIn_onUpdateDiscipline` | 019bd724 | -- |
| `Event_NetIn_onUpdateItem` | 019bd370 | -- |
| `Event_NetIn_onUpdateKnownCrafts` | 019bd76c | -- |
| `Event_NetIn_onUpdateRacialParadigmLevel` | 019bd744 | -- |
| `Event_NetIn_onVaultOpen` | 019bd3e8 | -- |
| `Event_NetIn_onVersionInfo` | 019bcdc0 | 019bfd8c, 019bfe7c |
| `Event_NetIn_onVisible` | 019bd95c | 019c00f4 |
| `Event_NetIn_onClearOrgVaultInventory` | 019bd3a8 | -- |
| `Event_NetIn_setupStargateInfo` | 019bda6c | -- |
| `Event_NetIn_setupWorldParameters` | 019bda48 | -- |
| `Event_NetIn_updateStargateAddress` | 019bda8c | -- |

---

## Handler Function Addresses

### Architecture

Each event in the client has a three-part handler chain:

1. **Registration function** (`register_NetOut_*` / `register_NetIn_*`) -- Returns the event name string. Located via Ghidra function search.
2. **EventSignal emit** (`CME_EventSignal_VEvent_NetOut_*___TypedEmitInfo__vfunc_0`) -- The EventSignal system's typed emit function. Serializes arguments.
3. **Network handler** (`SGWNetworkManager_VEvent_NetOut_*___EventHandler__vfunc_0`) -- The SGWNetworkManager handler that serializes the event into a Mercury message bundle.

For NetIn events, the handler chain is reversed: Mercury message arrives, SGWNetworkManager deserializes, fires the Event_NetIn signal, and client systems handle it.

### NetOut Registration + Handler Addresses (representative sample)

| Event Name | register_NetOut Addr | SGWNetworkManager Handler Addr | CME_EventSignal Emit Addr |
|------------|---------------------|-------------------------------|--------------------------|
| `UseAbility` | 00cb7d90 | 00d67b30 | 00c95090 |
| `useAbilityOnGroundTarget` | 00d2cbb0 | 00d67b50 | 00d2b150 |
| `MoveItem` | 00d942c0 | 00d67c30 | (via vtable) |
| `ChatJoin` | 00cfc770 | 00d67c70 | 00cf4750 |
| `ChatLeave` | 00cfc920 | 00d67c90 | 00cf4770 |
| `Kill` | 00cb80f0 | 00d69170 | (via vtable) |
| `CreateCharacter` | 00d37070 | 00d67970 | (via vtable) |
| `Disconnect` | 00d9a6a0 | 00d679b0 | (via vtable) |
| `ClientReady` | 00d93d80 | 00d67b90 | 00d93e60 |
| `GiveItem` | 00cb7880 | 00d68830 | (via vtable) |
| `SetGodMode` | 00cb82a0 | (via vtable) | (via vtable) |
| `Spawn` | 00cba2b0 | (via vtable) | (via vtable) |
| `Despawn` | 00cba460 | 00d690f0 | (via vtable) |
| `OrganizationInvite` | 00d94800 | 00d67e10 | (via vtable) |
| `LootItem` | 00d935a0 | 00d67b70 | (via vtable) |
| `StartMinigame` | 00d910e0 | (via vtable) | (via vtable) |
| `versionInfoRequest` | 00449b10 | (pre-login, separate) | (pre-login, separate) |

### NetIn Registration Addresses (representative sample)

| Event Name | register_NetIn Addr |
|------------|---------------------|
| `onEffectResults` | 00d771e0 |
| `onPlayerCommunication` | 00d76760 |
| `onMissionUpdate` | 00d80cd0 |
| `CharacterList` | 00d78980 |
| `onUpdateItem` | 00d7dae0 |
| `TimerUpdate` | 00d7f520 |
| `KnownAbilitiesUpdate` | 00d77480 |
| `AbilityTreeInfo` | 00d80250 |
| `onContainerInfo` | 00d7d300 |
| `onCashChanged` (CashChanged) | 00d7e2c0 |
| `onOrganizationInvite` | 00d8a280 |
| `setupStargateInfo` | 00d87340 |
| `onStartMinigame` | 00d7a660 |
| `onSetTarget` | 00d89560 |
| `DialogDisplay` | 00d29340 |
| `BMOpen` | 00d83ec0 |
| `onStatUpdate` | 00d86620 |
| `onStatBaseUpdate` | 00d868c0 |

### Key Observations

1. **`versionInfoRequest`** (at 0x00449b10) is registered in a completely separate code region from all other events (which cluster in 0x00ae-0x00e6 range). This confirms it is a pre-login protocol event, not part of the entity system.

2. **All SGWNetworkManager handlers** cluster tightly in the 0x00d679-0x00d695 range, suggesting they are vtable entries in a single class.

3. **Registration functions** are spread across the binary based on their compilation unit: SGWGmPlayer commands cluster around 0x00cb, Communicator around 0x00cfc-0x00cfd, account/player around 0x00d9, etc.

4. **No separate handler functions exist for NetIn events** in the SGWNetworkManager pattern. NetIn handling is done through the EventSignal dispatch system directly -- the `register_NetIn_*` function provides the event name, and client subsystems subscribe to those signals.

---

## Related Documents

- [Message Catalog](../protocol/message-catalog.md) -- Full 420-event catalog with addresses
- [Entity Type Catalog](../engine/entity-type-catalog.md) -- All 36 types with property/method tables
- [Entity Def Guide](../guides/entity-def-guide.md) -- How .def files define the protocol
- [Mercury Wire Format](../protocol/mercury-wire-format.md) -- Wire-level message format
- [CME Framework](../engine/cme-framework.md) -- EventSignal system architecture

## TODO

- [x] ~~Complete mapping for SGWCombatant, SGWAbilityManager, SGWBeing methods~~ → `findings/combat-wire-formats.md`
- [x] ~~Complete mapping for SGWBlackMarketManager, SGWMailManager methods~~ → `findings/black-market-wire-formats.md`, `findings/mail-wire-formats.md`
- [x] ~~Complete mapping for ContactListManager, SGWPoller, EventParticipant methods~~ → `findings/contact-list-wire-formats.md`
- [x] ~~Map remaining SGWPlayer methods (store, trade, duel, crafting)~~ → `findings/trade-wire-formats.md`, `findings/duel-wire-formats.md`, `findings/crafting-wire-formats.md`, `findings/inventory-wire-formats.md`
- [x] Verify event name to .def method name casing conventions via Ghidra → See [Casing Conventions](#casing-conventions) section below
- [x] Determine which unmapped events are protocol-level vs entity-level → See [Unmapped Event Classification](#unmapped-event-classification) section below
- [x] Add Ghidra string addresses for each event (from Script 04 results) → See [Ghidra String Addresses](#ghidra-string-addresses) section below
- [x] Cross-reference with handler function addresses when available → See [Handler Function Addresses](#handler-function-addresses) section below
- [x] ~~Document method index calculation~~ → Sequential order in .def file = wire index. See `findings/entity-property-sync.md`
