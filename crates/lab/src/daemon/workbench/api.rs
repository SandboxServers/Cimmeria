//! Cookie-gated, typed HTTP facade for the local companion window.

use std::sync::Arc;

use axum::{
    extract::{Extension, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};

use super::{bound_instance, cookie_value, session::SessionStore};

#[path = "mission_api.rs"]
mod mission_api;
#[path = "probe_api.rs"]
mod probe_api;
#[path = "trace_api.rs"]
mod trace_api;
use crate::{
    lease::permit::{self, Permit},
    server::Instances,
    uat::invoke::ServerInvoker,
    workbench::{
        bookmarks::{BookmarkDraft, BookmarkStore},
        dossier, spatial,
    },
};
#[cfg(test)]
pub(crate) use probe_api::leased;

use mission_api::MissionServer;

/// Routes merged into the DW-02A browser router before its session layer.
pub fn router(instances: Instances) -> Router<Arc<SessionStore>> {
    router_with_sources(instances, bookmark_store(), mission_api::default_server())
}

#[cfg(test)]
pub(crate) fn router_with_store(
    instances: Instances,
    store: BookmarkStore,
) -> Router<Arc<SessionStore>> {
    router_with_sources(instances, store, mission_api::default_server())
}

fn router_with_sources(
    instances: Instances,
    store: BookmarkStore,
    server: Option<Arc<dyn ServerInvoker>>,
) -> Router<Arc<SessionStore>> {
    Router::new()
        .route("/workbench/api/v1/session", get(session))
        .route("/workbench/api/v1/dossier", get(target_dossier))
        .route("/workbench/api/v1/spatial", get(spatial_frame))
        .route("/workbench/api/v1/bookmarks", get(bookmark_list))
        .route("/workbench/api/v1/bookmarks/save", post(bookmark_save))
        .route("/workbench/api/v1/bookmarks/delete", post(bookmark_delete))
        .route("/workbench/api/v1/bookmarks/action", post(bookmark_action))
        .route("/workbench/api/v1/missions", get(mission_api::mission_xray))
        .route("/workbench/api/v1/trace", get(trace_api::trace))
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
        .layer(Extension(store))
        .layer(Extension(instances))
        .layer(Extension(MissionServer(server)))
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

fn storage_failure_category(reason: &str) -> &'static str {
    let lower = reason.to_ascii_lowercase();
    if lower.contains("(os error 5)")
        || lower.contains("(os error 13)")
        || lower.contains("permission denied")
        || lower.contains("access is denied")
    {
        "storage_permission_denied"
    } else if lower.contains("(os error 112)")
        || lower.contains("(os error 28)")
        || lower.contains("no space left")
        || lower.contains("disk is full")
    {
        "storage_disk_full"
    } else if lower.contains(" at line ") && lower.contains(" column ") {
        "storage_corrupt_json"
    } else if matches!(
        reason,
        "bookmark file too large" | "bookmark count exceeds 256"
    ) {
        "storage_limit_exceeded"
    } else if lower.contains("invalid") || lower.contains("mismatch") || lower.contains("missing") {
        "bookmark_validation_failed"
    } else {
        "storage_other"
    }
}

fn spatial_error(instance: &str, status: StatusCode, code: &str, detail: &str) -> Response {
    let category = if matches!(
        code,
        "bookmark_read_failed" | "bookmark_save_failed" | "bookmark_delete_failed"
    ) {
        storage_failure_category(detail)
    } else if code == "bookmark_action_unavailable" {
        "session_identity_unbound"
    } else {
        code
    };
    tracing::warn!(target: "lab.workbench.spatial", instance, status = status.as_u16(), code, category, "spatial request failed");
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
            json!({"error":code,"category":category,"detail":safe_detail}),
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
    Extension(store): Extension<BookmarkStore>,
    headers: HeaderMap,
    Query(query): Query<SpatialQuery>,
) -> Response {
    let instance = match request_instance(&sessions, &headers, &query.instance) {
        Ok(v) => v,
        Err(r) => return *r,
    };
    match store.list(&instance) {
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
    Extension(store): Extension<BookmarkStore>,
    headers: HeaderMap,
    Json(body): Json<BookmarkSaveRequest>,
) -> Response {
    let instance = match request_instance(&sessions, &headers, &body.instance) {
        Ok(v) => v,
        Err(r) => return *r,
    };
    match store.save(&instance, body.id, body.bookmark) {
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
    Extension(store): Extension<BookmarkStore>,
    headers: HeaderMap,
    Json(body): Json<BookmarkDeleteRequest>,
) -> Response {
    let instance = match request_instance(&sessions, &headers, &body.instance) {
        Ok(v) => v,
        Err(r) => return *r,
    };
    match store.delete(&instance, &body.id) {
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
    Extension(store): Extension<BookmarkStore>,
    headers: HeaderMap,
    Json(body): Json<BookmarkActionRequest>,
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

fn reply(instance: &str, data: Value, gaps: Vec<Value>) -> Json<Value> {
    Json(json!({
        "schema_version": 1,
        "context": { "instance": instance },
        "data": data,
        "gaps": gaps,
    }))
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
