//! The duel tick: expire unanswered challenges and end countdowns.
//!
//! Called every AoI tick from the cell loop. It returns at once when the
//! registry is idle, and it opens no span of its own (instrumentation
//! discipline rule 3): each expiry or countdown end is one DEBUG event.
//!
//! # The countdown end, until SS-D2
//!
//! SS-D2 turns a finished countdown into an engaged duel (the PvP flag, the
//! harm gate, `onDuelEntitiesSet`). SS-D1 cannot: none of that exists yet,
//! and an `Engaged` duel with no end path (SS-D3) would leave both players
//! busy for the life of the cell process, unable to duel again. So until
//! SS-D2 replaces [`on_countdown_end`], the countdown ends the duel with
//! "Duel aborted" (878) and `reason = engage_not_implemented`.

use std::time::Instant;

use tokio::sync::mpsc;

use cimmeria_wire::cell::client_methods::duel::TEXT_DUEL_ABORTED;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

use super::find_player;
use super::outbound::{send_line, Recipient};
use super::registry::DuelId;
use super::response::abort_both;

/// Run the tick on the wall clock.
pub async fn run(tx: &mpsc::Sender<CellToBaseMsg>, mgr: &mut SpaceManager) {
    if mgr.duels.is_idle() {
        return;
    }
    run_at(tx, mgr, Instant::now()).await;
}

/// [`run`] on an explicit clock.
pub async fn run_at(tx: &mpsc::Sender<CellToBaseMsg>, mgr: &mut SpaceManager, now: Instant) {
    for pending in mgr.duels.expire_pending(now) {
        let challenger = find_player(mgr, pending.challenger);
        tracing::debug!(
            target: "duel",
            event = "duel.challenge_expired",
            duel_id = pending.duel_id,
            account_id = challenger.and_then(|p| p.account_id),
            player_id = pending.challenger,
            entity_id = challenger.map(|p| p.entity_id),
            target_player_id = pending.target,
            reason = "no_answer",
            "duel challenge expired unanswered"
        );
        abort_both(tx, mgr, &pending, "expired").await;
    }
    for duel_id in mgr.duels.countdowns_due(now) {
        on_countdown_end(tx, mgr, duel_id).await;
    }
}

/// SS-D2 replaces this body with the engage (see the module doc).
async fn on_countdown_end(
    tx: &mpsc::Sender<CellToBaseMsg>,
    mgr: &mut SpaceManager,
    duel_id: DuelId,
) {
    let Some(duel) = mgr.duels.end_duel(duel_id) else {
        return;
    };
    let challenger = find_player(mgr, duel.challenger);
    tracing::debug!(
        target: "duel",
        event = "duel.aborted",
        duel_id,
        account_id = challenger.and_then(|p| p.account_id),
        player_id = duel.challenger,
        entity_id = challenger.map(|p| p.entity_id),
        target_player_id = duel.target,
        space_id = duel.space_id,
        state = "start_pending",
        reason = "engage_not_implemented",
        "duel countdown ended: engaging is SS-D2, so the duel is aborted"
    );
    for (player_id, other) in [
        (duel.challenger, duel.target),
        (duel.target, duel.challenger),
    ] {
        if let Some(p) = find_player(mgr, player_id) {
            let to = Recipient::at(&p, player_id, Some(other));
            send_line(tx, to, TEXT_DUEL_ABORTED, Some(duel_id)).await;
        }
    }
}
