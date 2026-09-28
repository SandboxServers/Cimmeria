//! Full-category cooked-data resync (#840).
//!
//! When a client's cached version of a category differs from the server's,
//! the client is made to hold exactly the server's category: entries it has
//! and the server does not are removed, missing ones are added, changed
//! ones are replaced. `versionInfoRequest` carries only a version, so the
//! server cannot diff keys; instead it empties the category
//! (`InvalidateAll = 1`) and pushes every entry, then stamps its version so
//! the next login matches.
//!
//! Why every entry has to be pushed: the client's `onVersionInfo` handler
//! (`ServerSource<N>::onVersionInfo`, `0x00441630`) deletes every entry of
//! the category from its writable cache PAK when `InvalidateAll` is set and
//! does not fetch any of them back. Before #840 the server sent that flag
//! with nothing pushed for every category without an override list, which
//! is how every client lost its Kismet sequence table on 2026-09-20 (#754).
//!
//! The push runs on its own task per session ([`task`]), paced so that the
//! session never has more than [`SYNC_IN_FLIGHT_BUDGET`] reliable packets
//! outstanding. An entry the client looks up before it has been re-pushed
//! is requested with `elementDataRequest`, and [`serve_miss`] sends it next,
//! ahead of the stream. World entry waits only for the categories the
//! client has no miss path for ([`order::HELD_CATEGORIES`],
//! [`defer_until_synced`]); the rest keep streaming in the world.

mod decision;
mod order;
mod registry;
mod task;

#[cfg(test)]
mod tests;

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub use decision::{resync_pending_version, VersionReply};
pub use order::{is_held, rank, HELD_CATEGORIES};
pub use registry::{
    defer_until_synced, holds_world_entry, is_syncing, DeferredAction, EnqueueOutcome, MissOutcome,
    MissRefusal, SyncJob,
};
pub use task::SyncContext;

pub(crate) use super::cooked_data::MAX_CHUNK;
use super::ConnectedClientState;

/// TX-window slots the resync leaves free for everything else the session
/// sends (chat, entity updates, world entry). Game traffic is not paced, so
/// the headroom is what keeps a resync from pushing it into the
/// deferred-send queue.
pub const SYNC_RESERVED_SLOTS: usize = 8;

/// The most reliable packets a resync lets be outstanding on a session at
/// once, counting everything else in flight: the 32-slot TX window minus
/// [`SYNC_RESERVED_SLOTS`].
pub const SYNC_IN_FLIGHT_BUDGET: usize =
    cimmeria_mercury::consts::TX_WINDOW_SIZE - SYNC_RESERVED_SLOTS;

// The resync must leave real headroom in the window and still make progress.
const _: () = assert!(SYNC_RESERVED_SLOTS >= 8 && SYNC_IN_FLIGHT_BUDGET >= 16);

/// How often a resync blocked on a full window looks again. The client acks
/// every 60-100 ms while receiving, so this adds no measurable delay.
pub const SYNC_POLL_INTERVAL: Duration = Duration::from_millis(5);

/// Misses a session may have served per second, sustained.
pub const MISS_RATE_PER_SEC: u32 = 50;
/// Misses a session may have served at once before the rate applies.
pub const MISS_BURST: u32 = 100;
/// Misses a session may have waiting at once.
pub const MISS_QUEUE_CAP: usize = 256;

/// Serve one `elementDataRequest` (`0xC1` at character select, SGWPlayer
/// `0xD5` in-world): the entry goes out next on the session's task, ahead
/// of any background stream. Unknown categories and keys are refused, as
/// is anything over the session's rate limit; refusals log a throttled WARN.
pub fn serve_miss(ctx: SyncContext, category_id: u32, key: u32) -> MissOutcome {
    let now = Instant::now();
    let addr = ctx.addr;
    let account_id = ctx
        .connected
        .lock()
        .ok()
        .and_then(|c| c.get(&addr).map(|s| s.account_id))
        .unwrap_or(0);
    let Some(token) = registry::session_token(&ctx.connected, addr) else {
        return MissOutcome::Refused {
            why: MissRefusal::NoSession,
            log: None,
        };
    };
    let invalid = if ctx.cache.category(category_id).is_none() {
        Some(MissRefusal::UnknownCategory)
    } else if ctx.cache.get(category_id, key).is_none() {
        Some(MissRefusal::UnknownKey)
    } else {
        None
    };
    let outcome = match invalid {
        Some(why) => registry::refuse(addr, &token, why, now),
        None => registry::queue_miss(&token, addr, category_id, key, now),
    };
    match outcome {
        MissOutcome::Queued { start_task } => {
            if start_task {
                task::spawn(ctx, token);
            }
        }
        MissOutcome::Duplicate => {
            tracing::debug!(%addr, account_id, category_id, key, "cooked-data miss already queued");
        }
        MissOutcome::Refused { why, log } => {
            cimmeria_observability::counter!(
                "cooked_data_misses_total",
                "outcome" => "refused",
                "reason" => why.reason(),
            );
            if let Some(suppressed) = log {
                tracing::warn!(
                    %addr,
                    account_id,
                    event = "cooked_data.miss_refused",
                    reason = why.reason(),
                    category_id,
                    key,
                    suppressed,
                    "Refused a cooked-data cache miss"
                );
            }
        }
    }
    outcome
}

/// Queue `job` for the session at `ctx.addr`, spawning its resync task if
/// none is running. Returns what happened to the job.
pub fn start_resync(ctx: SyncContext, job: SyncJob) -> EnqueueOutcome {
    let (outcome, token) = registry::enqueue(&ctx.connected, ctx.addr, job);
    if let (EnqueueOutcome::StartTask, Some(token)) = (outcome, token) {
        task::spawn(ctx, token);
    }
    outcome
}

/// Whether in-world routing applies to the session at `addr`: once the
/// player entity exists, `0xC0`/`0xC1` are `SGWPlayer.chatJoin` /
/// `chatLeave`, not the cache messages.
pub fn in_world(
    connected: &Mutex<HashMap<SocketAddr, ConnectedClientState>>,
    addr: SocketAddr,
) -> bool {
    connected
        .lock()
        .ok()
        .and_then(|c| c.get(&addr).map(|s| s.player_entity_id.is_some()))
        .unwrap_or(false)
}

/// A context with the default poll interval ([`SYNC_POLL_INTERVAL`]; a
/// scheduler yield in this crate's own tests, so their simulated client's
/// acks interleave with the push without real sleeps).
pub fn context(
    transport: &Arc<dyn cimmeria_mercury::transport::Transport>,
    addr: SocketAddr,
    key: [u8; 32],
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    cache: &Arc<super::resources::ResourceCache>,
) -> SyncContext {
    SyncContext {
        transport: Arc::clone(transport),
        addr,
        key,
        connected: Arc::clone(connected),
        cache: Arc::clone(cache),
        poll: if cfg!(test) {
            Duration::ZERO
        } else {
            SYNC_POLL_INTERVAL
        },
    }
}
