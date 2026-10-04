//! The rank editor: `organizationSetRankPermissions` (CM 16) and
//! `organizationSetRankName` (CM 17) for a Team or Command id (ORG-08,
//! CAT-M-07, CAT-M-08).
//!
//! Under ORG-LOCK (D-ORG04) and the organization's order guard, in this
//! order (every check runs under the one lock; the order only picks the
//! reported reason):
//!
//! 1. the actor is a member (`not_member`);
//! 2. the rank is one the type uses (`rank_not_in_type`, D-ORG09 (5));
//! 3. CM 16 only: the `Leader` row is refused (`leader_row_pinned`, D-ORG08);
//! 4. the actor holds `AlterPerms` (CM 16) or `RankNames` (CM 17)
//!    (`missing_permission`, D-ORG09 (1));
//! 5. not the actor's own rank (`own_rank`, D-ORG09 (3)), and strictly
//!    below it (`rank_too_low`, D-ORG09 (2)), so an editor can neither
//!    widen its own rank nor strip the ranks above it;
//! 6. CM 16: the new mask is `OrgPermission::apply_edit` of the stored one
//!    (D-ORG22: only the type's editable bits move, and every bit that
//!    moves must be one the actor holds, D-ORG09 (6);
//!    `changes_unheld_bits`). CM 17: the name passed D-ORG10 / D-ORG23
//!    before the lock (trimmed, whitespace collapsed, 1-32 units).
//!
//! An edit that changes nothing is `ok` / `unchanged`: no write, no fanout,
//! and the actor still gets the line. Otherwise, after the commit: [49]
//! with the whole rank table, or [50] with every custom rank name, both as
//! read under the lock, to every online member; and, when a CM 16 moved
//! `OfficerNotes`, the officer-note sync for that rank's members
//! ([`super::officer_notes`]).
//!
//! [`rank_permissions_locked`] is the one permission-edit path: a GM
//! command (ORG-10 `.org_set_perms`) calls it with an `OrgAccess::system`
//! (Leader rank, every bit), which passes 4-6's authority checks but still
//! meets the rank-in-type rule, the `Leader` pin and the editable-bit clamp.

use cimmeria_entity::organization::{org_text, OrgPermission, OrgRank, PermEditReject, TextField};
use cimmeria_wire::cell::client_methods::organization::{
    build_on_organization_rank_name_update, build_on_organization_rank_update,
    ON_ORGANIZATION_RANK_NAME_UPDATE, ON_ORGANIZATION_RANK_UPDATE,
};
use sqlx::{Postgres, Transaction};

use super::broadcast::broadcast_to_org;
use super::edit_row::{units, EditRow};
use super::fanout::feedback;
use super::log_names::label;
use super::officer_notes::{sync_for_rank_locked, NoteSync};
use super::order::org_order_guard;
use super::telemetry::OrgReject;
use super::{OrgCtx, OrgPlayer};
use crate::base::organization::api::{lock_org, member_access_locked, OrgAccess};
use crate::base::organization::persistence::{
    load_ranks, set_rank_permissions, set_text, OrgStoreError, OrgTextTarget, RankRow,
};

/// A committed (or unchanged) rank-permission edit: what the fanout needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PermEdit {
    pub org_id: i32,
    pub rank: OrgRank,
    pub from: OrgPermission,
    pub to: OrgPermission,
    /// Every rank's mask after the edit, as read under the lock.
    pub table: Vec<(OrgRank, OrgPermission)>,
    /// Officer notes to show or hide, when `OfficerNotes` moved.
    pub note_sync: Option<NoteSync>,
}

impl PermEdit {
    pub fn changed(&self) -> bool {
        self.from != self.to
    }
}

/// The rank the wire names, if the type uses it.
fn rank_in_type(access: &OrgAccess, rank: i32) -> Result<OrgRank, OrgReject> {
    u8::try_from(rank)
        .ok()
        .and_then(|r| OrgRank::try_from(r).ok())
        .filter(|r| r.is_valid_for(access.org_type()))
        .ok_or(OrgReject::RankNotInType)
}

/// D-ORG09 (1)-(3) for an edit of `rank`'s row by `access`.
fn may_edit_rank(access: &OrgAccess, rank: OrgRank, bit: OrgPermission) -> Result<(), OrgReject> {
    if !access.permissions().contains(bit) {
        return Err(OrgReject::MissingPermission);
    }
    if access.rank() == rank {
        return Err(OrgReject::OwnRank);
    }
    if access.rank() <= rank {
        return Err(OrgReject::RankTooLow);
    }
    Ok(())
}

/// The permission edit, inside the caller's ORG-LOCK transaction, for a
/// member's `access` or a GM's `OrgAccess::system`. `rank` and `wire_mask`
/// are the client's values. Fills the masks into `row`.
pub(super) async fn rank_permissions_locked(
    tx: &mut Transaction<'_, Postgres>,
    access: &OrgAccess,
    rank: i32,
    wire_mask: i32,
    row: &mut EditRow,
) -> Result<PermEdit, OrgReject> {
    let org_id = access.org_id();
    let rank = rank_in_type(access, rank)?;
    if rank == OrgRank::LEADER {
        return Err(OrgReject::LeaderRowPinned);
    }
    may_edit_rank(access, rank, OrgPermission::ALTER_PERMS)?;
    let ranks = load_ranks(&mut **tx, org_id)
        .await
        .map_err(|e| row.db_failed(&e))?;
    let from = ranks
        .iter()
        .find(|r| r.rank == rank)
        .map(|r| r.permissions)
        .ok_or(OrgReject::RankNotInType)?;
    row.from_mask = Some(from.bits());
    let to = match OrgPermission::apply_edit(
        from,
        OrgPermission::from_wire(wire_mask),
        access.org_type(),
        access.permissions(),
    ) {
        Ok(to) => to,
        Err(PermEditReject::ChangesUnheldBits(bits)) => {
            row.unheld_mask = Some(bits.bits());
            return Err(OrgReject::ChangesUnheldBits);
        }
    };
    row.to_mask = Some(to.bits());
    let mut note_sync = None;
    if to != from {
        match set_rank_permissions(tx, access, org_id, rank, to).await {
            Ok(replaced) => debug_assert_eq!(replaced, from, "mask read under the same lock"),
            Err(OrgStoreError::LeaderPinned) => return Err(OrgReject::LeaderRowPinned),
            Err(OrgStoreError::RankNotInType(_)) => return Err(OrgReject::RankNotInType),
            Err(e) => return Err(row.db_failed(&e)),
        }
        note_sync = sync_for_rank_locked(tx, org_id, access.org_name(), rank, from, to)
            .await
            .map_err(|e| row.db_failed(&e))?;
    }
    let table = ranks
        .iter()
        .map(|r| (r.rank, if r.rank == rank { to } else { r.permissions }))
        .collect();
    Ok(PermEdit {
        org_id,
        rank,
        from,
        to,
        table,
        note_sync,
    })
}

/// After the commit of a changed [`PermEdit`]: [49] to every online
/// member, then the officer-note sync. The caller still holds the
/// organization's order guard.
pub(super) async fn announce_perm_edit(ctx: &OrgCtx<'_>, edit: &PermEdit) {
    if !edit.changed() {
        return;
    }
    let args = build_on_organization_rank_update(edit.org_id, &edit.table);
    broadcast_to_org(ctx, edit.org_id, ON_ORGANIZATION_RANK_UPDATE, &args, None).await;
    if let Some(sync) = &edit.note_sync {
        sync.send(ctx).await;
    }
}

/// Handle CM 16 for a Team or Command id.
#[tracing::instrument(
    name = "org.set_rank_permissions",
    level = "info",
    skip_all,
    fields(player_id = player.player_id, entity_id = player.entity_id, org_id, rank, wire_mask)
)]
pub async fn handle_set_rank_permissions(
    ctx: &OrgCtx<'_>,
    player: &OrgPlayer,
    org_id: i32,
    rank: i32,
    wire_mask: i32,
) -> Result<PermEdit, OrgReject> {
    let mut row = EditRow {
        event: "org.set_rank_permissions",
        action: "set_rank_permissions",
        account_id: player.account_id,
        player_id: Some(player.player_id),
        entity_id: Some(player.entity_id),
        org_id: Some(org_id),
        rank: Some(rank),
        wire_mask: Some(wire_mask),
        ..EditRow::default()
    };
    row.name_actor(ctx);
    let _order = org_order_guard(org_id).await;
    let decided = match ctx.db_pool.as_deref() {
        None => Err(OrgReject::NoDb),
        Some(pool) => match pool.begin().await {
            Ok(mut tx) => {
                let r = match member_locked(&mut tx, player, org_id, &mut row).await {
                    Ok(access) => {
                        rank_permissions_locked(&mut tx, &access, rank, wire_mask, &mut row).await
                    }
                    Err(why) => Err(why),
                };
                match r {
                    Ok(d) => tx.commit().await.map(|()| d).map_err(|e| row.db_failed(&e)),
                    Err(why) => Err(why),
                }
            }
            Err(e) => Err(row.db_failed(&e)),
        },
    };
    let edit = match decided {
        Ok(d) => d,
        Err(why) => return row.refuse(ctx, why, "").await,
    };
    if edit.changed() {
        tracing::debug!(
            target: "org",
            event = "rank_permissions_changed",
            account_id = player.account_id,
            account_name = row.account_name,
            player_id = player.player_id,
            player_name = row.player_name,
            org_id,
            org_name = row.org_name,
            rank = edit.rank.as_u8(),
            from_mask = edit.from.bits(),
            from_mask_names = %cimmeria_entity::organization::ORG_PERMISSIONS.render(edit.from.bits()),
            to_mask = edit.to.bits(),
            to_mask_names = %cimmeria_entity::organization::ORG_PERMISSIONS.render(edit.to.bits()),
            officer_notes_sync = edit.note_sync.is_some(),
            "organization rank permissions changed"
        );
    }
    announce_perm_edit(ctx, &edit).await;
    feedback(
        ctx,
        player.entity_id,
        &format!("Permissions for rank {} saved.", edit.rank.as_u8()),
    )
    .await;
    row.ok(if edit.changed() {
        "changed"
    } else {
        "unchanged"
    });
    Ok(edit)
}

/// Handle CM 17 for a Team or Command id. `Ok(true)` when the stored name
/// changed.
#[tracing::instrument(
    name = "org.set_rank_name",
    level = "info",
    skip_all,
    fields(player_id = player.player_id, entity_id = player.entity_id, org_id, rank)
)]
pub async fn handle_set_rank_name(
    ctx: &OrgCtx<'_>,
    player: &OrgPlayer,
    org_id: i32,
    rank: i32,
    name: &str,
) -> Result<bool, OrgReject> {
    let mut row = EditRow {
        event: "org.set_rank_name",
        action: "set_rank_name",
        account_id: player.account_id,
        player_id: Some(player.player_id),
        entity_id: Some(player.entity_id),
        org_id: Some(org_id),
        field: Some(TextField::RankName.name()),
        rank: Some(rank),
        to_units: Some(units(name)),
        ..EditRow::default()
    };
    row.name_actor(ctx);
    let name = match org_text::validate(TextField::RankName, name) {
        Ok(n) => n,
        Err(r) => return row.refuse(ctx, OrgReject::InvalidText(r), "").await,
    };
    row.to_units = Some(units(&name));
    let _order = org_order_guard(org_id).await;
    let decided = match ctx.db_pool.as_deref() {
        None => Err(OrgReject::NoDb),
        Some(pool) => match pool.begin().await {
            Ok(mut tx) => match name_locked(&mut tx, player, org_id, rank, &name, &mut row).await {
                Ok(d) => tx.commit().await.map(|()| d).map_err(|e| row.db_failed(&e)),
                Err(why) => Err(why),
            },
            Err(e) => Err(row.db_failed(&e)),
        },
    };
    let (edited, changed, names) = match decided {
        Ok(d) => d,
        Err(why) => return row.refuse(ctx, why, "").await,
    };
    if changed {
        let args = build_on_organization_rank_name_update(org_id, &names);
        broadcast_to_org(ctx, org_id, ON_ORGANIZATION_RANK_NAME_UPDATE, &args, None).await;
    }
    feedback(
        ctx,
        player.entity_id,
        &format!("Rank {} is now named {name}.", edited.as_u8()),
    )
    .await;
    row.ok(if changed { "changed" } else { "unchanged" });
    Ok(changed)
}

/// The rank-name edit under the lock. Returns the rank, whether the name
/// changed, and every custom name after the edit.
async fn name_locked(
    tx: &mut Transaction<'_, Postgres>,
    player: &OrgPlayer,
    org_id: i32,
    rank: i32,
    name: &str,
    row: &mut EditRow,
) -> Result<(OrgRank, bool, Vec<(OrgRank, String)>), OrgReject> {
    let access = member_locked(tx, player, org_id, row).await?;
    let rank = rank_in_type(&access, rank)?;
    may_edit_rank(&access, rank, OrgPermission::RANK_NAMES)?;
    let ranks: Vec<RankRow> = load_ranks(&mut **tx, org_id)
        .await
        .map_err(|e| row.db_failed(&e))?;
    let old = ranks
        .iter()
        .find(|r| r.rank == rank)
        .ok_or(OrgReject::RankNotInType)?
        .name
        .clone();
    row.from_units = Some(old.as_deref().map_or(0, units));
    let changed = old.as_deref() != Some(name);
    if changed {
        match set_text(tx, &access, org_id, OrgTextTarget::RankName { rank }, name).await {
            Ok(_) => {}
            Err(OrgStoreError::InvalidText(r)) => return Err(OrgReject::InvalidText(r)),
            Err(OrgStoreError::RankNotInType(_)) => return Err(OrgReject::RankNotInType),
            Err(e) => return Err(row.db_failed(&e)),
        }
    }
    let names = ranks
        .into_iter()
        .filter_map(|r| {
            if r.rank == rank {
                Some((r.rank, name.to_string()))
            } else {
                r.name.map(|n| (r.rank, n))
            }
        })
        .collect();
    Ok((rank, changed, names))
}

/// Lock the organization and read the actor's access (`not_member` for a
/// missing organization or membership).
async fn member_locked(
    tx: &mut Transaction<'_, Postgres>,
    player: &OrgPlayer,
    org_id: i32,
    row: &mut EditRow,
) -> Result<OrgAccess, OrgReject> {
    let header = lock_org(tx, org_id)
        .await
        .map_err(|e| row.db_failed(&e))?
        .ok_or(OrgReject::NotMember)?;
    row.org_type = Some(header.org_type.name());
    row.org_name = label(&header.name);
    let access = member_access_locked(tx, org_id, player.player_id)
        .await
        .map_err(|e| row.db_failed(&e))?
        .ok_or(OrgReject::NotMember)?;
    row.actor_rank = Some(access.rank().as_u8());
    Ok(access)
}
