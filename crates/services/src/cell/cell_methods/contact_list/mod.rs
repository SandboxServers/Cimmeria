//! ContactListManager interface exposed CellMethods (indices 55–60).
//!
//! Wire formats (C→S, confirmed against wire_log/decoders/generated.rs):
//! - contactListCreate     (55): WSTRING name, UINT32 flags
//! - contactListDelete     (56): INT32 listId
//! - contactListRename     (57): INT32 listId, WSTRING name
//! - contactListFlagsUpdate (58): INT32 listId, UINT32 flags
//! - contactListAddMembers  (59): INT32 listId, ARRAY<WSTRING> names
//! - contactListRemoveMembers (60): INT32 listId, ARRAY<WSTRING> names
//!
//! All handlers resolve `player_id` from `space_mgr`, then forward to base
//! via `CellToBaseMsg` variants. The base owns all DB mutations and client
//! echo responses.

use crate::base::contact_list::wire::MAX_MEMBERS_PER_REQUEST;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use crate::mercury::read_wstring;
use tokio::sync::mpsc;

pub const CREATE: u16 = 55;
pub const DELETE: u16 = 56;
pub const RENAME: u16 = 57;
pub const FLAGS_UPDATE: u16 = 58;
pub const ADD_MEMBERS: u16 = 59;
pub const REMOVE_MEMBERS: u16 = 60;

// ── Wire parse helpers ────────────────────────────────────────────────────────

/// Parse an ARRAY<WSTRING> from a byte slice at `offset`.
///
/// Wire: `[u32 count LE][WSTRING × count]`
/// Returns `Some((names, bytes_consumed))` or `None` if malformed.
/// Clamps count to `MAX_MEMBERS_PER_REQUEST` to mirror the base-side guard.
///
/// Individual WSTRINGs are decoded via `crate::mercury::read_wstring`.
fn parse_wstring_array(args: &[u8], offset: usize) -> Option<(Vec<String>, usize)> {
    if args.len() < offset + 4 {
        return None;
    }
    let raw_count = u32::from_le_bytes([
        args[offset],
        args[offset + 1],
        args[offset + 2],
        args[offset + 3],
    ]) as usize;
    let count = raw_count.min(MAX_MEMBERS_PER_REQUEST);
    let mut names = Vec::with_capacity(count);
    let mut pos = offset + 4;
    for _ in 0..count {
        let (name, consumed) = read_wstring(args, pos).ok()?;
        names.push(name);
        pos += consumed;
    }
    Some((names, pos - offset))
}

/// Resolve player_id for a contact-list op, refusing to fall back to 0.
/// Mirrors the pattern from `cell/mail.rs::resolve_mail_player_id`.
fn resolve_player_id(entity_id: u32, space_mgr: &SpaceManager, op: &str) -> Option<i32> {
    match space_mgr.get_entity(entity_id).and_then(|e| e.player_id) {
        Some(id) => Some(id),
        None => {
            tracing::warn!(
                entity_id,
                op,
                "contact list op dropped: entity has no player_id"
            );
            None
        }
    }
}

// ── Dispatch ──────────────────────────────────────────────────────────────────

pub async fn dispatch(
    entity_id: u32,
    method_index: u16,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
    match method_index {
        CREATE => {
            // WSTRING name, UINT32 flags
            let Some((name, consumed)) = read_wstring(args, 0).ok() else {
                tracing::warn!(entity_id, "contactListCreate: malformed WSTRING name");
                return true;
            };
            if args.len() < consumed + 4 {
                tracing::warn!(entity_id, "contactListCreate: missing flags field");
                return true;
            }
            let flags = u32::from_le_bytes([
                args[consumed],
                args[consumed + 1],
                args[consumed + 2],
                args[consumed + 3],
            ]);
            let Some(player_id) = resolve_player_id(entity_id, space_mgr, "contactListCreate")
            else {
                return true;
            };
            tracing::debug!(entity_id, player_id, name, flags, "contactListCreate");
            if let Err(e) = tx
                .send(CellToBaseMsg::ContactListCreate {
                    entity_id,
                    player_id,
                    name,
                    flags,
                })
                .await
            {
                tracing::warn!(
                    entity_id,
                    player_id,
                    error = %e,
                    "contactListCreate send to base failed — mutation dropped"
                );
            }
            true
        }

        DELETE => {
            if args.len() < 4 {
                return true;
            }
            let list_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
            let Some(player_id) = resolve_player_id(entity_id, space_mgr, "contactListDelete")
            else {
                return true;
            };
            tracing::debug!(entity_id, player_id, list_id, "contactListDelete");
            if let Err(e) = tx
                .send(CellToBaseMsg::ContactListDelete {
                    entity_id,
                    player_id,
                    list_id,
                })
                .await
            {
                tracing::warn!(
                    entity_id,
                    player_id,
                    list_id,
                    error = %e,
                    "contactListDelete send to base failed — mutation dropped"
                );
            }
            true
        }

        RENAME => {
            if args.len() < 4 {
                return true;
            }
            let list_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
            let Some((name, _)) = read_wstring(args, 4).ok() else {
                tracing::warn!(
                    entity_id,
                    list_id,
                    "contactListRename: malformed name WSTRING"
                );
                return true;
            };
            let Some(player_id) = resolve_player_id(entity_id, space_mgr, "contactListRename")
            else {
                return true;
            };
            tracing::debug!(entity_id, player_id, list_id, name, "contactListRename");
            if let Err(e) = tx
                .send(CellToBaseMsg::ContactListRename {
                    entity_id,
                    player_id,
                    list_id,
                    name,
                })
                .await
            {
                tracing::warn!(
                    entity_id,
                    player_id,
                    list_id,
                    error = %e,
                    "contactListRename send to base failed — mutation dropped"
                );
            }
            true
        }

        FLAGS_UPDATE => {
            if args.len() < 8 {
                return true;
            }
            let list_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
            let flags = u32::from_le_bytes([args[4], args[5], args[6], args[7]]);
            let Some(player_id) = resolve_player_id(entity_id, space_mgr, "contactListFlagsUpdate")
            else {
                return true;
            };
            tracing::debug!(
                entity_id,
                player_id,
                list_id,
                flags,
                "contactListFlagsUpdate"
            );
            if let Err(e) = tx
                .send(CellToBaseMsg::ContactListFlagsUpdate {
                    entity_id,
                    player_id,
                    list_id,
                    flags,
                })
                .await
            {
                tracing::warn!(
                    entity_id,
                    player_id,
                    list_id,
                    error = %e,
                    "contactListFlagsUpdate send to base failed — mutation dropped"
                );
            }
            true
        }

        ADD_MEMBERS => {
            if args.len() < 4 {
                return true;
            }
            let list_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
            let Some((names, _)) = parse_wstring_array(args, 4) else {
                tracing::warn!(
                    entity_id,
                    list_id,
                    "contactListAddMembers: malformed names array"
                );
                return true;
            };
            let Some(player_id) = resolve_player_id(entity_id, space_mgr, "contactListAddMembers")
            else {
                return true;
            };
            tracing::debug!(
                entity_id,
                player_id,
                list_id,
                count = names.len(),
                "contactListAddMembers"
            );
            if let Err(e) = tx
                .send(CellToBaseMsg::ContactListAddMembers {
                    entity_id,
                    player_id,
                    list_id,
                    names,
                })
                .await
            {
                tracing::warn!(
                    entity_id,
                    player_id,
                    list_id,
                    error = %e,
                    "contactListAddMembers send to base failed — mutation dropped"
                );
            }
            true
        }

        REMOVE_MEMBERS => {
            if args.len() < 4 {
                return true;
            }
            let list_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
            let Some((names, _)) = parse_wstring_array(args, 4) else {
                tracing::warn!(
                    entity_id,
                    list_id,
                    "contactListRemoveMembers: malformed names array"
                );
                return true;
            };
            let Some(player_id) =
                resolve_player_id(entity_id, space_mgr, "contactListRemoveMembers")
            else {
                return true;
            };
            tracing::debug!(
                entity_id,
                player_id,
                list_id,
                count = names.len(),
                "contactListRemoveMembers"
            );
            if let Err(e) = tx
                .send(CellToBaseMsg::ContactListRemoveMembers {
                    entity_id,
                    player_id,
                    list_id,
                    names,
                })
                .await
            {
                tracing::warn!(
                    entity_id,
                    player_id,
                    list_id,
                    error = %e,
                    "contactListRemoveMembers send to base failed — mutation dropped"
                );
            }
            true
        }

        _ => false,
    }
}

#[cfg(test)]
mod tests;
