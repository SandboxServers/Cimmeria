//! Cookie-gated static files for the local companion window.

use std::sync::Arc;

use axum::{
    extract::State,
    http::{header::CONTENT_TYPE, HeaderMap, HeaderValue, StatusCode},
    response::{Html, IntoResponse, Response},
};

use super::{bound_instance, session::SessionStore};

const INDEX: &str = include_str!("../../../workbench/index.html");
const JAVASCRIPT: &str = include_str!("../../../workbench/app.js");
const DOSSIER_JAVASCRIPT: &str = include_str!("../../../workbench/views/dossier.js");
const PROBE_JAVASCRIPT: &str = include_str!("../../../workbench/views/probe.js");
const MISSION_JAVASCRIPT: &str = include_str!("../../../workbench/views/mission.js");
const STYLESHEET: &str = include_str!("../../../workbench/app.css");

/// Serve the shell with only the session's validated instance label embedded.
pub(super) async fn index(
    State(sessions): State<Arc<SessionStore>>,
    headers: HeaderMap,
) -> Response {
    let Some(instance) = bound_instance(&sessions, &headers) else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    // Instance labels are constrained to this alphabet by the supervisor.
    if !instance
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }
    Html(INDEX.replace("__INSTANCE__", &instance)).into_response()
}

fn static_asset(content_type: &'static str, body: &'static str) -> Response {
    let mut response = body.into_response();
    response
        .headers_mut()
        .insert(CONTENT_TYPE, HeaderValue::from_static(content_type));
    response.headers_mut().insert(
        "x-content-type-options",
        HeaderValue::from_static("nosniff"),
    );
    response
}

/// Serve the bundled shell script through the browser session gate.
pub(super) async fn javascript() -> Response {
    static_asset("text/javascript; charset=utf-8", JAVASCRIPT)
}

/// Serve the read-only dossier view through the browser session gate.
pub(super) async fn dossier_javascript() -> Response {
    static_asset("text/javascript; charset=utf-8", DOSSIER_JAVASCRIPT)
}

/// Serve the leased probe view through the browser session gate.
pub(super) async fn probe_javascript() -> Response {
    static_asset("text/javascript; charset=utf-8", PROBE_JAVASCRIPT)
}

/// Serve the read-only mission X-ray view through the browser session gate.
pub(super) async fn mission_javascript() -> Response {
    static_asset("text/javascript; charset=utf-8", MISSION_JAVASCRIPT)
}

/// Serve the bundled stylesheet through the browser session gate.
pub(super) async fn stylesheet() -> Response {
    static_asset("text/css; charset=utf-8", STYLESHEET)
}
