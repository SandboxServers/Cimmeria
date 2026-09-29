//! What the GM `.duel_status` and `.duel_end` console commands (SS-U2) need
//! from the duel module: a read of one player's registry entry, and a GM
//! abort that tells both players.
//!
//! The console handlers in `cimmeria-cell-console` resolve the typed name
//! and write the GM's feedback; this file owns the registry and the sends
//! to the duelists, like the other duel handlers.

use super::DuelResources;
use tokio::sync::mpsc;

use cimmeria_wire::cell::client_methods::duel::TEXT_DUEL_ABORTED;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

use super::end::{end_engaged, EndReason};
use super::find_player;
use super::outbound::{send_line, Recipient};
use super::registry::{Duel, DuelRegistry, DuelState, GmAborted, PendingChallenge};

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

/// GM abort: end whatever `subject_player_id` is part of. No pair cooldown
/// starts. `None` when there was nothing to end; the caller logs that
/// refusal.
///
/// An engaged duel ends through [`end_engaged`] with
/// [`EndReason::GmAborted`], which clears both PvP flags, sends
/// `onDuelEntitiesClear` [153], drops the combat pair, strips the partner's
/// effects and sends "Duel aborted" (878). A challenge or a countdown has
/// none of that to undo: it is removed from the registry here and both
/// players still in the world get 878.
///
/// `gm_entity_id` is only for the log row.
pub async fn gm_end(
    tx: &mpsc::Sender<CellToBaseMsg>,
    mgr: &mut SpaceManager,
    gm_entity_id: u32,
    subject_player_id: i32,
) -> Option<GmAborted> {
    let engaged = mgr
        .resources
        .duels()
        .duel_of(subject_player_id)
        .filter(|d| matches!(d.state, DuelState::Engaged { .. }))
        .map(|d| d.duel_id);
    if let Some(duel_id) = engaged {
        if let Some(ended) = end_engaged(tx, mgr, duel_id, EndReason::GmAborted).await {
            let aborted = GmAborted::Duel(ended);
            log_gm_ended(mgr, gm_entity_id, subject_player_id, &aborted);
            return Some(aborted);
        }
    }
    let aborted = mgr.resources.duels_mut().gm_abort(subject_player_id)?;
    log_gm_ended(mgr, gm_entity_id, subject_player_id, &aborted);
    let (challenger, target) = aborted.players();
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

/// The `duel.gm_ended` audit row: the GM's ids and the subject.
fn log_gm_ended(
    mgr: &SpaceManager,
    gm_entity_id: u32,
    subject_player_id: i32,
    aborted: &GmAborted,
) {
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
}
