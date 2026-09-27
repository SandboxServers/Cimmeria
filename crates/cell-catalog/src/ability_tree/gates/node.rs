//! Node gates: the ability exists, the player is a loaded character who does
//! not already know it, it is in their archetype's tree, and they meet its
//! level and prerequisites.
//!
//! These are the six checks `train.rs` carried before the predicate existed,
//! in the same order, so the rejection a player gets is unchanged.

use super::super::catalog::TreeNode;
use super::super::predicate::{TrainContext, TrainReject};
use super::Resolved;

/// Find the node, checking in order: ability exists, player id, not already
/// known, archetype, node in the archetype's tree.
pub(in crate::ability_tree) fn resolve<'a>(
    ctx: &TrainContext<'a>,
) -> Result<Resolved<'a>, TrainReject> {
    if !ctx.ability_exists {
        return Err(TrainReject::UnknownAbility);
    }
    let player_id = ctx.player_id.ok_or(TrainReject::NoPlayerId)?;
    if ctx.known.knows(ctx.ability_id) {
        return Err(TrainReject::AlreadyKnown);
    }
    let archetype_id = ctx.archetype_id.ok_or(TrainReject::NoArchetype)?;
    let node = ctx
        .catalog
        .node(archetype_id, ctx.ability_id)
        .ok_or(TrainReject::NotInArchetypeTree)?;
    Ok(Resolved {
        player_id,
        archetype_id,
        node,
    })
}

/// The player's level meets the node's `level`.
pub(super) fn level(ctx: &TrainContext<'_>, node: &TreeNode) -> Result<(), TrainReject> {
    if ctx.level < node.level {
        return Err(TrainReject::LevelTooLow {
            required: node.level,
            actual: ctx.level,
        });
    }
    Ok(())
}

/// Every prerequisite is known. Reports the first missing one in node order.
pub(super) fn prerequisites(ctx: &TrainContext<'_>, node: &TreeNode) -> Result<(), TrainReject> {
    match node.prerequisites.iter().find(|&&p| !ctx.known.knows(p)) {
        Some(&missing) => Err(TrainReject::MissingPrerequisite { missing }),
        None => Ok(()),
    }
}
