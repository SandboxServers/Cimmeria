//! Backward-compatible `CM_*` and `CLIENT_MG_*` constant re-exports.
//!
//! Other modules (e.g. tests, service.rs) reference these constants by their
//! `crate::cell::dispatch::CM_*` paths. Re-export them from the canonical
//! `cell_methods` / `client_methods` sub-modules so the public surface stays
//! the same after the dispatch.rs split.

use super::super::cell_methods;
use super::super::client_methods;

// ── SGWBeing (0–1) ──────────────────────────────────────────────────────────
pub use cell_methods::being::SET_TARGET_ID as CM_SET_TARGET_ID;
pub use cell_methods::being::SET_MOVEMENT_TYPE as CM_SET_MOVEMENT_TYPE;

// ── SGWAbilityManager (2–4) ─────────────────────────────────────────────────
pub use cell_methods::ability_manager::TOGGLE_COMBAT_DEBUG as CM_TOGGLE_COMBAT_DEBUG;
pub use cell_methods::ability_manager::TOGGLE_COMBAT_VERBOSE_DEBUG as CM_TOGGLE_COMBAT_VERBOSE_DEBUG;
pub use cell_methods::ability_manager::CONFIRMATION_RESPONSE as CM_CONFIRMATION_RESPONSE;

// ── SGWCombatant (5–7) ──────────────────────────────────────────────────────
pub use cell_methods::combatant::SET_CROUCHED as CM_SET_CROUCHED;
pub use cell_methods::combatant::TOGGLE_HEAL_DEBUG as CM_TOGGLE_HEAL_DEBUG;
pub use cell_methods::combatant::REQUEST_HOLSTER_WEAPON as CM_REQUEST_HOLSTER_WEAPON;

// ── OrganizationMember (8–19) ───────────────────────────────────────────────
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

// ── MinigamePlayer (20–34) ──────────────────────────────────────────────────
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

// ── GateTravel (35) ─────────────────────────────────────────────────────────
pub use cell_methods::gate_travel::ON_DIAL_GATE as CM_ON_DIAL_GATE;

// ── SGWInventoryManager (36–42) ─────────────────────────────────────────────
pub use cell_methods::inventory::REMOVE_ITEM as CM_REMOVE_ITEM;
pub use cell_methods::inventory::LIST_ITEMS as CM_LIST_ITEMS;
pub use cell_methods::inventory::MOVE_ITEM as CM_MOVE_ITEM;
pub use cell_methods::inventory::USE_ITEM as CM_USE_ITEM;
pub use cell_methods::inventory::REPAIR_ITEM_REQUEST as CM_REPAIR_ITEM_REQUEST;
pub use cell_methods::inventory::REQUEST_ACTIVE_SLOT_CHANGE as CM_REQUEST_ACTIVE_SLOT_CHANGE;
pub use cell_methods::inventory::REQUEST_AMMO_CHANGE as CM_REQUEST_AMMO_CHANGE;

// ── SGWMailManager (43–51) ──────────────────────────────────────────────────
pub use cell_methods::mail::REQUEST_MAIL_HEADERS as CM_REQUEST_MAIL_HEADERS;
pub use cell_methods::mail::SEND_MAIL_MESSAGE as CM_SEND_MAIL_MESSAGE;
pub use cell_methods::mail::ARCHIVE_MAIL_MESSAGE as CM_ARCHIVE_MAIL_MESSAGE;
pub use cell_methods::mail::DELETE_MAIL_MESSAGE as CM_DELETE_MAIL_MESSAGE;
pub use cell_methods::mail::RETURN_MAIL_MESSAGE as CM_RETURN_MAIL_MESSAGE;
pub use cell_methods::mail::REQUEST_MAIL_BODY as CM_REQUEST_MAIL_BODY;
pub use cell_methods::mail::TAKE_CASH_FROM_MAIL as CM_TAKE_CASH_FROM_MAIL;
pub use cell_methods::mail::TAKE_ITEM_FROM_MAIL as CM_TAKE_ITEM_FROM_MAIL;
pub use cell_methods::mail::PAY_COD_FOR_MAIL as CM_PAY_COD_FOR_MAIL;

// ── Missionary (52–54) ──────────────────────────────────────────────────────
pub use cell_methods::missionary::ABANDON_MISSION as CM_ABANDON_MISSION;
pub use cell_methods::missionary::SHARE_MISSION as CM_SHARE_MISSION;
pub use cell_methods::missionary::SHARE_MISSION_RESPONSE as CM_SHARE_MISSION_RESPONSE;

// ── ContactListManager (55–60) ──────────────────────────────────────────────
pub use cell_methods::contact_list::CREATE as CM_CONTACT_LIST_CREATE;
pub use cell_methods::contact_list::DELETE as CM_CONTACT_LIST_DELETE;
pub use cell_methods::contact_list::RENAME as CM_CONTACT_LIST_RENAME;
pub use cell_methods::contact_list::FLAGS_UPDATE as CM_CONTACT_LIST_FLAGS_UPDATE;
pub use cell_methods::contact_list::ADD_MEMBERS as CM_CONTACT_LIST_ADD_MEMBERS;
pub use cell_methods::contact_list::REMOVE_MEMBERS as CM_CONTACT_LIST_REMOVE_MEMBERS;

// ── SGWBlackMarketManager (61–66) ───────────────────────────────────────────
pub use cell_methods::black_market::SEARCH as CM_BM_SEARCH;
pub use cell_methods::black_market::CREATE_AUCTION as CM_BM_CREATE_AUCTION;
pub use cell_methods::black_market::PLACE_BID as CM_BM_PLACE_BID;
pub use cell_methods::black_market::CANCEL_AUCTION as CM_BM_CANCEL_AUCTION;
pub use cell_methods::black_market::START_WATCHING as CM_BM_START_WATCHING;
pub use cell_methods::black_market::STOP_WATCHING as CM_BM_STOP_WATCHING;

// ── SGWPlayer own (67–108) ──────────────────────────────────────────────────
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
pub use client_methods::minigame::ON_START_MINIGAME as CLIENT_MG_ON_START_MINIGAME;
pub use client_methods::minigame::ON_END_MINIGAME as CLIENT_MG_ON_END_MINIGAME;
