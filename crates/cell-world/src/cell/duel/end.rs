//! Ending an engaged duel: the one clear every end path goes through.
//!
//! [`end_engaged`] removes the duel from the registry (so `can_harm` is false
//! from that instant), then, for each duelist still at the entity that was
//! engaged:
//!
//! 1. the PvP flag back to 0, to them and their witnesses;
//! 2. `onDuelEntitiesClear()` [153] to their own client;
//! 3. the other duelist dropped as a combat source (`BSF_InCombat` clears
//!    unless a mob still holds them);
//! 4. "Duel aborted" (878).
//!
//! SS-D2 has only the safety ends ([`sweep`]): an engaged duel older than
//! [`ENGAGED_LIMIT`](super::limits::ENGAGED_LIMIT), and one whose duelist is
//! no longer at the engaged entity in the duel's space (logged out, gate
//! travel). SS-D3 adds health, forfeit, range, disconnect and teleport, each
//! with its own reason and result text, through the same clear.

use std::time::Instant;

use tokio::sync::mpsc;

use cimmeria_wire::cell::client_methods::duel::TEXT_DUEL_ABORTED;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

use super::outbound::{
    send_duel_entities_clear, send_line, send_pvp_flag, send_state_field, Recipient,
};
use super::registry::{DuelId, DuelState};
use super::{combat, connected_player, find_player};

/// Why an engaged duel ended, for the `reason` log field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EndReason {
    /// Engaged for [`ENGAGED_LIMIT`](super::limits::ENGAGED_LIMIT): the
    /// backstop for an end path that never fired.
    EngagedLimit,
    /// A duelist is no longer at the engaged entity in the duel's space.
    DuelistGone,
}

impl EndReason {
    /// Stable value for the `reason` log field.
    pub fn reason(self) -> &'static str {
        match self {
            EndReason::EngagedLimit => "engaged_limit",
            EndReason::DuelistGone => "duelist_gone",
        }
    }
}

/// End the engaged duel `duel_id` and clear everything the engage set.
/// Does nothing when the duel is gone or not engaged.
pub async fn end_engaged(
    tx: &mpsc::Sender<CellToBaseMsg>,
    mgr: &mut SpaceManager,
    duel_id: DuelId,
    reason: EndReason,
) {
    let Some(duel) = mgr.duels.duel(duel_id).copied() else {
        return;
    };
    let (DuelState::Engaged { .. }, Some(entities)) = (duel.state, duel.engaged_entities) else {
        return;
    };
    mgr.duels.end_duel(duel_id);

    let mut cleared = [false; 2];
    for (i, (my_pid, other_pid, my_eid, other_eid)) in [
        (duel.challenger, duel.target, entities[0], entities[1]),
        (duel.target, duel.challenger, entities[1], entities[0]),
    ]
    .into_iter()
    .enumerate()
    {
        // Only the entity that was engaged carries the flag and the combat
        // source. A duelist who logged out has no entity left to clear; one
        // who is elsewhere now still hears the result on their new entity.
        if let Some(p) = connected_player(mgr, my_eid, my_pid) {
            let to = Recipient::at(&p, my_pid, Some(other_pid));
            send_pvp_flag(tx, mgr, to, false, duel_id).await;
            send_duel_entities_clear(tx, to, duel_id).await;
            if let Some(state) = combat::exit(mgr, my_eid, other_eid) {
                send_state_field(tx, mgr, to, state, duel_id).await;
            }
            cleared[i] = true;
        }
        if let Some(p) = find_player(mgr, my_pid) {
            let to = Recipient::at(&p, my_pid, Some(other_pid));
            send_line(tx, to, TEXT_DUEL_ABORTED, Some(duel_id)).await;
        }
    }
    tracing::debug!(
        target: "duel",
        event = "duel.ended",
        duel_id,
        account_id = mgr.get_entity(entities[0]).and_then(|e| e.account_id),
        player_id = duel.challenger,
        entity_id = entities[0],
        target_player_id = duel.target,
        target_entity_id = entities[1],
        space_id = duel.space_id,
        state = "engaged",
        cleared = cleared[0],
        target_cleared = cleared[1],
        reason = reason.reason(),
        "duel ended: PvP flags, duel entities and the combat pair cleared"
    );
}

/// The safety ends, run by the tick: every engaged duel past its limit, or
/// with a duelist no longer at the engaged entity in the duel's space.
pub(super) async fn sweep(tx: &mpsc::Sender<CellToBaseMsg>, mgr: &mut SpaceManager, now: Instant) {
    for duel in mgr.duels.engaged() {
        let (DuelState::Engaged { until }, Some(entities)) = (duel.state, duel.engaged_entities)
        else {
            continue;
        };
        let present = |eid: u32, pid: i32| {
            connected_player(mgr, eid, pid).is_some_and(|p| p.space_id == duel.space_id)
        };
        let reason = if now >= until {
            EndReason::EngagedLimit
        } else if !present(entities[0], duel.challenger) || !present(entities[1], duel.target) {
            EndReason::DuelistGone
        } else {
            continue;
        };
        end_engaged(tx, mgr, duel.duel_id, reason).await;
    }
}
