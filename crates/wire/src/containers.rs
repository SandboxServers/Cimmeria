//! Inventory container capacities: a re-export, not a table.
//!
//! The one capacity table is `cimmeria_entity::inventory::bag_max_slots`
//! (D-BV06). It lives in `cimmeria-entity` because `BAG_SIZES`, which
//! `onBagInfo` sends, is derived from it there, and this crate depends on
//! `cimmeria-entity` rather than the other way round.
//!
//! This path is kept for the callers that already use it: the cell's
//! bandolier slot check, and `cimmeria-resources`, which re-exports it again
//! as `base::resources::bag_max_slots`.

pub use cimmeria_entity::inventory::bag_max_slots;
