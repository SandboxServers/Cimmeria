//! The container smoke tests' login probe (`login-probe`, issue #1291)
//! against in-process servers: proof that each check passes on a healthy
//! server and fails on the breakage it exists to catch.
//!
//! - Two tests need no database and run in CI. Developer-mode auth without
//!   one accepts every password, the misconfiguration the wrong-password
//!   check must reject. Non-developer auth without one rejects every login
//!   as a database failure, which the probe must not mistake for a
//!   wrong-password rejection.
//! - The `live_db` tests start a full `Orchestrator` against the seeded
//!   database and run every check, including the Mercury handshake. Like
//!   the other live-DB modules here they skip without `DATABASE_URL` and
//!   are not run in CI (the live-DB job runs `--lib` only):
//!
//! ```text
//! DATABASE_URL=... cargo test -p cimmeria-wireclient --test it login_probe -- --test-threads=1
//! ```

use std::net::SocketAddr;

use cimmeria_services::auth::ShardInfo;
use cimmeria_wireclient::login_probe::{self, ProbeConfig, ProbeFailure};

use crate::support::{live_db_pool_or_skip, start_auth, start_server, SHARD};

/// Seeded account the container smoke logs in as
/// (`db/sgw/Accounts/Seed/account.sql`; password "test").
const SEEDED_USER: &str = "test";
const SEEDED_PASSWORD: &str = "test";

/// What `cimmeria_auth`'s `handle_user_auth` answers when it has no
/// database and is not in developer mode.
const DB_FAILURE_REASON: &str = "A request to the database server failed.";

fn probe_config(auth_url: String) -> ProbeConfig {
    ProbeConfig {
        auth_url,
        user: SEEDED_USER.into(),
        password: SEEDED_PASSWORD.into(),
        shard: SHARD.into(),
        expect_base: None,
        expect_account_id: None,
        handshake: true,
    }
}

fn shard() -> ShardInfo {
    ShardInfo {
        name: SHARD.into(),
        host: "127.0.0.1".into(),
        port: 32832,
        protected: false,
    }
}

#[tokio::test]
async fn login_probe_fails_when_auth_accepts_any_password() {
    let (mut auth, port) = start_auth(true, shard()).await;

    let err = login_probe::run(&probe_config(format!("http://127.0.0.1:{port}")))
        .await
        .unwrap_err();
    // Developer mode without a database answers every login as account 1.
    assert!(
        matches!(err, ProbeFailure::WrongPasswordAccepted { account_id: 1 }),
        "a server that accepts any password must fail the probe at the wrong-password step, got: {err}"
    );

    auth.stop().await;
}

/// A database failure also rejects the wrong password, but for the wrong
/// reason. The probe must fail on it rather than count it as "wrong
/// password rejected"; this fails if the reason check is removed.
#[tokio::test]
async fn login_probe_fails_when_auth_rejects_for_a_database_failure() {
    let (mut auth, port) = start_auth(false, shard()).await;

    let err = login_probe::run(&probe_config(format!("http://127.0.0.1:{port}")))
        .await
        .unwrap_err();
    match err {
        ProbeFailure::WrongPasswordOtherReason(reason) => {
            assert_eq!(reason, DB_FAILURE_REASON);
        }
        other => panic!("expected ProbeFailure::WrongPasswordOtherReason, got: {other}"),
    }

    auth.stop().await;
}

/// The seeded `test` account's id, read back rather than hard-coded.
async fn seeded_account_id(pool: &sqlx::PgPool) -> u32 {
    let id: i32 = sqlx::query_scalar("SELECT account_id FROM account WHERE account_name = $1")
        .bind(SEEDED_USER)
        .fetch_one(pool)
        .await
        .expect("seeded account 'test' must exist");
    u32::try_from(id).expect("account_id is positive")
}

#[tokio::test]
async fn login_probe_live_db_passes_every_check_against_a_full_server() {
    let pool = match live_db_pool_or_skip().await {
        Some(p) => p,
        None => return,
    };
    let account_id = seeded_account_id(&pool).await;
    let url = std::env::var("DATABASE_URL").expect("checked by live_db_pool_or_skip");
    let server = start_server(&url).await;
    let base_port = server.base_port;
    let expected_base: SocketAddr = format!("127.0.0.1:{base_port}").parse().unwrap();

    let cfg = ProbeConfig {
        expect_base: Some(expected_base),
        expect_account_id: Some(account_id),
        ..probe_config(server.auth_url.clone())
    };
    let report = login_probe::run(&cfg)
        .await
        .unwrap_or_else(|e| panic!("probe against a healthy server failed: {e}"));
    assert_eq!(report.account_id, account_id);
    assert_eq!(report.base_addr, expected_base);
    assert!(report.handshake_done);

    server.orchestrator.stop_all().await;
}

#[tokio::test]
async fn login_probe_live_db_rejects_a_wrong_advertised_base_endpoint() {
    let pool = match live_db_pool_or_skip().await {
        Some(p) => p,
        None => return,
    };
    let account_id = seeded_account_id(&pool).await;
    let url = std::env::var("DATABASE_URL").expect("checked by live_db_pool_or_skip");
    let server = start_server(&url).await;
    let base_port = server.base_port;
    let advertised: SocketAddr = format!("127.0.0.1:{base_port}").parse().unwrap();
    // What a container smoke expects when the image's BASE_EXTERNAL or
    // BASE_PORT drifted from the published endpoint.
    let expected: SocketAddr = format!("127.0.0.1:{}", base_port.wrapping_add(1))
        .parse()
        .unwrap();

    let cfg = ProbeConfig {
        expect_base: Some(expected),
        expect_account_id: Some(account_id),
        ..probe_config(server.auth_url.clone())
    };
    let err = login_probe::run(&cfg).await.unwrap_err();
    match err {
        ProbeFailure::BaseMismatch {
            expected: e,
            got: g,
        } => {
            assert_eq!(e, expected);
            assert_eq!(g, advertised);
        }
        other => panic!("expected ProbeFailure::BaseMismatch, got: {other}"),
    }

    server.orchestrator.stop_all().await;
}
