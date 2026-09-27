//! The authorization every Team or Command vault action starts with, in one
//! transaction and in the campaign's lock order (ORG-LOCK, D-ORG04; the
//! exception recorded in `organization::api`):
//!
//! 1. the actor's `sgw_player` row, `FOR KEY SHARE`, before any
//!    organization lock, so a character delete that holds that row and waits
//!    on its organizations cannot deadlock against a vault move whose
//!    `sgw_inventory` insert would wait on it;
//! 2. [`lock_org`] and [`member_access_locked`]: the organization row, then
//!    the actor's rank and permission bits, read under that lock;
//! 3. the vault's size.
//!
//! Only after this does a move take the per-player advisory locks and the
//! item rows.

use cimmeria_base_session::base::organization::api::{member_access_locked, OrgAccess};
use cimmeria_entity::cell_entity::VaultScope;
use cimmeria_entity::inventory::{COMMAND_VAULT_SLOTS, TEAM_VAULT_SLOTS_DEFAULT};
use cimmeria_entity::organization::OrgPermission;
use sqlx::{Postgres, Transaction};

/// The actor of one vault action, read under the organization lock.
#[derive(Debug, Clone, Copy)]
pub(crate) struct OrgVaultActor {
    pub account_id: i32,
    /// The actor's personal vault size, for the `onBagInfo` an open sends.
    pub bank_slots: i16,
    /// Rank and bits, bound to this transaction.
    pub access: OrgAccess,
    /// The size of the vault being used: the Team's `vault_slots`, or 100
    /// for a Command.
    pub vault_slots: i32,
}

/// Why the authorization failed. [`OrgLockMiss::reason`] is the stable
/// `reason` of `org_vault_open_rejected` / `org_move_rejected`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OrgLockMiss {
    /// No `sgw_player` row for the actor.
    PlayerMissing,
    /// The organization is gone (disbanded).
    NoSuchOrg,
    /// The actor is not a member (left or was removed).
    NotAMember,
    /// The organization is not of the vault's type (a Team id for the
    /// Command vault).
    WrongOrgType,
}

impl OrgLockMiss {
    pub(crate) fn reason(self) -> &'static str {
        match self {
            OrgLockMiss::PlayerMissing => "player_missing",
            OrgLockMiss::NoSuchOrg => "no_such_org",
            OrgLockMiss::NotAMember => "not_a_member",
            OrgLockMiss::WrongOrgType => "wrong_org_type",
        }
    }
}

/// Take the locks in order and read the actor's authority over `org_id`'s
/// `scope` vault. The outer `Err` is a database failure; the inner one a
/// refusal.
pub(crate) async fn lock_actor(
    tx: &mut Transaction<'static, Postgres>,
    player_id: i32,
    org_id: i32,
    scope: VaultScope,
) -> Result<Result<OrgVaultActor, OrgLockMiss>, sqlx::Error> {
    let player: Option<(i32, i16)> = sqlx::query_as(
        "SELECT account_id, bank_slots FROM sgw_player WHERE player_id = $1 FOR KEY SHARE",
    )
    .bind(player_id)
    .fetch_optional(&mut **tx)
    .await?;
    let Some((account_id, bank_slots)) = player else {
        return Ok(Err(OrgLockMiss::PlayerMissing));
    };
    // `member_access_locked` takes the organization lock itself, and tells
    // a missing organization from a missing member only in its own log.
    let Some(access) = member_access_locked(tx, org_id, player_id).await? else {
        let exists: Option<i32> =
            sqlx::query_scalar("SELECT org_id FROM sgw_organizations WHERE org_id = $1")
                .bind(org_id)
                .fetch_optional(&mut **tx)
                .await?;
        return Ok(Err(if exists.is_some() {
            OrgLockMiss::NotAMember
        } else {
            OrgLockMiss::NoSuchOrg
        }));
    };
    if Some(access.org_type()) != scope.org_type() {
        return Ok(Err(OrgLockMiss::WrongOrgType));
    }
    let vault_slots = match scope {
        VaultScope::Team => {
            let slots: i16 =
                sqlx::query_scalar("SELECT vault_slots FROM sgw_organizations WHERE org_id = $1")
                    .bind(org_id)
                    .fetch_one(&mut **tx)
                    .await?;
            i32::from(slots)
        }
        _ => COMMAND_VAULT_SLOTS,
    };
    Ok(Ok(OrgVaultActor {
        account_id,
        bank_slots,
        access,
        vault_slots,
    }))
}

/// The actor's Team vault size, for `onBagInfo`: the Team's `vault_slots`,
/// or the default when the actor is in no Team. A plain read (the column
/// only grows).
pub(crate) async fn team_vault_slots(
    tx: &mut Transaction<'static, Postgres>,
    player_id: i32,
) -> Result<i32, sqlx::Error> {
    let slots: Option<i16> = sqlx::query_scalar(
        "SELECT o.vault_slots FROM sgw_organization_members m \
         JOIN sgw_organizations o ON o.org_id = m.org_id \
         WHERE m.player_id = $1 AND m.org_type = 1",
    )
    .bind(player_id)
    .fetch_optional(&mut **tx)
    .await?;
    Ok(slots.map_or(TEAM_VAULT_SLOTS_DEFAULT, i32::from))
}

/// The bank bit a vault action needs, and its name for the `perm` field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BankBit {
    Deposit,
    Withdraw,
}

impl BankBit {
    pub(crate) fn permission(self) -> OrgPermission {
        match self {
            BankBit::Deposit => OrgPermission::DEPOSIT_BANK,
            BankBit::Withdraw => OrgPermission::WITHDRAW_BANK,
        }
    }

    /// The `perm` field: the `EOrganizationPermission` name.
    pub(crate) fn name(self) -> &'static str {
        match self {
            BankBit::Deposit => "DepositBank",
            BankBit::Withdraw => "WithdrawBank",
        }
    }
}
