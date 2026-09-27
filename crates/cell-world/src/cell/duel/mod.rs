//! Duels: the challenge, the answer and the countdown (SS-D1).
//!
//! The base receives `sendDuelChallenge` (0xD9), runs the rate limit, the
//! online lookup and the Ignore check, and forwards the challenge as
//! `BaseToCellMsg::Duel(DuelBaseToCell::Challenge)`. From there the cell
//! owns everything:
//!
//! 1. [`challenge::handle_at`] checks self (text 872), the space and the
//!    range (877, D-SS19), whether either side is busy (873, D-SS21) and the
//!    per-pair cooldown, stores the challenge and sends the target
//!    `onDuelChallenge` [143].
//! 2. [`response::handle_at`] runs `sendDuelResponse` (CM 102): only the
//!    challenge addressed to the caller counts, it is consumed on first use
//!    and it expires after 30 s (D-SS18). Decline or expiry tells both
//!    sides "Duel aborted" (878); accept starts the 5 s countdown.
//! 3. [`tick::run_at`] expires unanswered challenges and ends countdowns.
//!
//! # What SS-D1 does not do
//!
//! No PvP flag, no harm gate and no `onDuelEntities*` send: those are SS-D2
//! (D-SS25). [`DuelRegistry::can_harm`] is the predicate SS-D2's gates call;
//! it is true only for an `Engaged` duel, and nothing here engages one.
//! Until SS-D2 lands, a countdown that runs out aborts the duel with 878 so
//! neither player is left marked busy (see [`tick`]).

pub mod challenge;
pub mod limits;
mod outbound;
pub mod registry;
pub mod response;
pub mod tick;

#[cfg(test)]
mod tests;

pub use registry::{
    ChallengeRefusal, Duel, DuelId, DuelRegistry, DuelState, PendingChallenge, ResponseRefusal,
};

use cimmeria_common::Vector3;

use super::space_manager::SpaceManager;

/// A player's entity as the duel checks need it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct PlayerAt {
    pub entity_id: u32,
    pub space_id: u32,
    pub position: Vector3,
    pub account_id: Option<u32>,
}

/// `entity_id` if it is a connected player entity playing `player_id`.
///
/// The ids come from the base's session map, but the entity id may have
/// been recycled or the player may be mid-teardown by the time the message
/// is handled, so both must still agree.
pub(crate) fn connected_player(
    space_mgr: &SpaceManager,
    entity_id: u32,
    player_id: i32,
) -> Option<PlayerAt> {
    let &space_id = space_mgr.entity_space.get(&entity_id)?;
    let space = space_mgr.spaces.get(&space_id)?;
    if !space.players.contains(&entity_id) {
        return None;
    }
    let entity = space.entities.get(&entity_id)?;
    (entity.player_id == Some(player_id)).then_some(PlayerAt {
        entity_id,
        space_id,
        position: entity.position,
        account_id: entity.account_id,
    })
}

/// The connected player entity playing `player_id` right now, if any. A
/// linear scan over connected players: called once per duel event, never
/// per tick.
pub(crate) fn find_player(space_mgr: &SpaceManager, player_id: i32) -> Option<PlayerAt> {
    space_mgr.spaces.values().find_map(|space| {
        space.players.iter().find_map(|&eid| {
            let e = space.entities.get(&eid)?;
            (e.player_id == Some(player_id)).then_some(PlayerAt {
                entity_id: eid,
                space_id: space.space_id,
                position: e.position,
                account_id: e.account_id,
            })
        })
    })
}
