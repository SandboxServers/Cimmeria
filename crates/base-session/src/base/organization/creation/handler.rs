//! The base's creation handlers: the registrar eligibility check, the named
//! creation, and the GM `.org_create`. Each answers the client itself and
//! ends in one outcome row ([`super::telemetry`]).

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_entity::cell_entity::PlayerIdentity;
use cimmeria_entity::organization::{OrgRank, OrgType};
use cimmeria_mercury::transport::Transport;
use cimmeria_wire::cell::client_methods::player::{
    build_on_organization_creation_result, org_creation_ret_code, ON_ORGANIZATION_CREATION_RESULT,
    ORG_CREATION_RESULT_CREATED, ORG_CREATION_RESULT_REFUSED,
};
use sqlx::PgPool;
use tokio::sync::mpsc;

use super::super::handlers::{push_org_state, OrgCtx, OrgPlayer};
use super::super::persistence::OrgStoreError;
use super::telemetry::{Action, Outcome};
use super::{found_organization, is_eligible, FoundReject, Founded};
use crate::base::gm_feedback::send_gm_feedback_to_client;
use crate::base::helpers::send_to_witness_reliable;
use crate::base::ConnectedClientState;
use crate::cell::messages::{BaseToCellMsg, OrgBaseToCell};
use crate::mercury::{build_player_entity_method_packet, method_idx};

/// Minimum session `access_level` for `.org_create` (GameMaster), the same
/// threshold as the cell's `.`-console gate.
const GM_ACCESS_LEVEL: u32 = 2;

/// Everything the handlers reach: the pool, the cell channel and the
/// client-send maps.
pub struct CreationCtx<'a> {
    pub db_pool: &'a Option<Arc<PgPool>>,
    pub cell_tx: &'a Option<mpsc::Sender<BaseToCellMsg>>,
    pub transport: &'a Arc<dyn Transport>,
    pub connected: &'a Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    pub entity_to_addr: &'a Arc<Mutex<HashMap<u32, SocketAddr>>>,
}

/// What the base's own session says about the entity's client.
#[derive(Debug, Clone, Copy)]
struct Session {
    identity: PlayerIdentity,
    access_level: u32,
}

impl CreationCtx<'_> {
    fn session(&self, entity_id: u32) -> Option<Session> {
        let addr = self.entity_to_addr.lock().ok()?.get(&entity_id).copied()?;
        let clients = self.connected.lock().ok()?;
        let c = clients.get(&addr)?;
        Some(Session {
            identity: crate::base::session_identity::session_identity(c),
            access_level: c.access_level,
        })
    }

    /// The session behind `entity_id`, if it is still character
    /// `player_id`. Entity ids are recycled: a message the cell sent for
    /// one character must not act for whoever holds the id now.
    fn actor(&self, player_id: i32, entity_id: u32, action: Action) -> Option<Session> {
        let session = self.session(entity_id);
        match session {
            Some(s) if s.identity.player_id == Some(player_id) => Some(s),
            _ => {
                tracing::warn!(
                    target: "org",
                    event = "org.actor_mismatch",
                    reason = "actor_mismatch",
                    player_id,
                    entity_id,
                    session_player_id = session.and_then(|s| s.identity.player_id),
                    action = ?action,
                    "organization request names an entity that is no longer that character"
                );
                None
            }
        }
    }

    fn pool(&self) -> Option<&PgPool> {
        self.db_pool.as_deref()
    }

    /// Send one client method to `entity_id`; WARN on a dropped send.
    async fn send(&self, entity_id: u32, method_index: u16, args: &[u8]) -> bool {
        let out = send_to_witness_reliable(
            self.transport,
            self.connected,
            self.entity_to_addr,
            entity_id,
            |key, version, seq, acks| {
                build_player_entity_method_packet(
                    key,
                    seq,
                    acks,
                    entity_id,
                    method_index,
                    args,
                    version,
                )
            },
        )
        .await;
        match out.failure_reason() {
            None => true,
            Some(reason) => {
                tracing::warn!(
                    target: "org",
                    event = "org.send_failed",
                    reason,
                    entity_id,
                    method_index,
                    method_name = cimmeria_wire::names::player_client_method(method_index),
                    "organization client method could not be sent"
                );
                false
            }
        }
    }

    async fn line(&self, entity_id: u32, text: &str) {
        send_gm_feedback_to_client(
            entity_id,
            text,
            self.transport,
            self.connected,
            self.entity_to_addr,
        )
        .await;
    }

    /// Tell the cell, WARN if it cannot be told.
    async fn tell_cell(&self, msg: OrgBaseToCell) -> bool {
        let kind = msg.kind();
        let (player_id, entity_id) = msg.actor();
        let sent = match self.cell_tx {
            Some(tx) => tx.send(BaseToCellMsg::Org(msg)).await.is_ok(),
            None => false,
        };
        if !sent {
            tracing::warn!(
                target: "org",
                event = "org.cell_send_failed",
                reason = "cell_unreachable",
                kind,
                player_id,
                entity_id,
                "organization reply could not reach the cell"
            );
        }
        sent
    }
}

/// "Team" or "Command", for player-facing lines.
fn type_word(org_type: OrgType) -> &'static str {
    match org_type {
        OrgType::Team => "Team",
        OrgType::Command => "Command",
        OrgType::Squad => "Squad",
    }
}

/// The line an ineligible player gets (D-ORG18).
pub fn already_in_type_text(org_type: OrgType) -> String {
    format!(
        "You already belong to a {}. Leave it before founding another.",
        type_word(org_type)
    )
}

/// The line a successful founder gets beside the window opening.
pub fn founded_text(org_type: OrgType, name: &str) -> String {
    format!(
        "{} \"{name}\" founded. You are its leader.",
        type_word(org_type)
    )
}

/// The refusal text a failed creation gets.
pub const NAME_TAKEN_TEXT: &str = "That name is already taken. Choose another.";
/// The refusal text for a name that fails D-ORG10.
pub const NAME_INVALID_TEXT: &str =
    "That name is not allowed. Use 1-60 letters, digits, spaces, apostrophes, hyphens or periods.";
/// The refusal text when the database cannot be reached or fails.
pub const SERVER_ERROR_TEXT: &str = "The organization could not be created. Try again later.";

/// `OrgCellToBase::RegistrarOpen`: check D-ORG18 eligibility, then let the
/// cell open the naming dialog, or refuse with a line.
#[tracing::instrument(
    name = "org.registrar_open",
    level = "info",
    skip_all,
    fields(player_id, entity_id, npc_entity_id, org_type = org_type.name())
)]
pub async fn handle_registrar_open(
    ctx: &CreationCtx<'_>,
    player_id: i32,
    entity_id: u32,
    npc_entity_id: u32,
    org_type: OrgType,
) {
    let identity = ctx
        .session(entity_id)
        .map_or(PlayerIdentity::UNKNOWN, |s| s.identity);
    let mut row = Outcome::new(Action::RegistrarOpen, identity, entity_id, org_type);
    row.npc_entity_id = Some(npc_entity_id);
    if ctx
        .actor(player_id, entity_id, Action::RegistrarOpen)
        .is_none()
    {
        row.rejected("actor_mismatch");
        return;
    }
    let Some(pool) = ctx.pool() else {
        row.rejected("db_error");
        ctx.line(entity_id, SERVER_ERROR_TEXT).await;
        return;
    };
    match is_eligible(pool, player_id, org_type).await {
        Ok(true) => {}
        Ok(false) => {
            row.rejected("not_eligible");
            ctx.line(entity_id, &already_in_type_text(org_type)).await;
            return;
        }
        Err(e) => {
            tracing::warn!(
                target: "org",
                event = "org.registrar_open_lookup_failed",
                reason = e.reason(),
                player_id,
                entity_id,
                error = %e,
                "registrar eligibility read failed"
            );
            row.rejected("db_error");
            ctx.line(entity_id, SERVER_ERROR_TEXT).await;
            return;
        }
    }
    let forwarded = ctx
        .tell_cell(OrgBaseToCell::RegistrarEligible {
            player_id,
            entity_id,
            npc_entity_id,
            org_type,
        })
        .await;
    if !forwarded {
        row.rejected("cell_unreachable");
        ctx.line(entity_id, SERVER_ERROR_TEXT).await;
    }
    // Eligible and forwarded: the cell writes the `ok` row once the dialog
    // is open, so the action still has exactly one row.
}

/// `OrgCellToBase::Create`: found the organization, answer the client, and
/// tell the cell how its pending creation ended.
#[tracing::instrument(
    name = "org.create",
    level = "info",
    skip_all,
    fields(player_id, entity_id, org_type = org_type.name())
)]
pub async fn handle_create(
    ctx: &CreationCtx<'_>,
    player_id: i32,
    entity_id: u32,
    org_type: OrgType,
    name: &str,
) {
    let identity = ctx
        .session(entity_id)
        .map_or(PlayerIdentity::UNKNOWN, |s| s.identity);
    let mut row = Outcome::new(Action::Create, identity, entity_id, org_type);
    row.name_units = Some(name.encode_utf16().count());
    let created = if ctx.actor(player_id, entity_id, Action::Create).is_none() {
        row.rejected("actor_mismatch");
        false
    } else {
        found_and_answer(ctx, &mut row, player_id, name).await
    };
    ctx.tell_cell(OrgBaseToCell::CreateResult {
        player_id,
        entity_id,
        org_type,
        created,
    })
    .await;
}

/// `OrgCellToBase::GmCreate`: the GM `.org_create`. The access level is
/// re-read from this session, never taken from the cell (D-ORG13).
#[tracing::instrument(
    name = "org.gm_action",
    level = "info",
    skip_all,
    fields(player_id, entity_id, org_type = org_type.name())
)]
pub async fn handle_gm_create(
    ctx: &CreationCtx<'_>,
    player_id: i32,
    entity_id: u32,
    org_type: OrgType,
    name: &str,
) {
    let session = ctx.session(entity_id);
    let identity = session.map_or(PlayerIdentity::UNKNOWN, |s| s.identity);
    let mut row = Outcome::new(Action::GmCreate, identity, entity_id, org_type);
    row.name_units = Some(name.encode_utf16().count());
    let Some(session) = ctx.actor(player_id, entity_id, Action::GmCreate) else {
        row.rejected("actor_mismatch");
        return;
    };
    if session.access_level < GM_ACCESS_LEVEL {
        row.rejected("not_gm");
        ctx.line(entity_id, ".org_create is a GM command.").await;
        return;
    }
    found_and_answer(ctx, &mut row, player_id, name).await;
}

/// The refusal reason, 134 `RetCode` and line for a failed creation.
fn refusal(org_type: OrgType, e: &FoundReject) -> (&'static str, u8, String) {
    use org_creation_ret_code as rc;
    match e {
        FoundReject::Store(OrgStoreError::AlreadyInType) => (
            "already_in_org_type",
            rc::ALREADY_IN_ORG_TYPE,
            already_in_type_text(org_type),
        ),
        FoundReject::Store(OrgStoreError::NameTaken) => {
            ("name_taken", rc::NAME_TAKEN, NAME_TAKEN_TEXT.into())
        }
        FoundReject::Store(OrgStoreError::InvalidText(_)) => {
            ("text_invalid", rc::NAME_INVALID, NAME_INVALID_TEXT.into())
        }
        FoundReject::InsufficientFunds { cost, .. } => (
            "insufficient_funds",
            rc::INSUFFICIENT_FUNDS,
            format!(
                "Founding a {} costs {cost} naquadah, and you do not have enough.",
                type_word(org_type)
            ),
        ),
        FoundReject::Store(_) => ("db_error", rc::SERVER_ERROR, SERVER_ERROR_TEXT.into()),
    }
}

/// Found the organization and answer the founder: the push and a line on
/// success, 134 with a refusal code and a line otherwise. Writes the
/// outcome row. Returns whether it was created.
async fn found_and_answer(
    ctx: &CreationCtx<'_>,
    row: &mut Outcome,
    player_id: i32,
    name: &str,
) -> bool {
    let entity_id = row.entity_id;
    let org_type = row.org_type;
    let Some(pool) = ctx.pool() else {
        row.rejected("db_error");
        refuse(
            ctx,
            entity_id,
            org_creation_ret_code::SERVER_ERROR,
            SERVER_ERROR_TEXT,
        )
        .await;
        return false;
    };
    let founded = match found_organization(pool, org_type, name, player_id).await {
        Ok(f) => f,
        Err(e) => {
            let (reason, code, text) = refusal(org_type, &e);
            if reason == "db_error" {
                tracing::warn!(
                    target: "org",
                    event = "org.create_failed",
                    reason = e_reason(&e),
                    player_id,
                    entity_id,
                    error = %e,
                    "organization creation failed in the database"
                );
            }
            row.rejected(reason);
            refuse(ctx, entity_id, code, &text).await;
            return false;
        }
    };
    row.org_id = Some(founded.org.org_id);
    row.cash = founded.cash;
    tracing::debug!(
        target: "org",
        event = "member_joined",
        org_id = founded.org.org_id,
        org_type = org_type.name(),
        account_id = row.actor.account_id,
        player_id,
        rank = OrgRank::LEADER.as_u8(),
        "organization founder joined as leader"
    );
    row.ok();
    push_to_founder(ctx, row.actor, entity_id, player_id, org_type, &founded).await;
    true
}

fn e_reason(e: &FoundReject) -> &'static str {
    match e {
        FoundReject::Store(s) => s.reason(),
        FoundReject::InsufficientFunds { .. } => "insufficient_funds",
    }
}

/// 134 `(0, code)` then the line.
async fn refuse(ctx: &CreationCtx<'_>, entity_id: u32, code: u8, text: &str) {
    ctx.send(
        entity_id,
        ON_ORGANIZATION_CREATION_RESULT,
        &build_on_organization_creation_result(ORG_CREATION_RESULT_REFUSED, code),
    )
    .await;
    ctx.line(entity_id, text).await;
}

/// The creation result, ORG-06's state push, the new cash total when a cost
/// was paid, and the confirmation line.
///
/// 134 `(1, 0)` goes first so the naming dialog resolves whatever follows.
/// The organization's state is `push_org_state` with `new_member = true`:
/// [35] as Leader, the name, MOTD, cash, experience, rank masks and names,
/// the roster, and [37] marking the founder online. The organization is
/// committed by now; a push that fails is logged there (WARN
/// `org.state_push_failed`) and the window fills in at the next login.
async fn push_to_founder(
    ctx: &CreationCtx<'_>,
    actor: PlayerIdentity,
    entity_id: u32,
    player_id: i32,
    org_type: OrgType,
    founded: &Founded,
) {
    ctx.send(
        entity_id,
        ON_ORGANIZATION_CREATION_RESULT,
        &build_on_organization_creation_result(
            ORG_CREATION_RESULT_CREATED,
            org_creation_ret_code::OK,
        ),
    )
    .await;
    let org_ctx = OrgCtx {
        db_pool: ctx.db_pool,
        transport: ctx.transport,
        connected: ctx.connected,
        entity_to_addr: ctx.entity_to_addr,
        cell_tx: ctx.cell_tx,
    };
    let founder = OrgPlayer {
        account_id: actor.account_id,
        player_id,
        entity_id,
    };
    // Failure is logged inside (`org.state_push_failed`, with `reason`).
    let _ = push_org_state(&org_ctx, founded.org.org_id, &founder, true).await;
    if let Some((_, after)) = founded.cash {
        ctx.send(entity_id, method_idx::ON_CASH_CHANGED, &after.to_le_bytes())
            .await;
    }
    ctx.line(entity_id, &founded_text(org_type, &founded.org.name))
        .await;
}
