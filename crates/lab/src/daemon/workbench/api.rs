//! Cookie-gated, typed HTTP facade for the local companion window.

use std::sync::Arc;

use axum::{
    extract::{Extension, Query, State},
    http::{header::CONTENT_TYPE, HeaderMap, HeaderValue, StatusCode},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};

use super::{cookie_value, session::SessionStore};
use crate::{
    lease::permit::{self, Permit},
    server::Instances,
    workbench::dossier,
};

const INDEX: &str = include_str!("../../../workbench/index.html");
const JAVASCRIPT: &str = include_str!("../../../workbench/app.js");
const STYLESHEET: &str = include_str!("../../../workbench/app.css");

/// Routes merged into the DW-02A browser router before its session layer.
pub fn router(instances: Instances) -> Router<Arc<SessionStore>> {
    Router::new()
        .route("/workbench/api/v1/session", get(session))
        .route("/workbench/api/v1/dossier", get(target_dossier))
        .route("/workbench/api/v1/input/key", post(input_key))
        .layer(Extension(instances))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DossierQuery {
    instance: String,
    entity_id: u32,
}

async fn target_dossier(
    State(sessions): State<Arc<SessionStore>>,
    Extension(instances): Extension<Instances>,
    headers: HeaderMap,
    Query(query): Query<DossierQuery>,
) -> Response {
    let Some(instance) = bound_instance(&sessions, &headers) else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    if !instance.eq_ignore_ascii_case(&query.instance) {
        return StatusCode::FORBIDDEN.into_response();
    }
    let Some(hosted) = instances.by_label(&instance) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    Json(dossier::read_dossier(&hosted.supervisor, &hosted.label, query.entity_id).await)
        .into_response()
}

fn bound_instance(sessions: &SessionStore, headers: &HeaderMap) -> Option<String> {
    let cookie = cookie_value(headers)?;
    sessions.identity(cookie).map(|identity| identity.instance)
}

fn reply(instance: &str, data: Value, gaps: Vec<Value>) -> Json<Value> {
    Json(json!({
        "schema_version": 1,
        "context": { "instance": instance },
        "data": data,
        "gaps": gaps,
    }))
}

/// Serve the shell with only the session's validated instance label embedded.
pub async fn index(State(sessions): State<Arc<SessionStore>>, headers: HeaderMap) -> Response {
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
pub async fn javascript() -> Response {
    static_asset("text/javascript; charset=utf-8", JAVASCRIPT)
}

/// Serve the bundled stylesheet through the browser session gate.
pub async fn stylesheet() -> Response {
    static_asset("text/css; charset=utf-8", STYLESHEET)
}

async fn session(
    State(sessions): State<Arc<SessionStore>>,
    Extension(instances): Extension<Instances>,
    headers: HeaderMap,
) -> Response {
    let Some(cookie) = cookie_value(&headers) else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    let Some(identity) = sessions.identity(cookie) else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    let Some(hosted) = instances.by_label(&identity.instance) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let client = hosted.supervisor.status().await;
    let (client, gaps) = match client {
        Ok(value) => (Some(value), Vec::new()),
        Err(reason) => (None, vec![json!({ "source": "client", "reason": reason })]),
    };
    reply(
        &hosted.label,
        json!({
            "instance": hosted.label,
            "csrf": identity.csrf,
            "client": client,
            "lease": hosted.supervisor.leases().status(),
        }),
        gaps,
    )
    .into_response()
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct InputKeyRequest {
    instance: String,
    lease_id: String,
    key: String,
    #[serde(default = "tap")]
    action: String,
    hold_ms: Option<u64>,
}

fn tap() -> String {
    "tap".to_owned()
}

async fn input_key(
    State(sessions): State<Arc<SessionStore>>,
    Extension(instances): Extension<Instances>,
    headers: HeaderMap,
    Json(body): Json<InputKeyRequest>,
) -> Response {
    let Some(instance) = bound_instance(&sessions, &headers) else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    if !instance.eq_ignore_ascii_case(&body.instance) {
        return (
            StatusCode::FORBIDDEN,
            reply(&instance, json!({ "error": "instance_mismatch" }), vec![]),
        )
            .into_response();
    }
    let Some(hosted) = instances.by_label(&instance) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    if body.key.is_empty()
        || body.key.len() > 24
        || !matches!(body.action.as_str(), "tap" | "down" | "up")
        || body.hold_ms.is_some_and(|ms| ms > 2_000)
    {
        return (
            StatusCode::BAD_REQUEST,
            reply(&instance, json!({ "error": "input_invalid" }), vec![]),
        )
            .into_response();
    }
    let book = hosted.supervisor.leases().clone();
    if book
        .check(Some(&body.lease_id), "workbench.input_key")
        .is_err()
    {
        return (
            StatusCode::FORBIDDEN,
            reply(&instance, json!({ "error": "lease_invalid" }), vec![]),
        )
            .into_response();
    }
    let result = permit::scope(
        Permit::Lease {
            book,
            id: body.lease_id,
        },
        hosted
            .supervisor
            .input_key(&body.key, &body.action, body.hold_ms),
    )
    .await;
    match result {
        Ok(value) => reply(
            &hosted.label,
            json!({ "instance": hosted.label, "result": value }),
            vec![],
        )
        .into_response(),
        Err(reason) => (
            StatusCode::CONFLICT,
            reply(
                &hosted.label,
                json!({ "instance": hosted.label, "error": "input_failed", "detail": reason }),
                vec![],
            ),
        )
            .into_response(),
    }
}

#[cfg(test)]
#[path = "api_tests.rs"]
mod tests;
