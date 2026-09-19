//! Byte-exact guards on the per-message length-prefix width.
//!
//! Mercury frames each message in a bundle as
//! `[msg_id][length prefix][payload]`, and the prefix width comes from
//! the message's `InterfaceElement` descriptor rather than from the
//! payload: `CONSTANT_LENGTH` writes no prefix, `WORD_LENGTH` a `u16`
//! LE, `DWORD_LENGTH` a `u32` LE. A prefix written at the wrong width
//! does not truncate the payload — it moves every following byte, so
//! the peer reads the fields at the wrong offsets and then starts the
//! next message mid-payload.
//!
//! Packet size does not witness that. `build_connect_reply`'s plaintext
//! is 35 bytes with a `u32` prefix and 33 with a `u16` one, and both pad
//! to the same 48-byte AES-CBC ciphertext, so `connect_reply_size`
//! passes either way. The advertised length does not witness it either:
//! the reply's payload is 25 bytes, and `19 00 00 00` read as a `u16` is
//! still 25. Only the offsets of the fields behind the prefix separate
//! the two layouts, which is what these tests assert.
//!
//! The reply's width is pinned as `DWORD_LENGTH` because that is what
//! the builder emits today and what the original server's
//! `connect_handler.cpp` selected for it (it passes the server-side
//! `AUTHENTICATE` descriptor, which is `DWORD_LENGTH`). Whether the
//! client's reader agrees is an open question tracked separately; these
//! tests exist so a change of width is a deliberate edit rather than a
//! silent one.

use super::*;
use cimmeria_mercury::encryption::{EncryptionVersion, MercuryEncryption};

const TEST_KEY: [u8; 32] = [0x42u8; 32];

/// Every byte of a ticket that makes a truncated read obvious: no zero
/// bytes, so a prefix read at the wrong width cannot coincidentally
/// match the expected length.
const TICKET: [u8; 20] = [
    0xA1, 0xA2, 0xA3, 0xA4, 0xA5, 0xA6, 0xA7, 0xA8, 0xA9, 0xAA, 0xAB, 0xAC, 0xAD, 0xAE, 0xAF, 0xB0,
    0xB1, 0xB2, 0xB3, 0xB4,
];

/// Low bytes are non-zero so a `u16`-prefixed layout shifts them into
/// the length field's high half and fails the length assertion.
const REQUEST_ID: u32 = 0x1234_5678;

#[test]
fn connect_reply_frames_its_payload_with_a_four_byte_length_prefix() {
    let out = build_connect_reply(REQUEST_ID, &TICKET, &TEST_KEY, 1, EncryptionVersion::V1);
    let plaintext = MercuryEncryption::from_session_key(TEST_KEY)
        .decrypt(&out)
        .expect("connect reply must decrypt under the session key");

    assert_eq!(plaintext[0], REPLY_FLAGS, "outer packet flags");
    assert_eq!(
        plaintext[1], BASEMSG_REPLY_MESSAGE,
        "msg_id must be BASEMSG_REPLY_MESSAGE (0xFF)",
    );

    // DWORD_LENGTH: the prefix occupies bytes 2..6, and its value is the
    // payload size derived from the ticket rather than a repeated literal.
    let expected_payload_len = 4 + 1 + TICKET.len();
    assert_eq!(
        u32::from_le_bytes(plaintext[2..6].try_into().unwrap()) as usize,
        expected_payload_len,
        "u32 LE length prefix must equal request_id + ticketLen byte + ticket",
    );

    // Payload begins at 6 only if the prefix really consumed four bytes.
    assert_eq!(
        u32::from_le_bytes(plaintext[6..10].try_into().unwrap()),
        REQUEST_ID,
        "request_id must start immediately after a four-byte length prefix",
    );
    assert_eq!(
        plaintext[10],
        TICKET.len() as u8,
        "ticketLen byte follows request_id",
    );
    assert_eq!(
        &plaintext[11..11 + TICKET.len()],
        &TICKET,
        "ticket bytes follow ticketLen, unshifted",
    );

    // The seq_id footer lands where the DWORD framing puts it; a narrower
    // prefix would slide it two bytes earlier.
    let footer = 11 + TICKET.len();
    assert_eq!(
        u32::from_le_bytes(plaintext[footer..footer + 4].try_into().unwrap()),
        1,
        "seq_id footer position is set by the message's framed length",
    );
    assert_eq!(
        plaintext.len(),
        footer + 4,
        "no trailing bytes beyond the seq_id footer",
    );
}

/// `build_time_sync` packs `CONSTANT_LENGTH` messages, which carry no
/// length prefix at all. Pinning the first two proves the builder does
/// not reach for the generic `WORD_LENGTH` bundle encoder.
#[test]
fn time_sync_packs_constant_length_messages_with_no_length_prefix() {
    let out = build_time_sync(&TEST_KEY, 2, EncryptionVersion::V1);
    let plaintext = MercuryEncryption::from_session_key(TEST_KEY)
        .decrypt(&out)
        .expect("time sync must decrypt under the session key");

    assert_eq!(plaintext[0], REPLY_FLAGS, "outer packet flags");
    assert_eq!(
        plaintext[1], BASEMSG_UPDATE_FREQUENCY_NOTIFICATION,
        "first message id",
    );
    // CONSTANT_LENGTH = 1: the payload byte sits directly after the id.
    assert_eq!(plaintext[2], 10, "update frequency payload byte");
    assert_eq!(
        plaintext[3], BASEMSG_TICK_SYNC,
        "second message id must follow one payload byte, not a length prefix",
    );
}
