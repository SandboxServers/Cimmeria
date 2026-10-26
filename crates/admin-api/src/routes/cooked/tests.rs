//! The presync routes, through their injectable cores: one test per row of
//! the contract's routes table, the check order, and the quota.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Instant;

use axum::body::to_bytes;
use axum::http::{header, HeaderMap, HeaderName, StatusCode};
use axum::response::Response;

use cimmeria_cooked_pak::catalog::{decode_body, CategoryBody, Limits};
use cimmeria_cooked_pak::digest::category_digest;
use cimmeria_resources::base::served_catalog::{Published, ServedCatalog};

use super::handlers::{admit, category_inner, index_inner};
use crate::routes::dev_session::quota::WindowTable;

/// A made-up category: `keys` with a four-byte body each, the first one
/// marked changed. The PAK name is `cat<id>.pak`.
fn body(id: u32, version: u32, keys: &[u32]) -> (String, CategoryBody) {
    let elements: Vec<(u32, Vec<u8>)> =
        keys.iter().map(|key| (*key, vec![*key as u8; 4])).collect();
    let digest = category_digest(
        id,
        version,
        elements.iter().map(|(key, bytes)| (*key, bytes.as_slice())),
    );
    let changed: BTreeMap<u32, Vec<u8>> = elements.iter().take(1).cloned().collect();
    (
        format!("cat{id}.pak"),
        CategoryBody {
            category_id: id,
            version,
            keys: keys.to_vec(),
            changed,
            digest,
        },
    )
}

/// A published catalog holding categories 7 (version 3) and 9 (version 5).
fn ready() -> Option<Published> {
    let catalog = ServedCatalog::from_bodies(vec![body(7, 3, &[1, 2]), body(9, 5, &[4])]);
    Some(Published::Ready(Arc::new(catalog)))
}

/// A build that failed: the code is the reason the server logs.
fn failed() -> Option<Published> {
    Some(Published::Failed("raw_pak_unreadable"))
}

/// Split a response into its status, its headers and its body bytes.
async fn answer(response: Response) -> (StatusCode, HeaderMap, Vec<u8>) {
    let (parts, body) = response.into_parts();
    let bytes = to_bytes(body, usize::MAX).await.unwrap();
    (parts.status, parts.headers, bytes.to_vec())
}

/// A header's value as text, if the response carries it.
fn header_text(headers: &HeaderMap, name: HeaderName) -> Option<&str> {
    headers.get(name).map(|value| value.to_str().unwrap())
}

/// A body as text. The refusals are JSON, the bodies are binary: the
/// tests that want text read a refusal.
fn text(bytes: &[u8]) -> &str {
    std::str::from_utf8(bytes).unwrap()
}

/// The index route answers 200 with the index JSON, and `no-cache`.
#[tokio::test]
async fn the_index_is_served_with_no_cache() {
    let Some(Published::Ready(catalog)) = ready() else {
        unreachable!("ready() is Ready")
    };
    let (status, headers, body) = answer(index_inner(ready(), false, true)).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        header_text(&headers, header::CONTENT_TYPE),
        Some("application/json")
    );
    assert_eq!(
        header_text(&headers, header::CACHE_CONTROL),
        Some("no-cache")
    );
    assert_eq!(body, catalog.index_json().to_vec());
}

/// The category route answers 200 with the `CPS1` body, which decodes back
/// to the category it was built from.
#[tokio::test]
async fn a_category_is_served_as_a_cps1_body_that_decodes() {
    let (status, headers, bytes) =
        answer(category_inner(ready(), false, true, "7", Some("version=3"))).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        header_text(&headers, header::CONTENT_TYPE),
        Some("application/octet-stream")
    );
    let decoded = decode_body(&bytes, &Limits::DEFAULT).expect("the body decodes");
    assert_eq!(decoded.category_id, 7);
    assert_eq!(decoded.version, 3);
    assert_eq!(decoded.keys, vec![1, 2]);
    assert_eq!(decoded.changed.len(), 1);
}

/// While the build runs, both routes answer 503 `catalog_unavailable` with
/// `Retry-After: 5`.
#[tokio::test]
async fn a_build_still_running_answers_503_with_retry_after() {
    let (status, headers, body) = answer(index_inner(None, false, true)).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(header_text(&headers, header::RETRY_AFTER), Some("5"));
    assert_eq!(text(&body), r#"{"error":"catalog_unavailable"}"#);

    let (status, headers, body) =
        answer(category_inner(None, false, true, "7", Some("version=3"))).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(header_text(&headers, header::RETRY_AFTER), Some("5"));
    assert_eq!(text(&body), r#"{"error":"catalog_unavailable"}"#);
}

/// A failed build answers 503 `catalog_failed` on both routes, with no
/// `Retry-After`: the launcher fails at once instead of waiting.
#[tokio::test]
async fn a_failed_build_answers_catalog_failed_without_retry_after() {
    let (status, headers, body) = answer(index_inner(failed(), false, true)).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(header_text(&headers, header::RETRY_AFTER), None);
    assert_eq!(text(&body), r#"{"error":"catalog_failed"}"#);

    let (status, headers, body) = answer(category_inner(
        failed(),
        false,
        true,
        "7",
        Some("version=3"),
    ))
    .await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(header_text(&headers, header::RETRY_AFTER), None);
    assert_eq!(text(&body), r#"{"error":"catalog_failed"}"#);
}

/// The kill switch answers 503 `disabled` on both routes, with the catalog
/// ready and with the build still running.
#[tokio::test]
async fn the_kill_switch_answers_503_disabled() {
    let (status, _, body) = answer(index_inner(ready(), true, true)).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(text(&body), r#"{"error":"disabled"}"#);

    let (status, _, body) = answer(category_inner(None, true, true, "7", Some("version=3"))).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(text(&body), r#"{"error":"disabled"}"#);
}

/// The kill switch is checked before the catalog state: with the switch on
/// and the build failed, the answer is `disabled`, and the launcher skips
/// the presync and launches (D-LP19). Reordering the checks fails this.
#[tokio::test]
async fn the_kill_switch_wins_over_a_failed_catalog() {
    let (status, _, body) = answer(index_inner(failed(), true, true)).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(text(&body), r#"{"error":"disabled"}"#);

    let (_, _, body) = answer(category_inner(failed(), true, true, "7", Some("version=3"))).await;
    assert_eq!(text(&body), r#"{"error":"disabled"}"#);
}

/// An id the catalog does not hold is 404 `unknown_category`.
#[tokio::test]
async fn an_unknown_category_is_404() {
    let (status, _, body) = answer(category_inner(
        ready(),
        false,
        true,
        "99",
        Some("version=1"),
    ))
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(text(&body), r#"{"error":"unknown_category"}"#);
}

/// A version the index no longer names is 409 `version_changed`, carrying
/// the version the catalog serves now.
#[tokio::test]
async fn a_changed_version_is_409_with_the_served_version() {
    let (status, _, body) =
        answer(category_inner(ready(), false, true, "7", Some("version=2"))).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(text(&body), r#"{"error":"version_changed","version":3}"#);
}

/// A missing version, a repeated one, a non-number, and an id that is not a
/// `u32` are all 400 `bad_request`.
#[tokio::test]
async fn a_missing_or_bad_version_or_id_is_400() {
    for (id, query) in [
        ("7", None),
        ("7", Some("version=x")),
        ("7", Some("version=3&version=3")),
        ("7", Some("other=3")),
        ("seven", Some("version=3")),
        ("-1", Some("version=3")),
    ] {
        let (status, _, body) = answer(category_inner(ready(), false, true, id, query)).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{id} {query:?}");
        assert_eq!(text(&body), r#"{"error":"bad_request"}"#, "{id} {query:?}");
    }
}

/// The 61st request from one address inside a minute is refused: the quota
/// admits 60, then says no. Refused, the answer is 429 `rate_limited` with
/// `Retry-After: 60`, and the quota is checked before the kill switch.
#[tokio::test]
async fn quota_refuses_the_61st_request_in_a_minute() {
    let quota = WindowTable::new();
    let now = Instant::now();
    for _ in 0..60 {
        assert!(admit(&quota, 1, 60, now));
    }
    assert!(!admit(&quota, 1, 60, now));
    // Another address has its own allowance.
    assert!(admit(&quota, 2, 60, now));
    // A limit of 0 admits everything.
    assert!(admit(&quota, 3, 0, now));

    let (status, headers, body) = answer(index_inner(ready(), true, false)).await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(header_text(&headers, header::RETRY_AFTER), Some("60"));
    assert_eq!(text(&body), r#"{"error":"rate_limited"}"#);
}
