//! Archetype ability trees: the catalog and the single trainability
//! predicate.
//!
//! Both the cell (trainer window, `trainAbility` purchase gate) and the
//! base (player-load `onAbilityTreeInfo`) read `resources.archetype_ability_tree`.
//! This module owns the one loader for that table ([`AbilityTreeCatalog`])
//! and the one function that decides whether a player may train a node
//! ([`evaluate_train`]).
//!
//! **One predicate, two callers.** The trainer's `trainable` byte and the
//! purchase decision must agree, or the client enables a button whose
//! click does nothing (the client greys a node purely from that byte).
//! So `cell/interactions/trainer.rs` computes the byte as
//! `evaluate_train(..).is_ok()` and `cell/cell_methods/player/vendor/train.rs`
//! forwards a purchase only on `Ok`. Neither file carries a gate of its own.
//!
//! **Adding a gate.** Gates live in `gates/`, one file per family. See
//! `gates/mod.rs` for the recipe: a new file, its `mod` line, its
//! entries in `NODE_GATES`, and one [`TrainReject`] variant per reason.

mod catalog;
mod gates;
use cimmeria_wire::ability_tree::points_property;
mod predicate;

pub use catalog::{AbilityTreeCatalog, TreeNode};
pub use points_property::{training_points_property_args, GENERICPROPERTY_TRAINING_POINTS};
pub use predicate::{evaluate_train, KnownAbilities, TrainContext, TrainPlan, TrainReject};

#[cfg(test)]
mod tests;
