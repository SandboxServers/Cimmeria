//! What the GM `.duel_status` and `.duel_end` console commands (SS-U2) need
//! from the duel module: a read of one player's registry entry, and a GM
//! abort that tells both players.
//!
//! The console handlers in `cimmeria-cell-console` resolve the typed name
//! and write the GM's feedback; this file owns the registry and the sends
//! to the duelists, like the other duel handlers.

use tokio::sync::mpsc;

use cimmeria_wire::cell::client_methods::duel::TEXT_DUEL_ABORTED;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

use super::find_player;
use super::outbound::{send_line, Recipient};
use super::registry::{Duel, DuelRegistry, GmAborted, PendingChallenge};

/// One player's entry in the registry, as `.duel_status` shows it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DuelStatus {
    /// Not in a duel and no challenge either way.
    Idle,
    /// In a duel (the countdown or the fight).
    InDuel(Duel),
    /// A challenge addressed to the player, waiting for their answer.
    Challenged(PendingChallenge),
    /// A challenge the player sent, waiting for the target's answer.
    Challenging(PendingChallenge),
}

/// `player_id`'s entry. The busy rule allows at most one of the three.
pub fn status(reg: &DuelRegistry, player_id: i32) -> DuelStatus {
    if let Some(d) = reg.duel_of(player_id) {
        DuelStatus::InDuel(*d)
    } else if let Some(p) = reg.pending_for(player_id) {
        DuelStatus::Challenged(*p)
    } else if let Some(p) = reg.challenge_from(player_id) {
        DuelStatus::Challenging(*p)
    } else {
        DuelStatus::Idle
    }
}

/// The character name of the connected player playing `player_id`, if any.
pub fn online_name(mgr: &SpaceManager, player_id: i32) -> Option<String> {
    let p = find_player(mgr, player_id)?;
    mgr.get_entity(p.entity_id)?.character_name.clone()
}

/// GM abort: remove whatever `subject_player_id` is part of and send "Duel
/// aborted" (878) to both players who are still in the world. No pair
/// cooldown starts. `None` when there was nothing to end; the caller logs
/// that refusal.
///
/// `gm_entity_id` is only for the log row.
pub async fn gm_end(
    tx: &mpsc::Sender<CellToBaseMsg>,
    mgr: &mut SpaceManager,
    gm_entity_id: u32,
    subject_player_id: i32,
) -> Option<GmAborted> {
    let aborted = mgr.duels.gm_abort(subject_player_id)?;
    let (challenger, target) = aborted.players();
    let opponent = if subject_player_id == challenger {
        target
    } else {
        challenger
    };
    let gm = mgr.player_identity(gm_entity_id);
    tracing::info!(
        target: "duel",
        event = "duel.gm_ended",
        account_id = gm.account_id,
        player_id = gm.player_id,
        entity_id = gm_entity_id,
        subject_player_id,
        opponent_player_id = opponent,
        duel_id = aborted.duel_id(),
        stage = aborted.stage(),
        "GM ended a duel"
    );
    for (player_id, other) in [(challenger, target), (target, challenger)] {
        if let Some(p) = find_player(mgr, player_id) {
            let to = Recipient::at(&p, player_id, Some(other));
            send_line(tx, to, TEXT_DUEL_ABORTED, Some(aborted.duel_id())).await;
        } else {
            tracing::debug!(
                target: "duel",
                event = "duel.notify_skipped",
                player_id,
                target_player_id = other,
                duel_id = aborted.duel_id(),
                why = "gm_end",
                reason = "player_not_in_world",
                "duelist not in the world; no abort line sent"
            );
        }
    }
    Some(aborted)
}
