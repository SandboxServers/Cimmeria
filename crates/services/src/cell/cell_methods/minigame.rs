//! MinigamePlayer interface exposed CellMethods (indices 20–34).

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use tokio::sync::mpsc;

pub const DEBUG_START: u16 = 20;
pub const DEBUG_SPECTATE: u16 = 21;
pub const DEBUG_JOIN: u16 = 22;
pub const DEBUG_INSTANCE: u16 = 23;
pub const START: u16 = 24;
pub const END_CURRENT: u16 = 25;
pub const REQUEST_SPECTATE_LIST: u16 = 26;
pub const SPECTATE: u16 = 27;
pub const REGISTER_HELP: u16 = 28;
pub const UPDATE_REGISTER_HELP: u16 = 29;
pub const START_CANCEL: u16 = 30;
pub const CALL_ACCEPT: u16 = 31;
pub const CALL_DECLINE: u16 = 32;
pub const CALL_ABORT: u16 = 33;
pub const CONTACT_REQUEST: u16 = 34;

pub async fn dispatch(
    entity_id: u32,
    method_index: u16,
    args: &[u8],
    _tx: &mpsc::Sender<CellToBaseMsg>,
    _space_mgr: &mut SpaceManager,
) -> bool {
    match method_index {
        DEBUG_START => {
            if args.len() >= 4 {
                let game_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::info!(entity_id, game_id, "UNIMPLEMENTED: debugStartMinigame");
            }
            true
        }
        DEBUG_SPECTATE => {
            if args.len() >= 4 {
                let game_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::info!(entity_id, game_id, "UNIMPLEMENTED: debugSpectateMinigame");
            }
            true
        }
        DEBUG_JOIN => {
            if args.len() >= 4 {
                let game_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::info!(entity_id, game_id, "UNIMPLEMENTED: debugJoinMinigame");
            }
            true
        }
        DEBUG_INSTANCE => {
            if args.len() >= 4 {
                let instance_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::info!(
                    entity_id,
                    instance_id,
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
                    host_entity_id,
                    game_def_id,
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
                    game_id,
                    winner_id,
                    loser_id,
                    "UNIMPLEMENTED: endCurrentMinigame"
                );
            }
            true
        }
        REQUEST_SPECTATE_LIST => {
            if args.len() >= 4 {
                let game_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::info!(entity_id, game_id, "UNIMPLEMENTED: requestSpectateList");
            }
            true
        }
        SPECTATE => {
            if args.len() >= 4 {
                let game_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::info!(entity_id, game_id, "UNIMPLEMENTED: spectateMinigame");
            }
            true
        }
        REGISTER_HELP => {
            if args.len() >= 8 {
                let game_def_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let help_level = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                tracing::info!(
                    entity_id,
                    game_def_id,
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
                    game_def_id,
                    help_level,
                    "UNIMPLEMENTED: updateRegisterToMinigameHelp"
                );
            }
            true
        }
        START_CANCEL => {
            if args.len() >= 4 {
                let game_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::info!(entity_id, game_id, "UNIMPLEMENTED: minigameStartCancel");
            }
            true
        }
        CALL_ACCEPT => {
            if args.len() >= 4 {
                let game_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::info!(entity_id, game_id, "UNIMPLEMENTED: minigameCallAccept");
            }
            true
        }
        CALL_DECLINE => {
            if args.len() >= 4 {
                let game_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::info!(entity_id, game_id, "UNIMPLEMENTED: minigameCallDecline");
            }
            true
        }
        CALL_ABORT => {
            if args.len() >= 4 {
                let game_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::info!(entity_id, game_id, "UNIMPLEMENTED: minigameCallAbort");
            }
            true
        }
        CONTACT_REQUEST => {
            if args.len() >= 8 {
                let target_entity_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let game_def_id = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                tracing::info!(
                    entity_id,
                    target_entity_id,
                    game_def_id,
                    "UNIMPLEMENTED: minigameContactRequest"
                );
            }
            true
        }
        _ => false,
    }
}
