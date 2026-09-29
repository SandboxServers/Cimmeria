//! The service-wide squad registry (ORG-03, D-ORG03).
//!
//! Squads are ephemeral parties of up to six that live only on the cell and
//! are never persisted. One [`SquadRegistry`] serves every space: it is a
//! resource of [`SpaceManager`](super::space_manager::SpaceManager)
//! ([`SquadResources`]), the one container every cell handler already
//! receives, so a member who gates to another world keeps their squad. A
//! registry held per space would lose it on the first gate trip.
//!
//! This module is pure state: no I/O, no clock. Every time-dependent call
//! takes `now`, so the 60 s invite expiry and the 30 s rate window are
//! tested with exact instants. The handlers that resolve names, send the
//! client methods and write feedback live in `cimmeria-cell-interactions`
//! (`cell::organization::squad`: the base-forwarded invite and kick, the GM
//! commands and the shared fanout) and the org plugin, `cimmeria-cell-org`
//! (the cell methods, the disconnect and the world-entry replay).
//!
//! Everything is keyed by the character's `player_id`, never by entity id:
//! entity ids are recycled, and gate travel destroys and re-creates the
//! entity (D-ORG06).
//!
//! - [`registry`]: squads, membership, leave / kick / disconnect, loot mode.
//! - [`invites`]: pending invites (D-ORG06) and the invite rate limits.
//! - [`ping`]: the minimap ping check and its one-per-second limit (ORG-04).

use std::time::Duration;

use cimmeria_entity::cell_entity::CellEntity;

use super::space_manager::SpaceResources;

mod invites;
mod ping;
mod registry;

pub use invites::{
    ExpiredInvite, InviteReject, IssuedInvite, PendingInvite, ResponseReject, TakeMiss,
};
pub use ping::{PingReject, PING_MIN_INTERVAL};
pub use registry::{
    Departure, ForceJoinReject, JoinOutcome, KickReject, LootReject, Squad, SquadRegistry,
};

#[cfg(test)]
mod ping_and_join_tests;
#[cfg(test)]
mod tests;

/// How long an invite stays answerable (D-ORG06).
pub const INVITE_TTL: Duration = Duration::from_secs(60);

/// The window of the per-inviter rate limit.
pub const INVITE_RATE_WINDOW: Duration = Duration::from_secs(30);

/// Invites one player may issue inside [`INVITE_RATE_WINDOW`].
pub const INVITE_RATE_MAX: usize = 5;

/// Pending invites one player may hold at once.
pub const MAX_PENDING_PER_INVITEE: usize = 5;

/// Count one squad action on `squad_actions_total`. Every label is from a
/// closed set (the action, `ok` / `rejected`, and the refusal reason or
/// `none`); never an id.
pub fn count_action(action: &'static str, outcome: &'static str, reason: &'static str) {
    cimmeria_observability::counter!(
        "squad_actions_total",
        "action" => action,
        "outcome" => outcome,
        "reason" => reason,
    );
}

/// A member's roster snapshot, taken from their `CellEntity` when they
/// join. The roster (`RosterInfo`) shows name, level and archetype; the
/// snapshot is not refreshed on level-up, which the client roster would
/// only show after a rejoin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SquadMember {
    pub player_id: i32,
    pub name: String,
    pub level: u8,
    pub archetype: u8,
}

/// The squad registry as a `SpaceManager` resource (#962,
/// `docs/architecture/plugin-architecture.md` §3.5): it lives in
/// `SpaceManager::resources`, keyed by its type, not in a field of its own.
/// Call these on the `resources` field (`mgr.resources.squads()`), not on
/// the manager, so the borrow stays on that one field.
pub trait SquadResources {
    /// The registry. Empty (and nothing stored) until the first invite.
    fn squads(&self) -> &SquadRegistry;
    /// The registry, mutably; stored empty on first use.
    fn squads_mut(&mut self) -> &mut SquadRegistry;
}

impl SquadResources for SpaceResources {
    fn squads(&self) -> &SquadRegistry {
        static EMPTY: std::sync::LazyLock<SquadRegistry> =
            std::sync::LazyLock::new(SquadRegistry::new);
        self.get::<SquadRegistry>().unwrap_or(&EMPTY)
    }

    fn squads_mut(&mut self) -> &mut SquadRegistry {
        if !self.contains::<SquadRegistry>() {
            self.insert(SquadRegistry::new());
        }
        self.get_mut::<SquadRegistry>()
            .expect("the squad registry was stored above")
    }
}

/// The squad a player's cell entity is in (ORG-03), kept in
/// `CellEntity::extensions` (#962, ADR §3.5). Stands in for the `squad`
/// `CELL_PUBLIC` property, which is never sent to clients (audit A-19). A
/// mirror of the [`SquadRegistry`], which is authoritative: set and cleared
/// with membership, and re-stamped on every world entry because gate travel
/// re-creates the entity. Read and written through [`entity_squad_id`] and
/// [`set_entity_squad_id`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntitySquad(pub i32);

/// The squad `entity` is in, if any.
pub fn entity_squad_id(entity: &CellEntity) -> Option<i32> {
    entity.extensions.get::<EntitySquad>().map(|s| s.0)
}

/// Stamp (`Some`) or clear (`None`) the squad `entity` is in.
pub fn set_entity_squad_id(entity: &mut CellEntity, squad_id: Option<i32>) {
    match squad_id {
        Some(id) => {
            entity.extensions.insert(EntitySquad(id));
        }
        None => {
            entity.extensions.remove::<EntitySquad>();
        }
    }
}
