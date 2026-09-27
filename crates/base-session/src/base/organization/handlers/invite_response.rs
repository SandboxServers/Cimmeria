//! `organizationInviteResponse` (CM 8) for a base-issued request id
//! (`BASE_INVITE_REQUEST_FLAG` set), forwarded by the cell (ORG-07,
//! CAT-M-18).
//!
//! The invite is taken from the responder's own session in one call, keyed
//! by their `player_id` and the request id together (D-ORG06), so it is
//! single use and nobody else's id finds anything. A decline tells the
//! inviter. An accept re-validates everything under ORG-LOCK: the
//! organization still exists, the inviter is still a member whose rank
//! holds `Invite`, and the responder is in no organization of that type
//! (D-ORG18). Teams and Commands have no member cap, so there is no room
//! check. The responder joins at the type's entry rank (D-ORG07: Team 2,
//! Command 1), gets the organization's full state, and the other online
//! members get `onMemberJoinedOrganization` [37] as a new member.

use std::time::Instant;

use cimmeria_entity::organization::{OrgPermission, OrgRank, OrgType};
use cimmeria_wire::cell::client_methods::organization::{
    build_on_member_joined_organization, ON_MEMBER_JOINED_ORGANIZATION,
};
use sqlx::{Postgres, Transaction};

use super::answer::{db_failed, refusal_text, refuse, type_title};
use super::broadcast::broadcast_except;
use super::fanout::feedback;
use super::push::push_org_state;
use super::targets::online_now;
use super::telemetry::{ActionRow, OrgReject};
use super::{OrgCtx, OrgPlayer};
use crate::base::organization::api::{lock_org, member_access_locked, OrgAccess};
use crate::base::organization::invites::{PendingOrgInvite, TakeMiss};
use crate::base::organization::persistence::{add_member, OrgStoreError};

/// What a successful response did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InviteAnswer {
    Declined,
    /// The responder joined `org_id` at `rank`.
    Joined {
        org_id: i32,
        rank: OrgRank,
    },
}

/// Handle CM 8 for a base request id. `player` is the resolved session
/// (`resolve_actor`), never the payload.
#[tracing::instrument(
    name = "org.invite_response",
    level = "info",
    skip_all,
    fields(player_id = player.player_id, entity_id = player.entity_id, request_id, accept)
)]
pub async fn handle_invite_response(
    ctx: &OrgCtx<'_>,
    player: &OrgPlayer,
    request_id: i32,
    accept: bool,
    now: Instant,
) -> Result<InviteAnswer, OrgReject> {
    let mut row = ActionRow {
        event: "org.invite_response",
        action: "invite_response",
        account_id: player.account_id,
        player_id: Some(player.player_id),
        entity_id: Some(player.entity_id),
        request_id: Some(request_id),
        ..ActionRow::default()
    };
    let fail = |row: ActionRow, why: OrgReject, org_type: Option<OrgType>| async move {
        let text = match why {
            // The responder is the one already placed.
            OrgReject::AlreadyInOrgType | OrgReject::AlreadyMember => {
                format!("You are already in a {}.", type_title(org_type))
            }
            _ => refusal_text(why, "", org_type),
        };
        refuse(ctx, &row, player.entity_id, why, &text).await
    };

    let invite = match take(ctx, player, request_id, now) {
        Ok(i) => i,
        Err(miss) => {
            let why = match miss {
                TakeMiss::Unknown => OrgReject::InviteUnknown,
                TakeMiss::Expired => OrgReject::InviteExpired,
                TakeMiss::Foreign => OrgReject::InviteForeign,
            };
            return fail(row, why, None).await;
        }
    };
    row.org_id = Some(invite.org_id);
    row.org_type = Some(invite.org_type.name());
    row.target_player_id = Some(invite.inviter_player_id);
    let inviter_online = online_now(ctx, invite.inviter_player_id);
    row.target_account_id = inviter_online.and_then(|m| m.account_id);
    tracing::debug!(
        target: "org",
        event = "invite_consumed",
        account_id = player.account_id,
        player_id = player.player_id,
        target_player_id = invite.inviter_player_id,
        org_id = invite.org_id,
        request_id,
        accepted = accept,
        "organization invite answered"
    );

    if !accept {
        if let Some(inviter) = inviter_online {
            let name = super::targets::actor_name(ctx, player).unwrap_or_default();
            feedback(
                ctx,
                inviter.entity_id,
                &format!("{name} declined your invitation."),
            )
            .await;
        }
        row.ok("declined");
        return Ok(InviteAnswer::Declined);
    }

    let Some(pool) = ctx.db_pool.as_deref() else {
        return fail(row, OrgReject::NoDb, Some(invite.org_type)).await;
    };
    let joined = match pool.begin().await {
        Ok(mut tx) => match join_locked(&mut tx, player, &invite, &mut row).await {
            Ok(j) => match tx.commit().await {
                Ok(()) => Ok(j),
                Err(e) => Err(db_failed(&row, &e)),
            },
            Err(why) => Err(why),
        },
        Err(e) => Err(db_failed(&row, &e)),
    };
    let (rank, org_name, member_name) = match joined {
        Ok(j) => j,
        Err(why) => return fail(row, why, Some(invite.org_type)).await,
    };
    announce_join(
        ctx,
        invite.org_id,
        player.player_id,
        &member_name,
        rank,
        "invite",
    )
    .await;
    feedback(ctx, player.entity_id, &format!("You joined {org_name}.")).await;
    row.ok("joined");
    Ok(InviteAnswer::Joined {
        org_id: invite.org_id,
        rank,
    })
}

/// Take the invite from the responder's session in one lock hold, and tell
/// a foreign id (held by another session) from an unknown one for the log.
/// The session must still play `player` (a return to character select
/// clears the held invites anyway).
fn take(
    ctx: &OrgCtx<'_>,
    player: &OrgPlayer,
    request_id: i32,
    now: Instant,
) -> Result<PendingOrgInvite, TakeMiss> {
    let addr = ctx
        .entity_to_addr
        .lock()
        .ok()
        .and_then(|m| m.get(&player.entity_id).copied());
    let Ok(mut clients) = ctx.connected.lock() else {
        return Err(TakeMiss::Unknown);
    };
    let Some(mine) = addr
        .and_then(|a| clients.get_mut(&a))
        .filter(|c| c.active_player_id == Some(player.player_id))
    else {
        return Err(TakeMiss::Unknown);
    };
    let taken = mine.org_invites.take(player.player_id, request_id, now);
    for gone in mine.org_invites.drain_expired() {
        tracing::debug!(
            target: "org",
            event = "invite_expired",
            org_id = gone.org_id,
            request_id = gone.request_id,
            player_id = gone.inviter_player_id,
            target_player_id = gone.invitee_player_id,
            "organization invite expired unanswered"
        );
    }
    match taken {
        Err(TakeMiss::Unknown) if clients.values().any(|c| c.org_invites.holds(request_id)) => {
            Err(TakeMiss::Foreign)
        }
        other => other,
    }
}

/// The locked part of an accept: re-validate and add the member. Returns
/// the rank joined at, the organization's name and the member's name.
async fn join_locked(
    tx: &mut Transaction<'_, Postgres>,
    player: &OrgPlayer,
    invite: &PendingOrgInvite,
    row: &mut ActionRow,
) -> Result<(OrgRank, String, String), OrgReject> {
    let db = |row: &ActionRow, e: &dyn std::fmt::Display| db_failed(row, e);
    let header = lock_org(tx, invite.org_id)
        .await
        .map_err(|e| db(row, &e))?
        .ok_or(OrgReject::OrgGone)?;
    let inviter = member_access_locked(tx, invite.org_id, invite.inviter_player_id)
        .await
        .map_err(|e| db(row, &e))?
        .ok_or(OrgReject::InviterLeft)?;
    if !inviter.permissions().contains(OrgPermission::INVITE) {
        return Err(OrgReject::InviterMissingPermission);
    }
    // From the locked row, not the invite: the type never changes, but the
    // row is the authority (and keeps `add_member`'s rank check honest).
    let rank = OrgRank::entry_for(header.org_type);
    row.to_rank = Some(rank.as_u8());
    add_joined(tx, &inviter, invite.org_id, player.player_id, rank, row).await?;
    let name: String =
        sqlx::query_scalar("SELECT player_name FROM sgw_player WHERE player_id = $1")
            .bind(player.player_id)
            .fetch_one(&mut **tx)
            .await
            .map_err(|e| db(row, &e))?;
    Ok((rank, header.name, name))
}

/// `add_member` with its typed refusals mapped to outcome reasons, and the
/// DEBUG `member_joined` transition on success.
pub(super) async fn add_joined(
    tx: &mut Transaction<'_, Postgres>,
    actor: &OrgAccess,
    org_id: i32,
    player_id: i32,
    rank: OrgRank,
    row: &ActionRow,
) -> Result<(), OrgReject> {
    match add_member(tx, actor, org_id, player_id, rank).await {
        Ok(()) => {
            tracing::debug!(
                target: "org",
                event = "member_joined",
                via = row.action,
                account_id = row.account_id,
                player_id = row.player_id,
                target_player_id = player_id,
                org_id,
                org_type = actor.org_type().name(),
                rank = rank.as_u8(),
                "organization member joined"
            );
            Ok(())
        }
        Err(OrgStoreError::AlreadyInType) => Err(OrgReject::AlreadyInOrgType),
        Err(OrgStoreError::AlreadyMember) => Err(OrgReject::AlreadyMember),
        Err(OrgStoreError::NoSuchOrg) => Err(OrgReject::OrgGone),
        Err(e) => Err(db_failed(row, &e)),
    }
}

/// After a committed join: the new member's client gets the organization's
/// full state (if they are online now, re-resolved by character id), and
/// every other online member gets [37] with `aNewMember = 1`. `via` names
/// the path in the `org.state_push` and broadcast logs.
pub(super) async fn announce_join(
    ctx: &OrgCtx<'_>,
    org_id: i32,
    player_id: i32,
    member_name: &str,
    rank: OrgRank,
    via: &'static str,
) {
    let online = online_now(ctx, player_id);
    if let Some(m) = online {
        let joiner = OrgPlayer {
            account_id: m.account_id,
            player_id,
            entity_id: m.entity_id,
        };
        // A failed push is WARN `org.state_push_failed`; the membership is
        // committed and the next world entry restores it.
        let _ = push_org_state(ctx, org_id, &joiner, true).await;
    }
    let entity_id = online.map_or(0, |m| m.entity_id as i32);
    let args = build_on_member_joined_organization(member_name, entity_id, org_id, rank, true);
    let what = if via == "gm" {
        "member_joined_gm"
    } else {
        "member_joined"
    };
    broadcast_except(
        ctx,
        org_id,
        ON_MEMBER_JOINED_ORGANIZATION,
        &args,
        None,
        Some(player_id),
        what,
    )
    .await;
}
