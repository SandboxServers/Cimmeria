//! Duels: the challenge, the answer and the countdown (SS-D1); the engage,
//! the PvP flag and the harm gate (SS-D2); the end paths (SS-D3).
//!
//! # What lives here and what lives in the duel plugin
//!
//! Duels are a cell plugin (#962, `docs/architecture/plugin-architecture.md`
//! §3.8 step 2): `cimmeria-cell-duel`'s `DuelPlugin` owns the two duel cell
//! methods, the duel tick and the lifecycle hooks. This module keeps what
//! the lower cell crates still call directly, and the registry they read:
//!
//! - [`DuelRegistry`], a `SpaceManager` resource ([`DuelResources`]);
//! - [`challenge`], the base's forward (`BaseToCellMsg::Duel`), which
//!   `cimmeria-cell`'s base-message handler calls: the plugin model has no
//!   base-message seam yet (ADR §3.4, the envelope);
//! - the harm gate's inputs (`combat::player_may_attack` in this crate reads
//!   [`DuelRegistry::can_harm`]), [`engaged_opponent_entity`] and
//!   [`pvp_flag_on_enter`] (the AoI enter path);
//! - [`paths`]: the non-lethal clamp the damage resolvers in
//!   `cimmeria-cell-combat` call, and the leave paths the plugin's
//!   disconnect, travel and death hooks call;
//! - [`end`] (every end goes through [`end::end_engaged`]), [`gm`] (the GM
//!   `.duel_status` / `.duel_end` console commands in `cimmeria-cell-console`)
//!   and the shared pieces the plugin builds on ([`outbound`], [`combat`],
//!   [`limits`], [`connected_player`], [`find_player`]).
//!
//! # The flow
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
//! 2. The plugin's `response::handle_at` runs `sendDuelResponse` (CM 102):
//!    only the challenge addressed to the caller counts, it is consumed on
//!    first use and it expires after 30 s (D-SS18). Decline or expiry tells
//!    both sides "Duel aborted" (878); accept starts the 5 s countdown,
//!    shown on both clients as splash numbers (`onTimerUpdate` type 14).
//! 3. The plugin's `tick::run_at` expires unanswered challenges, engages
//!    duels whose countdown has run out and runs the safety ends
//!    ([`end::sweep`]).
//!
//! [`gm`] serves the GM `.duel_status` and `.duel_end` console commands
//! (SS-U2): a read of one player's entry, and a GM abort. An engaged duel
//! ends through [`end::end_engaged`] with `EndReason::GmAborted`.
//!
//! # The harm gate
//!
//! [`DuelRegistry::can_harm`] is true only for the two players of one
//! `Engaged` duel. `combat::player_may_attack` asks it for any player
//! target, and every hostility gate (single target, the warmup re-check,
//! ground AoE, cone) goes through that one function. The PvP flag
//! (`onEntityProperty(4, v)`) is presentation only; nothing reads it back.
//!
//! # The end
//!
//! Every end of an engaged duel goes through [`end::end_engaged`]: the
//! clear (flags, 153, partner effects, the combat pair), then 879 to the
//! winner and a line to the loser, or 878 to both for an abort. The paths:
//! forfeit (CM 103, the plugin's `forfeit`); the non-lethal clamp on partner
//! damage and death from anyone else, disconnect, and every teleport or gate
//! travel ([`paths`], the last three through the plugin's hooks); range, a
//! dead or departed duelist and the safety limit on the tick
//! ([`end::sweep`]); and the GM `.duel_end`. A challenge or a countdown is
//! withdrawn on the same leave paths.

pub mod challenge;
pub mod combat;
mod effects;
pub mod end;
pub mod gm;
pub mod limits;
pub mod outbound;
pub mod paths;
pub mod registry;

#[cfg(test)]
mod tests;

pub use end::{end_engaged, DefeatReason, EndReason};
pub use outbound::send_player_line;
pub use paths::{
    clamp_partner_lethal, finish_clamped, on_death, on_disconnect, on_travel, ClampSource,
    ClampedHit,
};
pub use registry::{
    ChallengeRefusal, Duel, DuelId, DuelRegistry, DuelState, GmAborted, PendingChallenge,
    ResponseRefusal, Withdrawn,
};

use cimmeria_common::Vector3;

use super::space_manager::{SpaceManager, SpaceResources};

/// The duel registry as a `SpaceManager` resource (#962,
/// `docs/architecture/plugin-architecture.md` §3.5): it lives in
/// `SpaceManager::resources`, keyed by its type, not in a field of its own.
/// Call these on the `resources` field (`mgr.resources.duels()`), not on the
/// manager, so the borrow stays on that one field.
pub trait DuelResources {
    /// The registry. Empty (and nothing stored) until the first challenge.
    fn duels(&self) -> &DuelRegistry;
    /// The registry, mutably; stored empty on first use.
    fn duels_mut(&mut self) -> &mut DuelRegistry;
}

impl DuelResources for SpaceResources {
    fn duels(&self) -> &DuelRegistry {
        static EMPTY: std::sync::LazyLock<DuelRegistry> =
            std::sync::LazyLock::new(DuelRegistry::default);
        self.get::<DuelRegistry>().unwrap_or(&EMPTY)
    }

    fn duels_mut(&mut self) -> &mut DuelRegistry {
        if !self.contains::<DuelRegistry>() {
            self.insert(DuelRegistry::default());
        }
        self.get_mut::<DuelRegistry>()
            .expect("the duel registry was stored above")
    }
}

/// A player's entity as the duel checks need it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlayerAt {
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
pub fn connected_player(
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
    let opponent = space_mgr.resources.duels().engaged_opponent(player_id)?;
    let duel = space_mgr.resources.duels().duel_of(player_id)?;
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
pub fn find_player(space_mgr: &SpaceManager, player_id: i32) -> Option<PlayerAt> {
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
