//! Live-DB test of W3-6: a disabled account gets `403 account_disabled` on every
//! Bearer route, so an access token minted before the disable cannot read or
//! change the account. The handlers are called directly on a state with a
//! database.

use std::sync::Arc;
use std::time::Instant;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use cimmeria_auth::auth::{AuthRouteDeps, LaunchTicketStore};
use cimmeria_test_support::require_db_or_skip;
use serde_json::json;
use sqlx::PgPool;

use crate::routes::account::{get_account, unlink};
use crate::routes::bearer::Bearer;
use crate::routes::ctx::RouteState;
use crate::routes::ticket::{end, heartbeat, LaunchRequest};
use crate::routes::token::span_tests::test_state;
use crate::stores::AccessGrant;

const ACC_DISABLED: u32 = 2_000_690;

/// Inserts a disabled sentinel account, replacing any row from an earlier run.
async fn insert_disabled(pool: &PgPool) {
    sqlx::query("DELETE FROM account WHERE account_id = $1")
        .bind(ACC_DISABLED as i32)
        .execute(pool)
        .await
        .expect("the sentinel row is removed");
    sqlx::query(
        "INSERT INTO account \
         (account_id, account_name, password, password_algo, accesslevel, enabled, test_account) \
         VALUES ($1, $2, NULL, 1, 0, false, false)",
    )
    .bind(ACC_DISABLED as i32)
    .bind(format!("W36-{ACC_DISABLED}"))
    .execute(pool)
    .await
    .expect("the sentinel row is inserted");
}

/// A route state on `pool` with the default limits and no providers.
fn state(pool: &PgPool) -> RouteState {
    let mut state = test_state();
    state.deps = Arc::new(AuthRouteDeps {
        db: Some(Arc::new(pool.clone())),
        launch_tickets: Arc::new(LaunchTicketStore::default()),
        developer_mode: false,
    });
    state
}

/// A code sign-in token of the disabled account, issued now.
fn grant() -> Bearer {
    Bearer(AccessGrant {
        account_id: ACC_DISABLED,
        family_id: "family-w36".to_owned(),
        via_code: true,
        issued_at: Instant::now(),
        minted_seq: 0,
    })
}

fn launch_body() -> Result<Json<LaunchRequest>, axum::extract::rejection::JsonRejection> {
    let request: LaunchRequest =
        serde_json::from_value(json!({"launch_id": "abcdefghijklmnop"})).expect("a launch body");
    Ok(Json(request))
}

/// Every Bearer route answers `403 account_disabled` for a disabled account.
#[tokio::test]
async fn live_db_disabled_account_is_refused_on_bearer_routes() {
    let pool = require_db_or_skip!();
    insert_disabled(&pool).await;
    let answers: [Response; 4] = [
        get_account(State(state(&pool)), grant())
            .await
            .unwrap_or_else(IntoResponse::into_response),
        unlink(State(state(&pool)), grant(), Ok(Path(1))).await,
        heartbeat(State(state(&pool)), grant(), launch_body()).await,
        end(State(state(&pool)), grant(), launch_body()).await,
    ];
    for answer in answers {
        assert_eq!(answer.status(), StatusCode::FORBIDDEN);
    }
}
