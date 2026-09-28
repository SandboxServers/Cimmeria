use super::*;

/// Pin the framing of `restoreClientAck` (msg 0x0B) at
/// CONSTANT_LENGTH = 4. Spec §2.5.2 names this — the sole
/// emitter at `ghidra://SGW.exe@0x00dd8bc9` writes a literal
/// `i32 = 0`, with no `u16` length prefix in front of it.
///
/// Bug shape: parsing it as WORD_LENGTH would read the first
/// two ack bytes as a length prefix (`0x00 0x00` → length 0),
/// advance only 2 bytes past the msg_id, then read the
/// remaining two ack bytes as the NEXT msg_id (`0x00 0x00` →
/// `baseAppLogin`) and cascade-fail. This guard reproduces the
/// exact bundle layout the bug fires on:
///
/// ```text
///   [0x0B][0x00 0x00 0x00 0x00][0x07][0x05 0x00][...5-byte body]
///    ack         payload         next msg_id  u16 len
/// ```
///
/// Correct behavior: after reading the 0x0B + its 4-byte body,
/// `offset` lands at 5, pointing exactly at the next msg_id
/// (`0x07`, REQUEST_ENTITY_UPDATE). Reverting to
/// `read_word_length_payload` lands at 3 (1 msg_id + 2 length
/// prefix), reads `0x00` as next msg_id, and the assertion at
/// the bottom trips.
#[test]
fn restore_client_ack_consumes_exactly_four_bytes() {
    // Bundle: 0x0B + 4-byte ack body + a real 0x07 message
    // (WORD_LENGTH) with a 5-byte payload. The framing-bug fix
    // is observable as "we read 0x07 as the next msg_id, not
    // 0x00", which only holds when 0x0B consumes 4 bytes.
    let bundle = [
        0x0B, // restoreClientAck
        0x00, 0x00, 0x00, 0x00, // ack body (i32 = 0)
        0x07, // REQUEST_ENTITY_UPDATE
        0x05, 0x00, // u16 length = 5
        0xDE, 0xAD, 0xBE, 0xEF, 0x42, // payload
    ];

    // First message: consume the ack.
    let mut offset = 1; // past msg_id 0x0B
    let ack_payload = read_client_message_payload(0x0B, &bundle, &mut offset)
        .expect("0x0B must produce a payload — it's CONSTANT_LENGTH = 4");
    assert_eq!(
        ack_payload,
        &[0x00, 0x00, 0x00, 0x00],
        "ack payload must be the literal i32 = 0 (four zero bytes)"
    );
    assert_eq!(
        offset, 5,
        "offset must advance to exactly 5 (1 msg_id + 4 body). \
         Pre-fix WORD_LENGTH parse advances to 3 (1 + 2 prefix + 0 length), \
         leaving two ack bytes unconsumed and misaligning every following message."
    );

    // Second message: confirm we land on 0x07 (the canary).
    let next_msg_id = bundle[offset];
    assert_eq!(
        next_msg_id, 0x07,
        "next msg_id must be 0x07 (REQUEST_ENTITY_UPDATE). Pre-fix this \
         would be 0x00 (baseAppLogin) because the WORD_LENGTH bug skips \
         only 2 bytes of the 4-byte ack, leaking 0x00 0x00 into the next \
         msg_id slot."
    );

    offset += 1;
    let req_payload = read_client_message_payload(0x07, &bundle, &mut offset)
        .expect("0x07 must produce a payload");
    assert_eq!(
        req_payload,
        &[0xDE, 0xAD, 0xBE, 0xEF, 0x42],
        "downstream message must round-trip cleanly: if 0x0B framing is \
         right, the parser arrives at 0x07's length prefix and reads the \
         5-byte payload as expected"
    );
    assert_eq!(
        offset,
        bundle.len(),
        "final offset must consume the entire bundle"
    );
}

/// Negative pin: a 0x0B payload truncated below 4 bytes must
/// return `None`, signalling the bundle scan to break — NOT a
/// silent advance past the end of `body`.
#[test]
fn restore_client_ack_truncation_returns_none() {
    let bundle = [0x0B, 0x00, 0x00, 0x00]; // only 3 ack bytes, not 4
    let mut offset = 1;
    assert!(
        read_client_message_payload(0x0B, &bundle, &mut offset).is_none(),
        "truncated 0x0B body must return None so the caller breaks the \
         bundle loop with a 'truncated' trace — silently advancing past \
         the end would corrupt all downstream offset arithmetic."
    );
}

/// Round-trip pin for the unchanged CONSTANT_LENGTH entries —
/// catches a future refactor that swaps the dispatch arms with
/// each other. Pre-fix this passed; the bug was the missing
/// 0x0B row, not these.
#[test]
fn constant_length_dispatch_widths_match_spec() {
    let cases: &[(u8, usize)] = &[
        (0x02, 36), // AVATAR_UPD_IMPLICIT
        (0x03, 40), // AVATAR_UPDATE_EXPLICIT
        (0x04, 36), // AVATAR_UPDW_IMPLICIT
        (0x05, 40), // AVATAR_UPDW_EXPLICIT
        (0x06, 0),  // SWITCH_INTERFACE
        (0x08, 8),  // ENABLE_ENTITIES
        (0x09, 8),  // VIEWPORT_ACK
        (0x0A, 8),  // VEHICLE_ACK
        (0x0B, 4),  // RESTORE_CLIENT_ACK
        (0x0C, 1),  // DISCONNECT
    ];
    for &(msg_id, expected_width) in cases {
        // Construct a fresh body with exactly `expected_width`
        // bytes of payload after the (implicit) msg_id slot.
        let body: Vec<u8> = vec![0xAA; expected_width];
        let mut offset = 0;
        let payload =
            read_client_message_payload(msg_id, &body, &mut offset).unwrap_or_else(|| {
                panic!(
                    "msg {msg_id:#04x} CONSTANT_LENGTH should accept a body of \
                     exactly {expected_width} bytes"
                )
            });
        assert_eq!(
            payload.len(),
            expected_width,
            "msg {msg_id:#04x} produced wrong payload width"
        );
        assert_eq!(
            offset, expected_width,
            "msg {msg_id:#04x} must advance offset by exactly {expected_width}"
        );
    }
}

/// Pin the WORD_LENGTH default arm by msg_id family. Two
/// distinct classes share the wildcard path:
///
/// - **0x07** — `requestEntityUpdate`, the only system msg_id
///   in the WORD_LENGTH group. An explicit arm so the test
///   catches a regression where it slips into a different
///   width by accident.
/// - **0xC2** — sample base-method msg_id from the `0xC0+`
///   entity-method range. `messages.cpp` documents every
///   `0x80..0xFE` byte as WORD_LENGTH per
///   `ServerConnection_startEntityMessage` (0x00dd6a60) and
///   `ServerConnection_startProxyMessage` (0x00dd6980). The
///   `_ => read_word_length_payload` wildcard arm handles
///   the whole range, so one representative msg_id is enough
///   to pin the contract.
///
/// Together these guard against a future refactor that adds
/// a special-case `0x07` arm with the wrong width, or that
/// changes the wildcard arm to CONSTANT (which would
/// catastrophically break every entity-method call).
#[test]
fn word_length_dispatch_arms_consume_u16_prefix_then_payload() {
    // Sweep both 0x07 (named arm) and 0xC2 (wildcard sample) so
    // a single test trips when EITHER arm regresses.
    for &msg_id in &[0x07u8, 0xC2u8] {
        // body: [len_lo, len_hi, payload...]
        let payload_bytes: &[u8] = &[0xDE, 0xAD, 0xBE, 0xEF];
        let mut body = Vec::with_capacity(2 + payload_bytes.len());
        body.extend_from_slice(&(payload_bytes.len() as u16).to_le_bytes());
        body.extend_from_slice(payload_bytes);

        let mut offset = 0;
        let read = read_client_message_payload(msg_id, &body, &mut offset)
            .unwrap_or_else(|| panic!("msg {msg_id:#04x} WORD_LENGTH must produce a payload"));
        assert_eq!(
            read, payload_bytes,
            "msg {msg_id:#04x} WORD_LENGTH must return the post-prefix payload bytes"
        );
        assert_eq!(
            offset,
            2 + payload_bytes.len(),
            "msg {msg_id:#04x} WORD_LENGTH must advance offset by 2 (prefix) + payload_len"
        );
    }
}

/// Truncated WORD_LENGTH prefix (only one byte of the u16) on
/// the wildcard arm must return None so the bundle loop
/// breaks cleanly. Symmetric to the CONSTANT-truncation guard
/// above; ensures every dispatch arm refuses to silently
/// over-read.
#[test]
fn word_length_truncated_prefix_returns_none() {
    let body = [0x05u8]; // missing the high byte of the u16 prefix
    let mut offset = 0;
    assert!(
        read_client_message_payload(0xC2, &body, &mut offset).is_none(),
        "truncated WORD_LENGTH prefix must return None — silently advancing \
         past the end of `body` would corrupt every downstream offset."
    );
}

// --- parse_request_entity_update parser pins (msg 0x07 body) ---
//
// Wire (corrected 2026-09-28, issue #838):
// `[u32 entityId][N × u32 cacheStamp]`. These pins replace the pre-#838
// `[u32 header][N × u32 entity_id]` suite, which decoded every real client
// payload (a bare 4-byte entity id) to an empty list -- see
// `docs/reverse-engineering/findings/request-entity-update-cache-stamp.md`.

/// The real client payload observed on the wire: exactly 4 bytes, the bare
/// entity id, no cache stamps. This is the byte-exact regression guard for
/// #838 -- it fails against the pre-fix `[u32 header][N × u32 entity_id]`
/// parser, which decodes this exact payload to an empty `Vec`.
#[test]
fn parses_bare_entity_id_with_no_cache_stamps() {
    // `52 88 01 00` little-endian = entity 100434, from the issue's colo
    // telemetry citation.
    let body = [0x52, 0x88, 0x01, 0x00];
    assert_eq!(parse_request_entity_update(&body), vec![100434]);
}

/// An entity id followed by cache stamps still decodes to just the entity
/// id -- the stamps are consumed but not interpreted (the server has no
/// per-property cache to diff them against, and no observed client build
/// ever sends a non-empty stamp list).
#[test]
fn cache_stamps_after_the_entity_id_do_not_change_the_decoded_id() {
    let mut body = Vec::new();
    body.extend_from_slice(&100u32.to_le_bytes()); // entityId
    body.extend_from_slice(&1u32.to_le_bytes()); // cacheStamp[0]
    body.extend_from_slice(&2u32.to_le_bytes()); // cacheStamp[1]
    assert_eq!(parse_request_entity_update(&body), vec![100]);
}

/// Sub-4-byte payloads (no complete entity id) defensively return empty.
/// The dispatch arm relies on this to no-op without panicking when the
/// client (or a fuzzer) sends a malformed body.
#[test]
fn truncated_payload_returns_empty() {
    for len in 0..4 {
        let body = vec![0u8; len];
        assert!(
            parse_request_entity_update(&body).is_empty(),
            "expected empty result for {len}-byte body"
        );
    }
}

/// Trailing cache-stamp bytes that don't form a complete u32 are consumed
/// without affecting the decoded entity id.
#[test]
fn trailing_partial_cache_stamp_is_dropped() {
    let mut body = Vec::new();
    body.extend_from_slice(&42u32.to_le_bytes()); // entityId
    body.extend_from_slice(&[0xAA, 0xBB, 0xCC]); // partial cacheStamp, dropped
    assert_eq!(parse_request_entity_update(&body), vec![42]);
}

/// Endianness: the entity id is little-endian.
#[test]
fn entity_id_is_little_endian() {
    // 0x01020304 little-endian = [04, 03, 02, 01]
    let body = [0x04, 0x03, 0x02, 0x01];
    assert_eq!(parse_request_entity_update(&body), vec![0x01020304]);
}

/// A zero entity id is a valid decode, not a sentinel for "empty" -- only a
/// too-short payload produces an empty `Vec`.
#[test]
fn zero_entity_id_decodes_as_a_real_id_not_as_empty() {
    let body = [0u8; 4];
    assert_eq!(parse_request_entity_update(&body), vec![0]);
}
