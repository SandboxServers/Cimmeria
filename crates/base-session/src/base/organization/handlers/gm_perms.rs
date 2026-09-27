//! GM `.org_set_perms <orgId> <rank> <mask>` (ORG-10, D-ORG13).
//!
//! The GM edit is a member's rank-editor press (CM 16, ORG-08) made by a
//! system actor: the same `rank_editor::rank_permissions_locked` and
//! `announce_perm_edit`, under the same per-organization order guard, so
//! the rules and the fanout cannot drift apart:
//!
//! - A rank the type does not use and the `Leader` row are refused.
//! - `OrgPermission::apply_edit` (D-ORG22) works out the stored mask: only
//!   the bits the type's editor shows (`editable_for`: 12 for a Team, 14 for
//!   a Command) take the GM's value; every other bit keeps what is stored.
//!   That is the D-ORG09 (6) clamp. A GM acts as a system actor holding
//!   every bit and the `Leader` rank, so the authority checks and the
//!   "grant only what you hold" half never refuse.
//!
//! An edit the clamp turns into no change is refused as
//! `permissions_unchanged`, telling the GM which bits were ignored.
//!
//! After the commit every online member gets the new rank table
//! (`onOrganizationRankUpdate` [49]), and when the edit moved
//! `OfficerNotes` the members of that rank get the officer-note sync [47]
//! (`officer_notes`).
//!
//! Ends in one INFO `org.gm_action` row, `action = gm_org_set_perms`, with
//! `rank` and the result; the change itself is the DEBUG
//! `permissions_changed` transition with `from_mask`, `to_mask`,
//! `wire_mask` and `ignored_bits`.

use cimmeria_entity::organization::OrgPermission;
use sqlx::{Postgres, Transaction};

use super::answer::{db_failed, refuse};
use super::disband::{GmCaller, GM_ACCESS_LEVEL};
use super::edit_row::EditRow;
use super::fanout::feedback;
use super::gm::{gm_actor, gm_session};
use super::order::org_order_guard;
use super::rank_editor::{announce_perm_edit, rank_permissions_locked, PermEdit};
use super::telemetry::{ActionRow, OrgReject, GM_ACTION_EVENT};
use super::{OrgCtx, OrgPlayer};
use crate::base::organization::api::OrgAccess;

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
    // Held until the last send (ORG-08): the [49] and any officer-note sync
    // must not cross another edit's fanout.
    let _order = org_order_guard(org_id).await;
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
    let perm_edit = match decided {
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
    let edit = PermsEdit {
        from: perm_edit.from,
        to: perm_edit.to,
        ignored,
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
    // The whole rank table as read under the lock, then the officer-note
    // sync if the edit moved `OfficerNotes`.
    announce_perm_edit(ctx, &perm_edit).await;
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
) -> Result<PermEdit, OrgReject> {
    let access = OrgAccess::system(tx, org_id, gm_actor(gm, "org_set_perms"))
        .await
        .map_err(|e| db_failed(row, &e))?
        .ok_or(OrgReject::NoSuchOrg)?;
    let org_type = access.org_type();
    row.org_type = Some(org_type.name());
    let wire = OrgPermission::from_bits_truncate(mask);
    let editable = OrgPermission::editable_for(org_type);
    *ignored = OrgPermission::from_bits_truncate(wire.bits() & !editable.bits());
    // The one permission-edit path (ORG-08). Its row only carries the masks
    // for a database-failure WARN; the GM's outcome row is `row`.
    let mut edit_row = EditRow {
        event: GM_ACTION_EVENT,
        action: "gm_org_set_perms",
        account_id: gm.account_id,
        player_id: Some(gm.player_id),
        entity_id: Some(gm.entity_id),
        org_id: Some(org_id),
        org_type: Some(org_type.name()),
        rank: Some(i32::from(rank)),
        wire_mask: Some(wire.to_wire()),
        ..EditRow::default()
    };
    let edit = rank_permissions_locked(tx, &access, i32::from(rank), wire.to_wire(), &mut edit_row)
        .await?;
    if !edit.changed() {
        return Err(OrgReject::PermissionsUnchanged);
    }
    Ok(edit)
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
