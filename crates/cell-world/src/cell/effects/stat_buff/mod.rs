//! The stat-buff ledger's `SpaceManager` face: timed buffs to primary
//! attributes (the consumable stimpacks, decision 28 of
//! `docs/architecture/abilities-and-effects-system.md`).
//!
//! The `StatBuff` script that applies and removes them is in
//! `cimmeria-cell-effect-scripts` (`cell::effects::stat_buff`, which
//! re-exports this module), with the stimpack table. What stays here is the
//! logged ledger API the script and the cell's stat-buff tick in
//! `cimmeria-cell-combat` both call: `SpaceManager::apply_stat_buff`,
//! `remove_stat_buffs` and [`StatBuffRemoval`], the reason a buff came off.
//!
//! Log target `abilities`.

mod ledger;

pub use ledger::StatBuffRemoval;
