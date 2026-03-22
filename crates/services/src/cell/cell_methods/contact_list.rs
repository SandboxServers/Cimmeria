//! ContactListManager interface exposed CellMethods (indices 55–60).

use tokio::sync::mpsc;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

pub const CREATE: u16 = 55;
pub const DELETE: u16 = 56;
pub const RENAME: u16 = 57;
pub const FLAGS_UPDATE: u16 = 58;
pub const ADD_MEMBERS: u16 = 59;
pub const REMOVE_MEMBERS: u16 = 60;

pub async fn dispatch(
    entity_id: u32,
    method_index: u16,
    args: &[u8],
    _tx: &mpsc::Sender<CellToBaseMsg>,
    _space_mgr: &mut SpaceManager,
) -> bool {
    match method_index {
        CREATE => {
            // WSTRING listName, UINT32 flags -- variable length, just log
            tracing::info!(entity_id, "UNIMPLEMENTED: contactListCreate");
            true
        }
        DELETE => {
            if args.len() >= 4 {
                let list_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::info!(entity_id, list_id, "UNIMPLEMENTED: contactListDelete");
            }
            true
        }
        RENAME => {
            if args.len() >= 4 {
                let list_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::info!(entity_id, list_id, "UNIMPLEMENTED: contactListRename");
            }
            true
        }
        FLAGS_UPDATE => {
            if args.len() >= 8 {
                let list_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let flags = u32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                tracing::info!(entity_id, list_id, flags, "UNIMPLEMENTED: contactListFlagsUpdate");
            }
            true
        }
        ADD_MEMBERS => {
            if args.len() >= 4 {
                let list_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::info!(entity_id, list_id, "UNIMPLEMENTED: contactListAddMembers");
            }
            true
        }
        REMOVE_MEMBERS => {
            if args.len() >= 4 {
                let list_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::info!(entity_id, list_id, "UNIMPLEMENTED: contactListRemoveMembers");
            }
            true
        }
        _ => false,
    }
}
