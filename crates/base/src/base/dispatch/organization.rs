//! SGWPlayer base methods 0xCF-0xD2: the OrganizationMember invite, invite
//! by type, kick and rank change.
//!
//! ORG-01 decodes each call and answers it; the organizations campaign fills
//! the behaviour in here, not in `mod.rs` (ORG-03 squad invites and kicks,
//! ORG-07 Team and Command invites, kicks and rank changes;
//! `docs/analysis/organizations/work-packets.md`).
//!
//! # Feedback until then
//!
//! Every well-formed call is answered, so the press is never silent:
//!
//! 1. `onErrorCode` [121] with `SystemID = ERRORCODE_SYSTEM_Ability` (0,
//!    the only `EErrorCodeSystem` value), `InstanceID` = the org id (or the
//!    type for invite-by-type), and `ErrorCodeID =
//!    CONDITION_FEEDBACK_InvalidEntity` (0), the generic "you can't do that"
//!    the GM gate already uses for a refused call (`gm_gate.rs`).
//!    `EConditionHandlerFeedback` has no organization token.
//! 2. A feedback chat line. AT-E1 found no Lua consumer of `onErrorCode`
//!    in the client (`docs/reverse-engineering/findings/ability-trainer-ui.md`
//!    §2), and whether it renders org text at all is ORG-E1 Q4, so the error
//!    code alone may show the player nothing.
//!
//! A payload that does not decode is a forged or corrupted call; it is
//! logged and gets no answer.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_base_session::base::gm_feedback::send_gm_feedback_to_client;
use cimmeria_base_session::base::helpers::send_to_witness_reliable;
use cimmeria_mercury::transport::Transport;
use cimmeria_wire::base::organization::{decode_org_base_method, OrgBaseCall};
use cimmeria_wire::cell::client_methods::organization::ORG_NOT_AVAILABLE_TEXT;
use cimmeria_wire::cell::client_methods::player::{
    build_on_error_code, CONDITION_FEEDBACK_INVALID_ENTITY, ERRORCODE_SYSTEM_ABILITY, ON_ERROR_CODE,
};

use crate::mercury::build_player_entity_method_packet;

use super::super::ConnectedClientState;

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

/// Decode, log and answer one organization base method.
pub(super) async fn handle_org_base_method(
    msg_id: u8,
    payload: &[u8],
    addr: SocketAddr,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
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

    let Some(entity_id) = entity_id else {
        // No player entity: the session has not entered the world, so there
        // is no client-side player to answer.
        return;
    };
    let args = build_on_error_code(
        ERRORCODE_SYSTEM_ABILITY,
        instance_id(&call),
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
    send_gm_feedback_to_client(
        entity_id,
        ORG_NOT_AVAILABLE_TEXT,
        transport,
        connected,
        entity_to_addr,
    )
    .await;
}
