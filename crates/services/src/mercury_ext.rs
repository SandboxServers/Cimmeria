//! Helpers for building encrypted Mercury packets from the BaseApp server side.
//!
//! These functions produce byte-identical wire output to the C++ BaseApp:
//! - [`build_connect_reply`] — `BASEMSG_REPLY_MESSAGE` echoing the ticket back.
//! - [`build_time_sync`] — three time-sync messages in one packet.
//!
//! Each function returns a `Vec<u8>` ready to write to the UDP socket.

use cimmeria_mercury::encryption::MercuryEncryption;
use cimmeria_mercury::packet::{
    build_outgoing, FLAG_HAS_SEQUENCE, FLAG_ON_CHANNEL, FLAG_RELIABLE,
};

/// Server→client reply flags (HAS_SEQUENCE | ON_CHANNEL | RELIABLE = 0x58).
const REPLY_FLAGS: u8 = FLAG_HAS_SEQUENCE | FLAG_ON_CHANNEL | FLAG_RELIABLE;

// ── Message IDs ───────────────────────────────────────────────────────────────

/// `BASEMSG_REPLY_MESSAGE` — reply to a client request (message 0xFF).
const BASEMSG_REPLY_MESSAGE: u8 = 0xFF;
/// `BASEMSG_UPDATE_FREQUENCY_NOTIFICATION` — tick update frequency (0x02).
const BASEMSG_UPDATE_FREQUENCY_NOTIFICATION: u8 = 0x02;
/// `BASEMSG_TICK_SYNC` — current tick counter and rate (0x0D).
const BASEMSG_TICK_SYNC: u8 = 0x0D;
/// `BASEMSG_SET_GAME_TIME` — set client game clock (0x03).
const BASEMSG_SET_GAME_TIME: u8 = 0x03;

// ── Public builders ───────────────────────────────────────────────────────────

/// Build and encrypt the `BASEMSG_REPLY_MESSAGE` packet.
///
/// This is the server's response to the `baseAppLogin` connect request.
/// The reply echoes `request_id` and the 20-byte `ticket` back to the client
/// so it can verify the server is legitimate.
///
/// # Wire layout (before encryption)
/// ```text
/// [0x58]                  flags
/// [0xFF]                  msg_id = BASEMSG_REPLY_MESSAGE
/// [25, 0, 0, 0]           DWORD_LENGTH = 25
/// [request_id: u32 LE]    request ID echoed from the client's baseAppLogin
/// [0x14]                  ticketLen = 20
/// [ticket: 20 bytes]      ticket echoed from the client's baseAppLogin
/// [seq_id: u32 LE]        footer: sequence number
/// ```
///
/// Total plaintext: 35 bytes → PKCS7-padded to 48 → encrypted → + 16 HMAC = 64 bytes.
pub fn build_connect_reply(
    request_id: u32,
    ticket: &[u8],
    key: &[u8; 32],
    seq_id: u32,
) -> Vec<u8> {
    assert_eq!(ticket.len(), 20, "ticket must be exactly 20 bytes");

    // Build message body: [0xFF][DWORD_LEN:u32][requestId:u32][ticketLen:u8][ticket:20]
    let mut body = Vec::with_capacity(1 + 4 + 4 + 1 + 20);
    body.push(BASEMSG_REPLY_MESSAGE);
    body.extend_from_slice(&25u32.to_le_bytes()); // DWORD_LENGTH = 25
    body.extend_from_slice(&request_id.to_le_bytes());
    body.push(ticket.len() as u8); // ticketLen = 20
    body.extend_from_slice(ticket);

    let plaintext = build_outgoing(REPLY_FLAGS, &body, Some(seq_id), &[], None);

    encrypt_packet(&plaintext, key)
}

/// Build and encrypt the time-sync bundle packet.
///
/// Packs three constant-length messages into one packet, matching the C++
/// `ClientHandler::onConnected()` sequence:
///
/// 1. `BASEMSG_UPDATE_FREQUENCY_NOTIFICATION` — `updateFreq = 10`
///    (1000 ms / 100 ticks/sec).
/// 2. `BASEMSG_TICK_SYNC` — `ticks = 0`, `tickRate = 100`.
/// 3. `BASEMSG_SET_GAME_TIME` — `ticks = 0`.
///
/// # Wire layout (before encryption)
/// ```text
/// [0x58]          flags
/// [0x02]          BASEMSG_UPDATE_FREQUENCY_NOTIFICATION
/// [10]            updateFreq = 10  (CONSTANT_LENGTH 1)
/// [0x0D]          BASEMSG_TICK_SYNC
/// [0,0,0,0]       ticks = 0        (CONSTANT_LENGTH 8)
/// [100,0,0,0]     tickRate = 100
/// [0x03]          BASEMSG_SET_GAME_TIME
/// [0,0,0,0]       ticks = 0        (CONSTANT_LENGTH 4)
/// [seq_id: u32 LE] footer
/// ```
///
/// Total plaintext: 25 bytes → PKCS7-padded to 32 → encrypted → + 16 HMAC = 48 bytes.
pub fn build_time_sync(key: &[u8; 32], seq_id: u32) -> Vec<u8> {
    const UPDATE_FREQ: u8 = 10; // 1000 ms / 100 ticks/sec
    const TICK_RATE: u32 = 100;
    const TICKS: u32 = 0;

    let mut body = Vec::with_capacity(2 + 9 + 5);

    // BASEMSG_UPDATE_FREQUENCY_NOTIFICATION (CONSTANT_LENGTH = 1)
    body.push(BASEMSG_UPDATE_FREQUENCY_NOTIFICATION);
    body.push(UPDATE_FREQ);

    // BASEMSG_TICK_SYNC (CONSTANT_LENGTH = 8)
    body.push(BASEMSG_TICK_SYNC);
    body.extend_from_slice(&TICKS.to_le_bytes());
    body.extend_from_slice(&TICK_RATE.to_le_bytes());

    // BASEMSG_SET_GAME_TIME (CONSTANT_LENGTH = 4)
    body.push(BASEMSG_SET_GAME_TIME);
    body.extend_from_slice(&TICKS.to_le_bytes());

    let plaintext = build_outgoing(REPLY_FLAGS, &body, Some(seq_id), &[], None);

    encrypt_packet(&plaintext, key)
}

// ── Encryption helper ─────────────────────────────────────────────────────────

/// Encrypt a plaintext Mercury packet (flags + body + footers).
///
/// Matches the C++ `EncryptionFilter::sendMessage()`:
/// 1. PKCS7-pad the entire plaintext to a multiple of 16 bytes.
/// 2. AES-256-CBC encrypt with IV = 16 zero bytes.
/// 3. Append 16-byte HMAC-MD5 of the ciphertext (key = full 32-byte AES key).
fn encrypt_packet(plaintext: &[u8], key: &[u8; 32]) -> Vec<u8> {
    let enc = MercuryEncryption::from_session_key(*key);
    enc.encrypt(plaintext)
        .expect("Mercury packet encryption failed")
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_KEY: [u8; 32] = [0x42u8; 32];

    #[test]
    fn connect_reply_size() {
        // 35-byte plaintext → PKCS7 pad to 48 → AES encrypt → + 16 HMAC = 64 bytes.
        let ticket = b"12345678901234567890";
        let out = build_connect_reply(0xDEADBEEF, ticket, &TEST_KEY, 1);
        assert_eq!(out.len(), 64, "reply should be 64 bytes: 48 ciphertext + 16 HMAC");
    }

    #[test]
    fn time_sync_size() {
        // 25-byte plaintext → PKCS7 pad to 32 → AES encrypt → + 16 HMAC = 48 bytes.
        let out = build_time_sync(&TEST_KEY, 2);
        assert_eq!(out.len(), 48, "time sync should be 48 bytes: 32 ciphertext + 16 HMAC");
    }

    #[test]
    fn connect_reply_deterministic() {
        let ticket = b"AABBCCDDEEFF00112233";
        let a = build_connect_reply(1, ticket, &TEST_KEY, 1);
        let b = build_connect_reply(1, ticket, &TEST_KEY, 1);
        assert_eq!(a, b, "same inputs → same encrypted output (deterministic CBC with zero IV)");
    }

    #[test]
    fn time_sync_deterministic() {
        let a = build_time_sync(&TEST_KEY, 2);
        let b = build_time_sync(&TEST_KEY, 2);
        assert_eq!(a, b);
    }

    #[test]
    fn reply_and_time_sync_differ() {
        let ticket = b"12345678901234567890";
        let reply = build_connect_reply(0, ticket, &TEST_KEY, 1);
        let sync = build_time_sync(&TEST_KEY, 2);
        assert_ne!(reply, sync, "reply and time sync packets must differ");
    }
}
