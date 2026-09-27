//! Refused purchases: nothing is charged, no log row is written, and each
//! is one WARN `expand_rejected` with its stable `reason` and a line.

use super::super::unreachable_pool;
use super::*;
use crate::test_support::{require_db_or_skip, LogCapture};

/// D-BV28: only the leader may buy. A member (who holds every default
/// bank bit) is refused `not_leader`, with the org fields and the rank.
/// Fails if the leader check is removed.
#[tokio::test]
async fn a_member_who_is_not_the_leader_is_refused() {
    let pool = require_db_or_skip!();
    let fx = Fx::new(&pool, 3, 2).await;
    let team = fx.org(OrgType::Team, 0, &[1], 500).await;
    fx.online(1);

    let capture = LogCapture::install();
    fx.expand(1, VaultScope::Team, Some(40)).await;
    assert_eq!(fx.state(team).await, (40, 500));
    one(
        &fx,
        &capture,
        "expand_rejected",
        Level::WARN,
        1,
        &[
            ("reason", "not_leader"),
            ("org_id", &team.to_string()),
            ("org_type", "team"),
            ("rank", "2"),
            ("vault_slots", "40"),
        ],
    );
    assert!(fx.saw_text(
        1,
        "orgvaultexpand: only the Team's leader may expand its vault. Nothing was charged."
    ));
    assert!(fx.cash_log(team).await.is_empty());
    fx.teardown().await;
}

/// A treasury short of the price buys nothing: `insufficient_org_cash`
/// with the price and the treasury. Fails if the treasury check and the
/// statement's `cash >= price` are removed (the `CHECK` then aborts it as
/// `query_failed`).
#[tokio::test]
async fn a_short_treasury_buys_nothing() {
    let pool = require_db_or_skip!();
    let fx = Fx::new(&pool, 4, 1).await;
    let team = fx.org(OrgType::Team, 0, &[], 99).await;
    fx.online(0);

    let capture = LogCapture::install();
    fx.expand(0, VaultScope::Team, Some(40)).await;
    assert_eq!(fx.state(team).await, (40, 99));
    one(
        &fx,
        &capture,
        "expand_rejected",
        Level::WARN,
        0,
        &[
            ("reason", "insufficient_org_cash"),
            ("price", "100"),
            ("org_cash", "99"),
        ],
    );
    assert!(fx.saw_text(
        0,
        "orgvaultexpand: the next step costs 100; the treasury holds 99. Nothing was charged."
    ));
    assert!(fx.cash_log(team).await.is_empty());
    fx.teardown().await;
}

/// The Command vault is fixed at 100 (D-BV14): its leader is refused
/// `command_vault_fixed` under the lock, with the Command's fields, and the
/// Command's treasury is untouched. Fails if the scope check is removed.
#[tokio::test]
async fn the_command_vault_is_refused() {
    let pool = require_db_or_skip!();
    let fx = Fx::new(&pool, 5, 1).await;
    let command = fx.org(OrgType::Command, 0, &[], 1000).await;
    fx.online(0);

    let capture = LogCapture::install();
    fx.expand(0, VaultScope::Command, Some(40)).await;
    assert_eq!(fx.state(command).await.1, 1000);
    one(
        &fx,
        &capture,
        "expand_rejected",
        Level::WARN,
        0,
        &[
            ("reason", "command_vault_fixed"),
            ("scope", "command"),
            ("org_id", &command.to_string()),
            ("org_type", "command"),
        ],
    );
    assert!(fx.saw_text(
        0,
        "orgvaultexpand: a Command vault is fixed at 100 slots. Nothing was charged."
    ));
    assert!(fx.cash_log(command).await.is_empty());
    fx.teardown().await;
}

/// A GM in no Team is `not_in_org`; a database that cannot be reached is
/// `query_failed` with the error; no database is `db_unavailable`. Each
/// with a line.
#[tokio::test]
async fn no_team_and_database_failures_are_refused() {
    let pool = require_db_or_skip!();
    let fx = Fx::new(&pool, 6, 1).await;
    fx.online(0);

    let capture = LogCapture::install();
    fx.expand(0, VaultScope::Team, None).await;
    one(
        &fx,
        &capture,
        "expand_rejected",
        Level::WARN,
        0,
        &[("reason", "not_in_org"), ("scope", "team")],
    );
    assert!(fx.saw_text(
        0,
        "orgvaultexpand: you are not in a Team. Nothing was charged."
    ));

    let broken = Some(Arc::new(unreachable_pool()));
    for (db_pool, reason) in [(broken, "query_failed"), (None, "db_unavailable")] {
        fx.clear_sends();
        let capture = LogCapture::install();
        let ctx = OrgCtx {
            db_pool: &db_pool,
            ..fx.ctx()
        };
        let req = OrgVaultExpandRequest {
            entity_id: fx.entity(0),
            account_id: Some(fx.account_id as u32),
            player_id: fx.player(0),
            scope: VaultScope::Team,
            from_slots: Some(40),
        };
        handle_org_vault_expand(req, &ctx).await;
        let row = one(
            &fx,
            &capture,
            "expand_rejected",
            Level::WARN,
            0,
            &[("reason", reason)],
        );
        assert_eq!(row.fields.contains_key("error"), reason == "query_failed");
        assert!(fx.saw_text(
            0,
            "orgvaultexpand: the purchase failed. Nothing was charged."
        ));
    }
    drop(capture);
    fx.teardown().await;
}
