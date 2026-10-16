//! Read-only mission evidence across definition, server, wire, client and UI layers.

use serde_json::{json, Value};

use crate::supervisor::Supervisor;
use crate::uat::invoke::{ServerInvoker, ServerTools};
use crate::workbench::context::CaptureContext;

/// One independently timed source reply.
#[derive(Debug, Clone)]
pub struct Sample {
    pub at_ms: i64,
    pub value: Value,
}

/// Inputs to the pure mission read model. A missing source is never synthesized.
#[derive(Debug, Clone)]
pub struct MissionInput {
    pub instance: String,
    pub mission_id: i32,
    pub definition: Result<Sample, String>,
    pub steps: Result<Sample, String>,
    pub chains: Result<Sample, String>,
    pub server: Result<Sample, String>,
    pub wire: Result<Sample, String>,
    pub client: Result<Sample, String>,
    pub ui: Result<Sample, String>,
    pub now_ms: i64,
}

fn layer(
    source: &str,
    input: &Result<Sample, String>,
    now_ms: i64,
    gaps: &mut Vec<Value>,
) -> Value {
    match input {
        Ok(sample) => json!({
            "source": source,
            "at_ms": sample.at_ms,
            "age_ms": now_ms.saturating_sub(sample.at_ms).max(0),
            "value": sample.value,
        }),
        Err(reason) => {
            gaps.push(json!({"source": source, "code": "unavailable", "detail": reason}));
            json!({"source": source, "at_ms": null, "age_ms": null, "value": null})
        }
    }
}

fn row(input: &Result<Sample, String>) -> Option<&Value> {
    input.as_ref().ok().map(|sample| &sample.value)
}

fn tracker_for(ui: &Value, label: &str) -> Option<Value> {
    let rows = ui.get("mission_tracker")?.as_array()?;
    let mut matches = Vec::new();
    let mut title: Option<&str> = None;
    let mut step: Option<&str> = None;
    for entry in rows {
        match entry.get("kind").and_then(Value::as_str) {
            Some("mission") => {
                if title == Some(label) {
                    matches.push(json!({"title": label, "step_text": step}));
                }
                title = entry.get("text").and_then(Value::as_str);
                step = None;
            }
            Some("step") if step.is_none() => step = entry.get("text").and_then(Value::as_str),
            _ => {}
        }
    }
    if title == Some(label) {
        matches.push(json!({"title": label, "step_text": step}));
    }
    // Duplicated titles are ambiguous even though the text is identical.
    (matches.len() == 1).then(|| matches.remove(0))
}

/// Compose the five layers without promoting UI text to server state.
pub fn build_mission(input: MissionInput) -> Value {
    let mut gaps = Vec::new();
    let definition = layer(
        "resources.missions",
        &input.definition,
        input.now_ms,
        &mut gaps,
    );
    let steps = layer(
        "resources.mission_steps",
        &input.steps,
        input.now_ms,
        &mut gaps,
    );
    let chains = layer(
        "resources.content_chains",
        &input.chains,
        input.now_ms,
        &mut gaps,
    );
    let server = layer("sgw_mission", &input.server, input.now_ms, &mut gaps);
    let wire = layer("server_packet_tap", &input.wire, input.now_ms, &mut gaps);
    let client = layer(
        "client.mission_state",
        &input.client,
        input.now_ms,
        &mut gaps,
    );
    let ui = layer(
        "client_ui_state.mission_tracker",
        &input.ui,
        input.now_ms,
        &mut gaps,
    );
    let def_row = row(&input.definition).filter(|v| {
        v.get("mission_id").and_then(Value::as_i64) == Some(i64::from(input.mission_id))
    });
    if input.definition.is_ok() && def_row.is_none() {
        gaps.push(json!({"source": "resources.missions", "code": "row_absent", "detail": "selected mission has no definition row"}));
    }
    let server_row = row(&input.server).filter(|v| {
        v.get("mission_id").and_then(Value::as_i64) == Some(i64::from(input.mission_id))
    });
    if input.server.is_ok() && server_row.is_none() {
        gaps.push(json!({"source": "sgw_mission", "code": "row_absent", "detail": "no persisted row for the bound player and selected mission"}));
    }
    let step_id = server_row
        .filter(|v| v.get("status").and_then(Value::as_i64) == Some(1))
        .and_then(|v| v.get("current_step_id"))
        .and_then(Value::as_i64);
    let step_row = row(&input.steps)
        .and_then(Value::as_array)
        .and_then(|rows| {
            rows.iter().find(|v| {
                v.get("step_id").and_then(Value::as_i64) == step_id
                    && v.get("mission_id").and_then(Value::as_i64)
                        == Some(i64::from(input.mission_id))
            })
        });
    let tracker = def_row
        .and_then(|v| v.get("mission_defn"))
        .and_then(Value::as_str)
        .and_then(|label| row(&input.ui).and_then(|ui| tracker_for(ui, label)));
    if input.ui.is_ok() && tracker.is_none() {
        gaps.push(json!({"source": "client_ui_state.mission_tracker", "code": "mission_unbound", "detail": "selected mission title is absent or ambiguous in the visible tracker"}));
    }
    let server_text = step_row
        .and_then(|v| v.get("step_display_log_text"))
        .and_then(Value::as_str);
    let tracker_text = tracker
        .as_ref()
        .and_then(|v| v.get("step_text"))
        .and_then(Value::as_str);
    let disagreement = server_text
        .zip(tracker_text)
        .map(|(server, visible)| server != visible);
    if disagreement == Some(true) {
        gaps.push(json!({"source": "sgw_mission+client_ui_state", "code": "step_disagreement", "detail": "persisted active-step description differs from the visible tracker step"}));
    }
    json!({
        "schema_version": 1,
        "context": {"instance": input.instance, "mission_id": input.mission_id, "observed_at_ms": input.now_ms},
        "data": {
            "definition": definition,
            "steps": steps,
            "chain_references": chains,
            "server": server,
            "wire": wire,
            "client": client,
            "ui": ui,
            "matched_tracker": tracker,
            "active_step": {"id": step_id, "description": server_text},
            "step_disagreement": disagreement,
        },
        "gaps": gaps,
    })
}

async fn db_rows(server: &dyn ServerInvoker, sql: String) -> Result<Sample, String> {
    let reply = server.call("server_db_query", json!({"sql": sql})).await;
    if !reply.ok {
        return Err(reply
            .error
            .unwrap_or_else(|| "server_db_query failed".into()));
    }
    if reply.json.get("truncated").and_then(Value::as_bool) != Some(false) {
        return Err("server_db_query rows truncated or truncation status unknown".into());
    }
    let rows = reply
        .json
        .get("rows")
        .and_then(Value::as_array)
        .ok_or("server_db_query reply has no rows array")?;
    Ok(Sample {
        at_ms: crate::supervisor::now_ms(),
        value: json!(rows),
    })
}

async fn bound_player(
    server: &dyn ServerInvoker,
    context: Option<&CaptureContext>,
    instance: &str,
    pid: Option<u32>,
) -> Result<i64, String> {
    let context = context.ok_or("trusted client-to-server session binding unavailable")?;
    if context.instance != instance || Some(context.client_pid) != pid {
        return Err("capture instance or client PID changed".into());
    }
    let entity = context
        .player_entity_id
        .ok_or("bound player entity unavailable")?;
    let session = context
        .server_session_id
        .as_deref()
        .ok_or("bound server session unavailable")?;
    let reply = server.call("server_sessions", json!({})).await;
    if !reply.ok {
        return Err(reply
            .error
            .unwrap_or_else(|| "server_sessions failed".into()));
    }
    let rows = reply
        .json
        .get("sessions")
        .and_then(Value::as_array)
        .ok_or("server_sessions has no rows")?;
    let matching_session: Vec<_> = rows
        .iter()
        .filter(|v| v.get("session").and_then(Value::as_str) == Some(session))
        .collect();
    let matching_entity: Vec<_> = rows
        .iter()
        .filter(|v| v.get("entity_id").and_then(Value::as_u64) == Some(u64::from(entity)))
        .collect();
    if matching_session.len() != 1
        || matching_entity.len() != 1
        || matching_session[0] != matching_entity[0]
    {
        return Err("server session and player entity do not identify one roster row".into());
    }
    matching_session[0]
        .get("player_id")
        .and_then(Value::as_i64)
        .ok_or("bound server player ID unavailable".into())
}

/// Accept tracker rows only after the mission UI section completed without error.
pub(super) fn ui_sample(value: Value) -> Result<Sample, String> {
    let errors = value
        .get("errors")
        .and_then(Value::as_array)
        .ok_or("client UI section status unavailable")?;
    if errors.iter().any(|error| {
        error
            .as_str()
            .is_none_or(|text| text.starts_with("missions:"))
    }) {
        return Err("mission tracker section failed".into());
    }
    if value
        .get("mission_tracker")
        .and_then(Value::as_array)
        .is_none()
    {
        return Err("mission tracker rows unavailable".into());
    }
    Ok(Sample {
        at_ms: crate::supervisor::now_ms(),
        value,
    })
}

/// Emit bounded source and code fields without logging transport error text.
pub(super) fn log_failure<T>(
    result: &Result<T, String>,
    instance: &str,
    mission_id: i32,
    source: &'static str,
    error_code: &'static str,
) {
    if let Err(reason) = result {
        let error_code = match reason.as_str() {
            "server lab endpoint unavailable" => "endpoint_unavailable",
            "trusted client-to-server session binding unavailable" => "binding_unavailable",
            "mission tracker section failed" => "ui_section_failed",
            _ => error_code,
        };
        tracing::warn!(target: "lab.workbench.mission", instance, mission_id, source, error_code, "mission evidence read failed");
    }
}

/// Read one selected mission. The live route supplies no binding until DW-03S-B.
pub async fn read_mission(
    supervisor: &Supervisor,
    instance: &str,
    mission_id: i32,
    context: Option<&CaptureContext>,
) -> Value {
    let server = ServerTools::from_env();
    let unavailable = || "server lab endpoint unavailable".to_owned();
    let definition = match server.as_ref() {
        Some(server) => db_rows(server, format!("SELECT mission_id, mission_label, mission_defn, is_enabled, is_hidden, script_name FROM resources.missions WHERE mission_id = {mission_id}")).await
            .and_then(|sample| {
                let [one] = sample.value.as_array().and_then(|v| <&[Value; 1]>::try_from(v.as_slice()).ok()).ok_or("mission definition row absent or ambiguous")?;
                Ok(Sample { at_ms: sample.at_ms, value: one.clone() })
            }),
        None => Err(unavailable()),
    };
    log_failure(
        &definition,
        instance,
        mission_id,
        "resources.missions",
        "db_read_failed",
    );
    let steps = match server.as_ref() {
        Some(server) => db_rows(server, format!("SELECT step_id, mission_id, step_display_log_text, index, step_enabled FROM resources.mission_steps WHERE mission_id = {mission_id} ORDER BY index, step_id")).await,
        None => Err(unavailable()),
    };
    log_failure(
        &steps,
        instance,
        mission_id,
        "resources.mission_steps",
        "db_read_failed",
    );
    let chains = match server.as_ref() {
        Some(server) => db_rows(server, format!("SELECT chain_id, description, scope_type, scope_id, enabled FROM resources.content_chains WHERE scope_type = 'mission' AND scope_id = {mission_id} ORDER BY chain_id")).await,
        None => Err(unavailable()),
    };
    log_failure(
        &chains,
        instance,
        mission_id,
        "resources.content_chains",
        "db_read_failed",
    );
    let pid = supervisor.client_pid().await;
    let player = match (context, server.as_ref()) {
        (None, _) => Err("trusted client-to-server session binding unavailable".into()),
        (Some(context), Some(server)) => bound_player(server, Some(context), instance, pid).await,
        (Some(_), None) => Err(unavailable()),
    };
    log_failure(
        &player,
        instance,
        mission_id,
        "server_sessions",
        "roster_unbound",
    );
    let live = match (server.as_ref(), player) {
        (Some(server), Ok(player_id)) => db_rows(server, format!("SELECT mission_id, player_id, current_step_id, status, completed_step_ids, completed_objective_ids, active_objective_ids, failed_objective_ids, repeats FROM sgw_mission WHERE player_id = {player_id} AND mission_id = {mission_id}")).await
            .and_then(|sample| {
                let [one] = sample.value.as_array().and_then(|v| <&[Value; 1]>::try_from(v.as_slice()).ok()).ok_or("persisted mission row absent or ambiguous")?;
                Ok(Sample { at_ms: sample.at_ms, value: one.clone() })
            }),
        (_, Err(reason)) => Err(reason),
        _ => Err(unavailable()),
    };
    log_failure(
        &live,
        instance,
        mission_id,
        "sgw_mission",
        "server_read_failed",
    );
    let ui = supervisor.ui_state(Some(0)).await.and_then(ui_sample);
    log_failure(
        &ui,
        instance,
        mission_id,
        "client_ui_state.mission_tracker",
        "bridge_or_section_read_failed",
    );
    build_mission(MissionInput {
        instance: instance.into(),
        mission_id,
        definition,
        steps,
        chains,
        server: live,
        wire: Err("no session-bound decoded mission packet reader".into()),
        client: Err("no selected mission-ID client state reader".into()),
        ui,
        now_ms: crate::supervisor::now_ms(),
    })
}
