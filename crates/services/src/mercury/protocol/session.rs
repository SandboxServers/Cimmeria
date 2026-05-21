//! Session lifecycle builders: connect reply, time sync, tick heartbeat,
//! entity reset, and logged-off.
//!
//! These are the packets that bracket a player's session — login handshake at
//! the start, periodic tick sync during play, and the teardown signals when
//! the server closes the session.

use cimmeria_mercury::packet::FLAG_HAS_ACKS;

use super::{
    encrypt_packet, BASEMSG_LOGGED_OFF, BASEMSG_REPLY_MESSAGE, BASEMSG_RESET_ENTITIES,
    BASEMSG_SET_GAME_TIME, BASEMSG_TICK_SYNC, BASEMSG_UPDATE_FREQUENCY_NOTIFICATION, REPLY_FLAGS,
    REPLY_FLAGS_UNRELIABLE,
};

/// Build and encrypt the `BASEMSG_REPLY_MESSAGE` packet.
///
/// This is the server's response to the `baseAppLogin` connect request.
/// The reply echoes `request_id` and the 20-byte `ticket` back to the client
/// so it can verify the server is legitimate.
pub fn build_connect_reply(request_id: u32, ticket: &[u8], key: &[u8; 32], seq_id: u32) -> Vec<u8> {
    use cimmeria_mercury::packet::build_outgoing;

    assert_eq!(ticket.len(), 20, "ticket must be exactly 20 bytes");

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
/// `ClientHandler::onConnected()` sequence.
pub fn build_time_sync(key: &[u8; 32], seq_id: u32) -> Vec<u8> {
    use cimmeria_mercury::packet::build_outgoing;

    const UPDATE_FREQ: u8 = 10;
    const TICK_RATE: u32 = 100;
    const TICKS: u32 = 0;

    let mut body = Vec::with_capacity(2 + 9 + 5);

    body.push(BASEMSG_UPDATE_FREQUENCY_NOTIFICATION);
    body.push(UPDATE_FREQ);

    body.push(BASEMSG_TICK_SYNC);
    body.extend_from_slice(&TICKS.to_le_bytes());
    body.extend_from_slice(&TICK_RATE.to_le_bytes());

    body.push(BASEMSG_SET_GAME_TIME);
    body.extend_from_slice(&TICKS.to_le_bytes());

    let plaintext = build_outgoing(REPLY_FLAGS, &body, Some(seq_id), &[], None);
    encrypt_packet(&plaintext, key)
}

/// Build and encrypt a single `BASEMSG_TICK_SYNC` heartbeat packet.
///
/// **Unreliable, on the dedicated unreliable seq counter.** Tick sync
/// rides the per-session `next_seq_unreliable` stream on
/// `ConnectedClientState`, separate from the reliable counter that
/// carries application packets. The SGW BigWorld client's recv-side
/// `UnAckedHandler::queueAckForPacket` (`ghidra://SGW.exe@0x0158cba0`)
/// only advances `inSeqAt` on reliable arrivals; unreliable packets go
/// through a separate dedup path (`FUN_0158bb50`) that never touches
/// `inSeqAt`. Because tickSync is on its own counter, it cannot consume
/// slots in the reliable seq stream — so the client's reliable-stream
/// expectation of contiguity (a lost reliable seq stalls all subsequent
/// reliable delivery) is satisfied regardless of how many ticks fire.
///
/// Why not reliable: tickSync fires at 10 Hz. The world-entry burst sends
/// ~27–30 reliable packets before the first ACK returns; any concurrent
/// reliable tickSync emission during that window would push the in-flight
/// count past the 32-slot TX-window cap and back-pressure downstream emits.
/// the next tick 100 ms later, on the next unreliable seq — the
/// receiver's unreliable dedup window absorbs occasional gaps and the
/// `gameTime` field self-corrects on the next successful arrival.
///
/// **ACK piggybacking on an unreliable packet is intentional and safe.**
/// ACKs are idempotent on the receiver (its cumulative-ack bitmap absorbs
/// duplicates), and if a tickSync carrying ACKs is lost, the next
/// tickSync 100 ms later carries any still-pending ACKs (they re-queue
/// on the server until the round-trip clears them). Meanwhile the
/// retransmit driver keeps re-sending the original reliable packet if
/// its RTO fires before any ACK arrives, so the un-acked sender doesn't
/// stall on a lost tickSync. The unreliable channel is fine for ACK
/// transport precisely because ACKs are self-correcting on both sides.
///
/// See `spec.protocol.mercury-wire-format` §1.7 + the disassembly of
/// `queueAckForPacket` for the receiver model.
pub fn build_ongoing_tick_sync(key: &[u8; 32], seq_id: u32, tick: u32, acks: &[u32]) -> Vec<u8> {
    use cimmeria_mercury::packet::build_outgoing;

    const TICK_RATE: u32 = 100;

    let mut body = Vec::with_capacity(9);
    body.push(BASEMSG_TICK_SYNC);
    body.extend_from_slice(&tick.to_le_bytes());
    body.extend_from_slice(&TICK_RATE.to_le_bytes());

    let flags = REPLY_FLAGS_UNRELIABLE | if acks.is_empty() { 0 } else { FLAG_HAS_ACKS };
    let plaintext = build_outgoing(flags, &body, Some(seq_id), acks, None);
    encrypt_packet(&plaintext, key)
}

/// Build and encrypt the entity teardown step: RESET_ENTITIES only.
///
/// The C++ server sends RESET_ENTITIES in its own flushed bundle, separate from
/// the cell/viewport data.  The client tears down all entities, then sends
/// ENABLE_ENTITIES, at which point the create-player step fires.
pub fn build_reset_entities(key: &[u8; 32], seq_id: u32, acks: &[u32]) -> Vec<u8> {
    use cimmeria_mercury::packet::build_outgoing;

    let mut body = Vec::with_capacity(4);
    body.push(BASEMSG_RESET_ENTITIES);
    body.push(0x00); // keepBase = false

    let flags = REPLY_FLAGS | if acks.is_empty() { 0 } else { FLAG_HAS_ACKS };
    let plaintext = build_outgoing(flags, &body, Some(seq_id), acks, None);
    encrypt_packet(&plaintext, key)
}

/// Build and encrypt a `LOGGED_OFF` message (0x37, CONSTANT_LENGTH = 1).
///
/// Sent when the client calls `logOff` (0xC2) — tells the client the server
/// has terminated the session.  The client calls `EntityManager::loggedOff()`
/// which does a partial cleanup (clears game entities, keeps login-screen
/// entities) and then tears down the Mercury channel.
///
/// C++ reference: `client_handler.cpp:461` — `BASEMSG_LOGGED_OFF` with
/// `reason = 0` followed by `flushBundle` + `channel->condemn()`.
pub fn build_logged_off(key: &[u8; 32], seq_id: u32, acks: &[u32]) -> Vec<u8> {
    use cimmeria_mercury::packet::build_outgoing;

    let body = vec![
        BASEMSG_LOGGED_OFF,
        0x00, // reason = 0 (normal logoff)
    ];

    let flags = REPLY_FLAGS | if acks.is_empty() { 0 } else { FLAG_HAS_ACKS };
    let plaintext = build_outgoing(flags, &body, Some(seq_id), acks, None);
    encrypt_packet(&plaintext, key)
}
