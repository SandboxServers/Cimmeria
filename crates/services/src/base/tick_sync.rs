use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use cimmeria_mercury::transport::Transport;
use tokio::sync::mpsc;

use cimmeria_entity::manager::EntityManager;
use cimmeria_mercury::packet::SEQUENCE_MASK;

use crate::cell::messages::BaseToCellMsg;
use crate::mercury::build_ongoing_tick_sync;

use super::helpers::destroy_client_entities;
use super::ConnectedClientState;

/// Builds one tickSync packet for a single loop iteration.
///
/// Advances `counter` on the **unreliable** stream, applies the 28-bit
/// `SEQUENCE_MASK`, and returns `(seq_id, encrypted_packet)`. The caller
/// uses `seq_id` for logging and `encrypted_packet` for `send_to`. Never
/// touches the reliable [`Channel`] TX window — that invariant is the
/// reason this helper exists and is tested separately.
///
/// [`Channel`]: cimmeria_mercury::channel::Channel
pub(crate) fn tick_sync_packet(
    counter: &AtomicU32,
    key: &[u8; 32],
    tick: u32,
    acks: &[u32],
) -> (u32, Vec<u8>) {
    let seq_id = counter.fetch_add(1, Ordering::Relaxed) & SEQUENCE_MASK;
    (seq_id, build_ongoing_tick_sync(key, seq_id, tick, acks))
}

/// Per-connection tick-sync heartbeat task.
///
/// Drives two independent concerns from one 100 ms loop:
///
/// 1. **Unreliable tickSync emission** — builds a `tickSync` packet on the
///    per-session unreliable seq counter (`next_seq_unreliable`) and fires
///    it onto the wire. The counter is the only outgoing seq state this
///    task touches.
/// 2. **Reliable retransmit scan** — calls into the per-session [`Channel`]
///    (reached through `connected`) to drain RTO-expired entries from the
///    reliable TX window and re-send them. The reliable seq state lives on
///    `ConnectedClientState::next_seq` and is owned by other paths
///    (application packets, AoI broadcasts); this task only reads the TX
///    window through the channel and never advances the reliable counter.
///
/// Splitting the two concerns into separate tasks would mean two timers
/// per session firing on the same cadence — co-locating them on one loop
/// is the simpler shape. The unreliable counter is plumbed in directly
/// (parameter); the reliable side comes in through `connected` because the
/// retransmit driver needs the full channel state, not just the counter.
///
/// [`Channel`]: cimmeria_mercury::channel::Channel
pub(crate) async fn run_tick_loop(
    transport: Arc<dyn Transport>,
    addr: SocketAddr,
    key: [u8; 32],
    next_seq_unreliable: Arc<AtomicU32>,
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

        // tickSync rides the **unreliable** seq stream — its own monotonic
        // counter on `ConnectedClientState`, distinct from the reliable
        // counter that carries application packets. Both failure modes the
        // split-counter design defends against:
        //
        //   - Shared reliable counter: the receiver's `inSeqAt` stalls
        //     because tickSync's high-rate ticks consume reliable seq
        //     slots the client expects to be contiguous, so any drop
        //     leaves a permanent gap reliable delivery can't fill.
        //   - Reliable + own counter: tickSync's 10 Hz cadence saturates
        //     the 32-slot reliable TX window during the world-entry burst
        //     (char list + versionInfo + resourceFragments), starving
        //     application packets of slots even though the receiver model
        //     would tolerate it.
        //
        // Unreliable on its own counter sidesteps both: fire-and-forget,
        // reliable stream stays contiguous (client's `inSeqAt` only tracks
        // reliable arrivals), no TX window pressure.
        let (seq_id, pkt) = tick_sync_packet(&next_seq_unreliable, &key, tick, &acks);
        if let Err(e) = transport.send_to(&pkt, addr).await {
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
            if let Err(e) = transport.send_to(raw, addr).await {
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
    destroy_client_entities(
        &connected,
        &entity_manager,
        addr,
        &cell_tx,
        &entity_to_addr,
        "inactivity_timeout",
    );
}
