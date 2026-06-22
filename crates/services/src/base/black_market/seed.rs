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
//! `seller_id` carries a foreign key to `sgw_player`, so the seed must be owned
//! by an existing player; if none exist yet (fresh DB) the seed is skipped.

use std::sync::Arc;

use sqlx::PgPool;

use super::helpers::now_unix_secs;
use super::types::auction_status;
use super::wire::auction_length_seconds;

/// One seed listing's gameplay fields. The owning `seller_id` is resolved at
/// insert time (it must satisfy the `sgw_player` foreign key).
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

/// Insert the seed listings iff the auction house currently has no active
/// listings. Skips quietly if no player exists to own them.
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

    let seller_id: Option<i32> =
        match sqlx::query_scalar("SELECT player_id FROM sgw_player ORDER BY player_id LIMIT 1")
            .fetch_optional(pool)
            .await
        {
            Ok(opt) => opt,
            Err(e) => {
                tracing::warn!("BM seed: seller lookup failed: {e}");
                return;
            }
        };
    let Some(seller_id) = seller_id else {
        tracing::info!("BM seed: no players exist yet; skipping seed auctions");
        return;
    };

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
        .bind(seller_id)
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
                tracing::warn!(item_def_id = spec.item_def_id, "BM seed: insert failed: {e}")
            }
        }
    }
    tracing::info!(inserted, seller_id, "BM seed: seeded Black Market auctions");
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
