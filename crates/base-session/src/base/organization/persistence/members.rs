//! Membership writes: add, remove and change rank.

use cimmeria_entity::organization::{OrgRank, OrgType};
use sqlx::{Postgres, Transaction};

use super::super::api::rank_from_db;
use super::{lock_or_miss, OrgStoreError};

/// What happened to the organization when a member row went.
///
/// The member-delete trigger decides this in the database; the Rust side
/// reads it back so the caller can fan out the right messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AfterRemoval {
    /// Nothing else changed.
    Unchanged,
    /// The removed member was the leader and the trigger promoted this
    /// player to `Leader` (D-ORG12).
    LeaderPromoted { player_id: i32 },
    /// The removed member was the last one and the vault was empty, so the
    /// trigger deleted the organization.
    Disbanded,
    /// The removed member was the last one and the vault was not empty, so
    /// the organization stays, memberless, for GM recovery (D-ORG20).
    Memberless,
}

/// The result of [`remove_member`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemberRemoval {
    /// The rank the removed member held.
    pub old_rank: OrgRank,
    pub after: AfterRemoval,
}

/// Add `player_id` to the organization at `rank`.
///
/// `rank` must be one the type uses (D-ORG07). It may be `Leader` only when
/// the organization is memberless, which is the GM recovery path of
/// D-ORG20; a memberless organization takes nobody else first
/// ([`OrgStoreError::NeedsLeader`]). Other refusals:
/// [`OrgStoreError::NoSuchOrg`], [`OrgStoreError::NoSuchPlayer`],
/// [`OrgStoreError::AlreadyMember`] and [`OrgStoreError::AlreadyInType`]
/// (D-ORG18). Authorization (who may invite) is the caller's, under the
/// same lock.
pub async fn add_member(
    tx: &mut Transaction<'_, Postgres>,
    org_id: i32,
    player_id: i32,
    rank: OrgRank,
) -> Result<(), OrgStoreError> {
    let header = lock_or_miss(tx, org_id).await?;
    if !rank.is_valid_for(header.org_type) {
        return Err(OrgStoreError::RankNotInType(rank));
    }
    let memberless: bool = sqlx::query_scalar(
        "SELECT NOT EXISTS (SELECT 1 FROM sgw_organization_members WHERE org_id = $1)",
    )
    .bind(org_id)
    .fetch_one(&mut **tx)
    .await?;
    match (memberless, rank == OrgRank::LEADER) {
        (true, false) => return Err(OrgStoreError::NeedsLeader),
        (false, true) => return Err(OrgStoreError::LeaderPinned),
        _ => {}
    }
    insert_member(tx, org_id, header.org_type, player_id, rank).await
}

/// Insert one member row. The organization row must already be locked (or
/// inserted) by this transaction.
///
/// The character is checked first with `FOR KEY SHARE`, which also holds it
/// against a concurrent delete until commit (the lock the foreign key would
/// take anyway, taken after the organization row, per ORG-LOCK). The insert
/// uses `ON CONFLICT DO NOTHING`, so both unique keys give a typed refusal
/// and leave the transaction usable.
pub(super) async fn insert_member(
    tx: &mut Transaction<'_, Postgres>,
    org_id: i32,
    org_type: OrgType,
    player_id: i32,
    rank: OrgRank,
) -> Result<(), OrgStoreError> {
    let player_exists: Option<i32> =
        sqlx::query_scalar("SELECT 1 FROM sgw_player WHERE player_id = $1 FOR KEY SHARE")
            .bind(player_id)
            .fetch_optional(&mut **tx)
            .await?;
    if player_exists.is_none() {
        return Err(OrgStoreError::NoSuchPlayer);
    }
    let inserted = sqlx::query(
        "INSERT INTO sgw_organization_members (org_id, player_id, org_type, rank) \
         VALUES ($1, $2, $3, $4) ON CONFLICT DO NOTHING",
    )
    .bind(org_id)
    .bind(player_id)
    .bind(i16::from(org_type.as_u8()))
    .bind(i16::from(rank.as_u8()))
    .execute(&mut **tx)
    .await?;
    if inserted.rows_affected() == 1 {
        return Ok(());
    }
    // One of the two unique keys matched: (org_id, player_id) or
    // (player_id, org_type). Which one decides the feedback.
    let same_org: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM sgw_organization_members \
         WHERE org_id = $1 AND player_id = $2)",
    )
    .bind(org_id)
    .bind(player_id)
    .fetch_one(&mut **tx)
    .await?;
    Err(if same_org {
        OrgStoreError::AlreadyMember
    } else {
        OrgStoreError::AlreadyInType
    })
}

/// Remove `player_id` from the organization, and report what the
/// member-delete trigger did about it.
///
/// Refused with [`OrgStoreError::NoSuchOrg`] or
/// [`OrgStoreError::NotAMember`]. Removing the leader is not refused here:
/// D-ORG12's "a leader cannot leave while others remain" is the caller's
/// rule, and if a caller does remove the leader the trigger promotes the
/// next member rather than leaving the organization leaderless. Removing
/// the last member disbands the organization when `org_vault_is_empty_sql`
/// says the vault is empty, and leaves it memberless otherwise; a caller
/// running a voluntary leave checks `api::org_vault_is_empty` first
/// (D-ORG20).
pub async fn remove_member(
    tx: &mut Transaction<'_, Postgres>,
    org_id: i32,
    player_id: i32,
) -> Result<MemberRemoval, OrgStoreError> {
    lock_or_miss(tx, org_id).await?;
    let leader_before = current_leader(tx, org_id).await?;
    let old_rank: Option<i16> = sqlx::query_scalar(
        "DELETE FROM sgw_organization_members WHERE org_id = $1 AND player_id = $2 \
         RETURNING rank",
    )
    .bind(org_id)
    .bind(player_id)
    .fetch_optional(&mut **tx)
    .await?;
    let old_rank = rank_from_db(old_rank.ok_or(OrgStoreError::NotAMember)?)?;

    // The trigger has run by now (AFTER ROW triggers fire when the DELETE
    // statement ends). Read back what it did.
    let org_left: bool =
        sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM sgw_organizations WHERE org_id = $1)")
            .bind(org_id)
            .fetch_one(&mut **tx)
            .await?;
    let after = if !org_left {
        AfterRemoval::Disbanded
    } else {
        // Compared with the leader before, not keyed on the removed rank,
        // so a trigger that heals a leaderless organization is reported too.
        match current_leader(tx, org_id).await? {
            None => AfterRemoval::Memberless,
            Some(p) if Some(p) != leader_before => AfterRemoval::LeaderPromoted { player_id: p },
            Some(_) => AfterRemoval::Unchanged,
        }
    };
    Ok(MemberRemoval { old_rank, after })
}

/// The member at rank 8, if any (at most one: a unique partial index).
async fn current_leader(
    tx: &mut Transaction<'_, Postgres>,
    org_id: i32,
) -> Result<Option<i32>, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT player_id FROM sgw_organization_members WHERE org_id = $1 AND rank = 8",
    )
    .bind(org_id)
    .fetch_optional(&mut **tx)
    .await
}

/// Set `player_id`'s rank to `rank`, returning the rank it held before.
///
/// `rank` must be one the type uses ([`OrgStoreError::RankNotInType`]; so
/// 0, and Team rank 5, are refused, D-ORG09 (5)). The `Leader` rank is
/// pinned ([`OrgStoreError::LeaderPinned`]): it is never assigned, and the
/// current leader is never moved off it, because either would break "the
/// leader is the member at rank 8" (D-ORG09 (4); no transfer feature
/// exists). Who may change whose rank (D-ORG09 (1)-(2)) is the caller's
/// check, under the same lock.
pub async fn set_rank(
    tx: &mut Transaction<'_, Postgres>,
    org_id: i32,
    player_id: i32,
    rank: OrgRank,
) -> Result<OrgRank, OrgStoreError> {
    let header = lock_or_miss(tx, org_id).await?;
    if !rank.is_valid_for(header.org_type) {
        return Err(OrgStoreError::RankNotInType(rank));
    }
    if rank == OrgRank::LEADER {
        return Err(OrgStoreError::LeaderPinned);
    }
    let old: Option<i16> = sqlx::query_scalar(
        "SELECT rank FROM sgw_organization_members WHERE org_id = $1 AND player_id = $2",
    )
    .bind(org_id)
    .bind(player_id)
    .fetch_optional(&mut **tx)
    .await?;
    let old = rank_from_db(old.ok_or(OrgStoreError::NotAMember)?)?;
    if old == OrgRank::LEADER {
        return Err(OrgStoreError::LeaderPinned);
    }
    let updated = sqlx::query(
        "UPDATE sgw_organization_members SET rank = $3 WHERE org_id = $1 AND player_id = $2",
    )
    .bind(org_id)
    .bind(player_id)
    .bind(i16::from(rank.as_u8()))
    .execute(&mut **tx)
    .await?;
    if updated.rows_affected() == 0 {
        return Err(OrgStoreError::NotAMember);
    }
    Ok(old)
}
