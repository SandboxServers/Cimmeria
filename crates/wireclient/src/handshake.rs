//! Mercury phase-3 handshake — the three packets that establish encryption.
//!
//! ## Wire shape
//!
//! ```text
//! ─────────── client → server (unencrypted) ───────────
//!   baseAppLogin        msg_id=0x00   WORD_LENGTH=25
//!     msg_id     : u8                 @ body[0]
//!     word_len   : u16 LE (= 25)      @ body[1..3]
//!     request_id : u32 LE             @ body[3..7]
//!     _padding   : u16 LE (= 0)       @ body[7..9]    ← 2-byte gap
//!     account_id : u32 LE             @ body[9..13]
//!     ticket_len : u8  (= 20)         @ body[13]
//!     ticket     : [u8; 20]   ASCII   @ body[14..34]
//!   flags = FLAG_HAS_REQUESTS | FLAG_HAS_SEQUENCE  (0x41)
//!   seq_id = 1                                     (footer)
//!   first_req_offset = 0                           (footer)
//!
//!   The 2-byte gap at body[7..9] matches the server's parser at
//!   `crates/services/src/base/login.rs:parse_baseapp_login` (reads
//!   account_id from body[9..13], not body[7..11]). The gap is observable
//!   in any captured `baseAppLogin` byte dump — e.g. the Castle Cellblock
//!   capture: `00 19 00 25 9d 01 00 [00 00] 01 00 00 00 14 …`.
//!
//! ─────────── server → client (encrypted) ───────────
//!   seq=1   BASEMSG_REPLY_MESSAGE (0xFF) echoing baseAppLogin
//!   seq=2   updateFrequencyNotification + tickSync + setGameTime
//! ```
//!
//! ## Why a hand-rolled builder
//!
//! Production Mercury sending lives in `cimmeria_mercury::Channel`, whose
//! state machine is direction-skewed for the server. For the three
//! handshake packets — exactly one unencrypted send, two known-shape
//! receives — we sidestep the state machine entirely. This module is the
//! smallest possible code that produces byte-exact output, and the byte-
//! exact tests pin it against the server's `parse_baseapp_login`.
//!
//! Steady-state post-handshake reliable I/O will land in a future
//! `channel.rs` (or by exposing a client-side façade on `Channel`).

use bytes::{BufMut, BytesMut};

use cimmeria_mercury::encryption::MercuryEncryption;
use cimmeria_mercury::packet::{
    build_outgoing, parse_incoming, FLAG_HAS_REQUESTS, FLAG_HAS_SEQUENCE, FLAG_ON_CHANNEL,
    FLAG_RELIABLE,
};

use crate::error::{Error, Result};

/// Static msg ID for `baseAppLogin` — first byte of the handshake body.
pub const MSG_BASEAPP_LOGIN: u8 = 0x00;

/// Static msg ID for `BASEMSG_REPLY_MESSAGE`, the server's reply to
/// `baseAppLogin`.
pub const MSG_BASEMSG_REPLY: u8 = 0xFF;

/// Static msg ID for `updateFrequencyNotification`.
pub const MSG_UPDATE_FREQ_NOTIFICATION: u8 = 0x02;

/// Static msg ID for `setGameTime`.
pub const MSG_SET_GAME_TIME: u8 = 0x03;

/// Static msg ID for `tickSync`.
pub const MSG_TICK_SYNC: u8 = 0x0D;

/// `baseAppLogin` body size as advertised in its WORD_LENGTH header. Used to
/// reject malformed replies before any other field check.
const BASEAPP_LOGIN_WORD_LENGTH: u16 = 25;

/// `BASEMSG_REPLY_MESSAGE` payload length advertised in its DWORD_LENGTH —
/// matches `BASEAPP_LOGIN_WORD_LENGTH` because the server echoes our login.
const REPLY_BODY_LENGTH: u32 = 25;

/// Exactly 20 bytes of ticket text on the wire — see
/// `parse_baseapp_login` server-side.
pub const TICKET_LEN: usize = 20;

/// Send-side flag combination for the unencrypted `baseAppLogin` packet.
/// Matches the C++ client at `BaseAppLoginHandler::onSend`: requests +
/// sequence, no ack, no channel-bit (the channel does not yet exist).
const FLAGS_LOGIN: u8 = FLAG_HAS_REQUESTS | FLAG_HAS_SEQUENCE;

/// Receive-side flag combination on the server's encrypted replies, used
/// only as a sanity check at parse time.
const FLAGS_REPLY: u8 = FLAG_ON_CHANNEL | FLAG_RELIABLE | FLAG_HAS_SEQUENCE;

// ── Outbound: baseAppLogin ──────────────────────────────────────────────────

/// Build the `baseAppLogin` Mercury packet (unencrypted, ready for
/// `UdpSocket::send_to`).
///
/// Inputs:
/// - `request_id` — caller-chosen 32-bit nonce; the reply echoes it back so
///   we can detect a wire-level desync. The client picks a fresh value per
///   attempt.
/// - `account_id` — 32-bit account ID from the auth Phase 2 response. The
///   server cross-checks this against its pending-logins map; mismatch =
///   silent reject. (TODO: surface `account_id` on `AuthSession` — Phase 2
///   currently returns only the ticket+endpoint+key.)
/// - `ticket` — 20-character ASCII ticket from Phase 2.
///
/// Returns the raw UDP datagram. `parse_incoming` on these bytes round-trips
/// to the same fields — see [`crate::handshake::tests::round_trip`].
pub fn build_baseapp_login(request_id: u32, account_id: u32, ticket: &str) -> Result<Vec<u8>> {
    if ticket.len() != TICKET_LEN {
        return Err(Error::InvalidTicketLength(ticket.len()));
    }

    // Body: [msg_id:u8][word_len:u16 LE][request_id:u32 LE][_pad:u16 LE = 0]
    //       [account_id:u32 LE][ticket_len:u8][ticket:20] = 34 bytes.
    //
    // The 2-byte pad between request_id and account_id is load-bearing —
    // the server's `parse_baseapp_login` reads account_id from body[9..13],
    // not body[7..11]. Dropping it produces a packet the server rejects
    // with `unexpected ticketLen`.
    let mut body = BytesMut::with_capacity(1 + 2 + 4 + 2 + 4 + 1 + TICKET_LEN);
    body.put_u8(MSG_BASEAPP_LOGIN);
    body.put_u16_le(BASEAPP_LOGIN_WORD_LENGTH);
    body.put_u32_le(request_id);
    body.put_u16_le(0); // 2-byte gap before account_id
    body.put_u32_le(account_id);
    body.put_u8(TICKET_LEN as u8);
    body.put_slice(ticket.as_bytes());

    // The C++ client emits `first_req_offset = 0` — the request begins at
    // the first byte of the body. `build_outgoing` writes the footer when
    // `FLAG_HAS_REQUESTS` is set.
    let pkt = build_outgoing(
        FLAGS_LOGIN,
        &body,
        Some(1), // seq_id
        &[],     // no acks
        Some(0), // first_req_offset
    );
    Ok(pkt.to_vec())
}

// ── Inbound: BASEMSG_REPLY_MESSAGE (seq=1) ──────────────────────────────────

/// Decoded `BASEMSG_REPLY_MESSAGE` body fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BaseAppReply {
    pub request_id: u32,
    pub ticket: String,
}

/// Decrypt + parse the first server reply (the `BASEMSG_REPLY_MESSAGE`
/// echo). Verifies:
///
/// - flags match the server's documented combination
/// - sequence ID is 1
/// - body length is exactly 25 (the WORD_LENGTH echo)
/// - msg_id is `BASEMSG_REPLY_MESSAGE` (`0xFF`)
/// - `request_id` matches what we sent
/// - ticket echoes what we sent
///
/// Any drift surfaces as a typed [`Error`] variant.
pub fn parse_baseapp_reply(
    raw: &[u8],
    enc: &MercuryEncryption,
    expected_request_id: u32,
    expected_ticket: &str,
) -> Result<BaseAppReply> {
    let plaintext = decrypt_packet(raw, enc)?;
    let parsed = parse_incoming(&plaintext).map_err(|e| Error::MercuryParse(e.to_string()))?;

    if parsed.flags != FLAGS_REPLY {
        return Err(Error::MercuryParse(format!(
            "phase-3 reply: unexpected flags 0x{:02x} (want 0x{:02x})",
            parsed.flags, FLAGS_REPLY
        )));
    }
    if parsed.seq_id != Some(1) {
        return Err(Error::MercuryParse(format!(
            "phase-3 reply: expected seq=1, got seq={:?}",
            parsed.seq_id
        )));
    }

    let body = parsed.body.as_ref();
    if body.is_empty() {
        return Err(Error::InvalidReplyLength(0));
    }
    if body[0] != MSG_BASEMSG_REPLY {
        return Err(Error::UnexpectedReplyMsg(body[0]));
    }
    // Body: [0xFF][DWORD_LENGTH=25 u32 LE][request_id u32 LE][ticket_len u8 = 20][ticket 20]
    // = 1 + 4 + 4 + 1 + 20 = 30 bytes.
    if body.len() < 30 {
        return Err(Error::InvalidReplyLength(body.len()));
    }
    let dword_len = u32::from_le_bytes([body[1], body[2], body[3], body[4]]);
    if dword_len != REPLY_BODY_LENGTH {
        return Err(Error::InvalidReplyLength(dword_len as usize));
    }
    let req = u32::from_le_bytes([body[5], body[6], body[7], body[8]]);
    if req != expected_request_id {
        return Err(Error::RequestIdMismatch {
            sent: expected_request_id,
            got: req,
        });
    }
    let ticket_len = body[9] as usize;
    if ticket_len != TICKET_LEN || body.len() < 10 + ticket_len {
        return Err(Error::InvalidReplyLength(body.len()));
    }
    let ticket = std::str::from_utf8(&body[10..10 + ticket_len])
        .map_err(|_| Error::MercuryParse("phase-3 reply: ticket is not ASCII".into()))?;
    if ticket != expected_ticket {
        return Err(Error::TicketMismatch);
    }

    Ok(BaseAppReply {
        request_id: req,
        ticket: ticket.to_string(),
    })
}

// ── Inbound: time sync triple (seq=2) ───────────────────────────────────────

/// Decoded `tickSync` snapshot from the server's seq=2 reply.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeSyncReply {
    pub update_frequency: u8,
    pub tick: u32,
    pub tick_rate: u32,
    pub game_time_tick: u32,
}

/// Decrypt + parse the second server reply (the three-message time-sync
/// bundle). Each sub-message is fixed-length:
///
/// - `updateFrequencyNotification`: 1-byte payload (the update frequency).
/// - `tickSync`: 8-byte payload (`tick:u32 LE`, `tick_rate:u32 LE`).
/// - `setGameTime`: 4-byte payload (`tick:u32 LE`).
pub fn parse_time_sync(raw: &[u8], enc: &MercuryEncryption) -> Result<TimeSyncReply> {
    let plaintext = decrypt_packet(raw, enc)?;
    let parsed = parse_incoming(&plaintext).map_err(|e| Error::MercuryParse(e.to_string()))?;

    if parsed.seq_id != Some(2) {
        return Err(Error::MercuryParse(format!(
            "time-sync: expected seq=2, got seq={:?}",
            parsed.seq_id
        )));
    }

    let body = parsed.body.as_ref();
    // 1 (msg) + 1 (freq) + 1 (msg) + 8 (tickSync) + 1 (msg) + 4 (setGameTime) = 16
    if body.len() < 16 {
        return Err(Error::InvalidReplyLength(body.len()));
    }

    let mut off = 0;
    if body[off] != MSG_UPDATE_FREQ_NOTIFICATION {
        return Err(Error::UnexpectedReplyMsg(body[off]));
    }
    off += 1;
    let update_frequency = body[off];
    off += 1;

    if body[off] != MSG_TICK_SYNC {
        return Err(Error::UnexpectedReplyMsg(body[off]));
    }
    off += 1;
    let tick = u32::from_le_bytes([body[off], body[off + 1], body[off + 2], body[off + 3]]);
    off += 4;
    let tick_rate = u32::from_le_bytes([body[off], body[off + 1], body[off + 2], body[off + 3]]);
    off += 4;

    if body[off] != MSG_SET_GAME_TIME {
        return Err(Error::UnexpectedReplyMsg(body[off]));
    }
    off += 1;
    let game_time_tick =
        u32::from_le_bytes([body[off], body[off + 1], body[off + 2], body[off + 3]]);

    Ok(TimeSyncReply {
        update_frequency,
        tick,
        tick_rate,
        game_time_tick,
    })
}

// ── shared ──────────────────────────────────────────────────────────────────

/// AES-decrypt + HMAC-verify a full encrypted Mercury datagram.
///
/// `MercuryEncryption::decrypt` consumes `[ciphertext || hmac_tag(16)]`
/// and does the HMAC verification itself — we hand it the raw datagram
/// directly. A drift to the wire format here surfaces as
/// `Error::Decryption("HMAC-MD5 verification failed")`, which is exactly
/// what the byte-exact handshake tests assert against.
fn decrypt_packet(raw: &[u8], enc: &MercuryEncryption) -> Result<Vec<u8>> {
    enc.decrypt(raw)
        .map_err(|e| Error::Decryption(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    const TICKET_SAMPLE: &str = "0011223344556677889A";

    #[test]
    fn build_baseapp_login_byte_shape() {
        let raw = build_baseapp_login(0x019D25, 1, TICKET_SAMPLE).unwrap();
        // Body offsets shift by +2 vs a naive layout because the wire format
        // carries a 2-byte gap between request_id and account_id. Offsets
        // here match the server's `parse_baseapp_login`.
        assert_eq!(raw[0], FLAGS_LOGIN, "flags byte (REQ|SEQ)");
        assert_eq!(raw[1], MSG_BASEAPP_LOGIN, "msg_id @ body[0]");
        assert_eq!(&raw[2..4], &[25, 0], "WORD_LENGTH 25 @ body[1..3]");
        assert_eq!(
            &raw[4..8],
            &0x019D25u32.to_le_bytes(),
            "request_id @ body[3..7]"
        );
        assert_eq!(&raw[8..10], &[0, 0], "2-byte gap @ body[7..9]");
        assert_eq!(
            &raw[10..14],
            &1u32.to_le_bytes(),
            "account_id @ body[9..13]"
        );
        assert_eq!(raw[14], 20, "ticket_len @ body[13]");
        assert_eq!(
            &raw[15..35],
            TICKET_SAMPLE.as_bytes(),
            "ticket @ body[14..34]"
        );
        // Total = 1 flags + 34 body + 2 first_req_offset + 4 seq = 41.
        assert_eq!(raw.len(), 41);
    }

    /// Byte-for-byte regression guard against the real Castle Cellblock
    /// capture. Source: `2026-05-24_17-18.pcap` packet #1 body, with
    /// request_id=0x00019D25, account_id=1, ticket "D0FC940335D6EC1A5FB5".
    ///
    /// A regression in the body layout — wrong offset, wrong endianness,
    /// dropped padding — will fail this test against a hardcoded literal
    /// pulled from the live wire dump. Constants under test cannot
    /// participate in their own assertion here.
    #[test]
    fn build_baseapp_login_matches_recorded_capture() {
        let raw = build_baseapp_login(0x019D25, 1, "D0FC940335D6EC1A5FB5").unwrap();
        let body = &raw[1..1 + 34];
        // Direct hex transcription of the body bytes from
        // `pcap_dissect.py` output, packet #1, body:
        //   00 19 00 25 9d 01 00 00 00 01 00 00 00 14
        //   44 30 46 43 39 34 30 33 33 35 44 36 45 43
        //   31 41 35 46 42 35
        #[rustfmt::skip]
        let expected: [u8; 34] = [
            0x00, 0x19, 0x00,
            0x25, 0x9d, 0x01, 0x00,
            0x00, 0x00,
            0x01, 0x00, 0x00, 0x00,
            0x14,
            b'D', b'0', b'F', b'C', b'9', b'4', b'0', b'3', b'3', b'5',
            b'D', b'6', b'E', b'C', b'1', b'A', b'5', b'F', b'B', b'5',
        ];
        assert_eq!(
            body,
            &expected[..],
            "wireclient body bytes must match the captured baseAppLogin byte-for-byte"
        );
    }

    #[test]
    fn build_baseapp_login_rejects_wrong_ticket_length() {
        let r = build_baseapp_login(0, 0, "TOO SHORT");
        assert!(matches!(r, Err(Error::InvalidTicketLength(9))));
    }

    #[test]
    fn build_baseapp_login_round_trips_through_parse_incoming() {
        let raw = build_baseapp_login(0xCAFEF00D, 42, TICKET_SAMPLE).unwrap();
        let parsed = parse_incoming(&raw).expect("parse_incoming");
        assert_eq!(parsed.flags, FLAGS_LOGIN);
        assert_eq!(parsed.seq_id, Some(1));
        assert_eq!(parsed.first_req_offset, Some(0));
        // Body must round-trip at 34 bytes — the 2-byte pad between
        // request_id and account_id is part of the canonical layout.
        assert_eq!(parsed.body.len(), 34);
        assert_eq!(parsed.body[0], MSG_BASEAPP_LOGIN);
    }

    #[test]
    fn parse_baseapp_reply_round_trips_against_known_plaintext() {
        // Construct the plaintext the server would have produced for a
        // ticket=TICKET_SAMPLE, request_id=0xCAFEF00D reply, then encrypt
        // with an all-zero key and parse it back.
        let key = [0u8; 32];
        let enc = MercuryEncryption::from_session_key(key);

        let mut body = Vec::with_capacity(30);
        body.push(MSG_BASEMSG_REPLY);
        body.extend_from_slice(&REPLY_BODY_LENGTH.to_le_bytes());
        body.extend_from_slice(&0xCAFEF00Du32.to_le_bytes());
        body.push(TICKET_LEN as u8);
        body.extend_from_slice(TICKET_SAMPLE.as_bytes());

        // Frame: flags + body + seq footer.
        let plaintext = build_outgoing(FLAGS_REPLY, &body, Some(1), &[], None);
        let mut wire = enc.encrypt(&plaintext).expect("encrypt");
        // `MercuryEncryption::encrypt` already appends the HMAC tag.
        // Sanity: `decrypt_packet` strips the trailing 16 bytes and feeds
        // the rest into `decrypt`.
        wire.shrink_to_fit();

        let reply = parse_baseapp_reply(&wire, &enc, 0xCAFEF00D, TICKET_SAMPLE).unwrap();
        assert_eq!(reply.request_id, 0xCAFEF00D);
        assert_eq!(reply.ticket, TICKET_SAMPLE);
    }

    #[test]
    fn parse_baseapp_reply_detects_request_id_mismatch() {
        let key = [0u8; 32];
        let enc = MercuryEncryption::from_session_key(key);
        let mut body = Vec::with_capacity(30);
        body.push(MSG_BASEMSG_REPLY);
        body.extend_from_slice(&REPLY_BODY_LENGTH.to_le_bytes());
        body.extend_from_slice(&0xDEADBEEFu32.to_le_bytes());
        body.push(TICKET_LEN as u8);
        body.extend_from_slice(TICKET_SAMPLE.as_bytes());
        let plaintext = build_outgoing(FLAGS_REPLY, &body, Some(1), &[], None);
        let wire = enc.encrypt(&plaintext).expect("encrypt");
        let err = parse_baseapp_reply(&wire, &enc, 0xCAFEF00D, TICKET_SAMPLE).unwrap_err();
        assert!(matches!(
            err,
            Error::RequestIdMismatch {
                sent: 0xCAFEF00D,
                got: 0xDEADBEEF
            }
        ));
    }

    #[test]
    fn parse_time_sync_decodes_all_three_subfields() {
        let key = [0u8; 32];
        let enc = MercuryEncryption::from_session_key(key);

        let mut body = Vec::with_capacity(16);
        body.push(MSG_UPDATE_FREQ_NOTIFICATION);
        body.push(10);
        body.push(MSG_TICK_SYNC);
        body.extend_from_slice(&42u32.to_le_bytes()); // tick
        body.extend_from_slice(&100u32.to_le_bytes()); // tick_rate
        body.push(MSG_SET_GAME_TIME);
        body.extend_from_slice(&7u32.to_le_bytes()); // game_time_tick

        let plaintext = build_outgoing(FLAGS_REPLY, &body, Some(2), &[], None);
        let wire = enc.encrypt(&plaintext).expect("encrypt");

        let out = parse_time_sync(&wire, &enc).unwrap();
        assert_eq!(out.update_frequency, 10);
        assert_eq!(out.tick, 42);
        assert_eq!(out.tick_rate, 100);
        assert_eq!(out.game_time_tick, 7);
    }
}
