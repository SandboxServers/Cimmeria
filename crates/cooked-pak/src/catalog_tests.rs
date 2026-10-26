use super::*;
use crate::digest::{category_digest, digest_hex};

/// A body for category 4, version 9, with three records and one change.
fn sample() -> CategoryBody {
    let full: BTreeMap<u32, Vec<u8>> = BTreeMap::from([
        (1, b"one".to_vec()),
        (2, b"two".to_vec()),
        (3, b"three".to_vec()),
    ]);
    CategoryBody {
        category_id: 4,
        version: 9,
        keys: vec![1, 2, 3],
        changed: BTreeMap::from([(2, b"two".to_vec())]),
        digest: category_digest(4, 9, full.iter().map(|(k, v)| (*k, v.as_slice()))),
    }
}

#[test]
fn body_round_trips() {
    let body = sample();
    let bytes = encode_body(&body);
    assert_eq!(&bytes[..4], b"CPS1");
    assert_eq!(decode_body(&bytes, &Limits::DEFAULT), Ok(body));
}

#[test]
fn a_wrong_magic_is_bad_magic() {
    let mut bytes = encode_body(&sample());
    bytes[0] = b'X';
    assert_eq!(
        decode_body(&bytes, &Limits::DEFAULT),
        Err(BodyError::BadMagic)
    );
}

#[test]
fn a_short_body_is_truncated() {
    let bytes = encode_body(&sample());
    assert_eq!(
        decode_body(&bytes[..bytes.len() - 1], &Limits::DEFAULT),
        Err(BodyError::Truncated)
    );
}

#[test]
fn unsorted_keys_are_not_ascending() {
    let mut body = sample();
    body.keys = vec![2, 1, 3];
    assert_eq!(
        decode_body(&encode_body(&body), &Limits::DEFAULT),
        Err(BodyError::NotAscending)
    );
}

#[test]
fn a_changed_key_outside_the_key_list_is_refused() {
    let mut body = sample();
    body.changed = BTreeMap::from([(9, b"x".to_vec())]);
    assert_eq!(
        decode_body(&encode_body(&body), &Limits::DEFAULT),
        Err(BodyError::ChangedKeyNotListed)
    );
}

#[test]
fn limits_are_checked_before_allocation() {
    // Magic, category, version, then key_count = u32::MAX and no keys.
    let mut bytes = Vec::from(&b"CPS1"[..]);
    bytes.extend_from_slice(&4u32.to_le_bytes());
    bytes.extend_from_slice(&9u32.to_le_bytes());
    bytes.extend_from_slice(&u32::MAX.to_le_bytes());
    bytes.extend_from_slice(&0u32.to_le_bytes());
    assert_eq!(bytes.len(), 20);
    assert_eq!(
        decode_body(&bytes, &Limits::DEFAULT),
        Err(BodyError::TooLarge)
    );
}

#[test]
fn an_oversize_record_is_too_large() {
    let bytes = encode_body(&sample());
    let limits = Limits {
        max_entry_bytes: 2,
        ..Limits::DEFAULT
    };
    assert_eq!(decode_body(&bytes, &limits), Err(BodyError::TooLarge));
}

#[test]
fn the_running_total_is_checked() {
    let mut body = sample();
    body.changed = BTreeMap::from([(1, b"aaa".to_vec()), (2, b"bbb".to_vec())]);
    let limits = Limits {
        max_total_bytes: 5,
        ..Limits::DEFAULT
    };
    assert_eq!(
        decode_body(&encode_body(&body), &limits),
        Err(BodyError::TooLarge)
    );
}

#[test]
fn more_changed_records_than_keys_is_too_large() {
    // One key, then a changed_count of 2 with no records after it.
    let mut bytes = Vec::from(&b"CPS1"[..]);
    bytes.extend_from_slice(&4u32.to_le_bytes());
    bytes.extend_from_slice(&9u32.to_le_bytes());
    bytes.extend_from_slice(&1u32.to_le_bytes());
    bytes.extend_from_slice(&1u32.to_le_bytes());
    bytes.extend_from_slice(&2u32.to_le_bytes());
    assert_eq!(
        decode_body(&bytes, &Limits::DEFAULT),
        Err(BodyError::TooLarge)
    );
}

#[test]
fn unsorted_changed_records_are_not_ascending() {
    // Keys 1 and 2; the changed records come as 2 then 1.
    let mut bytes = Vec::from(&b"CPS1"[..]);
    bytes.extend_from_slice(&4u32.to_le_bytes());
    bytes.extend_from_slice(&9u32.to_le_bytes());
    bytes.extend_from_slice(&2u32.to_le_bytes());
    bytes.extend_from_slice(&1u32.to_le_bytes());
    bytes.extend_from_slice(&2u32.to_le_bytes());
    bytes.extend_from_slice(&2u32.to_le_bytes());
    bytes.extend_from_slice(&2u32.to_le_bytes());
    bytes.extend_from_slice(&1u32.to_le_bytes());
    bytes.extend_from_slice(b"a");
    bytes.extend_from_slice(&1u32.to_le_bytes());
    bytes.extend_from_slice(&1u32.to_le_bytes());
    bytes.extend_from_slice(b"b");
    bytes.extend_from_slice(&[0; 32]);
    assert_eq!(
        decode_body(&bytes, &Limits::DEFAULT),
        Err(BodyError::NotAscending)
    );
}

#[test]
fn a_repeated_key_is_not_ascending() {
    let mut body = sample();
    body.keys = vec![1, 1, 2];
    body.changed = BTreeMap::new();
    assert_eq!(
        decode_body(&encode_body(&body), &Limits::DEFAULT),
        Err(BodyError::NotAscending)
    );
}

#[test]
fn bytes_after_the_digest_are_trailing() {
    let mut bytes = encode_body(&sample());
    bytes.push(b'!');
    assert_eq!(
        decode_body(&bytes, &Limits::DEFAULT),
        Err(BodyError::Trailing)
    );
}

#[test]
fn index_json_ignores_unknown_fields() {
    let json = format!(
        r#"{{"schema":1,"extra":true,"categories":[{{"category_id":4,"pak":"CookedDataItems.pak","version":123,"entries":5000,"changed":12,"body_bytes":40960,"digest":"{}","note":"x"}}]}}"#,
        digest_hex(&[0; 32])
    );
    let index: Index = serde_json::from_str(&json).unwrap();
    assert_eq!(index.schema, 1);
    assert_eq!(index.categories.len(), 1);
    let entry = &index.categories[0];
    assert_eq!(entry.category_id, 4);
    assert_eq!(entry.pak, "CookedDataItems.pak");
    assert_eq!(entry.version, 123);
    assert_eq!(entry.body_bytes, 40960);
}
