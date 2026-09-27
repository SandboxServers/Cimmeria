//! Duels: the challenge, the answer and the countdown (SS-D1); the engage,
//! the PvP flag and the harm gate (SS-D2).
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
//!    sides "Duel aborted" (878); accept starts the 5 s countdown, shown
//!    on both clients as splash numbers (`onTimerUpdate` type 14).
//! 3. [`tick::run_at`] expires unanswered challenges, engages duels whose
//!    countdown has run out ([`engage`]) and runs the safety ends
//!    ([`end::sweep`]).
//!
//! # The harm gate
//!
//! [`DuelRegistry::can_harm`] is true only for the two players of one
//! `Engaged` duel. `combat::player_may_attack` asks it for any player
//! target, and every hostility gate (single target, the warmup re-check,
//! ground AoE, cone) goes through that one function. The PvP flag
//! (`onEntityProperty(4, v)`) is presentation only; nothing reads it back.
//!
//! # What SS-D2 does not do
//!
//! The real end paths (health with the 1 HP clamp, forfeit, range,
//! disconnect, teleport) are SS-D3. They end a duel through
//! [`end::end_engaged`], the same clear the safety ends use.

pub mod challenge;
mod combat;
mod effects;
pub mod end;
mod engage;
pub mod limits;
mod outbound;
pub mod registry;
pub mod response;
pub mod tick;

#[cfg(test)]
mod tests;

pub use end::{end_engaged, EndReason};
pub use outbound::send_player_line;
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

/// The entity of the other duelist, when `player_id` is in an engaged duel
/// and that entity is still the connected player playing the opponent.
///
/// Used where the duel adds one entity to a set of "things a duelist
/// fights": the ground-AoE and cone candidates (`combat::area_candidates`)
/// and the pet owner's combat sources (`npc_ai::pet::defend`). The stored
/// engaged entity id is re-checked with [`connected_player`], so an id the
/// opponent's entity released, and something else was given, is never
/// returned (entity ids are recycled; the sweep ends such a duel next tick).
pub fn engaged_opponent_entity(space_mgr: &SpaceManager, player_id: i32) -> Option<u32> {
    let opponent = space_mgr.duels.engaged_opponent(player_id)?;
    let duel = space_mgr.duels.duel_of(player_id)?;
    let entities = duel.engaged_entities?;
    let eid = if opponent == duel.challenger {
        entities[0]
    } else {
        entities[1]
    };
    connected_player(space_mgr, eid, opponent)
        .filter(|p| p.space_id == duel.space_id)
        .map(|p| p.entity_id)
}

/// The PvP-flag replay for a witness meeting `observee` on the AoI enter
/// path: `onEntityProperty(PvPFlag, 1)` on the observee, to `witness_id`,
/// when the observee is a player in an engaged duel, at the engaged entity.
/// `None` otherwise: an unflagged player needs nothing, since 0 is the
/// client's default.
pub fn pvp_flag_on_enter(
    duels: &DuelRegistry,
    witness_id: u32,
    observee: &cimmeria_entity::cell_entity::CellEntity,
) -> Option<crate::cell::messages::CellToBaseMsg> {
    let pid = observee.player_id.filter(|_| observee.is_player)?;
    let entities = duels
        .duel_of(pid)
        .filter(|d| matches!(d.state, DuelState::Engaged { .. }))?
        .engaged_entities?;
    let observee_eid = observee.entity_id.0 as u32;
    if !entities.contains(&observee_eid) {
        return None;
    }
    Some(crate::cell::messages::CellToBaseMsg::WitnessEntityMethod {
        witness_id,
        entity_id: observee_eid,
        method_index: cimmeria_wire::cell::client_methods::duel::ON_ENTITY_PROPERTY,
        args: cimmeria_wire::cell::client_methods::duel::build_pvp_flag(true),
        entity_is_player: true,
    })
}

/// The connected player entity playing `player_id` right now, if any. A
/// linear scan over connected players: called once per duel event. The
/// safety sweep, which runs every tick, uses [`connected_player`] instead.
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
