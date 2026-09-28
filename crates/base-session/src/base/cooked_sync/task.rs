//! The per-session resync task: one `tokio` task per session with work,
//! serving misses first and pushing queued categories in rank order, every
//! packet paced through the session's reliable window.
//!
//! Per category the client receives, in order on the reliable channel:
//!
//! 1. `onVersionInfo(InvalidateAll = 1, RequiredUpdates = 0,
//!    Version = resync_pending_version(server))`: the client empties the
//!    category (`FUN_0047a690`) and stamps the placeholder version.
//!    `RequiredUpdates` is 0 because the client only asks for a missing
//!    entry while it is 0 (`this+0x48 == 0` in every request function, e.g.
//!    `0x00cfe060`); with `N` it would sit on every miss until the whole
//!    category had arrived.
//! 2. One `resourceFragment` transfer per entry, in ascending key order,
//!    with any served misses sent between entries. The client writes each
//!    entry to its cache as it arrives (`0x0043dad0` → `0x0043bdb0`).
//! 3. `onVersionInfo(InvalidateAll = 0, RequiredUpdates = 0, no keys,
//!    Version = server)`: stamps the real version, ordered behind every
//!    entry, so a client that disconnects part-way keeps the placeholder
//!    and resyncs on its next login.
//!
//! Both replies go to whichever entity the session has: the Account
//! (client method 0) at character select, the player (SGWPlayer client
//! method 96) once it is in the world.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::AtomicU32;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use cimmeria_mercury::encryption::EncryptionVersion;
use cimmeria_mercury::transport::Transport;

use super::super::helpers::{drain_acks_and_seq, get_enc_version, shadow_register_reliable_send};
use super::super::resources::ResourceCache;
use super::super::ConnectedClientState;
use super::decision::resync_pending_version;
use super::registry::{self, DeferredAction, Miss, Next, SyncJob};
use super::{MAX_CHUNK, SYNC_IN_FLIGHT_BUDGET};
use crate::mercury::{
    build_resource_fragment, build_version_info, build_version_info_to_player, FRAG_FIRST,
    FRAG_FIRST_AND_LAST, FRAG_LAST, FRAG_MIDDLE,
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

/// Why the task stopped before finishing.
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

/// A category part-way through its push.
struct InProgress {
    job: SyncJob,
    keys: Vec<u32>,
    next: usize,
    started: Instant,
    entries: u32,
    bytes: u64,
    packets: u32,
}

pub(super) fn spawn(ctx: SyncContext, token: Arc<AtomicU32>) {
    tokio::spawn(run(ctx, token));
}

async fn run(ctx: SyncContext, token: Arc<AtomicU32>) {
    let account_id = account_id(&ctx);
    let mut current: Option<InProgress> = None;
    loop {
        // Wait for room before choosing what to send, so a miss that
        // arrives while the window is full still goes out next.
        let step = match wait_for_window(&ctx, &token).await {
            Ok(()) => registry::next(&ctx.connected, ctx.addr, &token, current.is_some()),
            Err(_) => Next::Gone,
        };
        let result = match step {
            Next::Miss(miss) => serve_miss(&ctx, &token, miss, account_id).await,
            Next::Start(job) => match start(&ctx, &token, job, account_id).await {
                Ok(p) => {
                    current = Some(p);
                    Ok(())
                }
                Err(e) => Err(e),
            },
            Next::Continue => {
                let p = current.as_mut().expect("Continue only while in progress");
                if p.next < p.keys.len() {
                    push_next_entry(&ctx, &token, p).await
                } else {
                    let p = current.take().expect("checked above");
                    finish(&ctx, &token, p, account_id).await
                }
            }
            Next::Idle => return,
            Next::Gone => Err(Abandon::SessionGone),
        };
        if let Err(abandon) = result {
            if let Some(p) = &current {
                warn_abandoned(&ctx, account_id, p, abandon);
            }
            for lost in registry::abandon(ctx.addr, &token) {
                warn_never_started(&ctx, account_id, lost);
            }
            return;
        }
    }
}

async fn start(
    ctx: &SyncContext,
    token: &Arc<AtomicU32>,
    job: SyncJob,
    account_id: u32,
) -> Result<InProgress, Abandon> {
    let mut keys: Vec<u32> = ctx
        .cache
        .category(job.category_id)
        .map(|c| c.elements.keys().copied().collect())
        .unwrap_or_default();
    keys.sort_unstable();
    let bytes: u64 = keys
        .iter()
        .filter_map(|k| ctx.cache.get(job.category_id, *k))
        .map(|v| v.len() as u64)
        .sum();
    tracing::info!(
        addr = %ctx.addr,
        account_id,
        event = "cooked_data.sync_start",
        category_id = job.category_id,
        client_version = job.client_version,
        server_version = job.server_version,
        entry_count = keys.len(),
        bytes,
        held = super::order::is_held(job.category_id),
        "Cooked-data resync started"
    );
    send_version_info(
        ctx,
        token,
        job.category_id,
        resync_pending_version(job.server_version),
        true,
    )
    .await?;
    Ok(InProgress {
        job,
        keys,
        next: 0,
        started: Instant::now(),
        entries: 0,
        bytes: 0,
        packets: 1,
    })
}

async fn push_next_entry(
    ctx: &SyncContext,
    token: &Arc<AtomicU32>,
    p: &mut InProgress,
) -> Result<(), Abandon> {
    let element_id = p.keys[p.next];
    p.next += 1;
    let Some(xml) = ctx.cache.get(p.job.category_id, element_id) else {
        return Ok(());
    };
    p.packets += send_entry(ctx, token, p.job.category_id, element_id, xml).await?;
    p.entries += 1;
    p.bytes += xml.len() as u64;
    Ok(())
}

async fn finish(
    ctx: &SyncContext,
    token: &Arc<AtomicU32>,
    mut p: InProgress,
    account_id: u32,
) -> Result<(), Abandon> {
    send_version_info(ctx, token, p.job.category_id, p.job.server_version, false).await?;
    p.packets += 1;
    tracing::info!(
        addr = %ctx.addr,
        account_id,
        event = "cooked_data.sync_finish",
        outcome = "complete",
        category_id = p.job.category_id,
        client_version = p.job.client_version,
        server_version = p.job.server_version,
        entry_count = p.entries,
        bytes = p.bytes,
        packets = p.packets,
        duration_ms = p.started.elapsed().as_millis() as u64,
        "Cooked-data resync finished"
    );
    cimmeria_observability::counter!("cooked_data_resyncs_total", "outcome" => "complete");
    release(ctx, account_id, registry::finish_job(ctx.addr, token));
    Ok(())
}

fn release(ctx: &SyncContext, account_id: u32, deferred: Vec<DeferredAction>) {
    if deferred.is_empty() {
        return;
    }
    tracing::info!(
        addr = %ctx.addr,
        account_id,
        event = "cooked_data.world_entry_released",
        held_actions = deferred.len(),
        "Held cooked-data categories resynced: releasing world entry"
    );
    for action in deferred {
        tokio::spawn(action());
    }
}

async fn serve_miss(
    ctx: &SyncContext,
    token: &Arc<AtomicU32>,
    miss: Miss,
    account_id: u32,
) -> Result<(), Abandon> {
    // Validated when queued; the cache is immutable.
    let Some(xml) = ctx.cache.get(miss.category_id, miss.key) else {
        return Ok(());
    };
    let packets = send_entry(ctx, token, miss.category_id, miss.key, xml).await?;
    tracing::info!(
        addr = %ctx.addr,
        account_id,
        event = "cooked_data.miss_served",
        category_id = miss.category_id,
        key = miss.key,
        bytes = xml.len(),
        packets,
        latency_ms = miss.requested_at.elapsed().as_millis() as u64,
        "Served a cooked-data cache miss"
    );
    cimmeria_observability::counter!("cooked_data_misses_total", "outcome" => "served");
    Ok(())
}

/// Send one entry as a `resourceFragment` transfer. Returns the packets
/// sent.
async fn send_entry(
    ctx: &SyncContext,
    token: &Arc<AtomicU32>,
    category_id: u32,
    element_id: u32,
    xml: &[u8],
) -> Result<u32, Abandon> {
    let chunks: Vec<&[u8]> = if xml.is_empty() {
        vec![&[][..]]
    } else {
        xml.chunks(MAX_CHUNK).collect()
    };
    if chunks.len() > usize::from(u8::MAX) + 1 {
        // chunk_id is a u8 on the wire. No shipped entry comes close (the
        // largest, CookedCharCreation, is 122 chunks).
        tracing::error!(
            addr = %ctx.addr,
            category_id,
            element_id,
            bytes = xml.len(),
            reason = "entry_exceeds_256_fragments",
            "Cooked-data entry too large for one transfer: skipped"
        );
        return Ok(0);
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
            (Some(0u8), Some(category_id), Some(element_id))
        } else {
            (None, None, None)
        };
        send_paced(ctx, token, |seq, acks, enc, _| {
            build_resource_fragment(
                &ctx.key, seq, acks, data_id, i as u8, flags, mt, cat, elem, chunk, enc,
            )
        })
        .await?;
    }
    Ok(chunks.len() as u32)
}

/// `onVersionInfo` with no keys and `RequiredUpdates = 0`, addressed to the
/// session's current entity.
async fn send_version_info(
    ctx: &SyncContext,
    token: &Arc<AtomicU32>,
    category_id: u32,
    version: u32,
    invalidate_all: bool,
) -> Result<(), Abandon> {
    send_paced(ctx, token, |seq, acks, enc, target| match target {
        Target::Player(eid) => build_version_info_to_player(
            &ctx.key,
            seq,
            acks,
            category_id,
            version,
            0,
            invalidate_all,
            &[],
            eid,
            enc,
        ),
        Target::Account(eid) => build_version_info(
            &ctx.key,
            seq,
            acks,
            category_id,
            version,
            0,
            invalidate_all,
            &[],
            eid,
            enc,
        ),
    })
    .await
}

/// Which entity a client-method call goes to right now.
#[derive(Debug, Clone, Copy)]
enum Target {
    Account(u32),
    Player(u32),
}

fn target(ctx: &SyncContext) -> Option<Target> {
    let clients = ctx.connected.lock().ok()?;
    let c = clients.get(&ctx.addr)?;
    Some(match c.player_entity_id {
        Some(eid) => Target::Player(eid),
        None => Target::Account(c.account_entity_id),
    })
}

/// Wait until the session's reliable window has room, then build, send and
/// register one reliable packet.
async fn send_paced(
    ctx: &SyncContext,
    token: &Arc<AtomicU32>,
    build: impl FnOnce(u32, &[u32], EncryptionVersion, Target) -> Vec<u8>,
) -> Result<(), Abandon> {
    wait_for_window(ctx, token).await?;
    let target = target(ctx).ok_or(Abandon::SessionGone)?;
    let (acks, seq) =
        drain_acks_and_seq(&ctx.connected, ctx.addr).map_err(|_| Abandon::SessionGone)?;
    let enc = get_enc_version(&ctx.connected, ctx.addr);
    let pkt = build(seq, &acks, enc, target);
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
/// the budget keeps the push inside the 32-slot window with room to spare
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

fn warn_abandoned(ctx: &SyncContext, account_id: u32, p: &InProgress, abandon: Abandon) {
    tracing::warn!(
        addr = %ctx.addr,
        account_id,
        event = "cooked_data.sync_finish",
        outcome = "abandoned",
        reason = abandon.reason(),
        category_id = p.job.category_id,
        client_version = p.job.client_version,
        server_version = p.job.server_version,
        entries_sent = p.entries,
        entry_count = p.keys.len(),
        bytes = p.bytes,
        packets = p.packets,
        duration_ms = p.started.elapsed().as_millis() as u64,
        "Cooked-data resync abandoned part-way; the client keeps the placeholder \
         version and resyncs on its next login"
    );
    cimmeria_observability::counter!("cooked_data_resyncs_total", "outcome" => "abandoned");
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
