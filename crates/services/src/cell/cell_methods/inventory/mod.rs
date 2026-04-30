//! SGWInventoryManager interface exposed CellMethods (indices 36–42).

pub mod constants;
mod dispatch;
mod item_ops;
mod bandolier;
mod weapon_events;

pub use constants::*;
pub use dispatch::dispatch;
pub use bandolier::flush_dirty_bandolier_ammo;
pub use weapon_events::fire_equipped_weapon_attack_event;

#[cfg(test)]
mod tests;
