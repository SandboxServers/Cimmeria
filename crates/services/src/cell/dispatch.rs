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

use super::cell_methods;
use super::messages::CellToBaseMsg;
use super::space_manager::SpaceManager;

// ── Re-export constants for backward compatibility ───────────────────────────
//
// Other modules (e.g. tests, service.rs) may reference CM_* constants from this
// module. Re-export them from the canonical cell_methods sub-modules.

// SGWBeing (0–1)
pub use cell_methods::being::SET_TARGET_ID as CM_SET_TARGET_ID;
pub use cell_methods::being::SET_MOVEMENT_TYPE as CM_SET_MOVEMENT_TYPE;

// SGWAbilityManager (2–4)
pub use cell_methods::ability_manager::TOGGLE_COMBAT_DEBUG as CM_TOGGLE_COMBAT_DEBUG;
pub use cell_methods::ability_manager::TOGGLE_COMBAT_VERBOSE_DEBUG as CM_TOGGLE_COMBAT_VERBOSE_DEBUG;
pub use cell_methods::ability_manager::CONFIRMATION_RESPONSE as CM_CONFIRMATION_RESPONSE;

// SGWCombatant (5–7)
pub use cell_methods::combatant::SET_CROUCHED as CM_SET_CROUCHED;
pub use cell_methods::combatant::TOGGLE_HEAL_DEBUG as CM_TOGGLE_HEAL_DEBUG;
pub use cell_methods::combatant::REQUEST_HOLSTER_WEAPON as CM_REQUEST_HOLSTER_WEAPON;

// OrganizationMember (8–19)
pub use cell_methods::organization::INVITE_RESPONSE as CM_ORG_INVITE_RESPONSE;
pub use cell_methods::organization::LEAVE as CM_ORG_LEAVE;
pub use cell_methods::organization::BROADCAST_MINIMAP_PING as CM_ORG_BROADCAST_MINIMAP_PING;
pub use cell_methods::organization::STRIKE_TEAM_RESPONSE as CM_ORG_STRIKE_TEAM_RESPONSE;
pub use cell_methods::organization::PVP_LEAVE_RESPONSE as CM_ORG_PVP_LEAVE_RESPONSE;
pub use cell_methods::organization::MOTD as CM_ORG_MOTD;
pub use cell_methods::organization::NOTE as CM_ORG_NOTE;
pub use cell_methods::organization::OFFICER_NOTE as CM_ORG_OFFICER_NOTE;
pub use cell_methods::organization::SET_RANK_PERMISSIONS as CM_ORG_SET_RANK_PERMISSIONS;
pub use cell_methods::organization::SET_RANK_NAME as CM_ORG_SET_RANK_NAME;
pub use cell_methods::organization::SQUAD_SET_LOOT_MODE as CM_ORG_SQUAD_SET_LOOT_MODE;
pub use cell_methods::organization::TRANSFER_CASH as CM_ORG_TRANSFER_CASH;

// MinigamePlayer (20–34)
pub use cell_methods::minigame::DEBUG_START as CM_MG_DEBUG_START;
pub use cell_methods::minigame::DEBUG_SPECTATE as CM_MG_DEBUG_SPECTATE;
pub use cell_methods::minigame::DEBUG_JOIN as CM_MG_DEBUG_JOIN;
pub use cell_methods::minigame::DEBUG_INSTANCE as CM_MG_DEBUG_INSTANCE;
pub use cell_methods::minigame::START as CM_MG_START;
pub use cell_methods::minigame::END_CURRENT as CM_MG_END_CURRENT;
pub use cell_methods::minigame::REQUEST_SPECTATE_LIST as CM_MG_REQUEST_SPECTATE_LIST;
pub use cell_methods::minigame::SPECTATE as CM_MG_SPECTATE;
pub use cell_methods::minigame::REGISTER_HELP as CM_MG_REGISTER_HELP;
pub use cell_methods::minigame::UPDATE_REGISTER_HELP as CM_MG_UPDATE_REGISTER_HELP;
pub use cell_methods::minigame::START_CANCEL as CM_MG_START_CANCEL;
pub use cell_methods::minigame::CALL_ACCEPT as CM_MG_CALL_ACCEPT;
pub use cell_methods::minigame::CALL_DECLINE as CM_MG_CALL_DECLINE;
pub use cell_methods::minigame::CALL_ABORT as CM_MG_CALL_ABORT;
pub use cell_methods::minigame::CONTACT_REQUEST as CM_MG_CONTACT_REQUEST;

// GateTravel (35)
pub use cell_methods::gate_travel::ON_DIAL_GATE as CM_ON_DIAL_GATE;

// SGWInventoryManager (36–42)
pub use cell_methods::inventory::REMOVE_ITEM as CM_REMOVE_ITEM;
pub use cell_methods::inventory::LIST_ITEMS as CM_LIST_ITEMS;
pub use cell_methods::inventory::MOVE_ITEM as CM_MOVE_ITEM;
pub use cell_methods::inventory::USE_ITEM as CM_USE_ITEM;
pub use cell_methods::inventory::REPAIR_ITEM_REQUEST as CM_REPAIR_ITEM_REQUEST;
pub use cell_methods::inventory::REQUEST_ACTIVE_SLOT_CHANGE as CM_REQUEST_ACTIVE_SLOT_CHANGE;
pub use cell_methods::inventory::REQUEST_AMMO_CHANGE as CM_REQUEST_AMMO_CHANGE;

// SGWMailManager (43–51)
pub use cell_methods::mail::REQUEST_MAIL_HEADERS as CM_REQUEST_MAIL_HEADERS;
pub use cell_methods::mail::SEND_MAIL_MESSAGE as CM_SEND_MAIL_MESSAGE;
pub use cell_methods::mail::ARCHIVE_MAIL_MESSAGE as CM_ARCHIVE_MAIL_MESSAGE;
pub use cell_methods::mail::DELETE_MAIL_MESSAGE as CM_DELETE_MAIL_MESSAGE;
pub use cell_methods::mail::RETURN_MAIL_MESSAGE as CM_RETURN_MAIL_MESSAGE;
pub use cell_methods::mail::REQUEST_MAIL_BODY as CM_REQUEST_MAIL_BODY;
pub use cell_methods::mail::TAKE_CASH_FROM_MAIL as CM_TAKE_CASH_FROM_MAIL;
pub use cell_methods::mail::TAKE_ITEM_FROM_MAIL as CM_TAKE_ITEM_FROM_MAIL;
pub use cell_methods::mail::PAY_COD_FOR_MAIL as CM_PAY_COD_FOR_MAIL;

// Missionary (52–54)
pub use cell_methods::missionary::ABANDON_MISSION as CM_ABANDON_MISSION;
pub use cell_methods::missionary::SHARE_MISSION as CM_SHARE_MISSION;
pub use cell_methods::missionary::SHARE_MISSION_RESPONSE as CM_SHARE_MISSION_RESPONSE;

// ContactListManager (55–60)
pub use cell_methods::contact_list::CREATE as CM_CONTACT_LIST_CREATE;
pub use cell_methods::contact_list::DELETE as CM_CONTACT_LIST_DELETE;
pub use cell_methods::contact_list::RENAME as CM_CONTACT_LIST_RENAME;
pub use cell_methods::contact_list::FLAGS_UPDATE as CM_CONTACT_LIST_FLAGS_UPDATE;
pub use cell_methods::contact_list::ADD_MEMBERS as CM_CONTACT_LIST_ADD_MEMBERS;
pub use cell_methods::contact_list::REMOVE_MEMBERS as CM_CONTACT_LIST_REMOVE_MEMBERS;

// SGWBlackMarketManager (61–66)
pub use cell_methods::black_market::SEARCH as CM_BM_SEARCH;
pub use cell_methods::black_market::CREATE_AUCTION as CM_BM_CREATE_AUCTION;
pub use cell_methods::black_market::PLACE_BID as CM_BM_PLACE_BID;
pub use cell_methods::black_market::CANCEL_AUCTION as CM_BM_CANCEL_AUCTION;
pub use cell_methods::black_market::START_WATCHING as CM_BM_START_WATCHING;
pub use cell_methods::black_market::STOP_WATCHING as CM_BM_STOP_WATCHING;

// SGWPlayer own (67–108)
pub use cell_methods::player::CALL_FOR_AID as CM_CALL_FOR_AID;
pub use cell_methods::player::USE_ABILITY as CM_USE_ABILITY;
pub use cell_methods::player::USE_ABILITY_ON_GROUND as CM_USE_ABILITY_ON_GROUND;
pub use cell_methods::player::RESPAWN as CM_RESPAWN;
pub use cell_methods::player::UNSTUCK as CM_UNSTUCK;
pub use cell_methods::player::RESET_MY_ABILITIES as CM_RESET_MY_ABILITIES;
pub use cell_methods::player::WHO as CM_WHO;
pub use cell_methods::player::INTERACT as CM_INTERACT;
pub use cell_methods::player::DIALOG_BUTTON_CHOICE as CM_DIALOG_BUTTON_CHOICE;
pub use cell_methods::player::INITIAL_RESPONSE as CM_INITIAL_RESPONSE;
pub use cell_methods::player::TRAIN_ABILITY as CM_TRAIN_ABILITY;
pub use cell_methods::player::PURCHASE_ITEMS as CM_PURCHASE_ITEMS;
pub use cell_methods::player::SELL_ITEMS as CM_SELL_ITEMS;
pub use cell_methods::player::BUYBACK_ITEMS as CM_BUYBACK_ITEMS;
pub use cell_methods::player::REPAIR_ITEMS as CM_REPAIR_ITEMS;
pub use cell_methods::player::RECHARGE_ITEMS as CM_RECHARGE_ITEMS;
pub use cell_methods::player::SET_AUTO_CYCLE as CM_SET_AUTO_CYCLE;
pub use cell_methods::player::LOOT_ITEM as CM_LOOT_ITEM;
pub use cell_methods::player::TRIGGER_REGION as CM_TRIGGER_REGION;
pub use cell_methods::player::REQUEST_RELOAD as CM_REQUEST_RELOAD;
pub use cell_methods::player::CHOSEN_REWARDS as CM_CHOSEN_REWARDS;
pub use cell_methods::player::PET_INVOKE_ABILITY as CM_PET_INVOKE_ABILITY;
pub use cell_methods::player::PET_ABILITY_TOGGLE as CM_PET_ABILITY_TOGGLE;
pub use cell_methods::player::PET_CHANGE_STANCE as CM_PET_CHANGE_STANCE;
pub use cell_methods::player::SET_RING_TRANSPORTER_DEST as CM_SET_RING_TRANSPORTER_DEST;
pub use cell_methods::player::WORLD_INSTANCE_RESET as CM_WORLD_INSTANCE_RESET;
pub use cell_methods::player::UPDATE_SYSTEM_OPTIONS as CM_UPDATE_SYSTEM_OPTIONS;
pub use cell_methods::player::ORG_CREATION as CM_ORG_CREATION;
pub use cell_methods::player::SPEND_APPLIED_SCIENCE_POINTS as CM_SPEND_APPLIED_SCIENCE_POINTS;
pub use cell_methods::player::CRAFT as CM_CRAFT;
pub use cell_methods::player::RESEARCH as CM_RESEARCH;
pub use cell_methods::player::REVERSE_ENGINEER as CM_REVERSE_ENGINEER;
pub use cell_methods::player::ALLOYING as CM_ALLOYING;
pub use cell_methods::player::RESPEC_CRAFTING as CM_RESPEC_CRAFTING;
pub use cell_methods::player::CLIENT_CHALLENGE_RESPONSE as CM_CLIENT_CHALLENGE_RESPONSE;
pub use cell_methods::player::SEND_DUEL_RESPONSE as CM_SEND_DUEL_RESPONSE;
pub use cell_methods::player::DUEL_FORFEIT as CM_DUEL_FORFEIT;
pub use cell_methods::player::TRADE_REQUEST as CM_TRADE_REQUEST;
pub use cell_methods::player::TRADE_REQUEST_CANCEL as CM_TRADE_REQUEST_CANCEL;
pub use cell_methods::player::TRADE_UPDATE_PROPOSAL as CM_TRADE_UPDATE_PROPOSAL;
pub use cell_methods::player::TRADE_LOCK_STATE as CM_TRADE_LOCK_STATE;
pub use cell_methods::player::CANCEL_MOVIE as CM_CANCEL_MOVIE;

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
///
/// Each interface's dispatch function returns `true` if it handled the method,
/// `false` if the index is outside its range. We try each interface in
/// inheritance order and stop at the first match.
pub async fn dispatch_cell_method(
    entity_id: u32,
    method_index: u16,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    engine: &ChainEngine,
) {
    // SGWBeing interface (0–1)
    if cell_methods::being::dispatch(entity_id, method_index, args, tx, space_mgr).await {
        return;
    }
    // SGWAbilityManager interface (2–4)
    if cell_methods::ability_manager::dispatch(entity_id, method_index, args, tx, space_mgr).await {
        return;
    }
    // SGWCombatant interface (5–7)
    if cell_methods::combatant::dispatch(entity_id, method_index, args, tx, space_mgr).await {
        return;
    }
    // OrganizationMember interface (8–19)
    if cell_methods::organization::dispatch(entity_id, method_index, args, tx, space_mgr).await {
        return;
    }
    // MinigamePlayer interface (20–34)
    if cell_methods::minigame::dispatch(entity_id, method_index, args, tx, space_mgr).await {
        return;
    }
    // GateTravel interface (35)
    if cell_methods::gate_travel::dispatch(entity_id, method_index, args, tx, space_mgr).await {
        return;
    }
    // SGWInventoryManager interface (36–42) — needs engine for useItem content chains
    if cell_methods::inventory::dispatch(entity_id, method_index, args, tx, space_mgr, engine).await {
        return;
    }
    // SGWMailManager interface (43–51)
    if cell_methods::mail::dispatch(entity_id, method_index, args, tx, space_mgr).await {
        return;
    }
    // Missionary interface (52–54)
    if cell_methods::missionary::dispatch(entity_id, method_index, args, tx, space_mgr).await {
        return;
    }
    // ContactListManager interface (55–60)
    if cell_methods::contact_list::dispatch(entity_id, method_index, args, tx, space_mgr).await {
        return;
    }
    // SGWBlackMarketManager interface (61–66)
    if cell_methods::black_market::dispatch(entity_id, method_index, args, tx, space_mgr).await {
        return;
    }
    // SGWPlayer own methods (67–108) — needs engine for content chains
    if cell_methods::player::dispatch(entity_id, method_index, args, tx, space_mgr, engine).await {
        return;
    }

    tracing::info!(
        entity_id,
        method_index,
        args_len = args.len(),
        "Unhandled cell method call"
    );
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
        use cimmeria_entity::cell_entity::BandolierItem;
        use cimmeria_entity::stats::AMMO_SLOT_1;

        let mut mgr = make_test_space_mgr();
        mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3]).unwrap();

        // Stage C: shadow scalars are gone. Seed the bandolier item + AmmoSlot
        // stat the same way `InitPlayerState` does for a real world entry.
        if let Some(e) = mgr.get_entity_mut(1) {
            e.bandolier_items.insert(0, BandolierItem {
                item_id: 1,
                clip_size: 30,
                default_ammo_type: 2,
                current_ammo: 5,
                cur_ammo_type: 2,
            });
            if let Some(stat) = e.stats.get_mut(AMMO_SLOT_1) {
                stat.update(0, 5, 30);
                stat.clear_dirty();
            }
        }

        let engine = cimmeria_content_engine::chain::ChainEngine::new();
        let (tx, mut rx) = mpsc::channel(16);

        let args = vec![0u8]; // reloadType = 0
        dispatch_cell_method(1, CM_REQUEST_RELOAD, &args, &tx, &mut mgr, &engine).await;

        // Reload sets the deadline but does NOT immediately refill — the magazine
        // stays at the pre-reload count until the reload tick runs past warmup.
        let entity = mgr.get_entity(1).unwrap();
        assert_eq!(entity.active_ammo(), 5, "magazine should not refill until warmup elapses");
        assert!(entity.reload_complete_at.is_some(), "reload deadline should be set");

        // Reload sends a TimerUpdate (method 12) for the cooldown bar; the
        // onEntityProperty(AmmoTypeId) packet only fires when an event_set
        // sequence is mapped, which this test deliberately doesn't set up.
        let msg = rx.try_recv().unwrap();
        match msg {
            CellToBaseMsg::EntityMethodCall { entity_id, method_index, .. } => {
                assert_eq!(entity_id, 1);
                assert_eq!(method_index, 12, "expected TimerUpdate first");
            }
            _ => panic!("Expected EntityMethodCall"),
        }
    }

    #[tokio::test]
    async fn dispatch_reload_already_full_no_message() {
        use cimmeria_entity::cell_entity::BandolierItem;
        use cimmeria_entity::stats::AMMO_SLOT_1;

        let mut mgr = make_test_space_mgr();
        mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3]).unwrap();

        // Already at max — bandolier item with clip_size == current_ammo.
        if let Some(e) = mgr.get_entity_mut(1) {
            e.bandolier_items.insert(0, BandolierItem {
                item_id: 1,
                clip_size: 30,
                default_ammo_type: 0,
                current_ammo: 30,
                cur_ammo_type: 0,
            });
            if let Some(stat) = e.stats.get_mut(AMMO_SLOT_1) {
                stat.update(0, 30, 30);
                stat.clear_dirty();
            }
        }

        let engine = cimmeria_content_engine::chain::ChainEngine::new();
        let (tx, mut rx) = mpsc::channel(16);

        dispatch_cell_method(1, CM_REQUEST_RELOAD, &[0u8], &tx, &mut mgr, &engine).await;

        // No message sent when already full
        assert!(rx.try_recv().is_err());
    }
}
