//! Squad invites: issuing one (`/squadinvite`, base 0xD0 type 0) and
//! answering one (CM 8 `organizationInviteResponse`).

use std::time::Instant;

use tokio::sync::mpsc;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::{PlayerNameLookup, SpaceManager};
use crate::cell::squad::ResponseReject;

use cimmeria_entity::organization::OrgType;
use cimmeria_wire::base::organization::ORGANIZATION_INVITE_BY_TYPE;
use cimmeria_wire::cell::cell_methods::organization::INVITE_RESPONSE;
use cimmeria_wire::cell::client_methods::organization::{
    build_on_organization_invite, ON_ORGANIZATION_INVITE,
};

use super::{actor, confirm, fanout, feedback, forwarded_actor, reject};

const INVITE: u16 = ORGANIZATION_INVITE_BY_TYPE as u16;

/// `organizationInviteByType(0, target_name)`, forwarded by the base with
/// the inviter's ids from its session.
///
/// Resolves the name across every space; an ambiguous, travelling, unknown
/// or non-player target, or the inviter themselves, is refused with
/// feedback, never guessed. The registry then checks both sides and the
/// limits (D-ORG06). The contact-list ignore check the packet names is not
/// made: ignore lists live only in the base's database and the cell has no
/// copy (a known gap, recorded in the ORG-03 worknote).
///
/// On success the target gets `onOrganizationInvite` [34] and the inviter a
/// confirmation line.
pub async fn handle_invite(
    player_id: i32,
    entity_id: u32,
    target_name: &str,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let Some(inviter) = forwarded_actor(space_mgr, player_id, entity_id) else {
        return;
    };
    let instance = space_mgr.squads.squad_of(player_id).unwrap_or(0);
    let refuse = |reason: &'static str| {
        tracing::debug!(
            target: "squad",
            event = "squad.invite_rejected",
            player_id,
            entity_id,
            reason,
            "squad invite refused"
        );
    };
    let target_entity = match space_mgr.find_online_player_by_name(target_name) {
        PlayerNameLookup::Found { entity_id, .. } => entity_id,
        PlayerNameLookup::InTransition { .. } => {
            refuse("target_in_transition");
            let text = feedback::target_travelling(target_name);
            return reject(tx, entity_id, INVITE, instance, &text).await;
        }
        PlayerNameLookup::NotFound => {
            refuse("target_not_found");
            let text = feedback::target_not_found(target_name);
            return reject(tx, entity_id, INVITE, instance, &text).await;
        }
        PlayerNameLookup::Ambiguous { entity_ids } => {
            tracing::warn!(
                target: "squad",
                event = "squad.invite_rejected",
                player_id,
                entity_id,
                reason = "target_ambiguous",
                ?entity_ids,
                "squad invite refused: the name matches more than one entity"
            );
            let text = feedback::target_ambiguous(target_name);
            return reject(tx, entity_id, INVITE, instance, &text).await;
        }
    };
    if target_entity == entity_id {
        refuse("self");
        return reject(tx, entity_id, INVITE, instance, feedback::INVITE_SELF).await;
    }
    let Some(target) = actor(space_mgr, target_entity) else {
        // `character_name` is player-only, so this is a player entity that
        // has not finished `InitPlayerState`, or a corrupt row.
        refuse("target_not_player");
        let text = feedback::target_not_found(target_name);
        return reject(tx, entity_id, INVITE, instance, &text).await;
    };
    match space_mgr
        .squads
        .invite(player_id, &inviter.name, target.player_id, Instant::now())
    {
        Ok(issued) => {
            tracing::debug!(
                target: "squad",
                event = "squad.invite_sent",
                player_id,
                entity_id,
                target_player_id = target.player_id,
                request_id = issued.request_id,
                squad_id = issued.squad_id,
                "squad invite sent"
            );
            // Squads have no name; the client's squad invite window shows
            // the inviter's.
            fanout::send(
                tx,
                target_entity,
                ON_ORGANIZATION_INVITE,
                build_on_organization_invite(
                    &inviter.name,
                    OrgType::Squad,
                    issued.request_id,
                    "",
                    false,
                ),
            )
            .await;
            confirm(tx, entity_id, &feedback::invite_sent(&target.name)).await;
        }
        Err(r) => {
            refuse(r.reason());
            let text = feedback::invite_rejected(r, &target.name);
            reject(tx, entity_id, INVITE, instance, &text).await;
        }
    }
}

/// CM 8 `organizationInviteResponse` for a squad request id.
///
/// The invite is looked up under the caller's own `player_id` and the
/// request id together and consumed by this response whatever it is
/// (D-ORG06); an accept is then re-validated before anyone joins.
pub async fn respond(
    entity_id: u32,
    request_id: i32,
    accept: bool,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let Some(invitee) = actor(space_mgr, entity_id) else {
        return reject(tx, entity_id, INVITE_RESPONSE, 0, feedback::NOT_READY).await;
    };
    let player_id = invitee.player_id;
    let Some(invite) = space_mgr
        .squads
        .take_invite(player_id, request_id, Instant::now())
    else {
        // Never issued to this player, already answered, or expired. A
        // real client answers only invites it was shown, inside the
        // window, so this is a replay, a forged id or a very late answer.
        tracing::warn!(
            target: "squad",
            event = "squad.response_rejected",
            player_id,
            entity_id,
            request_id,
            accept,
            reason = ResponseReject::UnknownRequest.reason(),
            "squad invite response matches no pending invite for this player"
        );
        let text = feedback::response_rejected(ResponseReject::UnknownRequest, "");
        return reject(tx, entity_id, INVITE_RESPONSE, 0, &text).await;
    };

    if !accept {
        tracing::debug!(
            target: "squad",
            event = "squad.invite_declined",
            player_id,
            entity_id,
            request_id,
            inviter_player_id = invite.inviter_player_id,
            "squad invite declined"
        );
        if let Some(inviter_entity) = space_mgr.player_entity_by_player_id(invite.inviter_player_id)
        {
            confirm(
                tx,
                inviter_entity,
                &feedback::invite_declined(&invitee.name),
            )
            .await;
        }
        return;
    }

    let inviter = space_mgr
        .player_entity_by_player_id(invite.inviter_player_id)
        .and_then(|eid| actor(space_mgr, eid));
    match space_mgr.squads.accept(&invite, invitee, inviter) {
        Ok(out) => {
            tracing::info!(
                target: "squad",
                event = "squad.joined",
                player_id,
                entity_id,
                squad_id = out.squad_id,
                created = out.created,
                inviter_player_id = invite.inviter_player_id,
                "player joined a squad"
            );
            let newcomers: &[i32] = if out.created {
                &[invite.inviter_player_id, player_id]
            } else {
                &[player_id]
            };
            fanout::announce_join(tx, space_mgr, out.squad_id, newcomers).await;
        }
        Err(r) => {
            tracing::debug!(
                target: "squad",
                event = "squad.response_rejected",
                player_id,
                entity_id,
                request_id,
                accept,
                reason = r.reason(),
                "squad invite accept failed re-validation"
            );
            let text = feedback::response_rejected(r, &invite.inviter_name);
            reject(tx, entity_id, INVITE_RESPONSE, 0, &text).await;
        }
    }
}
