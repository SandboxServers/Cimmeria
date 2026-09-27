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
    fx.clear_sends();
    fx.expand(0, VaultScope::Command, None).await;
    assert!(
        fx.saw_text(
            0,
            "orgvaultexpand: you are not in a Command. Nothing was charged."
        ),
        "the line names the scope the GM typed"
    );

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

/// The leader leaves while the purchase waits on the organization lock:
/// the transaction holding the lock deletes the leader's member row, and
/// the member-delete trigger promotes the other member. (The Leader rank
/// moves only that way; `org_member_before_update` refuses a direct
/// demotion.) The purchase reads membership and rank only once it has the
/// lock, so it sees the change: `not_a_member`, nothing charged. Fails if
/// they are read before the lock (from the unlocked membership lookup,
/// say).
#[tokio::test]
async fn a_leader_who_leaves_while_the_purchase_waits_buys_nothing() {
    use std::time::{Duration, Instant};

    let pool = require_db_or_skip!();
    let fx = Fx::new(&pool, 7, 2).await;
    let team = fx.org(OrgType::Team, 0, &[1], 500).await;
    fx.online(0);

    let mut gate = pool.begin().await.unwrap();
    let gate_pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&mut *gate)
        .await
        .unwrap();
    sqlx::query("SELECT 1 FROM sgw_organizations WHERE org_id = $1 FOR UPDATE")
        .bind(team)
        .execute(&mut *gate)
        .await
        .unwrap();
    let capture = LogCapture::install();
    let release = async {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            let held: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM pg_stat_activity WHERE $1 = ANY(pg_blocking_pids(pid))",
            )
            .bind(gate_pid)
            .fetch_one(&pool)
            .await
            .unwrap();
            if held >= 1 {
                break;
            }
            assert!(Instant::now() < deadline, "the purchase never parked");
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        sqlx::query("DELETE FROM sgw_organization_members WHERE org_id = $1 AND player_id = $2")
            .bind(team)
            .bind(fx.player(0))
            .execute(&mut *gate)
            .await
            .unwrap();
        gate.commit().await.unwrap();
    };
    tokio::time::timeout(Duration::from_secs(20), async {
        tokio::join!(fx.expand(0, VaultScope::Team, Some(40)), release)
    })
    .await
    .expect("the purchase hung past 20 s");

    assert_eq!(fx.state(team).await, (40, 500));
    one(
        &fx,
        &capture,
        "expand_rejected",
        Level::WARN,
        0,
        &[("reason", "not_a_member"), ("org_id", &team.to_string())],
    );
    let leader: i16 = sqlx::query_scalar(
        "SELECT rank FROM sgw_organization_members WHERE org_id = $1 AND player_id = $2",
    )
    .bind(team)
    .bind(fx.player(1))
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(leader, 8, "the other member was promoted");
    assert!(fx.cash_log(team).await.is_empty());
    fx.teardown().await;
}

/// D-BV33: the entity id the cell named now belongs to **another**
/// character's session (the buyer gated and the id was reused). The
/// purchase itself commits (the leader asked for it), but nothing reaches
/// that other session: neither the buyer's `onBagInfo` nor the line. Each
/// dropped send logs `bank_feedback_send_failed reason=no_client_address`.
/// Fails if the replies are addressed by entity id.
#[tokio::test]
async fn a_recycled_entity_id_receives_nothing() {
    let pool = require_db_or_skip!();
    let fx = Fx::new(&pool, 8, 2).await;
    let team = fx.org(OrgType::Team, 0, &[], 500).await;
    // Character 1's session now plays character 0's old entity id.
    let mut s = test_default_connected_client_state();
    s.active_player_id = Some(fx.player(1));
    s.player_entity_id = Some(fx.entity(0));
    s.listed_online = true;
    fx.connected.lock().unwrap().insert(fx.addr(1), s);
    fx.entity_to_addr
        .lock()
        .unwrap()
        .insert(fx.entity(0), fx.addr(1));

    let capture = LogCapture::install();
    fx.expand(0, VaultScope::Team, Some(40)).await;
    assert_eq!(fx.state(team).await, (50, 400), "the purchase commits");
    assert!(
        fx.typed.filter_to(fx.addr(1)).is_empty(),
        "the other character's session gets nothing"
    );
    let dropped = bank_rows(&capture, "bank_feedback_send_failed");
    assert_eq!(dropped.len(), 2, "{dropped:#?}");
    assert!(dropped
        .iter()
        .all(|d| d.has_field("reason", "no_client_address")));
    fx.teardown().await;
}
