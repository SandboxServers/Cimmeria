//! Forwarded-address tests (C18, D-LT19): the per-IP mint quota keyed on the
//! caller behind a loopback proxy, and the rule that the forwarded address
//! never reaches a log row.

use std::time::Instant;

use tracing_subscriber::layer::SubscriberExt;

use super::handlers::{mint_with_headers, Tables};
use super::refusal_log::{log_refusal_in, Throttle};
use super::tests::{ip, policy, request, EnvGuard, Rows, NOW_UNIX};
use super::token::AuthError;

/// One `X-Forwarded-For` header naming `caller`, as the board proxy sets it.
fn forwarded_for(caller: &str) -> axum::http::HeaderMap {
    let mut headers = axum::http::HeaderMap::new();
    headers.insert("x-forwarded-for", caller.parse().unwrap());
    headers
}

// Behind the colo's loopback proxy the per-IP quota keys on the forwarded
// caller (C18): distinct callers do not share the proxy's allowance, one
// repeated caller is refused at quota + 1, and a remote peer cannot buy
// new allowances by rotating the header.
#[test]
fn mint_keys_the_per_ip_quota_on_the_forwarded_caller_behind_a_loopback_proxy() {
    let _g = EnvGuard::install();
    let t = Tables::new();
    let p = policy(2, 100, 10);
    let now = Instant::now();
    let proxy = ip("127.0.0.1");
    for (i, caller) in ["198.51.100.10", "198.51.100.10", "198.51.100.11"]
        .into_iter()
        .enumerate()
    {
        mint_with_headers(
            &t,
            &p,
            proxy,
            &forwarded_for(caller),
            request(&format!("id-{i}")),
            now,
            NOW_UNIX,
        )
        .unwrap_or_else(|e| panic!("mint {i} from {caller} should pass, got {e}"));
    }
    let err = mint_with_headers(
        &t,
        &p,
        proxy,
        &forwarded_for("198.51.100.10"),
        request("id-3"),
        now,
        NOW_UNIX,
    )
    .unwrap_err();
    assert!(matches!(err, AuthError::QuotaExceeded(_)));

    let remote = ip("203.0.113.9");
    for (i, caller) in ["198.51.100.20", "198.51.100.21"].into_iter().enumerate() {
        mint_with_headers(
            &t,
            &p,
            remote,
            &forwarded_for(caller),
            request(&format!("spoof-{i}")),
            now,
            NOW_UNIX,
        )
        .unwrap_or_else(|e| panic!("spoof mint {i} should pass, got {e}"));
    }
    let err = mint_with_headers(
        &t,
        &p,
        remote,
        &forwarded_for("198.51.100.22"),
        request("spoof-2"),
        now,
        NOW_UNIX,
    )
    .unwrap_err();
    assert!(matches!(err, AuthError::QuotaExceeded(_)));
}

// Directive 6 (D-LT19): the forwarded address is a quota key and nothing
// else. A mint that passes and a mint refused on quota, both behind a
// loopback proxy that forwarded `198.51.100.7`, leave no row that names it.
#[test]
fn forwarded_address_never_reaches_a_mint_log_row() {
    let _g = EnvGuard::install();
    let t = Tables::new();
    let p = policy(1, 100, 10);
    let now = Instant::now();
    let proxy = ip("127.0.0.1");
    let headers = forwarded_for("198.51.100.7");
    let rows = Rows::default();
    let sub = tracing_subscriber::registry().with(rows.clone());
    tracing::subscriber::with_default(sub, || {
        mint_with_headers(&t, &p, proxy, &headers, request("leak-a"), now, NOW_UNIX)
            .expect("the first mint passes");
        let err = mint_with_headers(&t, &p, proxy, &headers, request("leak-b"), now, NOW_UNIX)
            .unwrap_err();
        log_refusal_in(&Throttle::default(), "mint", &err, Instant::now());
    });
    let captured = rows.0.lock().unwrap().clone();
    assert!(!captured.is_empty(), "the refused mint should leave a row");
    for (_, fields) in &captured {
        for (key, value) in fields {
            assert!(
                !key.contains("198.51.100.7") && !value.contains("198.51.100.7"),
                "a log row carries the forwarded address: {key}={value}"
            );
        }
    }
}
