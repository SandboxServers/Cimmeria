//! The engage: the countdown has run out, and the duel starts (SS-D2).
//!
//! Both duelists must still be connected, in the duel's space. Then, for
//! each side:
//!
//! 1. `onDuelEntitiesSet([challenger, target])` [151] to their own client;
//! 2. the PvP flag set to 1, to them and their witnesses (D-SS23,
//!    presentation only);
//! 3. the other duelist becomes a combat source (`BSF_InCombat` on);
//! 4. "The duel has begun."
//!
//! From here [`DuelRegistry::can_harm`](super::DuelRegistry::can_harm) is
//! true for the pair, and the four hostility gates let them damage each
//! other (`combat::player_may_attack`). If either duelist has gone, nothing
//! is flagged and the duel is aborted with 878.

use super::DuelResources;
use std::time::Instant;

use tokio::sync::mpsc;

use cimmeria_wire::cell::client_methods::duel::{TEXT_DUEL_ABORTED, TEXT_DUEL_ENGAGED};

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

use super::outbound::{
    send_duel_entities_set, send_line, send_pvp_flag, send_state_field, Recipient,
};
use super::registry::{Duel, DuelId};
use super::{combat, find_player, PlayerAt};

/// Engage `duel_id`, whose countdown has run out, or abort it when a
/// duelist is gone.
pub(super) async fn on_countdown_end(
    tx: &mpsc::Sender<CellToBaseMsg>,
    mgr: &mut SpaceManager,
    duel_id: DuelId,
    now: Instant,
) {
    let Some(duel) = mgr.resources.duels().duel(duel_id).copied() else {
        return;
    };
    let in_space = |p: &PlayerAt| p.space_id == duel.space_id;
    let challenger = find_player(mgr, duel.challenger).filter(in_space);
    let target = find_player(mgr, duel.target).filter(in_space);
    let (Some(challenger), Some(target)) = (challenger, target) else {
        abort_countdown(tx, mgr, &duel, challenger, target).await;
        return;
    };

    let entities = [challenger.entity_id, target.entity_id];
    let Some(duel) = mgr.resources.duels_mut().engage(duel_id, entities, now) else {
        return;
    };
    let mut witnesses = [0usize; 2];
    for (i, (me, my_pid, other, other_pid)) in [
        (challenger, duel.challenger, target, duel.target),
        (target, duel.target, challenger, duel.challenger),
    ]
    .into_iter()
    .enumerate()
    {
        let to = Recipient::at(&me, my_pid, Some(other_pid));
        send_duel_entities_set(tx, to, entities, duel_id).await;
        witnesses[i] = send_pvp_flag(tx, mgr, to, true, duel_id).await;
        if let Some(state) = combat::enter(mgr, me.entity_id, other.entity_id) {
            send_state_field(tx, mgr, to, state, duel_id).await;
        }
        send_line(tx, to, TEXT_DUEL_ENGAGED, Some(duel_id)).await;
    }
    tracing::debug!(
        target: "duel",
        event = "duel.engaged",
        duel_id,
        account_id = challenger.account_id,
        player_id = duel.challenger,
        entity_id = challenger.entity_id,
        target_player_id = duel.target,
        target_entity_id = target.entity_id,
        target_account_id = target.account_id,
        space_id = duel.space_id,
        state = "engaged",
        pvp_flag_witnesses = witnesses[0],
        target_pvp_flag_witnesses = witnesses[1],
        "duel engaged: both flagged, the pair may harm each other"
    );
}

/// A duelist left the world or the space during the countdown. Nothing was
/// flagged yet, so the duel is dropped and whoever is left hears 878.
async fn abort_countdown(
    tx: &mpsc::Sender<CellToBaseMsg>,
    mgr: &mut SpaceManager,
    duel: &Duel,
    challenger: Option<PlayerAt>,
    target: Option<PlayerAt>,
) {
    mgr.resources.duels_mut().end_duel(duel.duel_id);
    let gone = match (challenger.is_some(), target.is_some()) {
        (false, false) => "both",
        (false, true) => "challenger",
        _ => "target",
    };
    tracing::debug!(
        target: "duel",
        event = "duel.engage_refused",
        duel_id = duel.duel_id,
        account_id = challenger.and_then(|p| p.account_id),
        player_id = duel.challenger,
        entity_id = challenger.map(|p| p.entity_id),
        target_player_id = duel.target,
        target_entity_id = target.map(|p| p.entity_id),
        space_id = duel.space_id,
        gone,
        reason = "duelist_gone",
        "duel countdown ended with a duelist out of the world or the space; aborted"
    );
    // Tell anyone still in the world, including a duelist who only changed
    // space: the line is their feedback that the duel will not start.
    for (my_pid, other_pid) in [
        (duel.challenger, duel.target),
        (duel.target, duel.challenger),
    ] {
        if let Some(p) = find_player(mgr, my_pid) {
            let to = Recipient::at(&p, my_pid, Some(other_pid));
            send_line(tx, to, TEXT_DUEL_ABORTED, Some(duel.duel_id)).await;
        }
    }
}
