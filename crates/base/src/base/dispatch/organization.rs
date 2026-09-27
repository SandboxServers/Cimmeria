//! SGWPlayer base methods 0xCF-0xD2: the OrganizationMember invite, invite
//! by type, kick and rank change.
//!
//! The organizations campaign fills the behaviour in here, not in `mod.rs`
//! (`docs/analysis/organizations/work-packets.md`):
//!
//! - **Squads (ORG-03).** Squads live on the cell (D-ORG03), so the base
//!   resolves nothing: `organizationInviteByType` with type 0 is forwarded
//!   as `OrgBaseToCell::SquadInvite`, and `organizationKick` with an org id
//!   in the squad range (D-ORG05) as `OrgBaseToCell::SquadKick`. ORG-E1 Q2:
//!   the client's `squadInvite` and `squadKick` natives use exactly these
//!   two methods. The actor's `player_id` and `entity_id` come from this
//!   session, never from the payload.
//! - **Invite by type above 2** names no organization type and is refused
//!   (CAT-M-02).
//! - **Everything else** (Team and Command invites, kicks and rank changes)
//!   waits for ORG-07 and is answered as below.
//!
//! # Feedback until ORG-07
//!
//! Every well-formed call the base cannot serve yet is answered, so the
//! press is never silent:
//!
//! 1. `onErrorCode` [121] with `SystemID = ERRORCODE_SYSTEM_Ability` (0,
//!    the only `EErrorCodeSystem` value), `InstanceID` = the org id (or the
//!    type for invite-by-type), and `ErrorCodeID =
//!    CONDITION_FEEDBACK_InvalidEntity` (0), the generic "you can't do that"
//!    the GM gate already uses for a refused call (`gm_gate.rs`).
//!    `EConditionHandlerFeedback` has no organization token.
//! 2. A feedback chat line. AT-E1 found no Lua consumer of `onErrorCode`
//!    in the client (`docs/reverse-engineering/findings/ability-trainer-ui.md`
//!    §2), and ORG-E1 Q4 found no org text for it, so the error code alone
//!    may show the player nothing.
//!
//! A payload that does not decode is a forged or corrupted call; it is
//! logged and gets no answer.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_base_session::base::gm_feedback::send_gm_feedback_to_client;
use cimmeria_base_session::base::helpers::send_to_witness_reliable;
use cimmeria_entity::organization::{route_org_id, OrgRoute, OrgType};
use cimmeria_mercury::transport::Transport;
use cimmeria_wire::base::organization::{decode_org_base_method, OrgBaseCall};
use cimmeria_wire::cell::client_methods::organization::ORG_NOT_AVAILABLE_TEXT;
use cimmeria_wire::cell::client_methods::player::{
    build_on_error_code, CONDITION_FEEDBACK_INVALID_ENTITY, ERRORCODE_SYSTEM_ABILITY, ON_ERROR_CODE,
};
use tokio::sync::mpsc;

use crate::cell::messages::{BaseToCellMsg, OrgBaseToCell};
use crate::mercury::build_player_entity_method_packet;

use super::super::ConnectedClientState;

/// The line an `organizationInviteByType` with a type above 2 gets.
pub(super) const UNKNOWN_ORG_TYPE_TEXT: &str = "That is not an organization type.";

/// `InstanceID` for the error code: the org id the call names, or the type
/// byte for invite-by-type.
fn instance_id(call: &OrgBaseCall) -> i32 {
    match *call {
        OrgBaseCall::Invite { org_id, .. }
        | OrgBaseCall::Kick { org_id, .. }
        | OrgBaseCall::RankChange { org_id, .. } => org_id,
        OrgBaseCall::InviteByType { org_type, .. } => i32::from(org_type),
    }
}

/// The squad message a call forwards to the cell, if it is a squad call.
fn squad_forward(call: &OrgBaseCall, player_id: i32, entity_id: u32) -> Option<OrgBaseToCell> {
    match call {
        OrgBaseCall::InviteByType {
            org_type,
            player_name,
        } if *org_type == OrgType::Squad.as_u8() => Some(OrgBaseToCell::SquadInvite {
            player_id,
            entity_id,
            target_name: player_name.clone(),
        }),
        OrgBaseCall::Kick {
            org_id,
            player_name,
        } if route_org_id(*org_id) == Some(OrgRoute::Squad) => Some(OrgBaseToCell::SquadKick {
            player_id,
            entity_id,
            org_id: *org_id,
            target_name: player_name.clone(),
        }),
        _ => None,
    }
}

/// Decode, route and answer one organization base method.
pub(super) async fn handle_org_base_method(
    msg_id: u8,
    payload: &[u8],
    addr: SocketAddr,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
    cell_tx: &Option<mpsc::Sender<BaseToCellMsg>>,
) {
    // The actor comes from this session, never from the payload.
    let (player_id, entity_id) = {
        let clients = connected.lock().unwrap();
        clients
            .get(&addr)
            .map_or((None, None), |c| (c.active_player_id, c.player_entity_id))
    };

    let call = match decode_org_base_method(msg_id, payload) {
        Ok(call) => call,
        Err(e) => {
            tracing::warn!(
                target: "org",
                event = "org.base_method_malformed",
                %addr,
                msg_id = format_args!("{msg_id:#04x}"),
                player_id,
                entity_id,
                reason = e.reason(),
                error = %e,
                "organization base method payload did not decode"
            );
            return;
        }
    };

    let (Some(actor_player), Some(actor_entity)) = (player_id, entity_id) else {
        // No player entity: the session has not entered the world, so there
        // is no client-side player to act for or to answer.
        tracing::debug!(
            target: "org",
            event = "org.base_method_no_player",
            %addr,
            msg_id = format_args!("{msg_id:#04x}"),
            method = call.method_name(),
            "organization base method from a session with no player entity"
        );
        return;
    };
    let reply = |text: &'static str| {
        answer(
            instance_id(&call),
            text,
            actor_entity,
            transport,
            connected,
            entity_to_addr,
        )
    };

    if let OrgBaseCall::InviteByType { org_type, .. } = call {
        if OrgType::try_from(org_type).is_err() {
            tracing::warn!(
                target: "org",
                event = "org.invite_by_type_rejected",
                %addr,
                player_id,
                entity_id,
                org_type,
                reason = "type_out_of_range",
                "organizationInviteByType names no organization type"
            );
            reply(UNKNOWN_ORG_TYPE_TEXT).await;
            return;
        }
    }

    let Some(fwd) = squad_forward(&call, actor_player, actor_entity) else {
        tracing::debug!(
            target: "org",
            event = "org.base_method_unimplemented",
            %addr,
            msg_id = format_args!("{msg_id:#04x}"),
            method = call.method_name(),
            player_id,
            entity_id,
            instance_id = instance_id(&call),
            "organization base method has no handler yet; answering with feedback"
        );
        reply(ORG_NOT_AVAILABLE_TEXT).await;
        return;
    };
    let kind = fwd.kind();
    let forwarded = match cell_tx {
        Some(tx) => tx.send(BaseToCellMsg::Org(fwd)).await.is_ok(),
        None => false,
    };
    if forwarded {
        tracing::debug!(
            target: "org",
            event = "org.squad_forwarded",
            %addr,
            player_id,
            entity_id,
            kind,
            "squad call forwarded to the cell"
        );
    } else {
        tracing::warn!(
            target: "org",
            event = "org.squad_forward_failed",
            %addr,
            player_id,
            entity_id,
            kind,
            reason = "cell_unreachable",
            "squad call could not reach the cell -- answering with feedback"
        );
        reply(ORG_NOT_AVAILABLE_TEXT).await;
    }
}

/// `onErrorCode(0, instance_id, 0)` then `text` on the feedback channel.
async fn answer(
    instance_id: i32,
    text: &str,
    entity_id: u32,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    let args = build_on_error_code(
        ERRORCODE_SYSTEM_ABILITY,
        instance_id,
        CONDITION_FEEDBACK_INVALID_ENTITY,
    );
    send_to_witness_reliable(
        transport,
        connected,
        entity_to_addr,
        entity_id,
        |key, version, seq, acks| {
            build_player_entity_method_packet(
                key,
                seq,
                acks,
                entity_id,
                ON_ERROR_CODE,
                &args,
                version,
            )
        },
    )
    .await;
    send_gm_feedback_to_client(entity_id, text, transport, connected, entity_to_addr).await;
}
