//! The launch's auto-cycle commit: arm the loop, or clear it for an
//! `AF_DEACTIVATE_AUTO_CYCLE` ability. Runs once the cooldown has started
//! and its timer has gone out, from [`super::handle::handle_use_ability`].

use tokio::sync::mpsc;

use cimmeria_entity::abilities::{
    AbilityDef, AF_DEACTIVATE_AUTO_CYCLE, AF_DO_NOT_ACTIVATE_AUTO_CYCLE,
};

use super::super::super::combat;
use super::super::super::messages::CellToBaseMsg;
use super::super::super::space_manager::SpaceManager;
use super::super::auto_cycle_state::send_auto_cycle_state;

/// The committed cast the loop is classified for.
#[derive(Debug, Clone, Copy)]
pub(super) struct CommitCast {
    pub entity_id: u32,
    pub ability_id: i32,
    pub target_id: i32,
    /// A support shot at an ally or a beneficial cast: never arms the loop.
    pub friendly_cast: bool,
}

/// Three cases after the cooldown has started:
///
/// 1. `AF_DEACTIVATE_AUTO_CYCLE` flag (mask `0x400`) on the firing ability:
///    break the loop. One-shot specials that mustn't auto-repeat.
/// 2. `auto_cycle == true` (button armed): stash the ability id AND set
///    `BSF_AUTO_CYCLING`. The driver tick reads `current_target_id` live at
///    re-fire time, so no target is stashed here.
/// 3. `auto_cycle == false`: no-op.
///
/// A `DoNotActivate_AutoCycle` (512) ability neither arms nor clears the
/// loop: python passed `autoCycle = False` for it (`SGWPlayer.py:1177`). A
/// support shot at an ally never arms it either: auto-cycle is an attack
/// loop, and its tick stops on any player it may not attack.
pub(super) async fn commit_auto_cycle(
    cast: CommitCast,
    ability_def: Option<&AbilityDef>,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let CommitCast {
        entity_id,
        ability_id,
        target_id,
        friendly_cast,
    } = cast;
    let Some(entity) = space_mgr.get_entity(entity_id) else {
        return;
    };
    let (is_player, auto_cycle_armed) = (entity.is_player, entity.abilities.auto_cycle);
    let who = entity.identity();
    let (has_deactivate_flag, never_arms) = ability_def.map_or((false, false), |d| {
        (
            d.flags & AF_DEACTIVATE_AUTO_CYCLE != 0,
            d.flags & AF_DO_NOT_ACTIVATE_AUTO_CYCLE != 0,
        )
    });
    if !(is_player && auto_cycle_armed && !friendly_cast && (has_deactivate_flag || !never_arms)) {
        return;
    }
    if has_deactivate_flag {
        if let Some(new_state) = combat::clear_auto_cycle(space_mgr, entity_id) {
            tracing::info!(
                account_id = who.account_id,
                player_id = who.player_id,
                entity_id,
                ability_id,
                "auto-cycle: cleared by AF_DEACTIVATE_AUTO_CYCLE flag"
            );
            send_auto_cycle_state(entity_id, new_state, tx, space_mgr).await;
        }
    } else if let Some(new_state) =
        combat::arm_auto_cycle(space_mgr, entity_id, ability_id, target_id)
    {
        tracing::info!(
            account_id = who.account_id,
            player_id = who.player_id,
            entity_id,
            ability_id,
            target_id,
            "auto-cycle: armed (first commit) — BSF_AUTO_CYCLING set"
        );
        send_auto_cycle_state(entity_id, new_state, tx, space_mgr).await;
    }
    // Bit-already-set path: `arm_auto_cycle` updates the stash
    // unconditionally; only the `Some(new_state)` branch needs to broadcast.
}
