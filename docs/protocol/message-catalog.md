# Network Message Catalog

> **Status**: Phase 2 update — wire formats documented for combat & inventory
> **Total messages**: 420 (253 NetOut + 167 NetIn)
> **Source**: Ghidra string search of sgw.exe + entity .def file correlation
> **Last updated**: 2026-07-25

> [!WARNING]
> **The per-message "Implemented" column and the Summary-by-System counts are
> known-stale and are being reworked.** A 2026-07-25 audit against
> `crates/cell-methods/src/cell/cell_methods/`, `crates/cell/src/cell/dispatch/`
> and `crates/base/src/base/dispatch/mod.rs` found three defect classes:
>
> - **Understated.** Whole subsystems marked "NO"/"Not implemented" do have
>   dispatch arms today — all of Crafting, Mail, Black Market, Trading, Pets,
>   and most Minigame rows. Check `cell_methods/` before trusting a "NO".
>   Black Market's base-side service
>   (`crates/base-methods/src/base/world_entry/methods/black_market/`) landed on 2026-09-27
>   (packet BM-01, the port of PR #586), so its four rows below now read
>   PARTIAL: served by the server, but the client drops the replies until
>   the client patch ships (#587). Packet BM-02 made the server's argument
>   layouts follow the `.def` order in the dispatch tables (the shared codec
>   `cimmeria-patch-wire`); see `docs/gameplay/black-market.md`.
> - **Overstated.** The nine `Chat*` rows (`ChatList`, `ChatIgnore`,
>   `ChatFriend`, `ChatMute`, `ChatKick`, `ChatOp`, `ChatBan`, `ChatPassword`)
>   and `SendGMShout` are marked implemented but have no handler.
> - **Wrong interface attribution.** Several rows credit a method to an
>   interface when the `.def` places it on `SGWPlayer` itself — e.g.
>   `useAbility`, `useAbilityOnGroundTarget`, `setAutoCycle`, `lootItem`,
>   `chosenRewards`, `createOrganization`, and the five vendor methods
>   (`purchaseItems`/`sellItems`/`buybackItems`/`repairItems`/`rechargeItems`).
>
> The **index/name/args** data in
> [cell-method-dispatch-table.md](cell-method-dispatch-table.md) and
> [client-method-dispatch-table.md](client-method-dispatch-table.md) was
> re-derived from `entities/defs/` in the same audit and verified clean; prefer
> those two documents over this one for anything index-related.

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
| Contact Lists | 6 | Implemented |
| Organizations | 15 | Implemented server-side except the unsolicited PvP leave response (cash transfer is bank-vault BV-08); not yet client-verified |
| Mail | 9 | Not implemented |
| Trading | 4 | Not implemented |
| Black Market | 4 | Partial (server only; client patch pending) |
| Crafting & Research | 6 | Implemented server-side by the crafting campaign (CR-01 to CR-10); not yet client-verified |
| Abilities & Training | 4 | Partial |
| Stargates | 5 | Partial (~20%) |
| Minigames | 14 | Not implemented |
| Dueling | 3 | All three: challenge and response (SS-D1), forfeit (SS-D3) |
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

> **Implemented server-side, not yet client-verified (2026-09-27, organizations close-out ORG-11).** ORG-01 decodes every organization call the client sends: cell methods 8-19 and SGWPlayer cell method 94 (`onOrganizationCreation`; earlier editions of this table named it `OrganizationMember.createOrganization`, which does not exist) through `decode_org_cell_method` / `decode_on_organization_creation` in `crates/wire/src/cell/cell_methods/organization/`, and base methods 0xCF-0xD2 through `decode_org_base_method` in `crates/wire/src/base/organization.rs`. ORG-03 to ORG-10 handle them: squads on the cell, Teams and Commands on the base ([organization-system.md](../gameplay/organization-system.md)). `Impl` is YES where the call does what the client asks. `organizationTransferCash` moves naquadah between the wallet and the treasury (the Bank campaign's BV-08), and `pvpOrganizationLeaveResponse` is refused as unsolicited because no strike-team request is ever sent (CAT-M-17). The `.def Method` column keeps this table's older names; the dispatch tables have the real ones. The server-to-client organization methods have serializers; see [client-method-dispatch-table.md](client-method-dispatch-table.md). Campaign: [docs/analysis/organizations/](../analysis/organizations/README.md).

| Event Name | String Addr | Handler Addr | .def Method | Impl |
|------------|-------------|--------------|-------------|------|
| `Event_NetOut_OrganizationCreation` | 0195fb88 | TBD | SGWPlayer.onOrganizationCreation | YES |
| `Event_NetOut_OrganizationInvite` | — | TBD | OrganizationMember.organizationInvite | YES |
| `Event_NetOut_OrganizationInviteByType` | — | TBD | OrganizationMember.organizationInviteByType | YES |
| `Event_NetOut_OrganizationInviteResponse` | — | TBD | OrganizationMember.organizationInviteResponse | YES |
| `Event_NetOut_OrganizationLeave` | — | TBD | OrganizationMember.organizationLeave | YES |
| `Event_NetOut_OrganizationKick` | — | TBD | OrganizationMember.organizationKick | YES |
| `Event_NetOut_OrganizationRankChange` | — | TBD | OrganizationMember.organizationRankChange | YES |
| `Event_NetOut_OrganizationSetRankName` | — | TBD | OrganizationMember.setRankName | YES |
| `Event_NetOut_OrganizationSetRankPermissions` | — | TBD | OrganizationMember.setRankPermissions | YES |
| `Event_NetOut_OrganizationMOTD` | — | TBD | OrganizationMember.setMOTD | YES |
| `Event_NetOut_OrganizationNote` | — | TBD | OrganizationMember.setNote | YES |
| `Event_NetOut_OrganizationOfficerNote` | — | TBD | OrganizationMember.setOfficerNote | YES |
| `Event_NetOut_OrganizationTransferCash` | — | TBD | OrganizationMember.transferCash | NO |
| `Event_NetOut_ReloadOrganizations` | 019b2e6c | TBD | (debug) | YES |
| `Event_NetOut_PvPOrganizationLeaveResponse` | — | TBD | OrganizationMember.pvpLeaveResponse | NO |

### Crafting & Research

| Event Name | String Addr | Handler Addr | .def Method | Impl |
|------------|-------------|--------------|-------------|------|
| `Event_NetOut_Craft` | — | TBD | SGWPlayer.craft | YES |
| `Event_NetOut_Alloy` | — | TBD | SGWPlayer.alloy | YES |
| `Event_NetOut_Research` | — | TBD | SGWPlayer.research | YES |
| `Event_NetOut_ReverseEngineer` | — | TBD | SGWPlayer.reverseEngineer | YES |
| `Event_NetOut_RespecCraft` | 0195fb28 | TBD | SGWPlayer.respecCraft | YES (confirms a respec a player's `.respeccraft` opened) |
| `Event_NetOut_SpendAppliedSciencePoint` | — | TBD | SGWPlayer.spendAppliedSciencePoint | YES |

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
| `Event_NetOut_BMCreateAuction` | — | TBD | SGWBlackMarketManager.createAuction | PARTIAL |
| `Event_NetOut_BMCancelAuction` | — | TBD | SGWBlackMarketManager.cancelAuction | PARTIAL |
| `Event_NetOut_BMPlaceBid` | — | TBD | SGWBlackMarketManager.placeBid | PARTIAL |
| `Event_NetOut_BMSearch` | — | TBD | SGWBlackMarketManager.searchBlackMarket | PARTIAL |

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
| `Event_NetOut_DuelChallenge` | 019b4478 | TBD | SGWPlayer.duelChallenge | YES (base 0xD9, SS-D1) |
| `Event_NetOut_DuelResponse` | 0195fb58 | TBD | SGWPlayer.duelResponse | YES (CM 102, SS-D1) |
| `Event_NetOut_DuelForfeit` | 019b44a8 | TBD | SGWPlayer.duelForfeit | YES (CM 103, SS-D3) |

### Pets

| Event Name | String Addr | Handler Addr | .def Method | Impl |
|------------|-------------|--------------|-------------|------|
| `Event_NetOut_PetInvokeAbility` | 019b42a4 | TBD | SGWPet.invokeAbility | NO |
| `Event_NetOut_PetAbilityToggle` | 019b42d8 | TBD | SGWPet.toggleAbility | NO |
| `Event_NetOut_PetChangeStance` | 019bc070 | TBD | SGWPet.changePetStance | NO |

### Contact Lists

Wire formats: see [`../reverse-engineering/findings/contact-list-wire-formats.md`](../reverse-engineering/findings/contact-list-wire-formats.md). Note that the RTTI-canonical names use camelCase (`contactList*`); V5 Documentation Campaign session 1 surfaced a cyclic-shift name-misassignment bug at `0x00e5f990`–`0x00e5f9f0` documented in the findings doc.

| Event Name | String Addr | Handler Addr | .def Method | Impl |
|------------|-------------|--------------|-------------|------|
| `Event_NetOut_contactListCreate` | — | TBD | ContactListManager.contactListCreate | YES |
| `Event_NetOut_contactListDelete` | — | TBD | ContactListManager.contactListDelete | YES |
| `Event_NetOut_contactListRename` | — | `0x00e5f990` | ContactListManager.contactListRename | YES |
| `Event_NetOut_contactListFlagsUpdate` | — | `0x00e5f9b0` | ContactListManager.contactListFlagsUpdate | YES |
| `Event_NetOut_contactListAddMembers` | — | `0x00e5f9d0` | ContactListManager.contactListAddMembers | YES |
| `Event_NetOut_contactListRemoveMembers` | — | `0x00e5f9f0` | ContactListManager.contactListRemoveMembers | YES |

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

### Client Telemetry (session-1 discovered)

Client-to-server telemetry pushes surfaced by V5 Documentation Campaign session 1 (2026-05-12). Not in any prior protocol doc; wire formats not yet decompiled. Cimmeria should handle gracefully — no-op or log.

| Event Name | String Addr | Handler Addr | .def Method | Impl | Wire Format | RE Status |
|------------|-------------|--------------|-------------|------|-------------|-----------|
| `SystemOptions` | — | `0x00d9cc40` | (telemetry) | NO | not yet decompiled | session-1 discovered |
| `PerfStats` | — | `0x00d9cee0` | (telemetry) | NO | not yet decompiled | session-1 discovered |

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
| Organizations | 18 | Sent: all but 41 and 42 (no strike teams); 44 is always 0 |
| Contact Lists | 5 | Implemented |
| Mail | 4 | Not implemented |
| Stargates | 8 | Partial (~20%) |
| Crafting | 6 | Sent: 112 (respec prompt), 136-139 at login and on every change, 140 at login and when stations or tools change (crafting campaign) |
| Black Market | 5 | Not implemented |
| Minigames | 12 | Not implemented |
| Dueling | 4 | Partial: `onDuelChallenge` (SS-D1); `onDuelEntitiesSet` and `Clear` (SS-D2); 152 is AoI's only |
| UI & Navigation | 13 | Partial |
| Media | 2 | Not implemented |
| Misc | 5 | Partial |

> **Note**: Full per-message tables for Event_NetIn follow the same format as Event_NetOut above.
> They are documented in [../technical/network-messages.md](../technical/network-messages.md) with categories.
> Handler addresses and implementation status will be populated as RE work progresses.

### Routing: `onSequence` (client method 1)

An ability's Ability_Begin, Ability_End and Ability_Interrupt
`onSequence` go to the caster's own client **and** to every player with
the caster in AoI, as Python `AbilityManager.playSequence` did
(`ent.client` then `ent.witnesses`). The cell sends all three through
`send_entity_method_to_self_and_witnesses`: a player caster gets one
`EntityMethodCall` and each witness one `WitnessEntityMethod`, encoded
under the SGWPlayer idbase for a player ghost and the NPC idbase for an
NPC. An NPC has no client, so it gets the witness fan-out alone. Before
NA43 a player's charge, shot and cancel went to their own client only,
and no other player saw them.

Since AT-10 the phases leave in different ticks: Ability_Begin at launch
(`use_ability/warmup/mod.rs`), Ability_End when the cast fires
(`use_ability/fire.rs`, in the same pass for a zero warmup, otherwise in
the warmup tick), and Ability_Interrupt when a warmup is cancelled
(`use_ability/warmup/interrupt.rs`). All three call
`play_ability_sequence` in
`crates/cell-combat/src/cell/abilities/use_ability/sequence.rs`.

---

## Cooked-Data Resource Cache (BASEMSG)

The cooked-data wire path is its own little protocol on top of Mercury. Three messages co-operate to keep the client's runtime cache (`Cache.en-US/`) in sync with whatever PAK content the server is serving — including any in-memory mission overrides applied at server startup. See [../architecture/mission-pak-overrides.md](../architecture/mission-pak-overrides.md) for the override mechanism.

### `versionInfoRequest` (NetOut, BASEMSG 0xC0)

The client sends one of these per resource category when its connection comes up (`ServerSource<N>` subscribes to `Event_Net_Connected`, handler `0x0044c560` → emitter `0x00449d30`), which is at character select, just after the character list. The server only reads `0xC0` as this message while the Account entity is active: in-world, `0xC0` is `SGWPlayer.chatJoin` (see below).

```text
[categoryId: u32][version: u32]
```

`version` is the `MetaData` value from the client's local copy of the category. The server compares it against the version it serves: the PAK's `MetaData`, plus the content-hash bump for a category with Cimmeria overrides (`crates/base-session/src/base/cooked_data.rs`, `crates/base-session/src/base/cooked_sync/decision.rs`).

Implemented; see `handle_version_info_request`.

### `onVersionInfo` (NetIn, BASEMSG 0x80)

The server's reply. Wire format (encoder `build_version_info` in `crates/wire/src/mercury/protocol/resources.rs`):

```text
[accountEntityId: u32]
[categoryId:      u32]
[version:         u32]   — the client writes this into the cache PAK's MetaData on receipt
[requiredUpdates: u32]   — count of entries the server is about to push
[invalidateAll:   u8]    — 1 = delete every entry of the category; 0 = per-key
[invalidKeys:     ARRAY<u32>]  — { count: u32, ids: [u32; count] }
```

What the client does with it (`ServerSource<N>::onVersionInfo`, `0x00441630`, re-decompiled 2026-09-28 for #840): stores `RequiredUpdates`; if `InvalidateAll` is set, clears its element list and deletes every entry from the writable cache PAK (`FUN_0047a690`), otherwise drops each `InvalidKeys` entry; writes `Version` to `MetaData` (`ServerSource_SetVersion`, `0x00479e90`); writes out any entries that arrived before this reply; marks the category ready. It never requests an entry it dropped. Nothing but the entry pushes and `onCookedDataError` decrements `RequiredUpdates`, and no reader of it was found.

The server decides per request (`VersionReply::decide`, `crates/base-session/src/base/cooked_sync/decision.rs`):

| Server state | What the client receives |
|---|---|
| No category data | One reply echoing the client's version, `invalidateAll = 0`, no keys. |
| Versions match | One reply with the served version, `invalidateAll = 0`, no keys. |
| Versions differ (any category, with or without overrides) | A **full resync** on the session's resync task: (1) `onVersionInfo(invalidateAll = 1, requiredUpdates = 0, version = !served)`; (2) one `resourceFragment` transfer per entry of the server's category, in ascending key order; (3) `onVersionInfo(invalidateAll = 0, requiredUpdates = 0, no keys, version = served)`. |

- **Placeholder version.** The opening reply stamps the bitwise NOT of the served version, because the client writes `Version` before any entry arrives. The closing reply is ordered behind every entry on the reliable channel, so the client takes the real version only once it holds the whole category. A client that disconnects part-way keeps the placeholder and resyncs at its next login.
- **`RequiredUpdates = 0`.** The client asks for a missing entry only while the category's `RequiredUpdates` is 0: every per-category request function checks `this+0x48 == 0`, e.g. `0x00cfe060` for category 11 and `0x00d20150` for category 3. With `N` it would wait on every miss until the whole category had arrived. With 0 it asks, and the server serves the miss ahead of the stream (below).
- **Addressing.** Both replies go to the session's current entity. At character select that is the Account, as client method 0 (`0x80`). In-world it is the player, as SGWPlayer client method 96 (`0xBD`, sub-index 35; `build_version_info_to_player`). `0x80` addressed to the player would be SGWPlayer client method 0.
- **Pacing and order.** Every packet goes through the session's reliable window: at most `SYNC_IN_FLIGHT_BUDGET` (24) reliable packets outstanding on the session, counting everything else in flight, which leaves 8 of the 32 TX-window slots for game traffic. Categories stream in rank order: the held ones first, then missions, dialogs and items, then the rest, with TextStrings (29,126 entries) last.
- **World entry.** `playCharacter` waits only for the held categories: 12 (world info, needed by `onClientMapLoad`), 16, 17, 18, 20 and 21. The client has no miss path for any of them, and together they are about 230 entries, under a second. Everything else keeps streaming after world entry.

**Misses.** `elementDataRequest` (`0xC1` at character select, SGWPlayer `0xD5` in-world; `[categoryId: i32][key: i32]`) is served from the server's category, PAK plus overrides. The entry goes out as the next transfer on the session's task, ahead of the background stream (`cooked_sync::serve_miss`). Rules:

- An unknown category or key is refused. So is anything past a session's rate limit: a bucket of 100, refilled at 50 per second, with at most 256 misses waiting.
- A repeat of an entry already waiting is dropped.
- A refusal logs a WARN, throttled to one per reason per session every 5 s with the suppressed count, because the client asks again on every lookup.

**Telemetry:**

- `cooked_data.version_reply` on every request: `outcome`, `reason`, `category_id`, `client_version`, `server_version`.
- `cooked_data.sync_start` / `cooked_data.sync_finish`: `outcome=complete` at INFO with `entry_count`, `bytes`, `packets` and `duration_ms`; `outcome=abandoned` at WARN with `reason`.
- `cooked_data.miss_served` (INFO): `category_id`, `key`, `bytes`, `latency_ms`.
- `cooked_data.miss_refused` (WARN): `reason`, `category_id`, `key`, `suppressed`.
- `cooked_data.world_entry_held` / `world_entry_released`.

Design: [mission-pak-overrides.md § How the handshake works](../architecture/mission-pak-overrides.md#how-the-handshake-works).

Before #840 the server sent `invalidateAll = 1` with nothing pushed for any mismatched category without an override list, and the client emptied and persisted that category (the 2026-09-20 Kismet sequence wipe, #754). A build from before #840 still does, which is why a client moving between builds can lose a category (see [troubleshooting](../troubleshooting.md#a-cooked-data-category-went-empty-after-logging-in-to-another-server)).

`InvalidKeys` is parsed as a `PropertyList<long>` (Ghidra decomp of `ServerConnection::onVersionInfo`, `FUN_00449460`). The server no longer sends per-key invalidations.

### `resourceFragment` (NetIn, BASEMSG 0x36)

How the server ships XML payloads for a single category-element pair. Already documented in [docs/engine/cooked-data-pak-format.md](../engine/cooked-data-pak-format.md). The server emits it in reply to `elementDataRequest` (`crates/base-session/src/base/cooked_data.rs`) and, for a resync, once per entry of the category between the two `onVersionInfo` replies (`crates/base-session/src/base/cooked_sync/task.rs`). The client's proxy-data handler (`0x0043dad0`) decrements `RequiredUpdates` (if nonzero) per completed entry and writes the entry to its cache at once if the category's `onVersionInfo` has arrived, or buffers it until then. `RequiredUpdates` is only a counter and a gate on miss requests; the resync sends 0.

---

## Implementation Coverage Summary

| Category | NetOut Impl | NetOut Total | NetIn Impl | NetIn Total | Overall |
|----------|-------------|-------------|------------|-------------|---------|
| Login/Character | 11 | 11 | 14 | 14 | 100% |
| Combat/Abilities | 5 | 19 | 5 | 13 | 31% |
| Inventory/Items | 6 | 17 | 4 | 10 | 37% |
| Missions | 3 | 16 | 3 | 7 | 26% |
| Chat | 14 | 14 | 7 | 7 | 100% |
| Organizations | 13 | 15 | 16 | 18 | 88% |
| Mail | 0 | 9 | 0 | 4 | 0% |
| Trading | 0 | 4 | 0 | 2 | 0% |
| Black Market | 0 | 4 | 0 | 5 | 0% |
| Crafting | 6 | 6 | 6 | 6 | 100% |
| Stargates | 1 | 5 | 1 | 8 | 15% |
| Minigames | 0 | 14 | 0 | 12 | 0% |
| Dueling | 3 | 3 | 3 | 4 | 86% |
| Pets | 0 | 3 | 0 | 3 | 0% |
| Contact Lists | 6 | 6 | 5 | 5 | 100% |
| World/Entity | — | — | 13 | 13 | 100% |
| GM/Debug | ~20 | 59 | — | — | ~34% |
| Protocol | 4 | 7 | — | — | 57% |
| **TOTAL** | **~83** | **253** | **~71** | **167** | **~37%** |

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
