//! Public ring-transport entry points and the per-tick deadline scan.
//!
//! These turn external events (chain action, cell-method call, point-set
//! crossing, player teardown, tick) into FSM transitions on a
//! [`super::transporter::RingTransporter`] plus an `Effect` dispatch via
//! [`super::dispatch`].
//!
//! - [`entry`] — what the outside world calls: `handle_interact`,
//!   `handle_select_destination`, `handle_region_trigger`,
//!   `handle_remote_player_loaded`.
//! - [`tick`] — the 100ms deadline scan that drives every timer, including
//!   the bounded stall aborts, plus the reconciliation of work queued by
//!   synchronous callers that cannot dispatch effects themselves.
//! - [`teardown`] — `forget_player`, the departing-player hook.

mod entry;
mod teardown;
mod tick;

pub use entry::{
    handle_interact, handle_region_trigger, handle_remote_player_loaded, handle_select_destination,
};
pub use teardown::forget_player;
pub use tick::run_tick_with_engine;

/// Re-exported for `super::tests`, which drives the source/destination
/// cross-link by hand to assert one half of the trip at a time. Production
/// reaches it through [`tick::run_one_deadline`], never by name.
#[cfg(test)]
pub(in crate::cell::ring_transport) use tick::advance_destination_after_warmup;
