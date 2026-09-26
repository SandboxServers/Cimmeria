//! Trainability gates, one file per family.
//!
//! `node::resolve` runs first and finds the tree node; every later gate is a
//! [`NodeGate`] that reads the context and that node. To add a family:
//!
//! 1. Write `gates/<family>.rs` with one `pub(super) fn` per gate, each
//!    returning `Err(TrainReject::<Variant>)` on failure.
//! 2. Add `pub(super) mod <family>;` below and append its gates to
//!    [`NODE_GATES`] in the place their rejection should rank.
//! 3. Add the new [`TrainReject`] variants and their `reason()` names.
//!
//! Both callers pick the new gate up with no further edit: the trainer's
//! `trainable` byte and the purchase gate both call `evaluate_train`.

use super::catalog::TreeNode;
use super::predicate::{TrainContext, TrainReject};

pub(super) mod node;
pub(super) mod spend;

/// A gate over a resolved node.
pub(super) type NodeGate = fn(&TrainContext<'_>, &TreeNode) -> Result<(), TrainReject>;

/// Every gate after resolution, in rejection-priority order.
pub(super) const NODE_GATES: &[NodeGate] = &[
    node::level,
    node::prerequisites,
    spend::branch_points,
    spend::points,
    // Trainer gates (gates/trainer.rs) go here.
];

/// What `node::resolve` establishes before any [`NodeGate`] runs.
pub(super) struct Resolved<'a> {
    pub(super) player_id: i32,
    pub(super) archetype_id: i32,
    pub(super) node: &'a TreeNode,
}
