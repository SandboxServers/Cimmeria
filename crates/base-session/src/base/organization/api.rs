//! The organization API other campaigns build on (ORG-API).
//!
//! The Bank / Vault campaign builds the Team and Command vaults and the
//! treasury on these three calls (campaign ledger
//! `docs/analysis/organizations/work-packets.md` § "Bank campaign API"):
//!
//! - [`lock_org`] takes the organization row lock. **ORG-LOCK (D-ORG04):**
//!   every Team or Command mutation opens a transaction and calls it
//!   *first*; the global lock order is the organization row, then
//!   `sgw_player` rows, then item rows (the order trade and vendor use).
//! - [`member_access_locked`] reads a player's rank and permissions inside
//!   that transaction. There is deliberately no pool-level variant: an
//!   authorization read outside the lock lets a member who was kicked or
//!   demoted a moment ago still act.
//! - [`org_vault_is_empty`] is the vault predicate every voluntary disband
//!   checks (D-ORG20). A stub until the Bank campaign's vault lands.
//!
//! `broadcast_to_org` (the fanout primitive) is ORG-07's.
//!
//! All of this assumes READ COMMITTED, the server's isolation level: a
//! statement after the lock wait sees what the previous lock holder
//! committed. Under REPEATABLE READ, locking a row changed since the
//! snapshot fails with a serialization error instead.

use cimmeria_entity::organization::{OrgPermission, OrgRank, OrgType, UnknownValue};
use sqlx::{Postgres, Transaction};

/// One organization row, as [`lock_org`] read it under the lock.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrgHeader {
    pub org_id: i32,
    pub org_type: OrgType,
    /// The display name, normalised when it was created (D-ORG10).
    pub name: String,
    /// The message of the day; empty when unset.
    pub motd: String,
    /// The treasury. Never negative (a `CHECK` on the column); the Bank
    /// campaign owns every change to it.
    pub cash: i64,
    /// Always 0: nothing says how an organization earns experience.
    pub experience: i64,
}

/// What one member may do in one organization, read under ORG-LOCK.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrgAccess {
    pub org_id: i32,
    pub org_type: OrgType,
    pub rank: OrgRank,
    /// The permission mask of `rank`'s row in this organization.
    pub permissions: OrgPermission,
}

/// Lock the organization row (`SELECT … FOR UPDATE`) and return it.
///
/// `Ok(None)` means there is no such organization (never created, or
/// disbanded); the caller rejects. Taking the lock twice in one transaction
/// is harmless, which is why every mutation in [`super::persistence`] takes
/// it again itself.
pub async fn lock_org(
    tx: &mut Transaction<'_, Postgres>,
    org_id: i32,
) -> Result<Option<OrgHeader>, sqlx::Error> {
    let row: Option<(i32, i16, String, String, i64, i64)> = sqlx::query_as(
        "SELECT org_id, org_type, name, motd, cash, experience \
         FROM sgw_organizations WHERE org_id = $1 FOR UPDATE",
    )
    .bind(org_id)
    .fetch_optional(&mut **tx)
    .await?;
    row.map(|(org_id, org_type, name, motd, cash, experience)| {
        Ok(OrgHeader {
            org_id,
            org_type: org_type_from_db(org_type)?,
            name,
            motd,
            cash,
            experience,
        })
    })
    .transpose()
}

/// Lock the organization, then read `player_id`'s rank and that rank's
/// permissions in it.
///
/// `Ok(None)` means the organization does not exist or the player is not a
/// member of it; either way the caller rejects. The lock is taken here
/// (again, if the caller already holds it), so the answer cannot be stale
/// by the time the caller acts on it inside the same transaction.
pub async fn member_access_locked(
    tx: &mut Transaction<'_, Postgres>,
    org_id: i32,
    player_id: i32,
) -> Result<Option<OrgAccess>, sqlx::Error> {
    if lock_org(tx, org_id).await?.is_none() {
        return Ok(None);
    }
    let row: Option<(i16, i16, i32)> = sqlx::query_as(
        "SELECT m.org_type, m.rank, r.permissions \
         FROM sgw_organization_members m \
         JOIN sgw_organization_ranks r ON r.org_id = m.org_id AND r.rank = m.rank \
         WHERE m.org_id = $1 AND m.player_id = $2",
    )
    .bind(org_id)
    .bind(player_id)
    .fetch_optional(&mut **tx)
    .await?;
    row.map(|(org_type, rank, permissions)| {
        Ok(OrgAccess {
            org_id,
            org_type: org_type_from_db(org_type)?,
            rank: rank_from_db(rank)?,
            permissions: permissions_from_db(permissions),
        })
    })
    .transpose()
}

/// `true` when the organization's vault holds no items and its treasury no
/// cash (D-ORG20). Every voluntary disband checks it under ORG-LOCK and
/// refuses when it is `false`.
///
/// **Stub:** always `true` until the Bank / Vault campaign replaces it. Its
/// SQL twin, `org_vault_is_empty_sql(org_id)` (`db/sgw/_functions.sql`), is
/// what the member-delete trigger calls when a character delete removes the
/// last member; the Bank campaign replaces both, and they must agree.
pub async fn org_vault_is_empty(
    _tx: &mut Transaction<'_, Postgres>,
    _org_id: i32,
) -> Result<bool, sqlx::Error> {
    Ok(true)
}

// ── Row decoding ─────────────────────────────────────────────────────────────
//
// The column CHECKs and foreign keys make every one of these unreachable for
// a row the server wrote; a hand-edited row surfaces as a decode error
// rather than a guessed value.

pub(super) fn org_type_from_db(v: i16) -> Result<OrgType, sqlx::Error> {
    u8::try_from(v)
        .ok()
        .and_then(|b| OrgType::try_from(b).ok())
        .filter(|t| t.is_persistent())
        .ok_or_else(|| sqlx::Error::Decode(Box::new(UnknownValue(i64::from(v)))))
}

pub(super) fn rank_from_db(v: i16) -> Result<OrgRank, sqlx::Error> {
    OrgRank::try_from(i32::from(v)).map_err(|e| sqlx::Error::Decode(Box::new(e)))
}

/// The column is range-checked to the 26 defined bits, so truncation never
/// drops anything the server stored.
pub(super) fn permissions_from_db(v: i32) -> OrgPermission {
    OrgPermission::from_wire(v)
}
