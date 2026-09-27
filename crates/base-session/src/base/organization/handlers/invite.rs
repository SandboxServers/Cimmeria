//! `organizationInvite` (0xCF) and `organizationInviteByType` (0xD0, types
//! 1 and 2) for Teams and Commands (ORG-07, CAT-M-01, CAT-M-02).
//!
//! 1. The inviter's rate limit (their own session), before any database
//!    work.
//! 2. The invitee: online, not the inviter, not mid world entry, and not
//!    ignoring the inviter ([`online_target`]).
//! 3. Under ORG-LOCK (D-ORG04): the organization is locked, the inviter is a
//!    member whose rank holds `Invite`, and the invitee is in no
//!    organization of that type (D-ORG18). Invite-by-type only ever finds
//!    the inviter's existing organization of that type; it never creates
//!    one (CAT-M-02). The transaction writes nothing and is rolled back.
//! 4. The invite is recorded on the invitee's session (D-ORG06,
//!    [`crate::base::organization::invites`]) and `onOrganizationInvite`
//!    [34] goes to the invitee. The accept re-validates all of step 3.

use std::time::Instant;

use cimmeria_entity::organization::{OrgPermission, OrgType};
use cimmeria_wire::cell::client_methods::organization::{
    build_on_organization_invite, ON_ORGANIZATION_INVITE,
};
use sqlx::{Postgres, Transaction};

use super::answer::{db_failed, refusal_text, refuse};
use super::fanout::{feedback, send_to_player};
use super::targets::{actor_may_send, online_target, with_actor_session, OnlineTarget};
use super::telemetry::{ActionRow, OrgReject};
use super::{OrgCtx, OrgPlayer};
use crate::base::organization::api::{lock_org, member_access_locked};
use crate::base::organization::invites::IssueReject;
use crate::base::organization::persistence::OrgStoreError;

const INVITE_SELF_TEXT: &str = "You cannot invite yourself.";

/// Which organization an invite names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InviteInto {
    /// 0xCF: an organization id from the client (routing is not
    /// authorization: the membership check still runs).
    Org(i32),
    /// 0xD0: the inviter's own Team or Command. The caller has refused
    /// every other type byte.
    ByType(OrgType),
}

/// What the locked read found.
struct Checked {
    org_id: i32,
    org_type: OrgType,
    org_name: String,
}

/// Handle 0xCF or 0xD0 for a Team or Command. `player` is the session's
/// character, never the payload; `target_name` is the typed name. Returns
/// the request id the invitee must answer with.
#[tracing::instrument(
    name = "org.invite",
    level = "info",
    skip_all,
    fields(player_id = player.player_id, entity_id = player.entity_id)
)]
pub async fn handle_invite(
    ctx: &OrgCtx<'_>,
    player: &OrgPlayer,
    into: InviteInto,
    target_name: &str,
    now: Instant,
) -> Result<i32, OrgReject> {
    let (org_id, by_type) = match into {
        InviteInto::Org(id) => (Some(id), None),
        InviteInto::ByType(t) => (None, Some(t)),
    };
    let mut row = ActionRow {
        event: "org.invite",
        action: "invite",
        account_id: player.account_id,
        player_id: Some(player.player_id),
        entity_id: Some(player.entity_id),
        org_id,
        org_type: by_type.map(|t| t.name()),
        ..ActionRow::default()
    };
    let fail = |row: ActionRow, why: OrgReject, org_type: Option<OrgType>| async move {
        let text = refusal_text(why, INVITE_SELF_TEXT, org_type);
        refuse(ctx, &row, player.entity_id, why, &text).await
    };

    if !actor_may_send(ctx, player, now) {
        return fail(row, OrgReject::RateLimited, by_type).await;
    }
    let inviter_name = super::targets::actor_name(ctx, player).unwrap_or_default();
    let target = match online_target(ctx, player, &inviter_name, target_name) {
        Ok(t) => t,
        Err((why, t)) => {
            if let Some(t) = t {
                row.target_account_id = Some(t.account_id);
                row.target_player_id = Some(t.player_id);
            }
            return fail(row, why, by_type).await;
        }
    };
    row.target_account_id = Some(target.account_id);
    row.target_player_id = Some(target.player_id);

    let Some(pool) = ctx.db_pool.as_deref() else {
        return fail(row, OrgReject::NoDb, by_type).await;
    };
    let checked = match pool.begin().await {
        // Read-only: dropping `tx` rolls it back.
        Ok(mut tx) => check_locked(&mut tx, player, into, &target, &mut row).await,
        Err(e) => Err(db_failed(&row, &e)),
    };
    let checked = match checked {
        Ok(c) => c,
        Err(why) => {
            let org_type = by_type.or_else(|| row.org_type.and_then(type_from_name));
            return fail(row, why, org_type).await;
        }
    };

    // Record the invite on the invitee's session, then count the send on
    // the inviter's. Each takes and drops the map lock; nothing is held
    // across an `.await`.
    let issued = match ctx.connected.lock() {
        Err(_) => Err(OrgReject::TargetNotFound),
        Ok(mut clients) => {
            let session = clients.values_mut().find(|c| {
                c.active_player_id == Some(target.player_id)
                    && c.player_entity_id == Some(target.entity_id)
            });
            match session {
                None => Err(OrgReject::TargetNotFound),
                Some(c) => {
                    let issued = c
                        .org_invites
                        .issue(
                            target.player_id,
                            player.player_id,
                            &inviter_name,
                            checked.org_id,
                            checked.org_type,
                            now,
                        )
                        .map_err(|e| match e {
                            IssueReject::InviteLimit => OrgReject::InviteLimit,
                            IssueReject::IdsExhausted => OrgReject::IdsExhausted,
                        });
                    for gone in c.org_invites.drain_expired() {
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
                    issued
                }
            }
        }
    };
    let invite = match issued {
        Ok(i) => i,
        Err(why) => return fail(row, why, Some(checked.org_type)).await,
    };
    with_actor_session(ctx, player, |c| c.org_invites.record_sent(now));
    row.request_id = Some(invite.request_id);
    tracing::debug!(
        target: "org",
        event = "invite_created",
        account_id = player.account_id,
        player_id = player.player_id,
        target_account_id = target.account_id,
        target_player_id = target.player_id,
        org_id = checked.org_id,
        org_type = checked.org_type.name(),
        request_id = invite.request_id,
        expires_in_secs = crate::base::organization::invites::INVITE_TTL.as_secs(),
        "organization invite created"
    );

    let args = build_on_organization_invite(
        &inviter_name,
        checked.org_type,
        invite.request_id,
        &checked.org_name,
        false,
    );
    if let Err(reason) =
        send_to_player(ctx, target.entity_id, &[(ON_ORGANIZATION_INVITE, args)]).await
    {
        // The entry stays: it expires on its own, and a resend is the
        // inviter's call. The inviter still hears the invite was made.
        tracing::warn!(
            target: "org",
            event = "org.send_failed",
            what = "organization_invite",
            org_id = checked.org_id,
            account_id = player.account_id,
            player_id = player.player_id,
            target_account_id = target.account_id,
            target_player_id = target.player_id,
            entity_id = target.entity_id,
            reason,
            "onOrganizationInvite could not be sent to the invitee"
        );
    }
    feedback(
        ctx,
        player.entity_id,
        &format!("You invited {} to join {}.", target.name, checked.org_name),
    )
    .await;
    row.ok("invited");
    Ok(invite.request_id)
}

/// The locked part: find the organization, lock it, authorize the inviter
/// and check the invitee's standing, inside `tx`.
async fn check_locked(
    tx: &mut Transaction<'_, Postgres>,
    player: &OrgPlayer,
    into: InviteInto,
    target: &OnlineTarget,
    row: &mut ActionRow,
) -> Result<Checked, OrgReject> {
    let db = |row: &ActionRow, e: &dyn std::fmt::Display| db_failed(row, e);
    let org_id = match into {
        InviteInto::Org(id) => id,
        InviteInto::ByType(t) => {
            // A display read to find the id; the lock below re-checks
            // membership. Never creates (CAT-M-02).
            let found: Option<i32> = sqlx::query_scalar(
                "SELECT org_id FROM sgw_organization_members \
                 WHERE player_id = $1 AND org_type = $2",
            )
            .bind(player.player_id)
            .bind(i16::from(t.as_u8()))
            .fetch_optional(&mut **tx)
            .await
            .map_err(|e| db(row, &e))?;
            found.ok_or(OrgReject::NotMember)?
        }
    };
    row.org_id = Some(org_id);
    let header = lock_org(tx, org_id)
        .await
        .map_err(|e| db(row, &e))?
        .ok_or(OrgReject::NotMember)?;
    row.org_type = Some(header.org_type.name());
    let access = member_access_locked(tx, org_id, player.player_id)
        .await
        .map_err(|e| db(row, &e))?
        .ok_or(OrgReject::NotMember)?;
    row.actor_rank = Some(access.rank().as_u8());
    if !access.permissions().contains(OrgPermission::INVITE) {
        return Err(OrgReject::MissingPermission);
    }
    let held: Option<i32> = sqlx::query_scalar(
        "SELECT org_id FROM sgw_organization_members WHERE player_id = $1 AND org_type = $2",
    )
    .bind(target.player_id)
    .bind(i16::from(header.org_type.as_u8()))
    .fetch_optional(&mut **tx)
    .await
    .map_err(|e| db(row, &OrgStoreError::from(e)))?;
    match held {
        Some(id) if id == org_id => Err(OrgReject::AlreadyMember),
        Some(_) => Err(OrgReject::AlreadyInOrgType),
        None => Ok(Checked {
            org_id,
            org_type: header.org_type,
            org_name: header.name,
        }),
    }
}

/// The `OrgType` whose [`OrgType::name`] is `name`.
fn type_from_name(name: &str) -> Option<OrgType> {
    OrgType::ALL.into_iter().find(|t| t.name() == name)
}
