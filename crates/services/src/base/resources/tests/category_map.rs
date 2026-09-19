//! Category-map tests for the cooked-data enum.
//!
//! These pin the client's resource-category numbering so a future drift
//! back toward the legacy `resource.cpp` table (which reserved 21 for
//! `pet_command` and pushed `behavior_event` to 22) fails loudly instead
//! of silently dropping behavior-event fragments on the wire.

use super::super::{CATEGORY_BEHAVIOR_EVENTS, CATEGORY_PAKS};

/// The client must be served a *contiguous* 1..=21 enum — no category 0,
/// no category 22, no `pet_command`. The legacy `resource.cpp` table had 22
/// entries and drifted at the high end.
#[test]
fn category_paks_cover_exactly_the_client_1_to_21_enum() {
    let mut ids: Vec<u32> = CATEGORY_PAKS.iter().map(|&(id, _)| id).collect();
    ids.sort_unstable();
    let expected: Vec<u32> = (1..=21).collect();
    assert_eq!(
        ids, expected,
        "Rust category map must match the client's contiguous 1..=21 registration",
    );
}

/// Category 21 must be `behavior_event` (`CookedBehaviorEvents.pak`), not
/// `pet_command` (legacy) and not 22 (legacy drift).
#[test]
fn behavior_event_sits_at_category_21_with_its_pak() {
    let (id, pak) = CATEGORY_PAKS
        .iter()
        .find(|&&(_, pak)| pak == "CookedBehaviorEvents.pak")
        .copied()
        .expect("CookedBehaviorEvents.pak must be registered");
    assert_eq!(
        id, CATEGORY_BEHAVIOR_EVENTS,
        "CookedBehaviorEvents.pak must be category {}, not 22",
        CATEGORY_BEHAVIOR_EVENTS,
    );
    assert_eq!(
        pak, "CookedBehaviorEvents.pak",
        "the pak backing the behavior-event category",
    );
    assert_eq!(
        CATEGORY_BEHAVIOR_EVENTS, 21,
        "the behavior-event category id must be 21",
    );
}

/// The wire carries the category id in the first fragment header: build a
/// `BASEMSG_RESOURCE_FRAGMENT` for the behavior-event category and assert
/// the decrypted payload tags category 21 (LE u32 `0x15 00 00 00`), not 22.
#[test]
fn behavior_event_fragment_tags_category_21_on_the_wire() {
    use cimmeria_mercury::encryption::{EncryptionVersion, MercuryEncryption};

    use crate::mercury::protocol::build_resource_fragment;
    use crate::mercury::{FRAG_FIRST_AND_LAST, FRAG_MIDDLE};

    const TEST_KEY: [u8; 32] = [0x42u8; 32];
    let xml = b"<COOKED_BEHAVIOR_EVENT Behavior=\"7\" />";
    let out = build_resource_fragment(
        &TEST_KEY,
        5,
        &[],
        1,                              // data_id
        0,                              // chunk_id
        FRAG_FIRST_AND_LAST,            // single-fragment transfer
        Some(0),                        // msg_type = MESSAGE_CacheData
        Some(CATEGORY_BEHAVIOR_EVENTS), // category_id = 21
        Some(1024),                     // element_id
        xml,
        EncryptionVersion::V1,
    );

    let enc = MercuryEncryption::from_session_key(TEST_KEY);
    let plaintext = enc.decrypt(&out).expect("decrypt failed");

    // Body layout from build_resource_fragment: msg_id(1) + word_len(u16) then
    // payload = data_id(2) chunk_id(1) frag_flags(1) msg_type(1)
    //           category_id(4) element_id(4) xml. Payload starts after the
    // flags byte (offset 0) + msg_id(1) + word_len(2) = offset 4 in plaintext.
    let payload = &plaintext[4..];
    // data_id(2) chunk_id(1) frag_flags(1) msg_type(1) => category_id at offset 5.
    let cat_bytes = &payload[5..9];
    assert_eq!(
        cat_bytes,
        &[0x15, 0x00, 0x00, 0x00],
        "behavior-event fragment must carry category 21 (0x15 LE), not 22",
    );
    // Guard against the off-by-one as well: 22 would be 0x16 LE.
    assert_ne!(
        u32::from_le_bytes(cat_bytes.try_into().unwrap()),
        22,
        "category byte must not be the legacy 22 drift",
    );
    // Element id follows the category id.
    assert_eq!(
        u32::from_le_bytes(payload[9..13].try_into().unwrap()),
        1024,
        "element id must follow the category id",
    );
    // The FRAG flag should not accidentally slip into the header.
    assert_ne!(plaintext[4 + 3], FRAG_MIDDLE, "frag_flags byte sanity");
}
