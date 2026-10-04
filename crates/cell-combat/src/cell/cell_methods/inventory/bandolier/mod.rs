//! Bandolier slot operations, split by concern:
//! - [`active_slot`]: dirty-ammo persistence flush + the active-slot swap
//!   (holster choreography, per-weapon ability grant).
//! - [`ammo_change`]: the per-slot ammo-type swap.
//! - [`switch_return`]: unfired special rounds back to the bags on an
//!   ammo-type swap (ammo campaign AM-02; created empty by AM-F).

mod active_slot;
mod ammo_change;
mod switch_return;

// Re-export discipline: keep the import paths the inventory `mod.rs`
// re-exports and `dispatch.rs` reaches for identical after the split.
pub use active_slot::flush_dirty_bandolier_ammo;
pub use active_slot::handle_request_active_slot_change;
pub use active_slot::weapon_ability_set;
pub use ammo_change::handle_request_ammo_change;
pub use switch_return::{begin_switch_return, handle_switch_returned, SwitchReturn};
