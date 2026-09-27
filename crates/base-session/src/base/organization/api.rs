//! The organization API other campaigns build on (ORG-API).
//!
//! The Bank / Vault campaign builds the Team and Command vaults and the
//! treasury on these three calls (campaign ledger
//! `docs/analysis/organizations/work-packets.md` § "Bank campaign API"):
//!
//! - [`lock_org`] takes the organization row lock. **ORG-LOCK (D-ORG04):**
//!   every Team or Command mutation opens a transaction and takes it before
//!   it reads or writes anything else about the organization. It is the
//!   first row lock the transaction takes, except for the members'
//!   `sgw_player` rows a path must pre-lock (see "Lock order" below).
//! - [`member_access_locked`] reads a player's rank and permissions inside
//!   that transaction and returns an [`OrgAccess`]. There is deliberately no
//!   pool-level variant: an authorization read outside the lock lets a
//!   member who was kicked or demoted a moment ago still act.
//! - [`OrgAccess`] is the proof of that read. Only [`member_access_locked`]
//!   and [`OrgAccess::system`] build one; every persistence mutation takes
//!   one and refuses it if it names another organization or was read in
//!   another transaction (`OrgStoreError::ActorMismatch`,
//!   `OrgStoreError::StaleAccess`).
//! - [`org_vault_is_empty`] is the vault predicate every voluntary disband
//!   checks (D-ORG20): no rows in `sgw_organization_vault_items` and no
//!   treasury cash (bank-vault BV-07).
//!
//! - [`broadcast_to_org`] sends one client method to every online member,
//!   optionally only those whose rank holds a permission (ORG-07). Call it
//!   after the commit.
//!
//! ## Lock order
//!
//! 1. **Organization-scoped work** (every persistence mutation, the Bank's
//!    cash and vault changes): the organization row, then non-member
//!    `sgw_player` rows, then item rows (the order trade and vendor use for
//!    players and items).
//! 2. **Organization work that waits on a member's `sgw_player` row** (the
//!    Bank's vault withdraw, which inserts `sgw_inventory` rows referencing
//!    the character, and its cash `UPDATE`): take those player rows
//!    `FOR KEY SHARE` **before** [`lock_org`], in `player_id` order when
//!    there are several, then the organization row, then item rows. This
//!    mirrors the character-delete order below, so the two never cross.
//!    The Bank's write to the acting character's row stays a plain
//!    `UPDATE` (no key change), never `SELECT … FOR UPDATE`, so it does not
//!    block `add_member`'s `FOR KEY SHARE` on a joining character.
//! 3. **Character deletes** (from any path, an account delete cascading to
//!    its characters included): the character's `sgw_player` row
//!    (`FOR UPDATE`), then its organization rows in `org_id` order (the
//!    `sgw_player_before_delete_lock_orgs` trigger), then its member rows.
//!    An account delete takes this order for all its characters at once
//!    (`account_before_delete_lock_orgs`): every character row in
//!    `player_id` order, then all their organizations in `org_id` order,
//!    so two characters in two organizations never lock them out of order.
//!    A character can be deleted while it is in the world (an admin
//!    account delete), so no path may assume a member's row is safe to wait
//!    on because its owner is online.
//! 4. **Creation** (ORG-05): the founder's creation advisory lock, then the
//!    name's, then the new organization row, then the founder's
//!    `sgw_player` row. It never waits on an existing organization row.
//!    `add_member` takes the joining character's creation lock too (ORG-07),
//!    after its membership check and before the `FOR KEY SHARE`, so an
//!    accepted invite and a creation by the same character serialise. The
//!    advisory locks come before any row lock.
//!
//! The orders never form a cycle because no transaction holding an
//! organization waits on the `sgw_player` row, or the creation lock, of one
//! of that organization's members: a path that needs a member's row takes
//! it before the organization (2); `add_member` checks membership before it
//! takes either for the joining character, so it only waits on a
//! non-member's; and kicks and rank changes touch member rows, not player
//! rows. One reachable exception: an account delete
//! (`account_before_delete_lock_orgs`) locks every character on the
//! account, so it can hold a non-member character that an `add_member`
//! waits on while itself waiting on an organization a sibling character
//! belongs to. Postgres detects that cycle (40P01) rather than hanging, and
//! it is reachable only from credential cleanup and tests; a future
//! account-delete path must drop the account's sessions first. A new path
//! that waits on a member's `sgw_player` row while holding the organization
//! breaks this and must take the row first, as in (2).
//!
//! The Team and Command vaults (bank-vault BV-07) are such a path. A
//! withdrawal inserts into `sgw_inventory`, whose foreign key takes `KEY
//! SHARE` on the mover's `sgw_player` row, and a GM or account delete of an
//! online mover would hold that row while waiting on its organizations. So
//! every vault action takes `FOR KEY SHARE` on the actor's `sgw_player` row
//! **before** [`lock_org`] (`inventory::org_vault::access::lock_actor`):
//! the delete then waits on the action, or the action on the delete, with
//! no cycle. `KEY SHARE` does not conflict with the `NO KEY UPDATE` a cash
//! change takes. Two rules follow (agreed with the organizations campaign
//! for BV-07 and BV-08):
//!
//! - a cash change (BV-08) takes the `KEY SHARE` first and makes its plain
//!   `UPDATE` of the actor's row after [`lock_org`], never a `SELECT … FOR
//!   UPDATE`;
//! - a vault transaction that touches another member's `sgw_player` row
//!   takes every player row it needs, `FOR KEY SHARE` in `player_id`
//!   order, before [`lock_org`].
//!
//! A vault *move* takes the mover's per-player advisory locks, `(player, 0)`
//! and the carried container's, before both (`move_::org::rows::move_locks`).
//! That is the order vendor purchases and trades use, advisory lock before
//! the `sgw_player` row; the reverse, `KEY SHARE` held while waiting on
//! `(player, 0)`, deadlocks against them. Nothing that holds an organization
//! lock waits on a per-player advisory lock, and the personal move path,
//! which holds those, never takes an organization lock.
//!
//! All of this assumes READ COMMITTED, the server's isolation level: a
//! statement after the lock wait sees what the previous lock holder
//! committed. Under REPEATABLE READ, locking a row changed since the
//! snapshot fails with a serialization error instead.

use cimmeria_entity::organization::{OrgPermission, OrgRank, OrgType, UnknownValue};
use sqlx::{Postgres, Transaction};

pub use super::handlers::broadcast::broadcast_to_org;

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

/// What one actor may do in one organization, read under ORG-LOCK in one
/// transaction.
///
/// Only [`member_access_locked`] (a member) and [`OrgAccess::system`] (a
/// GM or server path) build one: the private fields rule out a struct
/// literal. It records the transaction it was read in, and every
/// persistence mutation refuses it in any other transaction, so an
/// authorization cannot outlive the lock it was read under.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrgAccess {
    org_id: i32,
    org_type: OrgType,
    rank: OrgRank,
    permissions: OrgPermission,
    /// The member this access belongs to; `None` for a system actor.
    player_id: Option<i32>,
    /// `txid_current()` of the transaction that read it.
    tx_id: i64,
}

/// Who a system [`OrgAccess`] acts for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemActor<'a> {
    /// A GM command (`.org_join`, `.org_disband`, ...): logged as
    /// `org.gm_access` with the GM's identity. The command's own result is
    /// its `org.gm_action` row, written by the handler.
    Gm {
        account_id: Option<i32>,
        player_id: Option<i32>,
        command: &'a str,
    },
    /// The server itself (a sweep, a test fixture): logged as
    /// `system_action` with `source`.
    Server { source: &'static str },
}

impl OrgAccess {
    pub fn org_id(&self) -> i32 {
        self.org_id
    }

    pub fn org_type(&self) -> OrgType {
        self.org_type
    }

    /// The actor's rank; `Leader` for a system actor.
    pub fn rank(&self) -> OrgRank {
        self.rank
    }

    /// The actor's rank's mask; every bit for a system actor.
    pub fn permissions(&self) -> OrgPermission {
        self.permissions
    }

    /// The acting member; `None` for a system actor.
    pub fn player_id(&self) -> Option<i32> {
        self.player_id
    }

    /// `true` for [`OrgAccess::system`].
    pub fn is_system(&self) -> bool {
        self.player_id.is_none()
    }

    pub(super) fn tx_id(&self) -> i64 {
        self.tx_id
    }

    /// Lock the organization and act on it with no member's authority: a
    /// GM command or a server path. Holds `Leader` rank and every
    /// permission bit, so the caller must have authorized the GM itself
    /// (D-ORG13). Logs one INFO `org.gm_access` (a GM) or `system_action`
    /// (the server) with the organization and the actor.
    ///
    /// `Ok(None)` when there is no such organization (WARN `system_access`,
    /// `reason = no_such_org`).
    pub async fn system(
        tx: &mut Transaction<'_, Postgres>,
        org_id: i32,
        actor: SystemActor<'_>,
    ) -> Result<Option<OrgAccess>, sqlx::Error> {
        let Some((header, tx_id)) = lock_org_with_tx(tx, org_id).await? else {
            tracing::warn!(
                target: "org",
                event = "system_access",
                org_id,
                reason = "no_such_org",
                "Organization lookup missed"
            );
            return Ok(None);
        };
        match actor {
            SystemActor::Gm {
                account_id,
                player_id,
                command,
            } => tracing::info!(
                target: "org",
                event = "org.gm_access",
                org_id,
                org_type = header.org_type.name(),
                account_id,
                player_id,
                command,
                "GM acting on an organization"
            ),
            SystemActor::Server { source } => tracing::info!(
                target: "org",
                event = "system_action",
                org_id,
                org_type = header.org_type.name(),
                source,
                "Server acting on an organization"
            ),
        }
        Ok(Some(OrgAccess {
            org_id,
            org_type: header.org_type,
            rank: OrgRank::LEADER,
            permissions: OrgPermission::ALL,
            player_id: None,
            tx_id,
        }))
    }
}

/// Lock the organization row (`SELECT … FOR UPDATE`) and return it.
///
/// ORG-LOCK: this is the transaction's first row lock. The only rows that
/// may be locked before it are members' `sgw_player` rows, taken
/// `FOR KEY SHARE` in `player_id` order by a path that will wait on them
/// (the Bank's vault withdraw and cash update; module doc, "Lock order"
/// (2)); the creation advisory locks, which are not row locks, come before
/// both.
///
/// `Ok(None)` means there is no such organization (never created, or
/// disbanded); the caller rejects. Taking the lock twice in one transaction
/// is harmless, which is why every mutation in [`super::persistence`] takes
/// it again itself.
///
/// A miss logs WARN `event = lock_org`, `reason = no_such_org` on `org`.
pub async fn lock_org(
    tx: &mut Transaction<'_, Postgres>,
    org_id: i32,
) -> Result<Option<OrgHeader>, sqlx::Error> {
    let header = lock_org_quiet(tx, org_id).await?;
    if header.is_none() {
        tracing::warn!(
            target: "org",
            event = "lock_org",
            org_id,
            reason = "no_such_org",
            "Organization lookup missed"
        );
    }
    Ok(header)
}

/// [`lock_org`] without the miss log, for callers that log the miss
/// themselves (the persistence layer's typed refusals), so one miss is one
/// WARN.
pub(super) async fn lock_org_quiet(
    tx: &mut Transaction<'_, Postgres>,
    org_id: i32,
) -> Result<Option<OrgHeader>, sqlx::Error> {
    Ok(lock_org_with_tx(tx, org_id)
        .await?
        .map(|(header, _)| header))
}

/// [`lock_org_quiet`], also returning the transaction's `txid_current()`,
/// which [`OrgAccess`] records and the mutations compare.
pub(super) async fn lock_org_with_tx(
    tx: &mut Transaction<'_, Postgres>,
    org_id: i32,
) -> Result<Option<(OrgHeader, i64)>, sqlx::Error> {
    let row: Option<(i32, i16, String, String, i64, i64, i64)> = sqlx::query_as(
        "SELECT org_id, org_type, name, motd, cash, experience, txid_current() \
         FROM sgw_organizations WHERE org_id = $1 FOR UPDATE",
    )
    .bind(org_id)
    .fetch_optional(&mut **tx)
    .await?;
    row.map(|(org_id, org_type, name, motd, cash, experience, tx_id)| {
        Ok((
            OrgHeader {
                org_id,
                org_type: org_type_from_db(org_type)?,
                name,
                motd,
                cash,
                experience,
            },
            tx_id,
        ))
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
///
/// A miss logs WARN `event = member_access_locked` with `reason` =
/// `no_such_org` or `not_a_member`; a hit logs DEBUG with the rank and mask.
pub async fn member_access_locked(
    tx: &mut Transaction<'_, Postgres>,
    org_id: i32,
    player_id: i32,
) -> Result<Option<OrgAccess>, sqlx::Error> {
    let miss = |reason: &'static str| {
        tracing::warn!(
            target: "org",
            event = "member_access_locked",
            org_id,
            player_id,
            reason,
            "Organization access lookup missed"
        );
    };
    let Some((_, tx_id)) = lock_org_with_tx(tx, org_id).await? else {
        miss("no_such_org");
        return Ok(None);
    };
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
    let Some((org_type, rank, permissions)) = row else {
        miss("not_a_member");
        return Ok(None);
    };
    let access = OrgAccess {
        org_id,
        org_type: org_type_from_db(org_type)?,
        rank: rank_from_db(rank)?,
        permissions: permissions_from_db(permissions),
        player_id: Some(player_id),
        tx_id,
    };
    tracing::debug!(
        target: "org",
        event = "member_access_locked",
        org_id,
        player_id,
        rank = access.rank.as_u8(),
        permissions = access.permissions.bits(),
        "Organization access read under the lock"
    );
    Ok(Some(access))
}

/// `true` when the organization's vault holds no items
/// (`sgw_organization_vault_items`) and its treasury no cash (D-ORG20,
/// D-BV13). Every voluntary disband checks it under ORG-LOCK and refuses
/// when it is `false`.
///
/// It runs `org_vault_is_empty_sql(org_id)` (`db/sgw/_functions.sql`), the
/// function the member-delete trigger calls when a character delete removes
/// the last member, rather than repeating its query, so the two cannot
/// disagree. The caller holds the organization lock, which every vault and
/// cash write takes first, so the answer holds until it commits.
pub async fn org_vault_is_empty(
    tx: &mut Transaction<'_, Postgres>,
    org_id: i32,
) -> Result<bool, sqlx::Error> {
    #[cfg(test)]
    if let Ok(empty) = VAULT_EMPTY_OVERRIDE.try_with(|v| *v) {
        return Ok(empty);
    }
    sqlx::query_scalar("SELECT org_vault_is_empty_sql($1)")
        .bind(org_id)
        .fetch_one(&mut **tx)
        .await
}

#[cfg(test)]
tokio::task_local! {
    /// Test seam: a test scopes `false` to drive the `vault_not_empty`
    /// refusal (ORG-06) without staging vault rows. Task-local, so parallel
    /// tests never see each other's value. The real predicate is guarded by
    /// the Bank campaign's live-DB tests on real vault rows (BV-07).
    pub(crate) static VAULT_EMPTY_OVERRIDE: bool;
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
