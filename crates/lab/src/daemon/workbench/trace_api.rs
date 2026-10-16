//! Cookie-gated, passive read of one bounded causal trace.

use std::sync::Arc;

use axum::{
    extract::{Extension, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use serde_json::json;

use super::{bound_instance, SessionStore};
use crate::{server::Instances, workbench::causal};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TraceQuery {
    instance: String,
    click_id: String,
}

/// Read the shared event store without consuming another reader's cursor.
pub(super) async fn trace(
    State(sessions): State<Arc<SessionStore>>,
    Extension(instances): Extension<Instances>,
    headers: HeaderMap,
    Query(query): Query<TraceQuery>,
) -> Response {
    let Some(instance) = bound_instance(&sessions, &headers) else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    if !instance.eq_ignore_ascii_case(&query.instance) {
        return StatusCode::FORBIDDEN.into_response();
    }
    if query.click_id.is_empty()
        || query.click_id.len() > 160
        || !query
            .click_id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_:".contains(&b))
    {
        return StatusCode::BAD_REQUEST.into_response();
    }
    let Some(hosted) = instances.by_label(&instance) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let (snapshot, dropped) = hosted.supervisor.trace_snapshot(0, 8192).await;
    let trace = causal::assemble(
        &query.click_id,
        &snapshot.events,
        &[],
        snapshot.gap || dropped > 0,
    );
    let mut gaps = Vec::new();
    if snapshot.gap {
        gaps.push(json!({"source":"client_event_ring","code":"history_evicted"}));
    }
    if dropped > 0 {
        gaps.push(
            json!({"source":"client_event_ring","code":"bridge_dropped","count_total":dropped}),
        );
    }
    Json(json!({
        "schema_version": 1,
        "context": {"instance": hosted.label},
        "data": trace,
        "gaps": gaps,
    }))
    .into_response()
}

#[cfg(test)]
mod tests {
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };

    use reqwest::header::COOKIE;
    use serde_json::Value;

    use crate::{
        daemon::http_tests::spawn_router,
        server::{Hosted, Instances},
        supervisor::events::fake_bridge::{self, Responder},
    };

    use super::SessionStore;

    #[tokio::test]
    async fn trace_route_is_cookie_and_instance_bound_and_passive() {
        let calls = Arc::new(AtomicUsize::new(0));
        let counted = calls.clone();
        let responder: Responder = Arc::new(move |_, _| {
            counted.fetch_add(1, Ordering::SeqCst);
            Err("a trace read must not call the bridge".into())
        });
        let instances = Instances::new(vec![Hosted {
            label: "default".into(),
            supervisor: Arc::new(fake_bridge::supervisor(responder).await),
        }]);
        let sessions = Arc::new(SessionStore::default());
        let cookie = sessions.redeem(&sessions.open("default")).unwrap().cookie;
        let router = super::super::router(instances)
            .route_layer(axum::middleware::from_fn_with_state(
                sessions.clone(),
                super::super::super::require_session,
            ))
            .with_state(sessions);
        let base = spawn_router(router).await.replace("/mcp", "");
        let http = reqwest::Client::new();
        let good = format!("{base}/workbench/api/v1/trace?instance=default&click_id=E:1");
        assert_eq!(
            http.get(&good).send().await.unwrap().status(),
            reqwest::StatusCode::UNAUTHORIZED
        );
        let wrong = format!("{base}/workbench/api/v1/trace?instance=p2&click_id=E:1");
        assert_eq!(
            http.get(&wrong)
                .header(COOKIE, format!("cimmeria_workbench={cookie}"))
                .send()
                .await
                .unwrap()
                .status(),
            reqwest::StatusCode::FORBIDDEN
        );
        let result: Value = http
            .get(&good)
            .header(COOKIE, format!("cimmeria_workbench={cookie}"))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        assert_eq!(result["data"]["first_unobserved_stage"], "UiEvent");
        assert_eq!(result["data"]["stages"][0]["status"], "unknown");
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }
}
