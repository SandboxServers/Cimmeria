//! Squads (ORG-03): the handlers that turn base-forwarded squad calls and
//! GM commands into `SquadRegistry` changes and client messages, and the
//! pieces every squad handler shares.
//!
//! Entry points here (the lower crates call them):
//!
//! - [`handle_invite`], [`handle_kick`]: `OrgBaseToCell::SquadInvite` and
//!   `SquadKick`, which the base forwards from `organizationInviteByType`
//!   (0xD0, type 0) and `organizationKick` (0xD1, squad-range id). ORG-E1 Q2:
//!   the client's `squadInvite` / `squadKick` natives use these shared wire
//!   methods; there is no squad-specific path.
//! - [`gm_invite`], [`gm_join`]: the squad half of the GM console's
//!   `.squad_invite` and `.squad_join` (ORG-04); the console logs them.
//!
//! The org plugin (`cimmeria-cell-org`, `cell::organization::squad`)
//! re-exports this module and adds the cell methods (8 `respond`, 9
//! `leave`, 10 the minimap ping, 18 the loot mode), the disconnect and the
//! world-entry replay. They build on [`fanout`], [`feedback`],
//! [`telemetry`], [`actor`], [`reject`] and [`confirm`].
//!
//! State is the service-wide squad registry, a `SpaceManager` resource
//! (`cell::squad::SquadResources`, D-ORG03); every
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

pub mod fanout;
pub mod feedback;
mod gm;
mod invite;
mod kick;
pub mod telemetry;

pub use gm::{gm_invite, gm_join, GmOutcome};
pub use invite::handle_invite;
pub use kick::handle_kick;

/// The roster snapshot of the player behind `entity_id`, or `None` when it
/// is not a fully initialised player.
pub fn actor(space_mgr: &SpaceManager, entity_id: u32) -> Option<SquadMember> {
    space_mgr
        .get_entity(entity_id)
        .and_then(fanout::member_snapshot)
}

/// A base-forwarded call names both ids from the base's session. Check the
/// cell entity is still that character before acting: entity ids are
/// recycled, and a stale id must not act for whoever holds it now.
pub fn forwarded_actor(
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
pub async fn reject(
    tx: &mpsc::Sender<CellToBaseMsg>,
    entity_id: u32,
    instance_id: i32,
    text: &str,
) {
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
pub async fn confirm(tx: &mpsc::Sender<CellToBaseMsg>, entity_id: u32, text: &str) {
    if tx
        .send(super::feedback_line(entity_id, text))
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
