//! The behavior-event category's id on the wire: a `RESOURCE_FRAGMENT`
//! built for `CATEGORY_BEHAVIOR_EVENTS` must tag category 21, not the legacy
//! 22.
//!
//! Moved here from `cimmeria-resources`' `base::resources::tests::category_map`
//! when `base::resources` was split out (services-crate-split W1b): the
//! fragment builder it drives, `crate::mercury::protocol`, is still in this
//! crate. The category-map tests that need only the table stayed there.

use crate::base::resources::CATEGORY_BEHAVIOR_EVENTS;

/// The wire carries the category id in the first fragment header: build a
/// `BASEMSG_RESOURCE_FRAGMENT` for the behavior-event category and assert
/// the decrypted payload tags category 21 (LE u32 `0x15 00 00 00`), not 22.
#[test]
fn behavior_event_fragment_tags_category_21_on_the_wire() {
    use cimmeria_mercury::encryption::{EncryptionVersion, MercuryEncryption};

    use crate::mercury::protocol::build_resource_fragment;
    use crate::mercury::FRAG_FIRST_AND_LAST;

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
    // Single-fragment transfer stays tagged first+last, not a bare MIDDLE.
    assert_eq!(
        payload[3], FRAG_FIRST_AND_LAST,
        "frag_flags byte must carry the first+last marker for a single fragment",
    );
}
