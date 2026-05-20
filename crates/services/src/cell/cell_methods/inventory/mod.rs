//! SGWInventoryManager interface exposed CellMethods (indices 36–42).

mod bandolier;
pub mod constants;
mod dispatch;
mod item_ops;

pub use bandolier::flush_dirty_bandolier_ammo;
pub(crate) use bandolier::handle_request_active_slot_change;
pub use constants::*;
pub use dispatch::dispatch;

#[cfg(test)]
mod tests;
