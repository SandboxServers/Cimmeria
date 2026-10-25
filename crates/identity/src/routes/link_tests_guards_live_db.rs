//! Live-database guard tests for the link routes: a malformed account name is
//! refused before the link limiter, and the server-wide link-code cap answers
//! `rate_limited` after 50 redeems. Kept apart from `link_tests_live_db.rs` to
//! stay under the file-size cap; it reuses that file's harness.

use axum::http::StatusCode;
use cimmeria_test_support::require_db_or_skip;
use serde_json::json;

use super::tests_live_db::{clean, grant, harness, post_json, verified};

/// A name the grammar refuses gets `invalid_request` before any credential is
/// checked (the answer without the guard is 401 `invalid_credentials`).
#[tokio::test]
async fn live_db_link_existing_refuses_a_malformed_name() {
    let pool = require_db_or_skip!();
    clean(&pool).await;
    let env = harness(&pool);
    let g = grant(&env, verified("dev", "ct26-30", None, false));
    let (status, body) = post_json(
        &env,
        "/link/existing",
        json!({
            "link_grant": g,
            "account_name": "ab",
            "password": "x",
            "password_removal_ack": true,
        }),
    )
    .await;
    clean(&pool).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert_eq!(body["error"], "invalid_request");
}

/// The global link-code cap: 50 redeems are charged whatever their answer, and
/// the 51st is refused with 429 before it is redeemed.
#[tokio::test]
async fn live_db_link_code_global_cap() {
    let pool = require_db_or_skip!();
    clean(&pool).await;
    let env = harness(&pool);
    for i in 0..50 {
        let g = grant(&env, verified("dev", &format!("ct26-cap-{i}"), None, false));
        let (status, body) = post_json(
            &env,
            "/link/existing",
            json!({
                "link_grant": g,
                "link_code": "AAAAAAAAAA",
                "password_removal_ack": true,
            }),
        )
        .await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "redeem {i}: {body}");
    }
    let g = grant(&env, verified("dev", "ct26-cap-51", None, false));
    let (status, body) = post_json(
        &env,
        "/link/existing",
        json!({
            "link_grant": g,
            "link_code": "AAAAAAAAAA",
            "password_removal_ack": true,
        }),
    )
    .await;
    clean(&pool).await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS, "{body}");
    assert_eq!(body["error"], "rate_limited");
}
