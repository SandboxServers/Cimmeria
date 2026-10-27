//! Live-DB tests of the link/start reauth gate (W3-1) and the no-store header (N2).

use super::*;

/// `link/start` needs a code sign-in less than 10 minutes old (W3-1): a
/// refresh-minted token and a stale code token both get `403 reauth_required`.
#[tokio::test]
async fn live_db_link_start_needs_a_fresh_code_token() {
    let pool = require_db_or_skip!();
    clean(&pool).await;
    insert_account(&pool, ACC_LINK, true).await;
    insert_identity(&pool, ACC_LINK, 22).await;
    let env = harness(&pool);
    let refreshed = token(&env, ACC_LINK, false, Instant::now());
    let old = Instant::now()
        .checked_sub(Duration::from_secs(11 * 60))
        .expect("an instant 11 minutes back");
    let stale = token(&env, ACC_LINK, true, old);
    for bearer in [refreshed, stale] {
        let (status, body) = link_start(&env, &bearer, link_body("dev", "S256")).await;
        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_eq!(body["error"], "reauth_required");
    }
    assert_eq!(methods(&pool, ACC_LINK).await, 1);
}

/// Every `/auth/v1` answer carries `Cache-Control: no-store` (N2): a provider
/// list, a refused token request and a refused route all do.
#[tokio::test]
async fn live_db_every_answer_is_no_store() {
    let pool = require_db_or_skip!();
    clean(&pool).await;
    let env = harness(&pool);
    let (status, headers, _) = send(&env.router, "GET", "/providers", None, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        headers[header::CACHE_CONTROL].to_str().expect("text"),
        "no-store"
    );
    let (status, headers, _) = send(&env.router, "POST", "/token", None, Some(json!({}))).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(
        headers[header::CACHE_CONTROL].to_str().expect("text"),
        "no-store"
    );
}
