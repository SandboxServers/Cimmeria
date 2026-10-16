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
    supervisor::ui::window_read::WalkOptions,
    workbench::{
        dossier,
        probe_catalog::{ProbeCatalog, ProbeDefinition, ProbeMode},
        probes,
    },
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
        .route("/workbench/api/v1/probes", get(probe_list))
        .route("/workbench/api/v1/probes/save", post(probe_save))
        .route("/workbench/api/v1/probes/run", post(probe_run))
        .route("/workbench/api/v1/probes/globals", post(probe_globals))
        .route("/workbench/api/v1/probes/window", post(probe_window))
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

fn catalog() -> ProbeCatalog {
    ProbeCatalog::new(crate::daemon::config::state_dir())
}

fn probe_error(instance: &str, status: StatusCode, code: &'static str, detail: String) -> Response {
    (
        status,
        reply(instance, json!({"error":code,"detail":detail}), vec![]),
    )
        .into_response()
}

fn probe_instance(
    sessions: &SessionStore,
    headers: &HeaderMap,
    body: &str,
) -> Result<String, Box<Response>> {
    let instance = bound_instance(sessions, headers)
        .ok_or_else(|| Box::new(StatusCode::UNAUTHORIZED.into_response()))?;
    if !instance.eq_ignore_ascii_case(body) {
        return Err(Box::new(probe_error(
            &instance,
            StatusCode::FORBIDDEN,
            "instance_mismatch",
            "browser session is bound to a different instance".into(),
        )));
    }
    Ok(instance)
}

async fn probe_list(State(sessions): State<Arc<SessionStore>>, headers: HeaderMap) -> Response {
    let Some(instance) = bound_instance(&sessions, &headers) else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    let data = catalog().list(&instance).and_then(|definitions| {
        catalog()
            .history(&instance)
            .map(|history| json!({"definitions":definitions,"history":history}))
    });
    match data {
        Ok(data) => reply(&instance, data, vec![]).into_response(),
        Err(e) => probe_error(
            &instance,
            StatusCode::INTERNAL_SERVER_ERROR,
            "catalog_error",
            e,
        ),
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ProbeSaveRequest {
    instance: String,
    name: String,
    mode: ProbeMode,
    chunk: String,
    expected_sha256: Option<String>,
    #[serde(default)]
    favorite: bool,
}

async fn probe_save(
    State(sessions): State<Arc<SessionStore>>,
    headers: HeaderMap,
    Json(body): Json<ProbeSaveRequest>,
) -> Response {
    let instance = match probe_instance(&sessions, &headers, &body.instance) {
        Ok(v) => v,
        Err(r) => return *r,
    };
    let catalog = catalog();
    let mut def = match ProbeDefinition::new(body.name, body.mode, body.chunk) {
        Ok(v) => v,
        Err(e) => return probe_error(&instance, StatusCode::BAD_REQUEST, "probe_invalid", e),
    };
    def.favorite = body.favorite;
    let old = match catalog.get(&instance, &def.name) {
        Ok(v) => v,
        Err(e) => {
            return probe_error(
                &instance,
                StatusCode::INTERNAL_SERVER_ERROR,
                "catalog_error",
                e,
            )
        }
    };
    if old.as_ref().map(|v| v.sha256.as_str()) != body.expected_sha256.as_deref() {
        return probe_error(
            &instance,
            StatusCode::CONFLICT,
            "probe_stale",
            "saved definition changed; reload before saving".into(),
        );
    }
    match catalog.save(&instance, def) {
        Ok(saved) => reply(&instance, json!({"definition": saved}), vec![]).into_response(),
        Err(e) => probe_error(
            &instance,
            StatusCode::INTERNAL_SERVER_ERROR,
            "catalog_error",
            e,
        ),
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ProbeRunRequest {
    instance: String,
    lease_id: String,
    name: String,
    sha256: String,
    #[serde(default)]
    parameters: serde_json::Map<String, Value>,
}

pub(crate) fn leased<'a>(
    instances: &'a Instances,
    instance: &str,
    lease_id: &str,
    action: &str,
) -> Result<(&'a crate::server::Hosted, Permit), Box<Response>> {
    let hosted = instances
        .by_label(instance)
        .ok_or_else(|| Box::new(StatusCode::NOT_FOUND.into_response()))?;
    let book = hosted.supervisor.leases().clone();
    if book.check(Some(lease_id), action).is_err() {
        return Err(Box::new(probe_error(
            instance,
            StatusCode::FORBIDDEN,
            "lease_invalid",
            "a current lab lease is required".into(),
        )));
    }
    Ok((
        hosted,
        Permit::Lease {
            book,
            id: lease_id.to_owned(),
        },
    ))
}

async fn probe_run(
    State(sessions): State<Arc<SessionStore>>,
    Extension(instances): Extension<Instances>,
    headers: HeaderMap,
    Json(body): Json<ProbeRunRequest>,
) -> Response {
    let instance = match probe_instance(&sessions, &headers, &body.instance) {
        Ok(v) => v,
        Err(r) => return *r,
    };
    let (hosted, permit) = match leased(&instances, &instance, &body.lease_id, "client_lua_eval") {
        Ok(v) => v,
        Err(r) => return *r,
    };
    let def = match catalog().get(&instance, &body.name) {
        Ok(Some(v)) => v,
        Ok(None) => {
            return probe_error(&instance, StatusCode::NOT_FOUND, "probe_missing", body.name)
        }
        Err(e) => {
            return probe_error(
                &instance,
                StatusCode::INTERNAL_SERVER_ERROR,
                "catalog_error",
                e,
            )
        }
    };
    if def.sha256 != body.sha256 {
        return probe_error(
            &instance,
            StatusCode::CONFLICT,
            "probe_stale",
            "probe hash changed; reload before running".into(),
        );
    }
    let run = permit::scope(
        permit,
        probes::execute(&def, &body.parameters, |chunk| {
            hosted
                .supervisor
                .bridge_call("lua_eval", json!({"chunk":chunk}))
        }),
    )
    .await;
    match run {
        Ok(result) => {
            let _ = catalog().record(&instance, &def.name, result.status);
            reply(&instance, json!({"definition":{"name":def.name,"mode":def.mode,"sha256":def.sha256},"result":result}), vec![]).into_response()
        }
        Err(e) => probe_error(&instance, StatusCode::BAD_REQUEST, "parameters_invalid", e),
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ProbeBrowseRequest {
    instance: String,
    lease_id: String,
    name: String,
}

async fn probe_globals(
    State(sessions): State<Arc<SessionStore>>,
    Extension(instances): Extension<Instances>,
    headers: HeaderMap,
    Json(body): Json<ProbeBrowseRequest>,
) -> Response {
    let instance = match probe_instance(&sessions, &headers, &body.instance) {
        Ok(v) => v,
        Err(r) => return *r,
    };
    let (hosted, permit) = match leased(&instances, &instance, &body.lease_id, "client_lua_eval") {
        Ok(v) => v,
        Err(r) => return *r,
    };
    let chunk = match probes::globals_chunk(&body.name) {
        Ok(v) => v,
        Err(e) => return probe_error(&instance, StatusCode::BAD_REQUEST, "global_invalid", e),
    };
    let result = permit::scope(
        permit,
        probes::execute_raw(chunk, |chunk| {
            hosted
                .supervisor
                .bridge_call("lua_eval", json!({"chunk":chunk}))
        }),
    )
    .await;
    match result {
        Ok(value) => reply(
            &instance,
            json!({"name":body.name,"result":value,"source":"client_lua_eval"}),
            vec![],
        )
        .into_response(),
        Err(e) => probe_error(&instance, StatusCode::BAD_REQUEST, "global_invalid", e),
    }
}

async fn probe_window(
    State(sessions): State<Arc<SessionStore>>,
    Extension(instances): Extension<Instances>,
    headers: HeaderMap,
    Json(body): Json<ProbeBrowseRequest>,
) -> Response {
    let instance = match probe_instance(&sessions, &headers, &body.instance) {
        Ok(v) => v,
        Err(r) => return *r,
    };
    if body.name.is_empty() || body.name.len() > 128 {
        return probe_error(
            &instance,
            StatusCode::BAD_REQUEST,
            "window_invalid",
            "window name must be 1..128 bytes".into(),
        );
    }
    let (hosted, permit) = match leased(&instances, &instance, &body.lease_id, "client_window_read")
    {
        Ok(v) => v,
        Err(r) => return *r,
    };
    let opts = WalkOptions {
        max_depth: 4,
        max_nodes: 128,
        include_hidden: false,
        max_rows: 32,
    };
    let result = permit::scope(
        permit,
        tokio::time::timeout(
            std::time::Duration::from_secs(3),
            hosted.supervisor.window_read(&body.name, opts, true),
        ),
    )
    .await;
    match result {
        Ok(Ok(value)) => {
            if serde_json::to_vec(&value).is_ok_and(|bytes| bytes.len() > probes::OUTPUT_CAP) {
                return reply(
                    &instance,
                    json!({"name":body.name,"source":"client_window_read","result":{"truncated":true,"reason":"output_limit"}}),
                    vec![json!({"source":"client_window_read","reason":"output_limit"})],
                ).into_response();
            }
            reply(
                &instance,
                json!({"name":body.name,"source":"client_window_read","result":value}),
                vec![],
            )
            .into_response()
        }
        Ok(Err(e)) => reply(
            &instance,
            json!({"name":body.name,"error":"window_read_failed","detail":e}),
            vec![json!({"source":"client_window_read","reason":"read_failed"})],
        )
        .into_response(),
        Err(_) => reply(
            &instance,
            json!({"name":body.name,"error":"timeout"}),
            vec![json!({"source":"client_window_read","reason":"execution_unknown"})],
        )
        .into_response(),
    }
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
