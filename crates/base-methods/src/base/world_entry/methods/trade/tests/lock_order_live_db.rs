//! Concurrency guard (TESTING.md type 5) for the trade's `sgw_player`
//! row-lock order against text-only and cash-only gate mail (#913).
//!
//! Gate mail without an item takes no advisory lock: it locks the
//! recipients' and the sender's `sgw_player` rows `ORDER BY player_id FOR
//! UPDATE` (`mail/send/deliver.rs`). The trade must take the same two
//! rows in the same ascending order whichever player is `p1`, or a mail
//! from the lower id to the higher one deadlocks against a trade whose
//! `p1` is the higher id.
//!
//! Sentinels: accounts and players `0x7000_C600..=0x7000_C603`, after
//! `crafting_bag`'s `0x7000_C500..=0x7000_C58F`.

use std::time::Duration;

use super::*;
use crate::base::world_entry::methods::trade::handle_execute_trade;
use crate::test_support::require_db_or_skip;

const ACCOUNT_LO: i32 = 0x7000_C600;
const ACCOUNT_HI: i32 = 0x7000_C601;
const PLAYER_LO: i32 = 0x7000_C602;
const PLAYER_HI: i32 = 0x7000_C603;
const ENTITY_LO: u32 = 0x7000_C602;
const ENTITY_HI: u32 = 0x7000_C603;

/// The mail's order, held open: it has locked the lower player's row and
/// will lock the higher one's next. A cash trade with `p1` = the higher
/// player starts meanwhile and must wait on the lower row without holding
/// the higher one, so the mail's second lock goes through, the mail
/// commits and the trade then runs.
///
/// Revert-verifier: with the player rows locked `p1` then `p2`, the trade
/// holds the higher row while it waits on the lower, the mail's second
/// lock closes the cycle, and Postgres aborts one side as a deadlock:
/// either the mail's lock fails or the trade is refused and no cash moves.
#[tokio::test]
async fn live_db_trade_locks_player_rows_ascending_against_gate_mail() {
    let pool = require_db_or_skip!();
    let accounts = [ACCOUNT_LO, ACCOUNT_HI];
    let players = [PLAYER_LO, PLAYER_HI];
    cleanup(&pool, &accounts, &players).await;
    insert_account_and_player(&pool, ACCOUNT_LO, PLAYER_LO, 100, "lock-lo").await;
    insert_account_and_player(&pool, ACCOUNT_HI, PLAYER_HI, 100, "lock-hi").await;

    // The mail, first half: the lower player's row.
    let mut mail = pool.begin().await.expect("begin mail");
    let mail_pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&mut *mail)
        .await
        .unwrap();
    sqlx::query("SELECT naquadah FROM sgw_player WHERE player_id = $1 FOR UPDATE")
        .bind(PLAYER_LO)
        .execute(&mut *mail)
        .await
        .expect("mail locks the lower player's row");

    // p1 is the HIGHER player: the order the trade is handed must not be
    // the order it locks in. The higher player pays 30.
    let trade = tokio::spawn({
        let pool = pool.clone();
        async move {
            let transport: Arc<dyn Transport> = Arc::new(TestTransport::new());
            let connected = Arc::new(Mutex::new(HashMap::new()));
            let e2a = Arc::new(Mutex::new(HashMap::new()));
            handle_execute_trade(
                ENTITY_HI,
                PLAYER_HI,
                ENTITY_LO,
                PLAYER_LO,
                vec![],
                30,
                vec![],
                0,
                &Some(Arc::new(pool)),
                &transport,
                &connected,
                &e2a,
            )
            .await;
        }
    });

    // Wait until the trade is blocked behind the mail, so the two really
    // overlap; a trade that never blocks makes this test vacuous.
    let mut blocked = false;
    for _ in 0..200 {
        let waiting: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM pg_stat_activity WHERE $1 = ANY(pg_blocking_pids(pid))",
        )
        .bind(mail_pid)
        .fetch_one(&pool)
        .await
        .unwrap();
        if waiting > 0 {
            blocked = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    assert!(blocked, "the trade never waited on the mail's row lock");

    // The mail, second half: the higher player's row, a write, commit.
    sqlx::query("SELECT naquadah FROM sgw_player WHERE player_id = $1 FOR UPDATE")
        .bind(PLAYER_HI)
        .execute(&mut *mail)
        .await
        .expect("mail locks the higher player's row without a deadlock");
    sqlx::query("UPDATE sgw_player SET naquadah = naquadah - 5 WHERE player_id = $1")
        .bind(PLAYER_LO)
        .execute(&mut *mail)
        .await
        .expect("mail debits postage");
    mail.commit().await.expect("mail commits");

    tokio::time::timeout(Duration::from_secs(20), trade)
        .await
        .expect("the trade finishes once the mail commits")
        .expect("trade task");

    let lo = naquadah_of(&pool, PLAYER_LO).await;
    let hi = naquadah_of(&pool, PLAYER_HI).await;
    cleanup(&pool, &accounts, &players).await;
    assert_eq!(
        (lo, hi),
        (100 - 5 + 30, 70),
        "both the mail and the trade committed"
    );
}
