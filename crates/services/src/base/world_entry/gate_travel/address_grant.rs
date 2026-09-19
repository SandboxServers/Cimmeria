//! The persistence leg of a content-authored stargate-address grant
//! (`CellToBaseMsg::GrantStargateAddress`, Harset H55).
//!
//! ## Why this sits next to `persist_arrival`
//!
//! Both statements append to the same `integer[]` column under the same
//! ownership predicate, and [`append_known_stargate`] is deliberately the
//! *same shape* as the sub-SELECT inside
//! [`super::persist_arrival`] — minus the origin-world `UNION ALL` branch,
//! which only an arrival has. Two appends written independently is how the
//! two drift, and the column has no uniqueness constraint to catch it:
//! `resources.stargates.stargate_id` is a plain integer and
//! `sgw_player.known_stargates` is a bare array, so a double-append is
//! silent, permanent, and shows the player a duplicated row in their DHD.
//!
//! The set-difference sub-SELECT stays **inline and correlated** rather
//! than lifted into a CTE for the reason `persist_arrival` records: under
//! READ COMMITTED a correlated sub-SELECT in `SET` is re-evaluated against
//! the freshly-locked row on an EvalPlanQual recheck, while a CTE is a
//! stable subplan from the old snapshot and could append a value a
//! concurrent writer had already added.
//!
//! ## This leg is not the authority for the session
//!
//! The cell has already appended to `CellEntity::known_stargates` and sent
//! the client `updateStargateAddress` by the time this runs (see
//! `cell::content::executor::stargate`). Nothing waits on this write, and
//! failing it costs the player the address at next login rather than now —
//! which is why every branch here logs and returns instead of propagating.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use sqlx::PgPool;

use super::super::super::ConnectedClientState;

/// Append `stargate_id` to the player's address book, exactly once.
///
/// Returns the row's post-update `known_stargates` on success, `None` when
/// the `(player_id, account_id)` pair matched no row.
///
/// `account_id` is not redundant with the `player_id` primary key: it is
/// the ownership predicate that makes a wrong `player_id` **miss** rather
/// than land on a stranger's row.
async fn append_known_stargate(
    pool: &PgPool,
    player_id: i32,
    account_id: i32,
    stargate_id: i32,
) -> Result<Option<Vec<i32>>, sqlx::Error> {
    sqlx::query_scalar::<_, Vec<i32>>(
        "UPDATE sgw_player \
            SET known_stargates = known_stargates || ( \
                 SELECT COALESCE(array_agg(DISTINCT t.x ORDER BY t.x), '{}'::integer[]) \
                   FROM (SELECT unnest($1::integer[]) AS x) t \
                  WHERE t.x IS NOT NULL AND NOT (t.x = ANY(known_stargates)) \
                ) \
          WHERE player_id = $2 AND account_id = $3 \
      RETURNING known_stargates",
    )
    // A one-element array rather than a scalar `$1::integer`, so the
    // statement is `persist_arrival`'s append verbatim. It also means a
    // future multi-address grant is a bind change, not a rewrite.
    .bind(vec![stargate_id])
    .bind(player_id)
    .bind(account_id)
    .fetch_optional(pool)
    .await
}

/// Handle `CellToBaseMsg::GrantStargateAddress`.
///
/// Resolves the session's `account_id` the same way gate travel does —
/// `entity_id` → address → `ConnectedClientState` — and fails closed if
/// either lookup misses, because an append keyed on a guessed account is
/// how a grant lands on another character of the same player.
pub(crate) async fn handle_grant_stargate_address(
    entity_id: u32,
    player_id: i32,
    stargate_id: i32,
    db_pool: &Option<Arc<PgPool>>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    let Some(pool) = db_pool else {
        tracing::warn!(
            entity_id,
            player_id,
            stargate_id,
            reason = "grant_address_no_db_pool",
            "GrantStargateAddress: no DB pool -- the address works for this session \
             only and is lost on relog"
        );
        return;
    };

    let addr = {
        let map = entity_to_addr.lock().unwrap();
        map.get(&entity_id).copied()
    };
    let Some(addr) = addr else {
        tracing::warn!(
            entity_id,
            player_id,
            stargate_id,
            reason = "grant_address_no_client_addr",
            "GrantStargateAddress: no client address for the entity -- cannot resolve \
             the owning account, so the grant is not persisted"
        );
        return;
    };

    let account_id = {
        let clients = match connected.lock() {
            Ok(c) => c,
            Err(poisoned) => poisoned.into_inner(),
        };
        clients.get(&addr).map(|c| c.account_id)
    };
    let Some(account_id) = account_id else {
        tracing::warn!(
            entity_id,
            player_id,
            stargate_id,
            %addr,
            reason = "grant_address_no_session",
            "GrantStargateAddress: no connected-client state for the address -- cannot \
             resolve the owning account, so the grant is not persisted"
        );
        return;
    };

    // `as i32` wraps above 2^31; the same cast is on both sides of every
    // other `account_id` bind in this module tree, so a change here would
    // have to be a change everywhere.
    match append_known_stargate(pool.as_ref(), player_id, account_id as i32, stargate_id).await {
        Ok(Some(known)) => {
            tracing::debug!(
                entity_id,
                player_id,
                account_id,
                stargate_id,
                known_count = known.len(),
                "GrantStargateAddress: address book persisted"
            );
        }
        Ok(None) => {
            tracing::warn!(
                entity_id,
                player_id,
                account_id,
                stargate_id,
                rows_affected = 0,
                expected = 1,
                reason = "grant_address_rows_affected_zero",
                "GrantStargateAddress: UPDATE matched 0 rows -- the player/account pair \
                 names no character; the address works for this session only and is \
                 lost on relog"
            );
        }
        Err(e) => {
            tracing::error!(
                entity_id,
                player_id,
                account_id,
                stargate_id,
                reason = "grant_address_persist_failed",
                "GrantStargateAddress: failed to persist the address ({e}) -- it works \
                 for this session only and is lost on relog"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::require_db_or_skip;

    /// Sentinel range for H55, per the Harset packet allocation
    /// (`0x70NN_xxxx`, NN = packet number). Neighbours: `0x7006_0600`+ is
    /// H06's `persist_arrival`, `0x7003_xxxx` H03, `0x7007_xxxx` H07,
    /// `0x7008_xxxx` H08. `0x7055_xxxx` is otherwise unclaimed. Cleanup
    /// deletes by exact id, never by range.
    const TEST_BASE: i32 = 0x7055_0000;

    /// `Harset`'s own gate (`stargates.sql`, world 57). The address this
    /// packet exists to grant.
    const HARSET_GATE: i32 = 3;

    async fn cleanup(pool: &PgPool, account_id: i32, player_id: i32) {
        let _ = sqlx::query("DELETE FROM sgw_player WHERE player_id = $1")
            .bind(player_id)
            .execute(pool)
            .await;
        let _ = sqlx::query("DELETE FROM account WHERE account_id = $1")
            .bind(account_id)
            .execute(pool)
            .await;
    }

    /// Same NOT NULL columns `persist_arrival`'s fixture names: `extra_name`
    /// and `bodyset` are NOT NULL with a `NULL::varchar` default, so they
    /// must be named even though nothing here reads them.
    ///
    /// `Castle_CellBlock` (world 12) is real and **gateless**, so nothing
    /// else in the schema can put an address into this row.
    async fn seed(pool: &PgPool, account_id: i32, player_id: i32, known: &[i32]) {
        cleanup(pool, account_id, player_id).await;
        sqlx::query("INSERT INTO account (account_id, account_name, password) VALUES ($1, $2, '')")
            .bind(account_id)
            .bind(format!("h55-{account_id}"))
            .execute(pool)
            .await
            .expect("insert sentinel account");
        sqlx::query(
            "INSERT INTO sgw_player \
               (account_id, player_id, player_name, extra_name, bodyset, world_location, \
                alignment, archetype, gender, pos_x, pos_y, pos_z, skin_color_id, \
                known_stargates) \
             VALUES ($1, $2, $3, '', 'BS_HumanMale.BS_HumanMale', 'Castle_CellBlock', \
                     1, 1, 1, 0, 0, 0, 0, $4::integer[])",
        )
        .bind(account_id)
        .bind(player_id)
        .bind(format!("H55Pin{player_id}"))
        .bind(known)
        .execute(pool)
        .await
        .expect("insert sentinel player");
    }

    /// Read the column back the way a relog does: `client_ready`'s
    /// `PlayerInitRow` SELECT is what fills `InitPlayerState.known_stargates`
    /// and therefore `CellEntity::known_stargates`, and it reads this column.
    async fn known_of(pool: &PgPool, player_id: i32) -> Vec<i32> {
        sqlx::query_scalar::<_, Vec<i32>>(
            "SELECT known_stargates FROM sgw_player WHERE player_id = $1",
        )
        .bind(player_id)
        .fetch_one(pool)
        .await
        .expect("sentinel player row must exist")
    }

    /// The grant reaches the column, so the next login hands it back to the
    /// cell and the dial gate accepts it. This is the "survives a relog"
    /// acceptance: the row IS what a relog reads.
    #[tokio::test]
    async fn a_granted_address_lands_in_the_column_a_relog_reads() {
        let pool = require_db_or_skip!();
        let (account_id, player_id) = (TEST_BASE, TEST_BASE + 10);
        seed(&pool, account_id, player_id, &[]).await;

        let got = append_known_stargate(&pool, player_id, account_id, HARSET_GATE)
            .await
            .expect("the append must succeed");

        assert_eq!(
            got,
            Some(vec![HARSET_GATE]),
            "the statement must return the post-update book"
        );
        assert_eq!(
            known_of(&pool, player_id).await,
            vec![HARSET_GATE],
            "a fresh character's empty book must now hold exactly the granted address"
        );
        cleanup(&pool, account_id, player_id).await;
    }

    /// The exactly-once contract. Replacing the
    /// `WHERE NOT (t.x = ANY(known_stargates))` filter with a bare
    /// `array_append` (or `|| $1`) duplicates the id here — and nothing in
    /// the schema would ever clean it up, because `known_stargates` is a
    /// bare `integer[]` with no uniqueness constraint.
    #[tokio::test]
    async fn a_second_grant_of_the_same_address_does_not_double_append() {
        let pool = require_db_or_skip!();
        let (account_id, player_id) = (TEST_BASE + 100, TEST_BASE + 110);
        seed(&pool, account_id, player_id, &[]).await;

        for _ in 0..3 {
            append_known_stargate(&pool, player_id, account_id, HARSET_GATE)
                .await
                .expect("the append must succeed");
        }

        assert_eq!(
            known_of(&pool, player_id).await,
            vec![HARSET_GATE],
            "three identical grants must leave exactly one entry -- a retried or \
             re-fired chain must not duplicate the player's DHD row"
        );
        cleanup(&pool, account_id, player_id).await;
    }

    /// An address the player already holds is left alone, and the addresses
    /// they already held are not disturbed or reordered. The client's DHD
    /// list is serialised in array order (`world_data::map_loaded`), so a
    /// statement that rewrote the array would shuffle the player's dial UI.
    #[tokio::test]
    async fn an_address_already_held_leaves_the_book_untouched() {
        let pool = require_db_or_skip!();
        let (account_id, player_id) = (TEST_BASE + 200, TEST_BASE + 210);
        seed(&pool, account_id, player_id, &[10, HARSET_GATE, 2]).await;

        append_known_stargate(&pool, player_id, account_id, HARSET_GATE)
            .await
            .expect("the append must succeed");

        assert_eq!(
            known_of(&pool, player_id).await,
            vec![10, HARSET_GATE, 2],
            "an already-held address must be a no-op, in place and in order"
        );
        cleanup(&pool, account_id, player_id).await;
    }

    /// `account_id` is load-bearing, not decoration: it is the ownership
    /// predicate that makes a wrong caller miss rather than land on a
    /// stranger's row. Dropping `AND account_id = $3` from the statement
    /// makes this grant succeed.
    #[tokio::test]
    async fn a_wrong_account_writes_nothing() {
        let pool = require_db_or_skip!();
        let (account_id, player_id) = (TEST_BASE + 300, TEST_BASE + 310);
        seed(&pool, account_id, player_id, &[]).await;

        let got = append_known_stargate(&pool, player_id, account_id + 1, HARSET_GATE)
            .await
            .expect("the statement must run");

        assert_eq!(got, None, "a mismatched account must match no row");
        assert!(
            known_of(&pool, player_id).await.is_empty(),
            "a mismatched account must not write the victim's address book"
        );
        cleanup(&pool, account_id, player_id).await;
    }
}
