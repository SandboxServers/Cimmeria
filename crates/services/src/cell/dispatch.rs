//! Client->server cell method dispatch for the SGWPlayer entity type.
//!
//! When the client calls an exposed CellMethod, the BaseApp forwards it as a
//! [`CellMethodCall`](super::messages::BaseToCellMsg::CellMethodCall) message.
//! This module maps the flattened method index to a named handler.
//!
//! ## Flattened EXPOSED CellMethod index ordering
//!
//! The client encodes cell method calls as `msg_id = index | 0x80` (direct, 0-60)
//! or `msg_id = 0xBD` with sub-index (extended, >= 61). The index is the
//! flattened position across the entity type hierarchy, counting only `<Exposed/>`
//! methods in CellMethods sections.
//!
//! See `docs/protocol/cell-method-dispatch-table.md` for the complete 109-method
//! table with arg formats, interface sources, and .def line references.

use tokio::sync::mpsc;

use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_entity::stats::{HEALTH, FOCUS};

use super::combat;
use super::messages::CellToBaseMsg;
use super::space_manager::SpaceManager;

// ── Flattened EXPOSED CellMethod indices ────────────────────────────────────
//
// SGWBeing interface (2 exposed)
//

/// SGWBeing: setTargetID(INT32 targetId)
pub const CM_SET_TARGET_ID: u16 = 0;
/// SGWBeing: setMovementType(UINT8 movementType)
pub const CM_SET_MOVEMENT_TYPE: u16 = 1;

//
// SGWAbilityManager interface (3 exposed)
//

/// SGWAbilityManager: toggleCombatDebug()
pub const CM_TOGGLE_COMBAT_DEBUG: u16 = 2;
/// SGWAbilityManager: toggleCombatVerboseDebug()
pub const CM_TOGGLE_COMBAT_VERBOSE_DEBUG: u16 = 3;
/// SGWAbilityManager: confirmationResponse(INT8 choice)
pub const CM_CONFIRMATION_RESPONSE: u16 = 4;

//
// SGWCombatant interface (3 exposed)
//

/// SGWCombatant: setCrouched(INT8 crouched)
pub const CM_SET_CROUCHED: u16 = 5;
/// SGWCombatant: toggleHealDebug()
pub const CM_TOGGLE_HEAL_DEBUG: u16 = 6;
/// SGWCombatant: requestHolsterWeapon(INT8 holstered)
pub const CM_REQUEST_HOLSTER_WEAPON: u16 = 7;

//
// OrganizationMember interface (12 exposed, indices 8-19)
//

/// OrganizationMember: organizationInviteResponse(INT32 requestId, UINT8 response)
pub const CM_ORG_INVITE_RESPONSE: u16 = 8;
/// OrganizationMember: organizationLeave(INT32 organizationId)
pub const CM_ORG_LEAVE: u16 = 9;
/// OrganizationMember: BroadcastMinimapPing(INT32 orgId, VECTOR3 location)
pub const CM_ORG_BROADCAST_MINIMAP_PING: u16 = 10;
/// OrganizationMember: strikeTeamResponse(INT32 orgId, UINT8 response)
pub const CM_ORG_STRIKE_TEAM_RESPONSE: u16 = 11;
/// OrganizationMember: pvpOrganizationLeaveResponse(INT32 orgId, UINT8 response)
pub const CM_ORG_PVP_LEAVE_RESPONSE: u16 = 12;
/// OrganizationMember: organizationMOTD(INT32 orgId, WSTRING motd)
pub const CM_ORG_MOTD: u16 = 13;
/// OrganizationMember: organizationNote(INT32 orgId, WSTRING note)
pub const CM_ORG_NOTE: u16 = 14;
/// OrganizationMember: organizationOfficerNote(INT32 orgId, WSTRING name, WSTRING note)
pub const CM_ORG_OFFICER_NOTE: u16 = 15;
/// OrganizationMember: organizationSetRankPermissions(INT32 orgId, INT32 rank, INT32 permissions)
pub const CM_ORG_SET_RANK_PERMISSIONS: u16 = 16;
/// OrganizationMember: organizationSetRankName(INT32 orgId, INT32 rank, WSTRING name)
pub const CM_ORG_SET_RANK_NAME: u16 = 17;
/// OrganizationMember: squadSetLootMode(INT32 lootMode)
pub const CM_ORG_SQUAD_SET_LOOT_MODE: u16 = 18;
/// OrganizationMember: organizationTransferCash(INT32 orgId, INT32 cash)
pub const CM_ORG_TRANSFER_CASH: u16 = 19;

//
// MinigamePlayer interface (15 exposed, indices 20-34)
//

/// MinigamePlayer: debugStartMinigame(INT32 gameId)
pub const CM_MG_DEBUG_START: u16 = 20;
/// MinigamePlayer: debugSpectateMinigame(INT32 gameId)
pub const CM_MG_DEBUG_SPECTATE: u16 = 21;
/// MinigamePlayer: debugJoinMinigame(INT32 gameId)
pub const CM_MG_DEBUG_JOIN: u16 = 22;
/// MinigamePlayer: debugMinigameInstance(INT32 instanceId)
pub const CM_MG_DEBUG_INSTANCE: u16 = 23;
/// MinigamePlayer: startMinigame(INT32 hostEntityId, INT32 gameDefId)
pub const CM_MG_START: u16 = 24;
/// MinigamePlayer: endCurrentMinigame(INT32 gameId, INT32 winnerId, INT32 loserId)
pub const CM_MG_END_CURRENT: u16 = 25;
/// MinigamePlayer: requestSpectateList(INT32 gameId)
pub const CM_MG_REQUEST_SPECTATE_LIST: u16 = 26;
/// MinigamePlayer: spectateMinigame(INT32 gameId)
pub const CM_MG_SPECTATE: u16 = 27;
/// MinigamePlayer: registerToMinigameHelp(INT32 gameDefId, INT32 helpLevel)
pub const CM_MG_REGISTER_HELP: u16 = 28;
/// MinigamePlayer: updateRegisterToMinigameHelp(INT32 gameDefId, INT32 helpLevel)
pub const CM_MG_UPDATE_REGISTER_HELP: u16 = 29;
/// MinigamePlayer: minigameStartCancel(INT32 gameId)
pub const CM_MG_START_CANCEL: u16 = 30;
/// MinigamePlayer: minigameCallAccept(INT32 gameId)
pub const CM_MG_CALL_ACCEPT: u16 = 31;
/// MinigamePlayer: minigameCallDecline(INT32 gameId)
pub const CM_MG_CALL_DECLINE: u16 = 32;
/// MinigamePlayer: minigameCallAbort(INT32 gameId)
pub const CM_MG_CALL_ABORT: u16 = 33;
/// MinigamePlayer: minigameContactRequest(INT32 targetEntityId, INT32 gameDefId)
pub const CM_MG_CONTACT_REQUEST: u16 = 34;

//
// GateTravel interface (1 exposed)
//

/// GateTravel: onDialGate(INT32 targetAddressId, INT32 sourceAddressId)
pub const CM_ON_DIAL_GATE: u16 = 35;

//
// SGWInventoryManager interface (7 exposed, indices 36-42)
//

/// SGWInventoryManager: removeItem(ItemID itemId, INT16 quantity)
pub const CM_REMOVE_ITEM: u16 = 36;
/// SGWInventoryManager: listItems()
pub const CM_LIST_ITEMS: u16 = 37;
/// SGWInventoryManager: moveItem(INT32 itemId, INT32 targetBag, INT32 targetSlot, INT32 quantity)
pub const CM_MOVE_ITEM: u16 = 38;
/// SGWInventoryManager: useItem(INT32 itemId, INT32 targetId)
pub const CM_USE_ITEM: u16 = 39;
/// SGWInventoryManager: repairItemRequest(INT32 itemId, FLOAT repairRatio)
pub const CM_REPAIR_ITEM_REQUEST: u16 = 40;
/// SGWInventoryManager: requestActiveSlotChange(INT32 bagId, INT32 slotId)
pub const CM_REQUEST_ACTIVE_SLOT_CHANGE: u16 = 41;
/// SGWInventoryManager: requestAmmoChange(INT32 itemId, INT32 ammoType)
pub const CM_REQUEST_AMMO_CHANGE: u16 = 42;

//
// SGWMailManager interface (9 exposed, indices 43-51)
//

/// SGWMailManager: requestMailHeaders(UINT8 bArchive)
pub const CM_REQUEST_MAIL_HEADERS: u16 = 43;
/// SGWMailManager: sendMailMessage(INT32, ARRAY<WSTRING>, WSTRING, WSTRING, INT32, UINT8, INT32, INT32)
pub const CM_SEND_MAIL_MESSAGE: u16 = 44;
/// SGWMailManager: archiveMailMessage(INT32 mailId)
pub const CM_ARCHIVE_MAIL_MESSAGE: u16 = 45;
/// SGWMailManager: deleteMailMessage(INT32 mailId)
pub const CM_DELETE_MAIL_MESSAGE: u16 = 46;
/// SGWMailManager: returnMailMessage(INT32 mailId)
pub const CM_RETURN_MAIL_MESSAGE: u16 = 47;
/// SGWMailManager: requestMailBody(INT32 mailId)
pub const CM_REQUEST_MAIL_BODY: u16 = 48;
/// SGWMailManager: takeCashFromMailMessage(INT32 mailId)
pub const CM_TAKE_CASH_FROM_MAIL: u16 = 49;
/// SGWMailManager: takeItemFromMailMessage(INT32 mailId, INT32 containerId, INT32 slotId)
pub const CM_TAKE_ITEM_FROM_MAIL: u16 = 50;
/// SGWMailManager: payCODForMailMessage(INT32 mailId)
pub const CM_PAY_COD_FOR_MAIL: u16 = 51;

//
// Missionary interface (3 exposed, indices 52-54)
//

/// Missionary: abandonMission(INT32 missionId)
pub const CM_ABANDON_MISSION: u16 = 52;
/// Missionary: shareMission(INT32 missionId)
pub const CM_SHARE_MISSION: u16 = 53;
/// Missionary: shareMissionResponse(INT8 choice)
pub const CM_SHARE_MISSION_RESPONSE: u16 = 54;

// SGWPoller: 0 exposed

//
// ContactListManager interface (6 exposed, indices 55-60)
//

/// ContactListManager: contactListCreate(WSTRING listName, UINT32 flags)
pub const CM_CONTACT_LIST_CREATE: u16 = 55;
/// ContactListManager: contactListDelete(INT32 listId)
pub const CM_CONTACT_LIST_DELETE: u16 = 56;
/// ContactListManager: contactListRename(INT32 listId, WSTRING newName)
pub const CM_CONTACT_LIST_RENAME: u16 = 57;
/// ContactListManager: contactListFlagsUpdate(INT32 listId, UINT32 flags)
pub const CM_CONTACT_LIST_FLAGS_UPDATE: u16 = 58;
/// ContactListManager: contactListAddMembers(INT32 listId, ARRAY<WSTRING> members)
pub const CM_CONTACT_LIST_ADD_MEMBERS: u16 = 59;
/// ContactListManager: contactListRemoveMembers(INT32 listId, ARRAY<WSTRING> members)
pub const CM_CONTACT_LIST_REMOVE_MEMBERS: u16 = 60;

//
// SGWBlackMarketManager interface (6 exposed, indices 61-66)
//

/// SGWBlackMarketManager: BMSearch(BMSearchOptions searchOptions)
pub const CM_BM_SEARCH: u16 = 61;
/// SGWBlackMarketManager: BMCreateAuction(INT32 itemId, INT32 initialBid, INT32 buyoutPrice, INT32 durationDays)
pub const CM_BM_CREATE_AUCTION: u16 = 62;
/// SGWBlackMarketManager: BMPlaceBid(INT32 auctionId, INT32 bidAmount)
pub const CM_BM_PLACE_BID: u16 = 63;
/// SGWBlackMarketManager: BMCancelAuction(INT32 auctionId)
pub const CM_BM_CANCEL_AUCTION: u16 = 64;
/// SGWBlackMarketManager: BMStartWatchingItem(INT32 auctionId)
pub const CM_BM_START_WATCHING: u16 = 65;
/// SGWBlackMarketManager: BMStopWatchingItem(INT32 auctionId)
pub const CM_BM_STOP_WATCHING: u16 = 66;

// ClientCache: 0 exposed CellMethods

//
// SGWPlayer own CellMethods (42 exposed, indices 67-108)
//

/// SGWPlayer: callForAid(INT32 respawnerID)
pub const CM_CALL_FOR_AID: u16 = 67;
/// SGWPlayer: useAbility(INT32 abilityId, INT32 targetId)
pub const CM_USE_ABILITY: u16 = 68;
/// SGWPlayer: useAbilityOnGroundTarget(INT32 abilityId, FLOAT x, FLOAT y, FLOAT z)
pub const CM_USE_ABILITY_ON_GROUND: u16 = 69;
/// SGWPlayer: respawn()
pub const CM_RESPAWN: u16 = 70;
/// SGWPlayer: unstuck()
pub const CM_UNSTUCK: u16 = 71;
/// SGWPlayer: resetMyAbilities()
pub const CM_RESET_MY_ABILITIES: u16 = 72;
/// SGWPlayer: who(WSTRING name, WSTRING archetype, WSTRING alignment, WSTRING playerType)
pub const CM_WHO: u16 = 73;
/// SGWPlayer: interact(INT32 overrideTarget)
pub const CM_INTERACT: u16 = 74;
/// SGWPlayer: dialogButtonChoice(INT32 dialogId, INT32 buttonId)
pub const CM_DIALOG_BUTTON_CHOICE: u16 = 75;
/// SGWPlayer: initialResponse(INT32 dialogSetMapId)
pub const CM_INITIAL_RESPONSE: u16 = 76;
/// SGWPlayer: trainAbility(INT32 abilityId)
pub const CM_TRAIN_ABILITY: u16 = 77;
/// SGWPlayer: purchaseItems(ARRAY<INT32> itemIndices, ARRAY<INT32> quantities)
pub const CM_PURCHASE_ITEMS: u16 = 78;
/// SGWPlayer: sellItems(ARRAY<INT32> itemIds, ARRAY<INT32> quantities)
pub const CM_SELL_ITEMS: u16 = 79;
/// SGWPlayer: buybackItems(ARRAY<INT32> itemIds, ARRAY<INT32> quantities)
pub const CM_BUYBACK_ITEMS: u16 = 80;
/// SGWPlayer: repairItems(ARRAY<INT32> itemIds)
pub const CM_REPAIR_ITEMS: u16 = 81;
/// SGWPlayer: rechargeItems(ARRAY<INT32> itemIds)
pub const CM_RECHARGE_ITEMS: u16 = 82;
/// SGWPlayer: setAutoCycle(INT8 enabled)
pub const CM_SET_AUTO_CYCLE: u16 = 83;
/// SGWPlayer: lootItem(INT32 index)
pub const CM_LOOT_ITEM: u16 = 84;
/// SGWPlayer: triggerClientHintedGenericRegion(INT32 id, UINT8 bEntering, VECTOR3 position)
pub const CM_TRIGGER_REGION: u16 = 85;
/// SGWPlayer: requestReload(UINT8 reloadType)
pub const CM_REQUEST_RELOAD: u16 = 86;
/// SGWPlayer: chosenRewards(RewardChoices choices, INT32 missionId)
pub const CM_CHOSEN_REWARDS: u16 = 87;
/// SGWPlayer: petInvokeAbility(INT32 entityId, INT32 abilityId, INT32 targetId)
pub const CM_PET_INVOKE_ABILITY: u16 = 88;
/// SGWPlayer: petAbilityToggle(INT32 entityId, INT32 abilityId, INT8 toggle)
pub const CM_PET_ABILITY_TOGGLE: u16 = 89;
/// SGWPlayer: petChangeStance(INT32 entityId, INT8 stance)
pub const CM_PET_CHANGE_STANCE: u16 = 90;
/// SGWPlayer: setRingTransporterDestination(INT32 regionId, INT32 destinationId)
pub const CM_SET_RING_TRANSPORTER_DEST: u16 = 91;
/// SGWPlayer: onWorldInstanceReset()
pub const CM_WORLD_INSTANCE_RESET: u16 = 92;
/// SGWPlayer: updateSystemOptions(ARRAY<NameValuePair> options)
pub const CM_UPDATE_SYSTEM_OPTIONS: u16 = 93;
/// SGWPlayer: onOrganizationCreation(WSTRING organizationName)
pub const CM_ORG_CREATION: u16 = 94;
/// SGWPlayer: spendAppliedSciencePoints(INT32 disciplineSeqId)
pub const CM_SPEND_APPLIED_SCIENCE_POINTS: u16 = 95;
/// SGWPlayer: craft(INT32 craftId, ARRAY<ItemID> items, INT32 quantity)
pub const CM_CRAFT: u16 = 96;
/// SGWPlayer: research(ItemID itemId, ARRAY<ItemID> kickers)
pub const CM_RESEARCH: u16 = 97;
/// SGWPlayer: reverseEngineer(ItemID itemId)
pub const CM_REVERSE_ENGINEER: u16 = 98;
/// SGWPlayer: alloying(INT32 craftId, ItemID currentTierItemId, ARRAY<ItemID> lowerTierItems)
pub const CM_ALLOYING: u16 = 99;
/// SGWPlayer: respecCrafting()
pub const CM_RESPEC_CRAFTING: u16 = 100;
/// SGWPlayer: onClientChallengeResponse(INT32, WSTRING, INT32, WSTRING, INT32, INT32, WSTRING)
pub const CM_CLIENT_CHALLENGE_RESPONSE: u16 = 101;
/// SGWPlayer: sendDuelResponse(INT8 response)
pub const CM_SEND_DUEL_RESPONSE: u16 = 102;
/// SGWPlayer: duelForfeit()
pub const CM_DUEL_FORFEIT: u16 = 103;
/// SGWPlayer: tradeRequest(INT32 entityId, LocalTradeProposal proposal)
pub const CM_TRADE_REQUEST: u16 = 104;
/// SGWPlayer: tradeRequestCancel(INT32 entityId)
pub const CM_TRADE_REQUEST_CANCEL: u16 = 105;
/// SGWPlayer: tradeUpdateProposal(INT32 entityId, LocalTradeProposal proposal)
pub const CM_TRADE_UPDATE_PROPOSAL: u16 = 106;
/// SGWPlayer: tradeLockState(INT32 localVersionId, INT32 remoteVersionId, INT8 lockState)
pub const CM_TRADE_LOCK_STATE: u16 = 107;
/// SGWPlayer: cancelMovie(WSTRING movieName)
pub const CM_CANCEL_MOVIE: u16 = 108;

// ── Flattened ClientMethod indices (server→client) ──────────────────────────
//
// These are the indices used in entity method call packets sent FROM the server
// TO the client. They are a DIFFERENT flat index space from the CellMethod
// indices above (which are client→server).
//
// BigWorld .def parse order (from entity_description.cpp:parseInterface):
//   For each entity in the inheritance chain (root→leaf):
//     1. Parse <Implements> interfaces FIRST (recursively, in document order)
//     2. Then parse the entity's OWN <ClientMethods>
//
// For SGWPlayer the full order is:
//   SGWSpawnableEntity own (12)    → indices 0-11
//   SGWBeing interface (8)         → indices 12-19
//   SGWCombatant interface (6)     → indices 20-25
//   SGWBeing own (1)               → index 26
//   Communicator (7)               → indices 27-33
//   OrganizationMember (18)        → indices 34-51
//   MinigamePlayer (13)            → indices 52-64
//   GateTravel (4)                 → indices 65-68
//   SGWInventoryManager (7)        → indices 69-75
//   SGWMailManager (4)             → indices 76-79
//   Missionary (5)                 → indices 80-84
//   ContactListManager (5)         → indices 85-89
//   SGWBlackMarketManager (6)      → indices 90-95
//   ClientCache (2)                → indices 96-97
//   SGWPlayer own (59)             → indices 98-156
//
// See docs/protocol/client-method-dispatch-table.md for the complete table.

// MinigamePlayer ClientMethods: see `client_methods::minigame` (indices 52-64)
pub use super::client_methods::minigame::ON_START_MINIGAME as CLIENT_MG_ON_START_MINIGAME;
pub use super::client_methods::minigame::ON_END_MINIGAME as CLIENT_MG_ON_END_MINIGAME;

// ── Dispatch ────────────────────────────────────────────────────────────────

/// Dispatch a client->server cell method call to the appropriate handler.
pub async fn dispatch_cell_method(
    entity_id: u32,
    method_index: u16,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    engine: &ChainEngine,
) {
    match method_index {
        // ── SGWBeing interface ───────────────────────────────────────────

        CM_SET_TARGET_ID => {
            if args.len() >= 4 {
                let target_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::debug!(entity_id, target_id, "setTargetID");
                // TODO: Update entity target and notify witnesses via onTargetUpdate
            }
        }

        CM_SET_MOVEMENT_TYPE => {
            if !args.is_empty() {
                let movement_type = args[0];
                tracing::debug!(entity_id, movement_type, "setMovementType");
                // TODO: Update entity movement type and notify witnesses
            }
        }

        // ── SGWAbilityManager interface ──────────────────────────────────

        CM_TOGGLE_COMBAT_DEBUG | CM_TOGGLE_COMBAT_VERBOSE_DEBUG | CM_TOGGLE_HEAL_DEBUG => {
            tracing::debug!(entity_id, method_index, "Debug toggle (no-op)");
        }

        CM_CONFIRMATION_RESPONSE => {
            if args.len() >= 5 {
                let effect_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let accepted = args[4] != 0;
                tracing::debug!(entity_id, effect_id, accepted, "confirmationResponse");
                // TODO: Process effect confirmation
            }
        }

        // ── SGWCombatant interface ───────────────────────────────────────

        CM_SET_CROUCHED => {
            if !args.is_empty() {
                let enabled = args[0] != 0;
                tracing::debug!(entity_id, enabled, "setCrouched");
                // TODO: Update entity crouch state, notify witnesses via onCrouchToggled
            }
        }

        CM_REQUEST_HOLSTER_WEAPON => {
            if !args.is_empty() {
                let holster = args[0] != 0;
                tracing::debug!(entity_id, holster, "requestHolsterWeapon");
                // TODO: Update entity holster state, notify witnesses
            }
        }

        // ── OrganizationMember interface (12 methods) ────────────────────

        CM_ORG_INVITE_RESPONSE => {
            if args.len() >= 5 {
                let request_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let response = args[4];
                tracing::info!(entity_id, request_id, response, "UNIMPLEMENTED: organizationInviteResponse");
            }
        }

        CM_ORG_LEAVE => {
            if args.len() >= 4 {
                let org_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::info!(entity_id, org_id, "UNIMPLEMENTED: organizationLeave");
            }
        }

        CM_ORG_BROADCAST_MINIMAP_PING => {
            if args.len() >= 16 {
                let org_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let x = f32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                let y = f32::from_le_bytes([args[8], args[9], args[10], args[11]]);
                let z = f32::from_le_bytes([args[12], args[13], args[14], args[15]]);
                tracing::info!(entity_id, org_id, x, y, z, "UNIMPLEMENTED: BroadcastMinimapPing");
            }
        }

        CM_ORG_STRIKE_TEAM_RESPONSE => {
            if args.len() >= 5 {
                let org_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let response = args[4];
                tracing::info!(entity_id, org_id, response, "UNIMPLEMENTED: strikeTeamResponse");
            }
        }

        CM_ORG_PVP_LEAVE_RESPONSE => {
            if args.len() >= 5 {
                let org_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let response = args[4];
                tracing::info!(entity_id, org_id, response, "UNIMPLEMENTED: pvpOrganizationLeaveResponse");
            }
        }

        CM_ORG_MOTD => {
            if args.len() >= 4 {
                let org_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::info!(entity_id, org_id, "UNIMPLEMENTED: organizationMOTD");
            }
        }

        CM_ORG_NOTE => {
            if args.len() >= 4 {
                let org_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::info!(entity_id, org_id, "UNIMPLEMENTED: organizationNote");
            }
        }

        CM_ORG_OFFICER_NOTE => {
            if args.len() >= 4 {
                let org_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::info!(entity_id, org_id, "UNIMPLEMENTED: organizationOfficerNote");
            }
        }

        CM_ORG_SET_RANK_PERMISSIONS => {
            if args.len() >= 12 {
                let org_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let rank = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                let permissions = i32::from_le_bytes([args[8], args[9], args[10], args[11]]);
                tracing::info!(entity_id, org_id, rank, permissions, "UNIMPLEMENTED: organizationSetRankPermissions");
            }
        }

        CM_ORG_SET_RANK_NAME => {
            if args.len() >= 8 {
                let org_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let rank = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                tracing::info!(entity_id, org_id, rank, "UNIMPLEMENTED: organizationSetRankName");
            }
        }

        CM_ORG_SQUAD_SET_LOOT_MODE => {
            if args.len() >= 4 {
                let loot_mode = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::info!(entity_id, loot_mode, "UNIMPLEMENTED: squadSetLootMode");
            }
        }

        CM_ORG_TRANSFER_CASH => {
            if args.len() >= 8 {
                let org_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let cash = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                tracing::info!(entity_id, org_id, cash, "UNIMPLEMENTED: organizationTransferCash");
            }
        }

        // ── MinigamePlayer interface (15 methods) ────────────────────────

        CM_MG_DEBUG_START => {
            if args.len() >= 4 {
                let game_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::info!(entity_id, game_id, "UNIMPLEMENTED: debugStartMinigame");
            }
        }

        CM_MG_DEBUG_SPECTATE => {
            if args.len() >= 4 {
                let game_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::info!(entity_id, game_id, "UNIMPLEMENTED: debugSpectateMinigame");
            }
        }

        CM_MG_DEBUG_JOIN => {
            if args.len() >= 4 {
                let game_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::info!(entity_id, game_id, "UNIMPLEMENTED: debugJoinMinigame");
            }
        }

        CM_MG_DEBUG_INSTANCE => {
            if args.len() >= 4 {
                let instance_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::info!(entity_id, instance_id, "UNIMPLEMENTED: debugMinigameInstance");
            }
        }

        CM_MG_START => {
            if args.len() >= 8 {
                let host_entity_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let game_def_id = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                tracing::info!(entity_id, host_entity_id, game_def_id, "UNIMPLEMENTED: startMinigame");
            }
        }

        CM_MG_END_CURRENT => {
            if args.len() >= 12 {
                let game_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let winner_id = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                let loser_id = i32::from_le_bytes([args[8], args[9], args[10], args[11]]);
                tracing::info!(entity_id, game_id, winner_id, loser_id, "UNIMPLEMENTED: endCurrentMinigame");
            }
        }

        CM_MG_REQUEST_SPECTATE_LIST => {
            if args.len() >= 4 {
                let game_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::info!(entity_id, game_id, "UNIMPLEMENTED: requestSpectateList");
            }
        }

        CM_MG_SPECTATE => {
            if args.len() >= 4 {
                let game_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::info!(entity_id, game_id, "UNIMPLEMENTED: spectateMinigame");
            }
        }

        CM_MG_REGISTER_HELP => {
            if args.len() >= 8 {
                let game_def_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let help_level = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                tracing::info!(entity_id, game_def_id, help_level, "UNIMPLEMENTED: registerToMinigameHelp");
            }
        }

        CM_MG_UPDATE_REGISTER_HELP => {
            if args.len() >= 8 {
                let game_def_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let help_level = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                tracing::info!(entity_id, game_def_id, help_level, "UNIMPLEMENTED: updateRegisterToMinigameHelp");
            }
        }

        CM_MG_START_CANCEL => {
            if args.len() >= 4 {
                let game_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::info!(entity_id, game_id, "UNIMPLEMENTED: minigameStartCancel");
            }
        }

        CM_MG_CALL_ACCEPT => {
            if args.len() >= 4 {
                let game_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::info!(entity_id, game_id, "UNIMPLEMENTED: minigameCallAccept");
            }
        }

        CM_MG_CALL_DECLINE => {
            if args.len() >= 4 {
                let game_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::info!(entity_id, game_id, "UNIMPLEMENTED: minigameCallDecline");
            }
        }

        CM_MG_CALL_ABORT => {
            if args.len() >= 4 {
                let game_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::info!(entity_id, game_id, "UNIMPLEMENTED: minigameCallAbort");
            }
        }

        CM_MG_CONTACT_REQUEST => {
            if args.len() >= 8 {
                let target_entity_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let game_def_id = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                tracing::info!(entity_id, target_entity_id, game_def_id, "UNIMPLEMENTED: minigameContactRequest");
            }
        }

        // ── GateTravel interface ─────────────────────────────────────────

        CM_ON_DIAL_GATE => {
            if args.len() >= 8 {
                let target_address_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let source_address_id = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                tracing::debug!(entity_id, target_address_id, source_address_id, "onDialGate");
                super::gate_travel::handle_dial_gate(
                    entity_id, target_address_id, source_address_id, tx, space_mgr,
                ).await;
            }
        }

        // ── SGWInventoryManager interface (7 methods) ────────────────────

        CM_REMOVE_ITEM => {
            if args.len() >= 6 {
                let item_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let quantity = i16::from_le_bytes([args[4], args[5]]);
                tracing::info!(entity_id, item_id, quantity, "UNIMPLEMENTED: removeItem");
            }
        }

        CM_LIST_ITEMS => {
            tracing::info!(entity_id, "UNIMPLEMENTED: listItems");
        }

        CM_MOVE_ITEM => {
            if args.len() >= 16 {
                let item_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let target_bag = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                let target_slot = i32::from_le_bytes([args[8], args[9], args[10], args[11]]);
                let quantity = i32::from_le_bytes([args[12], args[13], args[14], args[15]]);
                tracing::info!(entity_id, item_id, target_bag, target_slot, quantity, "UNIMPLEMENTED: moveItem");
            }
        }

        CM_USE_ITEM => {
            if args.len() >= 8 {
                let item_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let target_id = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                tracing::info!(entity_id, item_id, target_id, "useItem");

                let player_id = space_mgr.get_entity(entity_id)
                    .and_then(|e| e.player_id).unwrap_or(0);
                super::content::fire_item_use(
                    entity_id, player_id, item_id, engine, tx, space_mgr,
                ).await;
            }
        }

        CM_REPAIR_ITEM_REQUEST => {
            if args.len() >= 8 {
                let item_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let repair_ratio = f32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                tracing::info!(entity_id, item_id, repair_ratio, "UNIMPLEMENTED: repairItemRequest");
            }
        }

        CM_REQUEST_ACTIVE_SLOT_CHANGE => {
            if args.len() >= 8 {
                let bag_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let slot_id = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                tracing::info!(entity_id, bag_id, slot_id, "UNIMPLEMENTED: requestActiveSlotChange");
            }
        }

        CM_REQUEST_AMMO_CHANGE => {
            if args.len() >= 8 {
                let item_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let ammo_type = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                tracing::info!(entity_id, item_id, ammo_type, "UNIMPLEMENTED: requestAmmoChange");
            }
        }

        // ── SGWMailManager interface (9 methods) ─────────────────────────

        CM_REQUEST_MAIL_HEADERS => {
            let b_archive = if !args.is_empty() { args[0] } else { 0 };
            tracing::debug!(entity_id, b_archive, "requestMailHeaders");
            super::mail::handle_request_mail_headers(entity_id, b_archive, tx).await;
        }

        CM_SEND_MAIL_MESSAGE => {
            tracing::info!(entity_id, "UNIMPLEMENTED: sendMailMessage");
        }

        CM_ARCHIVE_MAIL_MESSAGE => {
            if args.len() >= 4 {
                let mail_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::debug!(entity_id, mail_id, "archiveMailMessage");
                super::mail::handle_archive_mail(entity_id, mail_id, tx).await;
            }
        }

        CM_DELETE_MAIL_MESSAGE => {
            if args.len() >= 4 {
                let mail_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::debug!(entity_id, mail_id, "deleteMailMessage");
                super::mail::handle_delete_mail(entity_id, mail_id, tx).await;
            }
        }

        CM_RETURN_MAIL_MESSAGE => {
            if args.len() >= 4 {
                let mail_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::info!(entity_id, mail_id, "UNIMPLEMENTED: returnMailMessage");
            }
        }

        CM_REQUEST_MAIL_BODY => {
            if args.len() >= 4 {
                let mail_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::debug!(entity_id, mail_id, "requestMailBody");
                super::mail::handle_request_mail_body(entity_id, mail_id, tx).await;
            }
        }

        CM_TAKE_CASH_FROM_MAIL => {
            if args.len() >= 4 {
                let mail_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::info!(entity_id, mail_id, "UNIMPLEMENTED: takeCashFromMailMessage");
            }
        }

        CM_TAKE_ITEM_FROM_MAIL => {
            if args.len() >= 12 {
                let mail_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let container_id = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                let slot_id = i32::from_le_bytes([args[8], args[9], args[10], args[11]]);
                tracing::info!(entity_id, mail_id, container_id, slot_id, "UNIMPLEMENTED: takeItemFromMailMessage");
            }
        }

        CM_PAY_COD_FOR_MAIL => {
            if args.len() >= 4 {
                let mail_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::info!(entity_id, mail_id, "UNIMPLEMENTED: payCODForMailMessage");
            }
        }

        // ── Missionary interface (3 methods) ─────────────────────────────

        CM_ABANDON_MISSION => {
            if args.len() >= 4 {
                let mission_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::debug!(entity_id, mission_id, "abandonMission");
                super::missions::abandon_mission(entity_id, mission_id, tx, space_mgr).await;
            }
        }

        CM_SHARE_MISSION => {
            if args.len() >= 4 {
                let mission_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::info!(entity_id, mission_id, "UNIMPLEMENTED: shareMission");
            }
        }

        CM_SHARE_MISSION_RESPONSE => {
            if !args.is_empty() {
                let choice = args[0] as i8;
                tracing::info!(entity_id, choice, "UNIMPLEMENTED: shareMissionResponse");
            }
        }

        // ── ContactListManager interface (6 methods) ─────────────────────

        CM_CONTACT_LIST_CREATE => {
            // WSTRING listName, UINT32 flags -- variable length, just log
            tracing::info!(entity_id, "UNIMPLEMENTED: contactListCreate");
        }

        CM_CONTACT_LIST_DELETE => {
            if args.len() >= 4 {
                let list_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::info!(entity_id, list_id, "UNIMPLEMENTED: contactListDelete");
            }
        }

        CM_CONTACT_LIST_RENAME => {
            if args.len() >= 4 {
                let list_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::info!(entity_id, list_id, "UNIMPLEMENTED: contactListRename");
            }
        }

        CM_CONTACT_LIST_FLAGS_UPDATE => {
            if args.len() >= 8 {
                let list_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let flags = u32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                tracing::info!(entity_id, list_id, flags, "UNIMPLEMENTED: contactListFlagsUpdate");
            }
        }

        CM_CONTACT_LIST_ADD_MEMBERS => {
            if args.len() >= 4 {
                let list_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::info!(entity_id, list_id, "UNIMPLEMENTED: contactListAddMembers");
            }
        }

        CM_CONTACT_LIST_REMOVE_MEMBERS => {
            if args.len() >= 4 {
                let list_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::info!(entity_id, list_id, "UNIMPLEMENTED: contactListRemoveMembers");
            }
        }

        // ── SGWBlackMarketManager interface (6 methods) ──────────────────

        CM_BM_SEARCH => {
            // BMSearchOptions is a complex struct, just log arrival
            tracing::info!(entity_id, "UNIMPLEMENTED: BMSearch");
        }

        CM_BM_CREATE_AUCTION => {
            if args.len() >= 16 {
                let item_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let initial_bid = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                let buyout_price = i32::from_le_bytes([args[8], args[9], args[10], args[11]]);
                let duration_days = i32::from_le_bytes([args[12], args[13], args[14], args[15]]);
                tracing::info!(entity_id, item_id, initial_bid, buyout_price, duration_days, "UNIMPLEMENTED: BMCreateAuction");
            }
        }

        CM_BM_PLACE_BID => {
            if args.len() >= 8 {
                let auction_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let bid_amount = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                tracing::info!(entity_id, auction_id, bid_amount, "UNIMPLEMENTED: BMPlaceBid");
            }
        }

        CM_BM_CANCEL_AUCTION => {
            if args.len() >= 4 {
                let auction_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::info!(entity_id, auction_id, "UNIMPLEMENTED: BMCancelAuction");
            }
        }

        CM_BM_START_WATCHING => {
            if args.len() >= 4 {
                let auction_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::info!(entity_id, auction_id, "UNIMPLEMENTED: BMStartWatchingItem");
            }
        }

        CM_BM_STOP_WATCHING => {
            if args.len() >= 4 {
                let auction_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::info!(entity_id, auction_id, "UNIMPLEMENTED: BMStopWatchingItem");
            }
        }

        // ── SGWPlayer own methods ────────────────────────────────────────

        CM_CALL_FOR_AID => {
            if args.len() >= 4 {
                let respawner_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::info!(entity_id, respawner_id, "UNIMPLEMENTED: callForAid");
            }
        }

        CM_USE_ABILITY => {
            if args.len() >= 8 {
                let ability_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let target_id = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                tracing::debug!(entity_id, ability_id, target_id, "useAbility");
                super::abilities::handle_use_ability(entity_id, ability_id, target_id, tx, space_mgr).await;

                // Check if target NPC died -- fire entity_death for content chains
                // and set AI state to Dead so the NPC stops acting.
                if target_id > 0 {
                    let target_eid = target_id as u32;
                    let death_info = space_mgr.get_entity(target_eid).and_then(|target| {
                        let is_dead = target.stats.get(cimmeria_entity::stats::HEALTH)
                            .map_or(false, |s| s.cur <= 0);
                        if is_dead && !target.is_player {
                            Some((target.tag.clone(), target.is_player))
                        } else {
                            None
                        }
                    });

                    if let Some((tag, _is_player)) = death_info {
                        // Set AI state to Dead and clear threat
                        if let Some(target) = space_mgr.get_entity_mut(target_eid) {
                            target.ai_state = cimmeria_entity::cell_entity::AiState::Dead;
                            target.threat_list.clear();
                        }

                        // Fire entity_death for content engine (mission kill chains)
                        if let Some(tag) = tag {
                            let player_id = space_mgr.get_entity(entity_id)
                                .and_then(|e| e.player_id).unwrap_or(0);
                            super::content::fire_entity_death(
                                entity_id, player_id, &tag, engine, tx, space_mgr,
                            ).await;
                        }
                    }
                }
            }
        }

        CM_USE_ABILITY_ON_GROUND => {
            if args.len() >= 16 {
                let ability_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let x = f32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                let y = f32::from_le_bytes([args[8], args[9], args[10], args[11]]);
                let z = f32::from_le_bytes([args[12], args[13], args[14], args[15]]);
                tracing::debug!(entity_id, ability_id, x, y, z, "useAbilityOnGroundTarget");
                // TODO: Ground-target AoE ability handling
            }
        }

        CM_RESPAWN => {
            tracing::debug!(entity_id, "respawn");
            handle_respawn(entity_id, tx, space_mgr).await;
        }

        CM_UNSTUCK => {
            tracing::info!(entity_id, "UNIMPLEMENTED: unstuck");
        }

        CM_RESET_MY_ABILITIES => {
            tracing::info!(entity_id, "UNIMPLEMENTED: resetMyAbilities");
        }

        CM_WHO => {
            // who(WSTRING name, WSTRING archetype, WSTRING alignment, WSTRING playerType)
            tracing::info!(entity_id, "UNIMPLEMENTED: who");
        }

        CM_INTERACT => {
            if args.len() >= 4 {
                let target_entity_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::info!(entity_id, target_entity_id, "interact");

                // ── Step 1: Fire interact_tag event ──
                // Python: self.first('entity.interact.tag::' + target.tag, ...)
                let mut handled = false;
                if let Some(target) = space_mgr.get_entity(target_entity_id as u32) {
                    let tag = target.tag.clone();
                    let template_name = target.npc_name.clone();
                    let player_id = space_mgr.get_entity(entity_id)
                        .and_then(|e| e.player_id).unwrap_or(0);

                    if let Some(ref tag) = tag {
                        handled = super::content::fire_interact_tag(
                            entity_id, player_id, tag, target_entity_id as u32,
                            engine, tx, space_mgr,
                        ).await;
                    }

                    // ── Step 2: Fire interact_template event ──
                    // Python: self.first('entity.interact.template::' + target.template.templateName, ...)
                    if !handled {
                        if let Some(ref name) = template_name {
                            handled = super::content::fire_interact_template(
                                entity_id, player_id, name, target_entity_id as u32,
                                engine, tx, space_mgr,
                            ).await;
                        }
                    }
                }

                // ── Step 3: Fall through to available_interactions / static interaction ──
                // Python: target.onInteract(self)
                if !handled {
                    let dialog_id = super::interactions::handle_interact(
                        entity_id, target_entity_id as u32, tx, space_mgr,
                    ).await;

                    if let Some(did) = dialog_id {
                        let player_id = space_mgr.get_entity(entity_id)
                            .and_then(|e| e.player_id)
                            .unwrap_or(0);
                        super::content::fire_dialog_open(
                            entity_id, player_id, did, engine, tx, space_mgr,
                        ).await;
                    }
                }
            }
        }

        CM_DIALOG_BUTTON_CHOICE => {
            if args.len() >= 8 {
                let dialog_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let button_id = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                tracing::info!(entity_id, dialog_id, button_id, "dialogButtonChoice");

                let player_id = space_mgr.get_entity(entity_id)
                    .and_then(|e| e.player_id).unwrap_or(0);
                super::content::fire_dialog_choice(
                    entity_id, player_id, dialog_id, engine, tx, space_mgr,
                ).await;
            }
        }

        CM_INITIAL_RESPONSE => {
            if args.len() >= 4 {
                let interaction_set_map_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::info!(entity_id, interaction_set_map_id, "initialResponse");

                super::interactions::handle_initial_response(
                    entity_id, interaction_set_map_id, engine, tx, space_mgr,
                ).await;
            }
        }

        CM_TRAIN_ABILITY => {
            if args.len() >= 4 {
                let ability_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::info!(entity_id, ability_id, "UNIMPLEMENTED: trainAbility");
            }
        }

        CM_PURCHASE_ITEMS => {
            // ARRAY<INT32> itemIndices, ARRAY<INT32> quantities
            tracing::info!(entity_id, "UNIMPLEMENTED: purchaseItems");
        }

        CM_SELL_ITEMS => {
            // ARRAY<INT32> itemIds, ARRAY<INT32> quantities
            tracing::info!(entity_id, "UNIMPLEMENTED: sellItems");
        }

        CM_BUYBACK_ITEMS => {
            // ARRAY<INT32> itemIds, ARRAY<INT32> quantities
            tracing::info!(entity_id, "UNIMPLEMENTED: buybackItems");
        }

        CM_REPAIR_ITEMS => {
            // ARRAY<INT32> itemIds
            tracing::info!(entity_id, "UNIMPLEMENTED: repairItems");
        }

        CM_RECHARGE_ITEMS => {
            // ARRAY<INT32> itemIds
            tracing::info!(entity_id, "UNIMPLEMENTED: rechargeItems");
        }

        CM_SET_AUTO_CYCLE => {
            if !args.is_empty() {
                let enabled = args[0] != 0;
                tracing::debug!(entity_id, enabled, "setAutoCycle");
                if let Some(entity) = space_mgr.get_entity_mut(entity_id) {
                    entity.abilities.auto_cycle = enabled;
                    if !enabled {
                        entity.abilities.auto_cycle_ability_id = None;
                    }
                }
            }
        }

        CM_LOOT_ITEM => {
            if args.len() >= 4 {
                let index = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::info!(entity_id, index, "UNIMPLEMENTED: lootItem");
            }
        }

        CM_TRIGGER_REGION => {
            // triggerClientHintedGenericRegion(INT32 id, UINT8 bEntering, VECTOR3 position)
            if args.len() >= 17 {
                let region_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let b_entering = args[4] != 0;
                let _x = f32::from_le_bytes([args[5], args[6], args[7], args[8]]);
                let _y = f32::from_le_bytes([args[9], args[10], args[11], args[12]]);
                let _z = f32::from_le_bytes([args[13], args[14], args[15], args[16]]);

                // Look up the region tag from the auto-incrementing runtime ID.
                // The runtime ID is NOT the DB set_id or the number from the tag.
                let region_tag = space_mgr.get_region(region_id as u32)
                    .map(|r| r.tag.clone());

                if let Some(tag) = region_tag {
                    tracing::info!(entity_id, region_id, %tag, b_entering, "triggerClientHintedGenericRegion");

                    let player_id = space_mgr.get_entity(entity_id)
                        .and_then(|e| e.player_id).unwrap_or(0);

                    if b_entering {
                        super::content::fire_enter_region(
                            entity_id, player_id, &tag, engine, tx, space_mgr,
                        ).await;
                    } else {
                        super::content::fire_exit_region(
                            entity_id, player_id, &tag, engine, tx, space_mgr,
                        ).await;
                    }
                } else {
                    tracing::warn!(entity_id, region_id, "Unknown region ID in triggerClientHintedGenericRegion");
                }
            }
        }

        CM_REQUEST_RELOAD => {
            if !args.is_empty() {
                let reload_type = args[0];
                tracing::debug!(entity_id, reload_type, "requestReload");
                handle_reload(entity_id, tx, space_mgr).await;
            }
        }

        CM_CHOSEN_REWARDS => {
            // RewardChoices choices, INT32 missionId -- complex struct
            tracing::info!(entity_id, "UNIMPLEMENTED: chosenRewards");
        }

        CM_PET_INVOKE_ABILITY => {
            if args.len() >= 12 {
                let pet_entity_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let ability_id = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                let target_id = i32::from_le_bytes([args[8], args[9], args[10], args[11]]);
                tracing::info!(entity_id, pet_entity_id, ability_id, target_id, "UNIMPLEMENTED: petInvokeAbility");
            }
        }

        CM_PET_ABILITY_TOGGLE => {
            if args.len() >= 9 {
                let pet_entity_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let ability_id = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                let toggle = args[8] as i8;
                tracing::info!(entity_id, pet_entity_id, ability_id, toggle, "UNIMPLEMENTED: petAbilityToggle");
            }
        }

        CM_PET_CHANGE_STANCE => {
            if args.len() >= 5 {
                let pet_entity_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let stance = args[4] as i8;
                tracing::info!(entity_id, pet_entity_id, stance, "UNIMPLEMENTED: petChangeStance");
            }
        }

        CM_SET_RING_TRANSPORTER_DEST => {
            if args.len() >= 8 {
                let region_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let destination_id = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                tracing::info!(entity_id, region_id, destination_id, "UNIMPLEMENTED: setRingTransporterDestination");
            }
        }

        CM_WORLD_INSTANCE_RESET => {
            tracing::info!(entity_id, "UNIMPLEMENTED: onWorldInstanceReset");
        }

        CM_UPDATE_SYSTEM_OPTIONS => {
            // ARRAY<NameValuePair> options -- variable length
            tracing::info!(entity_id, "UNIMPLEMENTED: updateSystemOptions");
        }

        CM_ORG_CREATION => {
            // WSTRING organizationName -- variable length
            tracing::info!(entity_id, "UNIMPLEMENTED: onOrganizationCreation");
        }

        CM_SPEND_APPLIED_SCIENCE_POINTS => {
            if args.len() >= 4 {
                let discipline_seq_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::info!(entity_id, discipline_seq_id, "UNIMPLEMENTED: spendAppliedSciencePoints");
            }
        }

        CM_CRAFT => {
            // INT32 craftId, ARRAY<ItemID> items, INT32 quantity
            if args.len() >= 4 {
                let craft_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::info!(entity_id, craft_id, "UNIMPLEMENTED: craft");
            }
        }

        CM_RESEARCH => {
            // ItemID itemId, ARRAY<ItemID> kickers
            tracing::info!(entity_id, "UNIMPLEMENTED: research");
        }

        CM_REVERSE_ENGINEER => {
            // ItemID itemId
            tracing::info!(entity_id, "UNIMPLEMENTED: reverseEngineer");
        }

        CM_ALLOYING => {
            // INT32 craftId, ItemID currentTierItemId, ARRAY<ItemID> lowerTierItems
            if args.len() >= 4 {
                let craft_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::info!(entity_id, craft_id, "UNIMPLEMENTED: alloying");
            }
        }

        CM_RESPEC_CRAFTING => {
            tracing::info!(entity_id, "UNIMPLEMENTED: respecCrafting");
        }

        CM_CLIENT_CHALLENGE_RESPONSE => {
            // INT32, WSTRING, INT32, WSTRING, INT32, INT32, WSTRING -- anti-cheat
            if args.len() >= 4 {
                let challenge = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::info!(entity_id, challenge, "UNIMPLEMENTED: onClientChallengeResponse");
            }
        }

        CM_SEND_DUEL_RESPONSE => {
            if !args.is_empty() {
                let response = args[0] as i8;
                tracing::info!(entity_id, response, "UNIMPLEMENTED: sendDuelResponse");
            }
        }

        CM_DUEL_FORFEIT => {
            tracing::info!(entity_id, "UNIMPLEMENTED: duelForfeit");
        }

        CM_TRADE_REQUEST => {
            // INT32 entityId, LocalTradeProposal proposal
            if args.len() >= 4 {
                let target_entity_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::info!(entity_id, target_entity_id, "UNIMPLEMENTED: tradeRequest");
            }
        }

        CM_TRADE_REQUEST_CANCEL => {
            if args.len() >= 4 {
                let target_entity_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::info!(entity_id, target_entity_id, "UNIMPLEMENTED: tradeRequestCancel");
            }
        }

        CM_TRADE_UPDATE_PROPOSAL => {
            // INT32 entityId, LocalTradeProposal proposal
            if args.len() >= 4 {
                let target_entity_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::info!(entity_id, target_entity_id, "UNIMPLEMENTED: tradeUpdateProposal");
            }
        }

        CM_TRADE_LOCK_STATE => {
            if args.len() >= 9 {
                let local_version_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let remote_version_id = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                let lock_state = args[8] as i8;
                tracing::info!(entity_id, local_version_id, remote_version_id, lock_state, "UNIMPLEMENTED: tradeLockState");
            }
        }

        CM_CANCEL_MOVIE => {
            // WSTRING movieName -- variable length
            tracing::info!(entity_id, "UNIMPLEMENTED: cancelMovie");
        }

        // ── Catch-all ────────────────────────────────────────────────────

        _ => {
            tracing::info!(
                entity_id,
                method_index,
                args_len = args.len(),
                "Unhandled cell method call"
            );
        }
    }

    // Suppress unused variable warnings for parameters used in future handlers
    let _ = (tx, space_mgr);
}

/// Return a human-readable name for a cell method index, if known.
pub fn cell_method_name(index: u16) -> &'static str {
    match index {
        // SGWBeing
        CM_SET_TARGET_ID => "setTargetID",
        CM_SET_MOVEMENT_TYPE => "setMovementType",
        // SGWAbilityManager
        CM_TOGGLE_COMBAT_DEBUG => "toggleCombatDebug",
        CM_TOGGLE_COMBAT_VERBOSE_DEBUG => "toggleCombatVerboseDebug",
        CM_CONFIRMATION_RESPONSE => "confirmationResponse",
        // SGWCombatant
        CM_SET_CROUCHED => "setCrouched",
        CM_TOGGLE_HEAL_DEBUG => "toggleHealDebug",
        CM_REQUEST_HOLSTER_WEAPON => "requestHolsterWeapon",
        // OrganizationMember
        CM_ORG_INVITE_RESPONSE => "organizationInviteResponse",
        CM_ORG_LEAVE => "organizationLeave",
        CM_ORG_BROADCAST_MINIMAP_PING => "BroadcastMinimapPing",
        CM_ORG_STRIKE_TEAM_RESPONSE => "strikeTeamResponse",
        CM_ORG_PVP_LEAVE_RESPONSE => "pvpOrganizationLeaveResponse",
        CM_ORG_MOTD => "organizationMOTD",
        CM_ORG_NOTE => "organizationNote",
        CM_ORG_OFFICER_NOTE => "organizationOfficerNote",
        CM_ORG_SET_RANK_PERMISSIONS => "organizationSetRankPermissions",
        CM_ORG_SET_RANK_NAME => "organizationSetRankName",
        CM_ORG_SQUAD_SET_LOOT_MODE => "squadSetLootMode",
        CM_ORG_TRANSFER_CASH => "organizationTransferCash",
        // MinigamePlayer
        CM_MG_DEBUG_START => "debugStartMinigame",
        CM_MG_DEBUG_SPECTATE => "debugSpectateMinigame",
        CM_MG_DEBUG_JOIN => "debugJoinMinigame",
        CM_MG_DEBUG_INSTANCE => "debugMinigameInstance",
        CM_MG_START => "startMinigame",
        CM_MG_END_CURRENT => "endCurrentMinigame",
        CM_MG_REQUEST_SPECTATE_LIST => "requestSpectateList",
        CM_MG_SPECTATE => "spectateMinigame",
        CM_MG_REGISTER_HELP => "registerToMinigameHelp",
        CM_MG_UPDATE_REGISTER_HELP => "updateRegisterToMinigameHelp",
        CM_MG_START_CANCEL => "minigameStartCancel",
        CM_MG_CALL_ACCEPT => "minigameCallAccept",
        CM_MG_CALL_DECLINE => "minigameCallDecline",
        CM_MG_CALL_ABORT => "minigameCallAbort",
        CM_MG_CONTACT_REQUEST => "minigameContactRequest",
        // GateTravel
        CM_ON_DIAL_GATE => "onDialGate",
        // SGWInventoryManager
        CM_REMOVE_ITEM => "removeItem",
        CM_LIST_ITEMS => "listItems",
        CM_MOVE_ITEM => "moveItem",
        CM_USE_ITEM => "useItem",
        CM_REPAIR_ITEM_REQUEST => "repairItemRequest",
        CM_REQUEST_ACTIVE_SLOT_CHANGE => "requestActiveSlotChange",
        CM_REQUEST_AMMO_CHANGE => "requestAmmoChange",
        // SGWMailManager
        CM_REQUEST_MAIL_HEADERS => "requestMailHeaders",
        CM_SEND_MAIL_MESSAGE => "sendMailMessage",
        CM_ARCHIVE_MAIL_MESSAGE => "archiveMailMessage",
        CM_DELETE_MAIL_MESSAGE => "deleteMailMessage",
        CM_RETURN_MAIL_MESSAGE => "returnMailMessage",
        CM_REQUEST_MAIL_BODY => "requestMailBody",
        CM_TAKE_CASH_FROM_MAIL => "takeCashFromMailMessage",
        CM_TAKE_ITEM_FROM_MAIL => "takeItemFromMailMessage",
        CM_PAY_COD_FOR_MAIL => "payCODForMailMessage",
        // Missionary
        CM_ABANDON_MISSION => "abandonMission",
        CM_SHARE_MISSION => "shareMission",
        CM_SHARE_MISSION_RESPONSE => "shareMissionResponse",
        // ContactListManager
        CM_CONTACT_LIST_CREATE => "contactListCreate",
        CM_CONTACT_LIST_DELETE => "contactListDelete",
        CM_CONTACT_LIST_RENAME => "contactListRename",
        CM_CONTACT_LIST_FLAGS_UPDATE => "contactListFlagsUpdate",
        CM_CONTACT_LIST_ADD_MEMBERS => "contactListAddMembers",
        CM_CONTACT_LIST_REMOVE_MEMBERS => "contactListRemoveMembers",
        // SGWBlackMarketManager
        CM_BM_SEARCH => "BMSearch",
        CM_BM_CREATE_AUCTION => "BMCreateAuction",
        CM_BM_PLACE_BID => "BMPlaceBid",
        CM_BM_CANCEL_AUCTION => "BMCancelAuction",
        CM_BM_START_WATCHING => "BMStartWatchingItem",
        CM_BM_STOP_WATCHING => "BMStopWatchingItem",
        // SGWPlayer own
        CM_CALL_FOR_AID => "callForAid",
        CM_USE_ABILITY => "useAbility",
        CM_USE_ABILITY_ON_GROUND => "useAbilityOnGroundTarget",
        CM_RESPAWN => "respawn",
        CM_UNSTUCK => "unstuck",
        CM_RESET_MY_ABILITIES => "resetMyAbilities",
        CM_WHO => "who",
        CM_INTERACT => "interact",
        CM_DIALOG_BUTTON_CHOICE => "dialogButtonChoice",
        CM_INITIAL_RESPONSE => "initialResponse",
        CM_TRAIN_ABILITY => "trainAbility",
        CM_PURCHASE_ITEMS => "purchaseItems",
        CM_SELL_ITEMS => "sellItems",
        CM_BUYBACK_ITEMS => "buybackItems",
        CM_REPAIR_ITEMS => "repairItems",
        CM_RECHARGE_ITEMS => "rechargeItems",
        CM_SET_AUTO_CYCLE => "setAutoCycle",
        CM_LOOT_ITEM => "lootItem",
        CM_TRIGGER_REGION => "triggerClientHintedGenericRegion",
        CM_REQUEST_RELOAD => "requestReload",
        CM_CHOSEN_REWARDS => "chosenRewards",
        CM_PET_INVOKE_ABILITY => "petInvokeAbility",
        CM_PET_ABILITY_TOGGLE => "petAbilityToggle",
        CM_PET_CHANGE_STANCE => "petChangeStance",
        CM_SET_RING_TRANSPORTER_DEST => "setRingTransporterDestination",
        CM_WORLD_INSTANCE_RESET => "onWorldInstanceReset",
        CM_UPDATE_SYSTEM_OPTIONS => "updateSystemOptions",
        CM_ORG_CREATION => "onOrganizationCreation",
        CM_SPEND_APPLIED_SCIENCE_POINTS => "spendAppliedSciencePoints",
        CM_CRAFT => "craft",
        CM_RESEARCH => "research",
        CM_REVERSE_ENGINEER => "reverseEngineer",
        CM_ALLOYING => "alloying",
        CM_RESPEC_CRAFTING => "respecCrafting",
        CM_CLIENT_CHALLENGE_RESPONSE => "onClientChallengeResponse",
        CM_SEND_DUEL_RESPONSE => "sendDuelResponse",
        CM_DUEL_FORFEIT => "duelForfeit",
        CM_TRADE_REQUEST => "tradeRequest",
        CM_TRADE_REQUEST_CANCEL => "tradeRequestCancel",
        CM_TRADE_UPDATE_PROPOSAL => "tradeUpdateProposal",
        CM_TRADE_LOCK_STATE => "tradeLockState",
        CM_CANCEL_MOVIE => "cancelMovie",
        _ => "unknown",
    }
}

// ── Respawn ──────────────────────────────────────────────────────────────────

/// Handle player respawn: restore health/focus, clear dead state, send updates.
///
/// Reference: `python/cell/SGWBeing.py:revive()` — restores pools to max,
/// clears combatantState dead flag, sends stat and state field updates.
async fn handle_respawn(
    entity_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let entity = match space_mgr.get_entity_mut(entity_id) {
        Some(e) => e,
        None => {
            tracing::warn!(entity_id, "respawn: entity not found");
            return;
        }
    };

    // Restore health and focus to max
    if let Some(health) = entity.stats.get_mut(HEALTH) {
        health.set_current(health.max);
    }
    if let Some(focus) = entity.stats.get_mut(FOCUS) {
        focus.set_current(focus.max);
    }

    // Serialize stat update (health + focus now dirty)
    let stat_update = entity.stats.serialize_dirty();
    entity.stats.clear_dirty();

    // Clear dead state
    let mut state_field = 0u32;
    combat::clear_dead_state(&mut state_field);

    // Clear all ability cooldowns
    entity.abilities.clear_all_cooldowns();

    // Reset position to spawn point if available
    // (Future: read spawn point from DB or initial position)

    tracing::info!(entity_id, "Player respawned");

    // Send onStatUpdate (index 20) — restored health/focus
    let _ = tx.send(CellToBaseMsg::EntityMethodCall {
        entity_id,
        method_index: 20,
        args: stat_update,
    }).await;

    // Send onStateFieldUpdate (index 19) — cleared dead flag
    let mut state_args = Vec::with_capacity(4);
    state_args.extend_from_slice(&state_field.to_le_bytes());
    let _ = tx.send(CellToBaseMsg::EntityMethodCall {
        entity_id,
        method_index: 19,
        args: state_args,
    }).await;
}

// ── Reload ────────────────────────────────────────────────────────────────────

/// Handle player weapon reload: reset ammo to max and send stat update.
///
/// Reference: `python/cell/SGWPlayer.py:1482-1498`
async fn handle_reload(
    entity_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let entity = match space_mgr.get_entity_mut(entity_id) {
        Some(e) => e,
        None => {
            tracing::warn!(entity_id, "requestReload: entity not found");
            return;
        }
    };

    // Reset ammo to max
    let old = entity.current_ammo;
    entity.current_ammo = entity.max_ammo;

    if old == entity.max_ammo {
        tracing::debug!(entity_id, "requestReload: already at max ammo");
        return;
    }

    tracing::info!(entity_id, old, new = entity.max_ammo, "Weapon reloaded");

    // Send onEntityProperty(GENERICPROPERTY_AmmoTypeId, ammo_type)
    // This tells the client to update the ammo display
    let mut args = Vec::with_capacity(8);
    args.extend_from_slice(&7i32.to_le_bytes()); // GENERICPROPERTY_AmmoTypeId = 7
    args.extend_from_slice(&entity.ammo_type.to_le_bytes());
    let _ = tx.send(CellToBaseMsg::EntityMethodCall {
        entity_id,
        method_index: 7, // onEntityProperty (SGWSpawnableEntity, flat index 7)
        args,
    }).await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cell_method_name_known() {
        assert_eq!(cell_method_name(CM_SET_TARGET_ID), "setTargetID");
        assert_eq!(cell_method_name(CM_SET_CROUCHED), "setCrouched");
        assert_eq!(cell_method_name(CM_REQUEST_HOLSTER_WEAPON), "requestHolsterWeapon");
    }

    #[test]
    fn cell_method_name_unknown() {
        assert_eq!(cell_method_name(255), "unknown");
    }

    #[test]
    fn indices_are_sequential() {
        // SGWBeing exposed CellMethods come first
        assert_eq!(CM_SET_TARGET_ID, 0);
        assert_eq!(CM_SET_MOVEMENT_TYPE, 1);
        // Then SGWAbilityManager
        assert_eq!(CM_TOGGLE_COMBAT_DEBUG, 2);
        assert_eq!(CM_TOGGLE_COMBAT_VERBOSE_DEBUG, 3);
        assert_eq!(CM_CONFIRMATION_RESPONSE, 4);
        // Then SGWCombatant
        assert_eq!(CM_SET_CROUCHED, 5);
        assert_eq!(CM_TOGGLE_HEAL_DEBUG, 6);
        assert_eq!(CM_REQUEST_HOLSTER_WEAPON, 7);
    }

    #[test]
    fn all_109_methods_have_names() {
        // Every index from 0-108 should resolve to a known method name
        for i in 0u16..=108 {
            let name = cell_method_name(i);
            assert_ne!(name, "unknown", "Index {} should have a name", i);
        }
    }

    #[test]
    fn all_109_method_constants_are_correct() {
        // Spot-check interface boundaries and key methods
        // OrganizationMember starts at 8
        assert_eq!(CM_ORG_INVITE_RESPONSE, 8);
        assert_eq!(CM_ORG_TRANSFER_CASH, 19);
        // MinigamePlayer starts at 20
        assert_eq!(CM_MG_DEBUG_START, 20);
        assert_eq!(CM_MG_CONTACT_REQUEST, 34);
        // GateTravel
        assert_eq!(CM_ON_DIAL_GATE, 35);
        // SGWInventoryManager
        assert_eq!(CM_REMOVE_ITEM, 36);
        assert_eq!(CM_REQUEST_AMMO_CHANGE, 42);
        // SGWMailManager
        assert_eq!(CM_REQUEST_MAIL_HEADERS, 43);
        assert_eq!(CM_PAY_COD_FOR_MAIL, 51);
        // Missionary
        assert_eq!(CM_ABANDON_MISSION, 52);
        assert_eq!(CM_SHARE_MISSION_RESPONSE, 54);
        // ContactListManager
        assert_eq!(CM_CONTACT_LIST_CREATE, 55);
        assert_eq!(CM_CONTACT_LIST_REMOVE_MEMBERS, 60);
        // SGWBlackMarketManager
        assert_eq!(CM_BM_SEARCH, 61);
        assert_eq!(CM_BM_STOP_WATCHING, 66);
        // SGWPlayer own
        assert_eq!(CM_CALL_FOR_AID, 67);
        assert_eq!(CM_DIALOG_BUTTON_CHOICE, 75);
        assert_eq!(CM_INITIAL_RESPONSE, 76);
        assert_eq!(CM_TRIGGER_REGION, 85);
        assert_eq!(CM_REQUEST_RELOAD, 86);
        assert_eq!(CM_CANCEL_MOVIE, 108);
    }

    // ── New method index tests ────────────────────────────────────────────

    #[test]
    fn new_method_indices_correct() {
        assert_eq!(CM_TRIGGER_REGION, 85);
        assert_eq!(CM_REQUEST_RELOAD, 86);
    }

    #[test]
    fn new_method_names_resolve() {
        assert_eq!(cell_method_name(CM_TRIGGER_REGION), "triggerClientHintedGenericRegion");
        assert_eq!(cell_method_name(CM_REQUEST_RELOAD), "requestReload");
    }

    #[test]
    fn quest_critical_method_names() {
        assert_eq!(cell_method_name(CM_DIALOG_BUTTON_CHOICE), "dialogButtonChoice");
        assert_eq!(cell_method_name(CM_INITIAL_RESPONSE), "initialResponse");
        assert_eq!(cell_method_name(CM_USE_ITEM), "useItem");
    }

    // ── Dispatch integration tests (async) ────────────────────────────────

    fn make_test_space_mgr() -> SpaceManager {
        let mut mgr = SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="true" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
        let cxml = r#"<?xml version="1.0"?><Spaces></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(cxml).unwrap();
        mgr
    }

    #[tokio::test]
    async fn dispatch_trigger_region_enter_fires_event() {
        use crate::cell::space_manager::RegionData;
        let mut mgr = make_test_space_mgr();
        mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3]).unwrap();
        if let Some(e) = mgr.get_entity_mut(1) {
            e.player_id = Some(100);
        }

        // Register a region with runtime_id=2 so the dispatch can look it up
        mgr.regions.insert(2, RegionData {
            runtime_id: 2,
            db_set_id: 42,
            tag: "Castle_Cellblock.Region2".to_string(),
            world_name: "Castle_CellBlock".to_string(),
            height: 0.0,
            radius: 0.0,
            flags: 1,
            points: vec![[0.0; 3]; 4],
        });

        let engine = cimmeria_content_engine::chain::ChainEngine::new();
        let (tx, mut rx) = mpsc::channel(16);

        // Build args: INT32 region_id=2, UINT8 bEntering=1, VECTOR3 position
        let mut args = Vec::new();
        args.extend_from_slice(&2i32.to_le_bytes());  // region_id
        args.push(1);                                  // bEntering = true
        args.extend_from_slice(&0.0f32.to_le_bytes()); // x
        args.extend_from_slice(&0.0f32.to_le_bytes()); // y
        args.extend_from_slice(&0.0f32.to_le_bytes()); // z

        dispatch_cell_method(1, CM_TRIGGER_REGION, &args, &tx, &mut mgr, &engine).await;

        // No chains registered so no messages, but no panic = dispatch worked
        assert!(rx.try_recv().is_err(), "Empty engine should produce no messages");
    }

    #[tokio::test]
    async fn dispatch_trigger_region_exit() {
        use crate::cell::space_manager::RegionData;
        let mut mgr = make_test_space_mgr();
        mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3]).unwrap();

        // Register a region with runtime_id=3
        mgr.regions.insert(3, RegionData {
            runtime_id: 3,
            db_set_id: 43,
            tag: "Castle_Cellblock.Region3".to_string(),
            world_name: "Castle_CellBlock".to_string(),
            height: 0.0,
            radius: 0.0,
            flags: 1,
            points: vec![[0.0; 3]; 4],
        });

        let engine = cimmeria_content_engine::chain::ChainEngine::new();
        let (tx, _rx) = mpsc::channel(16);

        let mut args = Vec::new();
        args.extend_from_slice(&3i32.to_le_bytes());
        args.push(0); // bEntering = false (exit)
        args.extend_from_slice(&[0u8; 12]);

        dispatch_cell_method(1, CM_TRIGGER_REGION, &args, &tx, &mut mgr, &engine).await;
        // No panic = success
    }

    #[tokio::test]
    async fn dispatch_trigger_region_unknown_id_warns() {
        let mut mgr = make_test_space_mgr();
        mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3]).unwrap();

        // No regions registered — runtime_id 99 should be unknown
        let engine = cimmeria_content_engine::chain::ChainEngine::new();
        let (tx, mut rx) = mpsc::channel(16);

        let mut args = Vec::new();
        args.extend_from_slice(&99i32.to_le_bytes());
        args.push(1);
        args.extend_from_slice(&[0u8; 12]);

        dispatch_cell_method(1, CM_TRIGGER_REGION, &args, &tx, &mut mgr, &engine).await;
        // Should warn but not panic, and produce no messages
        assert!(rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn dispatch_trigger_region_ignores_short_args() {
        let mut mgr = make_test_space_mgr();
        mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3]).unwrap();

        let engine = cimmeria_content_engine::chain::ChainEngine::new();
        let (tx, _rx) = mpsc::channel(16);

        // Only 4 bytes — less than required 17
        let args = vec![0u8; 4];
        dispatch_cell_method(1, CM_TRIGGER_REGION, &args, &tx, &mut mgr, &engine).await;
        // Should silently skip (no panic)
    }

    #[tokio::test]
    async fn dispatch_reload_sends_entity_property() {
        let mut mgr = make_test_space_mgr();
        mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3]).unwrap();

        // Set up ammo state
        if let Some(e) = mgr.get_entity_mut(1) {
            e.current_ammo = 5;
            e.max_ammo = 30;
            e.ammo_type = 2;
        }

        let engine = cimmeria_content_engine::chain::ChainEngine::new();
        let (tx, mut rx) = mpsc::channel(16);

        let args = vec![0u8]; // reloadType = 0
        dispatch_cell_method(1, CM_REQUEST_RELOAD, &args, &tx, &mut mgr, &engine).await;

        // Should reset ammo
        assert_eq!(mgr.get_entity(1).unwrap().current_ammo, 30);

        // Should have sent onEntityProperty
        let msg = rx.try_recv().unwrap();
        match msg {
            CellToBaseMsg::EntityMethodCall { entity_id, method_index, args } => {
                assert_eq!(entity_id, 1);
                assert_eq!(method_index, 7); // onEntityProperty
                // First 4 bytes: GENERICPROPERTY_AmmoTypeId = 7
                let prop_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                assert_eq!(prop_id, 7);
                // Next 4 bytes: ammo_type = 2
                let ammo = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                assert_eq!(ammo, 2);
            }
            _ => panic!("Expected EntityMethodCall"),
        }
    }

    #[tokio::test]
    async fn dispatch_reload_already_full_no_message() {
        let mut mgr = make_test_space_mgr();
        mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3]).unwrap();

        // Already at max
        if let Some(e) = mgr.get_entity_mut(1) {
            e.current_ammo = 30;
            e.max_ammo = 30;
        }

        let engine = cimmeria_content_engine::chain::ChainEngine::new();
        let (tx, mut rx) = mpsc::channel(16);

        dispatch_cell_method(1, CM_REQUEST_RELOAD, &[0u8], &tx, &mut mgr, &engine).await;

        // No message sent when already full
        assert!(rx.try_recv().is_err());
    }
}
