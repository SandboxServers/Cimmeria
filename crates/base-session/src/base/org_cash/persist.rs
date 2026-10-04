//! The treasury transfer transaction (BV-08) and the cash log row every
//! treasury change writes (BV-08, BV-09).
//!
//! Lock order, as `organization::api` § "Lock order" rules for a cash
//! change (agreed with the organizations campaign):
//!
//! 1. the actor's `sgw_player` row `FOR KEY SHARE`, before any organization
//!    lock, so a character delete holding that row and waiting on its
//!    organizations cannot cross this transaction;
//! 2. [`lock_org`] (the organization row, `FOR UPDATE`), then
//!    [`member_access_locked`] (rank and bits, read under that lock);
//! 3. the wallet write, a plain `UPDATE sgw_player` (`NO KEY UPDATE`,
//!    compatible with every `KEY SHARE`), never `SELECT … FOR UPDATE`;
//! 4. the treasury `UPDATE`, then the log row, then the commit.
//!
//! `KEY SHARE` does not stop another plain writer of `naquadah` (a mail
//! claim, a reward), so the wallet's balance is decided by the guarded
//! `UPDATE` and its `RETURNING`, never by the first read. The treasury is
//! locked from step 2, so its balance there is exact; the `UPDATE`'s guard
//! is a second line behind the Rust checks.

use cimmeria_entity::organization::{CashDir, OrgPermission, OrgRank, OrgType};
use sqlx::{PgPool, Postgres, Transaction};

use super::CashRefusal;
use crate::base::organization::api::{lock_org, member_access_locked};

/// Which way a treasury change went: the cash log's `direction` and the
/// `direction` field of `org_cash_transfer`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CashDirection {
    /// Wallet to treasury.
    Deposit,
    /// Treasury to wallet.
    Withdraw,
    /// Treasury to nothing: a Team vault +10 step (BV-09, D-BV28).
    VaultExpansion,
}

impl CashDirection {
    pub fn as_str(self) -> &'static str {
        match self {
            CashDirection::Deposit => "deposit",
            CashDirection::Withdraw => "withdraw",
            CashDirection::VaultExpansion => "vault_expansion",
        }
    }

    /// The direction and the amount, widened, of a decoded CM 19.
    pub fn of(dir: CashDir) -> (CashDirection, i64) {
        match dir {
            CashDir::Deposit(a) => (CashDirection::Deposit, i64::from(a)),
            CashDir::Withdraw(a) => (CashDirection::Withdraw, i64::from(a)),
        }
    }
}

/// One `sgw_organization_cash_log` row. The table's CHECKs pin the
/// arithmetic between the balances and the amount for each direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CashLogRow {
    pub org_id: i32,
    pub org_type: OrgType,
    pub account_id: i32,
    pub player_id: i32,
    pub rank: OrgRank,
    pub direction: CashDirection,
    pub amount: i64,
    /// The wallet before and after; `None` when it was not touched.
    pub player_cash: Option<(i32, i32)>,
    /// The treasury before and after.
    pub org_cash: (i64, i64),
    /// The Team vault's size before and after (a vault expansion only).
    pub vault_slots: Option<(i16, i16)>,
}

/// Write `row` in the transaction that made the change.
pub async fn insert_cash_log(
    tx: &mut Transaction<'_, Postgres>,
    row: &CashLogRow,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO sgw_organization_cash_log (\
            org_id, org_type, account_id, player_id, rank, direction, amount, \
            player_cash_before, player_cash_after, org_cash_before, org_cash_after, \
            vault_slots_before, vault_slots_after\
         ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)",
    )
    .bind(row.org_id)
    .bind(i16::from(row.org_type.as_u8()))
    .bind(row.account_id)
    .bind(row.player_id)
    .bind(i16::from(row.rank.as_u8()))
    .bind(row.direction.as_str())
    .bind(row.amount)
    .bind(row.player_cash.map(|c| c.0))
    .bind(row.player_cash.map(|c| c.1))
    .bind(row.org_cash.0)
    .bind(row.org_cash.1)
    .bind(row.vault_slots.map(|s| s.0))
    .bind(row.vault_slots.map(|s| s.1))
    .execute(&mut **tx)
    .await?;
    Ok(())
}

/// A committed transfer, with both balances before and after.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transferred {
    pub account_id: i32,
    /// The treasury's organization name, read under the lock (log only).
    pub org_name: String,
    pub org_type: OrgType,
    pub rank: OrgRank,
    pub player_cash_before: i32,
    pub player_cash_after: i32,
    pub org_cash_before: i64,
    pub org_cash_after: i64,
}

/// What a refused transfer had read when it stopped, for the log and the
/// actor's resync. Nothing was written.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Seen {
    pub account_id: Option<i32>,
    /// The organization's name once the row was locked (log only).
    pub org_name: Option<String>,
    pub org_type: Option<OrgType>,
    pub rank: Option<OrgRank>,
    pub permissions: Option<OrgPermission>,
    /// The wallet: the first read, or the re-read after a zero-row wallet
    /// `UPDATE`.
    pub player_cash: Option<i32>,
    /// The treasury under the lock.
    pub org_cash: Option<i64>,
    /// The membership read found the actor.
    pub member: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransferOutcome {
    Done(Transferred),
    Refused(CashRefusal, Seen),
}

/// Move `dir` between `player_id`'s wallet and `org_id`'s treasury in one
/// transaction, or refuse with nothing written. The outer `Err` is a
/// database failure (the transaction rolls back when it drops).
pub async fn transfer_cash(
    pool: &PgPool,
    player_id: i32,
    org_id: i32,
    dir: CashDir,
) -> Result<TransferOutcome, sqlx::Error> {
    let (direction, amount) = CashDirection::of(dir);
    let mut seen = Seen::default();
    let refused = |r: CashRefusal, seen: Seen| Ok(TransferOutcome::Refused(r, seen));
    let mut tx = pool.begin().await?;

    // 1. The actor's row, before any organization lock.
    let player: Option<(i32, i32)> = sqlx::query_as(
        "SELECT account_id, naquadah FROM sgw_player WHERE player_id = $1 FOR KEY SHARE",
    )
    .bind(player_id)
    .fetch_optional(&mut *tx)
    .await?;
    let Some((account_id, wallet_read)) = player else {
        return refused(CashRefusal::PlayerMissing, seen);
    };
    seen.account_id = Some(account_id);
    seen.player_cash = Some(wallet_read);

    // 2. The organization, then the actor's rank and bits under its lock.
    let Some(header) = lock_org(&mut tx, org_id).await? else {
        return refused(CashRefusal::NoSuchOrg, seen);
    };
    seen.org_type = Some(header.org_type);
    seen.org_name = Some(header.name.clone());
    seen.org_cash = Some(header.cash);
    let Some(access) = member_access_locked(&mut tx, org_id, player_id).await? else {
        return refused(CashRefusal::NotAMember, seen);
    };
    seen.member = true;
    seen.rank = Some(access.rank());
    seen.permissions = Some(access.permissions());
    let bit = match direction {
        CashDirection::Withdraw => OrgPermission::WITHDRAW_CASH,
        _ => OrgPermission::DEPOSIT_CASH,
    };
    if !access.permissions().contains(bit) {
        return refused(CashRefusal::NoPermission, seen);
    }
    // The treasury is locked, so these are exact.
    match direction {
        CashDirection::Withdraw if header.cash < amount => {
            return refused(CashRefusal::InsufficientOrgCash, seen);
        }
        CashDirection::Deposit if header.cash.checked_add(amount).is_none() => {
            return refused(CashRefusal::OrgCashOverflow, seen);
        }
        _ => {}
    }

    // 3. The wallet: the guard decides, on the row as it is now.
    let wallet_sql = match direction {
        CashDirection::Withdraw => {
            "UPDATE sgw_player SET naquadah = naquadah + $2 \
             WHERE player_id = $1 AND naquadah::bigint + $2 <= 2147483647 \
             RETURNING naquadah"
        }
        _ => {
            "UPDATE sgw_player SET naquadah = naquadah - $2 \
             WHERE player_id = $1 AND naquadah >= $2 \
             RETURNING naquadah"
        }
    };
    let wallet_after: Option<i32> = sqlx::query_scalar(wallet_sql)
        .bind(player_id)
        .bind(amount)
        .fetch_optional(&mut *tx)
        .await?;
    let Some(wallet_after) = wallet_after else {
        // Re-read only to tell the player what they hold now.
        seen.player_cash =
            sqlx::query_scalar("SELECT naquadah FROM sgw_player WHERE player_id = $1")
                .bind(player_id)
                .fetch_optional(&mut *tx)
                .await?;
        let reason = match direction {
            CashDirection::Withdraw => CashRefusal::PlayerCashOverflow,
            _ => CashRefusal::InsufficientPlayerCash,
        };
        return refused(reason, seen);
    };
    let wallet_before = match direction {
        CashDirection::Withdraw => i64::from(wallet_after) - amount,
        _ => i64::from(wallet_after) + amount,
    };
    // The row held `wallet_before` a statement ago, so it fits.
    let wallet_before = i32::try_from(wallet_before).map_err(|e| sqlx::Error::Decode(e.into()))?;

    // 4. The treasury, behind the checks above.
    let org_sql = match direction {
        CashDirection::Withdraw => {
            "UPDATE sgw_organizations SET cash = cash - $2 \
             WHERE org_id = $1 AND cash >= $2 RETURNING cash"
        }
        _ => {
            "UPDATE sgw_organizations SET cash = cash + $2 \
             WHERE org_id = $1 AND cash <= 9223372036854775807 - $2 RETURNING cash"
        }
    };
    let org_after: Option<i64> = sqlx::query_scalar(org_sql)
        .bind(org_id)
        .bind(amount)
        .fetch_optional(&mut *tx)
        .await?;
    let Some(org_after) = org_after else {
        // Unreachable while the lock holds; the rollback undoes the wallet.
        let reason = match direction {
            CashDirection::Withdraw => CashRefusal::InsufficientOrgCash,
            _ => CashRefusal::OrgCashOverflow,
        };
        return refused(reason, seen);
    };
    let org_before = match direction {
        CashDirection::Withdraw => org_after + amount,
        _ => org_after - amount,
    };

    insert_cash_log(
        &mut tx,
        &CashLogRow {
            org_id,
            org_type: header.org_type,
            account_id,
            player_id,
            rank: access.rank(),
            direction,
            amount,
            player_cash: Some((wallet_before, wallet_after)),
            org_cash: (org_before, org_after),
            vault_slots: None,
        },
    )
    .await?;
    tx.commit().await?;
    Ok(TransferOutcome::Done(Transferred {
        account_id,
        org_name: header.name,
        org_type: header.org_type,
        rank: access.rank(),
        player_cash_before: wallet_before,
        player_cash_after: wallet_after,
        org_cash_before: org_before,
        org_cash_after: org_after,
    }))
}
