//! GM `.org_join <orgId> [player]` and `.org_rank <player> <rank> [orgId]`
//! (ORG-07, D-ORG13).
//!
//! The cell's `.`-console runs only a GameMaster's line, but no privilege
//! bit travels: the base resolves the GM's own session by character **and**
//! entity id (`resolve_actor`, so a recycled entity id finds nobody) and
//! re-reads its access level. The commands skip the member permission and
//! rank checks (D-ORG09 (1)-(2)); they keep every data rule:
//!
//! - `.org_join` adds at the type's entry rank (D-ORG07), or as `Leader`
//!   when the organization is memberless (D-ORG20's recovery case), and is
//!   refused when the player is already in an organization of that type
//!   (D-ORG18). The player must be online, so they get the state push.
//! - `.org_rank` refuses a rank the type does not use and `Leader`
//!   (D-ORG09 (4), (5)); the persistence layer pins both again. The member
//!   may be offline. Without an org id it acts on the one Team or Command
//!   the member is in, and refuses when they are in both.
//!
//! Each command ends in one `org.gm_join` / `org.gm_rank` outcome row and
//! its `org.gm_action` twin with the GM, the target and the result
//! (`ActionRow::gm_audit`, ORG-10); `OrgAccess::system` adds the
//! `org.gm_access` lock audit once the organization is locked.

use cimmeria_entity::organization::OrgRank;
use sqlx::{Postgres, Transaction};

use super::answer::{db_failed, refusal_text, refuse};
use super::disband::{GmCaller, GM_ACCESS_LEVEL};
use super::invite_response::{add_joined, announce_join};
use super::officer_notes::{sync_for_member_locked, NoteSync};
use super::order::org_order_guard;
use super::rank::announce_rank;
use super::targets::{member_by_name, MemberTarget};
use super::telemetry::{ActionRow, OrgReject};
use super::{resolve_actor, OrgCtx, OrgPlayer};
use crate::base::organization::api::{OrgAccess, SystemActor};
use crate::base::organization::persistence::{set_rank, OrgStoreError};
use crate::base::player_index::{NameLookup, OnlinePlayerIndex};

/// The GM's session, if it still plays `gm.player_id` on `gm.entity_id`,
/// and its access level.
pub(super) fn gm_session(ctx: &OrgCtx<'_>, gm: GmCaller) -> Option<(OrgPlayer, u32)> {
    let player = resolve_actor(ctx, gm.player_id, gm.entity_id)?;
    let addr = ctx
        .entity_to_addr
        .lock()
        .ok()?
        .get(&gm.entity_id)
        .copied()?;
    let level = ctx.connected.lock().ok()?.get(&addr)?.access_level;
    Some((player, level))
}

pub(super) fn gm_actor(player: &OrgPlayer, command: &'static str) -> SystemActor<'static> {
    SystemActor::Gm {
        account_id: player.account_id.and_then(|a| i32::try_from(a).ok()),
        player_id: Some(player.player_id),
        command,
    }
}

/// `.org_join <orgId> [player]`: add `target_name` (default: the GM) to
/// `org_id`.
#[tracing::instrument(
    name = "org.gm_join",
    level = "info",
    skip_all,
    fields(entity_id = gm.entity_id, org_id)
)]
pub async fn gm_join(
    ctx: &OrgCtx<'_>,
    gm: GmCaller,
    org_id: i32,
    target_name: Option<&str>,
) -> Result<OrgRank, OrgReject> {
    let mut row = ActionRow {
        event: "org.gm_join",
        action: "gm_org_join",
        player_id: Some(gm.player_id),
        entity_id: Some(gm.entity_id),
        org_id: Some(org_id),
        gm_audit: true,
        ..ActionRow::default()
    };
    let fail = |row: ActionRow, why: OrgReject| async move {
        let text = format!("org_join: refused ({}).", why.reason());
        refuse(ctx, &row, gm.entity_id, why, &text).await
    };
    let Some((caller, level)) = gm_session(ctx, gm) else {
        return fail(row, OrgReject::NotGm).await;
    };
    row.account_id = caller.account_id;
    if level < GM_ACCESS_LEVEL {
        return fail(row, OrgReject::NotGm).await;
    }
    let target = match target_name {
        None => Ok(caller.player_id),
        Some(name) => match ctx.connected.lock() {
            Err(_) => Err(OrgReject::TargetNotFound),
            Ok(clients) => match OnlinePlayerIndex::new(&clients).lookup(name) {
                NameLookup::Found(p) => Ok(p.player_id),
                NameLookup::Ambiguous => Err(OrgReject::TargetAmbiguous),
                NameLookup::NotFound => Err(OrgReject::TargetNotFound),
            },
        },
    };
    let target = match target {
        Ok(t) => t,
        Err(why) => return fail(row, why).await,
    };
    row.target_player_id = Some(target);
    let Some(pool) = ctx.db_pool.as_deref() else {
        return fail(row, OrgReject::NoDb).await;
    };
    let joined = match pool.begin().await {
        Ok(mut tx) => match join_locked(&mut tx, &caller, org_id, target, &mut row).await {
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
        Err(why) => return fail(row, why).await,
    };
    announce_join(ctx, org_id, target, &member_name, rank, "gm").await;
    super::fanout::feedback(
        ctx,
        gm.entity_id,
        &format!(
            "org_join: added {member_name} to {org_name} (id {org_id}) at rank {}.",
            rank.as_u8()
        ),
    )
    .await;
    row.ok("joined");
    Ok(rank)
}

async fn join_locked(
    tx: &mut Transaction<'_, Postgres>,
    gm: &OrgPlayer,
    org_id: i32,
    target: i32,
    row: &mut ActionRow,
) -> Result<(OrgRank, String, String), OrgReject> {
    let db = |row: &ActionRow, e: &dyn std::fmt::Display| db_failed(row, e);
    let access = OrgAccess::system(tx, org_id, gm_actor(gm, "org_join"))
        .await
        .map_err(|e| db(row, &e))?
        .ok_or(OrgReject::NoSuchOrg)?;
    row.org_type = Some(access.org_type().name());
    let (org_name, memberless): (String, bool) = sqlx::query_as(
        "SELECT o.name, NOT EXISTS (SELECT 1 FROM sgw_organization_members m \
                                    WHERE m.org_id = o.org_id) \
         FROM sgw_organizations o WHERE o.org_id = $1",
    )
    .bind(org_id)
    .fetch_one(&mut **tx)
    .await
    .map_err(|e| db(row, &e))?;
    let rank = if memberless {
        OrgRank::LEADER
    } else {
        OrgRank::entry_for(access.org_type())
    };
    row.to_rank = Some(rank.as_u8());
    let (name, account_id): (String, i32) =
        sqlx::query_as("SELECT player_name, account_id FROM sgw_player WHERE player_id = $1")
            .bind(target)
            .fetch_one(&mut **tx)
            .await
            .map_err(|e| db(row, &e))?;
    row.target_account_id = u32::try_from(account_id).ok();
    add_joined(tx, &access, org_id, target, rank, row).await?;
    Ok((rank, org_name, name))
}

/// `.org_rank <player> <rank> [orgId]`: set a member's rank, skipping the
/// D-ORG09 authority checks but not the rank rules.
#[tracing::instrument(
    name = "org.gm_rank",
    level = "info",
    skip_all,
    fields(entity_id = gm.entity_id, org_id, rank)
)]
pub async fn gm_rank(
    ctx: &OrgCtx<'_>,
    gm: GmCaller,
    target_name: &str,
    rank: u8,
    org_id: Option<i32>,
) -> Result<OrgRank, OrgReject> {
    let mut row = ActionRow {
        event: "org.gm_rank",
        action: "gm_org_rank",
        player_id: Some(gm.player_id),
        entity_id: Some(gm.entity_id),
        org_id,
        to_rank: Some(rank),
        gm_audit: true,
        ..ActionRow::default()
    };
    let fail = |row: ActionRow, why: OrgReject| async move {
        let text = match why {
            OrgReject::OrgAmbiguous => refusal_text(why, "", None),
            _ => format!("org_rank: refused ({}).", why.reason()),
        };
        refuse(ctx, &row, gm.entity_id, why, &text).await
    };
    let Some((caller, level)) = gm_session(ctx, gm) else {
        return fail(row, OrgReject::NotGm).await;
    };
    row.account_id = caller.account_id;
    if level < GM_ACCESS_LEVEL {
        return fail(row, OrgReject::NotGm).await;
    }
    let Some(pool) = ctx.db_pool.as_deref() else {
        return fail(row, OrgReject::NoDb).await;
    };
    // The organization first, so its order guard (ORG-08) is held before
    // the transaction; the lock below re-reads the member.
    let org_id = match org_id {
        Some(id) => id,
        None => match org_of_member(pool, target_name).await {
            Ok(id) => id,
            Err(Ok(why)) => return fail(row, why).await,
            Err(Err(e)) => {
                let why = db_failed(&row, &e);
                return fail(row, why).await;
            }
        },
    };
    row.org_id = Some(org_id);
    let _order = org_order_guard(org_id).await;
    let decided = match pool.begin().await {
        Ok(mut tx) => {
            match rank_locked(&mut tx, &caller, target_name, rank, org_id, &mut row).await {
                Ok(d) => match tx.commit().await {
                    Ok(()) => Ok(d),
                    Err(e) => Err(db_failed(&row, &e)),
                },
                Err(why) => Err(why),
            }
        }
        Err(e) => Err(db_failed(&row, &e)),
    };
    let (target, to, org_name, note_sync) = match decided {
        Ok(d) => d,
        Err(why) => return fail(row, why).await,
    };
    tracing::debug!(
        target: "org",
        event = "rank_changed",
        via = "gm",
        account_id = caller.account_id,
        player_id = caller.player_id,
        target_account_id = row.target_account_id,
        target_player_id = target.player_id,
        org_id,
        from_rank = target.rank.as_u8(),
        to_rank = to.as_u8(),
        "organization member rank changed"
    );
    announce_rank(ctx, org_id, &target, to, &org_name, note_sync.as_ref()).await;
    super::fanout::feedback(
        ctx,
        gm.entity_id,
        &format!(
            "org_rank: {} is now rank {} in {org_name} (id {org_id}), was {}.",
            target.name,
            to.as_u8(),
            target.rank.as_u8()
        ),
    )
    .await;
    row.ok("rank_changed");
    Ok(target.rank)
}

async fn rank_locked(
    tx: &mut Transaction<'_, Postgres>,
    gm: &OrgPlayer,
    target_name: &str,
    rank: u8,
    org_id: i32,
    row: &mut ActionRow,
) -> Result<(MemberTarget, OrgRank, String, Option<NoteSync>), OrgReject> {
    let db = |row: &ActionRow, e: &dyn std::fmt::Display| db_failed(row, e);
    let access = OrgAccess::system(tx, org_id, gm_actor(gm, "org_rank"))
        .await
        .map_err(|e| db(row, &e))?
        .ok_or(OrgReject::NoSuchOrg)?;
    row.org_type = Some(access.org_type().name());
    let target = member_by_name(tx, org_id, target_name)
        .await
        .map_err(|e| db(row, &e))??;
    row.target_player_id = Some(target.player_id);
    row.target_account_id = u32::try_from(target.account_id).ok();
    row.target_rank = Some(target.rank.as_u8());
    let to = OrgRank::try_from(rank)
        .ok()
        .filter(|r| r.is_valid_for(access.org_type()))
        .ok_or(OrgReject::RankNotInType)?;
    if to == OrgRank::LEADER || target.rank == OrgRank::LEADER {
        return Err(OrgReject::LeaderNotAssignable);
    }
    if to == target.rank {
        return Err(OrgReject::RankUnchanged);
    }
    let org_name: String =
        sqlx::query_scalar("SELECT name FROM sgw_organizations WHERE org_id = $1")
            .bind(org_id)
            .fetch_one(&mut **tx)
            .await
            .map_err(|e| db(row, &e))?;
    match set_rank(tx, &access, org_id, target.player_id, to).await {
        Ok(_) => {
            let sync = sync_for_member_locked(tx, org_id, target.player_id, target.rank, to)
                .await
                .map_err(|e| db(row, &e))?;
            Ok((target, to, org_name, sync))
        }
        Err(OrgStoreError::LeaderPinned) => Err(OrgReject::LeaderNotAssignable),
        Err(OrgStoreError::RankNotInType(_)) => Err(OrgReject::RankNotInType),
        Err(e) => Err(db(row, &e)),
    }
}

/// The one Team or Command a member named `target_name` is in, for
/// `.org_rank` without an org id. A display read to pick the organization;
/// the locked transaction re-reads the member. `Err(Ok(_))` is a refusal.
async fn org_of_member(
    pool: &sqlx::PgPool,
    target_name: &str,
) -> Result<i32, Result<OrgReject, sqlx::Error>> {
    let ids: Vec<i32> = sqlx::query_scalar(
        "SELECT DISTINCT m.org_id FROM sgw_organization_members m \
         JOIN sgw_player p ON p.player_id = m.player_id \
         WHERE lower(p.player_name) = lower($1)",
    )
    .bind(target_name)
    .fetch_all(pool)
    .await
    .map_err(Err)?;
    match ids.as_slice() {
        [] => Err(Ok(OrgReject::TargetNotMember)),
        [one] => Ok(*one),
        _ => Err(Ok(OrgReject::OrgAmbiguous)),
    }
}
