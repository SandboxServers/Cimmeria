//! Isolated local-browser session boundary for the companion workbench.

pub mod session;

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::{
        header::{CACHE_CONTROL, COOKIE, HOST, ORIGIN, SET_COOKIE},
        HeaderMap, HeaderValue, Method, Request, StatusCode,
    },
    middleware::Next,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde_json::json;

use super::auth;
use session::SessionStore;

const COOKIE_NAME: &str = "cimmeria_workbench";
const CSRF_HEADER: &str = "x-cimmeria-csrf";

/// Mount the bearer-authenticated opener beside an independent browser gate.
pub fn router(sessions: Arc<SessionStore>, token: Arc<str>) -> Router {
    let browser = Router::new()
        .route("/workbench/", get(root))
        .route("/workbench/session", get(csrf).post(session_check))
        .route_layer(axum::middleware::from_fn_with_state(
            sessions.clone(),
            require_session,
        ))
        .with_state(sessions.clone());
    let opener = Router::new()
        .route("/workbench/session/open", post(open))
        .route_layer(axum::middleware::from_fn_with_state(
            token,
            auth::require_bearer,
        ))
        .with_state(sessions.clone());
    let redeem = Router::new()
        .route("/workbench/session/redeem/{nonce}", get(redeem))
        .with_state(sessions);
    browser.merge(opener).merge(redeem)
}

fn local_host(headers: &HeaderMap) -> Option<&str> {
    let raw = headers.get(HOST)?.to_str().ok()?;
    let authority: axum::http::uri::Authority = raw.parse().ok()?;
    let port = authority.port_u16()?;
    if port == 0 {
        return None;
    }
    let host = authority.host();
    if raw != format!("{host}:{port}") {
        return None;
    }
    match host {
        "localhost" | "127.0.0.1" | "[::1]" => Some(raw),
        _ => None,
    }
}

fn valid_origin(headers: &HeaderMap, required: bool) -> bool {
    let Some(host) = local_host(headers) else {
        return false;
    };
    match headers.get(ORIGIN) {
        Some(origin) => origin.to_str().ok() == Some(format!("http://{host}").as_str()),
        None => !required,
    }
}

fn secure_response(mut response: Response) -> Response {
    let headers = response.headers_mut();
    headers.insert("referrer-policy", HeaderValue::from_static("no-referrer"));
    headers.insert(
        "content-security-policy",
        HeaderValue::from_static("default-src 'self'"),
    );
    headers.insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

fn cookie_value(headers: &HeaderMap) -> Option<&str> {
    let mut found = None;
    for value in headers.get_all(COOKIE) {
        let line = value.to_str().ok()?;
        for part in line.split(';') {
            if let Some(value) = part.trim().strip_prefix(&format!("{COOKIE_NAME}=")) {
                if found.is_some()
                    || value.len() != 64
                    || !value.bytes().all(|c| c.is_ascii_hexdigit())
                {
                    return None;
                }
                found = Some(value);
            }
        }
    }
    found
}

async fn require_session(
    State(sessions): State<Arc<SessionStore>>,
    request: Request<axum::body::Body>,
    next: Next,
) -> Response {
    let method = request.method();
    let mutating = !matches!(*method, Method::GET | Method::HEAD | Method::OPTIONS);
    let headers = request.headers();
    if !valid_origin(headers, mutating) {
        return secure_response(StatusCode::FORBIDDEN.into_response());
    }
    let Some(csrf) = cookie_value(headers).and_then(|cookie| sessions.csrf(cookie)) else {
        return secure_response(StatusCode::UNAUTHORIZED.into_response());
    };
    if mutating {
        let supplied = headers.get(CSRF_HEADER).and_then(|v| v.to_str().ok());
        if !matches!(supplied, Some(value) if auth::constant_time_eq(value.as_bytes(), csrf.as_bytes()))
        {
            return secure_response(StatusCode::FORBIDDEN.into_response());
        }
    }
    secure_response(next.run(request).await)
}

async fn open(State(sessions): State<Arc<SessionStore>>, headers: HeaderMap) -> Response {
    if !valid_origin(&headers, false) || headers.contains_key(ORIGIN) {
        return secure_response(StatusCode::FORBIDDEN.into_response());
    }
    let nonce = sessions.open();
    secure_response(
        Json(json!({ "path": format!("/workbench/session/redeem/{nonce}") })).into_response(),
    )
}

async fn redeem(
    State(sessions): State<Arc<SessionStore>>,
    Path(nonce): Path<String>,
    headers: HeaderMap,
) -> Response {
    if !valid_origin(&headers, false) {
        return secure_response(StatusCode::FORBIDDEN.into_response());
    }
    let Some(credentials) = sessions.redeem(&nonce) else {
        return secure_response(StatusCode::UNAUTHORIZED.into_response());
    };
    let mut response = StatusCode::SEE_OTHER.into_response();
    response
        .headers_mut()
        .insert("location", HeaderValue::from_static("/workbench/"));
    let cookie = format!(
        "{COOKIE_NAME}={}; HttpOnly; SameSite=Strict; Path=/workbench/; Max-Age=28800",
        credentials.cookie
    );
    response.headers_mut().insert(
        SET_COOKIE,
        HeaderValue::from_str(&cookie).expect("cookie is ASCII"),
    );
    secure_response(response)
}

async fn root() -> StatusCode {
    // DW-02B mounts the static shell here. Keep its path session-gated now.
    StatusCode::NO_CONTENT
}

async fn csrf(State(sessions): State<Arc<SessionStore>>, headers: HeaderMap) -> Response {
    let Some(value) = cookie_value(&headers).and_then(|cookie| sessions.csrf(cookie)) else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    Json(json!({ "csrf": value })).into_response()
}

async fn session_check() -> StatusCode {
    StatusCode::NO_CONTENT
}
