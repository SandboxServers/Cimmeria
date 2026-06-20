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
mod tests {
    use super::*;
    use crate::mercury::read_wstring;

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

    /// read_wstring round-trips an ASCII string correctly.
    #[test]
    fn parse_wstring_ascii_round_trip() {
        let encoded = wstring("Friends");
        let (s, consumed) = read_wstring(&encoded, 0).expect("parse");
        assert_eq!(s, "Friends");
        assert_eq!(consumed, encoded.len());
    }

    /// read_wstring at a non-zero offset works correctly.
    #[test]
    fn parse_wstring_at_offset() {
        let mut buf = 99i32.to_le_bytes().to_vec(); // list_id prefix
        buf.extend(wstring("Renamed"));
        let (name, _) = read_wstring(&buf, 4).expect("parse at offset 4");
        assert_eq!(name, "Renamed");
    }

    /// read_wstring returns Err for a truncated buffer (body shorter than declared).
    #[test]
    fn parse_wstring_truncated_returns_none() {
        let encoded = wstring("Hello");
        // Only the 4-byte count prefix, no body.
        assert!(read_wstring(&encoded[..4], 0).is_err());
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

        let (name, consumed) = read_wstring(&args, 0).unwrap();
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
        let (name, _) = read_wstring(&args, 4).unwrap();
        assert_eq!(name, "Renamed");
    }

    /// Wire-format test: CM 56 contactListDelete layout.
    /// `[i32 list_id]`
    #[test]
    fn delete_args_layout() {
        let args: Vec<u8> = 42i32.to_le_bytes().to_vec();
        assert!(args.len() >= 4);
        let list_id = i32::from_le_bytes(args[0..4].try_into().unwrap());
        assert_eq!(list_id, 42);
    }

    /// Wire-format test: CM 56 contactListDelete rejects buffers shorter than 4 bytes.
    #[test]
    fn delete_args_too_short_is_rejected() {
        let short: Vec<u8> = vec![1, 2, 3]; // only 3 bytes
        assert!(short.len() < 4);
    }

    /// Wire-format test: CM 58 contactListFlagsUpdate layout.
    /// `[i32 list_id][u32 flags]`
    #[test]
    fn flags_update_args_layout() {
        let mut args = 13i32.to_le_bytes().to_vec();
        args.extend_from_slice(&300u32.to_le_bytes()); // flags = 300 (Friends moniker)

        let list_id = i32::from_le_bytes(args[0..4].try_into().unwrap());
        assert_eq!(list_id, 13);
        let flags = u32::from_le_bytes(args[4..8].try_into().unwrap());
        assert_eq!(flags, 300);
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

    /// Wire-format test: CM 60 contactListRemoveMembers layout.
    /// Same layout as CM 59: `[i32 list_id][u32 count][WSTRING × count]`
    #[test]
    fn remove_members_args_layout() {
        let mut args = 8i32.to_le_bytes().to_vec();
        args.extend(wstring_array(&["Enemy"]));

        let list_id = i32::from_le_bytes(args[0..4].try_into().unwrap());
        assert_eq!(list_id, 8);
        let (names, _) = parse_wstring_array(&args, 4).unwrap();
        assert_eq!(names, vec!["Enemy".to_string()]);
    }

    /// parse_wstring_array with a malformed element (truncated body) returns None.
    #[test]
    fn parse_wstring_array_malformed_element_returns_none() {
        // count=1, then only 2 bytes of a WSTRING body (needs 4 + char_count*2 = 6).
        let mut buf = 1u32.to_le_bytes().to_vec(); // count = 1
        buf.extend_from_slice(&1u32.to_le_bytes()); // char_count = 1
        buf.push(0x41); // only 1 byte of UTF-16LE body — too short
        assert!(
            parse_wstring_array(&buf, 0).is_none(),
            "malformed WSTRING element must cause array parse to fail"
        );
    }

    /// The ≤100-name clamp in parse_wstring_array is a parse-level guard that
    /// a malicious client cannot bypass by sending exactly MAX+1 names.
    #[test]
    fn parse_wstring_array_clamps_at_exact_boundary() {
        let one = wstring("x");
        let total = MAX_MEMBERS_PER_REQUEST + 1;
        let mut buf = (total as u32).to_le_bytes().to_vec();
        for _ in 0..total {
            buf.extend_from_slice(&one);
        }
        let (names, _) = parse_wstring_array(&buf, 0).expect("should parse up to the clamp");
        assert_eq!(
            names.len(),
            MAX_MEMBERS_PER_REQUEST,
            "exactly MAX+1 names must be clamped to MAX"
        );
    }
}
