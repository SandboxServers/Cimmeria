//! Disband: the `onOrganizationLeft(Disbanded)` fanout both disband paths
//! share, and `.org_disband <orgId>` for GMs.
//!
//! Every voluntary disband checks the vault under ORG-LOCK and refuses while
//! it holds anything (D-ORG20); `persistence::disband` does the check, so
//! the GM path honours it too. A GM may disband a memberless organization
//! (D-ORG20's recovery case) once its vault is empty.

use cimmeria_entity::organization::OrgLeaveReason;
use cimmeria_wire::cell::client_methods::organization::{
    build_on_organization_left, ON_ORGANIZATION_LEFT,
};

use super::fanout::{feedback, membership_ended, online_members, send_to_members};
use super::telemetry::{OrgReject, Row};
use super::OrgCtx;
use crate::base::organization::api::{OrgAccess, SystemActor};
use crate::base::organization::persistence::{disband, OrgStoreError};

/// Minimum `access_level` for `.org_disband`: GameMaster (D-ORG13).
pub const GM_ACCESS_LEVEL: u32 = 2;

/// Tell the online ones among `members` (the organization's members, the
/// actor excluded where they were told otherwise) that it was disbanded,
/// and log the DEBUG `disbanded` transition with `reason`.
pub(super) async fn fan_out_disband(
    ctx: &OrgCtx<'_>,
    org_id: i32,
    members: &[i32],
    reason: &'static str,
) -> usize {
    let recipients = online_members(ctx, members);
    let args = build_on_organization_left(OrgLeaveReason::Disbanded, org_id);
    let sent = send_to_members(
        ctx,
        org_id,
        &recipients,
        &[(ON_ORGANIZATION_LEFT, args)],
        "organization_left_disbanded",
    )
    .await;
    for &m in &recipients {
        membership_ended(ctx, m, org_id, OrgLeaveReason::Disbanded).await;
    }
    tracing::debug!(
        target: "org",
        event = "disbanded",
        reason,
        org_id,
        members = members.len(),
        recipients = sent,
        "organization disbanded"
    );
    sent
}

/// The GM who typed `.org_disband`, as the cell forwarded them.
#[derive(Debug, Clone, Copy)]
pub struct GmCaller {
    pub entity_id: u32,
    pub player_id: i32,
}

/// `.org_disband <orgId>`. The cell forwards only a GM's line, but the base
/// re-reads the session's access level itself: the message carries no
/// privilege bit (D-ORG13). Answers the GM on the feedback channel and ends
/// in one INFO `org.disband` row (`reason` = `not_gm` \| `no_such_org` \|
/// `vault_not_empty` \| `no_db` \| `db_error`); `OrgAccess::system` adds the
/// `org.gm_action` audit row once the organization is locked.
#[tracing::instrument(
    name = "org.disband",
    level = "info",
    skip_all,
    fields(entity_id = gm.entity_id, org_id)
)]
pub async fn gm_disband(ctx: &OrgCtx<'_>, gm: GmCaller, org_id: i32) -> Result<usize, OrgReject> {
    let (account_id, access_level) = session_of(ctx, gm);
    let mut row = Row {
        event: "org.disband",
        action: "disband",
        account_id,
        player_id: Some(gm.player_id),
        entity_id: Some(gm.entity_id),
        org_id,
        org_type: None,
    };
    let refuse = |row: Row, why: OrgReject, text: String| async move {
        row.rejected(why);
        feedback(ctx, gm.entity_id, &text).await;
        Err(why)
    };
    if access_level < GM_ACCESS_LEVEL {
        return refuse(
            row,
            OrgReject::NotGm,
            "org_disband: refused, GameMaster access is required.".into(),
        )
        .await;
    }
    let Some(pool) = ctx.db_pool.as_deref() else {
        return refuse(
            row,
            OrgReject::NoDb,
            "org_disband: failed, no database.".into(),
        )
        .await;
    };
    let db_failed = |e: &dyn std::fmt::Display| {
        tracing::warn!(
            target: "org",
            event = "org.disband_failed",
            account_id,
            player_id = gm.player_id,
            org_id,
            reason = "db_error",
            error = %e,
            "organization disband failed in the database"
        );
    };
    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            db_failed(&e);
            return refuse(
                row,
                OrgReject::DbError,
                "org_disband: failed, database error.".into(),
            )
            .await;
        }
    };
    let actor = SystemActor::Gm {
        account_id: account_id.and_then(|a| i32::try_from(a).ok()),
        player_id: Some(gm.player_id),
        command: "org_disband",
    };
    let access = match OrgAccess::system(&mut tx, org_id, actor).await {
        Ok(Some(a)) => a,
        Ok(None) => {
            return refuse(
                row,
                OrgReject::NoSuchOrg,
                format!("org_disband: no organization has id {org_id}."),
            )
            .await;
        }
        Err(e) => {
            db_failed(&e);
            return refuse(
                row,
                OrgReject::DbError,
                "org_disband: failed, database error.".into(),
            )
            .await;
        }
    };
    row.org_type = Some(access.org_type().name());
    let name: String = sqlx::query_scalar("SELECT name FROM sgw_organizations WHERE org_id = $1")
        .bind(org_id)
        .fetch_one(&mut *tx)
        .await
        .unwrap_or_default();
    let members = match disband(&mut tx, &access, org_id).await {
        Ok(m) => m,
        Err(OrgStoreError::VaultNotEmpty) => {
            return refuse(
                row,
                OrgReject::VaultNotEmpty,
                format!("org_disband: refused, the vault of organization {org_id} is not empty."),
            )
            .await;
        }
        Err(e) => {
            db_failed(&e);
            return refuse(
                row,
                OrgReject::DbError,
                "org_disband: failed, database error.".into(),
            )
            .await;
        }
    };
    if let Err(e) = tx.commit().await {
        db_failed(&e);
        return refuse(
            row,
            OrgReject::DbError,
            "org_disband: failed, database error.".into(),
        )
        .await;
    }
    let told = fan_out_disband(ctx, org_id, &members, "gm").await;
    feedback(
        ctx,
        gm.entity_id,
        &format!(
            "org_disband: disbanded {} '{}' (id {org_id}): {} member(s), {told} online told.",
            access.org_type().name(),
            name,
            members.len()
        ),
    )
    .await;
    row.ok("disbanded");
    Ok(members.len())
}

/// The GM session's `(account_id, access_level)`; a missing session is
/// level 0, never privileged. The session must still play the forwarded
/// character on the forwarded entity (ORG-07 review): an entity id alone
/// may have been recycled to another session since the cell sent it.
fn session_of(ctx: &OrgCtx<'_>, gm: GmCaller) -> (Option<u32>, u32) {
    let addr = ctx
        .entity_to_addr
        .lock()
        .ok()
        .and_then(|m| m.get(&gm.entity_id).copied());
    let Ok(clients) = ctx.connected.lock() else {
        return (None, 0);
    };
    addr.and_then(|a| clients.get(&a))
        .filter(|c| {
            c.player_entity_id == Some(gm.entity_id) && c.active_player_id == Some(gm.player_id)
        })
        .map_or((None, 0), |c| (Some(c.account_id), c.access_level))
}
