//! The one single-recipient feedback line: `onPlayerCommunication("SYSTEM",
//! 0, CHAN_FEEDBACK, text)` to one player's own client.
//!
//! Every "not supported", "not online", "not accepting your messages" and
//! "too quickly" line the base sends goes through [`send_feedback_line`], and
//! the GM confirmations in [`super::gm_feedback`] ride the same send. The
//! payload is the one shared serializer, `cimmeria_wire::cell::chat`.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;

use super::helpers::shadow_register_reliable_send;
use super::ConnectedClientState;
use crate::mercury::{build_player_entity_method_packet, method_idx};
pub use cimmeria_wire::cell::chat::{serialize_on_player_communication, CHAN_FEEDBACK};

/// Speaker name on every feedback line.
pub const FEEDBACK_SPEAKER: &str = "SYSTEM";

/// What the caller needs to reach a session: the socket and the session map.
#[derive(Clone, Copy)]
pub struct FeedbackCtx<'a> {
    pub transport: &'a Arc<dyn Transport>,
    pub connected: &'a Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
}

/// How one feedback send ended. Every non-`Sent` outcome is already logged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeedbackOutcome {
    Sent,
    /// No session at the address: it disconnected first. Harmless.
    NoSession,
    /// The session has no player entity (character select, mid world
    /// entry), so there is nothing to address `onPlayerCommunication` to.
    NotInWorld,
    SendError,
}

/// Send one feedback line to the player at `addr`, addressed to the
/// session's current player entity (read now, because it changes across
/// gate travel). Reliable: registered with the session's channel for
/// retransmit.
pub async fn send_feedback_line(
    ctx: &FeedbackCtx<'_>,
    addr: SocketAddr,
    text: &str,
) -> FeedbackOutcome {
    let entity_id = {
        let Ok(clients) = ctx.connected.lock() else {
            return FeedbackOutcome::NoSession;
        };
        match clients.get(&addr) {
            None => None,
            Some(c) => match c.player_entity_id {
                Some(eid) => Some(eid),
                None => {
                    tracing::warn!(
                        %addr,
                        reason = "not_in_world",
                        "feedback line dropped: session has no player entity",
                    );
                    return FeedbackOutcome::NotInWorld;
                }
            },
        }
    };
    let Some(entity_id) = entity_id else {
        tracing::debug!(
            %addr,
            reason = "no_session",
            "feedback line dropped: client disconnected first",
        );
        return FeedbackOutcome::NoSession;
    };
    send_feedback_to_entity(ctx, addr, entity_id, text).await
}

/// [`send_feedback_line`] with the entity id already known: the GM
/// confirmations resolve the GM's entity themselves.
pub(crate) async fn send_feedback_to_entity(
    ctx: &FeedbackCtx<'_>,
    addr: SocketAddr,
    entity_id: u32,
    text: &str,
) -> FeedbackOutcome {
    let payload = serialize_on_player_communication(FEEDBACK_SPEAKER, 0, CHAN_FEEDBACK, text);
    send_player_method(
        ctx,
        addr,
        entity_id,
        method_idx::ON_PLAYER_COMMUNICATION,
        &payload,
    )
    .await
}

/// Send one reliable client method on the player entity `entity_id` to the
/// session at `addr`: the transport under every feedback line. Failures are
/// logged here.
pub async fn send_player_method(
    ctx: &FeedbackCtx<'_>,
    addr: SocketAddr,
    entity_id: u32,
    method_index: u16,
    payload: &[u8],
) -> FeedbackOutcome {
    send_method(
        ctx,
        addr,
        Addressee::Entity(entity_id),
        method_index,
        payload,
    )
    .await
    .0
}

/// Send one reliable client method to whatever player entity the session at
/// `addr` has **now**, provided it still plays `player_id`. The entity id is
/// read under the same lock that allocates the sequence number, so a gate
/// travel between a caller's lookup and this send cannot address a stale
/// entity (the tell path, PR #893 review). Returns the entity it used.
///
/// `NotInWorld` when the session plays another character or has no player
/// entity at that moment; `NoSession` when it is gone.
pub async fn send_to_current_player(
    ctx: &FeedbackCtx<'_>,
    addr: SocketAddr,
    player_id: i32,
    method_index: u16,
    payload: &[u8],
) -> (FeedbackOutcome, Option<u32>) {
    send_method(
        ctx,
        addr,
        Addressee::CurrentOf(player_id),
        method_index,
        payload,
    )
    .await
}

/// Which player entity a send addresses.
#[derive(Debug, Clone, Copy)]
enum Addressee {
    /// A caller-supplied entity id.
    Entity(u32),
    /// The session's current `player_entity_id`, if it still plays this
    /// `player_id`.
    CurrentOf(i32),
}

async fn send_method(
    ctx: &FeedbackCtx<'_>,
    addr: SocketAddr,
    who: Addressee,
    method_index: u16,
    payload: &[u8],
) -> (FeedbackOutcome, Option<u32>) {
    let session = {
        let Ok(clients) = ctx.connected.lock() else {
            return (FeedbackOutcome::NoSession, None);
        };
        match clients.get(&addr) {
            None => None,
            Some(c) => {
                let entity_id = match who {
                    Addressee::Entity(e) => e,
                    Addressee::CurrentOf(player_id) => {
                        let current = (c.active_player_id == Some(player_id))
                            .then_some(c.player_entity_id)
                            .flatten();
                        let Some(e) = current else {
                            tracing::debug!(
                                %addr,
                                player_id,
                                account_id = c.account_id,
                                session_player_id = c.active_player_id,
                                method_index,
                                reason = if c.active_player_id == Some(player_id) {
                                    "not_in_world"
                                } else {
                                    "player_changed"
                                },
                                "player method dropped: the session no longer has that player in the world",
                            );
                            return (FeedbackOutcome::NotInWorld, None);
                        };
                        e
                    }
                };
                let seq = c.next_seq.fetch_add(1, Ordering::Relaxed)
                    & cimmeria_mercury::packet::SEQUENCE_MASK;
                let acks: Vec<u32> = c.pending_acks.lock().unwrap().drain(..).collect();
                Some((entity_id, c.key, c.enc_version, seq, acks))
            }
        }
    };
    let Some((entity_id, key, version, seq, acks)) = session else {
        tracing::debug!(
            %addr,
            ?who,
            method_index,
            reason = "no_session",
            "player method dropped: client disconnected first",
        );
        return (FeedbackOutcome::NoSession, None);
    };

    let packet = build_player_entity_method_packet(
        &key,
        seq,
        &acks,
        entity_id,
        method_index,
        payload,
        version,
    );
    if let Err(e) = ctx.transport.send_to(&packet, addr).await {
        tracing::warn!(
            %addr,
            entity_id,
            method_index,
            reason = "send_error",
            error = %e,
            "player method send failed",
        );
        return (FeedbackOutcome::SendError, Some(entity_id));
    }
    shadow_register_reliable_send(
        ctx.connected,
        addr,
        seq,
        cimmeria_mercury::packet::Bytes::copy_from_slice(&packet),
    );
    (FeedbackOutcome::Sent, Some(entity_id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{test_default_connected_client_state, TestTransport};

    /// Feedback wire shape: speaker "SYSTEM" (6 UTF-16 chars), flags 0,
    /// channel 9, then the text WSTRING. A drift here means the client
    /// renders the feedback on the wrong channel or not at all.
    #[test]
    fn feedback_wire_shape_is_system_on_feedback_channel() {
        let args = serialize_on_player_communication(FEEDBACK_SPEAKER, 0, CHAN_FEEDBACK, "hello");
        let mut expected = Vec::new();
        expected.extend_from_slice(&6u32.to_le_bytes());
        for ch in "SYSTEM".encode_utf16() {
            expected.extend_from_slice(&ch.to_le_bytes());
        }
        expected.extend_from_slice(&[0x00, 0x09]);
        expected.extend_from_slice(&5u32.to_le_bytes());
        for ch in "hello".encode_utf16() {
            expected.extend_from_slice(&ch.to_le_bytes());
        }
        assert_eq!(args, expected);
        assert_eq!(
            CHAN_FEEDBACK, 9,
            "feedback must use the tell channel, not 8"
        );
    }

    /// Decrypt a feedback packet (all-zero test key) and return the
    /// addressed entity id and the text.
    pub(crate) fn decode_feedback(packet: &[u8]) -> (u32, String) {
        let enc = cimmeria_mercury::encryption::MercuryEncryption::from_session_key([0u8; 32]);
        let pt = enc.decrypt(packet).expect("decrypt test packet");
        let body = &pt[1..pt.len() - 4];
        let entity_id = u32::from_le_bytes(body[3..7].try_into().unwrap());
        let args = &body[7..];
        let speaker_len = u32::from_le_bytes(args[0..4].try_into().unwrap()) as usize;
        let mut offset = 4 + speaker_len * 2 + 2;
        let text_len = u32::from_le_bytes(args[offset..offset + 4].try_into().unwrap()) as usize;
        offset += 4;
        let units: Vec<u16> = (0..text_len)
            .map(|i| {
                u16::from_le_bytes(args[offset + i * 2..offset + i * 2 + 2].try_into().unwrap())
            })
            .collect();
        (entity_id, String::from_utf16(&units).unwrap())
    }

    #[tokio::test]
    async fn send_feedback_line_reaches_the_session_entity_and_registers_reliable() {
        let addr: SocketAddr = "127.0.0.1:54500".parse().unwrap();
        let mut state = test_default_connected_client_state();
        state.player_entity_id = Some(4242);
        let connected = Arc::new(Mutex::new(HashMap::from([(addr, state)])));
        let test_transport = Arc::new(TestTransport::default());
        let transport: Arc<dyn Transport> = test_transport.clone();
        let ctx = FeedbackCtx {
            transport: &transport,
            connected: &connected,
        };

        assert_eq!(
            send_feedback_line(&ctx, addr, "You are sending messages too quickly.").await,
            FeedbackOutcome::Sent
        );
        let sent = test_transport.filter_to(addr);
        assert_eq!(sent.len(), 1);
        assert_eq!(
            decode_feedback(&sent[0]),
            (4242, "You are sending messages too quickly.".to_string())
        );
        let in_flight = connected.lock().unwrap()[&addr]
            .channel
            .lock()
            .unwrap()
            .tx_window
            .len();
        assert_eq!(
            in_flight, 1,
            "feedback is reliable: it must be in the TX window"
        );
    }

    #[tokio::test]
    async fn send_feedback_line_without_a_player_entity_sends_nothing() {
        let addr: SocketAddr = "127.0.0.1:54501".parse().unwrap();
        let connected = Arc::new(Mutex::new(HashMap::from([(
            addr,
            test_default_connected_client_state(),
        )])));
        let test_transport = Arc::new(TestTransport::default());
        let transport: Arc<dyn Transport> = test_transport.clone();
        let ctx = FeedbackCtx {
            transport: &transport,
            connected: &connected,
        };
        assert_eq!(
            send_feedback_line(&ctx, addr, "x").await,
            FeedbackOutcome::NotInWorld
        );
        let other: SocketAddr = "127.0.0.1:54502".parse().unwrap();
        assert_eq!(
            send_feedback_line(&ctx, other, "x").await,
            FeedbackOutcome::NoSession
        );
        assert!(test_transport.is_empty());
    }
}
