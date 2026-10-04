//! Founding a Team or Command, cell side (ORG-05): the org plugin's half.
//!
//! - [`on_organization_creation`]: cell method 94. Honoured only against
//!   the player's pending creation, whose type it uses (the wire carries
//!   only the name); the name must pass D-ORG10; then
//!   `OrgCellToBase::Create` goes to the base, which creates, answers the
//!   client and replies with `on_create_result`.
//! - [`on_disconnect`]: the plugin's `AfterDisconnectTradeCancel` hook drops
//!   the offer.
//!
//! The registrar reply (`on_registrar_eligible`), the create result
//! (`on_create_result`), the replies and the telemetry stay in
//! `cimmeria-cell-interactions` (`cell::organization::creation`), because
//! the base-message handler calls them. This module re-exports that one
//! whole, so the moved code names it by the same paths as before the move.

pub use cimmeria_cell_interactions::cell::organization::creation::*;

use std::time::Instant;

use tokio::sync::mpsc;

use cimmeria_entity::organization::{org_text, TextField};
use cimmeria_wire::cell::cell_methods::organization::decode_on_organization_creation;
use cimmeria_wire::cell::client_methods::player::org_creation_ret_code as rc;

use crate::cell::messages::{CellToBaseMsg, OrgCellToBase};
use crate::cell::org_creation::{OrgCreationResources, TakeMiss};
use crate::cell::space_manager::SpaceManager;
use telemetry::{Action, Outcome};

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
                method_name = cimmeria_wire::names::player_cell_method(94u16),
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

    let org_type = match space_mgr.resources.org_creations_mut().begin_attempt(
        player_id,
        space_id,
        Instant::now(),
    ) {
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
            let left = space_mgr
                .resources
                .org_creations_mut()
                .charge_attempt(player_id);
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
        let left = space_mgr
            .resources
            .org_creations_mut()
            .charge_attempt(player_id);
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

/// `DisconnectEntity`: drop the player's offer.
pub fn on_disconnect(entity_id: u32, space_mgr: &mut SpaceManager) {
    let actor = space_mgr.player_identity(entity_id);
    let Some(player_id) = actor.player_id else {
        return;
    };
    if let Some(p) = space_mgr.resources.org_creations_mut().clear(player_id) {
        telemetry::pending_expired(actor, &p, "disconnect");
    }
}
