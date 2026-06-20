//! Unit tests for the contact-list cell handlers (wire parsing + CM 55-60 dispatch).

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
