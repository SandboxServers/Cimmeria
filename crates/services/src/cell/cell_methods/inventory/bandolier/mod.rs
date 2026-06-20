//! Bandolier slot operations, split by concern:
//! - [`active_slot`]: dirty-ammo persistence flush + the active-slot swap
//!   (holster choreography, per-weapon ability grant).
//! - [`ammo_change`]: the per-slot ammo-type swap.

mod active_slot;
mod ammo_change;

// Re-export discipline: keep the import paths the inventory `mod.rs`
// re-exports and `dispatch.rs` reaches for identical after the split.
pub use active_slot::flush_dirty_bandolier_ammo;
pub(crate) use active_slot::handle_request_active_slot_change;
pub(crate) use ammo_change::handle_request_ammo_change;
