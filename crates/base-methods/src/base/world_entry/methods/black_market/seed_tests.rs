//! Tests for the boot seed and the reserved system seller (`seed.rs`):
//! the seed data, the id reservation, and the BM-07 read-back that refuses
//! to seed when ids 1 hold another account or character.

use super::*;
use crate::test_support::{require_db_or_skip, LogCapture};
use tracing::Level;

/// The seed data must be internally consistent: positive stacks/prices and
/// a duration tier, with buyout either disabled or >= the start bid.
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
async fn live_db_ensure_system_seller_is_idempotent_and_satisfies_fk() {
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
async fn live_db_seed_auctions_use_system_seller() {
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

    let system_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM sgw_auction WHERE status = $1 AND seller_id = $2")
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

// ── BM-07: the UAT listing set ────────────────────────────────────────────

/// `.bm_seed`'s set covers what the UAT checklist needs: every duration
/// tier, bid-only and buyout listings, a buyout of at most 50 for a new
/// character, a stack, and well-formed prices.
#[test]
fn uat_specs_cover_the_checklist() {
    let specs = uat_specs();
    for tier in UIAuctionTime::ALL {
        assert!(
            specs.iter().any(|s| s.auction_length == tier),
            "no listing on the {tier:?} tier"
        );
    }
    assert!(
        specs.iter().any(|s| s.buyout_price == 0),
        "a bid-only listing"
    );
    assert!(
        specs
            .iter()
            .any(|s| s.buyout_price > 0 && s.buyout_price <= 50),
        "a buyout a new character can afford"
    );
    assert!(specs.iter().any(|s| s.stack_size > 1), "a stack");
    for s in &specs {
        assert!(s.stack_size >= 1 && s.starting_price >= 1, "{s:?}");
        assert!(
            s.buyout_price == 0 || s.buyout_price >= s.starting_price,
            "{s:?}"
        );
    }
}

/// Live-DB: every UAT item is a real `resources.items` row that stacks as
/// far as its listing, and the pistols span five tech-competency tiers, so
/// a search's TC bounds visibly narrow the list.
#[tokio::test]
async fn live_db_uat_specs_name_real_items_across_tech_tiers() {
    let pool = require_db_or_skip!();
    let mut tiers = std::collections::BTreeSet::new();
    for s in uat_specs() {
        let row: Option<(String, i32, i32)> = sqlx::query_as(
            "SELECT name, tech_comp, max_stack_size FROM resources.items WHERE item_id = $1",
        )
        .bind(s.item_def_id)
        .fetch_optional(&pool)
        .await
        .unwrap();
        let (name, tech, max_stack) =
            row.unwrap_or_else(|| panic!("item {} is not in resources.items", s.item_def_id));
        assert!(
            s.stack_size <= max_stack,
            "{name}: stack {} > {max_stack}",
            s.stack_size
        );
        if name == "SI 3 9mm Pistol" {
            tiers.insert(tech);
        }
    }
    assert_eq!(
        tiers.into_iter().collect::<Vec<_>>(),
        vec![1, 5, 10, 15, 20],
        "the pistol's tech tiers"
    );
}

// ── BM-07: the reserved system seller is checked ──────────────────────────

/// The pure read-back: only account 1 = `Black Market` with player 1 =
/// its `Black Market` character passes; each other shape has its reason.
#[test]
fn check_system_seller_accepts_only_the_system_seller() {
    let reason = |r: Result<(), SystemSellerError>| match r {
        Ok(()) => "ok",
        Err(SystemSellerError::Conflict { reason, .. }) => reason,
        Err(SystemSellerError::Db(_)) => "db",
    };
    let bm = SYSTEM_SELLER_NAME;
    assert_eq!(reason(check_system_seller(Some(bm), Some((1, bm)))), "ok");
    assert_eq!(reason(check_system_seller(None, None)), "account_missing");
    assert_eq!(
        reason(check_system_seller(Some("cady"), Some((1, bm)))),
        "account_taken"
    );
    assert_eq!(
        reason(check_system_seller(Some(bm), None)),
        "player_missing"
    );
    assert_eq!(
        reason(check_system_seller(Some(bm), Some((7, bm)))),
        "player_taken"
    );
    assert_eq!(
        reason(check_system_seller(Some(bm), Some((1, "Gerger")))),
        "player_taken"
    );
}

/// Sentinels for the squatter fixtures: the Black Market's `0x7000_Axxx`
/// block, past every range `tests/mod.rs` lists.
const SQUATTER_ACCOUNT: i32 = 0x7000_AB00;
const SQUATTER_PLAYER: i32 = 0x7000_AB01;
const SQUATTER_NAME: &str = "bm07-squatter";

/// Clear ids 1 (the system seller and its listings) so a test can put
/// something else there. The next `ensure_system_seller` recreates them.
async fn free_reserved_ids(pool: &PgPool) {
    for sql in [
        "DELETE FROM sgw_auction WHERE seller_id = $1",
        "DELETE FROM sgw_player WHERE player_id = $1",
        "DELETE FROM account WHERE account_id = $1",
    ] {
        sqlx::query(sql)
            .bind(SYSTEM_SELLER_ID)
            .execute(pool)
            .await
            .unwrap();
    }
}

async fn insert_account(pool: &PgPool, account_id: i32, name: &str) {
    sqlx::query(
        "INSERT INTO account (account_id, account_name, password, enabled) \
         VALUES ($1, $2, '', false)",
    )
    .bind(account_id)
    .bind(name)
    .execute(pool)
    .await
    .unwrap();
}

async fn insert_player(pool: &PgPool, account_id: i32, player_id: i32, name: &str) {
    sqlx::query(
        "INSERT INTO sgw_player (\
            account_id, player_id, level, alignment, archetype, gender, \
            player_name, extra_name, world_location, bodyset, \
            pos_x, pos_y, pos_z, skin_color_id, bandolier_slot\
         ) VALUES ($1, $2, 1, 0, 1, 1, $3, '', 'CombatSim', \
                   'BS_HumanMale.BS_HumanMale', 0.0, 0.0, 0.0, 0, 0)",
    )
    .bind(account_id)
    .bind(player_id)
    .bind(name)
    .execute(pool)
    .await
    .unwrap();
}

async fn system_listings(pool: &PgPool) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM sgw_auction WHERE seller_id = $1")
        .bind(SYSTEM_SELLER_ID)
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn cleanup_squatters(pool: &PgPool) {
    for (sql, id) in [
        (
            "DELETE FROM sgw_player WHERE player_id = $1",
            SQUATTER_PLAYER,
        ),
        (
            "DELETE FROM account WHERE account_id = $1",
            SQUATTER_ACCOUNT,
        ),
    ] {
        sqlx::query(sql).bind(id).execute(pool).await.unwrap();
    }
    sqlx::query("DELETE FROM sgw_player WHERE player_id = $1 AND player_name = $2")
        .bind(SYSTEM_SELLER_ID)
        .bind(SQUATTER_NAME)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM account WHERE account_id = $1 AND account_name = $2")
        .bind(SYSTEM_ACCOUNT_ID)
        .bind(SQUATTER_NAME)
        .execute(pool)
        .await
        .unwrap();
}

/// Live-DB, BM-07: account 1 belongs to someone else. The boot seed lists
/// nothing, logs `bm.seed_refused reason=account_taken` at ERROR, and does
/// not create a `Black Market` character on that account.
///
/// Bug shape: the pre-BM-07 `ensure_system_seller` inserted with `ON
/// CONFLICT DO NOTHING` and never looked back, so the seed listed three
/// auctions whose sales pay account 1's owner.
#[tokio::test]
async fn live_db_seed_refuses_when_account_1_is_another_account() {
    let pool = require_db_or_skip!();
    free_reserved_ids(&pool).await;
    insert_account(&pool, SYSTEM_ACCOUNT_ID, SQUATTER_NAME).await;

    let capture = LogCapture::install();
    seed_active_auctions(&pool).await;

    let refused = capture.find_event(Level::ERROR, "BM seed refused", "account_taken");
    let listed = system_listings(&pool).await;
    let player: Option<i32> =
        sqlx::query_scalar("SELECT player_id FROM sgw_player WHERE player_id = $1")
            .bind(SYSTEM_SELLER_ID)
            .fetch_optional(&pool)
            .await
            .unwrap();
    cleanup_squatters(&pool).await;

    let refused = refused.expect("ERROR bm.seed_refused reason=account_taken");
    assert!(refused.has_field("event", "bm.seed_refused"));
    assert_eq!(listed, 0, "nothing may be listed for another account");
    assert_eq!(player, None, "no Black Market character on another account");
}

/// Live-DB, BM-07: account 1 is the Black Market's but player 1 is another
/// account's character. Refused as `player_taken`, nothing listed.
#[tokio::test]
async fn live_db_seed_refuses_when_player_1_is_another_character() {
    let pool = require_db_or_skip!();
    free_reserved_ids(&pool).await;
    cleanup_squatters(&pool).await;
    insert_account(&pool, SYSTEM_ACCOUNT_ID, SYSTEM_SELLER_NAME).await;
    insert_account(&pool, SQUATTER_ACCOUNT, SQUATTER_NAME).await;
    insert_player(&pool, SQUATTER_ACCOUNT, SYSTEM_SELLER_ID, SQUATTER_NAME).await;

    let capture = LogCapture::install();
    seed_active_auctions(&pool).await;

    let refused = capture.find_event(Level::ERROR, "BM seed refused", "player_taken");
    let listed = system_listings(&pool).await;
    cleanup_squatters(&pool).await;

    assert!(
        refused.is_some(),
        "ERROR bm.seed_refused reason=player_taken"
    );
    assert_eq!(listed, 0);
}

/// Live-DB, BM-07: another character already has the name `Black Market`,
/// so player 1 cannot be created (the name is unique). Refused as
/// `player_missing`, nothing listed.
#[tokio::test]
async fn live_db_seed_refuses_when_the_name_is_taken_elsewhere() {
    let pool = require_db_or_skip!();
    free_reserved_ids(&pool).await;
    cleanup_squatters(&pool).await;
    insert_account(&pool, SQUATTER_ACCOUNT, SQUATTER_NAME).await;
    insert_player(&pool, SQUATTER_ACCOUNT, SQUATTER_PLAYER, SYSTEM_SELLER_NAME).await;

    let capture = LogCapture::install();
    seed_active_auctions(&pool).await;

    let refused = capture.find_event(Level::ERROR, "BM seed refused", "player_missing");
    let listed = system_listings(&pool).await;
    cleanup_squatters(&pool).await;
    // Account 1 was created by this run and is the real system account.
    sqlx::query("DELETE FROM account WHERE account_id = $1")
        .bind(SYSTEM_ACCOUNT_ID)
        .execute(&pool)
        .await
        .unwrap();

    assert!(
        refused.is_some(),
        "ERROR bm.seed_refused reason=player_missing"
    );
    assert_eq!(listed, 0);
}

/// BM-07 authority review: a listing is a seed listing (minted on sale, no
/// escrow row) only when it names no item instance. A real player 1's
/// listing, on a database where ids 1 hold a character, settles through
/// escrow like anyone's. Fails if the old `|| seller_id == 1` comes back.
#[test]
fn only_an_instance_free_listing_is_a_seed_listing() {
    use super::super::escrow::is_seed_listing;
    use super::super::types::AuctionRow;
    let row = |seller_id, item_id| AuctionRow {
        sequence_id: 1,
        seller_id,
        item_id,
        item_def_id: 55,
        stack_size: 1,
        durability: 0,
        charges: 0,
        starting_price: 1,
        buyout_price: 0,
        current_bid: 0,
        current_bidder: None,
        auction_length: 5,
        created_at: 0,
        expires_at: 0,
        status: 0,
    };
    assert!(is_seed_listing(&row(SYSTEM_SELLER_ID, 0)));
    assert!(!is_seed_listing(&row(SYSTEM_SELLER_ID, 4242)));
    assert!(!is_seed_listing(&row(77, 4242)));
}

/// Live-DB, BM-07: an older boot's system account (created without
/// `enabled`, so enabled) is switched off by the next check, and still
/// passes it.
#[tokio::test]
async fn live_db_an_older_enabled_system_account_is_disabled() {
    let pool = require_db_or_skip!();
    free_reserved_ids(&pool).await;
    sqlx::query("INSERT INTO account (account_id, account_name, password) VALUES ($1, $2, '')")
        .bind(SYSTEM_ACCOUNT_ID)
        .bind(SYSTEM_SELLER_NAME)
        .execute(&pool)
        .await
        .unwrap();

    let checked = ensure_system_seller(&pool).await;
    let enabled: bool = sqlx::query_scalar("SELECT enabled FROM account WHERE account_id = $1")
        .bind(SYSTEM_ACCOUNT_ID)
        .fetch_one(&pool)
        .await
        .unwrap();

    assert!(checked.is_ok(), "{checked:?}");
    assert!(!enabled, "the system account cannot log in");
}
