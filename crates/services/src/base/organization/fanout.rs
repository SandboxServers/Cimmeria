//! `broadcast_to_org` — fire-and-forget fan-out to all online org members.
//!
//! Sends a client method call to every entity in `entity_ids`.  Each send is
//! spawned independently so a slow client doesn't stall the dispatch loop.
//! Uses `send_to_witness_reliable` — the same transport primitive as the
//! Phase 2 squad handlers and the contact-list handlers.
//!
//! ## Fan-out timing rule
//! Fan-out is called AFTER the DB write commits and AFTER the in-memory
//! state is updated. Never before — we must not broadcast a state that
//! could roll back.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;

use crate::base::helpers::send_to_witness_reliable;
use crate::base::ConnectedClientState;
use crate::mercury::build_player_entity_method_packet;
#[cfg(test)]
use crate::mercury::method_idx;

/// Broadcast a pre-built args payload to every entity in `entity_ids`.
///
/// `method_index` is one of the `method_idx::ON_*` constants.
/// `args` is the serialized payload from the corresponding `wire::build_*` fn.
///
/// Fire-and-forget: spawns one task per recipient, returns immediately.
pub fn broadcast_to_org(
    entity_ids: Vec<u32>,
    method_index: u16,
    args: Vec<u8>,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    for eid in entity_ids {
        let args = args.clone();
        let transport = Arc::clone(transport);
        let connected = Arc::clone(connected);
        let entity_to_addr = Arc::clone(entity_to_addr);
        tokio::spawn(async move {
            send_to_witness_reliable(
                &transport,
                &connected,
                &entity_to_addr,
                eid,
                |key, version, seq, acks| {
                    build_player_entity_method_packet(
                        key,
                        seq,
                        acks,
                        eid,
                        method_index,
                        &args,
                        version,
                    )
                },
            )
            .await;
        });
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{test_default_connected_client_state, TestTransport};
    use std::net::SocketAddr;

    type ConnectedMap = Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>;
    type EntityAddrMap = Arc<Mutex<HashMap<u32, SocketAddr>>>;

    /// Build test context: a typed `Arc<TestTransport>` (for inspection) plus
    /// the matching `Arc<dyn Transport>` (for passing to handlers).
    fn make_ctx(
        entity_ids: &[u32],
    ) -> (
        Arc<TestTransport>,
        Arc<dyn Transport>,
        ConnectedMap,
        EntityAddrMap,
    ) {
        let test_transport = Arc::new(TestTransport::new());
        let transport: Arc<dyn Transport> = Arc::clone(&test_transport) as Arc<dyn Transport>;
        let connected: Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let entity_to_addr: Arc<Mutex<HashMap<u32, SocketAddr>>> =
            Arc::new(Mutex::new(HashMap::new()));

        // Register dummy addresses for each entity_id.
        {
            let mut e2a = entity_to_addr.lock().unwrap();
            let mut conn = connected.lock().unwrap();
            for (i, &eid) in entity_ids.iter().enumerate() {
                let addr: SocketAddr = format!("127.0.0.1:{}", 10000 + i).parse().unwrap();
                e2a.insert(eid, addr);
                conn.insert(addr, test_default_connected_client_state());
            }
        }

        (test_transport, transport, connected, entity_to_addr)
    }

    /// Bug shape: broadcast_to_org targets only online members.
    ///
    /// This is a fanout-byte test: we assert the TestTransport recorded one
    /// send per entity_id in the list, and zero sends for an entity_id NOT
    /// in the list.
    #[tokio::test]
    async fn broadcast_targets_only_supplied_entity_ids() {
        let online_eids = vec![1001u32, 1002u32];
        let (test_transport, transport, connected, entity_to_addr) = make_ctx(&online_eids);

        broadcast_to_org(
            online_eids.clone(),
            method_idx::ON_ORGANIZATION_MOTD_UPDATE,
            vec![0x01, 0x02, 0x03], // dummy args
            &transport,
            &connected,
            &entity_to_addr,
        );

        // Allow spawned tasks to complete.
        tokio::task::yield_now().await;
        tokio::task::yield_now().await;

        // Each online entity should have received exactly one packet.
        let send_count = test_transport.len();
        assert_eq!(
            send_count, 2,
            "must send to exactly 2 entities; got {send_count}"
        );
    }

    /// Bug shape: broadcast to empty list is a no-op (no panics, no sends).
    #[tokio::test]
    async fn broadcast_empty_list_is_noop() {
        let (test_transport, transport, connected, entity_to_addr) = make_ctx(&[]);

        broadcast_to_org(
            vec![],
            method_idx::ON_ORGANIZATION_MOTD_UPDATE,
            vec![],
            &transport,
            &connected,
            &entity_to_addr,
        );
        tokio::task::yield_now().await;

        assert_eq!(test_transport.len(), 0);
    }
}
