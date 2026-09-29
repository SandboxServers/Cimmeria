//! Answering a squad invite: CM 8 `organizationInviteResponse` for a squad
//! request id. Issuing one is `cimmeria-cell-interactions`'
//! `cell::organization::squad` (the base forwards it).

use std::time::Instant;

use tokio::sync::mpsc;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use crate::cell::squad::{SquadResources, TakeMiss};

use super::telemetry::{self as tm, Action, Outcome, Reason};
use super::{actor, confirm, fanout, feedback, reject};

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
        .resources
        .squads_mut()
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
    match space_mgr
        .resources
        .squads_mut()
        .accept(&invite, invitee, inviter)
    {
        Ok(joined) => {
            out.squad_id = Some(joined.squad_id);
            out.ok();
            space_mgr
                .resources
                .squads_mut()
                .note_entity(player_id, entity_id);
            if let Some(eid) = inviter_entity {
                space_mgr
                    .resources
                    .squads_mut()
                    .note_entity(invite.inviter_player_id, eid);
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
