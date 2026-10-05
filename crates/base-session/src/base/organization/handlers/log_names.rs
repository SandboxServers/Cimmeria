//! Names for the ids the organization log lines carry (Rule 6).
//!
//! Resolved from the base's own session state, inside the branch that logs.
//! A character that is not online has no session, so it resolves to no name
//! and the field is left off the line; the callers that already hold a name
//! from a database row (a kicked member, an organization header) pass that
//! instead.

use cimmeria_entity::cell_entity::PlayerIdentity;
use cimmeria_entity::name_intern::intern;

use super::OrgCtx;
use crate::base::session_identity::session_identity;

/// The identity (ids and names) of the online session playing `player_id`,
/// or [`PlayerIdentity::UNKNOWN`] when that character is not online.
pub(super) fn identity_of_player(ctx: &OrgCtx<'_>, player_id: i32) -> PlayerIdentity {
    let Ok(clients) = ctx.connected.lock() else {
        return PlayerIdentity::UNKNOWN;
    };
    clients
        .values()
        .find(|c| c.active_player_id == Some(player_id))
        .map_or(PlayerIdentity::UNKNOWN, session_identity)
}

/// The interned form of an organization or character name read from a row,
/// so it fits a `Copy` outcome row. A blank or oversized name is `None`.
pub(super) fn label(name: &str) -> Option<&'static str> {
    intern(name)
}
