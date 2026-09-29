//! Squads (ORG-03, ORG-04): the org plugin's squad handlers.
//!
//! Entry points:
//!
//! - [`respond`], [`leave`], [`set_loot_mode`]: cell methods 8, 9 and 18,
//!   from the organization router.
//! - [`broadcast_minimap_ping`]: cell method 10 for a squad id (ORG-04),
//!   validated and logged, never fanned out.
//! - [`on_disconnect`]: the plugin's `AfterDisconnectTradeCancel` hook.
//! - [`on_world_entry`]: the plugin's `AfterInitPlayerState` hook, which
//!   re-sends the squad after a gate trip re-created the player.
//!
//! The base-forwarded invite and kick ([`handle_invite`], [`handle_kick`]),
//! the GM commands ([`gm_invite`], [`gm_join`]) and the pieces every squad
//! handler shares ([`fanout`], [`feedback`], [`telemetry`], [`actor`],
//! [`reject`], [`confirm`]) stay in `cimmeria-cell-interactions`
//! (`cell::organization::squad`), because the base-message handler and the
//! console call them. This module re-exports that one whole, so the moved
//! handlers name it by the same `super::…` paths as before the move.
//!
//! State is the service-wide squad registry, a `SpaceManager` resource
//! (`cell::squad::SquadResources`, D-ORG03). Logs use the `squad` target.

pub use cimmeria_cell_interactions::cell::organization::squad::*;

mod invite_response;
mod loot;
mod membership;
mod ping;
mod world_entry;

pub use invite_response::respond;
pub use loot::set_loot_mode;
pub use membership::{leave, on_disconnect};
pub use ping::broadcast_minimap_ping;
#[cfg(test)]
pub(super) use ping::ping_at;
pub use world_entry::on_world_entry;

/// [`actor`] for tests that seed the registry directly.
#[cfg(test)]
pub(super) fn test_snapshot(
    space_mgr: &crate::cell::space_manager::SpaceManager,
    entity_id: u32,
) -> crate::cell::squad::SquadMember {
    actor(space_mgr, entity_id).expect("an initialised player")
}
