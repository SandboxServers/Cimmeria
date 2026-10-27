use super::*;

/// Byte-exact default-version regression: with the DEFAULT server config, the
/// version selected for a session resolves to v1 and the outbound handshake
/// bytes are identical to an explicit v1 build. This is the guard that fails if
/// someone flips `ServerConfig::default().mercury_encryption_version` to 2 (or
/// the `from_config_u8` mapping changes) — producing v2 frames by default would
/// silently break every unpatched client. Reverting the default to v1 makes it
/// pass again; setting it to 2 makes it fail.
#[test]
fn default_config_session_produces_v1_handshake_bytes() {
    use cimmeria_common::ServerConfig;
    use cimmeria_mercury::encryption::EncryptionVersion;

    let config = ServerConfig::default();
    let selected = EncryptionVersion::from_config_u8(config.mercury_encryption_version);
    assert_eq!(
        selected,
        EncryptionVersion::V1,
        "default config must select v1 — unpatched clients only understand v1"
    );

    let key = [0x42u8; 32];
    let request_id = 0xDEAD_BEEF;
    let ticket = [0x11u8; 20];

    // The version threaded from the default config must reproduce the explicit
    // v1 bytes for both handshake builders.
    let reply_default = build_connect_reply(request_id, &ticket, &key, 1, selected);
    let reply_v1 = build_connect_reply(request_id, &ticket, &key, 1, EncryptionVersion::V1);
    assert_eq!(reply_default, reply_v1, "default connect_reply must be v1");

    let sync_default = build_time_sync(&key, 2, 0, selected);
    let sync_v1 = build_time_sync(&key, 2, 0, EncryptionVersion::V1);
    assert_eq!(sync_default, sync_v1, "default time_sync must be v1");

    // And the v1 frame must NOT start with the v2 version byte — the actual
    // wire-shape assertion, not just "two builders agree".
    assert_ne!(
        reply_default.first(),
        Some(&0x02),
        "v1 handshake frame must not carry the v2 version byte"
    );
}

/// A session built with v2 produces v2 handshake frames (leading `0x02`) — the
/// version selection actually reaches the outbound builders. Pairs with
/// `default_config_session_produces_v1_handshake_bytes` to prove the switch is
/// live in both directions of the config mapping.
#[test]
fn v2_config_session_produces_v2_handshake_frame() {
    use cimmeria_mercury::encryption::EncryptionVersion;

    let selected = EncryptionVersion::from_config_u8(2);
    assert_eq!(selected, EncryptionVersion::V2);

    let key = [0x42u8; 32];
    let reply = build_connect_reply(0xCAFE, &[0x22u8; 20], &key, 1, selected);
    assert_eq!(
        reply.first(),
        Some(&0x02),
        "v2-configured session must emit a v2 frame"
    );
}

#[test]
fn decode_session_key_valid() {
    let hex = "AABBCCDD".repeat(8); // 64 chars
    let key = decode_session_key(&hex).unwrap();
    assert_eq!(key[0], 0xAA);
    assert_eq!(key[1], 0xBB);
    assert_eq!(key[2], 0xCC);
    assert_eq!(key[3], 0xDD);
}

#[test]
fn decode_session_key_too_short() {
    let result = decode_session_key("AABB");
    assert!(result.is_err());
}

/// Decode rejects non-hex characters with a clear error rather than
/// silently producing zero bytes. Auth feeds this raw from the
/// upstream POST body, so a malformed key must surface as a Phase 3
/// reject, not a silent connection with a wrong key.
#[test]
fn decode_session_key_rejects_invalid_hex() {
    // 64 chars, all non-hex characters (Z is outside [0-9A-Fa-f]).
    let s = "ZZ".repeat(32);
    assert!(decode_session_key(&s).is_err());
}

/// Mixed lowercase hex must round-trip identically to uppercase —
/// the auth service may emit either depending on upstream casing.
#[test]
fn decode_session_key_accepts_lowercase_hex() {
    let upper = "AABBCCDD".repeat(8);
    let lower = "aabbccdd".repeat(8);
    assert_eq!(
        decode_session_key(&upper).unwrap(),
        decode_session_key(&lower).unwrap(),
        "uppercase and lowercase hex must decode to the same key bytes"
    );
}

#[test]
fn parse_baseapp_login_valid() {
    let mut raw = Vec::new();
    raw.push(0x41u8); // flags
    raw.push(0x00u8); // msg_id
    raw.extend_from_slice(&25u16.to_le_bytes());
    raw.extend_from_slice(&0xCAFEBABEu32.to_le_bytes());
    raw.extend_from_slice(&0u16.to_le_bytes());
    raw.extend_from_slice(&1u32.to_le_bytes());
    raw.push(20u8);
    raw.extend_from_slice(b"ABCDEF1234567890ABCD");
    raw.extend_from_slice(&1u16.to_le_bytes());
    raw.extend_from_slice(&3u32.to_le_bytes());

    assert_eq!(raw.len(), 41);

    let (req_id, ticket) = parse_baseapp_login(&raw).unwrap();
    assert_eq!(req_id, 0xCAFEBABE);
    assert_eq!(ticket, "ABCDEF1234567890ABCD");
}

/// Truncated body (less than the 34-byte fixed header for a valid
/// `baseAppLogin`) must reject — the auth → base seam is the first
/// validation point a malformed Phase 3 packet hits, and silently
/// continuing past truncation would index OOB on the ticket bytes.
///
/// The packet IS well-formed at the `parse_incoming` layer (valid
/// flags + footers); only the body is short. The error must come
/// from `parse_baseapp_login`'s own `body.len() < 34` check, not
/// from underflow in the upstream packet parser.
#[test]
fn parse_baseapp_login_rejects_truncated_body() {
    // FLAG_HAS_REQUESTS | FLAG_HAS_SEQUENCE = 0x41. With these flags the
    // upstream parser strips two trailing footers from the buffer
    // before yielding the body (innermost → outermost):
    //   * first_req_offset (u16) — where requests begin in the body
    //   * seq_id           (u32)
    // Layout:
    //   raw = [flags][..body..][first_req_offset u16][seq_id u32]
    // We give a 13-byte body (msg_id 0x00 + word_len 25u16 + 10
    // zero filler). 13 < 34, so parse_baseapp_login's own length
    // guard fires AFTER the upstream parser succeeds.
    let mut raw = vec![0x41u8]; // flags
    raw.push(0x00u8); // msg_id
    raw.extend_from_slice(&25u16.to_le_bytes()); // word_len (well-formed)
    raw.extend_from_slice(&[0u8; 10]); // zero filler — body total = 13 bytes
    raw.extend_from_slice(&0u16.to_le_bytes()); // first_req_offset footer
    raw.extend_from_slice(&7u32.to_le_bytes()); // seq_id footer

    let err =
        parse_baseapp_login(&raw).expect_err("13-byte body must trigger the 34-byte length guard");
    let msg = err.to_string();
    assert!(
        msg.contains("body too short") || msg.contains("34"),
        "error must come from parse_baseapp_login's body-length check, got: {msg}"
    );
}

/// An unexpected msg_id must reject — the parser is keyed on 0x00
/// (`baseAppLogin`). Without this guard a misrouted packet of a
/// different shape would surface as a confusing "ticketLen wrong"
/// error instead of the clear msg_id mismatch.
#[test]
fn parse_baseapp_login_rejects_wrong_msg_id() {
    let mut raw = vec![0x41u8, 0x99u8]; // msg_id = 0x99 (not 0x00)
    raw.extend_from_slice(&25u16.to_le_bytes());
    raw.extend_from_slice(&0u32.to_le_bytes());
    raw.extend_from_slice(&0u16.to_le_bytes());
    raw.extend_from_slice(&1u32.to_le_bytes());
    raw.push(20u8);
    raw.extend_from_slice(b"ABCDEF1234567890ABCD");
    raw.extend_from_slice(&1u16.to_le_bytes());
    raw.extend_from_slice(&3u32.to_le_bytes());
    let err = parse_baseapp_login(&raw).unwrap_err().to_string();
    assert!(
        err.contains("msg_id"),
        "error message should mention msg_id, got: {err}"
    );
}
