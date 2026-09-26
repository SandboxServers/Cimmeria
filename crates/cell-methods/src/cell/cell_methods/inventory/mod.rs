//! SGWInventoryManager interface exposed CellMethods (indices 36–42).

// The bandolier slot operations are in `cimmeria-cell-combat` (wave C2);
// imported at their old path.
use cimmeria_cell_combat::cell::cell_methods::inventory::bandolier;
pub use cimmeria_wire::cell::cell_methods::inventory::constants;
mod dispatch;
mod item_ops;

pub use bandolier::flush_dirty_bandolier_ammo;
pub use bandolier::handle_request_active_slot_change;
pub use constants::*;
pub use dispatch::dispatch;

#[cfg(test)]
mod tests;
