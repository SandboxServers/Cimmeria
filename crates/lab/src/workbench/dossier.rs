//! Read-only target dossier with explicit source, age and missing layers.

use serde_json::{json, Value};

use crate::supervisor::world::geometry::{client_to_server, Vec3};
use crate::supervisor::world::io::{LiveWorld, WorldIo};
use crate::supervisor::world::memory::slot_entity;
use crate::supervisor::Supervisor;
use crate::uat::invoke::{ServerTools, ToolOutcome};

const RECENT_LIMIT: usize = 20;

/// One source's value, sampled at a known wall-clock time.
#[derive(Debug, Clone)]
pub struct Sample {
    pub at_ms: i64,
    pub value: Value,
}

/// Client and server observations for a single requested runtime entity id.
#[derive(Debug, Clone)]
pub struct DossierInput {
    pub instance: String,
    pub entity_id: u32,
    pub selected_before: Result<Sample, String>,
    pub selected_after: Result<Sample, String>,
    pub client: Result<Sample, String>,
    pub server: Result<Sample, String>,
    pub witnesses: Result<Sample, String>,
    pub messages: Result<Sample, String>,
    pub now_ms: i64,
}

fn sourced(source: &str, sample: Option<&Sample>, value: Option<Value>, now_ms: i64) -> Value {
    json!({
        "source": source,
        "at_ms": sample.map(|s| s.at_ms),
        "age_ms": sample.map(|s| now_ms.saturating_sub(s.at_ms).max(0)),
        "value": value,
    })
}

fn gap(gaps: &mut Vec<Value>, source: &str, code: &str, detail: impl Into<String>) {
    gaps.push(json!({ "source": source, "code": code, "detail": detail.into() }));
}

fn result<'a>(
    input: &'a Result<Sample, String>,
    source: &str,
    gaps: &mut Vec<Value>,
) -> Option<&'a Sample> {
    match input {
        Ok(sample) => Some(sample),
        Err(reason) => {
            gap(gaps, source, "unavailable", reason);
            None
        }
    }
}

fn path(sample: Option<&Sample>, key: &str) -> Option<Value> {
    sample
        .and_then(|s| s.value.get(key))
        .filter(|v| !v.is_null())
        .cloned()
}

fn position_delta(client: Option<&Sample>, server: Option<&Sample>) -> Option<Value> {
    let c = client?.value.get("position")?;
    let s = server?.value.get("position")?.as_array()?;
    let client = Vec3::new(
        c.get("x")?.as_f64()?,
        c.get("y")?.as_f64()?,
        c.get("z")?.as_f64()?,
    );
    let server = Vec3::new(
        s.first()?.as_f64()?,
        s.get(1)?.as_f64()?,
        s.get(2)?.as_f64()?,
    );
    let c_metres = client_to_server(client);
    let delta = c_metres.sub(server);
    (client.is_finite() && server.is_finite()).then(|| {
        json!({
            "x": delta.x, "y": delta.y, "z": delta.z,
            "distance_m": (delta.x * delta.x + delta.y * delta.y + delta.z * delta.z).sqrt(),
            "unit": "m",
        })
    })
}

/// Compose one dossier; a target change invalidates all entity data.
pub fn build_dossier(input: DossierInput) -> Value {
    let mut gaps = Vec::new();
    let before = result(&input.selected_before, "client.target_slot", &mut gaps);
    let after = result(&input.selected_after, "client.target_slot", &mut gaps);
    let before_id = before.and_then(|s| s.value.as_u64());
    let after_id = after.and_then(|s| s.value.as_u64());
    let current = before_id == Some(u64::from(input.entity_id)) && after_id == before_id;
    if !current {
        gap(
            &mut gaps,
            "client.target_slot",
            "target_changed",
            "selected target changed or does not match requested entity id",
        );
    }
    let client = current
        .then(|| result(&input.client, "client.entity", &mut gaps))
        .flatten();
    let server = current
        .then(|| result(&input.server, "server.entity", &mut gaps))
        .flatten();
    let witness = current
        .then(|| result(&input.witnesses, "server.witnesses", &mut gaps))
        .flatten();
    let messages = current
        .then(|| result(&input.messages, "server.log_tail", &mut gaps))
        .flatten();
    let id_matches = |s: &Sample| {
        s.value.get("entity_id").and_then(Value::as_u64) == Some(u64::from(input.entity_id))
    };
    let client = client.filter(|s| id_matches(s));
    let server = server.filter(|s| id_matches(s));
    let witness = witness.filter(|s| id_matches(s));
    if current && input.client.is_ok() && client.is_none() {
        gap(
            &mut gaps,
            "client.entity",
            "id_mismatch",
            "client row is not the selected entity",
        );
    }
    if current && input.server.is_ok() && server.is_none() {
        gap(
            &mut gaps,
            "server.entity",
            "id_mismatch",
            "server row is not the selected entity",
        );
    }
    if current && input.witnesses.is_ok() && witness.is_none() {
        gap(
            &mut gaps,
            "server.witnesses",
            "id_mismatch",
            "witness report is for another entity",
        );
    }
    let now = input.now_ms;
    json!({
        "schema_version": 1,
        "context": {
            "instance": input.instance,
            "entity_id": input.entity_id,
            "selected_entity_id": sourced("client.target_slot", after, after.map(|s| s.value.clone()), now),
            "current": current,
            "observed_at_ms": now,
        },
        "data": {
            "client": {
                "actor": sourced("client.entity_table", client, path(client, "actor"), now),
                "rendered": sourced("client.entity_table", client, path(client, "rendered"), now),
                "targetable": sourced("client.target_slot", client, path(client, "targetable"), now),
                "mob_id": sourced("client.unit_api", client, path(client, "mob_id"), now),
                "name": sourced("client.unit_api", client, path(client, "name"), now),
                "position_ue3": sourced("client.actor", client, path(client, "position"), now),
            },
            "server": {
                "template_id": sourced("server_entity_get", server, path(server, "template_id"), now),
                "health_cur": sourced("server_entity_get", server, path(server, "health_cur"), now),
                "health_max": sourced("server_entity_get", server, path(server, "health_max"), now),
                "state_field": sourced("server_entity_get", server, path(server, "state_field"), now),
                "spawn_id": sourced("server_entity_get", server, path(server, "spawn_id"), now),
                "tag": sourced("server_entity_get", server, path(server, "tag"), now),
                "position_m": sourced("server_entity_get", server, path(server, "position"), now),
                "witnessed_by": sourced("server_witnesses", witness, path(witness, "witnessed_by"), now),
                "witnesses": sourced("server_witnesses", witness, path(witness, "witnesses"), now),
            },
            "position_delta": sourced("client_actor+server_entity_get", client, position_delta(client, server), now),
            "recent_messages": sourced("server_log_tail", messages, messages.map(|s| s.value.clone()), now),
        },
        "gaps": gaps,
    })
}

fn sampled(value: Value) -> Sample {
    Sample {
        at_ms: crate::supervisor::now_ms(),
        value,
    }
}

async fn selection(world: &mut LiveWorld<'_>) -> Result<Sample, String> {
    let unit = world.unit_slots().await?;
    let slots = world.slots().await?;
    let id = slot_entity(&slots, unit.target);
    Ok(sampled(if id == 0 { Value::Null } else { json!(id) }))
}

async fn client_entity(
    supervisor: &Supervisor,
    id: u32,
    world: &mut LiveWorld<'_>,
) -> Result<Sample, String> {
    let snapshot = world.snapshot().await?;
    let entity = snapshot.entity(id).ok_or("entity absent from client map")?;
    let table = supervisor.entity_table(Some(vec![id]), false, None).await?;
    let row = table
        .pointer("/entities/details/0")
        .ok_or("entity absent from client table")?;
    if row.get("id").and_then(Value::as_u64) != Some(u64::from(id)) {
        return Err("client table id mismatch".into());
    }
    let unit = world.unit_slots().await?;
    let info = world
        .unit_info(&[unit.target])
        .await?
        .into_iter()
        .find(|i| i.slot == unit.target);
    Ok(sampled(json!({
        "entity_id": id,
        "actor": row.get("actor"),
        "rendered": entity.rendered(),
        "targetable": entity.rendered() && info.as_ref().is_some_and(|i| i.exists),
        "mob_id": info.as_ref().and_then(|i| i.mob_id),
        "name": info.as_ref().map(|i| i.name.as_str()).filter(|name| !name.is_empty()),
        "position": entity.pose.map(|p| p.pos.to_json()),
    })))
}

async fn server_call(server: Option<&ServerTools>, tool: &str, id: u32) -> Result<Sample, String> {
    let server = server.ok_or("server lab endpoint unavailable")?;
    let reply = server.call(tool, json!({ "entity_id": id })).await;
    if !reply.ok {
        return Err(reply.error.unwrap_or_else(|| format!("{tool} failed")));
    }
    let value = if tool == "server_entity_get" {
        reply
            .json
            .get("entity")
            .filter(|v| !v.is_null())
            .cloned()
            .ok_or("entity absent on server")?
    } else {
        reply.json
    };
    Ok(sampled(value))
}

fn matching_messages(outcome: ToolOutcome, id: u32) -> Result<Sample, String> {
    if !outcome.ok {
        return Err(outcome.error.unwrap_or("server_log_tail failed".into()));
    }
    let entries = outcome
        .json
        .get("entries")
        .and_then(Value::as_array)
        .ok_or("log entries missing")?;
    let rows: Vec<Value> = entries
        .iter()
        .filter(|entry| {
            ["entity_id", "target_entity_id", "witness_id"]
                .iter()
                .any(|key| {
                    entry
                        .get("fields")
                        .and_then(|fields| fields.get(*key))
                        .and_then(Value::as_u64)
                        == Some(u64::from(id))
                })
        })
        .rev()
        .take(RECENT_LIMIT)
        .cloned()
        .collect();
    Ok(sampled(json!(rows.into_iter().rev().collect::<Vec<_>>())))
}

/// Read a live dossier without a lease or any slot-pinning calls.
pub async fn read_dossier(supervisor: &Supervisor, instance: &str, entity_id: u32) -> Value {
    let mut world = LiveWorld::new(supervisor);
    let selected_before = selection(&mut world).await;
    let current =
        selected_before.as_ref().ok().and_then(|s| s.value.as_u64()) == Some(u64::from(entity_id));
    let server = ServerTools::from_env();
    let (client, entity, witnesses, messages) = if current {
        let client = client_entity(supervisor, entity_id, &mut world).await;
        let entity = server_call(server.as_ref(), "server_entity_get", entity_id).await;
        let witnesses = server_call(server.as_ref(), "server_witnesses", entity_id).await;
        let messages = match server.as_ref() {
            Some(s) => matching_messages(
                s.call("server_log_tail", json!({ "limit": 100 })).await,
                entity_id,
            ),
            None => Err("server lab endpoint unavailable".into()),
        };
        (client, entity, witnesses, messages)
    } else {
        let absent = || Err("requested entity is not the selected target".into());
        (absent(), absent(), absent(), absent())
    };
    let selected_after = selection(&mut LiveWorld::new(supervisor)).await;
    build_dossier(DossierInput {
        instance: instance.into(),
        entity_id,
        selected_before,
        selected_after,
        client,
        server: entity,
        witnesses,
        messages,
        now_ms: crate::supervisor::now_ms(),
    })
}

#[cfg(test)]
#[path = "dossier_tests.rs"]
mod tests;
