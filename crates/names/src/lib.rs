//! # cimmeria-names
//!
//! The seed's names for static content ids, so a log line or a Discord post
//! can carry `ability_name="Staff Blast"` next to `ability_id=880`
//! (named telemetry campaign, `docs/analysis/named-telemetry/`, NT-01).
//!
//! - [`NameBook`]: one read-only snapshot of every name table (items,
//!   abilities, effects, missions with their steps and objectives, dialogs,
//!   dialog sets, speakers, entity templates, monikers, texts, error texts,
//!   stargates, respawners, spawn sets, containers, item lists, applied
//!   sciences, worlds, content chains, loot tables, trainer ability lists,
//!   Kismet event sets and sequences, dialog-set entries). Typed lookups
//!   return `Option<&str>`: `None` for an unknown id, a blank name or a seed
//!   placeholder such as `NO ITEM NAME`.
//! - [`global`] / [`book`]: the process's book, shared by the base and the
//!   cell. [`load_at_boot`] fills it once; [`reload`] swaps in a new one on
//!   content reload.
//! - [`owned`]: the same lookups copied out, for an ID held as an `Option`.
//! - [`archetype_name`], [`racial_paradigm_name`], [`ammo_name`]: names from
//!   small closed tables compiled in, which need no database.
//!
//! Runtime entity ids (recycled slots) are not here: they resolve through
//! the live `SpaceManager` (NT-02).
//!
//! ```ignore
//! let names = cimmeria_names::book();
//! tracing::info!(ability_id, ability_name = names.ability(ability_id), "...");
//! ```

#![warn(unreachable_pub)]

mod book;
mod fixed_tables;
mod handle;
mod load;
pub mod owned;
mod placeholder;

pub use book::{NameBook, Table};
pub use fixed_tables::{
    ammo_name, archetype_name, racial_paradigm_name, ARCHETYPE_NAMES, RACIAL_PARADIGM_NAMES,
};
pub use handle::{book, global, load_at_boot, reload, NameBookHandle, Trigger};
pub use load::{query, LoadReport, TableCount};
pub use placeholder::{classify, is_placeholder, Unresolved};

#[cfg(test)]
mod namebook_live_db_tests;

#[cfg(test)]
mod test_support {
    pub(crate) use cimmeria_test_support::*;
}
