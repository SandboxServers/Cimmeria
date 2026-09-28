//! Refused transfers: nothing moves, no log row is written, and each is one
//! WARN `org_cash_rejected` with its stable `reason` and a line for the
//! player.

use std::sync::Arc;

use super::*;
use crate::test_support::{require_db_or_skip, LogCapture};

/// A deposit larger than the wallet and a withdrawal larger than the
/// treasury change neither balance and write no log row. Each resends the
/// balance the client had wrong (the wallet, or the treasury), so the
/// window's maximum corrects itself. Fails if the wallet guard
/// (`naquadah >= $2`) or the treasury check is removed: the wallet or the
/// treasury would go negative, or the `CHECK` would turn the refusal into
/// `query_failed`.
#[tokio::test]
async fn live_db_overdrawing_either_side_changes_nothing() {
    let pool = require_db_or_skip!();
    let fx = Fx::new(&pool, 2, 1, 100).await;
    let team = fx.org(OrgType::Team, 0, &[]).await;
    fx.set_org_cash(team, 50).await;
    fx.online(0);

    let capture = LogCapture::install();
    fx.transfer(0, team, CashDir::Deposit(101)).await;
    assert_eq!((fx.wallet(0).await, fx.org_cash(team).await), (100, 50));
    one(
        &fx,
        &capture,
        "org_cash_rejected",
        Level::WARN,
        0,
        &[
            ("reason", "insufficient_player_cash"),
            ("direction", "deposit"),
            ("amount", "101"),
            ("player_cash_before", "100"),
            ("player_cash_after", "100"),
            ("org_cash_before", "50"),
        ],
    );
    let calls = fx.calls_to(0);
    assert_eq!(cash_updates(&calls), vec![100], "the wallet resent");
    assert_eq!(
        lines(&calls),
        vec!["You cannot deposit 101 naquadah: you have 100."]
    );

    drop(capture);
    fx.clear_sends();
    let capture = LogCapture::install();
    fx.transfer(0, team, CashDir::Withdraw(51)).await;
    assert_eq!((fx.wallet(0).await, fx.org_cash(team).await), (100, 50));
    one(
        &fx,
        &capture,
        "org_cash_rejected",
        Level::WARN,
        0,
        &[
            ("reason", "insufficient_org_cash"),
            ("direction", "withdraw"),
            ("amount", "51"),
            ("org_cash_before", "50"),
            ("org_cash_after", "50"),
        ],
    );
    let calls = fx.calls_to(0);
    assert_eq!(
        org_cash_updates(&calls),
        vec![(team, 50)],
        "the treasury resent"
    );
    assert_eq!(
        lines(&calls),
        vec!["You cannot withdraw 51 naquadah: the treasury holds 50."]
    );
    assert!(fx.cash_log(team).await.is_empty());
    fx.teardown().await;
}

/// No balance can wrap. A withdrawal that would take the wallet past
/// `i32::MAX` (including the client's `i32::MIN`, a withdrawal of 2^31) is
/// `player_cash_overflow`; a deposit that would take the treasury past
/// `i64::MAX` is `org_cash_overflow`. Nothing moves. Fails if the wallet's
/// `naquadah::bigint + $2 <= 2147483647` guard is removed (Postgres then
/// raises "integer out of range" and the refusal becomes `query_failed`),
/// or the treasury's `checked_add` and its SQL guard are.
#[tokio::test]
async fn live_db_overflow_on_either_side_is_refused() {
    let pool = require_db_or_skip!();
    let fx = Fx::new(&pool, 3, 1, i32::MAX - 10).await;
    let team = fx.org(OrgType::Team, 0, &[]).await;
    fx.set_org_cash(team, 1_000_000_000_000).await;
    fx.online(0);

    for (wallet, amount) in [(i32::MAX - 10, 11u32), (0, 1u32 << 31)] {
        fx.set_wallet(0, wallet).await;
        fx.clear_sends();
        let capture = LogCapture::install();
        fx.transfer(0, team, CashDir::Withdraw(amount)).await;
        assert_eq!(
            (fx.wallet(0).await, fx.org_cash(team).await),
            (wallet, 1_000_000_000_000),
            "{amount}"
        );
        one(
            &fx,
            &capture,
            "org_cash_rejected",
            Level::WARN,
            0,
            &[
                ("reason", "player_cash_overflow"),
                ("amount", &amount.to_string()),
                ("player_cash_after", &wallet.to_string()),
            ],
        );
        assert_eq!(
            lines(&fx.calls_to(0)),
            vec![format!("You cannot carry {amount} more naquadah.")]
        );
    }

    fx.set_wallet(0, 10).await;
    fx.set_org_cash(team, i64::MAX - 5).await;
    fx.clear_sends();
    let capture = LogCapture::install();
    fx.transfer(0, team, CashDir::Deposit(6)).await;
    assert_eq!(
        (fx.wallet(0).await, fx.org_cash(team).await),
        (10, i64::MAX - 5)
    );
    one(
        &fx,
        &capture,
        "org_cash_rejected",
        Level::WARN,
        0,
        &[
            ("reason", "org_cash_overflow"),
            ("org_cash_before", &(i64::MAX - 5).to_string()),
        ],
    );
    assert_eq!(
        lines(&fx.calls_to(0)),
        vec!["The treasury cannot hold 6 more naquadah."]
    );
    assert!(fx.cash_log(team).await.is_empty());
    fx.teardown().await;
}

/// A character outside the organization, an organization that does not
/// exist, and a character whose row is gone are refused before any write:
/// `not_a_member`, `no_such_org`, `player_missing`, each with a line. The
/// outsider is not sent the treasury. Fails if the membership read is
/// skipped (the outsider's deposit would land).
#[tokio::test]
async fn live_db_outsiders_missing_orgs_and_missing_characters_are_refused() {
    let pool = require_db_or_skip!();
    let fx = Fx::new(&pool, 4, 3, 500).await;
    let team = fx.org(OrgType::Team, 0, &[]).await;
    fx.online(1);
    fx.online(2);

    let capture = LogCapture::install();
    fx.transfer(1, team, CashDir::Deposit(100)).await;
    assert_eq!((fx.wallet(1).await, fx.org_cash(team).await), (500, 0));
    one(
        &fx,
        &capture,
        "org_cash_rejected",
        Level::WARN,
        1,
        &[("reason", "not_a_member"), ("org_id", &team.to_string())],
    );
    let calls = fx.calls_to(1);
    assert_eq!(
        lines(&calls),
        vec!["You are not a member of that organization."]
    );
    assert!(
        org_cash_updates(&calls).is_empty(),
        "no treasury to an outsider"
    );

    drop(capture);
    fx.clear_sends();
    let capture = LogCapture::install();
    // Inside the Team / Command id range, never created.
    let gone = 0x3FFF_FFF0;
    fx.transfer(1, gone, CashDir::Deposit(100)).await;
    assert_eq!(fx.wallet(1).await, 500);
    one(
        &fx,
        &capture,
        "org_cash_rejected",
        Level::WARN,
        1,
        &[("reason", "no_such_org"), ("org_id", &gone.to_string())],
    );
    assert_eq!(
        lines(&fx.calls_to(1)),
        vec!["You are not a member of that organization."],
        "the same line as an outsider's: no existence oracle"
    );

    // A session whose character row was deleted under it.
    sqlx::query("DELETE FROM sgw_player WHERE player_id = $1")
        .bind(fx.player(2))
        .execute(&pool)
        .await
        .unwrap();
    drop(capture);
    fx.clear_sends();
    let capture = LogCapture::install();
    fx.transfer(2, team, CashDir::Deposit(100)).await;
    one(
        &fx,
        &capture,
        "org_cash_rejected",
        Level::WARN,
        2,
        &[("reason", "player_missing")],
    );
    assert_eq!(
        lines(&fx.calls_to(2)),
        vec!["The transfer failed. No naquadah was moved."]
    );
    assert_eq!(fx.org_cash(team).await, 0);
    assert!(fx.cash_log(team).await.is_empty());
    fx.teardown().await;
}

/// A database that cannot be reached is `query_failed` with the error and a
/// line; a forward whose actor has no session is `actor_mismatch` and sends
/// nothing (there is no one to tell). Neither needs a live database.
#[tokio::test]
async fn infrastructure_refusals_are_logged() {
    let typed = Arc::new(TestTransport::new());
    let transport: Arc<dyn Transport> = typed.clone();
    let addr: SocketAddr = "127.0.0.1:43100".parse().unwrap();
    let mut s = test_default_connected_client_state();
    s.account_id = 7;
    s.active_player_id = Some(11);
    s.player_entity_id = Some(21);
    s.listed_online = true;
    let connected = Arc::new(Mutex::new(HashMap::from([(addr, s)])));
    let entity_to_addr = Arc::new(Mutex::new(HashMap::from([(21u32, addr)])));
    let db_pool = Some(Arc::new(
        sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .acquire_timeout(std::time::Duration::from_millis(50))
            .connect_lazy("postgres://nobody:nobody@127.0.0.1:1/none")
            .expect("connect_lazy accepts any well-formed URL"),
    ));
    let ctx = OrgCtx {
        db_pool: &db_pool,
        transport: &transport,
        connected: &connected,
        entity_to_addr: &entity_to_addr,
        cell_tx: &None,
    };

    let capture = LogCapture::install();
    handle_transfer_cash(&ctx, 11, 21, 5, CashDir::Withdraw(40)).await;
    let rows = bank_rows(&capture, "org_cash_rejected");
    assert_eq!(rows.len(), 1, "{rows:#?}");
    for (k, v) in [
        ("reason", "query_failed"),
        ("account_id", "7"),
        ("player_id", "11"),
        ("entity_id", "21"),
        ("org_id", "5"),
        ("direction", "withdraw"),
        ("amount", "40"),
    ] {
        assert!(rows[0].has_field(k, v), "{k}={v}: {:#?}", rows[0]);
    }
    assert!(rows[0].fields.contains_key("error"), "{:#?}", rows[0]);
    let calls: Vec<Call> = typed
        .filter_to(addr)
        .iter()
        .flat_map(|p| decode_bundle(p, 21))
        .collect();
    assert_eq!(
        lines(&calls),
        vec!["The transfer failed. No naquadah was moved."]
    );

    drop(capture);
    typed.clear();
    let capture = LogCapture::install();
    // Entity 22 is nobody's.
    handle_transfer_cash(&ctx, 11, 22, 5, CashDir::Deposit(40)).await;
    let rows = bank_rows(&capture, "org_cash_rejected");
    assert_eq!(rows.len(), 1, "{rows:#?}");
    assert!(
        rows[0].has_field("reason", "actor_mismatch"),
        "{:#?}",
        rows[0]
    );
    assert!(rows[0].has_field("entity_id", "22"), "{:#?}", rows[0]);
    assert!(typed.is_empty(), "no one to tell");
}

/// D-BV33: the entity id the cell named now belongs to **another**
/// character's session (the actor gated and the id was reused). The
/// transfer is refused `actor_mismatch` before any database work, nothing
/// moves, and that session is sent nothing, neither a balance nor a line.
/// Fails if the actor check is dropped and the result is addressed by the
/// entity id.
#[tokio::test]
async fn live_db_a_recycled_entity_id_moves_and_receives_nothing() {
    let pool = require_db_or_skip!();
    let fx = Fx::new(&pool, 6, 2, 500).await;
    let team = fx.org(OrgType::Team, 0, &[1]).await;
    fx.set_org_cash(team, 300).await;
    // Character 1's session now plays character 0's old entity id.
    let mut s = test_default_connected_client_state();
    s.account_id = fx.account_id as u32;
    s.active_player_id = Some(fx.player(1));
    s.player_entity_id = Some(fx.entity(0));
    s.listed_online = true;
    let addr: SocketAddr = "127.0.0.1:43101".parse().unwrap();
    fx.connected.lock().unwrap().insert(addr, s);
    fx.entity_to_addr.lock().unwrap().insert(fx.entity(0), addr);

    let capture = LogCapture::install();
    fx.transfer(0, team, CashDir::Withdraw(100)).await;
    assert_eq!(
        (
            fx.wallet(0).await,
            fx.wallet(1).await,
            fx.org_cash(team).await
        ),
        (500, 500, 300)
    );
    let rows = bank_rows(&capture, "org_cash_rejected");
    assert_eq!(rows.len(), 1, "{rows:#?}");
    assert!(
        rows[0].has_field("reason", "actor_mismatch"),
        "{:#?}",
        rows[0]
    );
    assert!(
        fx.typed.filter_to(addr).is_empty(),
        "the other session gets nothing"
    );
    assert!(fx.cash_log(team).await.is_empty());
    fx.teardown().await;
}
