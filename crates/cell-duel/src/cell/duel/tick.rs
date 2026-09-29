//! The duel tick: expire unanswered challenges, engage duels whose countdown
//! has run out, and run the tick's ends (range, a dead or departed duelist,
//! the safety limit).
//!
//! Called every AoI tick from the cell loop. It returns at once when the
//! registry is idle, and it opens no span of its own (instrumentation
//! discipline rule 3): each expiry, engage or end is one DEBUG event.
//!
//! - The countdown end is [`engage::on_countdown_end`](super::engage).
//! - The tick's ends are [`end::sweep`](super::end): range (D-SS19), a
//!   dead duelist, a duelist no longer at the engaged entity in the duel's
//!   space, and the `ENGAGED_LIMIT` abort. The event-driven paths (forfeit,
//!   the clamp, death, disconnect, travel) come first in practice; the sweep
//!   is what guarantees no duel stays engaged forever if one is missed.

use super::DuelResources;
use std::time::Instant;

use tokio::sync::mpsc;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

use super::find_player;
use super::response::abort_both;

/// Run the tick on the wall clock.
pub async fn run(tx: &mpsc::Sender<CellToBaseMsg>, mgr: &mut SpaceManager) {
    if mgr.resources.duels().is_idle() {
        return;
    }
    run_at(tx, mgr, Instant::now()).await;
}

/// [`run`] on an explicit clock.
pub async fn run_at(tx: &mpsc::Sender<CellToBaseMsg>, mgr: &mut SpaceManager, now: Instant) {
    for pending in mgr.resources.duels_mut().expire_pending(now) {
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
    for duel_id in mgr.resources.duels().countdowns_due(now) {
        super::engage::on_countdown_end(tx, mgr, duel_id, now).await;
    }
    super::end::sweep(tx, mgr, now).await;
}
