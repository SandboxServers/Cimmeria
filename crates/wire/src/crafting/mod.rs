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
//! - [`request`]: [`CraftRequest`] / [`CraftVerb`], the verbs the cell
//!   parses from methods 95-100.
//! - [`gm_points`]: [`GmGrantExpertise`] and [`GmGrantAppliedSciencePoints`],
//!   the GM expertise and ASP grants.
//!
//! The payloads the cell sends to the base ([`CraftRequest`],
//! [`CraftingStations`], [`GmAllCraft`], [`GmCraftGrant`], [`RespecCraftOpen`]
//! and the two GM point grants) travel in the `CellToBaseMsg::Plugin`
//! envelope (#962 step 5), consumed by `cimmeria-base-crafting`'s
//! `CraftingPlugin`. The base's declared envelope list
//! (`cimmeria-base-session`'s `base::plugin::PLUGIN_CELL_MESSAGES`) names
//! each, so a missing consumer fails the base's startup check.
//! - [`stations`]: [`CraftingStations`], the station set the cell reports.
//! - [`gm_allcraft`]: [`GmAllCraft`], the GM `.allcraft` grant.
//! - [`gm_craft_grant`]: [`GmCraftGrant`], the GM `.craftkit` and
//!   `.learnblueprint` grants.
//! - [`respec`]: [`RespecCraftOpen`], a player's `.respeccraft`.
//!
//! Campaign ledger: `docs/analysis/crafting/`. The `CraftingOptions` layout
//! is recorded in `docs/protocol/client-method-dispatch-table.md` (row 140).

pub mod client_methods;
pub mod gm_allcraft;
pub mod gm_craft_grant;
pub mod gm_points;
pub mod request;
pub mod respec;
pub mod stations;

pub use client_methods::{
    applied_science_points_property_args, crafting_options_args, crafting_respec_prompt_args,
    discipline_respec_args, known_crafts_args, racial_paradigm_level_args, update_discipline_args,
    CraftingInfo, CraftingOptions, GENERICPROPERTY_APPLIED_SCIENCE_POINTS,
};
pub use gm_allcraft::GmAllCraft;
pub use gm_craft_grant::{GmCraftGrant, GmCraftGrantKind};
pub use gm_points::{GmGrantAppliedSciencePoints, GmGrantExpertise};
pub use request::{CraftRequest, CraftVerb};
pub use respec::RespecCraftOpen;
pub use stations::{CraftingStations, StationChangeCause, StationSet};

#[cfg(test)]
mod tests;
