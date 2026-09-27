//! The global GM broadcast fan-out (SS-C2, D-SS16): one
//! `onPlayerCommunication` to every online player.
//!
//! The cell has already authorized the GM and validated the text, and hands
//! over the finished payload (`ChatCellToBase::GmBroadcast`). This module
//! only decides who receives it and sends it: every session the online name
//! index lists ([`OnlinePlayerIndex`]: in the world since `onClientReady`,
//! not logged off) that has a player entity, addressed to that entity as it
//! is *now* (it changes across gate travel). Sessions at character select
//! or mid world entry are not listed, so they are not sent a method call on
//! an entity their client does not have yet.

use std::net::SocketAddr;
use std::sync::atomic::Ordering;

use super::feedback::FeedbackCtx;
use super::helpers::shadow_register_reliable_send;
use super::player_index::OnlinePlayerIndex;
use crate::mercury::{build_player_entity_method_packet, method_idx};

/// How one global broadcast went. Every failed send is already logged.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GmBroadcastReport {
    /// Listed sessions the line was sent to.
    pub delivered: usize,
    /// Listed sessions whose send failed on the socket.
    pub failed: usize,
    /// Listed sessions with no player entity to address.
    pub not_in_world: usize,
}

/// Send `args` (a serialized `onPlayerCommunication`) reliably to every
/// listed online player.
pub async fn broadcast_to_online_players(ctx: &FeedbackCtx<'_>, args: &[u8]) -> GmBroadcastReport {
    let mut report = GmBroadcastReport::default();

    // Snapshot the recipients and reserve each one's sequence number under
    // one lock; send after releasing it (never hold the map across await).
    let targets: Vec<(SocketAddr, u32, [u8; 32], _, u32, Vec<u32>)> = {
        let Ok(clients) = ctx.connected.lock() else {
            tracing::warn!(
                target: "chat",
                event = "chat.gm_broadcast_skipped",
                reason = "session_map_poisoned",
                "GM broadcast not sent: the session map lock is poisoned",
            );
            return report;
        };
        let index = OnlinePlayerIndex::new(&clients);
        let mut targets = Vec::new();
        for (_, player) in index.entries() {
            let Some(c) = clients.get(&player.addr) else {
                continue;
            };
            let Some(entity_id) = c.player_entity_id else {
                report.not_in_world += 1;
                continue;
            };
            let seq = c.next_seq.fetch_add(1, Ordering::Relaxed)
                & cimmeria_mercury::packet::SEQUENCE_MASK;
            let acks: Vec<u32> = c.pending_acks.lock().unwrap().drain(..).collect();
            targets.push((player.addr, entity_id, c.key, c.enc_version, seq, acks));
        }
        targets
    };

    for (addr, entity_id, key, version, seq, acks) in targets {
        let packet = build_player_entity_method_packet(
            &key,
            seq,
            &acks,
            entity_id,
            method_idx::ON_PLAYER_COMMUNICATION,
            args,
            version,
        );
        if let Err(e) = ctx.transport.send_to(&packet, addr).await {
            tracing::warn!(
                target: "chat",
                event = "chat.gm_broadcast_send_failed",
                %addr,
                entity_id,
                reason = "send_error",
                error = %e,
                "GM broadcast line send failed for one recipient",
            );
            report.failed += 1;
            continue;
        }
        shadow_register_reliable_send(
            ctx.connected,
            addr,
            seq,
            cimmeria_mercury::packet::Bytes::copy_from_slice(&packet),
        );
        report.delivered += 1;
    }
    report
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    use cimmeria_mercury::transport::Transport;
    use cimmeria_wire::cell::chat::{serialize_gm_broadcast, CHAN_SERVER, SPEAKER_GM};

    use super::*;
    use crate::test_support::{test_default_connected_client_state, TestTransport};

    /// Decrypt one packet (all-zero test key): `(entity_id, method args)`.
    fn decode(packet: &[u8]) -> (u32, Vec<u8>) {
        let enc = cimmeria_mercury::encryption::MercuryEncryption::from_session_key([0u8; 32]);
        let pt = enc.decrypt(packet).expect("decrypt test packet");
        let body = &pt[1..pt.len() - 4];
        let entity_id = u32::from_le_bytes(body[3..7].try_into().unwrap());
        (entity_id, body[7..].to_vec())
    }

    fn listed(
        entity_id: Option<u32>,
        player_id: i32,
        name: &str,
    ) -> crate::base::ConnectedClientState {
        let mut s = test_default_connected_client_state();
        s.player_entity_id = entity_id;
        s.active_player_id = Some(player_id);
        s.player_name = Some(name.to_string());
        s.listed_online = true;
        s
    }

    /// Type 8 fan-out: two listed in-world sessions each get exactly one
    /// packet carrying the GM line byte for byte, addressed to their own
    /// entity. A session at character select (not listed) and a listed
    /// session with no entity get nothing.
    #[tokio::test]
    async fn gm_broadcast_global_reaches_every_listed_session() {
        let a: SocketAddr = "127.0.0.1:54600".parse().unwrap();
        let b: SocketAddr = "127.0.0.1:54601".parse().unwrap();
        let char_select: SocketAddr = "127.0.0.1:54602".parse().unwrap();
        let no_entity: SocketAddr = "127.0.0.1:54603".parse().unwrap();
        let mut unlisted = test_default_connected_client_state();
        unlisted.player_entity_id = Some(9);
        let connected = Arc::new(Mutex::new(HashMap::from([
            (a, listed(Some(101), 1, "Alice")),
            (b, listed(Some(202), 2, "Bob")),
            (char_select, unlisted),
            (no_entity, listed(None, 3, "Carol")),
        ])));
        let test_transport = Arc::new(TestTransport::default());
        let transport: Arc<dyn Transport> = test_transport.clone();
        let ctx = FeedbackCtx {
            transport: &transport,
            connected: &connected,
        };
        let args = serialize_gm_broadcast("Gm", "Server restart in 5 minutes");

        let report = broadcast_to_online_players(&ctx, &args).await;

        assert_eq!(
            report,
            GmBroadcastReport {
                delivered: 2,
                failed: 0,
                not_in_world: 1
            }
        );
        for (addr, eid) in [(a, 101), (b, 202)] {
            let sent = test_transport.filter_to(addr);
            assert_eq!(sent.len(), 1, "exactly one line to {addr}");
            let (entity_id, got) = decode(&sent[0]);
            assert_eq!(entity_id, eid, "addressed to the recipient's own entity");
            assert_eq!(got, args, "the GM line, byte for byte");
            // Flag and channel: speaker "Gm" is 4 + 4 bytes.
            assert_eq!(got[8], SPEAKER_GM);
            assert_eq!(got[9], CHAN_SERVER);
        }
        assert!(test_transport.filter_to(char_select).is_empty());
        assert!(test_transport.filter_to(no_entity).is_empty());
        for addr in [a, b] {
            let in_flight = connected.lock().unwrap()[&addr]
                .channel
                .lock()
                .unwrap()
                .tx_window
                .len();
            assert_eq!(in_flight, 1, "the broadcast is reliable");
        }
    }
}
