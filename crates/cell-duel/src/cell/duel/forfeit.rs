//! `duelForfeit` (CM 103): surrender (CAT-M-14).
//!
//! The call carries no arguments. It acts only on the caller's own engaged
//! duel, at the entity that was engaged, read from the caller's cell entity:
//! a client cannot forfeit anyone else's duel. Anything else (no duel, a
//! challenge still waiting, the countdown) is refused with "You cannot
//! forfeit a duel until you are engaged in one" (880), and nothing changes.

use super::DuelResources;
use tokio::sync::mpsc;
use tracing::Instrument;

use cimmeria_wire::cell::client_methods::duel::TEXT_FORFEIT_NOT_ENGAGED;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

use super::connected_player;
use super::duelist_names as names;
use super::end::{end_engaged, DefeatReason, EndReason};
use super::outbound::{send_line, Recipient};
use super::registry::DuelState;

/// Handle `duelForfeit` from `entity_id`.
pub async fn handle(entity_id: u32, tx: &mpsc::Sender<CellToBaseMsg>, mgr: &mut SpaceManager) {
    let id = mgr.player_identity(entity_id);
    let span = tracing::info_span!(
        target: "duel",
        "duel.forfeit",
        account_id = id.account_id,
        player_id = id.player_id,
        entity_id,
    );
    forfeit(entity_id, tx, mgr).instrument(span).await;
}

async fn forfeit(entity_id: u32, tx: &mpsc::Sender<CellToBaseMsg>, mgr: &mut SpaceManager) {
    let id = mgr.player_identity(entity_id);
    let Some((pid, me)) = id
        .player_id
        .and_then(|pid| connected_player(mgr, entity_id, pid).map(|p| (pid, p)))
    else {
        tracing::warn!(
            target: "duel",
            event = "duel.forfeit_refused",
            account_id = id.account_id,
            account_name = id.account_name,
            entity_id,
            entity_name = names::entity_name(mgr, Some(entity_id)),
            reason = "not_a_player",
            "duelForfeit from an entity that is not a connected player"
        );
        return;
    };
    let duel = mgr.resources.duels().duel_of(pid).copied();
    let engaged = duel.filter(|d| {
        matches!(d.state, DuelState::Engaged { .. })
            && d.engaged_entities.is_some_and(|e| e.contains(&entity_id))
    });
    let Some(duel) = engaged else {
        let stage = match duel.map(|d| d.state) {
            Some(DuelState::StartPending { .. }) => "countdown",
            Some(DuelState::Engaged { .. }) => "engaged_elsewhere",
            None if mgr.resources.duels().is_busy(pid) => "challenge",
            None => "none",
        };
        tracing::debug!(
            target: "duel",
            event = "duel.forfeit_refused",
            account_id = id.account_id,
            account_name = id.account_name,
            player_id = pid,
            player_name = id.player_name,
            entity_id,
            entity_name = names::entity_name(mgr, Some(entity_id)),
            target_player_id = duel.and_then(|d| d.opponent_of(pid)),
            target_player_name = duel.and_then(|d| d.opponent_of(pid)).and_then(|o| names::player_name_of(mgr, o)),
            duel_id = duel.map(|d| d.duel_id), // nt:id-only duel row id with no name column; the duelists are named in the same event
            stage,
            reason = "not_engaged",
            "duelForfeit from a player who is not in an engaged duel"
        );
        let to = Recipient::at(&me, pid, duel.and_then(|d| d.opponent_of(pid)));
        send_line(tx, to, TEXT_FORFEIT_NOT_ENGAGED, duel.map(|d| d.duel_id)).await;
        return;
    };
    end_engaged(
        tx,
        mgr,
        duel.duel_id,
        EndReason::defeat(pid, DefeatReason::Forfeit),
    )
    .await;
}
