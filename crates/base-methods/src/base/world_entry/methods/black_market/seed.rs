//! Boot-time seed: ensure the Black Market has a few active listings so the
//! auction house returns data for end-to-end validation even before any player
//! posts one. The GM `.bm_seed` command (BM-07, [`super::gm`]) inserts the
//! wider UAT set through the same insert.
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
//!
//! # The reserved ids are checked (BM-07)
//!
//! The sequences keep ids 1 free, but nothing stops an operator or an import
//! from putting a real account or character there, and every listing the
//! system seller owns is settled as a seed listing: its sale mints the item
//! and its cash goes to player 1. So [`ensure_system_seller`] reads the rows
//! back after its idempotent inserts and refuses unless account 1 is the
//! `Black Market` account and player 1 is its `Black Market` character. A
//! refusal seeds nothing and logs `bm.seed_refused` at ERROR with the
//! `reason` and what the ids hold.

use std::sync::Arc;

use sqlx::{PgConnection, PgPool};

use super::helpers::now_unix_secs;
use super::telemetry::{count_bm_outcome, item_name};
use super::types::auction_status;
use super::wire::auction_length_seconds;
use cimmeria_wire::black_market::UIAuctionTime;

/// Reserved account_id for the system seller — below the sequence start of 2,
/// so it can never be allocated to a real account.
pub const SYSTEM_ACCOUNT_ID: i32 = 1;

/// Reserved player_id for the system seller — below the sequence start of 61,
/// so it can never be allocated to a real player. The corresponding `account`
/// row uses [`SYSTEM_ACCOUNT_ID`].
pub const SYSTEM_SELLER_ID: i32 = 1;

/// The system seller's account name and character name. Both are unique
/// columns, so no real account or character can share them while the
/// system seller exists.
pub const SYSTEM_SELLER_NAME: &str = "Black Market";

/// One seed listing's gameplay fields. The owning `seller_id` is always
/// [`SYSTEM_SELLER_ID`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct SeedSpec {
    pub item_def_id: i32,
    pub stack_size: i32,
    pub starting_price: i32,
    pub buyout_price: i32,
    /// The listing's duration tier; drives `expires_at`, and is stored as
    /// its 1-based `UIAuctionTime` value.
    pub auction_length: UIAuctionTime,
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
            auction_length: UIAuctionTime::Long,
        },
        // P90 (item def 21) — SMG; items_event_sets (21, RANGED) → 559.
        SeedSpec {
            item_def_id: 21,
            stack_size: 1,
            starting_price: 120,
            buyout_price: 1000,
            auction_length: UIAuctionTime::Long,
        },
        // Health Slappack TC1 (item def 2893) — stackable consumable, bid-only.
        SeedSpec {
            item_def_id: 2893,
            stack_size: 5,
            starting_price: 30,
            buyout_price: 0, // bid-only (no buyout)
            auction_length: UIAuctionTime::Long,
        },
    ]
}

/// The UAT listing set `.bm_seed` cycles through (BM-07): the SI 3 9mm
/// Pistol at five tech-competency tiers (1, 5, 10, 15, 20) so a search's
/// TC bounds visibly narrow the list, every duration tier, bid-only and
/// buyout listings, a buyout cheap enough for a new character, and a stack.
/// Every item is a real `resources.items` row (pinned by a live-DB test).
pub(super) fn uat_specs() -> [SeedSpec; 8] {
    use UIAuctionTime::*;
    let spec = |item_def_id, stack_size, starting_price, buyout_price, auction_length| SeedSpec {
        item_def_id,
        stack_size,
        starting_price,
        buyout_price,
        auction_length,
    };
    [
        // SI 3 9mm Pistol, TC 1: a buyout a new character can afford.
        spec(55, 1, 10, 40, VeryLong),
        // SI 3 9mm Pistol, TC 5.
        spec(3236, 1, 150, 1_200, Long),
        // SI 3 9mm Pistol, TC 10: bid-only.
        spec(3238, 1, 400, 0, Medium),
        // SI 3 9mm Pistol, TC 15.
        spec(3240, 1, 800, 4_000, Short),
        // SI 3 9mm Pistol, TC 20: the shortest tier.
        spec(3242, 1, 1_200, 6_000, VeryShort),
        // Health Slappack TC1, a stack of 5: bid-only.
        spec(2893, 5, 30, 0, Long),
        // SGHC 6 SMG (P90 art), TC 1.
        spec(21, 1, 120, 1_000, VeryLong),
        // SGHC 6 SMG, TC 5.
        spec(3127, 1, 300, 2_500, Medium),
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

/// Why the reserved system seller cannot be used.
#[derive(Debug)]
pub enum SystemSellerError {
    /// The database failed.
    Db(sqlx::Error),
    /// The reserved ids hold something else. `reason` is a closed label
    /// (`account_missing`, `account_taken`, `player_missing`,
    /// `player_taken`); `found` says what is there.
    Conflict { reason: &'static str, found: String },
}

impl From<sqlx::Error> for SystemSellerError {
    fn from(e: sqlx::Error) -> Self {
        Self::Db(e)
    }
}

/// Idempotently ensure the system seller account and player rows exist, then
/// check that ids 1 really are the system seller.
///
/// Uses `INSERT … ON CONFLICT DO NOTHING` for both rows so this is safe to
/// call on every boot regardless of DB state. The system player satisfies
/// every NOT NULL column the schema requires; it carries no inventory,
/// no missions, and no contact lists — it is a pure FK anchor. The account
/// is disabled: nobody logs in as the Black Market.
///
/// `ON CONFLICT DO NOTHING` covers every unique key, so an insert that loses
/// to a squatter (another row at id 1, or another row already named
/// `Black Market`) is silent. The read-back after it is what catches that.
pub async fn ensure_system_seller(pool: &PgPool) -> Result<(), SystemSellerError> {
    // account row first (sgw_player has a FK → account).
    sqlx::query(
        "INSERT INTO account (account_id, account_name, password, enabled) \
         VALUES ($1, $2, '', false) \
         ON CONFLICT DO NOTHING",
    )
    .bind(SYSTEM_ACCOUNT_ID)
    .bind(SYSTEM_SELLER_NAME)
    .execute(pool)
    .await?;
    // Boots before BM-07 created the account without `enabled`, which
    // defaults to true. Nobody logs in as the seller that owns every
    // minting listing, so switch an older system account off.
    sqlx::query(
        "UPDATE account SET enabled = false          WHERE account_id = $1 AND account_name = $2 AND enabled",
    )
    .bind(SYSTEM_ACCOUNT_ID)
    .bind(SYSTEM_SELLER_NAME)
    .execute(pool)
    .await?;
    // Check the account before the character goes in: a squatter's account
    // must not gain a `Black Market` character.
    let account: Option<String> =
        sqlx::query_scalar("SELECT account_name FROM account WHERE account_id = $1")
            .bind(SYSTEM_ACCOUNT_ID)
            .fetch_optional(pool)
            .await?;
    check_system_seller(
        account.as_deref(),
        Some((SYSTEM_ACCOUNT_ID, SYSTEM_SELLER_NAME)),
    )?;

    // sgw_player row — mirrors the minimal column set used in contact-list
    // persistence tests (insert_minimal_player) and BM tests
    // (insert_account_and_player), extended with bandolier_slot which has no
    // DEFAULT in the schema.
    sqlx::query(
        "INSERT INTO sgw_player (\
            account_id, player_id, level, alignment, archetype, gender, \
            player_name, extra_name, world_location, bodyset, \
            pos_x, pos_y, pos_z, skin_color_id, bandolier_slot\
         ) VALUES ($1, $2, 1, 0, 1, 1, $3, '', 'CombatSim', \
                   'BS_HumanMale.BS_HumanMale', 0.0, 0.0, 0.0, 0, 0) \
         ON CONFLICT DO NOTHING",
    )
    .bind(SYSTEM_ACCOUNT_ID)
    .bind(SYSTEM_SELLER_ID)
    .bind(SYSTEM_SELLER_NAME)
    .execute(pool)
    .await?;

    let player: Option<(i32, String)> =
        sqlx::query_as("SELECT account_id, player_name FROM sgw_player WHERE player_id = $1")
            .bind(SYSTEM_SELLER_ID)
            .fetch_optional(pool)
            .await?;
    check_system_seller(
        account.as_deref(),
        player.as_ref().map(|(a, n)| (*a, n.as_str())),
    )
}

/// The pure half of [`ensure_system_seller`]'s read-back: account 1's name
/// and player 1's `(account_id, player_name)`, as found.
fn check_system_seller(
    account: Option<&str>,
    player: Option<(i32, &str)>,
) -> Result<(), SystemSellerError> {
    let conflict = |reason, found: String| Err(SystemSellerError::Conflict { reason, found });
    match account {
        None => {
            return conflict(
                "account_missing",
                format!(
                "no account {SYSTEM_ACCOUNT_ID} (another account is named '{SYSTEM_SELLER_NAME}')"
            ),
            )
        }
        Some(name) if name != SYSTEM_SELLER_NAME => {
            return conflict(
                "account_taken",
                format!("account {SYSTEM_ACCOUNT_ID} is '{name}'"),
            )
        }
        Some(_) => {}
    }
    match player {
        None => conflict(
            "player_missing",
            format!(
                "no player {SYSTEM_SELLER_ID} (another character is named '{SYSTEM_SELLER_NAME}')"
            ),
        ),
        Some((account_id, name))
            if account_id != SYSTEM_ACCOUNT_ID || name != SYSTEM_SELLER_NAME =>
        {
            conflict(
                "player_taken",
                format!("player {SYSTEM_SELLER_ID} is '{name}' on account {account_id}"),
            )
        }
        Some(_) => Ok(()),
    }
}

/// Log a refused boot seed (`bm.seed_refused`, ERROR) and count it.
/// `.bm_seed` logs its own refusal with the GM's ids (`bm.gm_rejected`).
fn log_seed_refused(source: &'static str, err: &SystemSellerError) {
    match err {
        SystemSellerError::Conflict { reason, found } => {
            tracing::error!(
                event = "bm.seed_refused",
                reason = *reason,
                source,
                account_id = SYSTEM_ACCOUNT_ID,
                account_name = SYSTEM_SELLER_NAME,
                player_id = SYSTEM_SELLER_ID,
                player_name = SYSTEM_SELLER_NAME,
                found = found.as_str(),
                "BM seed refused: the reserved system seller ids hold another account or \
                 character, so no listing was seeded (a sale would mint items and pay player 1)"
            );
            count_bm_outcome("seed", reason);
        }
        SystemSellerError::Db(e) => {
            tracing::error!(
                event = "bm.seed_refused",
                reason = "internal",
                source,
                error = %e,
                "BM seed refused: checking the system seller failed"
            );
            count_bm_outcome("seed", "internal");
        }
    }
}

/// Insert one system-seller listing and return its `sequence_id`. The one
/// insert the boot seed and `.bm_seed` share; the caller has run
/// [`ensure_system_seller`].
pub(super) async fn insert_system_listing(
    conn: &mut PgConnection,
    spec: &SeedSpec,
    now: i32,
) -> Result<i32, sqlx::Error> {
    let expires_at = (now as i64)
        .saturating_add(auction_length_seconds(spec.auction_length))
        .min(i32::MAX as i64) as i32;
    sqlx::query_scalar(
        "INSERT INTO sgw_auction \
            (seller_id, item_id, item_def_id, stack_size, durability, charges, \
             starting_price, buyout_price, current_bid, current_bidder, \
             auction_length, created_at, expires_at, status) \
         VALUES ($1, 0, $2, $3, 0, 0, $4, $5, 0, NULL, $6, $7, $8, $9) \
         RETURNING sequence_id",
    )
    .bind(SYSTEM_SELLER_ID)
    .bind(spec.item_def_id)
    .bind(spec.stack_size)
    .bind(spec.starting_price)
    .bind(spec.buyout_price)
    .bind(i16::from(spec.auction_length as u8))
    .bind(now)
    .bind(expires_at)
    .bind(auction_status::ACTIVE)
    .fetch_one(conn)
    .await
}

/// Insert the seed listings iff the auction house currently has no active
/// listings. Always uses [`SYSTEM_SELLER_ID`] as the seller — never a real
/// player, and never ids 1 that turn out to hold one (BM-07): the seller is
/// checked before anything else.
async fn seed_active_auctions(pool: &PgPool) {
    if let Err(e) = ensure_system_seller(pool).await {
        log_seed_refused("boot", &e);
        return;
    }

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

    let now = now_unix_secs();
    let mut conn = match pool.acquire().await {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!("BM seed: no connection: {e}");
            return;
        }
    };
    let mut inserted = 0u32;
    for spec in seed_specs() {
        match insert_system_listing(&mut conn, &spec, now).await {
            Ok(_) => inserted += 1,
            Err(e) => {
                tracing::warn!(
                    item_type_id = spec.item_def_id,
                    item_name = item_name(spec.item_def_id),
                    "BM seed: insert failed: {e}"
                )
            }
        }
    }
    tracing::info!(
        inserted,
        seller_id = SYSTEM_SELLER_ID,
        seller_name = SYSTEM_SELLER_NAME,
        "BM seed: seeded Black Market auctions"
    );
}

#[cfg(test)]
#[path = "seed_tests.rs"]
mod tests;
