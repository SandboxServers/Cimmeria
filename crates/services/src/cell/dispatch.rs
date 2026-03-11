//! Client→server cell method dispatch for the SGWPlayer entity type.
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
//! ### SGWBeing interface CellMethods (exposed only)
//! | Index | Method | Args |
//! |-------|--------|------|
//! | 0 | setTargetID | INT32 targetId |
//! | 1 | setMovementType | UINT8 movementType |
//!
//! ### SGWAbilityManager interface CellMethods (exposed only)
//! | 2 | toggleCombatDebug | (none) |
//! | 3 | toggleCombatVerboseDebug | (none) |
//! | 4 | confirmationResponse | INT32 effectId, UINT8 accepted |
//!
//! ### SGWCombatant interface CellMethods (exposed only)
//! | 5 | setCrouched | INT8 enabled |
//! | 6 | toggleHealDebug | (none) |
//! | 7 | requestHolsterWeapon | INT8 holster |
//!
//! ### SGWPlayer interface + own CellMethods (exposed only)
//! Indices 8+ follow the interface traversal order then SGWPlayer's own methods.
//! (Full mapping TBD as handlers are implemented.)

use tokio::sync::mpsc;

use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_entity::stats::{HEALTH, FOCUS};

use super::combat;
use super::messages::CellToBaseMsg;
use super::space_manager::SpaceManager;

// ── Flattened EXPOSED CellMethod indices ────────────────────────────────────

/// SGWBeing: setTargetID(INT32)
pub const CM_SET_TARGET_ID: u16 = 0;
/// SGWBeing: setMovementType(UINT8)
pub const CM_SET_MOVEMENT_TYPE: u16 = 1;
/// SGWAbilityManager: toggleCombatDebug()
pub const CM_TOGGLE_COMBAT_DEBUG: u16 = 2;
/// SGWAbilityManager: toggleCombatVerboseDebug()
pub const CM_TOGGLE_COMBAT_VERBOSE_DEBUG: u16 = 3;
/// SGWAbilityManager: confirmationResponse(INT32, UINT8)
pub const CM_CONFIRMATION_RESPONSE: u16 = 4;
/// SGWCombatant: setCrouched(INT8)
pub const CM_SET_CROUCHED: u16 = 5;
/// SGWCombatant: toggleHealDebug()
pub const CM_TOGGLE_HEAL_DEBUG: u16 = 6;
/// SGWCombatant: requestHolsterWeapon(INT8)
pub const CM_REQUEST_HOLSTER_WEAPON: u16 = 7;

// OrganizationMember: indices 8–19 (12 exposed)
/// First OrganizationMember exposed CellMethod index.
pub const CM_ORG_FIRST: u16 = 8;
/// Last OrganizationMember exposed CellMethod index.
pub const CM_ORG_LAST: u16 = 19;
// MinigamePlayer: indices 20–34 (15 exposed)
/// First MinigamePlayer exposed CellMethod index.
pub const CM_MINIGAME_FIRST: u16 = 20;
/// Last MinigamePlayer exposed CellMethod index.
pub const CM_MINIGAME_LAST: u16 = 34;
// GateTravel: index 35 (1 exposed: onDialGate)
/// GateTravel: onDialGate(INT32 TargetAddressId, INT32 SourceAddressId)
pub const CM_ON_DIAL_GATE: u16 = 35;
// SGWInventoryManager: indices 36–42 (7 exposed)
// SGWMailManager: indices 43–51 (9 exposed)
/// SGWMailManager: requestMailHeaders(UINT8)
pub const CM_REQUEST_MAIL_HEADERS: u16 = 43;
/// SGWMailManager: sendMailMessage(INT32, ARRAY<WSTRING>, WSTRING, WSTRING, INT32, UINT8, INT32, INT32)
pub const CM_SEND_MAIL_MESSAGE: u16 = 44;
/// SGWMailManager: archiveMailMessage(INT32)
pub const CM_ARCHIVE_MAIL_MESSAGE: u16 = 45;
/// SGWMailManager: deleteMailMessage(INT32)
pub const CM_DELETE_MAIL_MESSAGE: u16 = 46;
/// SGWMailManager: returnMailMessage(INT32)
pub const CM_RETURN_MAIL_MESSAGE: u16 = 47;
/// SGWMailManager: requestMailBody(INT32)
pub const CM_REQUEST_MAIL_BODY: u16 = 48;
/// SGWMailManager: takeCashFromMailMessage(INT32)
pub const CM_TAKE_CASH_FROM_MAIL: u16 = 49;
/// SGWMailManager: takeItemFromMailMessage(INT32, INT32, INT32)
pub const CM_TAKE_ITEM_FROM_MAIL: u16 = 50;
/// SGWMailManager: payCODForMailMessage(INT32)
pub const CM_PAY_COD_FOR_MAIL: u16 = 51;
// Missionary: indices 52–54 (3 exposed)
// SGWPoller: 0 exposed
// ContactListManager: indices 55–60 (6 exposed)
/// ContactListManager: contactListCreate(WSTRING, UINT32)
pub const CM_CONTACT_LIST_CREATE: u16 = 55;
/// ContactListManager: contactListDelete(INT32)
pub const CM_CONTACT_LIST_DELETE: u16 = 56;
/// ContactListManager: contactListRename(INT32, WSTRING)
pub const CM_CONTACT_LIST_RENAME: u16 = 57;
/// ContactListManager: contactListFlagsUpdate(INT32, UINT32)
pub const CM_CONTACT_LIST_FLAGS_UPDATE: u16 = 58;
/// ContactListManager: contactListAddMembers(INT32, ARRAY<WSTRING>)
pub const CM_CONTACT_LIST_ADD_MEMBERS: u16 = 59;
/// ContactListManager: contactListRemoveMembers(INT32, ARRAY<WSTRING>)
pub const CM_CONTACT_LIST_REMOVE_MEMBERS: u16 = 60;
// SGWBlackMarketManager: indices 61–66 (6 exposed)
/// First SGWBlackMarketManager exposed CellMethod index.
pub const CM_BLACK_MARKET_FIRST: u16 = 61;
/// Last SGWBlackMarketManager exposed CellMethod index.
pub const CM_BLACK_MARKET_LAST: u16 = 66;
// ClientCache: 0 exposed

// Missionary: indices 52–54 (3 exposed)
/// Missionary: abandonMission(INT32)
pub const CM_ABANDON_MISSION: u16 = 52;
/// Missionary: shareMission(INT32)
pub const CM_SHARE_MISSION: u16 = 53;
/// Missionary: shareMissionResponse(INT8)
pub const CM_SHARE_MISSION_RESPONSE: u16 = 54;

// SGWPlayer own CellMethods (exposed only, starting at 67)
/// SGWPlayer: callForAid()
pub const CM_CALL_FOR_AID: u16 = 67;
/// SGWPlayer: useAbility(INT32 AbilityID, INT32 TargetID)
pub const CM_USE_ABILITY: u16 = 68;
/// SGWPlayer: useAbilityOnGroundTarget(INT32 AbilityID, FLOAT x, FLOAT y, FLOAT z)
pub const CM_USE_ABILITY_ON_GROUND: u16 = 69;
/// SGWPlayer: respawn()
pub const CM_RESPAWN: u16 = 70;
/// SGWPlayer: interact(INT32 entityId)
pub const CM_INTERACT: u16 = 74;
/// SGWPlayer: setAutoCycle(INT8 enabled)
pub const CM_SET_AUTO_CYCLE: u16 = 83;

// ── Dispatch ────────────────────────────────────────────────────────────────

/// Dispatch a client→server cell method call to the appropriate handler.
pub async fn dispatch_cell_method(
    entity_id: u32,
    method_index: u16,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    engine: &ChainEngine,
) {
    match method_index {
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

        CM_CONFIRMATION_RESPONSE => {
            if args.len() >= 5 {
                let effect_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let accepted = args[4] != 0;
                tracing::debug!(entity_id, effect_id, accepted, "confirmationResponse");
                // TODO: Process effect confirmation
            }
        }

        CM_TOGGLE_COMBAT_DEBUG | CM_TOGGLE_COMBAT_VERBOSE_DEBUG | CM_TOGGLE_HEAL_DEBUG => {
            tracing::debug!(entity_id, method_index, "Debug toggle (no-op)");
        }

        CM_USE_ABILITY => {
            if args.len() >= 8 {
                let ability_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let target_id = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                tracing::debug!(entity_id, ability_id, target_id, "useAbility");
                super::abilities::handle_use_ability(entity_id, ability_id, target_id, tx, space_mgr).await;
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

        CM_RESPAWN => {
            tracing::debug!(entity_id, "respawn");
            handle_respawn(entity_id, tx, space_mgr).await;
        }

        CM_ABANDON_MISSION => {
            if args.len() >= 4 {
                let mission_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::debug!(entity_id, mission_id, "abandonMission");
                super::missions::abandon_mission(entity_id, mission_id, tx, space_mgr).await;
            }
        }

        CM_SHARE_MISSION | CM_SHARE_MISSION_RESPONSE => {
            tracing::debug!(entity_id, method_index, "Mission sharing (not implemented)");
        }

        CM_REQUEST_MAIL_HEADERS => {
            let b_archive = if !args.is_empty() { args[0] } else { 0 };
            tracing::debug!(entity_id, b_archive, "requestMailHeaders");
            super::mail::handle_request_mail_headers(entity_id, b_archive, tx).await;
        }

        CM_REQUEST_MAIL_BODY => {
            if args.len() >= 4 {
                let mail_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::debug!(entity_id, mail_id, "requestMailBody");
                super::mail::handle_request_mail_body(entity_id, mail_id, tx).await;
            }
        }

        CM_DELETE_MAIL_MESSAGE => {
            if args.len() >= 4 {
                let mail_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::debug!(entity_id, mail_id, "deleteMailMessage");
                super::mail::handle_delete_mail(entity_id, mail_id, tx).await;
            }
        }

        CM_ARCHIVE_MAIL_MESSAGE => {
            if args.len() >= 4 {
                let mail_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::debug!(entity_id, mail_id, "archiveMailMessage");
                super::mail::handle_archive_mail(entity_id, mail_id, tx).await;
            }
        }

        CM_SEND_MAIL_MESSAGE | CM_RETURN_MAIL_MESSAGE |
        CM_TAKE_CASH_FROM_MAIL | CM_TAKE_ITEM_FROM_MAIL | CM_PAY_COD_FOR_MAIL => {
            tracing::debug!(entity_id, method_index, "Mail operation (not yet implemented)");
        }

        CM_CONTACT_LIST_CREATE | CM_CONTACT_LIST_DELETE | CM_CONTACT_LIST_RENAME |
        CM_CONTACT_LIST_FLAGS_UPDATE | CM_CONTACT_LIST_ADD_MEMBERS |
        CM_CONTACT_LIST_REMOVE_MEMBERS => {
            tracing::debug!(entity_id, method_index, "Contact list operation (stub — no persistence)");
        }

        idx if (CM_ORG_FIRST..=CM_ORG_LAST).contains(&idx) => {
            tracing::debug!(entity_id, method_index, "Organization operation (stub)");
        }

        idx if (CM_MINIGAME_FIRST..=CM_MINIGAME_LAST).contains(&idx) => {
            tracing::debug!(entity_id, method_index, "Minigame operation (stub)");
        }

        idx if (CM_BLACK_MARKET_FIRST..=CM_BLACK_MARKET_LAST).contains(&idx) => {
            tracing::debug!(entity_id, method_index, "Black market operation (stub)");
        }

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

        CM_INTERACT => {
            if args.len() >= 4 {
                let target_entity_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::debug!(entity_id, target_entity_id, "interact");
                let dialog_id = super::interactions::handle_interact(
                    entity_id, target_entity_id as u32, tx, space_mgr,
                ).await;

                // Fire content engine event if a dialog was opened
                if let Some(did) = dialog_id {
                    // Get player_id from entity (0 fallback since DB persist uses entity lookup)
                    let player_id = space_mgr.get_entity(entity_id)
                        .and_then(|e| e.player_id)
                        .unwrap_or(0);
                    super::content::fire_dialog_open(
                        entity_id, player_id, did, engine, tx, space_mgr,
                    ).await;
                }
            }
        }

        _ => {
            tracing::trace!(
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
        CM_SET_TARGET_ID => "setTargetID",
        CM_SET_MOVEMENT_TYPE => "setMovementType",
        CM_TOGGLE_COMBAT_DEBUG => "toggleCombatDebug",
        CM_TOGGLE_COMBAT_VERBOSE_DEBUG => "toggleCombatVerboseDebug",
        CM_CONFIRMATION_RESPONSE => "confirmationResponse",
        CM_SET_CROUCHED => "setCrouched",
        CM_TOGGLE_HEAL_DEBUG => "toggleHealDebug",
        CM_REQUEST_HOLSTER_WEAPON => "requestHolsterWeapon",
        CM_REQUEST_MAIL_HEADERS => "requestMailHeaders",
        CM_SEND_MAIL_MESSAGE => "sendMailMessage",
        CM_ARCHIVE_MAIL_MESSAGE => "archiveMailMessage",
        CM_DELETE_MAIL_MESSAGE => "deleteMailMessage",
        CM_RETURN_MAIL_MESSAGE => "returnMailMessage",
        CM_REQUEST_MAIL_BODY => "requestMailBody",
        CM_TAKE_CASH_FROM_MAIL => "takeCashFromMailMessage",
        CM_TAKE_ITEM_FROM_MAIL => "takeItemFromMailMessage",
        CM_PAY_COD_FOR_MAIL => "payCODForMailMessage",
        CM_CONTACT_LIST_CREATE => "contactListCreate",
        CM_CONTACT_LIST_DELETE => "contactListDelete",
        CM_CONTACT_LIST_RENAME => "contactListRename",
        CM_CONTACT_LIST_FLAGS_UPDATE => "contactListFlagsUpdate",
        CM_CONTACT_LIST_ADD_MEMBERS => "contactListAddMembers",
        CM_CONTACT_LIST_REMOVE_MEMBERS => "contactListRemoveMembers",
        CM_ON_DIAL_GATE => "onDialGate",
        CM_ABANDON_MISSION => "abandonMission",
        CM_SHARE_MISSION => "shareMission",
        CM_SHARE_MISSION_RESPONSE => "shareMissionResponse",
        CM_CALL_FOR_AID => "callForAid",
        CM_USE_ABILITY => "useAbility",
        CM_USE_ABILITY_ON_GROUND => "useAbilityOnGroundTarget",
        CM_RESPAWN => "respawn",
        CM_INTERACT => "interact",
        CM_SET_AUTO_CYCLE => "setAutoCycle",
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
}
