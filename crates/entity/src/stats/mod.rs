//! Combat stat system for beings (players and NPCs).
//!
//! Each stat has two sets of values:
//! - **Base** (`base_min`/`base_cur`/`base_max`): Change on level-up, archetype, etc.
//! - **Dynamic** (`min`/`cur`/`max`): Change due to buffs, debuffs, equipment, effects.
//!
//! The wire format for stat updates is `StatUpdateList` from `custom_alias.xml`:
//! `count:u32`, then per entry: `stat_id:i32, min:i32, current:i32, max:i32`.
//!
//! This corresponds to the Python `Stat` class in `python/cell/SGWBeing.py:40`
//! and stat IDs from `python/Atrea/enums.py:295`.

mod archetype;
mod stat;
mod stat_ids;
mod stat_list;

#[cfg(test)]
mod tests;

pub use archetype::ArchetypeStatValues;
pub use stat::Stat;
pub use stat_ids::*;
pub use stat_list::StatList;
