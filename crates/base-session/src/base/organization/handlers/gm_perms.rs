//! GM `.org_set_perms <orgId> <rank> <mask>` (ORG-10, D-ORG13).
//!
//! The GM edit goes through the same two functions a member's rank-editor
//! press (CM 16, ORG-08) does, so the rules cannot drift apart:
//!
//! - `OrgPermission::apply_edit` (D-ORG22) works out the stored mask: only
//!   the bits the type's editor shows (`editable_for`: 12 for a Team, 14 for
//!   a Command) take the GM's value; every other bit keeps what is stored.
//!   That is the D-ORG09 (6) clamp. A GM acts as a system actor holding
//!   every bit, so the "grant only what you hold" half never refuses.
//! - `persistence::set_rank_permissions` (ORG-02) writes it under ORG-LOCK
//!   and refuses the `Leader` row and a rank the type does not use.
//!
//! The `Leader` row and an unused rank are refused before the write too, so
//! the GM reads why. An edit the clamp turns into no change is refused as
//! `permissions_unchanged`, telling the GM which bits were ignored.
//!
//! After the commit every online member gets the new rank table
//! (`onOrganizationRankUpdate` [49], through `broadcast_to_org`).
//!
//! Ends in one INFO `org.gm_action` row, `action = gm_org_set_perms`, with
//! `rank` and the result; the change itself is the DEBUG
//! `permissions_changed` transition with `from_mask`, `to_mask`,
//! `wire_mask` and `ignored_bits`.

use cimmeria_entity::organization::{OrgPermission, OrgRank};
use cimmeria_wire::cell::client_methods::organization::{
    build_on_organization_rank_update, ON_ORGANIZATION_RANK_UPDATE,
};
use sqlx::{Postgres, Transaction};

use super::answer::{db_failed, refuse};
use super::broadcast::broadcast_to_org;
use super::disband::{GmCaller, GM_ACCESS_LEVEL};
use super::fanout::feedback;
use super::gm::{gm_actor, gm_session};
use super::telemetry::{ActionRow, OrgReject, GM_ACTION_EVENT};
use super::{OrgCtx, OrgPlayer};
use crate::base::organization::api::{permissions_from_db, OrgAccess};
use crate::base::organization::persistence::{load_ranks, set_rank_permissions, OrgStoreError};

/// What one accepted edit changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PermsEdit {
    pub from: OrgPermission,
    pub to: OrgPermission,
    /// Bits of the GM's mask the type's editor does not show, which the
    /// clamp left at their stored value.
    pub ignored: OrgPermission,
}

/// `.org_set_perms <orgId> <rank> <mask>`.
#[tracing::instrument(
    name = "org.gm_set_perms",
    level = "info",
    skip_all,
    fields(entity_id = gm.entity_id, org_id, rank)
)]
pub async fn gm_set_perms(
    ctx: &OrgCtx<'_>,
    gm: GmCaller,
    org_id: i32,
    rank: u8,
    mask: u32,
) -> Result<PermsEdit, OrgReject> {
    let mut row = ActionRow {
        event: GM_ACTION_EVENT,
        action: "gm_org_set_perms",
        player_id: Some(gm.player_id),
        entity_id: Some(gm.entity_id),
        org_id: Some(org_id),
        rank: Some(rank),
        ..ActionRow::default()
    };
    let fail = |row: ActionRow, why: OrgReject, text: String| async move {
        refuse(ctx, &row, gm.entity_id, why, &text).await
    };
    let Some((caller, level)) = gm_session(ctx, gm) else {
        return fail(row, OrgReject::NotGm, refused_text(OrgReject::NotGm)).await;
    };
    row.account_id = caller.account_id;
    if level < GM_ACCESS_LEVEL {
        return fail(row, OrgReject::NotGm, refused_text(OrgReject::NotGm)).await;
    }
    let Some(pool) = ctx.db_pool.as_deref() else {
        return fail(row, OrgReject::NoDb, refused_text(OrgReject::NoDb)).await;
    };
    // The mask bits the clamp dropped, known once the type is read.
    let mut ignored = OrgPermission::NONE;
    let decided = match pool.begin().await {
        Ok(mut tx) => {
            match edit_locked(&mut tx, &caller, org_id, rank, mask, &mut row, &mut ignored).await {
                Ok(edit) => match tx.commit().await {
                    Ok(()) => Ok(edit),
                    Err(e) => Err(db_failed(&row, &e)),
                },
                Err(why) => Err(why),
            }
        }
        Err(e) => Err(db_failed(&row, &e)),
    };
    let edit = match decided {
        Ok(edit) => edit,
        Err(OrgReject::PermissionsUnchanged) => {
            let text = format!(
                "org_set_perms: rank {rank} of organization {org_id} already has that mask \
                 (bits {:#09x} are not editable for its type and were ignored).",
                ignored.bits()
            );
            return fail(row, OrgReject::PermissionsUnchanged, text).await;
        }
        Err(why) => return fail(row, why, refused_text(why)).await,
    };
    tracing::debug!(
        target: "org",
        event = "permissions_changed",
        via = "gm",
        account_id = caller.account_id,
        player_id = caller.player_id,
        org_id,
        org_type = row.org_type,
        rank,
        from_mask = edit.from.bits(),
        to_mask = edit.to.bits(),
        wire_mask = mask,
        ignored_bits = edit.ignored.bits(),
        "organization rank permissions changed"
    );
    // The client's rank table is the whole list, so send all of it.
    match load_ranks(pool, org_id).await {
        Ok(ranks) => {
            let masks: Vec<(OrgRank, OrgPermission)> =
                ranks.iter().map(|r| (r.rank, r.permissions)).collect();
            let args = build_on_organization_rank_update(org_id, &masks);
            broadcast_to_org(ctx, org_id, ON_ORGANIZATION_RANK_UPDATE, &args, None).await;
        }
        Err(e) => tracing::warn!(
            target: "org",
            event = "org.broadcast_failed",
            what = "rank_update",
            org_id,
            reason = e.reason(),
            error = %e,
            "organization rank table not re-sent: the ranks could not be read"
        ),
    }
    let mut text = format!(
        "org_set_perms: rank {rank} of organization {org_id} is now {:#09x}, was {:#09x}.",
        edit.to.bits(),
        edit.from.bits()
    );
    if !edit.ignored.is_empty() {
        text.push_str(&format!(
            " Bits {:#09x} are not editable for its type and were ignored.",
            edit.ignored.bits()
        ));
    }
    feedback(ctx, gm.entity_id, &text).await;
    row.ok("permissions_changed");
    Ok(edit)
}

async fn edit_locked(
    tx: &mut Transaction<'_, Postgres>,
    gm: &OrgPlayer,
    org_id: i32,
    rank: u8,
    mask: u32,
    row: &mut ActionRow,
    ignored: &mut OrgPermission,
) -> Result<PermsEdit, OrgReject> {
    let access = OrgAccess::system(tx, org_id, gm_actor(gm, "org_set_perms"))
        .await
        .map_err(|e| db_failed(row, &e))?
        .ok_or(OrgReject::NoSuchOrg)?;
    let org_type = access.org_type();
    row.org_type = Some(org_type.name());
    let rank = OrgRank::try_from(rank)
        .ok()
        .filter(|r| r.is_valid_for(org_type))
        .ok_or(OrgReject::RankNotInType)?;
    if rank == OrgRank::LEADER {
        return Err(OrgReject::LeaderRowPinned);
    }
    let old: i32 = sqlx::query_scalar(
        "SELECT permissions FROM sgw_organization_ranks WHERE org_id = $1 AND rank = $2",
    )
    .bind(org_id)
    .bind(i16::from(rank.as_u8()))
    .fetch_optional(&mut **tx)
    .await
    .map_err(|e| db_failed(row, &e))?
    .ok_or(OrgReject::RankNotInType)?;
    let old = permissions_from_db(old);
    let wire = OrgPermission::from_bits_truncate(mask);
    let editable = OrgPermission::editable_for(org_type);
    *ignored = OrgPermission::from_bits_truncate(wire.bits() & !editable.bits());
    // A system actor holds every bit, so this is the clamp alone.
    let to = OrgPermission::apply_edit(old, wire, org_type, access.permissions())
        .map_err(|_| OrgReject::MissingPermission)?;
    if to == old {
        return Err(OrgReject::PermissionsUnchanged);
    }
    match set_rank_permissions(tx, &access, org_id, rank, to).await {
        Ok(from) => Ok(PermsEdit {
            from,
            to,
            ignored: *ignored,
        }),
        Err(OrgStoreError::LeaderPinned) => Err(OrgReject::LeaderRowPinned),
        Err(OrgStoreError::RankNotInType(_)) => Err(OrgReject::RankNotInType),
        Err(e) => Err(db_failed(row, &e)),
    }
}

fn refused_text(why: OrgReject) -> String {
    match why {
        OrgReject::NotGm => "org_set_perms: refused, GameMaster access is required.".into(),
        OrgReject::NoSuchOrg => "org_set_perms: no organization has that id.".into(),
        OrgReject::RankNotInType => {
            "org_set_perms: that rank does not exist in this organization's type.".into()
        }
        OrgReject::LeaderRowPinned => {
            "org_set_perms: refused, the Leader rank always holds every permission.".into()
        }
        OrgReject::NoDb => "org_set_perms: failed, no database.".into(),
        other => format!("org_set_perms: refused ({}).", other.reason()),
    }
}
