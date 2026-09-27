//! Crafting on the wire: the server-to-client crafting payloads and the
//! crafting request the cell forwards to the base.
//!
//! - [`client_methods`]: argument bytes for `onCraftingRespecPrompt` (112),
//!   `onUpdateDiscipline` (136, re-exported from `cimmeria-entity`),
//!   `onDisciplineRespec` (137), `onUpdateRacialParadigmLevel` (138),
//!   `onUpdateKnownCrafts` (139) and `onUpdateCraftingOptions` (140). Each is
//!   byte-exact tested; the index constants are in
//!   `cell::client_methods::player`.
//! - [`request`]: [`CraftRequest`] / [`CraftVerb`], carried by
//!   `CellToBaseMsg::Crafting`.
//!
//! Campaign ledger: `docs/analysis/crafting/`. The `CraftingOptions` layout
//! is recorded in `docs/protocol/client-method-dispatch-table.md` (row 140).

pub mod client_methods;
pub mod request;

pub use client_methods::{
    crafting_options_args, crafting_respec_prompt_args, discipline_respec_args, known_crafts_args,
    racial_paradigm_level_args, update_discipline_args, CraftingInfo, CraftingOptions,
};
pub use request::{CraftRequest, CraftVerb};

#[cfg(test)]
mod tests;
