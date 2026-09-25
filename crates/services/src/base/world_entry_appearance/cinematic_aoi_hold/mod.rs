//! First-login cinematic AoI hold — keep entity introductions off the wire
//! while the client plays the fullscreen intro movie.
//!
//! # The bug shape this guards against
//!
//! 2026-09-19, Castle_CellBlock: a first-login player never saw the
//! `class_id 0` static-mesh guard corpse two metres away until they
//! relogged. The server had sent its CREATE_ENTITY + cascade reliably at
//! `onClientReady`, in the same instant as `onPlayMovie`, and the client
//! ACKed both first try — one retransmit in the whole 75 s window, 17 s
//! after the create burst. So the drop is inside the client, after
//! delivery. The cinematic-exit `CollectGarbage` is already known to reclaim
//! the player's own appearance (#288, healed by the appearance spam in
//! [`super::cinematic`]); a static mesh whose entity was created mid-movie
//! has no such heal path.
//!
//! # The hold
//!
//! [`begin`] runs inside `handle_on_client_ready`'s `pending_client_ready`
//! take, so no `EnteredAoI` can slip through between the pre-ready gate
//! opening and the hold closing. While the hold is set,
//! [`crate::base::deferred_aoi::should_hold_entity_traffic`] buffers entity
//! introductions and everything that depends on them. The hold ends at
//! whichever comes first:
//!
//! - the client's `cancelMovie` (Esc / Lua stop) — [`release_on_cancel`];
//! - [`HOLD_DURATION`] elapsing — the task armed by [`arm_timeout`]. The
//!   client sends nothing when the movie ends on its own.
//!
//! Either way the buffer flushes through the ordinary deferred-AoI path, so
//! the client creates the entities after the movie and its exit GC.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use cimmeria_mercury::transport::Transport;
use tokio::time::Instant;

use super::super::world_entry::cell_dispatch::flush_deferred_aoi;
use super::super::ConnectedClientState;

/// How long entity introductions wait when the movie runs to its natural
/// end.
///
/// `Cine-SGWLogo` is 13.10 s (314 frames @ 23.976 fps — see the BIK-header
/// note in [`super::cinematic`]). The remainder covers the cinematic-exit GC.
/// The player is reading the intro dialog by then: in the 2026-09-19 repro
/// the first input came 16.4 s after `onClientReady`, so NPCs arriving at
/// 16 s land behind that dialog rather than popping into an empty room.
pub(crate) const HOLD_DURATION: Duration = Duration::from_secs(16);

/// An active hold, stored on [`ConnectedClientState::cinematic_aoi_hold`].
#[derive(Clone, Copy, Debug)]
pub(crate) struct CinematicAoiHold {
    /// Distinguishes this hold from a later one on the same session, so a
    /// stale timeout task cannot release a hold it did not start.
    pub token: u64,
    /// When the hold began — the origin [`HOLD_DURATION`] is measured from.
    pub started: Instant,
    /// A release has claimed this hold and is flushing it. A second releaser
    /// (the timeout racing `cancelMovie`) must leave it alone: see [`release`].
    pub releasing: bool,
}

/// Why a hold ended — the `reason` field on the release log line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ReleaseReason {
    CancelMovie,
    Timeout,
}

impl ReleaseReason {
    fn as_str(self) -> &'static str {
        match self {
            Self::CancelMovie => "cancel_movie",
            Self::Timeout => "timeout",
        }
    }
}

/// Start a hold on `state`. Caller holds the `connected` lock and is taking
/// `pending_client_ready` in the same critical section.
pub(crate) fn begin(state: &mut ConnectedClientState) -> CinematicAoiHold {
    static NEXT_TOKEN: AtomicU64 = AtomicU64::new(1);
    let hold = CinematicAoiHold {
        token: NEXT_TOKEN.fetch_add(1, Ordering::Relaxed),
        started: Instant::now(),
        releasing: false,
    };
    state.cinematic_aoi_hold = Some(hold);
    hold
}

/// Spawn the task that releases `hold` once [`HOLD_DURATION`] has passed
/// **since the hold began**, if `cancelMovie` hasn't released it first.
///
/// The deadline is `hold.started + HOLD_DURATION`, not "now + duration":
/// `handle_on_client_ready` arms this only after its DB reads, cell sends and
/// the cinematic dispatch, and a slow dependency there must shorten the
/// remaining wait rather than stretch the hold past the movie.
pub(crate) fn arm_timeout(
    hold: CinematicAoiHold,
    witness_id: u32,
    addr: SocketAddr,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    let transport = Arc::clone(transport);
    let connected = Arc::clone(connected);
    let entity_to_addr = Arc::clone(entity_to_addr);
    tracing::info!(
        target: "aoi.cinematic_hold",
        event = "hold_started",
        %addr,
        witness_id,
        token = hold.token,
        hold_ms = HOLD_DURATION.as_millis() as u64,
        "Cinematic AoI hold: entity introductions buffered until the movie ends"
    );
    tokio::spawn(async move {
        tokio::time::sleep_until(hold.started + HOLD_DURATION).await;
        release(
            Some(hold.token),
            ReleaseReason::Timeout,
            witness_id,
            addr,
            &transport,
            &connected,
            &entity_to_addr,
        )
        .await;
    });
}

/// Release whatever hold the session has, because the client dismissed the
/// movie. No-op when no hold is active (mid-session cinematics, or a
/// `cancelMovie` that lost the race with the timeout).
pub(crate) async fn release_on_cancel(
    witness_id: u32,
    addr: SocketAddr,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    release(
        None,
        ReleaseReason::CancelMovie,
        witness_id,
        addr,
        transport,
        connected,
        entity_to_addr,
    )
    .await;
}

/// Flush the held traffic, then lift the hold.
///
/// `expected` is the timeout task's own token: a task that outlived its hold
/// (released by `cancelMovie`, or the session moved on to a new world entry)
/// must not release a later one. `None` releases any active hold.
///
/// The hold lifts only once a flush leaves the buffer empty **under the same
/// lock**. Messages that arrive while a flush is awaiting its sends buffer
/// behind it and go out on the next pass, so nothing held is overtaken by
/// live traffic — a live `LeftAoI(X)` beating the buffered `EnteredAoI(X)`
/// would leave a ghost on the client.
///
/// Exactly one task releases a hold. The first to get here claims it
/// (`releasing`) under the lock; a second — `cancelMovie` racing the timeout
/// at the 16-second boundary — returns at once. Without the claim the second
/// task would find the buffer momentarily empty while the first is still
/// awaiting its sends, lift the hold, and let live traffic overtake them.
async fn release(
    expected: Option<u64>,
    reason: ReleaseReason,
    witness_id: u32,
    addr: SocketAddr,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    // Claim the hold. Later passes compare against `claimed`, so a hold that
    // was replaced while we were sending is left to its own owner.
    let claimed = {
        let Ok(mut clients) = connected.lock() else {
            return;
        };
        let Some(hold) = clients
            .get_mut(&addr)
            .and_then(|state| state.cinematic_aoi_hold.as_mut())
        else {
            return;
        };
        if hold.releasing || expected.is_some_and(|token| token != hold.token) {
            return;
        }
        hold.releasing = true;
        hold.token
    };

    let mut flushed = 0usize;
    let held_for;
    loop {
        {
            let Ok(mut clients) = connected.lock() else {
                return;
            };
            let Some(state) = clients.get_mut(&addr) else {
                return;
            };
            let Some(active) = state.cinematic_aoi_hold else {
                return;
            };
            if active.token != claimed {
                return;
            }
            if state.deferred_aoi_msgs.is_empty() {
                state.cinematic_aoi_hold = None;
                held_for = active.started.elapsed();
                break;
            }
            flushed += state.deferred_aoi_msgs.len();
        }
        flush_deferred_aoi(
            witness_id,
            addr,
            "cinematic_hold_release",
            transport,
            connected,
            entity_to_addr,
        )
        .await;
    }
    tracing::info!(
        target: "aoi.cinematic_hold",
        event = "hold_released",
        %addr,
        witness_id,
        reason = reason.as_str(),
        flushed,
        held_ms = held_for.as_millis() as u64,
        "Cinematic AoI hold: released, held entity introductions flushed"
    );
}

#[cfg(test)]
mod tests;
