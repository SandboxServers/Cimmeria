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
//! outstanding. World entry waits for it ([`defer_until_synced`]): an entry
//! the client looks up before it has been re-pushed is simply missing, and
//! an in-world cache miss (`SGWPlayer.elementDataRequest`) is not served.

mod decision;
mod registry;
mod task;

#[cfg(test)]
mod tests;

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub use decision::{resync_pending_version, VersionReply};
pub use registry::{defer_until_synced, is_syncing, DeferredAction, EnqueueOutcome, SyncJob};
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
