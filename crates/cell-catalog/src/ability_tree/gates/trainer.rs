//! Trainer authority gates: a purchase must happen at a trainer that offers
//! the node and that the player can still reach.
//!
//! Without these a forged `trainAbility` trained from anywhere (audit A-06):
//! the node gates only ask whether the player *may* learn the ability, never
//! whether anyone is teaching it.
//!
//! The predicate never touches `SpaceManager`, so the caller resolves the
//! player's pinned `last_interaction_target` into a [`TrainerPin`] and these
//! gates only read it. On the cell that resolution is
//! `cell::interactions::trainer_pin`, and its range test is the same
//! `interact_target_in_range` the `interact` handler uses, so the trainer
//! gate adds no distance constant of its own.

use super::super::catalog::TreeNode;
use super::super::predicate::{TrainContext, TrainReject};

/// What the player's pinned interaction target says about trainer authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrainerPin<'a> {
    /// No `last_interaction_target`: the player never interacted with anyone.
    Unpinned,
    /// The pinned entity no longer exists (despawned, or the player changed
    /// space and the pin is stale).
    Despawned,
    /// The pinned entity exists but its template has no trainer list.
    NotATrainer,
    /// The pinned entity is a trainer.
    Trainer {
        /// What the trainer's list offers the player's archetype.
        offered: &'a [i32],
        /// The pinned trainer still passes `interact_target_in_range`.
        in_range: bool,
    },
}

/// A trainer is pinned, and it still exists.
pub(super) fn pinned(ctx: &TrainContext<'_>, _node: &TreeNode) -> Result<(), TrainReject> {
    match ctx.trainer {
        TrainerPin::Unpinned => Err(TrainReject::NoTrainerPinned),
        TrainerPin::Despawned => Err(TrainReject::TrainerDespawned),
        TrainerPin::NotATrainer => Err(TrainReject::PinNotATrainer),
        TrainerPin::Trainer { .. } => Ok(()),
    }
}

/// The pinned trainer offers this node to this archetype.
pub(super) fn offered(ctx: &TrainContext<'_>, node: &TreeNode) -> Result<(), TrainReject> {
    match ctx.trainer {
        TrainerPin::Trainer { offered, .. } if !offered.contains(&node.ability_id) => {
            Err(TrainReject::NotOfferedByTrainer)
        }
        _ => Ok(()),
    }
}

/// The player is still within interaction range of the pinned trainer.
pub(super) fn in_range(ctx: &TrainContext<'_>, _node: &TreeNode) -> Result<(), TrainReject> {
    match ctx.trainer {
        TrainerPin::Trainer {
            in_range: false, ..
        } => Err(TrainReject::TrainerOutOfRange),
        _ => Ok(()),
    }
}
