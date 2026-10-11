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
//!
//! # Lab chaos `Delay`
//!
//! A `lab-chaos` build reuses the hold for the CH-07 `Delay` flush mode
//! (`chaos_delay.rs`): world entry begins a hold for a session with a delay
//! set even without a cinematic, and every release waits the delay out on a
//! spawned timer before it drains. The receive path never sleeps.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use cimmeria_mercury::transport::Transport;

use super::super::world_entry::cell_dispatch::{flush_deferred_aoi, flush_deferred_self_methods};
use super::super::ConnectedClientState;

// The hold record and `begin` are session state (`base::cinematic_aoi_hold`);
// arming the timeout and releasing the hold flush through the world-entry AoI
// dispatch, so they stay here.
pub(crate) use super::super::cinematic_aoi_hold::{begin, CinematicAoiHold};

/// How long entity introductions wait when the movie runs to its natural
/// end.
///
/// `Cine-SGWLogo` is 13.10 s (314 frames @ 23.976 fps — see the BIK-header
/// note in [`super::cinematic`]). The remainder covers the cinematic-exit GC.
/// The player is reading the intro dialog by then: in the 2026-09-19 repro
/// the first input came 16.4 s after `onClientReady`, so NPCs arriving at
/// 16 s land behind that dialog rather than popping into an empty room.
pub(crate) const HOLD_DURATION: Duration = Duration::from_secs(16);

/// Why a hold ended — the `reason` field on the release log line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ReleaseReason {
    CancelMovie,
    Timeout,
    /// `onClientReady` of a world entry with no cinematic: only a hold begun
    /// for a lab chaos `Delay` ends this way ([`on_client_ready`]).
    ClientReady,
}

impl ReleaseReason {
    fn as_str(self) -> &'static str {
        match self {
            Self::CancelMovie => "cancel_movie",
            Self::Timeout => "timeout",
            Self::ClientReady => "client_ready",
        }
    }
}

/// True when world entry should hold this witness's AoI traffic although no
/// cinematic plays: a lab chaos `Delay` is set on the session. Always false
/// without the `lab-chaos` feature. Called under the `connected` lock (lock
/// order: `connected`, then the chaos registry).
pub(crate) fn chaos_wants_hold(addr: SocketAddr, witness_id: u32) -> bool {
    #[cfg(feature = "lab-chaos")]
    {
        chaos_delay::wanted(addr, witness_id)
    }
    #[cfg(not(feature = "lab-chaos"))]
    {
        let _ = (addr, witness_id);
        false
    }
}

/// `onClientReady`'s flush for a session world entry put on hold: the
/// player-self method calls go now, the entity traffic stays held. A
/// cinematic hold (`cinematic`) arms its timeout. A hold begun only for a
/// lab chaos `Delay` (see [`chaos_wants_hold`]) is released here, where the
/// ordinary flush would have run; the release claims it and, with the delay
/// still set, returns at once and drains from a timer.
pub(crate) async fn on_client_ready(
    hold: CinematicAoiHold,
    cinematic: bool,
    witness_id: u32,
    addr: SocketAddr,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    flush_deferred_self_methods(witness_id, addr, transport, connected, entity_to_addr).await;
    if cinematic {
        arm_timeout(hold, witness_id, addr, transport, connected, entity_to_addr);
        return;
    }
    release(
        Some(hold.token),
        ReleaseReason::ClientReady,
        witness_id,
        addr,
        transport,
        connected,
        entity_to_addr,
    )
    .await;
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
        witness_name = crate::base::session_identity::identity_for_addr(&connected, addr).player_name,
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

/// Log a release that found no hold to end, at DEBUG. `witness_name` is `None`
/// when the session itself is gone, so there is no name to resolve.
fn log_skipped(
    witness_id: u32,
    addr: SocketAddr,
    witness_name: Option<&str>,
    reason: &'static str,
) {
    tracing::debug!(
        target: "aoi.cinematic_hold",
        event = "hold_release_skipped",
        %addr,
        witness_id,
        witness_name,
        reason,
        "Cinematic AoI hold: release found no hold to end"
    );
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
///
/// A lab chaos `Delay` is read once, right after the claim: the drain then
/// runs from a spawned timer and this returns at once, with the hold still
/// up so live AoI traffic keeps buffering behind the held introductions.
/// The claim happens once per hold, so the delay does too.
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
        let Some(state) = clients.get_mut(&addr) else {
            log_skipped(witness_id, addr, None, "session_gone");
            return;
        };
        let Some(hold) = state.cinematic_aoi_hold.as_mut() else {
            log_skipped(witness_id, addr, state.player_name.as_deref(), "no_hold");
            return;
        };
        if hold.releasing {
            log_skipped(
                witness_id,
                addr,
                state.player_name.as_deref(),
                "already_releasing",
            );
            return;
        }
        if expected.is_some_and(|token| token != hold.token) {
            log_skipped(
                witness_id,
                addr,
                state.player_name.as_deref(),
                "token_mismatch",
            );
            return;
        }
        hold.releasing = true;
        hold.token
    };

    #[cfg(feature = "lab-chaos")]
    if chaos_delay::defer_release(
        claimed,
        reason,
        witness_id,
        addr,
        transport,
        connected,
        entity_to_addr,
    ) {
        return;
    }
    drain_and_lift(
        claimed,
        reason,
        witness_id,
        addr,
        transport,
        connected,
        entity_to_addr,
    )
    .await;
}

/// The release's flush loop for the hold `claimed`, then the lift.
async fn drain_and_lift(
    claimed: u64,
    reason: ReleaseReason,
    witness_id: u32,
    addr: SocketAddr,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    let mut flushed = 0usize;
    let held_for;
    loop {
        {
            let Ok(mut clients) = connected.lock() else {
                return;
            };
            let Some(state) = clients.get_mut(&addr) else {
                log_skipped(witness_id, addr, None, "session_gone");
                return;
            };
            let Some(active) = state.cinematic_aoi_hold else {
                log_skipped(witness_id, addr, state.player_name.as_deref(), "no_hold");
                return;
            };
            if active.token != claimed {
                log_skipped(
                    witness_id,
                    addr,
                    state.player_name.as_deref(),
                    "token_mismatch",
                );
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
        witness_name = crate::base::session_identity::identity_for_addr(connected, addr).player_name,
        reason = reason.as_str(),
        flushed,
        held_ms = held_for.as_millis() as u64,
        "Cinematic AoI hold: released, held entity introductions flushed"
    );
}

#[cfg(feature = "lab-chaos")]
mod chaos_delay;

#[cfg(test)]
mod tests;
#[cfg(all(test, feature = "lab-chaos"))]
mod tests_chaos_delay;
