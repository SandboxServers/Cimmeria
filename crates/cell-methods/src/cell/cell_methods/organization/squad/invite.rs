//! Squad invites: issuing one (`/squadinvite`, base 0xD0 type 0) and
//! answering one (CM 8 `organizationInviteResponse`).

use std::time::Instant;

use tokio::sync::mpsc;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::{PlayerNameLookup, SpaceManager};
use crate::cell::squad::TakeMiss;

use cimmeria_entity::organization::OrgType;
use cimmeria_wire::cell::client_methods::organization::{
    build_on_organization_invite, ON_ORGANIZATION_INVITE,
};

use super::telemetry::{self as tm, Action, Outcome, Reason};
use super::{actor, confirm, fanout, feedback, forwarded_actor, reject};

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
#[tracing::instrument(
    name = "squad.invite",
    level = "info",
    target = "squad",
    skip_all,
    fields(player_id = player_id, entity_id = entity_id)
)]
pub async fn handle_invite(
    player_id: i32,
    entity_id: u32,
    target_name: &str,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let Some(inviter) = forwarded_actor(space_mgr, player_id, entity_id) else {
        return Outcome::new(Action::Invite, entity_id, tm::claimed(player_id))
            .rejected(Reason::ActorMismatch);
    };
    let mut out = Outcome::new(
        Action::Invite,
        entity_id,
        tm::of_entity(space_mgr, entity_id),
    );
    out.squad_id = space_mgr.squads.squad_of(player_id);
    let instance = out.squad_id.unwrap_or(0);
    let resolved = match space_mgr.find_online_player_by_name(target_name) {
        PlayerNameLookup::Found { entity_id: t, .. } if t == entity_id => {
            Err((Reason::SelfTarget, feedback::INVITE_SELF.to_owned()))
        }
        PlayerNameLookup::Found { entity_id: t, .. } => Ok(t),
        PlayerNameLookup::InTransition { .. } => Err((
            Reason::TargetInTransition,
            feedback::target_travelling(target_name),
        )),
        PlayerNameLookup::NotFound => Err((
            Reason::TargetNotFound,
            feedback::target_not_found(target_name),
        )),
        PlayerNameLookup::Ambiguous { .. } => Err((
            Reason::TargetAmbiguous,
            feedback::target_ambiguous(target_name),
        )),
    };
    let target_entity = match resolved {
        Ok(t) => t,
        Err((reason, text)) => {
            out.rejected(reason);
            return reject(tx, entity_id, instance, &text).await;
        }
    };
    out.target = Some(tm::of_entity(space_mgr, target_entity));
    let Some(target) = actor(space_mgr, target_entity) else {
        // `character_name` is player-only, so this is a player entity that
        // has not finished `InitPlayerState`, or a corrupt row.
        out.rejected(Reason::NotAPlayer);
        let text = feedback::target_not_found(target_name);
        return reject(tx, entity_id, instance, &text).await;
    };
    let result =
        space_mgr
            .squads
            .invite(player_id, &inviter.name, target.player_id, Instant::now());
    tm::invites_expired(space_mgr);
    match result {
        Ok(issued) => {
            out.request_id = Some(issued.request_id);
            tm::invite_created(
                issued.request_id,
                issued.squad_id,
                out.actor,
                out.target.unwrap_or_default(),
            );
            out.ok();
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
            out.rejected(r.into());
            let text = feedback::invite_rejected(r, &target.name);
            reject(tx, entity_id, instance, &text).await;
        }
    }
}

/// CM 8 `organizationInviteResponse` for a squad request id.
///
/// The invite is looked up under the caller's own `player_id` and the
/// request id together and consumed by this response whatever it is
/// (D-ORG06); an accept is then re-validated before anyone joins.
#[tracing::instrument(
    name = "squad.invite_response",
    level = "info",
    target = "squad",
    skip_all,
    fields(entity_id = entity_id, request_id = request_id, accept = accept)
)]
pub async fn respond(
    entity_id: u32,
    request_id: i32,
    accept: bool,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let mut out = Outcome::new(
        Action::InviteResponse,
        entity_id,
        tm::of_entity(space_mgr, entity_id),
    );
    out.request_id = Some(request_id);
    let Some(invitee) = actor(space_mgr, entity_id) else {
        out.rejected(Reason::NotReady);
        return reject(tx, entity_id, 0, feedback::NOT_READY).await;
    };
    let player_id = invitee.player_id;
    let taken = space_mgr
        .squads
        .take_invite(player_id, request_id, Instant::now());
    tm::invites_expired(space_mgr);
    let invite = match taken {
        Ok(invite) => invite,
        Err(miss) => {
            // Never issued to this player, already answered, expired, or
            // another player's id. A real client answers only invites it
            // was shown, so all but the late answer are replays or forgeries.
            out.rejected(miss.into());
            let text = if miss == TakeMiss::Expired {
                feedback::INVITE_EXPIRED
            } else {
                feedback::INVITE_INVALID
            };
            return reject(tx, entity_id, 0, text).await;
        }
    };
    let inviter_id = tm::of_player(space_mgr, invite.inviter_player_id);
    out.target = Some(inviter_id);
    out.squad_id = invite.squad_id;
    tm::invite_consumed(request_id, invite.squad_id, accept, out.actor, inviter_id);

    if !accept {
        out.ok();
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

    let inviter_entity = space_mgr.player_entity_by_player_id(invite.inviter_player_id);
    let inviter = inviter_entity.and_then(|eid| actor(space_mgr, eid));
    match space_mgr.squads.accept(&invite, invitee, inviter) {
        Ok(joined) => {
            out.squad_id = Some(joined.squad_id);
            out.ok();
            space_mgr.squads.note_entity(player_id, entity_id);
            if let Some(eid) = inviter_entity {
                space_mgr.squads.note_entity(invite.inviter_player_id, eid);
            }
            let newcomers: &[i32] = if joined.created {
                &[invite.inviter_player_id, player_id]
            } else {
                &[player_id]
            };
            fanout::announce_join(tx, space_mgr, joined.squad_id, joined.created, newcomers).await;
        }
        Err(r) => {
            out.rejected(r.into());
            let text = feedback::response_rejected(r, &invite.inviter_name);
            reject(tx, entity_id, 0, &text).await;
        }
    }
}
