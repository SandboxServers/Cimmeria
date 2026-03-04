//! Helpers for building encrypted Mercury packets from the BaseApp server side.
//!
//! These functions produce byte-identical wire output to the C++ BaseApp:
//! - [`build_connect_reply`]           — `BASEMSG_REPLY_MESSAGE` echoing the ticket back (Phase 3).
//! - [`build_time_sync`]               — three time-sync messages in one packet (Phase 3).
//! - [`build_char_list_response`]      — game-state + empty character list (Phase 4, char creation).
//! - [`build_char_list_with_character`] — game-state + 1-char list (Phase 4, char select).
//! - [`build_ongoing_tick_sync`]       — single tick-sync for the 100 ms heartbeat.
//! - [`build_world_entry`]             — createBasePlayer + viewport + cell + position (Phase 5).
//!
//! Each function returns a `Vec<u8>` ready to write to the UDP socket.

use cimmeria_mercury::encryption::MercuryEncryption;
use cimmeria_mercury::packet::{
    build_outgoing, FLAG_HAS_ACKS, FLAG_HAS_SEQUENCE, FLAG_ON_CHANNEL, FLAG_RELIABLE,
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
/// `BASEMSG_CREATE_BASE_PLAYER` — create a base entity on the client (0x05).
/// Wire format: `[entityID:u32][classID:u8][propertyCount:u8]`.
const BASEMSG_CREATE_BASE_PLAYER: u8 = 0x05;
/// Base entity method: `onCharacterList` (msg_id = 0x80 + methodId 2 = 0x82).
/// Wire format: `[entityID:u32][ARRAY<CharacterInfo>]`.
const BASEMSG_ON_CHARACTER_LIST: u8 = 0x82;

/// `BASEMSG_SPACE_VIEWPORT_INFO` — CME-custom space/viewport setup (0x08).
/// Wire format: `[entityID:u32][entityID2:u32][spaceID:u32][viewportID:u8]`.
const BASEMSG_SPACE_VIEWPORT_INFO: u8 = 0x08;
/// `BASEMSG_CREATE_CELL_PLAYER` — create cell entity with position (0x06).
/// Wire format: `[spaceID:u32][vehicleID:u32][pos:3×f32][rot:3×f32]`.
const BASEMSG_CREATE_CELL_PLAYER: u8 = 0x06;
/// `BASEMSG_FORCED_POSITION` — authoritative position set (0x31).
/// Wire format: `[entityID:u32][spaceID:u32][vehicleID:u32][pos:3×f32][vel:3×f32][rot:3×f32][flags:u8]`.
const BASEMSG_FORCED_POSITION: u8 = 0x31;

/// Account entity ID (assigned by the server on login).
const ACCOUNT_ENTITY_ID: u32 = 2;
/// Account entity class ID (EntityTypeID 7 in entity definitions).
const ACCOUNT_CLASS_ID: u8 = 0x07;
/// Player entity ID (first non-Account entity, assigned on character play).
const PLAYER_ENTITY_ID: u32 = 1;
/// SGWPlayer entity class ID (EntityTypeID 2 in entity definitions).
const SGWPLAYER_CLASS_ID: u8 = 0x02;
/// Default space ID for CombatSim (matches reference server session).
const DEFAULT_SPACE_ID: u32 = 65553;

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

/// Build and encrypt the Phase 4 character list response packet.
///
/// Sent immediately after the client confirms login with msg_id=0x01.
/// Contains two messages:
///
/// 1. `BASEMSG_CREATE_BASE_PLAYER` (0x05, WORD_LENGTH=6) — creates the Account
///    entity (entityID=2, classID=7) on the client.
///
/// 2. `BASEMSG_ON_CHARACTER_LIST` (0x82, WORD_LENGTH=8) — base entity method
///    `onCharacterList` (methodId=2) on entity 2.  We send `count=0` (no
///    characters) which triggers the character-creation screen.
///
/// # Wire layout (before encryption)
/// ```text
/// [0x58|0x5C]           flags (0x5C if acks present)
/// [0x05]                BASEMSG_CREATE_BASE_PLAYER
/// [06, 00]              WORD_LENGTH = 6
/// [02, 00, 00, 00]      entityID = 2
/// [07]                  classID = 7 (Account)
/// [00]                  propertyCount = 0
/// [0x82]                BASEMSG_ON_CHARACTER_LIST
/// [08, 00]              WORD_LENGTH = 8
/// [02, 00, 00, 00]      entityID = 2
/// [00, 00, 00, 00]      count = 0 characters
/// [seq_id: u32 LE]      footer: sequence number
/// [ack_seq: u32 LE]...  footer: acks (if any)
/// [ack_count: u8]       footer: ack count (if any)
/// ```
pub fn build_char_list_response(key: &[u8; 32], seq_id: u32, acks: &[u32]) -> Vec<u8> {
    let mut body = Vec::with_capacity(20);

    // BASEMSG_CREATE_BASE_PLAYER (WORD_LENGTH = 6)
    body.push(BASEMSG_CREATE_BASE_PLAYER);
    body.extend_from_slice(&6u16.to_le_bytes());
    body.extend_from_slice(&ACCOUNT_ENTITY_ID.to_le_bytes()); // entityID = 2
    body.push(ACCOUNT_CLASS_ID); // classID = 7
    body.push(0x00); // propertyCount = 0

    // Base entity method: onCharacterList (WORD_LENGTH = 8: entityID + count)
    body.push(BASEMSG_ON_CHARACTER_LIST);
    body.extend_from_slice(&8u16.to_le_bytes());
    body.extend_from_slice(&ACCOUNT_ENTITY_ID.to_le_bytes()); // entityID = 2
    body.extend_from_slice(&0u32.to_le_bytes()); // 0 characters → char creation screen

    let flags = REPLY_FLAGS | if acks.is_empty() { 0 } else { FLAG_HAS_ACKS };
    let plaintext = build_outgoing(flags, &body, Some(seq_id), acks, None);
    encrypt_packet(&plaintext, key)
}

/// Build and encrypt a single `BASEMSG_TICK_SYNC` heartbeat packet.
///
/// Sent every 100 ms after Phase 4 to keep the client's connection alive.
/// Unlike [`build_time_sync`] (which sends the full init bundle), this sends
/// only the tick-sync message so the client can update its clock.
///
/// If `acks` is non-empty, the packet piggybacks acknowledgements for the
/// client's reliable messages (avoids sending separate ACK packets).
///
/// # Wire layout (before encryption)
/// ```text
/// [0x58|0x5C]         flags (0x5C if acks present)
/// [0x0D]              BASEMSG_TICK_SYNC  (CONSTANT_LENGTH = 8)
/// [tick: u32 LE]      current tick counter (increments each 100 ms)
/// [100, 0, 0, 0]      tickRate = 100 ticks/sec
/// [seq_id: u32 LE]    footer
/// [ack_seq: u32]...   footer: acks (if any)
/// [ack_count: u8]     footer: ack count (if any)
/// ```
pub fn build_ongoing_tick_sync(key: &[u8; 32], seq_id: u32, tick: u32, acks: &[u32]) -> Vec<u8> {
    const TICK_RATE: u32 = 100;

    let mut body = Vec::with_capacity(9);
    body.push(BASEMSG_TICK_SYNC);
    body.extend_from_slice(&tick.to_le_bytes());
    body.extend_from_slice(&TICK_RATE.to_le_bytes());

    let flags = REPLY_FLAGS | if acks.is_empty() { 0 } else { FLAG_HAS_ACKS };
    let plaintext = build_outgoing(flags, &body, Some(seq_id), acks, None);
    encrypt_packet(&plaintext, key)
}

/// Build and encrypt the Phase 4 character list with one hardcoded character.
///
/// Same structure as [`build_char_list_response`] but sends `count=1` with a
/// pre-built `CharacterInfo` so the client shows a character select screen
/// instead of the character creation screen.
///
/// # Wire layout (character list portion, before encryption)
/// ```text
/// [0x82]                  BASEMSG_ON_CHARACTER_LIST
/// [WORD_LENGTH: u16 LE]   payload length
/// [entityID: u32 = 2]     Account entity
/// [count: u32 = 1]        one character
/// [CharacterInfo...]      FIXED_DICT fields in order
/// ```
pub fn build_char_list_with_character(key: &[u8; 32], seq_id: u32, acks: &[u32]) -> Vec<u8> {
    let mut body = Vec::with_capacity(128);

    // BASEMSG_CREATE_BASE_PLAYER (WORD_LENGTH = 6)
    body.push(BASEMSG_CREATE_BASE_PLAYER);
    body.extend_from_slice(&6u16.to_le_bytes());
    body.extend_from_slice(&ACCOUNT_ENTITY_ID.to_le_bytes()); // entityID = 2
    body.push(ACCOUNT_CLASS_ID); // classID = 7
    body.push(0x00); // propertyCount = 0

    // BASEMSG_ON_CHARACTER_LIST with 1 character
    body.push(BASEMSG_ON_CHARACTER_LIST);

    // Build payload: [entityID][ARRAY<CharacterInfo>]
    let mut payload = Vec::with_capacity(80);
    payload.extend_from_slice(&ACCOUNT_ENTITY_ID.to_le_bytes()); // entityID = 2

    // CharacterInfoList = ARRAY<CharacterInfo> with count = 1
    payload.extend_from_slice(&1u32.to_le_bytes()); // count = 1

    // CharacterInfo FIXED_DICT (fields in alias.xml declaration order):
    payload.extend_from_slice(&1i32.to_le_bytes()); // playerId = 1
    write_wstring(&mut payload, "Wanderer"); // name
    write_wstring(&mut payload, ""); // extraName
    payload.push(0); // alignment (0 = default)
    payload.push(1); // level = 1
    payload.push(0); // gender (0 = male)
    write_wstring(&mut payload, "CombatSim"); // worldLocation
    payload.push(0); // archetype (0 = Soldier)
    payload.push(0); // title (0 = none)
    payload.extend_from_slice(&0i32.to_le_bytes()); // playerType = 0
    payload.push(1); // playable = 1

    body.extend_from_slice(&(payload.len() as u16).to_le_bytes()); // WORD_LENGTH
    body.extend_from_slice(&payload);

    let flags = REPLY_FLAGS | if acks.is_empty() { 0 } else { FLAG_HAS_ACKS };
    let plaintext = build_outgoing(flags, &body, Some(seq_id), acks, None);
    encrypt_packet(&plaintext, key)
}

/// Build and encrypt the Phase 5 world entry packet.
///
/// Sent when the client calls `playCharacter`. Contains four messages packed
/// into one reliable packet:
///
/// 1. `createBasePlayer` (0x05) — creates the SGWPlayer base entity.
/// 2. `spaceViewportInfo` (0x08) — CME viewport setup for the target space.
/// 3. `createCellPlayer` (0x06) — creates the cell entity with position.
/// 4. `forcedPosition` (0x31) — sets authoritative position + velocity.
///
/// # Hardcoded defaults
/// - Entity ID = 1 (first non-Account entity)
/// - Class ID = 2 (SGWPlayer)
/// - Position/rotation = (0, 0, 0) — adjust once proper maps are loaded
/// - Velocity = (0, 0, 0)
pub fn build_world_entry(key: &[u8; 32], seq_id: u32, acks: &[u32]) -> Vec<u8> {
    let pos = [0.0f32, 0.0, 0.0]; // starting position
    let rot = [0.0f32, 0.0, 0.0]; // rotation (rotX, rotZ, rotY — Z/Y swapped)

    let mut body = Vec::with_capacity(128);

    // 1. createBasePlayer (0x05, WORD_LENGTH = 6)
    body.push(BASEMSG_CREATE_BASE_PLAYER);
    body.extend_from_slice(&6u16.to_le_bytes());
    body.extend_from_slice(&PLAYER_ENTITY_ID.to_le_bytes()); // entityID = 1
    body.push(SGWPLAYER_CLASS_ID); // classID = 2
    body.push(0x00); // propertyCount = 0

    // 2. spaceViewportInfo (0x08, CONSTANT_LENGTH = 13)
    body.push(BASEMSG_SPACE_VIEWPORT_INFO);
    body.extend_from_slice(&PLAYER_ENTITY_ID.to_le_bytes()); // entityID
    body.extend_from_slice(&PLAYER_ENTITY_ID.to_le_bytes()); // entityID2
    body.extend_from_slice(&DEFAULT_SPACE_ID.to_le_bytes()); // spaceID
    body.push(0x00); // viewportID = 0

    // 3. createCellPlayer (0x06, WORD_LENGTH = 32)
    body.push(BASEMSG_CREATE_CELL_PLAYER);
    body.extend_from_slice(&32u16.to_le_bytes());
    body.extend_from_slice(&DEFAULT_SPACE_ID.to_le_bytes()); // spaceID
    body.extend_from_slice(&0u32.to_le_bytes()); // vehicleID = 0
    for &c in &pos {
        body.extend_from_slice(&c.to_le_bytes());
    } // position x, y, z
    for &c in &rot {
        body.extend_from_slice(&c.to_le_bytes());
    } // rotation rotX, rotZ, rotY

    // 4. forcedPosition (0x31, CONSTANT_LENGTH = 49)
    body.push(BASEMSG_FORCED_POSITION);
    body.extend_from_slice(&PLAYER_ENTITY_ID.to_le_bytes()); // entityID
    body.extend_from_slice(&DEFAULT_SPACE_ID.to_le_bytes()); // spaceID
    body.extend_from_slice(&0u32.to_le_bytes()); // vehicleID = 0
    for &c in &pos {
        body.extend_from_slice(&c.to_le_bytes());
    } // position
    for &c in &[0.0f32, 0.0, 0.0] {
        body.extend_from_slice(&c.to_le_bytes());
    } // velocity (all zero)
    for &c in &rot {
        body.extend_from_slice(&c.to_le_bytes());
    } // rotation
    body.push(0x01); // flags

    let flags = REPLY_FLAGS | if acks.is_empty() { 0 } else { FLAG_HAS_ACKS };
    let plaintext = build_outgoing(flags, &body, Some(seq_id), acks, None);
    encrypt_packet(&plaintext, key)
}

// ── Serialization helpers ────────────────────────────────────────────────────

/// Write a BigWorld `WSTRING` to a buffer.
///
/// Wire format: `[char_count: u32 LE][UTF-16LE data: char_count × 2 bytes]`.
fn write_wstring(buf: &mut Vec<u8>, s: &str) {
    let chars: Vec<u16> = s.encode_utf16().collect();
    buf.extend_from_slice(&(chars.len() as u32).to_le_bytes());
    for &ch in &chars {
        buf.extend_from_slice(&ch.to_le_bytes());
    }
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

    #[test]
    fn char_list_response_size_no_acks() {
        // body=20B, flags(1)+body(20)+seq(4)=25B → pad to 32 → +16 HMAC = 48B.
        let out = build_char_list_response(&TEST_KEY, 3, &[]);
        assert_eq!(out.len(), 48, "char list response should be 48 bytes");
    }

    #[test]
    fn char_list_response_with_ack() {
        // body=20B, flags(1)+body(20)+seq(4)+ack(4)+count(1)=30B → pad to 32 → +16 HMAC = 48B.
        let out = build_char_list_response(&TEST_KEY, 3, &[0]);
        assert_eq!(out.len(), 48, "char list with 1 ack should be 48 bytes");
    }

    #[test]
    fn char_list_response_deterministic() {
        let a = build_char_list_response(&TEST_KEY, 3, &[]);
        let b = build_char_list_response(&TEST_KEY, 3, &[]);
        assert_eq!(a, b);
    }

    #[test]
    fn ongoing_tick_sync_size() {
        // body=9B, flags(1)+body(9)+seq(4)=14B → pad to 16 → +16 HMAC = 32B.
        let out = build_ongoing_tick_sync(&TEST_KEY, 4, 0, &[]);
        assert_eq!(out.len(), 32, "tick sync should be 32 bytes");
    }

    #[test]
    fn ongoing_tick_sync_with_acks() {
        // body=9B, flags(1)+body(9)+seq(4)+ack(4)+count(1)=19B → pad to 32 → +16 HMAC = 48B.
        let out = build_ongoing_tick_sync(&TEST_KEY, 4, 0, &[0]);
        assert_eq!(out.len(), 48, "tick sync with 1 ack should be 48 bytes");
    }

    #[test]
    fn ongoing_tick_sync_changes_with_tick() {
        let a = build_ongoing_tick_sync(&TEST_KEY, 4, 0, &[]);
        let b = build_ongoing_tick_sync(&TEST_KEY, 4, 1, &[]);
        assert_ne!(a, b, "different tick values must produce different ciphertexts");
    }

    #[test]
    fn ongoing_tick_sync_changes_with_seq() {
        let a = build_ongoing_tick_sync(&TEST_KEY, 4, 0, &[]);
        let b = build_ongoing_tick_sync(&TEST_KEY, 5, 0, &[]);
        assert_ne!(a, b, "different seq_ids must produce different ciphertexts");
    }
}
