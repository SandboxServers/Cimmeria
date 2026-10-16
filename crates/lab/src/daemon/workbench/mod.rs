//! Isolated local-browser session boundary for the companion workbench.

pub mod api;
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
use crate::server::Instances;
use session::SessionStore;

const COOKIE_NAME: &str = "cimmeria_workbench";
const CSRF_HEADER: &str = "x-cimmeria-csrf";

/// Mount the bearer-authenticated opener beside an independent browser gate.
pub fn router(sessions: Arc<SessionStore>, token: Arc<str>, instances: Instances) -> Router {
    let browser = Router::new()
        .route("/workbench/", get(api::index))
        .route("/workbench/app.js", get(api::javascript))
        .route("/workbench/views/dossier.js", get(api::dossier_javascript))
        .route("/workbench/app.css", get(api::stylesheet))
        .route("/workbench/session", get(csrf).post(session_check))
        .merge(api::router(instances.clone()))
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
        .with_state((sessions.clone(), instances));
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

fn check_origin(headers: &HeaderMap, required: bool) -> Result<(), &'static str> {
    let host = local_host(headers).ok_or("host_invalid")?;
    match headers.get(ORIGIN) {
        Some(origin) if origin.to_str().ok() == Some(format!("http://{host}").as_str()) => Ok(()),
        None if !required => Ok(()),
        _ => Err("origin_invalid"),
    }
}

fn refusal(status: StatusCode, reason: &'static str) -> Response {
    tracing::warn!(target: "lab.workbench.auth", reason, "workbench request rejected");
    secure_response(status.into_response())
}

fn requested_instance(uri: &axum::http::Uri) -> Option<&str> {
    let mut values = uri
        .query()?
        .split('&')
        .filter_map(|part| part.strip_prefix("instance="));
    let value = values.next()?;
    (values.next().is_none()
        && !value.is_empty()
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_'))
    .then_some(value)
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
    if let Err(reason) = check_origin(headers, mutating) {
        return refusal(StatusCode::FORBIDDEN, reason);
    }
    let Some(identity) = cookie_value(headers).and_then(|cookie| sessions.identity(cookie)) else {
        return refusal(StatusCode::UNAUTHORIZED, "cookie_invalid");
    };
    if (request.uri().path() == "/workbench/session"
        || request.uri().path().starts_with("/workbench/api/"))
        && !requested_instance(request.uri())
            .is_some_and(|instance| identity.instance.eq_ignore_ascii_case(instance))
    {
        return refusal(StatusCode::FORBIDDEN, "instance_mismatch");
    }
    if mutating {
        let supplied = headers.get(CSRF_HEADER).and_then(|v| v.to_str().ok());
        if !matches!(supplied, Some(value) if auth::constant_time_eq(value.as_bytes(), identity.csrf.as_bytes()))
        {
            return refusal(StatusCode::FORBIDDEN, "csrf_invalid");
        }
    }
    secure_response(next.run(request).await)
}

#[derive(serde::Deserialize)]
struct OpenRequest {
    instance: String,
}

async fn open(
    State((sessions, instances)): State<(Arc<SessionStore>, Instances)>,
    headers: HeaderMap,
    Json(request): Json<OpenRequest>,
) -> Response {
    if let Err(reason) = check_origin(&headers, false) {
        return refusal(StatusCode::FORBIDDEN, reason);
    }
    if headers.contains_key(ORIGIN) {
        return refusal(StatusCode::FORBIDDEN, "origin_invalid");
    }
    let Some(hosted) = instances.by_label(&request.instance) else {
        return refusal(StatusCode::BAD_REQUEST, "instance_unknown");
    };
    let nonce = sessions.open(&hosted.label);
    secure_response(
        Json(json!({ "path": format!("/workbench/session/redeem/{nonce}"), "instance": hosted.label })).into_response(),
    )
}

async fn redeem(
    State(sessions): State<Arc<SessionStore>>,
    Path(nonce): Path<String>,
    headers: HeaderMap,
) -> Response {
    if let Err(reason) = check_origin(&headers, false) {
        return refusal(StatusCode::FORBIDDEN, reason);
    }
    let Some(credentials) = sessions.redeem(&nonce) else {
        return refusal(StatusCode::UNAUTHORIZED, "nonce_invalid");
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

async fn csrf(State(sessions): State<Arc<SessionStore>>, headers: HeaderMap) -> Response {
    let Some(identity) = cookie_value(&headers).and_then(|cookie| sessions.identity(cookie)) else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    Json(json!({ "instance": identity.instance, "csrf": identity.csrf })).into_response()
}

async fn session_check() -> StatusCode {
    StatusCode::NO_CONTENT
}
