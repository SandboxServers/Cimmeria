//! Crafting on the wire: the server-to-client crafting payloads and the
//! crafting request the cell forwards to the base.
//!
//! - [`client_methods`]: argument bytes for `onCraftingRespecPrompt` (112),
//!   `onUpdateDiscipline` (136, re-exported from `cimmeria-entity`),
//!   `onDisciplineRespec` (137), `onUpdateRacialParadigmLevel` (138),
//!   `onUpdateKnownCrafts` (139), `onUpdateCraftingOptions` (140) and the
//!   ASP entity property (`onEntityProperty(2, total)`). Each is
//!   byte-exact tested; the index constants are in
//!   `cell::client_methods::player`.
//! - [`request`]: [`CraftRequest`] / [`CraftVerb`], carried by
//!   `CellToBaseMsg::Crafting`.
//! - [`cell_events`]: [`CraftingStations`] and [`GmAllCraft`], the other two
//!   crafting messages the cell sends the base.
//!
//! Campaign ledger: `docs/analysis/crafting/`. The `CraftingOptions` layout
//! is recorded in `docs/protocol/client-method-dispatch-table.md` (row 140).

pub mod cell_events;
pub mod client_methods;
pub mod request;

pub use cell_events::{CraftingStations, GmAllCraft, StationSet};
pub use client_methods::{
    applied_science_points_property_args, crafting_options_args, crafting_respec_prompt_args,
    discipline_respec_args, known_crafts_args, racial_paradigm_level_args, update_discipline_args,
    CraftingInfo, CraftingOptions, GENERICPROPERTY_APPLIED_SCIENCE_POINTS,
};
pub use request::{CraftRequest, CraftVerb};

#[cfg(test)]
mod tests;
