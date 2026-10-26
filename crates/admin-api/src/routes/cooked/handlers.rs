//! The cooked presync handlers and their injectable cores.
//!
//! Each route runs the checks in the order the contract pins: the per-address
//! quota, the kill switch, the catalog state, then the route's own checks.
//! The cores take the quota decision, the kill switch and the published
//! catalog as plain values, so the tests drive every branch without a
//! listener, the environment or the process-wide catalog.

use std::net::SocketAddr;
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

use axum::body::Body;
use axum::extract::{ConnectInfo, Path, RawQuery};
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};

use cimmeria_cooked_pak::catalog::Index;
use cimmeria_resources::base::served_catalog::{self, Published, ServedCatalog};

use crate::routes::dev_session::env_u32;
use crate::routes::dev_session::forwarded_for;
use crate::routes::dev_session::quota::{ip_key, WindowTable};

/// The quota window: one minute, counted from an address's first request
/// in it. It is fixed, so a refused caller is told to wait 60 seconds.
const QUOTA_WINDOW: Duration = Duration::from_secs(60);

/// Requests per peer address per [`QUOTA_WINDOW`]. A full presync costs one
/// index request and one per changed category, and a launcher runs one per
/// Play; the allowance is for the address, so players behind one NAT share it.
pub(super) const DEFAULT_QUOTA_PER_IP: u32 = 60;

/// `Retry-After` on a 429: the whole window.
const RATE_LIMIT_RETRY_AFTER: u64 = 60;

/// `Retry-After` on a 503 while the build is still running.
const BUILDING_RETRY_AFTER: u64 = 5;

/// The per-address quota table. It lives for the process, so the count
/// survives from one request to the next.
fn quota() -> &'static WindowTable {
    static QUOTA: OnceLock<WindowTable> = OnceLock::new();
    QUOTA.get_or_init(WindowTable::new)
}

/// `CIMMERIA_COOKED_PRESYNC_QUOTA_PER_IP`, read per request; 0 disables the
/// quota. A malformed value falls back to the default.
fn quota_limit() -> u32 {
    env_u32("CIMMERIA_COOKED_PRESYNC_QUOTA_PER_IP", DEFAULT_QUOTA_PER_IP)
}

/// `CIMMERIA_COOKED_PRESYNC_DISABLED`, read per request: `1` or `true` turns
/// the presync routes off.
fn kill_switch_on() -> bool {
    switch_is_on(
        std::env::var("CIMMERIA_COOKED_PRESYNC_DISABLED")
            .ok()
            .as_deref(),
    )
}

/// Whether a kill-switch value turns the routes off: `1` or `true`, nothing
/// else, unset included.
pub(super) fn switch_is_on(value: Option<&str>) -> bool {
    matches!(value, Some("1" | "true"))
}

/// The routes, for the metric's `route` label and the log rows.
#[derive(Clone, Copy)]
pub(super) enum Route {
    Index,
    Category,
}

impl Route {
    fn name(self) -> &'static str {
        match self {
            Route::Index => "index",
            Route::Category => "category",
        }
    }
}

/// Charge one request to `key` and say whether it is within the quota. The
/// window is [`QUOTA_WINDOW`], the scope is `cooked/ip`, and a `limit` of 0
/// admits everything.
pub(super) fn admit(quota: &WindowTable, key: u64, limit: u32, now: Instant) -> bool {
    quota
        .check_and_record(key, limit, QUOTA_WINDOW, "cooked/ip", now)
        .is_ok()
}

/// `GET /api/cooked/v1/index`.
pub(super) async fn index(
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
) -> Response {
    let admitted = admit_peer(peer, &headers);
    index_inner(served_catalog::current(), kill_switch_on(), admitted)
}

/// `GET /api/cooked/v1/category/{id}?version=<v>`. The path segment and the
/// query come through unparsed, so a bad one is answered in the route's own
/// JSON, not by an extractor's plain-text refusal.
pub(super) async fn category(
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Path(id): Path<String>,
    RawQuery(query): RawQuery,
) -> Response {
    let admitted = admit_peer(peer, &headers);
    category_inner(
        served_catalog::current(),
        kill_switch_on(),
        admitted,
        &id,
        query.as_deref(),
    )
}

/// The quota decision for one request: the peer's address, or the forwarded
/// caller behind a loopback proxy (C18), as the launcher-summary route keys it.
fn admit_peer(peer: SocketAddr, headers: &HeaderMap) -> bool {
    admit_peer_with(quota(), peer, headers, quota_limit(), Instant::now())
}

/// [`admit_peer`] against the given table, limit and clock.
pub(super) fn admit_peer_with(
    quota: &WindowTable,
    peer: SocketAddr,
    headers: &HeaderMap,
    limit: u32,
    now: Instant,
) -> bool {
    let key = ip_key(forwarded_for::rate_limit_ip(peer.ip(), headers));
    admit(quota, key, limit, now)
}

/// The index, for the given catalog state, kill switch and quota decision.
pub(super) fn index_inner(catalog: Option<Published>, disabled: bool, admit: bool) -> Response {
    let catalog = match gate(Route::Index, 0, catalog, disabled, admit) {
        Ok(catalog) => catalog,
        Err(refusal) => return *refusal,
    };
    let index = catalog.index_json();
    let categories = serde_json::from_slice::<Index>(&index).map_or(0, |i| i.categories.len());
    tracing::debug!(
        event = "cooked_presync.index_served",
        categories,
        "cooked presync index served"
    );
    served(Route::Index, "application/json", index.to_vec(), true)
}

/// One category's `CPS1` body, for the given catalog state, kill switch and
/// quota decision. `id` is the path segment and `query` the raw query string.
pub(super) fn category_inner(
    catalog: Option<Published>,
    disabled: bool,
    admit: bool,
    id: &str,
    query: Option<&str>,
) -> Response {
    let parsed_id = id.parse::<u32>().ok();
    let catalog = match gate(
        Route::Category,
        parsed_id.unwrap_or(0),
        catalog,
        disabled,
        admit,
    ) {
        Ok(catalog) => catalog,
        Err(refusal) => return *refusal,
    };
    let Some(category_id) = parsed_id else {
        return refuse(
            Route::Category,
            0,
            "bad_request",
            StatusCode::BAD_REQUEST,
            None,
        );
    };
    let Some(version) = version_param(query) else {
        return refuse(
            Route::Category,
            category_id,
            "bad_request",
            StatusCode::BAD_REQUEST,
            None,
        );
    };
    let Some(served_category) = catalog.category(category_id) else {
        return refuse(
            Route::Category,
            category_id,
            "unknown_category",
            StatusCode::NOT_FOUND,
            None,
        );
    };
    if served_category.entry.version != version {
        note(
            Route::Category,
            category_id,
            "version_changed",
            StatusCode::CONFLICT,
        );
        return error_response(
            StatusCode::CONFLICT,
            format!(
                r#"{{"error":"version_changed","version":{}}}"#,
                served_category.entry.version
            ),
            None,
        );
    }
    tracing::info!(
        event = "cooked_presync.served",
        category_id,
        category_name = cimmeria_resources::base::resources::category_name(category_id),
        version,
        bytes = served_category.entry.body_bytes,
        changed = served_category.entry.changed,
        "cooked presync category served"
    );
    served(
        Route::Category,
        "application/octet-stream",
        served_category.body.to_vec(),
        false,
    )
}

/// The checks every route runs before its own: the quota (429), the kill
/// switch (503 `disabled`), then the catalog state. A `Failed` build is
/// `catalog_failed` with no `Retry-After`, and a build still running is
/// `catalog_unavailable` with one. Returns the catalog when all three pass.
fn gate(
    route: Route,
    category_id: u32,
    catalog: Option<Published>,
    disabled: bool,
    admit: bool,
) -> Result<Arc<ServedCatalog>, Box<Response>> {
    if !admit {
        return Err(Box::new(refuse(
            route,
            category_id,
            "rate_limited",
            StatusCode::TOO_MANY_REQUESTS,
            Some(RATE_LIMIT_RETRY_AFTER),
        )));
    }
    if disabled {
        return Err(Box::new(refuse(
            route,
            category_id,
            "disabled",
            StatusCode::SERVICE_UNAVAILABLE,
            None,
        )));
    }
    match catalog {
        None => Err(Box::new(refuse(
            route,
            category_id,
            "catalog_unavailable",
            StatusCode::SERVICE_UNAVAILABLE,
            Some(BUILDING_RETRY_AFTER),
        ))),
        Some(Published::Failed(_)) => Err(Box::new(refuse(
            route,
            category_id,
            "catalog_failed",
            StatusCode::SERVICE_UNAVAILABLE,
            None,
        ))),
        Some(Published::Ready(catalog)) => Ok(catalog),
    }
}

/// The `version` query parameter: exactly one, and a number. `None` for a
/// missing or repeated one, or one that is not a `u32`.
fn version_param(query: Option<&str>) -> Option<u32> {
    let mut found = None;
    for pair in query?.split('&') {
        if let Some(value) = pair.strip_prefix("version=") {
            if found.is_some() {
                return None;
            }
            found = Some(value.parse().ok()?);
        }
    }
    found
}

/// Log, count and answer one refusal with `{"error":"<code>"}`.
fn refuse(
    route: Route,
    category_id: u32,
    code: &'static str,
    status: StatusCode,
    retry_after: Option<u64>,
) -> Response {
    note(route, category_id, code, status);
    error_response(status, format!(r#"{{"error":"{code}"}}"#), retry_after)
}

/// The refusal's log row and its counter. The reason is the JSON error code,
/// so the launcher's failure and the server's refusal share one name.
fn note(route: Route, category_id: u32, code: &'static str, status: StatusCode) {
    tracing::info!(
        event = "cooked_presync.refused",
        route = route.name(),
        reason = code,
        status = status.as_u16(),
        category_id,
        "cooked presync request refused"
    );
    count(route, code);
}

/// Bump `cooked_presync_requests_total` for one answer.
fn count(route: Route, outcome: &'static str) {
    cimmeria_observability::counter!(
        "cooked_presync_requests_total",
        "route" => route.name(),
        "outcome" => outcome
    );
}

/// A 200 answer. The index is `no-cache`: the launcher must see a rebuilt
/// catalog. A category is addressed by its version, so it is not.
fn served(route: Route, content_type: &'static str, body: Vec<u8>, no_cache: bool) -> Response {
    count(route, "served");
    let mut response = Response::new(Body::from(body));
    let headers = response.headers_mut();
    headers.insert(header::CONTENT_TYPE, HeaderValue::from_static(content_type));
    if no_cache {
        headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-cache"));
    }
    response
}

/// A JSON refusal, with `Retry-After` when the caller should wait.
fn error_response(status: StatusCode, body: String, retry_after: Option<u64>) -> Response {
    let mut response = (status, [(header::CONTENT_TYPE, "application/json")], body).into_response();
    if let Some(seconds) = retry_after {
        let value =
            HeaderValue::from_str(&seconds.to_string()).expect("a number is a valid header value");
        response.headers_mut().insert(header::RETRY_AFTER, value);
    }
    response
}
