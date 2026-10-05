//! Crafting and applied science: the catalog and the crafting enumerations.
//!
//! [`CraftingCatalog`] holds `resources.disciplines`, `resources.blueprints`
//! with `resources.blueprints_components` grouped into alternative component
//! sets, and the crafting columns of `resources.items`. It is loaded once per
//! process through [`shared_crafting_catalog`], and both the cell and the base
//! read it, so the two sides of the split can never disagree about a recipe
//! (D-CR18, the same arrangement as the ability-tree catalog).
//!
//! Item flags: researchable is [`ItemFlags::CRAFT_RESEARCH`],
//! reverse-engineerable [`ItemFlags::CRAFT_REV_ENG`], a kicker
//! [`ItemFlags::KICKER`]. `ELEMENTARY_COMPONENT` is set on every seeded item
//! and must not be used to decide anything (audit C-24).
//!
//! Campaign ledger: `docs/analysis/crafting/`.

mod catalog;
mod constants;
mod shared;

pub use catalog::{
    Blueprint, Component, ComponentRow, ComponentSet, CraftItemAttrs, CraftingCatalog, Discipline,
};
pub use constants::*;
pub use shared::{loaded_crafting_catalog, shared_crafting_catalog};

#[cfg(test)]
mod tests;
