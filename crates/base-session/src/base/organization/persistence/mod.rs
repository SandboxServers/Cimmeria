//! Database persistence for Teams and Commands.
//!
//! Three tables (`db/sgw/Organizations/`): `sgw_organizations` (one row per
//! organization), `sgw_organization_ranks` (one row per rank the type uses,
//! with its name and permission mask) and `sgw_organization_members` (one
//! row per member). The leader is the member at rank 8; there is no leader
//! column. An `AFTER DELETE` trigger on the member table promotes a new
//! leader, or disbands, whenever a member row goes (D-ORG12, D-ORG20), so a
//! character delete never leaves a leaderless organization behind.
//!
//! **ORG-LOCK (D-ORG04).** Every mutation here takes a transaction and
//! locks the organization row ([`lock_org`]) before it reads or writes
//! anything else, so a caller's authorization read
//! ([`super::api::member_access_locked`]) and the write it authorizes see
//! the same state. [`create_org`] is the exception only because there is no
//! row to lock yet: it inserts the organization row first, which locks it.
//! The reads in [`loads`] take a pool or any executor; they feed display
//! (the login push, a roster), never an authorization decision.
//!
//! Text is validated here as well as by the callers: every name, MOTD, note
//! and rank name goes through `org_text::validate` (D-ORG10) before it
//! reaches the database, and the stored text is the validated one.

mod error;
mod loads;
mod members;
mod texts;

pub use error::OrgStoreError;
pub use loads::{
    load_memberships, load_ranks, load_roster, name_available, OrgMembership, RankRow, RosterMember,
};
pub use members::{add_member, remove_member, set_rank, AfterRemoval, MemberRemoval};
pub use texts::{set_rank_permissions, set_text, OrgTextTarget};

use cimmeria_entity::organization::{
    default_rank_permissions, org_text, OrgRank, OrgType, TextField,
};
use sqlx::{Postgres, Transaction};

use super::api::{lock_org, org_vault_is_empty, OrgHeader};

/// A freshly created organization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreatedOrg {
    pub org_id: i32,
    /// The name as stored: `org_text`-normalised (trimmed, inner spaces
    /// collapsed).
    pub name: String,
}

/// Create a Team or Command: the organization row, one rank row per rank
/// the type uses (from `default_rank_permissions`, D-ORG08 as amended by
/// D-ORG21), and `leader_player_id` as its `Leader`.
///
/// Runs inside the caller's transaction so ORG-05 can debit the creation
/// cost (D-ORG15) in the same one: the organization row is inserted (and so
/// locked) first, then the caller may lock the `sgw_player` row, which keeps
/// the ORG-LOCK order. A refused name leaves nothing written and costs
/// nothing.
///
/// Refusals: [`OrgStoreError::NotPersistent`] for a Squad,
/// [`OrgStoreError::InvalidText`] for a bad name,
/// [`OrgStoreError::NameTaken`], [`OrgStoreError::NoSuchPlayer`] and
/// [`OrgStoreError::AlreadyInType`] (D-ORG18). The writes run under a
/// savepoint that a refusal rolls back, so a refused creation leaves the
/// caller's transaction as it was: committing it anyway can never persist
/// an organization without its leader.
pub async fn create_org(
    tx: &mut Transaction<'_, Postgres>,
    org_type: OrgType,
    name: &str,
    leader_player_id: i32,
) -> Result<CreatedOrg, OrgStoreError> {
    if !org_type.is_persistent() {
        return Err(OrgStoreError::NotPersistent);
    }
    let name = org_text::validate(TextField::Name, name)?;
    let mut savepoint = sqlx::Acquire::begin(&mut **tx).await?;
    match insert_org(&mut savepoint, org_type, name, leader_player_id).await {
        Ok(created) => {
            savepoint.commit().await?;
            Ok(created)
        }
        Err(refused) => {
            savepoint.rollback().await?;
            Err(refused)
        }
    }
}

/// The writes of [`create_org`]: the organization row, its rank rows and
/// its leader.
async fn insert_org(
    tx: &mut Transaction<'_, Postgres>,
    org_type: OrgType,
    name: String,
    leader_player_id: i32,
) -> Result<CreatedOrg, OrgStoreError> {
    let name_key = org_text::name_key(&name);

    // ON CONFLICT on the name key alone, so a taken name is a typed refusal
    // and the transaction stays usable for the caller's feedback.
    let org_id: Option<i32> = sqlx::query_scalar(
        "INSERT INTO sgw_organizations (org_type, name, name_key) VALUES ($1, $2, $3) \
         ON CONFLICT (org_type, name_key) DO NOTHING RETURNING org_id",
    )
    .bind(i16::from(org_type.as_u8()))
    .bind(&name)
    .bind(&name_key)
    .fetch_optional(&mut **tx)
    .await?;
    let org_id = org_id.ok_or(OrgStoreError::NameTaken)?;

    let (ranks, masks): (Vec<i16>, Vec<i32>) = default_rank_permissions(org_type)
        .into_iter()
        .map(|(rank, perms)| (i16::from(rank.as_u8()), perms.to_wire()))
        .unzip();
    sqlx::query(
        "INSERT INTO sgw_organization_ranks (org_id, rank, permissions) \
         SELECT $1, r, p FROM UNNEST($2::smallint[], $3::integer[]) AS t(r, p)",
    )
    .bind(org_id)
    .bind(&ranks)
    .bind(&masks)
    .execute(&mut **tx)
    .await?;

    members::insert_member(tx, org_id, org_type, leader_player_id, OrgRank::LEADER).await?;
    Ok(CreatedOrg { org_id, name })
}

/// Disband an organization: delete its row, which cascades to its ranks and
/// members. Returns the `player_id`s of the members it had, for the
/// `onOrganizationLeft(Disbanded)` fanout.
///
/// Refused with [`OrgStoreError::VaultNotEmpty`] while the vault or the
/// treasury holds anything (D-ORG20), and with
/// [`OrgStoreError::NoSuchOrg`] when there is nothing to disband.
pub async fn disband(
    tx: &mut Transaction<'_, Postgres>,
    org_id: i32,
) -> Result<Vec<i32>, OrgStoreError> {
    lock_or_miss(tx, org_id).await?;
    if !org_vault_is_empty(tx, org_id).await? {
        return Err(OrgStoreError::VaultNotEmpty);
    }
    let members: Vec<i32> = sqlx::query_scalar(
        "SELECT player_id FROM sgw_organization_members WHERE org_id = $1 ORDER BY player_id",
    )
    .bind(org_id)
    .fetch_all(&mut **tx)
    .await?;
    let deleted = sqlx::query("DELETE FROM sgw_organizations WHERE org_id = $1")
        .bind(org_id)
        .execute(&mut **tx)
        .await?;
    if deleted.rows_affected() == 0 {
        return Err(OrgStoreError::NoSuchOrg);
    }
    Ok(members)
}

/// [`lock_org`], with a missing organization as a typed miss.
async fn lock_or_miss(
    tx: &mut Transaction<'_, Postgres>,
    org_id: i32,
) -> Result<OrgHeader, OrgStoreError> {
    lock_org(tx, org_id).await?.ok_or(OrgStoreError::NoSuchOrg)
}

#[cfg(test)]
mod tests;
