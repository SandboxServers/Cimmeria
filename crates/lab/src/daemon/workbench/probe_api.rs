//! Probe catalog and leased probe execution routes.

use std::sync::Arc;

use axum::{
    extract::{Extension, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};

use super::{bound_instance, reply, SessionStore};
use crate::{
    lease::permit::{self, Permit},
    server::Instances,
    supervisor::ui::window_read::WalkOptions,
    workbench::{
        probe_catalog::{ProbeCatalog, ProbeDefinition, ProbeMode},
        probes,
    },
};

fn catalog() -> ProbeCatalog {
    ProbeCatalog::new(crate::daemon::config::state_dir())
}

fn probe_error(instance: &str, status: StatusCode, code: &'static str, detail: String) -> Response {
    tracing::warn!(target: "lab.workbench.probes", status = status.as_u16(), code, "probe request failed");
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

/// List definitions and metadata-only run history for the bound instance.
pub(super) async fn probe_list(
    State(sessions): State<Arc<SessionStore>>,
    headers: HeaderMap,
) -> Response {
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
/// Version-checked probe save request.
pub(super) struct ProbeSaveRequest {
    instance: String,
    name: String,
    mode: ProbeMode,
    chunk: String,
    expected_sha256: Option<String>,
    #[serde(default)]
    favorite: bool,
}

/// Save a definition after matching the browser session and expected hash.
pub(super) async fn probe_save(
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
    match catalog.save_checked(&instance, def, body.expected_sha256.as_deref(), true) {
        Ok(saved) => reply(&instance, json!({"definition": saved}), vec![]).into_response(),
        Err(e) if e == "probe_stale" => probe_error(
            &instance,
            StatusCode::CONFLICT,
            "probe_stale",
            "saved definition changed; reload before saving".into(),
        ),
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
/// Leased execution request for one saved probe version.
pub(super) struct ProbeRunRequest {
    instance: String,
    lease_id: String,
    name: String,
    sha256: String,
    #[serde(default)]
    parameters: serde_json::Map<String, Value>,
}

/// Bind an action to one hosted instance and a current lab lease.
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

/// Run a named probe under the current lease.
pub(super) async fn probe_run(
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
            let gaps = if catalog()
                .record(&instance, &def.name, result.status)
                .is_err()
            {
                tracing::warn!(target: "lab.workbench.probes", reason = "history_write_failed", "probe metadata could not be saved");
                vec![json!({"source":"probe_history","reason":"write_failed"})]
            } else {
                vec![]
            };
            reply(&instance, json!({"definition":{"name":def.name,"mode":def.mode,"sha256":def.sha256},"result":result}), gaps).into_response()
        }
        Err(e) => probe_error(&instance, StatusCode::BAD_REQUEST, "parameters_invalid", e),
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
/// Leased request for a bounded object read.
pub(super) struct ProbeBrowseRequest {
    instance: String,
    lease_id: String,
    name: String,
}

/// Read one named Lua global through the bounded raw-access tree.
pub(super) async fn probe_globals(
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

/// Read one CEGUI window through the existing bounded walker.
pub(super) async fn probe_window(
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
