//! Boot-time seed: ensure the Black Market has a few active listings so the
//! auction house returns data for end-to-end validation even before any player
//! posts one.
//!
//! Seeded listings are *real* `sgw_auction` rows — they are served by the
//! search path and expire via the normal [`super::sweep`] exactly like
//! player-created auctions, so this exercises the live system rather than a
//! special-cased send. The seed is idempotent: it inserts only when the house
//! has no active listings, so it never duplicates and quietly re-seeds an
//! emptied house on the next boot.
//!
//! `seller_id` carries a foreign key to `sgw_player`. Previous code picked the
//! first real player row, which routed bid cash through a real account and
//! minted unsold items into that account's inventory on sweep settlement. The
//! fix uses a **reserved system seller** (account_id 1 / player_id 1) that
//! cannot collide with any real player (the `accounts_account_id_seq` starts at
//! 2 and the `sgw_characters_character_id_seq` starts at 61). The system row is
//! ensured idempotently before listing insertion so `spawn_seed` remains
//! self-contained even on a fresh DB.

use std::sync::Arc;

use sqlx::PgPool;

use super::helpers::now_unix_secs;
use super::types::auction_status;
use super::wire::auction_length_seconds;

/// Reserved account_id for the system seller — below the sequence start of 2,
/// so it can never be allocated to a real account.
pub const SYSTEM_ACCOUNT_ID: i32 = 1;

/// Reserved player_id for the system seller — below the sequence start of 61,
/// so it can never be allocated to a real player. The corresponding `account`
/// row uses [`SYSTEM_ACCOUNT_ID`].
pub const SYSTEM_SELLER_ID: i32 = 1;

/// One seed listing's gameplay fields. The owning `seller_id` is always
/// [`SYSTEM_SELLER_ID`].
struct SeedSpec {
    item_def_id: i32,
    stack_size: i32,
    starting_price: i32,
    buyout_price: i32,
    /// `EBlackMarketTime` duration enum (0..=4) — drives both the client's
    /// displayed time and the server's `expires_at`.
    auction_length: u8,
}

/// The fixed set of seed listings. Pure data, unit-testable without a DB.
fn seed_specs() -> Vec<SeedSpec> {
    vec![
        // Pistol (item def 55) — sidearm; items_event_sets (55, RANGED) → 579.
        SeedSpec {
            item_def_id: 55,
            stack_size: 1,
            starting_price: 50,
            buyout_price: 500,
            auction_length: 4,
        },
        // P90 (item def 21) — SMG; items_event_sets (21, RANGED) → 559.
        SeedSpec {
            item_def_id: 21,
            stack_size: 1,
            starting_price: 120,
            buyout_price: 1000,
            auction_length: 4,
        },
        // Health Slappack TC1 (item def 2893) — stackable consumable, bid-only.
        SeedSpec {
            item_def_id: 2893,
            stack_size: 5,
            starting_price: 30,
            buyout_price: 0, // bid-only (no buyout)
            auction_length: 4,
        },
    ]
}

/// Fire-and-forget boot seed. Mirrors [`super::sweep::spawn_sweep`] — a sync
/// spawner so the caller (base startup) need not be async. Benign if it races
/// the sweep: seeds carry a future `expires_at`, so the sweep's first pass
/// ignores them.
pub fn spawn_seed(pool: Arc<PgPool>) {
    tokio::spawn(async move {
        seed_active_auctions(&pool).await;
    });
}

/// Idempotently ensure the system seller account and player rows exist.
///
/// Uses `INSERT … ON CONFLICT DO NOTHING` for both rows so this is safe to
/// call on every boot regardless of DB state. The system player satisfies
/// every NOT NULL column the schema requires; it carries no inventory,
/// no missions, and no contact lists — it is a pure FK anchor.
async fn ensure_system_seller(pool: &PgPool) -> Result<(), sqlx::Error> {
    // account row first (sgw_player has a FK → account).
    sqlx::query(
        "INSERT INTO account (account_id, account_name, password) \
         VALUES ($1, 'Black Market', '') \
         ON CONFLICT DO NOTHING",
    )
    .bind(SYSTEM_ACCOUNT_ID)
    .execute(pool)
    .await?;

    // sgw_player row — mirrors the minimal column set used in contact-list
    // persistence tests (insert_minimal_player) and BM tests
    // (insert_account_and_player), extended with bandolier_slot which has no
    // DEFAULT in the schema.
    sqlx::query(
        "INSERT INTO sgw_player (\
            account_id, player_id, level, alignment, archetype, gender, \
            player_name, extra_name, world_location, bodyset, \
            pos_x, pos_y, pos_z, skin_color_id, bandolier_slot\
         ) VALUES ($1, $2, 1, 0, 1, 1, 'Black Market', '', 'CombatSim', \
                   'BS_HumanMale.BS_HumanMale', 0.0, 0.0, 0.0, 0, 0) \
         ON CONFLICT DO NOTHING",
    )
    .bind(SYSTEM_ACCOUNT_ID)
    .bind(SYSTEM_SELLER_ID)
    .execute(pool)
    .await?;

    Ok(())
}

/// Insert the seed listings iff the auction house currently has no active
/// listings. Always uses [`SYSTEM_SELLER_ID`] as the seller — never a real
/// player.
async fn seed_active_auctions(pool: &PgPool) {
    let active: i64 = match sqlx::query_scalar("SELECT COUNT(*) FROM sgw_auction WHERE status = $1")
        .bind(auction_status::ACTIVE)
        .fetch_one(pool)
        .await
    {
        Ok(n) => n,
        Err(e) => {
            tracing::warn!("BM seed: active-count query failed: {e}");
            return;
        }
    };
    if active > 0 {
        tracing::debug!(active, "BM seed: auctions already present; skipping");
        return;
    }

    if let Err(e) = ensure_system_seller(pool).await {
        tracing::warn!("BM seed: failed to ensure system seller: {e}");
        return;
    }

    let now = now_unix_secs();
    let mut inserted = 0u32;
    for spec in seed_specs() {
        let expires_at = (now as i64)
            .saturating_add(auction_length_seconds(spec.auction_length))
            .min(i32::MAX as i64) as i32;
        let res = sqlx::query(
            "INSERT INTO sgw_auction \
                (seller_id, item_id, item_def_id, stack_size, durability, charges, \
                 starting_price, buyout_price, current_bid, current_bidder, \
                 auction_length, created_at, expires_at, status) \
             VALUES ($1, 0, $2, $3, 0, 0, $4, $5, 0, NULL, $6, $7, $8, $9)",
        )
        .bind(SYSTEM_SELLER_ID)
        .bind(spec.item_def_id)
        .bind(spec.stack_size)
        .bind(spec.starting_price)
        .bind(spec.buyout_price)
        .bind(spec.auction_length as i16)
        .bind(now)
        .bind(expires_at)
        .bind(auction_status::ACTIVE)
        .execute(pool)
        .await;
        match res {
            Ok(_) => inserted += 1,
            Err(e) => {
                tracing::warn!(
                    item_def_id = spec.item_def_id,
                    "BM seed: insert failed: {e}"
                )
            }
        }
    }
    tracing::info!(
        inserted,
        seller_id = SYSTEM_SELLER_ID,
        "BM seed: seeded Black Market auctions"
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::require_db_or_skip;

    /// The seed data must be internally consistent: positive stacks/prices and
    /// a valid duration enum, with buyout either disabled or >= the start bid.
    #[test]
    fn seed_specs_are_well_formed() {
        let specs = seed_specs();
        assert_eq!(specs.len(), 3, "expected 3 seed listings");
        for s in &specs {
            assert!(s.stack_size >= 1, "stack size must be positive");
            assert!(s.starting_price > 0, "starting price must be positive");
            assert!(
                s.buyout_price == 0 || s.buyout_price >= s.starting_price,
                "buyout is either disabled (0) or at least the starting price"
            );
            assert!(
                s.auction_length <= 4,
                "auction_length is the EBlackMarketTime enum (0..=4)"
            );
        }
    }

    /// Pin the seed to the intended test items so a def-id drift is caught:
    /// Pistol (55), P90 (21), Health Slappack TC1 (2893).
    #[test]
    fn seed_uses_the_three_test_items() {
        let ids: Vec<i32> = seed_specs().iter().map(|s| s.item_def_id).collect();
        assert_eq!(
            ids,
            vec![55, 21, 2893],
            "expected Pistol (55), P90 (21), Health Slappack TC1 (2893)"
        );
    }

    /// SYSTEM_SELLER_ID must be below every real player_id that can be
    /// allocated. The sequence starts at 61, so value 1 cannot collide.
    ///
    /// Bug shape: if SYSTEM_SELLER_ID were raised to a value inside the
    /// sequence range (e.g. 100), a freshly-created player could receive
    /// that id and become the implicit system seller.
    #[test]
    fn system_seller_id_is_below_sequence_start() {
        // sgw_characters_character_id_seq START WITH 61 — any value below that
        // is permanently unreachable by the sequence.
        const SEQUENCE_START: i32 = 61;
        const { assert!(SYSTEM_SELLER_ID < SEQUENCE_START) };
    }

    /// SYSTEM_ACCOUNT_ID must be below every real account_id that can be
    /// allocated. The sequence starts at 2, so value 1 cannot collide.
    #[test]
    fn system_account_id_is_below_sequence_start() {
        // accounts_account_id_seq START WITH 2 — value 1 is unreachable.
        const SEQUENCE_START: i32 = 2;
        const { assert!(SYSTEM_ACCOUNT_ID < SEQUENCE_START) };
    }

    /// Live-DB: `ensure_system_seller` is idempotent and the system player row
    /// satisfies the sgw_auction seller_id FK after it runs.
    ///
    /// Bug shape: if `ensure_system_seller` were removed or the ON CONFLICT
    /// clause dropped, the INSERT would fail on the second boot with a unique-
    /// violation and seeded auctions would have no FK-valid seller.
    #[tokio::test]
    async fn ensure_system_seller_is_idempotent_and_satisfies_fk() {
        let pool = require_db_or_skip!();

        // Two calls must both succeed (idempotent).
        ensure_system_seller(&pool).await.expect("first call");
        ensure_system_seller(&pool)
            .await
            .expect("second call — must be idempotent");

        // The player row must now exist with the expected ids.
        let (account_id, player_id): (i32, i32) =
            sqlx::query_as("SELECT account_id, player_id FROM sgw_player WHERE player_id = $1")
                .bind(SYSTEM_SELLER_ID)
                .fetch_one(&pool)
                .await
                .expect("system seller player row must exist after ensure_system_seller");

        assert_eq!(account_id, SYSTEM_ACCOUNT_ID);
        assert_eq!(player_id, SYSTEM_SELLER_ID);

        // The account row must exist.
        let acc_name: String =
            sqlx::query_scalar("SELECT account_name FROM account WHERE account_id = $1")
                .bind(SYSTEM_ACCOUNT_ID)
                .fetch_one(&pool)
                .await
                .expect("system seller account row must exist");
        assert_eq!(acc_name, "Black Market");
    }

    /// Live-DB: seeded auction rows carry `seller_id = SYSTEM_SELLER_ID`, not a
    /// real player's id.
    ///
    /// Bug shape: the old `SELECT player_id … ORDER BY player_id LIMIT 1` lookup
    /// would set seller_id to the first real player if this assertion is removed
    /// — reverting the fix causes this test to fail whenever any real player
    /// exists in the DB.
    #[tokio::test]
    async fn seed_auctions_use_system_seller() {
        let pool = require_db_or_skip!();

        // Clear any existing active auctions so the idempotency guard doesn't
        // skip the insert.
        sqlx::query("DELETE FROM sgw_auction WHERE seller_id = $1")
            .bind(SYSTEM_SELLER_ID)
            .execute(&pool)
            .await
            .unwrap();

        seed_active_auctions(&pool).await;

        // Every active auction whose seller is the system account must use
        // SYSTEM_SELLER_ID.  A row with a different seller_id would indicate
        // the old real-player lookup is still active.
        let non_system_sellers: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM sgw_auction \
             WHERE status = $1 AND seller_id != $2",
        )
        .bind(auction_status::ACTIVE)
        .bind(SYSTEM_SELLER_ID)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(
            non_system_sellers, 0,
            "all seeded auctions must use SYSTEM_SELLER_ID; \
             found {non_system_sellers} row(s) with a real seller"
        );

        let system_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM sgw_auction WHERE status = $1 AND seller_id = $2",
        )
        .bind(auction_status::ACTIVE)
        .bind(SYSTEM_SELLER_ID)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(
            system_count, 3,
            "expected 3 seeded auctions owned by system seller"
        );

        // Cleanup.
        sqlx::query("DELETE FROM sgw_auction WHERE seller_id = $1")
            .bind(SYSTEM_SELLER_ID)
            .execute(&pool)
            .await
            .unwrap();
    }
}
