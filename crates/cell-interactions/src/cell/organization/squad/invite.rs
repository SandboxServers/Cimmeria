//! Issuing a squad invite (`/squadinvite`, base 0xD0 type 0). The answer
//! (CM 8 `organizationInviteResponse`) is the org plugin's
//! (`cimmeria-cell-org`).

use cimmeria_cell_world::cell::squad::SquadResources;
use std::time::Instant;

use tokio::sync::mpsc;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::{PlayerNameLookup, SpaceManager};

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
pub async fn handle_invite(
    player_id: i32,
    entity_id: u32,
    target_name: &str,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    // The outcome row already carries the refusal; the base forward has
    // nothing more to do with it.
    let _ = issue(player_id, entity_id, target_name, tx, space_mgr).await;
}

/// [`handle_invite`], returning the refusal reason (the GM console's
/// `.squad_invite` reports it on `org.gm_action`).
#[tracing::instrument(
    name = "squad.invite",
    level = "info",
    target = "squad",
    skip_all,
    fields(player_id = player_id, entity_id = entity_id)
)]
pub(super) async fn issue(
    player_id: i32,
    entity_id: u32,
    target_name: &str,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> Result<(), Reason> {
    let Some(inviter) = forwarded_actor(space_mgr, player_id, entity_id) else {
        Outcome::new(Action::Invite, entity_id, tm::claimed(player_id))
            .rejected(Reason::ActorMismatch);
        return Err(Reason::ActorMismatch);
    };
    let mut out = Outcome::new(
        Action::Invite,
        entity_id,
        tm::of_entity(space_mgr, entity_id),
    );
    out.squad_id = space_mgr.resources.squads().squad_of(player_id);
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
            reject(tx, entity_id, instance, &text).await;
            return Err(reason);
        }
    };
    out.target = Some(tm::of_entity(space_mgr, target_entity));
    let Some(target) = actor(space_mgr, target_entity) else {
        // `character_name` is player-only, so this is a player entity that
        // has not finished `InitPlayerState`, or a corrupt row.
        out.rejected(Reason::NotAPlayer);
        let text = feedback::target_not_found(target_name);
        reject(tx, entity_id, instance, &text).await;
        return Err(Reason::NotAPlayer);
    };
    let result = space_mgr.resources.squads_mut().invite(
        player_id,
        &inviter.name,
        target.player_id,
        Instant::now(),
    );
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
            Ok(())
        }
        Err(r) => {
            out.rejected(r.into());
            let text = feedback::invite_rejected(r, &target.name);
            reject(tx, entity_id, instance, &text).await;
            Err(r.into())
        }
    }
}
