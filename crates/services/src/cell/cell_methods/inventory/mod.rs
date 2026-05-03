//! SGWInventoryManager interface exposed CellMethods (indices 36–42).

mod bandolier;
pub mod constants;
mod dispatch;
mod item_ops;
mod weapon_events;

pub use bandolier::flush_dirty_bandolier_ammo;
pub use constants::*;
pub use dispatch::dispatch;
pub use weapon_events::fire_equipped_weapon_attack_event;

#[cfg(test)]
mod tests;
