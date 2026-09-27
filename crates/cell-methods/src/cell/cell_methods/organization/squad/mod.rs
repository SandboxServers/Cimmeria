//! Squads (ORG-03): the handlers that turn client calls and base-forwarded
//! squad calls into `SquadRegistry` changes and client messages.
//!
//! Entry points:
//!
//! - [`handle_invite`], [`handle_kick`]: `OrgBaseToCell::SquadInvite` and
//!   `SquadKick`, which the base forwards from `organizationInviteByType`
//!   (0xD0, type 0) and `organizationKick` (0xD1, squad-range id). ORG-E1 Q2:
//!   the client's `squadInvite` / `squadKick` natives use these shared wire
//!   methods; there is no squad-specific path.
//! - [`respond`], [`leave`], [`set_loot_mode`]: cell methods 8, 9 and 18,
//!   from the organization router.
//! - [`broadcast_minimap_ping`]: cell method 10 for a squad id (ORG-04),
//!   validated and logged, never fanned out.
//! - [`on_disconnect`]: the `DisconnectEntity` arm.
//! - [`gm_invite`], [`gm_join`]: the squad half of the GM console's
//!   `.squad_invite` and `.squad_join` (ORG-04); the console logs them.
//! - [`on_world_entry`]: `InitPlayerState`, which re-sends the squad after a
//!   gate trip re-created the player.
//!
//! State is the service-wide `SpaceManager::squads` (D-ORG03); every
//! refusal answers with `onErrorCode` and a feedback line, so no press is
//! silent. Logs use the `squad` target: one INFO span and one INFO outcome
//! row per action, DEBUG transitions, WARN on negative seams
//! (`telemetry`).

use tokio::sync::mpsc;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use crate::cell::squad::SquadMember;

use cimmeria_wire::cell::client_methods::communicator::ON_PLAYER_COMMUNICATION;
use cimmeria_wire::cell::client_methods::player::{
    build_on_error_code, CONDITION_FEEDBACK_INVALID_ENTITY, ERRORCODE_SYSTEM_ABILITY, ON_ERROR_CODE,
};

mod fanout;
mod feedback;
mod gm;
mod invite;
mod loot;
mod membership;
mod ping;
mod telemetry;
mod world_entry;

pub use gm::{gm_invite, gm_join, GmOutcome};
pub use invite::{handle_invite, respond};
pub use loot::set_loot_mode;
pub use membership::{handle_kick, leave, on_disconnect};
pub use ping::broadcast_minimap_ping;
#[cfg(test)]
pub(super) use ping::ping_at;
pub use world_entry::on_world_entry;

/// The roster snapshot of the player behind `entity_id`, or `None` when it
/// is not a fully initialised player.
fn actor(space_mgr: &SpaceManager, entity_id: u32) -> Option<SquadMember> {
    space_mgr
        .get_entity(entity_id)
        .and_then(fanout::member_snapshot)
}

/// [`actor`] for tests that seed the registry directly.
#[cfg(test)]
pub(super) fn test_snapshot(space_mgr: &SpaceManager, entity_id: u32) -> SquadMember {
    actor(space_mgr, entity_id).expect("an initialised player")
}

/// A base-forwarded call names both ids from the base's session. Check the
/// cell entity is still that character before acting: entity ids are
/// recycled, and a stale id must not act for whoever holds it now.
fn forwarded_actor(
    space_mgr: &SpaceManager,
    player_id: i32,
    entity_id: u32,
) -> Option<SquadMember> {
    let member = actor(space_mgr, entity_id).filter(|m| m.player_id == player_id);
    if member.is_none() {
        tracing::warn!(
            target: "squad",
            event = "squad.actor_mismatch",
            player_id,
            entity_id,
            "forwarded squad call names an entity that is not that player's -- dropped"
        );
    }
    member
}

/// Refuse with `onErrorCode(0, instance_id, 0)` and `text` on the
/// feedback channel. A dropped send logs WARN `squad.send_failed`.
async fn reject(tx: &mpsc::Sender<CellToBaseMsg>, entity_id: u32, instance_id: i32, text: &str) {
    fanout::send(
        tx,
        entity_id,
        ON_ERROR_CODE,
        build_on_error_code(
            ERRORCODE_SYSTEM_ABILITY,
            instance_id,
            CONDITION_FEEDBACK_INVALID_ENTITY,
        ),
    )
    .await;
    confirm(tx, entity_id, text).await;
}

/// A confirmation line on the feedback channel, with no error code.
async fn confirm(tx: &mpsc::Sender<CellToBaseMsg>, entity_id: u32, text: &str) {
    if tx
        .send(super::forward::feedback_line(entity_id, text))
        .await
        .is_err()
    {
        tracing::warn!(
            target: "squad",
            event = "squad.send_failed",
            entity_id,
            method_index = ON_PLAYER_COMMUNICATION,
            reason = "cell_to_base_closed",
            "squad confirmation line could not be queued"
        );
    }
}
