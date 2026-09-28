//! The per-session resync task: one `tokio` task per session with at least
//! one mismatched category, pushing each queued category in turn and pacing
//! every packet through the session's reliable window.
//!
//! Per category the client receives, in order on the reliable channel:
//!
//! 1. `onVersionInfo(InvalidateAll = 1, RequiredUpdates = N,
//!    Version = resync_pending_version(server))`: the client empties the
//!    category (`FUN_0047a690`) and stamps the placeholder version.
//! 2. One `resourceFragment` transfer per entry, `N` in all, in ascending
//!    key order. The client writes each entry to its cache as it arrives
//!    (`FUN_0043dad0` → `FUN_0043bdb0`) and counts `RequiredUpdates` down.
//! 3. `onVersionInfo(InvalidateAll = 0, RequiredUpdates = 0, no keys,
//!    Version = server)`: stamps the real version. It is ordered behind
//!    every entry, so a client that disconnects part-way keeps the
//!    placeholder and resyncs on its next login.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::AtomicU32;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use cimmeria_mercury::transport::Transport;

use super::super::helpers::{
    drain_acks_and_seq, get_active_entity_id, get_enc_version, shadow_register_reliable_send,
};
use super::super::resources::ResourceCache;
use super::super::ConnectedClientState;
use super::decision::resync_pending_version;
use super::registry::{self, NextJob, SyncJob};
use super::{MAX_CHUNK, SYNC_IN_FLIGHT_BUDGET};
use crate::mercury::{
    build_resource_fragment, build_version_info, FRAG_FIRST, FRAG_FIRST_AND_LAST, FRAG_LAST,
    FRAG_MIDDLE,
};

/// Everything the task needs from the session that started it.
#[derive(Clone)]
pub struct SyncContext {
    pub transport: Arc<dyn Transport>,
    pub addr: SocketAddr,
    pub key: [u8; 32],
    pub connected: Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    pub cache: Arc<ResourceCache>,
    /// How long to wait before looking at the window again when it is full.
    pub poll: Duration,
}

/// Why a category's push stopped before its closing stamp.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Abandon {
    /// The session disconnected or was replaced (relog).
    SessionGone,
    /// A socket send failed.
    SendFailed,
}

impl Abandon {
    fn reason(self) -> &'static str {
        match self {
            Self::SessionGone => "session_gone",
            Self::SendFailed => "send_failed",
        }
    }
}

/// Counts for one category's push, for the finish and abandon events.
#[derive(Debug, Default, Clone, Copy)]
struct Progress {
    entries: u32,
    bytes: u64,
    packets: u32,
}

pub(super) fn spawn(ctx: SyncContext, token: Arc<AtomicU32>) {
    tokio::spawn(run(ctx, token));
}

async fn run(ctx: SyncContext, token: Arc<AtomicU32>) {
    let account_id = account_id(&ctx);
    loop {
        match registry::next_job(&ctx.connected, ctx.addr, &token) {
            NextJob::Job(job, queued_after) => {
                if let Err(()) = run_job(&ctx, &token, job, queued_after, account_id).await {
                    for lost in registry::abandon(ctx.addr, &token) {
                        warn_never_started(&ctx, account_id, lost);
                    }
                    return;
                }
            }
            NextJob::Done(deferred) => {
                if !deferred.is_empty() {
                    tracing::info!(
                        addr = %ctx.addr,
                        account_id,
                        event = "cooked_data.world_entry_released",
                        held_actions = deferred.len(),
                        "Cooked-data resync finished: releasing held world entry"
                    );
                }
                for action in deferred {
                    tokio::spawn(action());
                }
                return;
            }
            NextJob::SessionGone(lost) => {
                for job in lost {
                    warn_never_started(&ctx, account_id, job);
                }
                return;
            }
        }
    }
}

async fn run_job(
    ctx: &SyncContext,
    token: &Arc<AtomicU32>,
    job: SyncJob,
    queued_after: usize,
    account_id: u32,
) -> Result<(), ()> {
    let started = Instant::now();
    let Some(category) = ctx.cache.category(job.category_id) else {
        // Unreachable in practice (the decision only resyncs served
        // categories), but a missing category must not stall the queue.
        return Ok(());
    };
    let mut keys: Vec<u32> = category.elements.keys().copied().collect();
    keys.sort_unstable();
    let total_bytes: u64 = category.elements.values().map(|v| v.len() as u64).sum();

    tracing::info!(
        addr = %ctx.addr,
        account_id,
        event = "cooked_data.sync_start",
        category_id = job.category_id,
        client_version = job.client_version,
        server_version = job.server_version,
        entry_count = keys.len(),
        bytes = total_bytes,
        queued_after,
        "Cooked-data resync started"
    );

    let mut progress = Progress::default();
    let outcome = push_category(ctx, token, job, &keys, &mut progress).await;
    let duration_ms = started.elapsed().as_millis() as u64;
    match outcome {
        Ok(()) => {
            tracing::info!(
                addr = %ctx.addr,
                account_id,
                event = "cooked_data.sync_finish",
                outcome = "complete",
                category_id = job.category_id,
                client_version = job.client_version,
                server_version = job.server_version,
                entry_count = progress.entries,
                bytes = progress.bytes,
                packets = progress.packets,
                duration_ms,
                "Cooked-data resync finished"
            );
            cimmeria_observability::counter!("cooked_data_resyncs_total", "outcome" => "complete");
            Ok(())
        }
        Err(abandon) => {
            tracing::warn!(
                addr = %ctx.addr,
                account_id,
                event = "cooked_data.sync_finish",
                outcome = "abandoned",
                reason = abandon.reason(),
                category_id = job.category_id,
                client_version = job.client_version,
                server_version = job.server_version,
                entries_sent = progress.entries,
                entry_count = keys.len(),
                bytes = progress.bytes,
                packets = progress.packets,
                duration_ms,
                "Cooked-data resync abandoned part-way; the client keeps the placeholder \
                 version and resyncs on its next login"
            );
            cimmeria_observability::counter!("cooked_data_resyncs_total", "outcome" => "abandoned");
            Err(())
        }
    }
}

async fn push_category(
    ctx: &SyncContext,
    token: &Arc<AtomicU32>,
    job: SyncJob,
    keys: &[u32],
    progress: &mut Progress,
) -> Result<(), Abandon> {
    let entity_id =
        get_active_entity_id(&ctx.connected, ctx.addr).map_err(|_| Abandon::SessionGone)?;

    // 1. Empty the category and stamp the placeholder version.
    let pending = resync_pending_version(job.server_version);
    let required = keys.len() as u32;
    send_paced(ctx, token, |seq, acks, enc| {
        build_version_info(
            &ctx.key,
            seq,
            acks,
            job.category_id,
            pending,
            required,
            true,
            &[],
            entity_id,
            enc,
        )
    })
    .await?;
    progress.packets += 1;

    // 2. Every entry, oldest key first.
    let category = ctx
        .cache
        .category(job.category_id)
        .ok_or(Abandon::SessionGone)?;
    for &element_id in keys {
        let Some(xml) = category.elements.get(&element_id) else {
            continue;
        };
        let chunks: Vec<&[u8]> = if xml.is_empty() {
            vec![&[][..]]
        } else {
            xml.chunks(MAX_CHUNK).collect()
        };
        if chunks.len() > usize::from(u8::MAX) + 1 {
            // chunk_id is a u8 on the wire. No shipped entry comes close
            // (the largest, CookedCharCreation, is 122 chunks).
            tracing::error!(
                addr = %ctx.addr,
                category_id = job.category_id,
                element_id,
                bytes = xml.len(),
                reason = "entry_exceeds_256_fragments",
                "Cooked-data resync skipped an entry too large for one transfer"
            );
            continue;
        }
        let data_id = alloc_data_id(ctx).ok_or(Abandon::SessionGone)?;
        let last = chunks.len() - 1;
        for (i, chunk) in chunks.iter().enumerate() {
            let flags = match (i == 0, i == last) {
                (true, true) => FRAG_FIRST_AND_LAST,
                (true, false) => FRAG_FIRST,
                (false, true) => FRAG_LAST,
                (false, false) => FRAG_MIDDLE,
            };
            let (mt, cat, elem) = if i == 0 {
                (Some(0u8), Some(job.category_id), Some(element_id))
            } else {
                (None, None, None)
            };
            send_paced(ctx, token, |seq, acks, enc| {
                build_resource_fragment(
                    &ctx.key, seq, acks, data_id, i as u8, flags, mt, cat, elem, chunk, enc,
                )
            })
            .await?;
            progress.packets += 1;
        }
        progress.entries += 1;
        progress.bytes += xml.len() as u64;
    }

    // 3. Stamp the server's version, behind every entry.
    send_paced(ctx, token, |seq, acks, enc| {
        build_version_info(
            &ctx.key,
            seq,
            acks,
            job.category_id,
            job.server_version,
            0,
            false,
            &[],
            entity_id,
            enc,
        )
    })
    .await?;
    progress.packets += 1;
    Ok(())
}

/// Wait until the session's reliable window has room, then build, send and
/// register one reliable packet.
async fn send_paced(
    ctx: &SyncContext,
    token: &Arc<AtomicU32>,
    build: impl FnOnce(u32, &[u32], cimmeria_mercury::encryption::EncryptionVersion) -> Vec<u8>,
) -> Result<(), Abandon> {
    wait_for_window(ctx, token).await?;
    let (acks, seq) =
        drain_acks_and_seq(&ctx.connected, ctx.addr).map_err(|_| Abandon::SessionGone)?;
    let enc = get_enc_version(&ctx.connected, ctx.addr);
    let pkt = build(seq, &acks, enc);
    ctx.transport
        .send_to(&pkt, ctx.addr)
        .await
        .map_err(|_| Abandon::SendFailed)?;
    shadow_register_reliable_send(
        &ctx.connected,
        ctx.addr,
        seq,
        cimmeria_mercury::packet::Bytes::copy_from_slice(&pkt),
    );
    Ok(())
}

/// Block until fewer than [`SYNC_IN_FLIGHT_BUDGET`] reliable packets are
/// outstanding on the session, or the session is gone.
///
/// Outstanding means the TX window plus the deferred-send queue: both hold
/// packets on the wire that the client has not acked. Keeping the sum under
/// the budget keeps the sync inside the 32-slot window with room to spare
/// for game traffic, which is never paced.
async fn wait_for_window(ctx: &SyncContext, token: &Arc<AtomicU32>) -> Result<(), Abandon> {
    loop {
        {
            let clients = ctx.connected.lock().map_err(|_| Abandon::SessionGone)?;
            let state = clients
                .get(&ctx.addr)
                .filter(|c| Arc::ptr_eq(&c.next_seq, token))
                .ok_or(Abandon::SessionGone)?;
            let channel = state.channel.lock().map_err(|_| Abandon::SessionGone)?;
            if channel.tx_window.len() + channel.unsent_packets.len() < SYNC_IN_FLIGHT_BUDGET {
                return Ok(());
            }
        }
        if ctx.poll.is_zero() {
            // Tests: hand the runtime to the simulated client's acks.
            tokio::task::yield_now().await;
        } else {
            tokio::time::sleep(ctx.poll).await;
        }
    }
}

fn alloc_data_id(ctx: &SyncContext) -> Option<u16> {
    let mut clients = ctx.connected.lock().ok()?;
    let c = clients.get_mut(&ctx.addr)?;
    let id = c.next_data_id;
    c.next_data_id = c.next_data_id.wrapping_add(1);
    Some(id)
}

fn account_id(ctx: &SyncContext) -> u32 {
    ctx.connected
        .lock()
        .ok()
        .and_then(|c| c.get(&ctx.addr).map(|s| s.account_id))
        .unwrap_or(0)
}

fn warn_never_started(ctx: &SyncContext, account_id: u32, job: SyncJob) {
    tracing::warn!(
        addr = %ctx.addr,
        account_id,
        event = "cooked_data.sync_finish",
        outcome = "abandoned",
        reason = "session_gone_before_start",
        category_id = job.category_id,
        client_version = job.client_version,
        server_version = job.server_version,
        entries_sent = 0u32,
        "Cooked-data resync abandoned before it started; the client was never told to \
         empty this category and resyncs on its next login"
    );
    cimmeria_observability::counter!("cooked_data_resyncs_total", "outcome" => "abandoned");
}
