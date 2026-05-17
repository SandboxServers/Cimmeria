use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tokio::net::UdpSocket;
use tokio::sync::mpsc;

use cimmeria_entity::manager::EntityManager;

use crate::cell::messages::BaseToCellMsg;
use crate::mercury::build_ongoing_tick_sync;

use super::helpers::destroy_client_entities;
use super::ConnectedClientState;

/// Per-connection tick-sync heartbeat task.
pub(crate) async fn run_tick_loop(
    socket: Arc<UdpSocket>,
    addr: SocketAddr,
    key: [u8; 32],
    next_seq: Arc<AtomicU32>,
    pending_acks: Arc<Mutex<Vec<u32>>>,
    last_recv: Arc<Mutex<Instant>>,
    cancelled: Arc<AtomicBool>,
    connected: Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_manager: Arc<Mutex<EntityManager>>,
    cell_tx: Option<mpsc::Sender<BaseToCellMsg>>,
    entity_to_addr: Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    const INACTIVITY_TIMEOUT: Duration = Duration::from_secs(60);

    let mut tick: u32 = 0;

    tracing::debug!(%addr, "Tick-sync loop started");

    loop {
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Check if the session was torn down by handle_log_off.
        if cancelled.load(Ordering::Relaxed) {
            tracing::info!(%addr, "Tick-sync stopping: session cancelled (logOff)");
            break;
        }

        let idle = last_recv.lock().unwrap().elapsed();
        if idle > INACTIVITY_TIMEOUT {
            tracing::info!(
                %addr,
                idle_secs = idle.as_secs(),
                "Tick-sync stopping: client inactive for {}s",
                idle.as_secs()
            );
            break;
        }

        let acks: Vec<u32> = {
            let mut pending = pending_acks.lock().unwrap();
            pending.drain(..).collect()
        };
        if !acks.is_empty() {
            tracing::trace!(%addr, ?acks, "Piggybacking ACKs on tick_sync");
        }

        let seq_id =
            next_seq.fetch_add(1, Ordering::Relaxed) & cimmeria_mercury::packet::SEQUENCE_MASK;
        let pkt = build_ongoing_tick_sync(&key, seq_id, tick, &acks);
        if let Err(e) = socket.send_to(&pkt, addr).await {
            tracing::debug!(%addr, "Tick-sync stopped (send error): {e}");
            break;
        }

        // Drive the per-session Channel's retransmit scan. Any reliable
        // packet whose adaptive RTO has elapsed without receiving an ack
        // gets re-sent here, cached encrypted bytes straight to the wire.
        // The Channel applies the per-tick retransmit budget (at most
        // `RETRANSMIT_BUDGET_PER_TICK` entries per scan, per
        // `mercury-wire-format` spec §1.7) and Karn's exponential backoff
        // internally.
        let retransmits = super::helpers::collect_pending_retransmits(&connected, addr);
        for (batch_index, raw) in retransmits.iter().enumerate() {
            tracing::debug!(
                %addr,
                len = raw.len(),
                batch_index,
                batch_total = retransmits.len(),
                "Channel retransmit: RTO fired before ACK"
            );
            if let Err(e) = socket.send_to(raw, addr).await {
                tracing::debug!(%addr, "Retransmit send_to failed: {e}");
                break;
            }
        }

        if tick.is_multiple_of(100) {
            tracing::debug!(%addr, tick, seq_id, "Tick-sync heartbeat (every 100th)");
        }

        tick = tick.wrapping_add(1);
    }

    // Clean up entities for this disconnected client.
    destroy_client_entities(&connected, &entity_manager, addr, &cell_tx, &entity_to_addr);
}
