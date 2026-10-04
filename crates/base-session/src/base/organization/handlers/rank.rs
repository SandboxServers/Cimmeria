//! `organizationRankChange` (0xD2) for a Team or Command id (ORG-07,
//! CAT-M-06).
//!
//! Under ORG-LOCK (D-ORG04), in this order (the order only chooses which
//! reason a refusal reports; every check runs under the one lock):
//!
//! 1. the actor is a member (`not_member`);
//! 2. the target is found by name among that organization's members
//!    (`target_not_member`, `target_ambiguous`) and is not the actor
//!    (`self_target`);
//! 3. the new rank is one the type uses (`rank_not_in_type`, D-ORG09 (5):
//!    0, and Team rank 5, are refused) and is not `Leader`
//!    (`leader_not_assignable`, D-ORG09 (4));
//! 4. it differs from the current rank (`rank_unchanged`), which fixes the
//!    direction before a bit is chosen;
//! 5. the actor holds `Promote` for a raise or `Demote` for a lowering
//!    (`missing_permission`, D-ORG09 (1));
//! 6. the actor's rank is strictly above the target's current rank **and**
//!    the new one (`rank_too_low`, D-ORG09 (2)), so a Senior Officer can
//!    make nobody a Senior Officer and can touch no peer.
//!
//! After the commit every online member, the target included, gets
//! `onMemberRankChangedOrganization` [40] (`broadcast_to_org`).

use cimmeria_entity::organization::{OrgPermission, OrgRank};
use cimmeria_wire::cell::client_methods::organization::{
    build_on_member_rank_changed_organization, ON_MEMBER_RANK_CHANGED_ORGANIZATION,
};
use sqlx::{Postgres, Transaction};

use super::answer::{db_failed, refusal_text, refuse};
use super::broadcast::broadcast_to_org;
use super::fanout::feedback;
use super::log_names::label;
use super::officer_notes::{sync_for_member_locked, NoteSync};
use super::order::org_order_guard;
use super::targets::{member_by_name, online_now, MemberTarget};
use super::telemetry::{ActionRow, OrgReject};
use super::{OrgCtx, OrgPlayer};
use crate::base::organization::api::{lock_org, member_access_locked};
use crate::base::organization::persistence::{set_rank, OrgStoreError};

const RANK_SELF_TEXT: &str = "You cannot change your own rank.";

/// Handle 0xD2 for a Team or Command id. `rank` is the raw wire byte.
/// Returns the rank the target held before.
#[tracing::instrument(
    name = "org.rank_change",
    level = "info",
    skip_all,
    fields(player_id = player.player_id, entity_id = player.entity_id, org_id, rank)
)]
pub async fn handle_rank_change(
    ctx: &OrgCtx<'_>,
    player: &OrgPlayer,
    org_id: i32,
    target_name: &str,
    rank: u8,
) -> Result<OrgRank, OrgReject> {
    let mut row = ActionRow {
        event: "org.rank_change",
        action: "rank_change",
        account_id: player.account_id,
        player_id: Some(player.player_id),
        entity_id: Some(player.entity_id),
        org_id: Some(org_id),
        to_rank: Some(rank),
        ..ActionRow::default()
    };
    row.name_actor(ctx);
    // Held until the last send: the rank move may change who reads officer
    // notes, and its sync must not cross another edit's fanout (ORG-08).
    let _order = org_order_guard(org_id).await;
    let decided = match ctx.db_pool.as_deref() {
        None => Err(OrgReject::NoDb),
        Some(pool) => match pool.begin().await {
            Ok(mut tx) => {
                match rank_locked(ctx, &mut tx, player, org_id, target_name, rank, &mut row).await {
                    Ok(d) => match tx.commit().await {
                        Ok(()) => Ok(d),
                        Err(e) => Err(db_failed(&row, &e)),
                    },
                    Err(why) => Err(why),
                }
            }
            Err(e) => Err(db_failed(&row, &e)),
        },
    };
    let (target, to, org_name, note_sync) = match decided {
        Ok(d) => d,
        Err(why) => {
            let text = refusal_text(why, RANK_SELF_TEXT, None);
            return refuse(ctx, &row, player.entity_id, why, &text).await;
        }
    };
    tracing::debug!(
        target: "org",
        event = "rank_changed",
        account_id = player.account_id,
        account_name = row.account_name,
        player_id = player.player_id,
        player_name = row.player_name,
        target_account_id = row.target_account_id,
        target_account_name = row.target_account_name,
        target_player_id = target.player_id,
        target_player_name = row.target_player_name,
        org_id,
        org_name = org_name.as_str(),
        from_rank = target.rank.as_u8(),
        to_rank = to.as_u8(),
        "organization member rank changed"
    );
    announce_rank(ctx, org_id, &target, to, &org_name, note_sync.as_ref()).await;
    feedback(
        ctx,
        player.entity_id,
        &format!("{} is now rank {} in {org_name}.", target.name, to.as_u8()),
    )
    .await;
    row.ok("rank_changed");
    Ok(target.rank)
}

/// After a committed rank change: [40] to every online member, the
/// officer-note sync when the move changed whether the target may read
/// them (ORG-08), and a line to the target if they are online now.
pub(super) async fn announce_rank(
    ctx: &OrgCtx<'_>,
    org_id: i32,
    target: &MemberTarget,
    to: OrgRank,
    org_name: &str,
    note_sync: Option<&NoteSync>,
) {
    let online = online_now(ctx, target.player_id);
    let member_id = online.map_or(0, |m| m.entity_id as i32);
    let args = build_on_member_rank_changed_organization(member_id, to, org_id, &target.name);
    broadcast_to_org(
        ctx,
        org_id,
        ON_MEMBER_RANK_CHANGED_ORGANIZATION,
        &args,
        None,
    )
    .await;
    if let Some(sync) = note_sync {
        sync.send(ctx).await;
    }
    if let Some(m) = online {
        feedback(
            ctx,
            m.entity_id,
            &format!("Your rank in {org_name} is now {}.", to.as_u8()),
        )
        .await;
    }
}

/// The locked part: every D-ORG09 check, then the write, inside `tx`.
/// Returns the target (with the rank they held), the new rank, the
/// organization's name and the officer-note sync the move needs.
async fn rank_locked(
    ctx: &OrgCtx<'_>,
    tx: &mut Transaction<'_, Postgres>,
    player: &OrgPlayer,
    org_id: i32,
    target_name: &str,
    rank: u8,
    row: &mut ActionRow,
) -> Result<(MemberTarget, OrgRank, String, Option<NoteSync>), OrgReject> {
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
    let to = OrgRank::try_from(rank)
        .ok()
        .filter(|r| r.is_valid_for(header.org_type))
        .ok_or(OrgReject::RankNotInType)?;
    if to == OrgRank::LEADER {
        return Err(OrgReject::LeaderNotAssignable);
    }
    if to == target.rank {
        return Err(OrgReject::RankUnchanged);
    }
    let needed = if to > target.rank {
        OrgPermission::PROMOTE
    } else {
        OrgPermission::DEMOTE
    };
    if !access.permissions().contains(needed) {
        return Err(OrgReject::MissingPermission);
    }
    if access.rank() <= target.rank || access.rank() <= to {
        return Err(OrgReject::RankTooLow);
    }
    match set_rank(tx, &access, org_id, target.player_id, to).await {
        Ok(_) => {
            let sync =
                sync_for_member_locked(tx, org_id, row.org_name, target.player_id, target.rank, to)
                    .await
                    .map_err(|e| db(row, &e))?;
            Ok((target, to, header.name, sync))
        }
        // Ruled out above, under the lock; kept typed in case a caller
        // bypasses the checks.
        Err(OrgStoreError::LeaderPinned) => Err(OrgReject::LeaderNotAssignable),
        Err(OrgStoreError::RankNotInType(_)) => Err(OrgReject::RankNotInType),
        Err(e) => Err(db(row, &e)),
    }
}
