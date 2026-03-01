# Network Message Catalog

> **Status**: Phase 2 update — wire formats documented for combat & inventory
> **Total messages**: 975 (479 NetOut + 496 NetIn)
> **Source**: Ghidra string search of sgw.exe + entity .def file correlation

This is the **RE-focused** message catalog. For the readable categorized list, see [../network-messages.md](../network-messages.md) and [../technical/network-messages.md](../technical/network-messages.md).

This document adds what those don't have:
- Ghidra string addresses for each event name
- Handler function addresses (populated as scripts 01-04 are run)
- Correlation to entity .def method definitions
- Server implementation status in Cimmeria
- Wire format notes (populated during Phase 2-4 RE work)

## How Events Work

The client uses `CME::EventSignal` for **client-side UI event dispatch only**. The `SGWNetworkManager` class subscribes to `Event_NetOut_*` signals and routes them through the **universal RPC dispatcher** at `0x00c6fc40`, which serializes method arguments using BigWorld's `DataType::addToStream` virtual methods. Incoming Mercury messages trigger `Event_NetIn_*` signals that the UI and game systems subscribe to.

**Key Phase 2 finding**: Event registration functions (e.g., `register_NetOut_UseAbility` at `0x00cb7d90`) simply return a name string — they do NOT contain serialization logic. Wire formats are entirely driven by `.def` file method signatures. See `docs/reverse-engineering/findings/combat-wire-formats.md` for full details.

Wire format for all entity method calls:
- **Cell methods**: `[1 byte: methodID | 0x80] [serialized args per .def]`
- **Base methods**: `[1 byte: methodID | 0xC0] [serialized args per .def]`
- **Client methods**: `[method ID] [serialized args per .def]`

For documented wire formats, see:
- [Combat Wire Formats](../reverse-engineering/findings/combat-wire-formats.md)
- [Inventory Wire Formats](../reverse-engineering/findings/inventory-wire-formats.md)
- [Entity Property Sync](../reverse-engineering/findings/entity-property-sync.md)

---

## Event_NetOut — Client to Server (253 messages)

Messages sent FROM the client TO the server. These correspond to `CellMethods` and `BaseMethods` marked `<Exposed/>` in entity .def files, plus internal protocol messages.

### Summary by System

| System | Count | Server Status |
|--------|-------|---------------|
| Login & Character | 11 | Implemented |
| Combat & Abilities | 15 | Partial (~70%) |
| Pets | 3 | Not implemented |
| Inventory & Items | 17 | Partial (~80%) |
| Missions | 16 | Partial (~40%) |
| Chat & Communication | 14 | Implemented |
| Contact Lists | 6 | Not implemented |
| Organizations | 15 | Stub (~5%) |
| Mail | 9 | Not implemented |
| Trading | 4 | Not implemented |
| Black Market | 4 | Not implemented |
| Crafting & Research | 6 | Not implemented |
| Abilities & Training | 4 | Partial |
| Stargates | 5 | Partial (~20%) |
| Minigames | 14 | Not implemented |
| Dueling | 3 | Not implemented |
| Space Queue | 4 | Not implemented |
| GM Commands | 33 | Partial |
| GM Give Commands | 15 | Partial |
| GM Mob/AI Control | 11 | Partial |
| GM Data Loading | 11 | Partial |
| Debug | 25 | Partial |
| Protocol/Internal | 7 | Implemented |

### Login & Character

| Event Name | String Addr | Handler Addr | .def Method | Impl |
|------------|-------------|--------------|-------------|------|
| `Event_NetOut_CreateCharacter` | 019bbdb0 | TBD | Account.createCharacter | YES |
| `Event_NetOut_DeleteCharacter` | — | TBD | Account.deleteCharacter | YES |
| `Event_NetOut_PlayCharacter` | — | TBD | Account.playCharacter | YES |
| `Event_NetOut_RequestCharacterVisuals` | — | TBD | Account.requestCharacterVisuals | YES |
| `Event_NetOut_Disconnect` | — | TBD | (internal) | YES |
| `Event_NetOut_LogOff` | — | TBD | Account.logOff | YES |
| `Event_NetOut_ClientReady` | 019be6d0 | TBD | SGWPlayer.onClientReady | YES |
| `Event_NetOut_InitialResponse` | 019b3dfc | TBD | SGWSpawnableEntity.onInitialResponse | YES |
| `Event_NetOut_onClientVersion` | — | TBD | Account.onClientVersion | YES |
| `Event_NetOut_onClientChallengeResponse` | — | TBD | (internal) | YES |
| `Event_NetOut_Respawn` | 019b33e0 | TBD | SGWPlayer.respawn | YES |

### Combat & Abilities

| Event Name | String Addr | Handler Addr | .def Method | Impl |
|------------|-------------|--------------|-------------|------|
| `Event_NetOut_UseAbility` | 019b37f0 | TBD | SGWCombatant.useAbility | PARTIAL |
| `Event_NetOut_useAbilityOnGroundTarget` | 019bb70c | TBD | SGWCombatant.useAbilityOnGroundTarget | NO |
| `Event_NetOut_ConfirmEffect` | 019b4610 | TBD | SGWCombatant.confirmEffect | NO |
| `Event_NetOut_SetAutoCycle` | 019b3ec8 | TBD | SGWCombatant.setAutoCycle | PARTIAL |
| `Event_NetOut_SetCrouched` | 019be6b4 | TBD | SGWBeing.setCrouched | NO |
| `Event_NetOut_SetTarget` | 019b3a08 | TBD | SGWBeing.setTarget | YES |
| `Event_NetOut_SetTargetID` | — | TBD | SGWBeing.setTargetID | YES |
| `Event_NetOut_TestLOS` | — | TBD | SGWCombatant.testLOS | NO |
| `Event_NetOut_ToggleCombatLOS` | — | TBD | (debug) | NO |
| `Event_NetOut_SetMovementType` | 019b3efc | TBD | SGWBeing.setMovementType | PARTIAL |
| `Event_NetOut_callForAid` | 0195f9c0 | TBD | SGWPlayer.callForAid | NO |
| `Event_NetOut_ChangeWeaponState` | — | TBD | SGWCombatant.changeWeaponState | NO |
| `Event_NetOut_RequestAmmoChange` | 019b430c | TBD | SGWCombatant.requestAmmoChange | NO |
| `Event_NetOut_RequestReload` | 019b40cc | TBD | SGWCombatant.requestReload | NO |
| `Event_NetOut_RequestActiveSlotChange` | 019be2bc | TBD | SGWInventoryManager.requestActiveSlotChange | NO |

### Inventory & Items

| Event Name | String Addr | Handler Addr | .def Method | Impl |
|------------|-------------|--------------|-------------|------|
| `Event_NetOut_UseItem` | — | TBD | SGWInventoryManager.useItem | PARTIAL |
| `Event_NetOut_MoveItem` | — | TBD | SGWInventoryManager.moveItem | YES |
| `Event_NetOut_RemoveItem` | — | TBD | SGWInventoryManager.removeItem | YES |
| `Event_NetOut_GMRemoveItem` | — | TBD | SGWGmPlayer.gmRemoveItem | NO |
| `Event_NetOut_LootItem` | 019be5e8 | TBD | Lootable.lootItem | PARTIAL |
| `Event_NetOut_GetItemInfo` | 019b30c0 | TBD | (internal) | NO |
| `Event_NetOut_requestItemData` | 019bccb0 | TBD | (internal) | NO |
| `Event_NetOut_PurchaseItems` | — | TBD | SGWInventoryManager.purchaseItems | PARTIAL |
| `Event_NetOut_SellItems` | — | TBD | SGWInventoryManager.sellItems | PARTIAL |
| `Event_NetOut_BuybackItems` | — | TBD | SGWInventoryManager.buybackItems | NO |
| `Event_NetOut_RepairItem` | 019b3380 | TBD | SGWInventoryManager.repairItem | NO |
| `Event_NetOut_RepairItems` | — | TBD | SGWInventoryManager.repairItems | NO |
| `Event_NetOut_RechargeItem` | 019b3c2c | TBD | SGWInventoryManager.rechargeItem | NO |
| `Event_NetOut_RechargeItems` | — | TBD | SGWInventoryManager.rechargeItems | NO |
| `Event_NetOut_ReloadInventory` | 019b2ea0 | TBD | (debug) | NO |
| `Event_NetOut_ShowInventory` | 019b32f0 | TBD | (debug) | NO |
| `Event_NetOut_ListItems` | 019b2ef4 | TBD | (debug) | NO |

### Missions

| Event Name | String Addr | Handler Addr | .def Method | Impl |
|------------|-------------|--------------|-------------|------|
| `Event_NetOut_MissionAssign` | 019b3410 | TBD | Missionary.assignMission | PARTIAL |
| `Event_NetOut_MissionAbandon` | 019b2fb8 | TBD | Missionary.abandonMission | PARTIAL |
| `Event_NetOut_AbandonMission` | 019baea4 | TBD | Missionary.abandonMission | PARTIAL |
| `Event_NetOut_MissionAdvance` | 019b3590 | TBD | Missionary.advanceMission | PARTIAL |
| `Event_NetOut_MissionComplete` | 019b35f4 | TBD | Missionary.completeMission | NO |
| `Event_NetOut_MissionReset` | 019b35c0 | TBD | (debug) | NO |
| `Event_NetOut_MissionClear` | 019b3440 | TBD | (debug) | NO |
| `Event_NetOut_MissionClearActive` | 019b347c | TBD | (debug) | NO |
| `Event_NetOut_MissionClearHistory` | 019b34bc | TBD | (debug) | NO |
| `Event_NetOut_MissionDetails` | 019b3560 | TBD | (debug) | NO |
| `Event_NetOut_MissionList` | 019b34f4 | TBD | (debug) | NO |
| `Event_NetOut_MissionListFull` | 019b3530 | TBD | (debug) | NO |
| `Event_NetOut_MissionSetAvailable` | 019b362c | TBD | (debug) | NO |
| `Event_NetOut_ShareMission` | 019b2f20 | TBD | Missionary.shareMission | NO |
| `Event_NetOut_ShareMissionResponse` | 019b2f50 | TBD | Missionary.shareMissionResponse | NO |
| `Event_NetOut_ChosenRewards` | 019baed4 | TBD | Missionary.chosenRewards | NO |

### Chat & Communication

| Event Name | String Addr | Handler Addr | .def Method | Impl |
|------------|-------------|--------------|-------------|------|
| `Event_NetOut_sendPlayerCommunication` | 019b9b14 | TBD | Communicator.processPlayerCommunication | YES |
| `Event_NetOut_ChatJoin` | 019b9920 | TBD | Communicator.chatJoin | YES |
| `Event_NetOut_ChatLeave` | 019b994c | TBD | Communicator.chatLeave | YES |
| `Event_NetOut_ChatList` | 019b9a38 | TBD | Communicator.chatList | YES |
| `Event_NetOut_ChatIgnore` | 019b99e0 | TBD | Communicator.chatIgnore | YES |
| `Event_NetOut_ChatFriend` | 019b30ec | TBD | Communicator.chatFriend | YES |
| `Event_NetOut_ChatMute` | 019b9a64 | TBD | Communicator.chatMute | YES |
| `Event_NetOut_ChatKick` | 019b9a90 | TBD | Communicator.chatKick | YES |
| `Event_NetOut_ChatOp` | 019b9ab8 | TBD | Communicator.chatOp | YES |
| `Event_NetOut_ChatBan` | 019b9ae4 | TBD | Communicator.chatBan | YES |
| `Event_NetOut_ChatPassword` | 019b9b14 | TBD | Communicator.chatPassword | YES |
| `Event_NetOut_ChatSetAFKMessage` | 019b9978 | TBD | Communicator.chatSetAFKMessage | YES |
| `Event_NetOut_ChatSetDNDMessage` | 019b99ac | TBD | Communicator.chatSetDNDMessage | YES |
| `Event_NetOut_SendGMShout` | 019b9b50 | TBD | SGWGmPlayer.sendGMShout | YES |

### Organizations

| Event Name | String Addr | Handler Addr | .def Method | Impl |
|------------|-------------|--------------|-------------|------|
| `Event_NetOut_OrganizationCreation` | 0195fb88 | TBD | OrganizationMember.createOrganization | NO |
| `Event_NetOut_OrganizationInvite` | — | TBD | OrganizationMember.organizationInvite | NO |
| `Event_NetOut_OrganizationInviteByType` | — | TBD | OrganizationMember.organizationInviteByType | NO |
| `Event_NetOut_OrganizationInviteResponse` | — | TBD | OrganizationMember.organizationInviteResponse | NO |
| `Event_NetOut_OrganizationLeave` | — | TBD | OrganizationMember.organizationLeave | NO |
| `Event_NetOut_OrganizationKick` | — | TBD | OrganizationMember.organizationKick | NO |
| `Event_NetOut_OrganizationRankChange` | — | TBD | OrganizationMember.organizationRankChange | NO |
| `Event_NetOut_OrganizationSetRankName` | — | TBD | OrganizationMember.setRankName | NO |
| `Event_NetOut_OrganizationSetRankPermissions` | — | TBD | OrganizationMember.setRankPermissions | NO |
| `Event_NetOut_OrganizationMOTD` | — | TBD | OrganizationMember.setMOTD | NO |
| `Event_NetOut_OrganizationNote` | — | TBD | OrganizationMember.setNote | NO |
| `Event_NetOut_OrganizationOfficerNote` | — | TBD | OrganizationMember.setOfficerNote | NO |
| `Event_NetOut_OrganizationTransferCash` | — | TBD | OrganizationMember.transferCash | NO |
| `Event_NetOut_ReloadOrganizations` | 019b2e6c | TBD | (debug) | NO |
| `Event_NetOut_PvPOrganizationLeaveResponse` | — | TBD | OrganizationMember.pvpLeaveResponse | NO |

### Crafting & Research

| Event Name | String Addr | Handler Addr | .def Method | Impl |
|------------|-------------|--------------|-------------|------|
| `Event_NetOut_Craft` | — | TBD | SGWPlayer.craft | NO |
| `Event_NetOut_Alloy` | — | TBD | SGWPlayer.alloy | NO |
| `Event_NetOut_Research` | — | TBD | SGWPlayer.research | NO |
| `Event_NetOut_ReverseEngineer` | — | TBD | SGWPlayer.reverseEngineer | NO |
| `Event_NetOut_RespecCraft` | 0195fb28 | TBD | SGWPlayer.respecCraft | NO |
| `Event_NetOut_SpendAppliedSciencePoint` | — | TBD | SGWPlayer.spendAppliedSciencePoint | NO |

### Mail

| Event Name | String Addr | Handler Addr | .def Method | Impl |
|------------|-------------|--------------|-------------|------|
| `Event_NetOut_SendMailMessage` | — | TBD | SGWMailManager.sendMail | NO |
| `Event_NetOut_RequestMailHeaders` | — | TBD | SGWMailManager.requestMailHeaders | NO |
| `Event_NetOut_RequestMailBody` | — | TBD | SGWMailManager.requestMailBody | NO |
| `Event_NetOut_DeleteMailMessage` | — | TBD | SGWMailManager.deleteMail | NO |
| `Event_NetOut_ArchiveMailMessage` | — | TBD | SGWMailManager.archiveMail | NO |
| `Event_NetOut_ReturnMailMessage` | — | TBD | SGWMailManager.returnMail | NO |
| `Event_NetOut_TakeItemFromMailMessage` | — | TBD | SGWMailManager.takeItem | NO |
| `Event_NetOut_TakeCashFromMailMessage` | — | TBD | SGWMailManager.takeCash | NO |
| `Event_NetOut_PayCODForMailMessage` | — | TBD | SGWMailManager.payCOD | NO |

### Black Market

| Event Name | String Addr | Handler Addr | .def Method | Impl |
|------------|-------------|--------------|-------------|------|
| `Event_NetOut_BMCreateAuction` | — | TBD | SGWBlackMarketManager.createAuction | NO |
| `Event_NetOut_BMCancelAuction` | — | TBD | SGWBlackMarketManager.cancelAuction | NO |
| `Event_NetOut_BMPlaceBid` | — | TBD | SGWBlackMarketManager.placeBid | NO |
| `Event_NetOut_BMSearch` | — | TBD | SGWBlackMarketManager.searchBlackMarket | NO |

### Trading

| Event Name | String Addr | Handler Addr | .def Method | Impl |
|------------|-------------|--------------|-------------|------|
| `Event_NetOut_TradeProposal` | — | TBD | SGWInventoryManager.tradeProposal | NO |
| `Event_NetOut_TradeLockState` | — | TBD | SGWInventoryManager.tradeLockState | NO |
| `Event_NetOut_TradeRequestCancel` | — | TBD | SGWInventoryManager.tradeRequestCancel | NO |

### Stargates

| Event Name | String Addr | Handler Addr | .def Method | Impl |
|------------|-------------|--------------|-------------|------|
| `Event_NetOut_onDialGate` | 019be588 | TBD | GateTravel.onDialGate | PARTIAL |
| `Event_NetOut_DHD` | 019be1d8 | TBD | GateTravel.dhd | PARTIAL |
| `Event_NetOut_SetRingTransporterDestination` | 0195fa78 | TBD | GateTravel.setRingTransporterDestination | NO |
| `Event_NetOut_GiveStargateAddress` | 019b3f54 | TBD | (GM) GateTravel.giveStargateAddress | NO |
| `Event_NetOut_RemoveStargateAddress` | 019b3f8c | TBD | (GM) GateTravel.removeStargateAddress | NO |

### Minigames

| Event Name | String Addr | Handler Addr | .def Method | Impl |
|------------|-------------|--------------|-------------|------|
| `Event_NetOut_StartMinigame` | 019be3ec | TBD | MinigamePlayer.startMinigame | NO |
| `Event_NetOut_EndMinigame` | 019be408 | TBD | MinigamePlayer.endMinigame | NO |
| `Event_NetOut_MinigameComplete` | 019b31f8 | TBD | MinigamePlayer.minigameComplete | NO |
| `Event_NetOut_MinigameCallRequest` | 019be4dc | TBD | MinigamePlayer.minigameCallRequest | NO |
| `Event_NetOut_MinigameCallAccept` | 019be520 | TBD | MinigamePlayer.minigameCallAccept | NO |
| `Event_NetOut_MinigameCallDecline` | 019be540 | TBD | MinigamePlayer.minigameCallDecline | NO |
| `Event_NetOut_MinigameCallAbort` | 019be500 | TBD | MinigamePlayer.minigameCallAbort | NO |
| `Event_NetOut_MinigameContactRequest` | 019be564 | TBD | MinigamePlayer.minigameContactRequest | NO |
| `Event_NetOut_MinigameStartCancel` | 019be4b8 | TBD | MinigamePlayer.minigameStartCancel | NO |
| `Event_NetOut_SpectateMinigame` | 019be498 | TBD | MinigamePlayer.spectateMinigame | NO |
| `Event_NetOut_RegisterToMinigameHelp` | 019be424 | TBD | MinigamePlayer.registerToMinigameHelp | NO |
| `Event_NetOut_UpdateRegisterToMinigameHelp` | 019be448 | TBD | MinigamePlayer.updateRegisterToMinigameHelp | NO |
| `Event_NetOut_RequestSpectateList` | 019be474 | TBD | MinigamePlayer.requestSpectateList | NO |
| `Event_NetOut_GiveMinigameContact` | 019b3230 | TBD | (GM) MinigamePlayer.giveMinigameContact | NO |

### Dueling

| Event Name | String Addr | Handler Addr | .def Method | Impl |
|------------|-------------|--------------|-------------|------|
| `Event_NetOut_DuelChallenge` | 019b4478 | TBD | SGWPlayer.duelChallenge | NO |
| `Event_NetOut_DuelResponse` | 0195fb58 | TBD | SGWPlayer.duelResponse | NO |
| `Event_NetOut_DuelForfeit` | 019b44a8 | TBD | SGWPlayer.duelForfeit | NO |

### Pets

| Event Name | String Addr | Handler Addr | .def Method | Impl |
|------------|-------------|--------------|-------------|------|
| `Event_NetOut_PetInvokeAbility` | 019b42a4 | TBD | SGWPet.invokeAbility | NO |
| `Event_NetOut_PetAbilityToggle` | 019b42d8 | TBD | SGWPet.toggleAbility | NO |
| `Event_NetOut_PetChangeStance` | 019bc070 | TBD | SGWPet.changePetStance | NO |

### Contact Lists

| Event Name | String Addr | Handler Addr | .def Method | Impl |
|------------|-------------|--------------|-------------|------|
| `Event_NetOut_contactListCreate` | — | TBD | ContactListManager.contactListCreate | NO |
| `Event_NetOut_contactListDelete` | — | TBD | ContactListManager.contactListDelete | NO |
| `Event_NetOut_contactListRename` | — | TBD | ContactListManager.contactListRename | NO |
| `Event_NetOut_contactListAddMembers` | — | TBD | ContactListManager.contactListAddMembers | NO |
| `Event_NetOut_contactListRemoveMembers` | — | TBD | ContactListManager.contactListRemoveMembers | NO |
| `Event_NetOut_contactListFlagsUpdate` | — | TBD | ContactListManager.contactListFlagsUpdate | NO |

### Abilities & Training

| Event Name | String Addr | Handler Addr | .def Method | Impl |
|------------|-------------|--------------|-------------|------|
| `Event_NetOut_TrainAbility` | — | TBD | SGWPlayer.trainAbility | PARTIAL |
| `Event_NetOut_ResetAbilities` | 019b36b0 | TBD | (GM) SGWPlayer.resetAbilities | NO |
| `Event_NetOut_RespecAbility` | 0195fac0 | TBD | SGWPlayer.respecAbility | NO |
| `Event_NetOut_Respec` | 019b370c | TBD | SGWPlayer.respec | NO |

### Protocol / Internal

| Event Name | String Addr | Handler Addr | .def Method | Impl |
|------------|-------------|--------------|-------------|------|
| `Event_NetOut_versionInfoRequest` | 017fba14 | TBD | ClientCache.versionInfoRequest | YES |
| `Event_NetOut_elementDataRequest` | 019b9bac | TBD | ClientCache.elementDataRequest | YES |
| `Event_NetOut_onSpaceQueueStatus` | 019b4510 | TBD | (internal) | NO |
| `Event_NetOut_onSpaceQueueReadyResponse` | 0195fa34 | TBD | (internal) | NO |
| `Event_NetOut_onSpaceQueuedResponse` | 0195f9f4 | TBD | (internal) | NO |
| `Event_NetOut_onStrikeTeamResponse` | 019be0d8 | TBD | (internal) | NO |
| `Event_NetOut_Petition` | 019b9bac | TBD | SGWEntity.logPetition | NO |

---

## Event_NetIn — Server to Client (167 messages)

Messages sent FROM the server TO the client. These correspond to `ClientMethods` in entity .def files.

### Summary by System

| System | Count | Server Status |
|--------|-------|---------------|
| Login & Account | 14 | Implemented |
| World & Entity | 13 | Implemented |
| Being / Character | 10 | Implemented |
| Stats & Progression | 8 | Partial |
| Combat | 8 | Partial (~70%) |
| Abilities & Pets | 5 | Partial |
| Inventory & Items | 6 | Partial (~80%) |
| Store & Trading | 5 | Partial |
| Vault | 4 | Not implemented |
| Missions | 7 | Partial (~40%) |
| Chat | 7 | Implemented |
| Organizations | 18 | Not implemented |
| Contact Lists | 5 | Not implemented |
| Mail | 4 | Not implemented |
| Stargates | 8 | Partial (~20%) |
| Crafting | 6 | Not implemented |
| Black Market | 5 | Not implemented |
| Minigames | 12 | Not implemented |
| Dueling | 4 | Not implemented |
| UI & Navigation | 13 | Partial |
| Media | 2 | Not implemented |
| Misc | 5 | Partial |

> **Note**: Full per-message tables for Event_NetIn follow the same format as Event_NetOut above.
> They are documented in [../technical/network-messages.md](../technical/network-messages.md) with categories.
> Handler addresses and implementation status will be populated as RE work progresses.

---

## Implementation Coverage Summary

| Category | NetOut Impl | NetOut Total | NetIn Impl | NetIn Total | Overall |
|----------|-------------|-------------|------------|-------------|---------|
| Login/Character | 11 | 11 | 14 | 14 | 100% |
| Combat/Abilities | 5 | 19 | 5 | 13 | 31% |
| Inventory/Items | 6 | 17 | 4 | 10 | 37% |
| Missions | 3 | 16 | 3 | 7 | 26% |
| Chat | 14 | 14 | 7 | 7 | 100% |
| Organizations | 0 | 15 | 0 | 18 | 0% |
| Mail | 0 | 9 | 0 | 4 | 0% |
| Trading | 0 | 4 | 0 | 2 | 0% |
| Black Market | 0 | 4 | 0 | 5 | 0% |
| Crafting | 0 | 6 | 0 | 6 | 0% |
| Stargates | 1 | 5 | 1 | 8 | 15% |
| Minigames | 0 | 14 | 0 | 12 | 0% |
| Dueling | 0 | 3 | 0 | 4 | 0% |
| Pets | 0 | 3 | 0 | 3 | 0% |
| Contact Lists | 0 | 6 | 0 | 5 | 0% |
| World/Entity | — | — | 13 | 13 | 100% |
| GM/Debug | ~20 | 59 | — | — | ~34% |
| Protocol | 4 | 7 | — | — | 57% |
| **TOTAL** | **~64** | **253** | **~47** | **167** | **~26%** |

---

## Updating This Document

When running Ghidra annotation script 04 (`04_event_signal_annotator.py`):
1. The script will identify handler function addresses for each event
2. Update the "Handler Addr" column with the discovered addresses
3. Spot-check handler functions against .def method signatures

When implementing a new system:
1. Update the "Impl" column from NO → PARTIAL → YES
2. Add wire format notes in the corresponding `gameplay/*.md` document
3. Update the coverage summary table
