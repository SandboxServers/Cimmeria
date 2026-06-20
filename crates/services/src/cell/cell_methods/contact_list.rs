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

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use tokio::sync::mpsc;

pub const CREATE: u16 = 55;
pub const DELETE: u16 = 56;
pub const RENAME: u16 = 57;
pub const FLAGS_UPDATE: u16 = 58;
pub const ADD_MEMBERS: u16 = 59;
pub const REMOVE_MEMBERS: u16 = 60;

/// Hard limit on names per ADD/REMOVE request. Mirrors the base-side clamp.
const MAX_MEMBERS_PER_REQUEST: usize = 100;

// ── Wire parse helpers ────────────────────────────────────────────────────────

/// Parse a WSTRING from a byte slice at `offset`.
///
/// Wire: `[u32 char_count LE][UTF-16LE × char_count]`
/// Returns `Some((string, bytes_consumed))` or `None` if the buffer is too short
/// or contains invalid UTF-16.
fn parse_wstring(args: &[u8], offset: usize) -> Option<(String, usize)> {
    if args.len() < offset + 4 {
        return None;
    }
    let char_count = u32::from_le_bytes([
        args[offset],
        args[offset + 1],
        args[offset + 2],
        args[offset + 3],
    ]) as usize;
    let bytes_needed = 4 + char_count * 2;
    if args.len() < offset + bytes_needed {
        return None;
    }
    let mut units = Vec::with_capacity(char_count);
    for i in 0..char_count {
        let pos = offset + 4 + i * 2;
        units.push(u16::from_le_bytes([args[pos], args[pos + 1]]));
    }
    let s = String::from_utf16(&units).ok()?;
    Some((s, bytes_needed))
}

/// Parse an ARRAY<WSTRING> from a byte slice at `offset`.
///
/// Wire: `[u32 count LE][WSTRING × count]`
/// Returns `Some((names, bytes_consumed))` or `None` if malformed.
/// Clamps count to `MAX_MEMBERS_PER_REQUEST` to mirror the base-side guard.
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
        let (name, consumed) = parse_wstring(args, pos)?;
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
            let Some((name, consumed)) = parse_wstring(args, 0) else {
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
            let _ = tx
                .send(CellToBaseMsg::ContactListCreate {
                    entity_id,
                    player_id,
                    name,
                    flags,
                })
                .await;
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
            let _ = tx
                .send(CellToBaseMsg::ContactListDelete {
                    entity_id,
                    player_id,
                    list_id,
                })
                .await;
            true
        }

        RENAME => {
            if args.len() < 4 {
                return true;
            }
            let list_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
            let Some((name, _)) = parse_wstring(args, 4) else {
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
            let _ = tx
                .send(CellToBaseMsg::ContactListRename {
                    entity_id,
                    player_id,
                    list_id,
                    name,
                })
                .await;
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
            let _ = tx
                .send(CellToBaseMsg::ContactListFlagsUpdate {
                    entity_id,
                    player_id,
                    list_id,
                    flags,
                })
                .await;
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
            let _ = tx
                .send(CellToBaseMsg::ContactListAddMembers {
                    entity_id,
                    player_id,
                    list_id,
                    names,
                })
                .await;
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
            let _ = tx
                .send(CellToBaseMsg::ContactListRemoveMembers {
                    entity_id,
                    player_id,
                    list_id,
                    names,
                })
                .await;
            true
        }

        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper: encode a WSTRING.
    fn wstring(s: &str) -> Vec<u8> {
        let chars: Vec<u16> = s.encode_utf16().collect();
        let mut buf = Vec::with_capacity(4 + chars.len() * 2);
        buf.extend_from_slice(&(chars.len() as u32).to_le_bytes());
        for ch in chars {
            buf.extend_from_slice(&ch.to_le_bytes());
        }
        buf
    }

    // Helper: encode an ARRAY<WSTRING>.
    fn wstring_array(names: &[&str]) -> Vec<u8> {
        let mut buf = (names.len() as u32).to_le_bytes().to_vec();
        for s in names {
            buf.extend(wstring(s));
        }
        buf
    }

    /// parse_wstring round-trips an ASCII string correctly.
    #[test]
    fn parse_wstring_ascii_round_trip() {
        let encoded = wstring("Friends");
        let (s, consumed) = parse_wstring(&encoded, 0).expect("parse");
        assert_eq!(s, "Friends");
        assert_eq!(consumed, encoded.len());
    }

    /// parse_wstring at a non-zero offset works correctly.
    #[test]
    fn parse_wstring_at_offset() {
        let mut buf = 99i32.to_le_bytes().to_vec(); // list_id prefix
        buf.extend(wstring("Renamed"));
        let (name, _) = parse_wstring(&buf, 4).expect("parse at offset 4");
        assert_eq!(name, "Renamed");
    }

    /// parse_wstring returns None for a truncated buffer (body shorter than declared).
    #[test]
    fn parse_wstring_truncated_returns_none() {
        let encoded = wstring("Hello");
        // Only the 4-byte count prefix, no body.
        assert!(parse_wstring(&encoded[..4], 0).is_none());
    }

    /// parse_wstring_array round-trips two names.
    #[test]
    fn parse_wstring_array_two_names() {
        let encoded = wstring_array(&["Friendly", "Sheppard"]);
        let (names, consumed) = parse_wstring_array(&encoded, 0).expect("parse");
        assert_eq!(names, vec!["Friendly".to_string(), "Sheppard".to_string()]);
        assert_eq!(consumed, encoded.len());
    }

    /// parse_wstring_array with zero elements is valid.
    #[test]
    fn parse_wstring_array_empty() {
        let encoded = wstring_array(&[]);
        let (names, consumed) = parse_wstring_array(&encoded, 0).expect("parse");
        assert!(names.is_empty());
        assert_eq!(consumed, 4); // just the count u32
    }

    /// parse_wstring_array clamps to MAX_MEMBERS_PER_REQUEST without panicking.
    #[test]
    fn parse_wstring_array_clamps_to_max() {
        let one = wstring("x");
        let total = MAX_MEMBERS_PER_REQUEST + 5;
        let mut buf = (total as u32).to_le_bytes().to_vec();
        for _ in 0..total {
            buf.extend_from_slice(&one);
        }
        let (names, _) = parse_wstring_array(&buf, 0).expect("parse with clamped count");
        assert_eq!(
            names.len(),
            MAX_MEMBERS_PER_REQUEST,
            "parser must clamp to MAX_MEMBERS_PER_REQUEST"
        );
    }

    /// Wire-format test: CM 55 contactListCreate layout.
    /// `[WSTRING name][u32 flags]`
    #[test]
    fn create_args_layout() {
        let mut args = wstring("MyList");
        args.extend_from_slice(&42u32.to_le_bytes()); // flags

        let (name, consumed) = parse_wstring(&args, 0).unwrap();
        assert_eq!(name, "MyList");
        let flags = u32::from_le_bytes(args[consumed..consumed + 4].try_into().unwrap());
        assert_eq!(flags, 42);
    }

    /// Wire-format test: CM 57 contactListRename layout.
    /// `[i32 list_id][WSTRING name]`
    #[test]
    fn rename_args_layout() {
        let mut args = 99i32.to_le_bytes().to_vec();
        args.extend(wstring("Renamed"));

        let list_id = i32::from_le_bytes(args[0..4].try_into().unwrap());
        assert_eq!(list_id, 99);
        let (name, _) = parse_wstring(&args, 4).unwrap();
        assert_eq!(name, "Renamed");
    }

    /// Wire-format test: CM 59 contactListAddMembers layout.
    /// `[i32 list_id][u32 count][WSTRING × count]`
    #[test]
    fn add_members_args_layout() {
        let mut args = 7i32.to_le_bytes().to_vec();
        args.extend(wstring_array(&["Alice", "Bob"]));

        let list_id = i32::from_le_bytes(args[0..4].try_into().unwrap());
        assert_eq!(list_id, 7);
        let (names, _) = parse_wstring_array(&args, 4).unwrap();
        assert_eq!(names, vec!["Alice".to_string(), "Bob".to_string()]);
    }
}
