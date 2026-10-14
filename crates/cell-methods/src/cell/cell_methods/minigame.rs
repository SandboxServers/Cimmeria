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
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
    // Rare client calls: every call logs once with its `args_len`. Each arm
    // reads only the arguments `MinigamePlayer.def` declares for that method.
    let mgr = &*space_mgr;
    let args_len = args.len();
    match method_index {
        DEBUG_START => {
            tracing::info!(
                entity_id,
                entity_name = mgr.entity_label(entity_id),
                args_len,
                game_id = int32_arg(args, 0), // nt:id-only unimplemented stub: the minigame id has no name table
                "UNIMPLEMENTED: debugStartMinigame"
            );
            true
        }
        DEBUG_SPECTATE => {
            tracing::info!(
                entity_id,
                entity_name = mgr.entity_label(entity_id),
                args_len,
                "UNIMPLEMENTED: debugSpectateMinigame"
            );
            true
        }
        DEBUG_JOIN => {
            tracing::info!(
                entity_id,
                entity_name = mgr.entity_label(entity_id),
                args_len,
                "UNIMPLEMENTED: debugJoinMinigame"
            );
            true
        }
        DEBUG_INSTANCE => {
            tracing::info!(
                entity_id,
                entity_name = mgr.entity_label(entity_id),
                args_len,
                instance_id = int32_arg(args, 0), // nt:id-only unimplemented stub: a minigame instance has no name
                seed = int32_arg(args, 1),
                tech_comp = int32_arg(args, 2),
                "UNIMPLEMENTED: debugMinigameInstance"
            );
            true
        }
        START => {
            tracing::info!(
                entity_id,
                entity_name = mgr.entity_label(entity_id),
                args_len,
                "UNIMPLEMENTED: startMinigame"
            );
            true
        }
        END_CURRENT => {
            if let Err(error) = tx.send(CellToBaseMsg::CancelMinigame { entity_id }).await {
                tracing::error!(entity_id, entity_name = mgr.entity_label(entity_id), args_len,
                    %error, reason = "base_channel_closed", "Minigame cancel lost");
            } else {
                tracing::info!(
                    entity_id,
                    entity_name = mgr.entity_label(entity_id),
                    args_len,
                    reason = "player_closed_window",
                    "Minigame cancel requested"
                );
            }
            true
        }
        REQUEST_SPECTATE_LIST => {
            tracing::info!(
                entity_id,
                entity_name = mgr.entity_label(entity_id),
                args_len,
                "UNIMPLEMENTED: requestSpectateList"
            );
            true
        }
        SPECTATE => {
            let player_id = int32_arg(args, 0);
            tracing::info!(
                entity_id,
                entity_name = mgr.entity_label(entity_id),
                args_len,
                target_player_id = player_id,
                target_player_name = player_id.and_then(|id| wire_label(mgr, id)),
                "UNIMPLEMENTED: spectateMinigame"
            );
            true
        }
        REGISTER_HELP => {
            tracing::info!(
                entity_id,
                entity_name = mgr.entity_label(entity_id),
                args_len,
                "UNIMPLEMENTED: registerToMinigameHelp"
            );
            true
        }
        UPDATE_REGISTER_HELP => {
            tracing::info!(
                entity_id,
                entity_name = mgr.entity_label(entity_id),
                args_len,
                "UNIMPLEMENTED: updateRegisterToMinigameHelp"
            );
            true
        }
        START_CANCEL => {
            tracing::info!(
                entity_id,
                entity_name = mgr.entity_label(entity_id),
                args_len,
                "UNIMPLEMENTED: minigameStartCancel"
            );
            true
        }
        CALL_ACCEPT => {
            let calling_player_id = int32_arg(args, 0);
            tracing::info!(
                entity_id,
                entity_name = mgr.entity_label(entity_id),
                args_len,
                calling_player_id,
                calling_player_name = calling_player_id.and_then(|id| wire_label(mgr, id)),
                "UNIMPLEMENTED: minigameCallAccept"
            );
            true
        }
        CALL_DECLINE => {
            let calling_player_id = int32_arg(args, 0);
            tracing::info!(
                entity_id,
                entity_name = mgr.entity_label(entity_id),
                args_len,
                calling_player_id,
                calling_player_name = calling_player_id.and_then(|id| wire_label(mgr, id)),
                "UNIMPLEMENTED: minigameCallDecline"
            );
            true
        }
        CALL_ABORT => {
            tracing::info!(
                entity_id,
                entity_name = mgr.entity_label(entity_id),
                args_len,
                "UNIMPLEMENTED: minigameCallAbort"
            );
            true
        }
        CONTACT_REQUEST => {
            let contact_id = int32_arg(args, 0);
            tracing::info!(
                entity_id,
                entity_name = mgr.entity_label(entity_id),
                args_len,
                contact_id,
                contact_name = contact_id.and_then(|id| wire_label(mgr, id)),
                "UNIMPLEMENTED: minigameContactRequest"
            );
            true
        }
        _ => false,
    }
}

/// The `index`th little-endian `INT32` argument, or `None` when the payload
/// is too short to hold it.
fn int32_arg(args: &[u8], index: usize) -> Option<i32> {
    let at = index * 4;
    let bytes: [u8; 4] = args.get(at..at + 4)?.try_into().ok()?;
    Some(i32::from_le_bytes(bytes))
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
