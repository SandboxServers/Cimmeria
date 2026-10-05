//! The names that pair with the ids on the duel lifecycle events (Rule 6).
//!
//! Every function here is a lookup meant to sit in a tracing field
//! expression, which `tracing` evaluates only when the event is enabled, so
//! the tick and the sweeps pay nothing outside the log branch. A duelist who
//! has left the world resolves to `None`; the field is then dropped, never
//! filled with a placeholder.

use crate::cell::space_manager::SpaceManager;

use super::find_player;

/// The character name of the player on `entity_id`, when it is a named player.
pub(super) fn player_name(mgr: &SpaceManager, entity_id: Option<u32>) -> Option<&'static str> {
    entity_id.and_then(|e| mgr.player_identity(e).player_name)
}

/// The login name of the player on `entity_id`.
pub(super) fn account_name(mgr: &SpaceManager, entity_id: Option<u32>) -> Option<&'static str> {
    entity_id.and_then(|e| mgr.player_identity(e).account_name)
}

/// The label of any live entity (character name, or NPC display text).
pub(super) fn entity_name(mgr: &SpaceManager, entity_id: Option<u32>) -> Option<&str> {
    entity_id.and_then(|e| mgr.entity_label(e))
}

/// The character name of `player_id`, when that player is connected.
pub(super) fn player_name_of(mgr: &SpaceManager, player_id: i32) -> Option<&'static str> {
    player_name(mgr, find_player(mgr, player_id).map(|p| p.entity_id))
}

/// The login name of `player_id`, when that player is connected.
pub(super) fn account_name_of(mgr: &SpaceManager, player_id: i32) -> Option<&'static str> {
    account_name(mgr, find_player(mgr, player_id).map(|p| p.entity_id))
}

/// The world a duel space runs, when the space is still loaded.
pub(super) fn world(mgr: &SpaceManager, space_id: u32) -> Option<&str> {
    mgr.world_name_for_space(space_id)
}
