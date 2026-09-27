//! `organizationLeave` (CM 9) for a Team or Command, forwarded by the cell
//! (D-ORG05).
//!
//! Under ORG-LOCK (D-ORG04): lock the organization, read the caller's
//! access, count the members, then:
//!
//! - not a member: refused (`not_member`);
//! - the leader while others remain: refused (`leader_cannot_leave`,
//!   D-ORG12; there is no leader transfer);
//! - the last member: the organization is disbanded, but only if the vault
//!   is empty (`vault_not_empty`, D-ORG20, checked by
//!   `persistence::disband` through `api::org_vault_is_empty`);
//! - anyone else: the member row goes.
//!
//! After the commit the leaver gets `onOrganizationLeft` [36] with
//! `Requested` and the other online members `onMemberLeftOrganization` [39].
//! A refusal re-sends the organization's true state and a feedback line, so
//! a client that hid the organization on the press shows it again.

use cimmeria_entity::organization::{OrgLeaveReason, OrgRank};
use cimmeria_wire::cell::client_methods::organization::{
    build_on_member_left_organization, build_on_organization_left, ON_MEMBER_LEFT_ORGANIZATION,
    ON_ORGANIZATION_LEFT,
};
use sqlx::{Postgres, Transaction};

use super::disband::fan_out_disband;
use super::fanout::{
    feedback, membership_ended, online_members, send_to_members, send_to_player, OnlineMember,
};
use super::push::push_org_state;
use super::telemetry::{OrgReject, Row};
use super::{OrgCtx, OrgPlayer};
use crate::base::organization::api::member_access_locked;
use crate::base::organization::audit::{export_committed, ExportSource};
use crate::base::organization::persistence::{
    disband, load_roster, remove_member, OrgStoreError, RosterMember,
};

pub const NOT_MEMBER_TEXT: &str = "You are not a member of that organization.";
pub const LEADER_CANNOT_LEAVE_TEXT: &str =
    "You lead this organization and other members remain, so you cannot leave it.";
pub const VAULT_NOT_EMPTY_TEXT: &str =
    "The organization's vault is not empty, so it cannot be disbanded.";
pub const ORG_UNAVAILABLE_TEXT: &str =
    "Organizations are unavailable right now. Please try again later.";

/// What a successful leave did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeaveOutcome {
    /// The member row went; the organization stays.
    Left,
    /// The leaver was the last member, so the organization was disbanded.
    Disbanded,
}

/// What the locked part of the leave decided, for the fanout after commit.
struct Decided {
    outcome: LeaveOutcome,
    org_type: &'static str,
    /// The roster before the leave, the leaver included.
    roster: Vec<RosterMember>,
    from_rank: OrgRank,
    /// `remove_member`'s transaction, for the audit export.
    tx_id: Option<i64>,
}

/// Handle CM 9 for a Team or Command id. `player` is the resolved session
/// (`resolve_actor`), never the payload.
#[tracing::instrument(
    name = "org.leave",
    level = "info",
    skip_all,
    fields(player_id = player.player_id, entity_id = player.entity_id, org_id)
)]
pub async fn handle_leave(
    ctx: &OrgCtx<'_>,
    player: &OrgPlayer,
    org_id: i32,
) -> Result<LeaveOutcome, OrgReject> {
    let mut row = Row {
        event: "org.leave",
        action: "leave",
        account_id: player.account_id,
        player_id: Some(player.player_id),
        entity_id: Some(player.entity_id),
        org_id,
        org_type: None,
    };
    let Some(pool) = ctx.db_pool.as_deref() else {
        return refuse(ctx, row, player, OrgReject::NoDb).await;
    };
    let decided = match pool.begin().await {
        Ok(mut tx) => match decide(&mut tx, player, org_id).await {
            Ok(d) => match tx.commit().await {
                Ok(()) => Ok(d),
                Err(e) => Err(db_error(e.into(), player, org_id)),
            },
            // Dropping `tx` rolls back.
            Err(why) => Err(why),
        },
        Err(e) => Err(db_error(e.into(), player, org_id)),
    };
    let d = match decided {
        Ok(d) => d,
        Err((why, org_type)) => {
            row.org_type = org_type;
            return refuse(ctx, row, player, why).await;
        }
    };
    row.org_type = Some(d.org_type);

    tracing::debug!(
        target: "org",
        event = "member_left",
        reason = "requested",
        account_id = player.account_id,
        player_id = player.player_id,
        org_id,
        from_rank = d.from_rank.as_u8(),
        "organization member left"
    );
    if let Some(tx_id) = d.tx_id {
        // The trigger's audit rows (none on an ordinary leave, but a leave
        // that healed a leaderless organization writes one).
        if let Err(e) = export_committed(pool, tx_id, ExportSource::MemberRemoval).await {
            tracing::warn!(
                target: "org",
                event = "org_events_export",
                org_id,
                reason = "db_error",
                error = %e,
                "organization audit rows not exported; the startup sweep will"
            );
        }
    }

    let leaver = d.roster.iter().find(|m| m.player_id == player.player_id);
    let left = build_on_organization_left(OrgLeaveReason::Requested, org_id);
    if let Err(reason) =
        send_to_player(ctx, player.entity_id, &[(ON_ORGANIZATION_LEFT, left)]).await
    {
        tracing::warn!(
            target: "org",
            event = "org.send_failed",
            what = "organization_left",
            org_id,
            account_id = player.account_id,
            player_id = player.player_id,
            entity_id = player.entity_id,
            reason,
            "onOrganizationLeft could not be sent to the leaver"
        );
    }
    let me = OnlineMember {
        player_id: player.player_id,
        entity_id: player.entity_id,
        account_id: player.account_id.unwrap_or_default(),
    };
    membership_ended(ctx, me, org_id, OrgLeaveReason::Requested).await;
    match d.outcome {
        LeaveOutcome::Left => {
            let others: Vec<i32> = d
                .roster
                .iter()
                .map(|m| m.player_id)
                .filter(|&p| p != player.player_id)
                .collect();
            let recipients = online_members(ctx, &others);
            let name = leaver.map_or("", |m| m.name.as_str());
            let args = build_on_member_left_organization(
                player.entity_id as i32,
                OrgLeaveReason::Requested,
                org_id,
                name,
            );
            send_to_members(
                ctx,
                org_id,
                &recipients,
                &[(ON_MEMBER_LEFT_ORGANIZATION, args)],
                "member_left",
            )
            .await;
            row.ok("left");
        }
        LeaveOutcome::Disbanded => {
            let others: Vec<i32> = d
                .roster
                .iter()
                .map(|m| m.player_id)
                .filter(|&p| p != player.player_id)
                .collect();
            fan_out_disband(ctx, org_id, &others, "last_member_left").await;
            feedback(
                ctx,
                player.entity_id,
                "You were the last member, so the organization has been disbanded.",
            )
            .await;
            row.ok("disbanded");
        }
    }
    Ok(d.outcome)
}

type Refusal = (OrgReject, Option<&'static str>);

/// The locked part: authorize and write, inside `tx`.
async fn decide(
    tx: &mut Transaction<'_, Postgres>,
    player: &OrgPlayer,
    org_id: i32,
) -> Result<Decided, Refusal> {
    let db = |e: OrgStoreError| db_error(e, player, org_id);
    let access = member_access_locked(tx, org_id, player.player_id)
        .await
        .map_err(|e| db(e.into()))?
        .ok_or((OrgReject::NotMember, None))?;
    let org_type = access.org_type().name();
    let roster = load_roster(&mut **tx, org_id).await.map_err(db)?;
    let from_rank = access.rank();
    if roster.len() > 1 && from_rank == OrgRank::LEADER {
        return Err((OrgReject::LeaderCannotLeave, Some(org_type)));
    }
    if roster.len() <= 1 {
        return match disband(tx, &access, org_id).await {
            Ok(_) => Ok(Decided {
                outcome: LeaveOutcome::Disbanded,
                org_type,
                roster,
                from_rank,
                tx_id: None,
            }),
            Err(OrgStoreError::VaultNotEmpty) => Err((OrgReject::VaultNotEmpty, Some(org_type))),
            Err(e) => Err((db(e).0, Some(org_type))),
        };
    }
    let removal = remove_member(tx, &access, org_id, player.player_id)
        .await
        .map_err(|e| (db(e).0, Some(org_type)))?;
    Ok(Decided {
        outcome: LeaveOutcome::Left,
        org_type,
        roster,
        from_rank,
        tx_id: Some(removal.tx_id),
    })
}

/// A database failure: WARN with the error, then the `db_error` refusal.
/// Every other `OrgStoreError` here is a state the checks above rule out
/// under the lock, so it is reported the same way.
fn db_error(e: OrgStoreError, player: &OrgPlayer, org_id: i32) -> Refusal {
    tracing::warn!(
        target: "org",
        event = "org.leave_failed",
        account_id = player.account_id,
        player_id = player.player_id,
        org_id,
        reason = e.reason(),
        error = %e,
        "organization leave failed in the database"
    );
    (OrgReject::DbError, None)
}

/// Log the refusal, send the feedback line and, where the player is still a
/// member, re-send the organization's true state.
async fn refuse(
    ctx: &OrgCtx<'_>,
    row: Row,
    player: &OrgPlayer,
    why: OrgReject,
) -> Result<LeaveOutcome, OrgReject> {
    row.rejected(why);
    let text = match why {
        OrgReject::NotMember => NOT_MEMBER_TEXT,
        OrgReject::LeaderCannotLeave => LEADER_CANNOT_LEAVE_TEXT,
        OrgReject::VaultNotEmpty => VAULT_NOT_EMPTY_TEXT,
        _ => ORG_UNAVAILABLE_TEXT,
    };
    if matches!(why, OrgReject::LeaderCannotLeave | OrgReject::VaultNotEmpty) {
        // Still a member: restore anything the client hid on the press.
        let _ = push_org_state(ctx, row.org_id, player, false).await;
    }
    feedback(ctx, player.entity_id, text).await;
    Err(why)
}
