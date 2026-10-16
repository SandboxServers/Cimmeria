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

#[path = "probe_api.rs"]
mod probe_api;
use crate::{
    lease::permit::{self, Permit},
    server::Instances,
    workbench::{
        bookmarks::{BookmarkDraft, BookmarkStore},
        dossier, spatial,
    },
};
#[cfg(test)]
pub(crate) use probe_api::leased;

const INDEX: &str = include_str!("../../../workbench/index.html");
const JAVASCRIPT: &str = include_str!("../../../workbench/app.js");
const STYLESHEET: &str = include_str!("../../../workbench/app.css");

/// Routes merged into the DW-02A browser router before its session layer.
pub fn router(instances: Instances) -> Router<Arc<SessionStore>> {
    Router::new()
        .route("/workbench/api/v1/session", get(session))
        .route("/workbench/api/v1/dossier", get(target_dossier))
        .route("/workbench/api/v1/spatial", get(spatial_frame))
        .route("/workbench/api/v1/bookmarks", get(bookmark_list))
        .route("/workbench/api/v1/bookmarks/save", post(bookmark_save))
        .route("/workbench/api/v1/bookmarks/delete", post(bookmark_delete))
        .route("/workbench/api/v1/bookmarks/action", post(bookmark_action))
        .route("/workbench/api/v1/input/key", post(input_key))
        .route("/workbench/api/v1/probes", get(probe_api::probe_list))
        .route("/workbench/api/v1/probes/save", post(probe_api::probe_save))
        .route("/workbench/api/v1/probes/run", post(probe_api::probe_run))
        .route(
            "/workbench/api/v1/probes/globals",
            post(probe_api::probe_globals),
        )
        .route(
            "/workbench/api/v1/probes/window",
            post(probe_api::probe_window),
        )
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

fn bookmark_store() -> BookmarkStore {
    BookmarkStore::new(crate::daemon::config::state_dir())
}

fn spatial_error(instance: &str, status: StatusCode, code: &str, detail: &str) -> Response {
    tracing::warn!(target: "lab.workbench.spatial", instance, status = status.as_u16(), code, "spatial request failed");
    let safe_detail = match code {
        "instance_mismatch"
        | "bookmark_missing"
        | "lease_invalid"
        | "cross_world_or_unknown"
        | "cross_space_or_unknown" => detail,
        "bookmark_action_unavailable" => {
            "bookmark actions require a trusted client identity and guarded input boundary"
        }
        _ => "spatial operation failed; inspect the structured error code",
    };
    (
        status,
        reply(
            instance,
            json!({"error":code,"detail":safe_detail}),
            if code == "bookmark_action_unavailable" {
                vec![json!({"source":"client_session","code":"session_identity_unbound"})]
            } else {
                vec![]
            },
        ),
    )
        .into_response()
}

fn request_instance(
    sessions: &SessionStore,
    headers: &HeaderMap,
    requested: &str,
) -> Result<String, Box<Response>> {
    let instance = bound_instance(sessions, headers)
        .ok_or_else(|| Box::new(StatusCode::UNAUTHORIZED.into_response()))?;
    if !instance.eq_ignore_ascii_case(requested) {
        return Err(Box::new(spatial_error(
            &instance,
            StatusCode::FORBIDDEN,
            "instance_mismatch",
            "browser session belongs to another instance",
        )));
    }
    Ok(instance)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SpatialQuery {
    instance: String,
}

async fn spatial_frame(
    State(sessions): State<Arc<SessionStore>>,
    Extension(instances): Extension<Instances>,
    headers: HeaderMap,
    Query(query): Query<SpatialQuery>,
) -> Response {
    let instance = match request_instance(&sessions, &headers, &query.instance) {
        Ok(v) => v,
        Err(r) => return *r,
    };
    let Some(hosted) = instances.by_label(&instance) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    Json(spatial::read_spatial(&hosted.supervisor, &hosted.label).await).into_response()
}

async fn bookmark_list(
    State(sessions): State<Arc<SessionStore>>,
    headers: HeaderMap,
    Query(query): Query<SpatialQuery>,
) -> Response {
    let instance = match request_instance(&sessions, &headers, &query.instance) {
        Ok(v) => v,
        Err(r) => return *r,
    };
    match bookmark_store().list(&instance) {
        Ok(rows) => reply(&instance, json!({"bookmarks":rows}), vec![]).into_response(),
        Err(reason) => spatial_error(
            &instance,
            StatusCode::INTERNAL_SERVER_ERROR,
            "bookmark_read_failed",
            &reason,
        ),
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BookmarkSaveRequest {
    instance: String,
    id: Option<String>,
    bookmark: BookmarkDraft,
}

async fn bookmark_save(
    State(sessions): State<Arc<SessionStore>>,
    headers: HeaderMap,
    Json(body): Json<BookmarkSaveRequest>,
) -> Response {
    let instance = match request_instance(&sessions, &headers, &body.instance) {
        Ok(v) => v,
        Err(r) => return *r,
    };
    match bookmark_store().save(&instance, body.id, body.bookmark) {
        Ok(row) => reply(&instance, json!({"bookmark":row}), vec![]).into_response(),
        Err(reason) => spatial_error(
            &instance,
            StatusCode::BAD_REQUEST,
            "bookmark_save_failed",
            &reason,
        ),
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BookmarkDeleteRequest {
    instance: String,
    id: String,
}

async fn bookmark_delete(
    State(sessions): State<Arc<SessionStore>>,
    headers: HeaderMap,
    Json(body): Json<BookmarkDeleteRequest>,
) -> Response {
    let instance = match request_instance(&sessions, &headers, &body.instance) {
        Ok(v) => v,
        Err(r) => return *r,
    };
    match bookmark_store().delete(&instance, &body.id) {
        Ok(()) => reply(&instance, json!({"deleted":body.id}), vec![]).into_response(),
        Err(reason) => spatial_error(
            &instance,
            StatusCode::NOT_FOUND,
            "bookmark_delete_failed",
            &reason,
        ),
    }
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
enum BookmarkAction {
    Click,
    Teleport,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BookmarkActionRequest {
    instance: String,
    id: String,
    lease_id: String,
    action: BookmarkAction,
}

async fn bookmark_action(
    State(sessions): State<Arc<SessionStore>>,
    Extension(instances): Extension<Instances>,
    headers: HeaderMap,
    Json(body): Json<BookmarkActionRequest>,
) -> Response {
    let store = bookmark_store();
    bookmark_action_with_store(sessions, instances, headers, body, &store).await
}

async fn bookmark_action_with_store(
    sessions: Arc<SessionStore>,
    instances: Instances,
    headers: HeaderMap,
    body: BookmarkActionRequest,
    store: &BookmarkStore,
) -> Response {
    let instance = match request_instance(&sessions, &headers, &body.instance) {
        Ok(v) => v,
        Err(r) => return *r,
    };
    let Some(hosted) = instances.by_label(&instance) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let row = match store.list(&instance) {
        Ok(rows) => match rows.into_iter().find(|row| row.id == body.id) {
            Some(row) => row,
            None => {
                return spatial_error(
                    &instance,
                    StatusCode::NOT_FOUND,
                    "bookmark_missing",
                    "bookmark id is not saved",
                )
            }
        },
        Err(reason) => {
            return spatial_error(
                &instance,
                StatusCode::INTERNAL_SERVER_ERROR,
                "bookmark_read_failed",
                &reason,
            )
        }
    };
    let book = hosted.supervisor.leases().clone();
    if book
        .check(Some(&body.lease_id), "workbench.bookmark_action")
        .is_err()
    {
        return spatial_error(
            &instance,
            StatusCode::FORBIDDEN,
            "lease_invalid",
            "a current lab lease is required",
        );
    }
    let action = match body.action {
        BookmarkAction::Click => "click",
        BookmarkAction::Teleport => "teleport",
    };
    tracing::warn!(target: "lab.workbench.spatial", instance, action, bookmark_id = %row.id, "bookmark action refused without guarded client identity");
    spatial_error(
        &instance,
        StatusCode::CONFLICT,
        "bookmark_action_unavailable",
        "bookmark actions require a trusted client identity and guarded input boundary",
    )
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
