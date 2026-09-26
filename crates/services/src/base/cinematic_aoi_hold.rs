//! The session half of the first-login cinematic AoI hold: the hold record
//! stored on [`ConnectedClientState::cinematic_aoi_hold`] and [`begin`],
//! which starts one.
//!
//! While a hold is set, `deferred_aoi::should_hold_entity_traffic` buffers
//! entity introductions so they reach the client after the intro movie and
//! its exit GC, not during it. Arming the timeout and releasing the hold
//! (`arm_timeout`, `release_on_cancel`, `HOLD_DURATION`) flush the buffer
//! through the world-entry AoI dispatch, so they live with it in
//! `world_entry_appearance::cinematic_aoi_hold`, which also documents the bug
//! the hold guards against. The two halves were split so the session state
//! does not depend on world entry (docs/architecture/services-crate-split.md
//! §2I).

use std::sync::atomic::{AtomicU64, Ordering};

use tokio::time::Instant;

use super::ConnectedClientState;

/// An active hold, stored on [`ConnectedClientState::cinematic_aoi_hold`].
#[derive(Clone, Copy, Debug)]
pub(crate) struct CinematicAoiHold {
    /// Distinguishes this hold from a later one on the same session, so a
    /// stale timeout task cannot release a hold it did not start.
    pub token: u64,
    /// When the hold began — the origin `HOLD_DURATION` is measured from.
    pub started: Instant,
    /// A release has claimed this hold and is flushing it. A second releaser
    /// (the timeout racing `cancelMovie`) must leave it alone: see
    /// `world_entry_appearance::cinematic_aoi_hold::release`.
    pub releasing: bool,
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
