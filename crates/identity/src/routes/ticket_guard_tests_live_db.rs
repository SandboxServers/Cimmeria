//! Live-DB check for the launch-ticket guard (CT-77): a grant revoked before the
//! mint is refused and mints nothing. The sentinel account id is `0x7000_EA00`
//! (`Player_1234`); cleanup deletes exactly that id, before and after the test.

use std::sync::Arc;
use std::time::Instant;

use axum::Json;
use cimmeria_auth::auth::{AuthRouteDeps, LaunchTicketStore};
use cimmeria_test_support::require_db_or_skip;
use sqlx::PgPool;

use super::{mint_ticket, LaunchRequest};
use crate::config;
use crate::limits::RateLimiter;
use crate::link_code::LinkCodeStore;
use crate::providers::Providers;
use crate::routes::ctx::{IdentityCtx, RouteState};
use crate::routes::error::ApiError;
use crate::stores::Stores;

/// The sentinel account of this file; `0x7000_EA00..0x7000_EAFF` is CT-77's range.
const SENTINEL: u32 = 0x7000_EA00;

const TABLE: &str = r#"
public_base_url = "https://play.example.org"

[links]
website = "https://example.org"
account = "https://example.org/account"
forums = "https://example.org/forums"
"#;

/// Deletes the sentinel account; its rows cascade.
async fn clean(pool: &PgPool) {
    let _ = sqlx::query("DELETE FROM account WHERE account_id = $1")
        .bind(SENTINEL as i32)
        .execute(pool)
        .await;
}

/// A route state over `pool`, with the given launch-ticket store.
fn state(pool: &PgPool, tickets: Arc<LaunchTicketStore>) -> RouteState {
    let (cfg, _) = config::load(TABLE, &|_| None, false).expect("the fixture loads");
    RouteState {
        deps: Arc::new(AuthRouteDeps {
            db: Some(Arc::new(pool.clone())),
            launch_tickets: tickets,
            developer_mode: false,
        }),
        ctx: IdentityCtx {
            limiter: Arc::new(RateLimiter::with_limits(&cfg.limits)),
            cfg: Arc::new(cfg),
            providers: Arc::new(Providers::from_parts(Vec::new(), None)),
            stores: Arc::new(Stores::default()),
            link_codes: Box::leak(Box::new(LinkCodeStore::default())),
        },
    }
}

#[tokio::test]
async fn live_db_ticket_route_refuses_a_grant_revoked_before_the_mint() {
    let pool = require_db_or_skip!();
    clean(&pool).await;
    sqlx::query(
        "INSERT INTO account (account_id, account_name, password, password_algo, accesslevel, enabled) \
         VALUES ($1, $2, NULL, 1, 0, true)",
    )
    .bind(SENTINEL as i32)
    .bind("Player_1234")
    .execute(&pool)
    .await
    .expect("insert sentinel account");

    let tickets = Arc::new(LaunchTicketStore::default());
    let state = state(&pool, Arc::clone(&tickets));
    let now = Instant::now();
    let token = state
        .ctx
        .stores
        .access
        .mint(SENTINEL, "family-ct77", false, now);
    let grant = state
        .ctx
        .stores
        .access
        .check(&token, now)
        .expect("the fresh token is live");
    state.ctx.stores.access.revoke_account(SENTINEL);

    let body = Ok(Json(LaunchRequest {
        launch_id: Some("abcdefghijklmnop".to_owned()),
    }));
    let result = mint_ticket(&state, "203.0.113.5", &grant, body).await;
    assert!(matches!(result, Err(ApiError::InvalidToken)));
    assert_eq!(tickets.live_count(), 0);

    clean(&pool).await;
}
