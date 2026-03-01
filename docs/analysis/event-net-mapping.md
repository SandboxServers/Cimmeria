# Event_Net to .def Method Mapping

> **Last updated**: 2026-03-01
> **RE Status**: Foundation mapping from Ghidra strings + .def files
> **Sources**: Ghidra string analysis (STATUS.md), `entities/defs/*.def`, `entities/defs/interfaces/*.def`, `src/baseapp/mercury/sgw/messages.hpp`

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

| Category | Total Events | Mapped to .def | Unmapped | Coverage |
|----------|-------------|---------------|----------|----------|
| NetOut (Client->Server) | 253 | ~110 | ~143 | ~43% |
| NetIn (Server->Client) | 167 | ~85 | ~82 | ~51% |
| **Total** | **420** | **~195** | **~225** | **~46%** |

### Unmapped Events

The ~225 unmapped events fall into several categories:

1. **Events in interfaces not yet read**: SGWAbilityManager, SGWCombatant, SGWBeing (interface), SGWBlackMarketManager, SGWMailManager, SGWPoller, ClientCache, ContactListManager, etc.
2. **Protocol-level messages**: Events that map to Mercury protocol messages, not entity methods
3. **Client-internal events**: Some `Event_NetIn` names may not exactly match the `.def` method name (case differences, prefix stripping)
4. **Removed/unused events**: Events in the binary that were never fully wired up

## Related Documents

- [Message Catalog](../protocol/message-catalog.md) -- Full 420-event catalog with addresses
- [Entity Type Catalog](../engine/entity-type-catalog.md) -- All 36 types with property/method tables
- [Entity Def Guide](../guides/entity-def-guide.md) -- How .def files define the protocol
- [Mercury Wire Format](../protocol/mercury-wire-format.md) -- Wire-level message format
- [CME Framework](../engine/cme-framework.md) -- EventSignal system architecture

## TODO

- [ ] Complete mapping for SGWCombatant, SGWAbilityManager, SGWBeing (interface) methods
- [ ] Complete mapping for SGWBlackMarketManager, SGWMailManager methods
- [ ] Complete mapping for ContactListManager, SGWPoller, EventParticipant methods
- [ ] Map the remaining SGWPlayer-specific client/cell methods (store, trade, duel, crafting)
- [ ] Verify event name to .def method name casing conventions via Ghidra
- [ ] Determine which unmapped events are protocol-level vs entity-level
- [ ] Add Ghidra string addresses for each event (from Script 04 results)
- [ ] Cross-reference with handler function addresses when available
- [ ] Document the method index calculation (order in .def file = wire index)
