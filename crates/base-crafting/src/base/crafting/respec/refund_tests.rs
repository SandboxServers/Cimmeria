//! The respec refunds what the player paid, never more: disciplines a GM
//! granted for free are cleared but refund nothing, and a second respec
//! does not refund the first one's points again.

use cimmeria_cell_catalog::crafting::shared_crafting_catalog;
use sqlx::PgPool;

use super::tests::{confirm, open, player, snapshot};
use crate::base::crafting::handlers::grant_expertise_in_db;
use crate::base::crafting::spend::spend_in_db;
use crate::base::crafting::test_players::{cleanup, OneSession};
use crate::test_support::{require_db_or_skip, LogCapture};

use super::tests::ENTITY;

async fn spent(pool: &PgPool, player_id: i32) -> i32 {
    sqlx::query_scalar("SELECT applied_science_points_spent FROM sgw_player WHERE player_id = $1")
        .bind(player_id)
        .fetch_one(pool)
        .await
        .expect("read spent")
}

async fn gm_grant(pool: &PgPool, player_id: i32, discipline_id: i32) {
    grant_expertise_in_db(pool, player_id, discipline_id, 10)
        .await
        .expect("GM expertise grant");
}

/// The mint guard: disciplines that only a GM grant gave are cleared by the
/// respec, and the ASP total does not move.
#[tokio::test]
async fn live_db_gm_granted_disciplines_refund_nothing() {
    let pool = require_db_or_skip!();
    let capture = LogCapture::install();
    let (account_id, player_id) = player(&pool, 7, 2, &[], &[], &[5, 1, 1, 1, 1]).await;
    gm_grant(&pool, player_id, 78).await;
    gm_grant(&pool, player_id, 79).await;
    let session = OneSession::new(ENTITY, 55809);

    open(&pool, &session, player_id).await;
    confirm(&pool, &session, player_id).await;

    let after = snapshot(&pool, player_id).await;
    let respec = capture
        .all()
        .into_iter()
        .find(|e| e.target == "crafting" && e.has_field("event", "respec"))
        .expect("respec event");
    cleanup(&pool, account_id, player_id).await;
    assert_eq!(
        after.0,
        Vec::<i32>::new(),
        "the granted disciplines are cleared"
    );
    assert_eq!(after.1, 2, "no point minted");
    assert!(respec.has_field("asp_refund", "0"), "{respec:#?}");
    assert!(respec.has_field("disciplines_cleared", "2"), "{respec:#?}");
}

/// Three disciplines learned with points and one granted by a GM: the
/// respec refunds three. Learning again with GM grants only, a second
/// respec refunds nothing: the first one's points are not paid twice.
#[tokio::test]
async fn live_db_respec_refunds_the_points_spent_and_only_once() {
    let pool = require_db_or_skip!();
    let (account_id, player_id) = player(&pool, 8, 3, &[], &[], &[5, 1, 1, 1, 1]).await;
    let catalog = shared_crafting_catalog(&pool).await.expect("catalog");
    for root in [21, 40, 59] {
        let learned = spend_in_db(&pool, &catalog, player_id, root)
            .await
            .expect("no database error");
        assert!(learned.is_ok(), "learn {root}: {learned:?}");
    }
    gm_grant(&pool, player_id, 78).await;
    let session = OneSession::new(ENTITY, 55810);
    let before = (
        snapshot(&pool, player_id).await.1,
        spent(&pool, player_id).await,
    );

    open(&pool, &session, player_id).await;
    confirm(&pool, &session, player_id).await;
    let first = snapshot(&pool, player_id).await;
    let spent_after_first = spent(&pool, player_id).await;

    gm_grant(&pool, player_id, 79).await;
    open(&pool, &session, player_id).await;
    confirm(&pool, &session, player_id).await;
    let second = snapshot(&pool, player_id).await;
    cleanup(&pool, account_id, player_id).await;

    assert_eq!(before, (0, 3), "three points spent on three disciplines");
    assert_eq!(first.0, Vec::<i32>::new(), "all four disciplines cleared");
    assert_eq!(first.1, 3, "three points back, none for the granted one");
    assert_eq!(spent_after_first, 0, "the respec resets what was spent");
    assert_eq!(second.0, Vec::<i32>::new());
    assert_eq!(second.1, 3, "the second respec refunds nothing");
}
