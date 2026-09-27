//! Membership writes: add, remove and change rank.

use cimmeria_entity::organization::{OrgRank, OrgType};
use sqlx::{Postgres, Transaction};

use super::super::api::{rank_from_db, OrgAccess};
use super::observe::observed;
use super::{authorize, OrgStoreError};

/// What happened to the organization when a member row went.
///
/// The member-delete trigger decides this in the database; the Rust side
/// reads it back so the caller can fan out the right messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AfterRemoval {
    /// Nothing else changed.
    Unchanged,
    /// The trigger promoted this player to `Leader` (D-ORG12): the removed
    /// member was the leader, or the organization had none.
    LeaderPromoted { player_id: i32 },
    /// The removed member was the last one and the vault was empty, so the
    /// trigger deleted the organization.
    Disbanded,
    /// The removed member was the last one and the vault was not empty, so
    /// the organization stays, memberless, for GM recovery (D-ORG20).
    Memberless,
}

impl AfterRemoval {
    fn label(self) -> &'static str {
        match self {
            AfterRemoval::Unchanged => "unchanged",
            AfterRemoval::LeaderPromoted { .. } => "leader_changed",
            AfterRemoval::Disbanded => "disbanded",
            AfterRemoval::Memberless => "left_memberless",
        }
    }
}

/// The result of [`remove_member`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemberRemoval {
    /// The rank the removed member held.
    pub old_rank: OrgRank,
    pub after: AfterRemoval,
    /// The transaction's id. After committing, the caller passes it to
    /// `audit::export_committed(pool, tx_id, ExportSource::MemberRemoval)`,
    /// which logs the trigger's `sgw_organization_events` rows at INFO
    /// (`leader_changed`, `disbanded`, `left_memberless`). ORG-06's leave and
    /// ORG-07's kick handlers must make that call; a row they miss is
    /// logged by the next startup sweep instead.
    pub tx_id: i64,
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
    actor: &OrgAccess,
    org_id: i32,
    player_id: i32,
    rank: OrgRank,
) -> Result<(), OrgStoreError> {
    observed("add_member", Some(org_id), Some(player_id), async {
        let header = authorize(tx, actor, org_id).await?;
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
        let account_id = insert_member(tx, org_id, header.org_type, player_id, rank).await?;
        tracing::debug!(
            target: "org",
            event = "add_member",
            org_id,
            org_type = header.org_type.name(),
            player_id,
            account_id,
            rank = rank.as_u8(),
            rows_affected = 1u64,
            "Organization member added"
        );
        Ok(())
    })
    .await
}

/// Insert one member row and return the member's `account_id`. The
/// organization row must already be locked (or inserted) by this
/// transaction.
///
/// Membership in this organization is checked first, from the member rows
/// alone. Only a non-member's character is then read with `FOR KEY SHARE`,
/// which holds it against a concurrent delete until commit and gives the
/// `account_id` the member row keeps. That order is what keeps ORG-LOCK
/// deadlock-free: a transaction holding this organization never waits on
/// the `sgw_player` row of one of its members, which is the row a
/// character delete holds while it waits for the member's organizations
/// (`api` § "Lock order"). The insert uses `ON CONFLICT DO NOTHING`, so the
/// other unique key gives a typed refusal and leaves the transaction
/// usable.
pub(super) async fn insert_member(
    tx: &mut Transaction<'_, Postgres>,
    org_id: i32,
    org_type: OrgType,
    player_id: i32,
    rank: OrgRank,
) -> Result<i32, OrgStoreError> {
    let member: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM sgw_organization_members \
         WHERE org_id = $1 AND player_id = $2)",
    )
    .bind(org_id)
    .bind(player_id)
    .fetch_one(&mut **tx)
    .await?;
    if member {
        return Err(OrgStoreError::AlreadyMember);
    }
    let account_id: Option<i32> =
        sqlx::query_scalar("SELECT account_id FROM sgw_player WHERE player_id = $1 FOR KEY SHARE")
            .bind(player_id)
            .fetch_optional(&mut **tx)
            .await?;
    let account_id = account_id.ok_or(OrgStoreError::NoSuchPlayer)?;
    let inserted = sqlx::query(
        "INSERT INTO sgw_organization_members (org_id, player_id, account_id, org_type, rank) \
         VALUES ($1, $2, $3, $4, $5) ON CONFLICT DO NOTHING",
    )
    .bind(org_id)
    .bind(player_id)
    .bind(account_id)
    .bind(i16::from(org_type.as_u8()))
    .bind(i16::from(rank.as_u8()))
    .execute(&mut **tx)
    .await?;
    if inserted.rows_affected() == 1 {
        return Ok(account_id);
    }
    // Not a member of this organization (checked above, under its lock), so
    // the key that matched is (player_id, org_type).
    Err(OrgStoreError::AlreadyInType)
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
///
/// The trigger's `sgw_organization_events` rows are not logged here: the
/// caller exports them after it commits ([`MemberRemoval::tx_id`]).
pub async fn remove_member(
    tx: &mut Transaction<'_, Postgres>,
    actor: &OrgAccess,
    org_id: i32,
    player_id: i32,
) -> Result<MemberRemoval, OrgStoreError> {
    observed("remove_member", Some(org_id), Some(player_id), async {
        authorize(tx, actor, org_id).await?;
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

        // The trigger has run by now (AFTER ROW triggers fire when the
        // DELETE statement ends). Read back what it did.
        let org_left: bool =
            sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM sgw_organizations WHERE org_id = $1)")
                .bind(org_id)
                .fetch_one(&mut **tx)
                .await?;
        let after = if !org_left {
            AfterRemoval::Disbanded
        } else {
            // Compared with the leader before, not keyed on the removed
            // rank, so a trigger that heals a leaderless organization is
            // reported too.
            match current_leader(tx, org_id).await? {
                None => AfterRemoval::Memberless,
                Some(p) if Some(p) != leader_before => {
                    AfterRemoval::LeaderPromoted { player_id: p }
                }
                Some(_) => AfterRemoval::Unchanged,
            }
        };
        tracing::debug!(
            target: "org",
            event = "remove_member",
            org_id,
            player_id,
            from_rank = old_rank.as_u8(),
            after = after.label(),
            rows_affected = 1u64,
            "Organization member removed"
        );
        Ok(MemberRemoval {
            old_rank,
            after,
            tx_id: actor.tx_id(),
        })
    })
    .await
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
    actor: &OrgAccess,
    org_id: i32,
    player_id: i32,
    rank: OrgRank,
) -> Result<OrgRank, OrgStoreError> {
    observed("set_rank", Some(org_id), Some(player_id), async {
        let header = authorize(tx, actor, org_id).await?;
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
        tracing::debug!(
            target: "org",
            event = "set_rank",
            org_id,
            player_id,
            from_rank = old.as_u8(),
            to_rank = rank.as_u8(),
            rows_affected = updated.rows_affected(),
            "Organization member rank changed"
        );
        Ok(old)
    })
    .await
}
