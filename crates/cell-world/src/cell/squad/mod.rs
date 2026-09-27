//! The service-wide squad registry (ORG-03, D-ORG03).
//!
//! Squads are ephemeral parties of up to six that live only on the cell and
//! are never persisted. One [`SquadRegistry`] serves every space: it is a
//! field of [`SpaceManager`](super::space_manager::SpaceManager), the one
//! container every cell handler already receives, so a member who gates to
//! another world keeps their squad (PR #584's per-space manager lost it on
//! the first gate trip, audit A-30).
//!
//! This module is pure state: no I/O, no clock. Every time-dependent call
//! takes `now`, so the 60 s invite expiry and the 30 s rate window are
//! tested with exact instants. The handlers that resolve names, send the
//! client methods and write feedback live in `cimmeria-cell-methods`
//! (`cell_methods::organization::squad`).
//!
//! Everything is keyed by the character's `player_id`, never by entity id:
//! entity ids are recycled, and gate travel destroys and re-creates the
//! entity (D-ORG06).
//!
//! - [`registry`]: squads, membership, leave / kick / disconnect, loot mode.
//! - [`invites`]: pending invites (D-ORG06) and the invite rate limits.

use std::time::Duration;

mod invites;
mod registry;

pub use invites::{
    ExpiredInvite, InviteReject, IssuedInvite, PendingInvite, ResponseReject, TakeMiss,
};
pub use registry::{Departure, JoinOutcome, KickReject, LootReject, Squad, SquadRegistry};

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
