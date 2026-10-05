//! `organizationKick` (0xD1) for a Team or Command id (ORG-07, CAT-M-05).
//!
//! Under ORG-LOCK (D-ORG04): the actor is a member whose rank holds
//! `Eject` (D-ORG09 (1)); the target is found by name among that
//! organization's own members (so an offline member can be kicked); nobody
//! kicks themself; and the actor's rank is strictly above the target's
//! (D-ORG09 (2)), so the leader is never kicked. After the commit:
//!
//! - the kicked player, if online now, gets `onOrganizationLeft` [36] with
//!   `Kicked`, a feedback line, and the cell gets `OrgMembershipEnded`
//!   (`Kicked`) so the Bank can close an open vault session;
//! - every remaining online member gets `onMemberLeftOrganization` [39]
//!   (`broadcast_to_org`), the actor included.
//!
//! The member-delete trigger's audit rows are exported after the commit.

use cimmeria_entity::organization::{OrgLeaveReason, OrgPermission};
use cimmeria_wire::cell::client_methods::organization::{
    build_on_member_left_organization, build_on_organization_left, ON_MEMBER_LEFT_ORGANIZATION,
    ON_ORGANIZATION_LEFT,
};
use sqlx::{Postgres, Transaction};

use super::answer::{db_failed, refusal_text, refuse};
use super::broadcast::broadcast_to_org;
use super::fanout::{feedback, membership_ended, send_to_player};
use super::log_names::{identity_of_player, label};
use super::targets::{member_by_name, online_now, MemberTarget};
use super::telemetry::{ActionRow, OrgReject};
use super::{OrgCtx, OrgPlayer};
use crate::base::organization::api::{lock_org, member_access_locked};
use crate::base::organization::audit::{export_committed, ExportSource};
use crate::base::organization::persistence::remove_member;

const KICK_SELF_TEXT: &str = "You cannot remove yourself; leave the organization instead.";

/// What the locked part decided, for the fanout after the commit.
struct Kicked {
    target: MemberTarget,
    org_name: String,
    tx_id: i64,
}

/// Handle 0xD1 for a Team or Command id. `player` is the session's
/// character, never the payload. Returns the kicked member's `player_id`.
#[tracing::instrument(
    name = "org.kick",
    level = "info",
    skip_all,
    fields(player_id = player.player_id, entity_id = player.entity_id, org_id)
)]
pub async fn handle_kick(
    ctx: &OrgCtx<'_>,
    player: &OrgPlayer,
    org_id: i32,
    target_name: &str,
) -> Result<i32, OrgReject> {
    let mut row = ActionRow {
        event: "org.kick",
        action: "kick",
        account_id: player.account_id,
        player_id: Some(player.player_id),
        entity_id: Some(player.entity_id),
        org_id: Some(org_id),
        ..ActionRow::default()
    };
    row.name_actor(ctx);
    let Some(pool) = ctx.db_pool.as_deref() else {
        let text = refusal_text(OrgReject::NoDb, KICK_SELF_TEXT, None);
        return refuse(ctx, &row, player.entity_id, OrgReject::NoDb, &text).await;
    };
    let decided = match pool.begin().await {
        Ok(mut tx) => {
            match kick_locked(ctx, &mut tx, player, org_id, target_name, &mut row).await {
                Ok(k) => match tx.commit().await {
                    Ok(()) => Ok(k),
                    Err(e) => Err(db_failed(&row, &e)),
                },
                // Dropping `tx` rolls back.
                Err(why) => Err(why),
            }
        }
        Err(e) => Err(db_failed(&row, &e)),
    };
    let k = match decided {
        Ok(k) => k,
        Err(why) => {
            let text = refusal_text(why, KICK_SELF_TEXT, None);
            return refuse(ctx, &row, player.entity_id, why, &text).await;
        }
    };

    tracing::debug!(
        target: "org",
        event = "member_left",
        reason = "kicked",
        account_id = player.account_id,
        account_name = row.account_name,
        player_id = player.player_id,
        player_name = row.player_name,
        target_account_id = row.target_account_id,
        target_account_name = row.target_account_name,
        target_player_id = k.target.player_id,
        target_player_name = row.target_player_name,
        org_id,
        org_name = k.org_name.as_str(),
        from_rank = k.target.rank.as_u8(),
        "organization member kicked"
    );
    if let Err(e) = export_committed(pool, k.tx_id, ExportSource::MemberRemoval).await {
        tracing::warn!(
            target: "org",
            event = "org_events_export",
            org_id,
            org_name = k.org_name.as_str(),
            reason = "db_error",
            error = %e,
            "organization audit rows not exported; the startup sweep will"
        );
    }

    // Re-resolved by character id now: the kicked player may have logged
    // off or travelled while the transaction ran.
    let kicked_online = online_now(ctx, k.target.player_id);
    if let Some(m) = kicked_online {
        let left = build_on_organization_left(OrgLeaveReason::Kicked, org_id);
        if let Err(reason) = send_to_player(ctx, m.entity_id, &[(ON_ORGANIZATION_LEFT, left)]).await
        {
            let who = identity_of_player(ctx, m.player_id);
            tracing::warn!(
                target: "org",
                event = "org.send_failed",
                what = "organization_left_kicked",
                org_id,
                org_name = k.org_name.as_str(),
                target_account_id = m.account_id,
                target_account_name = who.account_name,
                target_player_id = m.player_id,
                target_player_name = who.player_name,
                entity_id = m.entity_id,
                entity_name = who.player_name,
                reason,
                "onOrganizationLeft could not be sent to the kicked member"
            );
        }
        feedback(
            ctx,
            m.entity_id,
            &format!("You were removed from {}.", k.org_name),
        )
        .await;
        membership_ended(ctx, m, org_id, Some(&k.org_name), OrgLeaveReason::Kicked).await;
    }
    let member_id = kicked_online.map_or(0, |m| m.entity_id as i32);
    let args = build_on_member_left_organization(
        member_id,
        OrgLeaveReason::Kicked,
        org_id,
        &k.target.name,
    );
    broadcast_to_org(ctx, org_id, ON_MEMBER_LEFT_ORGANIZATION, &args, None).await;
    feedback(
        ctx,
        player.entity_id,
        &format!("You removed {} from {}.", k.target.name, k.org_name),
    )
    .await;
    row.ok("kicked");
    Ok(k.target.player_id)
}

/// The locked part: authorize, find the target and remove them, inside
/// `tx`.
async fn kick_locked(
    ctx: &OrgCtx<'_>,
    tx: &mut Transaction<'_, Postgres>,
    player: &OrgPlayer,
    org_id: i32,
    target_name: &str,
    row: &mut ActionRow,
) -> Result<Kicked, OrgReject> {
    let db = |row: &ActionRow, e: &dyn std::fmt::Display| db_failed(row, e);
    let header = lock_org(tx, org_id)
        .await
        .map_err(|e| db(row, &e))?
        .ok_or(OrgReject::NotMember)?;
    row.org_type = Some(header.org_type.name());
    row.org_name = label(&header.name);
    let access = member_access_locked(tx, org_id, player.player_id)
        .await
        .map_err(|e| db(row, &e))?
        .ok_or(OrgReject::NotMember)?;
    row.actor_rank = Some(access.rank().as_u8());
    if !access.permissions().contains(OrgPermission::EJECT) {
        return Err(OrgReject::MissingPermission);
    }
    let target = member_by_name(tx, org_id, target_name)
        .await
        .map_err(|e| db(row, &e))??;
    row.target_player_id = Some(target.player_id);
    row.target_account_id = u32::try_from(target.account_id).ok();
    row.target_player_name = label(&target.name);
    row.name_target(ctx);
    row.target_rank = Some(target.rank.as_u8());
    if target.player_id == player.player_id {
        return Err(OrgReject::SelfTarget);
    }
    if access.rank() <= target.rank {
        return Err(OrgReject::RankTooLow);
    }
    let removal = remove_member(tx, &access, org_id, target.player_id)
        .await
        .map_err(|e| db(row, &e))?;
    Ok(Kicked {
        target,
        org_name: header.name,
        tx_id: removal.tx_id,
    })
}
