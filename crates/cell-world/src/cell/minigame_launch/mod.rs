//! The one launch path for a minigame (MG-22).
//!
//! Every launch (a content chain's `StartMinigame` action, and the start
//! dialog and effect-script paths that follow) builds a
//! [`MinigameLaunchRequest`] and calls [`launch_minigame`]. The `StartMinigame`
//! message is built the same way whatever started it, and the player's known
//! abilities go in as `abilityBitfield` (D-MG18). The hold (D-MG15) is taken
//! only after a successful send: a dropped launch never produces a result.

use tokio::sync::mpsc;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::minigame_hold::hold_for_minigame;
use crate::cell::space_manager::SpaceManager;

pub mod modifiers;

#[cfg(test)]
mod tests;

/// Where a launch came from; a log field, never logic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MinigameLaunchRequest {
    pub entity_id: u32,
    pub player_id: i32,
    pub game_name: String,
    pub difficulty: u32,
    pub on_victory_chains: Vec<i64>,
    pub on_failure_chains: Vec<i64>,
    /// `"content_chain"`, `"start_dialog"` or `"effect_script"`.
    pub source: &'static str,
}

/// techcomp sent to every game (D-MG18; README M9).
pub const MINIGAME_TECH_COMPETENCY: u32 = 1;

/// Send `CellToBaseMsg::StartMinigame` and take the hold. True when sent.
///
/// The player's level, intelligence and ability mask are read from the live
/// entity, so a levelled or buffed player is not served 1/0. Fails closed: an
/// entity the cell no longer holds (its id may already be recycled) gets no
/// send and no hold, and the call returns false.
pub async fn launch_minigame(
    req: MinigameLaunchRequest,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
    let Some((player_level, intelligence, abilities_mask)) =
        space_mgr.get_entity(req.entity_id).map(|e| {
            let int = e
                .stats
                .get(cimmeria_entity::stats::INTELLIGENCE)
                .map_or(0, |s| s.cur.max(0) as u32);
            (e.level, int, modifiers::abilities_mask(e))
        })
    else {
        tracing::warn!(
            entity_id = req.entity_id,
            entity_name = space_mgr.entity_names(req.entity_id).entity_name,
            player_id = req.player_id, // nt:id-only the player's name is not held on this path
            game_name = %req.game_name,
            source = req.source,
            reason = "entity_missing",
            "StartMinigame: entity not in the cell -- minigame not launched"
        );
        return false;
    };
    if let Err(e) = tx
        .send(CellToBaseMsg::StartMinigame {
            entity_id: req.entity_id,
            player_id: req.player_id,
            game_name: req.game_name.clone(),
            // Range-checked 1-5 at load time (loader/action.rs);
            // the seed default is 1.
            difficulty: req.difficulty,
            on_victory_chains: req.on_victory_chains.clone(),
            on_failure_chains: req.on_failure_chains.clone(),
            player_level,
            intelligence,
            tech_competency: MINIGAME_TECH_COMPETENCY,
            abilities_mask,
        })
        .await
    {
        // drop here means the minigame never
        // launches but the player click already fired —
        // chain stalls with no signal.
        tracing::warn!(
            entity_id = req.entity_id,
            entity_name = space_mgr.entity_names(req.entity_id).entity_name,
            player_id = req.player_id, // nt:id-only the player's name is not held on this path
            game_name = %req.game_name,
            source = req.source,
            reason = "cell_to_base_send_failed",
            "StartMinigame: cell→base send failed -- minigame will not launch: {e}"
        );
        return false;
    }
    // D-MG15: hold BSF_PlayingMinigame and one counted movement lock until a
    // MinigameResult of any code releases them.
    hold_for_minigame(req.entity_id, tx, space_mgr).await;
    true
}
