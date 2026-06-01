//! Tests for the DB-less early-return path of `handle_execute_trade`.
//!
//! These run without `DATABASE_URL` set — the no-pool branch is
//! reachable in production only when the server starts in DB-less
//! mode, but the branch must remain a clean early-return with a
//! Cancelled fan-out and a WARN log.

use std::sync::Arc;

use sqlx::PgPool;

use super::make_state;
use crate::base::world_entry::methods::trade::handle_execute_trade;

/// `handle_execute_trade` early-returns and emits Cancelled to both
/// players when `db_pool` is `None`. This path is reachable in
/// production only if the server starts in DB-less mode, but the
/// branch must remain a clean early-return — without it, the
/// `pool.clone()` on `None` would panic instead of cancelling.
///
/// Unit-level guard (no DB needed). The fan-out byte test would be
/// brittle against the witness-helper fan-out shape; the assertion
/// here is the warn log + the no-panic behaviour, which is what the
/// branch promises.
///
/// Revert-verifier: replacing the `None => { ... return; }` arm with
/// `None => panic!(...)` (or removing the early return) crashes the
/// test by panicking inside the trade task.
#[tokio::test]
async fn no_db_pool_cancels_both_without_panic() {
    use crate::test_support::LogCapture;
    let capture = LogCapture::install();

    let entity_a: u32 = 1234;
    let entity_b: u32 = 5678;
    let (transport, e2a, conn) = make_state(entity_a, entity_b);
    let db: Option<Arc<PgPool>> = None;

    // The handler must not panic with db_pool: None — it must early-
    // return after emitting Cancelled to both clients.
    handle_execute_trade(
        entity_a,
        11,
        entity_b,
        22,
        vec![100],
        0,
        vec![200],
        0,
        &db,
        &transport,
        &conn,
        &e2a,
    )
    .await;

    let event = capture
        .find_message(
            tracing::Level::WARN,
            "ExecuteTrade: no DB pool — sending Cancelled to both",
        )
        .expect(
            "handle_execute_trade with db_pool=None MUST log the WARN \
             before sending Cancelled. A regression that drops the warn \
             (or replaces it with the wrong-level log) makes operator \
             diagnosis of DB-less startups invisible.",
        );
    assert!(event.has_field("entity_id", "1234"));
    assert!(event.has_field("partner_entity_id", "5678"));
}

/// Negative-cash rejection is documented as a no-DB-work early return.
/// We exercise it WITHOUT a DB pool here to prove the order of
/// operations: db_pool-None check FIRST (the warn says "no DB pool"),
/// negative-cash check SECOND. If a refactor swaps the order, the
/// log substring on no-pool no-cash trades would change.
///
/// The no-DB no-pool warn must fire FIRST when both conditions hold,
/// because the function takes `db_pool` by reference and reads it
/// before inspecting the cash fields.
#[tokio::test]
async fn no_db_pool_warn_fires_before_negative_cash_warn() {
    use crate::test_support::LogCapture;
    let capture = LogCapture::install();

    let entity_a: u32 = 0xAAAA;
    let entity_b: u32 = 0xBBBB;
    let (transport, e2a, conn) = make_state(entity_a, entity_b);
    let db: Option<Arc<PgPool>> = None;

    handle_execute_trade(
        entity_a,
        11,
        entity_b,
        22,
        vec![],
        -100,
        vec![],
        0,
        &db,
        &transport,
        &conn,
        &e2a,
    )
    .await;

    // Only the no-DB-pool warn must fire — not the negative-cash one.
    assert!(
        capture
            .find_message(tracing::Level::WARN, "ExecuteTrade: no DB pool")
            .is_some(),
        "no-DB-pool warn must fire first when both no-pool and negative-cash hold"
    );
    assert!(
        capture
            .find_message(tracing::Level::WARN, "negative cash in proposal")
            .is_none(),
        "negative-cash warn must NOT fire when the no-DB-pool branch already \
         early-returned. A regression that re-orders the checks would let \
         both fire — fixable but the ordering invariant is load-bearing for \
         the SigNoz dashboard's 'reason' breakdown."
    );
}
