//! MinigamePlayer interface exposed CellMethods (indices 20–34).

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use tokio::sync::mpsc;

pub use cimmeria_wire::cell::cell_methods::minigame::{
    CALL_ABORT, CALL_ACCEPT, CALL_DECLINE, CONTACT_REQUEST, DEBUG_INSTANCE, DEBUG_JOIN,
    DEBUG_SPECTATE, DEBUG_START, END_CURRENT, REGISTER_HELP, REQUEST_SPECTATE_LIST, SPECTATE,
    START, START_CANCEL, UPDATE_REGISTER_HELP,
};

pub async fn dispatch(
    entity_id: u32,
    method_index: u16,
    args: &[u8],
    _tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
    // Rare client calls: each arm resolves its names only when it logs.
    let mgr = &*space_mgr;
    match method_index {
        DEBUG_START => {
            if args.len() >= 4 {
                let game_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::info!(
                    entity_id,
                    entity_name = mgr.entity_label(entity_id),
                    game_id, // nt:id-only unimplemented stub: the minigame id has no name table
                    "UNIMPLEMENTED: debugStartMinigame"
                );
            }
            true
        }
        DEBUG_SPECTATE => {
            if args.len() >= 4 {
                let game_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::info!(
                    entity_id,
                    entity_name = mgr.entity_label(entity_id),
                    game_id, // nt:id-only unimplemented stub: the minigame id has no name table
                    "UNIMPLEMENTED: debugSpectateMinigame"
                );
            }
            true
        }
        DEBUG_JOIN => {
            if args.len() >= 4 {
                let game_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::info!(
                    entity_id,
                    entity_name = mgr.entity_label(entity_id),
                    game_id, // nt:id-only unimplemented stub: the minigame id has no name table
                    "UNIMPLEMENTED: debugJoinMinigame"
                );
            }
            true
        }
        DEBUG_INSTANCE => {
            if args.len() >= 4 {
                let instance_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::info!(
                    entity_id,
                    entity_name = mgr.entity_label(entity_id),
                    instance_id, // nt:id-only unimplemented stub: a minigame instance has no name
                    "UNIMPLEMENTED: debugMinigameInstance"
                );
            }
            true
        }
        START => {
            if args.len() >= 8 {
                let host_entity_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let game_def_id = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                tracing::info!(
                    entity_id,
                    entity_name = mgr.entity_label(entity_id),
                    host_entity_id,
                    host_entity_name = wire_label(mgr, host_entity_id),
                    game_def_id, // nt:id-only unimplemented stub: minigame defs have no name table
                    "UNIMPLEMENTED: startMinigame"
                );
            }
            true
        }
        END_CURRENT => {
            if args.len() >= 12 {
                let game_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let winner_id = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                let loser_id = i32::from_le_bytes([args[8], args[9], args[10], args[11]]);
                tracing::info!(
                    entity_id,
                    entity_name = mgr.entity_label(entity_id),
                    game_id, // nt:id-only unimplemented stub: the minigame id has no name table
                    winner_id,
                    winner_name = wire_label(mgr, winner_id),
                    loser_id,
                    loser_name = wire_label(mgr, loser_id),
                    "UNIMPLEMENTED: endCurrentMinigame"
                );
            }
            true
        }
        REQUEST_SPECTATE_LIST => {
            if args.len() >= 4 {
                let game_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::info!(
                    entity_id,
                    entity_name = mgr.entity_label(entity_id),
                    game_id, // nt:id-only unimplemented stub: the minigame id has no name table
                    "UNIMPLEMENTED: requestSpectateList"
                );
            }
            true
        }
        SPECTATE => {
            if args.len() >= 4 {
                let game_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::info!(
                    entity_id,
                    entity_name = mgr.entity_label(entity_id),
                    game_id, // nt:id-only unimplemented stub: the minigame id has no name table
                    "UNIMPLEMENTED: spectateMinigame"
                );
            }
            true
        }
        REGISTER_HELP => {
            if args.len() >= 8 {
                let game_def_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let help_level = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                tracing::info!(
                    entity_id,
                    entity_name = mgr.entity_label(entity_id),
                    game_def_id, // nt:id-only unimplemented stub: minigame defs have no name table
                    help_level,
                    "UNIMPLEMENTED: registerToMinigameHelp"
                );
            }
            true
        }
        UPDATE_REGISTER_HELP => {
            if args.len() >= 8 {
                let game_def_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let help_level = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                tracing::info!(
                    entity_id,
                    entity_name = mgr.entity_label(entity_id),
                    game_def_id, // nt:id-only unimplemented stub: minigame defs have no name table
                    help_level,
                    "UNIMPLEMENTED: updateRegisterToMinigameHelp"
                );
            }
            true
        }
        START_CANCEL => {
            if args.len() >= 4 {
                let game_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::info!(
                    entity_id,
                    entity_name = mgr.entity_label(entity_id),
                    game_id, // nt:id-only unimplemented stub: the minigame id has no name table
                    "UNIMPLEMENTED: minigameStartCancel"
                );
            }
            true
        }
        CALL_ACCEPT => {
            if args.len() >= 4 {
                let game_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::info!(
                    entity_id,
                    entity_name = mgr.entity_label(entity_id),
                    game_id, // nt:id-only unimplemented stub: the minigame id has no name table
                    "UNIMPLEMENTED: minigameCallAccept"
                );
            }
            true
        }
        CALL_DECLINE => {
            if args.len() >= 4 {
                let game_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::info!(
                    entity_id,
                    entity_name = mgr.entity_label(entity_id),
                    game_id, // nt:id-only unimplemented stub: the minigame id has no name table
                    "UNIMPLEMENTED: minigameCallDecline"
                );
            }
            true
        }
        CALL_ABORT => {
            if args.len() >= 4 {
                let game_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::info!(
                    entity_id,
                    entity_name = mgr.entity_label(entity_id),
                    game_id, // nt:id-only unimplemented stub: the minigame id has no name table
                    "UNIMPLEMENTED: minigameCallAbort"
                );
            }
            true
        }
        CONTACT_REQUEST => {
            if args.len() >= 8 {
                let target_entity_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let game_def_id = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                tracing::info!(
                    entity_id,
                    entity_name = mgr.entity_label(entity_id),
                    target_entity_id,
                    target_entity_name = wire_label(mgr, target_entity_id),
                    game_def_id, // nt:id-only unimplemented stub: minigame defs have no name table
                    "UNIMPLEMENTED: minigameContactRequest"
                );
            }
            true
        }
        _ => false,
    }
}

/// The label of an entity ID the client sent as an `INT32`.
fn wire_label(mgr: &SpaceManager, entity_id: i32) -> Option<&str> {
    u32::try_from(entity_id)
        .ok()
        .and_then(|id| mgr.entity_label(id))
}

#[cfg(test)]
#[path = "minigame_names_tests.rs"]
mod names_tests;
