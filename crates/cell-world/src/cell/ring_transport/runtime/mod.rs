//! The departing-player hook, the world-side half of the ring runtime.
//!
//! - [`teardown`] — `forget_player`, called from
//!   `SpaceManager::disconnect_entity`, and `dispatch_release_effects`, the
//!   release-only effect dispatch the abort paths share.
//!
//! The public entry points (`handle_interact`, `handle_select_destination`,
//! `handle_region_trigger`, `handle_remote_player_loaded`) and the per-tick
//! deadline scan fire content chains and live above this crate, in
//! `cimmeria-cell-content`'s `cell::ring_transport::runtime`, which re-exports this
//! module beside them.

pub mod teardown;

pub use teardown::{dispatch_release_effects, forget_player};
