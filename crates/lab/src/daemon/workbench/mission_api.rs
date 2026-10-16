//! Read-only selected-mission API route and server source binding.

use std::sync::Arc;

#[cfg(test)]
use axum::Router;
use axum::{
    extract::{Extension, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;

use super::{bound_instance, SessionStore};
use crate::{
    server::Instances,
    uat::invoke::{ServerInvoker, ServerTools},
    workbench::mission,
};

/// Obtain the configured read-only server tools source, if available.
pub(super) fn default_server() -> Option<Arc<dyn ServerInvoker>> {
    ServerTools::from_env().map(|source| Arc::new(source) as Arc<dyn ServerInvoker>)
}

#[derive(Clone)]
/// Optional server tools source for the selected-mission route.
pub(super) struct MissionServer(pub(super) Option<Arc<dyn ServerInvoker>>);

#[cfg(test)]
fn router_with_mission_server(
    instances: Instances,
    server: Option<Arc<dyn ServerInvoker>>,
) -> Router<Arc<SessionStore>> {
    super::router_with_sources(instances, super::bookmark_store(), server)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
/// Selected mission and browser instance from the read-only route.
pub(super) struct MissionQuery {
    instance: String,
    mission_id: i32,
}

/// Read independent evidence for the selected mission without mutating state.
pub(super) async fn mission_xray(
    State(sessions): State<Arc<SessionStore>>,
    Extension(instances): Extension<Instances>,
    Extension(MissionServer(server)): Extension<MissionServer>,
    headers: HeaderMap,
    Query(query): Query<MissionQuery>,
) -> Response {
    let Some(instance) = bound_instance(&sessions, &headers) else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    if !instance.eq_ignore_ascii_case(&query.instance) {
        return StatusCode::FORBIDDEN.into_response();
    }
    if query.mission_id <= 0 {
        return StatusCode::BAD_REQUEST.into_response();
    }
    let Some(hosted) = instances.by_label(&instance) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    Json(
        mission::read_mission(
            &hosted.supervisor,
            &hosted.label,
            query.mission_id,
            None,
            server.as_deref(),
        )
        .await,
    )
    .into_response()
}

#[cfg(test)]
#[path = "mission_api_tests.rs"]
mod tests;
