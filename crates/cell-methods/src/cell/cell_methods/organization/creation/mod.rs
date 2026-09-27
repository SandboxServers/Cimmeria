//! Founding a Team or Command, cell side (ORG-05).
//!
//! - [`on_registrar_eligible`]: `OrgBaseToCell::RegistrarEligible`. The base
//!   found the player eligible after a registrar click
//!   (`interactions::org_registrar`); record the pending creation and open
//!   the naming dialog, `launchOrganizationCreation` [135].
//! - [`on_organization_creation`]: cell method 94. Honoured only against
//!   the player's pending creation, whose type it uses (the wire carries
//!   only the name); the name must pass D-ORG10; then
//!   `OrgCellToBase::Create` goes to the base, which creates, answers the
//!   client and replies with [`on_create_result`].
//! - [`on_create_result`]: `OrgBaseToCell::CreateResult`, which closes the
//!   pending creation or charges it an attempt.
//! - [`on_disconnect`]: the `DisconnectEntity` arm drops the offer.
//!
//! Every refusal decided here answers the client with
//! `onOrganizationCreationResult` [134] `(0, RetCode)` and a feedback line
//! (the client shows no text for either byte, ORG-E1 Q4), and writes one
//! outcome row ([`telemetry`]). State: `SpaceManager::org_creations`.

mod telemetry;

use std::time::Instant;

use tokio::sync::mpsc;

use cimmeria_entity::cell_entity::PlayerIdentity;
use cimmeria_entity::organization::{org_text, OrgType, TextField};
use cimmeria_wire::cell::cell_methods::organization::decode_on_organization_creation;
use cimmeria_wire::cell::client_methods::player::{
    build_launch_organization_creation, build_on_organization_creation_result,
    org_creation_ret_code as rc, LAUNCH_ORGANIZATION_CREATION, ON_ORGANIZATION_CREATION_RESULT,
    ORG_CREATION_RESULT_REFUSED,
};

use super::forward::feedback_line;
use crate::cell::messages::{CellToBaseMsg, OrgCellToBase};
use crate::cell::org_creation::{OpenReject, Opened, TakeMiss};
use crate::cell::space_manager::SpaceManager;
use telemetry::{Action, Outcome};

/// No registrar offer is open.
pub const NO_PENDING_TEXT: &str = "Talk to an Organization Registrar to found a Team or Command.";
/// The offer expired, or the player left the registrar's space.
pub const PENDING_EXPIRED_TEXT: &str =
    "The registrar's offer has expired. Talk to the registrar again.";
/// Every attempt of the window is spent.
pub const RATE_LIMITED_TEXT: &str =
    "Too many attempts. Talk to the registrar again in a few minutes.";
/// A name is already at the base.
pub const IN_FLIGHT_TEXT: &str = "Your organization is already being created. Please wait.";
/// The name fails D-ORG10 (the base sends the same text for its own check).
pub const NAME_INVALID_TEXT: &str =
    "That name is not allowed. Use 1-60 letters, digits, spaces, apostrophes, hyphens or periods.";
/// The request cannot be served (no player state, or the base is gone).
pub const UNAVAILABLE_TEXT: &str = "The registrar cannot help you right now. Try again later.";

/// Queue one client call; WARN if the base channel is closed.
async fn send(
    tx: &mpsc::Sender<CellToBaseMsg>,
    entity_id: u32,
    method_index: u16,
    args: Vec<u8>,
) -> bool {
    let sent = tx
        .send(CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index,
            args,
        })
        .await
        .is_ok();
    if !sent {
        tracing::warn!(
            target: "org",
            event = "org.feedback_send_failed",
            reason = "cell_to_base_closed",
            entity_id,
            method_index,
            "organization creation reply could not be queued"
        );
    }
    sent
}

/// The refusal pair: 134 `(0, code)` then the line.
async fn refuse(tx: &mpsc::Sender<CellToBaseMsg>, entity_id: u32, code: u8, text: &str) {
    send(
        tx,
        entity_id,
        ON_ORGANIZATION_CREATION_RESULT,
        build_on_organization_creation_result(ORG_CREATION_RESULT_REFUSED, code),
    )
    .await;
    say(tx, entity_id, text).await;
}

/// `text` on the feedback channel.
async fn say(tx: &mpsc::Sender<CellToBaseMsg>, entity_id: u32, text: &str) {
    if tx.send(feedback_line(entity_id, text)).await.is_err() {
        tracing::warn!(
            target: "org",
            event = "org.feedback_send_failed",
            reason = "cell_to_base_closed",
            entity_id,
            "organization creation line could not be queued"
        );
    }
}

/// The 134 code and line for a pending-creation miss.
fn miss_reply(miss: TakeMiss) -> (u8, &'static str) {
    match miss {
        TakeMiss::NoPending => (rc::NO_PENDING_CREATION, NO_PENDING_TEXT),
        TakeMiss::Expired | TakeMiss::SpaceChanged => {
            (rc::NO_PENDING_CREATION, PENDING_EXPIRED_TEXT)
        }
        TakeMiss::Exhausted => (rc::RATE_LIMITED, RATE_LIMITED_TEXT),
        TakeMiss::InFlight => (rc::RATE_LIMITED, IN_FLIGHT_TEXT),
    }
}

/// Cell method 94 `onOrganizationCreation(WSTRING name)`.
#[tracing::instrument(name = "org.create", level = "info", skip_all, fields(entity_id))]
pub async fn on_organization_creation(
    entity_id: u32,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let name = match decode_on_organization_creation(args) {
        Ok(name) => name,
        Err(e) => {
            // A real client always sends the `.def` shape: a forged or
            // corrupted call, logged and not answered.
            tracing::warn!(
                target: "org",
                event = "org.cell_method_malformed",
                entity_id,
                method_index = 94u16,
                reason = e.reason(),
                error = %e,
                "organization cell method payload did not decode"
            );
            return;
        }
    };
    let actor = space_mgr.player_identity(entity_id);
    let mut row = Outcome::new(Action::Create, actor, entity_id);
    row.name_units = Some(name.encode_utf16().count());

    let (Some(player_id), Some(space_id)) =
        (actor.player_id, space_mgr.get_entity_space_id(entity_id))
    else {
        row.rejected("not_ready");
        refuse(tx, entity_id, rc::SERVER_ERROR, UNAVAILABLE_TEXT).await;
        return;
    };

    let org_type = match space_mgr
        .org_creations
        .begin_attempt(player_id, space_id, Instant::now())
    {
        Ok(t) => t,
        Err((miss, pending)) => {
            if let Some(p) = pending.as_ref() {
                row.org_type = Some(p.org_type);
                row.attempts_left = Some(p.attempts_left);
                match miss {
                    TakeMiss::Expired => telemetry::pending_expired(actor, p, "ttl"),
                    TakeMiss::SpaceChanged => telemetry::pending_expired(actor, p, "space_changed"),
                    _ => {}
                }
            }
            row.rejected(miss.reason());
            let (code, text) = miss_reply(miss);
            refuse(tx, entity_id, code, text).await;
            return;
        }
    };
    row.org_type = Some(org_type);

    let name = match org_text::validate(TextField::Name, &name) {
        Ok(n) => n,
        Err(reject) => {
            let left = space_mgr.org_creations.charge_attempt(player_id);
            telemetry::attempt_charged(actor, left, "cell_text_check");
            row.attempts_left = left;
            row.text_reason = Some(reject.reason());
            row.rejected("text_invalid");
            refuse(tx, entity_id, rc::NAME_INVALID, NAME_INVALID_TEXT).await;
            return;
        }
    };

    let forwarded = tx
        .send(CellToBaseMsg::Org(OrgCellToBase::Create {
            player_id,
            entity_id,
            org_type,
            name,
        }))
        .await
        .is_ok();
    if !forwarded {
        tracing::warn!(
            target: "org",
            event = "org.create_forward_failed",
            reason = "cell_to_base_closed",
            account_id = actor.account_id,
            player_id,
            entity_id,
            "organization creation could not reach the base"
        );
        let left = space_mgr.org_creations.charge_attempt(player_id);
        row.attempts_left = left;
        row.rejected("base_unreachable");
        refuse(tx, entity_id, rc::SERVER_ERROR, UNAVAILABLE_TEXT).await;
        return;
    }
    // The base writes this action's outcome row once it has decided.
    tracing::debug!(
        target: "org",
        event = "org.create_forwarded",
        account_id = actor.account_id,
        player_id,
        entity_id,
        org_type = org_type.name(),
        name_units = row.name_units,
        "organization name forwarded to the base"
    );
}

/// The entity's identity, if it is still character `player_id` (a message
/// from the base names both; entity ids are recycled). WARNs otherwise.
fn forwarded_actor(
    space_mgr: &SpaceManager,
    player_id: i32,
    entity_id: u32,
    kind: &'static str,
) -> Option<PlayerIdentity> {
    let identity = space_mgr.player_identity(entity_id);
    if identity.player_id == Some(player_id) {
        return Some(identity);
    }
    tracing::warn!(
        target: "org",
        event = "org.actor_mismatch",
        reason = "actor_mismatch",
        player_id,
        entity_id,
        entity_player_id = identity.player_id,
        kind,
        "organization message from the base names an entity that is no longer that character"
    );
    None
}

/// `OrgBaseToCell::RegistrarEligible`: record the offer and open the
/// naming dialog.
#[tracing::instrument(
    name = "org.registrar_open",
    level = "info",
    skip_all,
    fields(player_id, entity_id, npc_entity_id, org_type = org_type.name())
)]
pub async fn on_registrar_eligible(
    player_id: i32,
    entity_id: u32,
    npc_entity_id: u32,
    org_type: OrgType,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let identity = forwarded_actor(space_mgr, player_id, entity_id, "registrar_eligible");
    let mut row = Outcome::new(
        Action::RegistrarOpen,
        identity.unwrap_or(PlayerIdentity {
            account_id: None,
            player_id: Some(player_id),
        }),
        entity_id,
    );
    row.org_type = Some(org_type);
    row.npc_entity_id = Some(npc_entity_id);
    let Some(actor) = identity else {
        // Whoever holds the entity now did not click the registrar.
        row.rejected("actor_mismatch");
        return;
    };
    let Some(space_id) = space_mgr.get_entity_space_id(entity_id) else {
        row.rejected("not_ready");
        return;
    };
    let opened =
        space_mgr
            .org_creations
            .open(player_id, org_type, npc_entity_id, space_id, Instant::now());
    let refreshed = match opened {
        Ok(Opened::Created) => false,
        Ok(Opened::Refreshed) => true,
        Err(OpenReject::Exhausted) => {
            row.attempts_left = Some(0);
            row.rejected("rate_limited");
            say(tx, entity_id, RATE_LIMITED_TEXT).await;
            return;
        }
    };
    let pending = *space_mgr
        .org_creations
        .get(player_id)
        .expect("open just recorded the offer");
    telemetry::pending_created(actor, &pending, refreshed);
    row.attempts_left = Some(pending.attempts_left);
    if !send(
        tx,
        entity_id,
        LAUNCH_ORGANIZATION_CREATION,
        build_launch_organization_creation(org_type),
    )
    .await
    {
        // The dialog never opened; do not leave an offer behind it.
        if let Some(p) = space_mgr.org_creations.clear(player_id) {
            telemetry::pending_expired(actor, &p, "send_failed");
        }
        row.rejected("base_unreachable");
        return;
    }
    row.ok();
}

/// `OrgBaseToCell::CreateResult`: close the offer on a creation, or charge
/// it the refused attempt.
pub fn on_create_result(
    player_id: i32,
    entity_id: u32,
    created: bool,
    space_mgr: &mut SpaceManager,
) {
    // The offer is keyed by character, so it is settled even if the entity
    // has gone meanwhile; the identity is only for the log.
    let actor = PlayerIdentity {
        player_id: Some(player_id),
        ..space_mgr.player_identity(entity_id)
    };
    if created {
        match space_mgr.org_creations.consume(player_id) {
            Some(p) => telemetry::pending_consumed(actor, &p),
            None => tracing::debug!(
                target: "org",
                event = "pending_creation_missing",
                account_id = actor.account_id,
                player_id,
                entity_id,
                "created with no offer left to close (GM path, expiry or disconnect)"
            ),
        }
    } else {
        let left = space_mgr.org_creations.charge_attempt(player_id);
        telemetry::attempt_charged(actor, left, "base");
    }
}

/// `DisconnectEntity`: drop the player's offer.
pub fn on_disconnect(entity_id: u32, space_mgr: &mut SpaceManager) {
    let actor = space_mgr.player_identity(entity_id);
    let Some(player_id) = actor.player_id else {
        return;
    };
    if let Some(p) = space_mgr.org_creations.clear(player_id) {
        telemetry::pending_expired(actor, &p, "disconnect");
    }
}
