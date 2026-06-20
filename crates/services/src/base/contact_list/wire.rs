//! Wire serializers for server→client contact-list methods (CM 85–89).
//!
//! Wire formats confirmed against the auto-generated decoders in
//! `crates/services/src/wire_log/decoders/generated.rs` (methods 85–89):
//!
//! - `onContactListUpdate`      (85): `[i32 list_id][WSTRING name][u32 flags]`
//! - `onContactListDelete`      (86): `[i32 list_id]`
//! - `onContactListAddMembers`  (87): `[i32 list_id][u32 count][WSTRING × count]`
//! - `onContactListRemoveMembers` (88): same layout as 87
//! - `onContactListEvent`       (89): `[WSTRING player_name][u32 event_id][i32 data_value]`
//!
//! WSTRING = 4-byte LE char count + N × 2-byte UTF-16LE code units.
//! ARRAY   = 4-byte LE element count + N WSTRINGs.
//!
//! All functions return a `Vec<u8>` `args` payload suitable for passing to
//! `crate::mercury::build_player_entity_method_packet`.

use crate::mercury::write_wstring;

/// Serialize `onContactListUpdate` (CM 85) args.
///
/// Sent to tell the client about a list header (id, name, flags). Used both
/// for the login-time push and as the echo after create/rename/flags_update.
pub(crate) fn build_on_contact_list_update(list_id: i32, name: &str, flags: u32) -> Vec<u8> {
    let mut buf = Vec::with_capacity(4 + 4 + name.len() * 2 + 4);
    buf.extend_from_slice(&list_id.to_le_bytes());
    write_wstring(&mut buf, name);
    buf.extend_from_slice(&flags.to_le_bytes());
    buf
}

/// Serialize `onContactListDelete` (CM 86) args.
pub(crate) fn build_on_contact_list_delete(list_id: i32) -> Vec<u8> {
    list_id.to_le_bytes().to_vec()
}

/// Serialize `onContactListAddMembers` (CM 87) args.
///
/// Also used for the login-time member push. `names` may be empty — in that
/// case the wire encodes `[list_id][0u32]` (an empty array), which is valid.
pub(crate) fn build_on_contact_list_add_members(list_id: i32, names: &[String]) -> Vec<u8> {
    let mut buf = Vec::with_capacity(4 + 4 + names.len() * 16);
    buf.extend_from_slice(&list_id.to_le_bytes());
    buf.extend_from_slice(&(names.len() as u32).to_le_bytes());
    for name in names {
        write_wstring(&mut buf, name);
    }
    buf
}

/// Serialize `onContactListRemoveMembers` (CM 88) args.
///
/// Wire layout is identical to CM 87.
pub(crate) fn build_on_contact_list_remove_members(list_id: i32, names: &[String]) -> Vec<u8> {
    build_on_contact_list_add_members(list_id, names)
}

/// Serialize `onContactListEvent` (CM 89) args.
///
/// `event_id` is the `EContactListEvent` **bitfield** value (UINT32), per
/// `entities/defs/enumerations.xml` (mirrored in `db/resources/Social/Types/
/// EContactListEvent.sql` and the original `Atrea/enums.py`):
///   1 = LoggedInStatus (data_value 1=online / 0=offline)
///   2 = GainLevel (data_value = new level), 4 = Death, 8 = GateTravel
/// NOTE: these are power-of-two bit flags, NOT a 0-based index — the client tests
/// the flag value, so sending 0 for LoggedInStatus never matches. Death/GateTravel
/// data_value semantics are still unconfirmed (UI-asset RE; not in the binary).
pub(crate) fn build_on_contact_list_event(
    player_name: &str,
    event_id: u32,
    data_value: i32,
) -> Vec<u8> {
    let mut buf = Vec::with_capacity(4 + player_name.len() * 2 + 4 + 4);
    write_wstring(&mut buf, player_name);
    buf.extend_from_slice(&event_id.to_le_bytes());
    buf.extend_from_slice(&data_value.to_le_bytes());
    buf
}

// ── Protocol constants ───────────────────────────────────────────────────────

/// Hard limit on names per ADD/REMOVE request (C→S and B→B).
///
/// Referenced by both the cell-side wire parser (`cell_methods/contact_list.rs`)
/// and the base-side handler (`handlers/crud.rs`). Defined here — the
/// contact-list wire module — so both layers share a single definition.
pub(crate) const MAX_MEMBERS_PER_REQUEST: usize = 100;

// ── EContactListEvent bitfield values (UINT32) ───────────────────────────────
// Power-of-two flags from entities/defs/enumerations.xml — NOT a 0-based index.

/// `EContactListEvent::ECONTACT_LIST_EVENT_LoggedInStatus` (bit 0).
pub(crate) const EVENT_LOGGED_IN_STATUS: u32 = 1;
// Defined now for protocol completeness; not yet emitted (the GainLevel/Death/
// GateTravel presence events are deferred — see #572). allow(dead_code) until wired.
/// `ECONTACT_LIST_EVENT_GainLevel` (bit 1). data_value = the player's new level.
#[allow(dead_code)]
pub(crate) const EVENT_GAIN_LEVEL: u32 = 2;
/// `ECONTACT_LIST_EVENT_Death` (bit 2). data_value semantics unconfirmed.
#[allow(dead_code)]
pub(crate) const EVENT_DEATH: u32 = 4;
/// `ECONTACT_LIST_EVENT_GateTravel` (bit 3). data_value semantics unconfirmed.
#[allow(dead_code)]
pub(crate) const EVENT_GATE_TRAVEL: u32 = 8;

/// `dataValue` for LoggedInStatus: player came online.
pub(crate) const DATA_ONLINE: i32 = 1;
/// `dataValue` for LoggedInStatus: player went offline.
pub(crate) const DATA_OFFLINE: i32 = 0;

#[cfg(test)]
mod tests {
    use super::*;

    // Helper: decode a WSTRING from a byte slice, returning (string, bytes_consumed).
    fn read_wstring(buf: &[u8]) -> (String, usize) {
        let count = u32::from_le_bytes(buf[0..4].try_into().unwrap()) as usize;
        let mut chars = Vec::with_capacity(count);
        for i in 0..count {
            let off = 4 + i * 2;
            chars.push(u16::from_le_bytes(buf[off..off + 2].try_into().unwrap()));
        }
        (String::from_utf16(&chars).unwrap(), 4 + count * 2)
    }

    // Helper: decode an ARRAY<WSTRING> from a byte slice, returning (names, bytes_consumed).
    fn read_wstring_array(buf: &[u8]) -> (Vec<String>, usize) {
        let count = u32::from_le_bytes(buf[0..4].try_into().unwrap()) as usize;
        let mut names = Vec::with_capacity(count);
        let mut off = 4usize;
        for _ in 0..count {
            let (s, n) = read_wstring(&buf[off..]);
            names.push(s);
            off += n;
        }
        (names, off)
    }

    /// Wire-format test: CM 85 `onContactListUpdate`.
    /// Byte layout: [i32 list_id LE][u32 char_count LE][UTF-16LE chars][u32 flags LE]
    #[test]
    fn on_contact_list_update_byte_layout() {
        let payload = build_on_contact_list_update(42, "Friends", 300);

        // list_id
        let list_id = i32::from_le_bytes(payload[0..4].try_into().unwrap());
        assert_eq!(list_id, 42);

        // name WSTRING
        let (name, consumed) = read_wstring(&payload[4..]);
        assert_eq!(name, "Friends");

        // flags
        let flags_off = 4 + consumed;
        let flags = u32::from_le_bytes(payload[flags_off..flags_off + 4].try_into().unwrap());
        assert_eq!(flags, 300);

        // total length: 4 + (4 + 7*2) + 4 = 4 + 18 + 4 = 26
        assert_eq!(payload.len(), flags_off + 4);
    }

    /// Wire-format test: CM 86 `onContactListDelete`.
    /// Byte layout: [i32 list_id LE]
    #[test]
    fn on_contact_list_delete_byte_layout() {
        let payload = build_on_contact_list_delete(7);
        assert_eq!(payload.len(), 4);
        assert_eq!(i32::from_le_bytes(payload[0..4].try_into().unwrap()), 7);
    }

    /// Wire-format test: CM 87 `onContactListAddMembers`.
    /// Byte layout: [i32 list_id LE][u32 count LE][WSTRING × count]
    #[test]
    fn on_contact_list_add_members_byte_layout() {
        let names = vec!["Friendly".to_string(), "Sheppard".to_string()];
        let payload = build_on_contact_list_add_members(3, &names);

        let list_id = i32::from_le_bytes(payload[0..4].try_into().unwrap());
        assert_eq!(list_id, 3);

        let (decoded, _) = read_wstring_array(&payload[4..]);
        assert_eq!(decoded, names);
    }

    /// Wire-format test: CM 87 with empty member list.
    #[test]
    fn on_contact_list_add_members_empty() {
        let payload = build_on_contact_list_add_members(5, &[]);
        assert_eq!(payload.len(), 8); // 4 (list_id) + 4 (count=0)
        let count = u32::from_le_bytes(payload[4..8].try_into().unwrap());
        assert_eq!(count, 0);
    }

    /// Wire-format test: CM 88 `onContactListRemoveMembers`.
    /// Layout is symmetric with CM 87.
    #[test]
    fn on_contact_list_remove_members_byte_layout() {
        let names = vec!["Annoying".to_string()];
        let payload = build_on_contact_list_remove_members(8, &names);

        let list_id = i32::from_le_bytes(payload[0..4].try_into().unwrap());
        assert_eq!(list_id, 8);

        let (decoded, _) = read_wstring_array(&payload[4..]);
        assert_eq!(decoded, names);
    }

    /// Wire-format test: CM 89 `onContactListEvent`.
    /// Byte layout: [WSTRING player_name][u32 event_id LE][i32 data_value LE]
    #[test]
    fn on_contact_list_event_byte_layout() {
        let payload = build_on_contact_list_event("Friendly", EVENT_LOGGED_IN_STATUS, DATA_ONLINE);

        let (player_name, consumed) = read_wstring(&payload[0..]);
        assert_eq!(player_name, "Friendly");

        let event_id = u32::from_le_bytes(payload[consumed..consumed + 4].try_into().unwrap());
        assert_eq!(event_id, EVENT_LOGGED_IN_STATUS);
        // Pin the literal wire value: LoggedInStatus is the bitfield flag 1, NOT 0.
        // (A self-referential `== EVENT_LOGGED_IN_STATUS` check let the 0 bug ship.)
        assert_eq!(
            event_id, 1,
            "LoggedInStatus must serialize as bitfield value 1"
        );

        let data_value =
            i32::from_le_bytes(payload[consumed + 4..consumed + 8].try_into().unwrap());
        assert_eq!(data_value, DATA_ONLINE);

        // total: (4 + 8*2) + 4 + 4 = 20 + 8 = 28
        assert_eq!(payload.len(), consumed + 8);
    }

    /// CM 89 offline variant: data_value must be 0.
    #[test]
    fn on_contact_list_event_offline_data_value() {
        let payload = build_on_contact_list_event("Annoying", EVENT_LOGGED_IN_STATUS, DATA_OFFLINE);
        let (_, consumed) = read_wstring(&payload[0..]);
        let data_value =
            i32::from_le_bytes(payload[consumed + 4..consumed + 8].try_into().unwrap());
        assert_eq!(data_value, DATA_OFFLINE);
    }

    /// Guard: EContactListEvent values are the power-of-two bit flags from
    /// `entities/defs/enumerations.xml` (1/2/4/8), not a 0-based index.
    #[test]
    fn event_ids_match_canonical_bitfield() {
        assert_eq!(EVENT_LOGGED_IN_STATUS, 1);
        assert_eq!(EVENT_GAIN_LEVEL, 2);
        assert_eq!(EVENT_DEATH, 4);
        assert_eq!(EVENT_GATE_TRAVEL, 8);
    }
}
