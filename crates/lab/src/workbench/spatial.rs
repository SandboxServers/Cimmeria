//! World-scoped spatial observations from existing client and server lab reads.

use serde_json::{json, Value};

use crate::{
    supervisor::{
        world::{
            find::{self, FindRequest},
            io::{LiveWorld, WorldIo},
        },
        Supervisor,
    },
    uat::invoke::ServerTools,
};

const FRAME_STALE_MS: i64 = 2_000;

/// A frame expires from its first actor read, not when projection finishes.
pub fn fresh_frame(at_ms: i64, now_ms: i64) -> bool {
    (0..=FRAME_STALE_MS).contains(&now_ms.saturating_sub(at_ms))
}

/// A client entity row with the world and instance of the sampling frame.
#[derive(Clone, Debug)]
pub struct SpatialEntity {
    pub instance: String,
    pub world_id: i64,
    pub space_id: Option<u64>,
    pub entity_id: u32,
    pub at_ms: i64,
    pub value: Value,
}

/// Join only an exact instance/world/space/entity identity and a fresh frame.
pub fn join_entity(client: &SpatialEntity, server: &SpatialEntity, now_ms: i64) -> Option<Value> {
    (client.instance == server.instance
        && client.world_id == server.world_id
        && client.space_id.is_some()
        && client.space_id == server.space_id
        && client.entity_id == server.entity_id
        && fresh_frame(client.at_ms, now_ms)
        && fresh_frame(server.at_ms, now_ms))
    .then(|| json!({"client":client.value,"server":server.value}))
}

pub(crate) fn definition_query(world_id: i64, table: &str) -> Result<String, String> {
    match table {
        "regions" => Ok(format!("SELECT region_id, tag, handler, flags, x1, y1, z1, x2, y2, z2, x3, y3, z3, x4, y4, z4, radius, height, world_id FROM resources.generic_regions WHERE world_id = {world_id} ORDER BY region_id LIMIT 101")),
        "spawns" => Ok(format!("SELECT spawn_id, tag, template_id, x, y, z, heading, world_id FROM resources.spawnlist WHERE world_id = {world_id} ORDER BY spawn_id LIMIT 101")),
        _ => Err("unknown spatial definition type".into()),
    }
}

pub(crate) fn definition_rows(result: &Value, world_id: i64, table: &str) -> Result<Value, String> {
    if result["truncated"] == true {
        return Err("definition query truncated".into());
    }
    let rows = result["rows"].as_array().ok_or("definition rows missing")?;
    if rows.len() > 100 {
        return Err("definition query exceeds 100 rows".into());
    }
    if rows
        .iter()
        .any(|row| row["world_id"].as_i64() != Some(world_id))
    {
        return Err("definition world mismatch".into());
    }
    let mut out = json!({"source":"server_db_query","world_id":world_id,"position_unit":"BigWorld m","rows":rows});
    match table {
        "regions" => {
            out["radius_unit"] = json!("m");
            out["height_unit"] = json!("m");
        }
        "spawns" => out["heading_unit"] = json!("rad"),
        _ => return Err("unknown spatial definition type".into()),
    }
    Ok(out)
}

async fn definitions(
    server: Option<&ServerTools>,
    world_id: i64,
    table: &str,
) -> Result<Value, String> {
    let server = server.ok_or("server lab endpoint unavailable")?;
    let sql = definition_query(world_id, table)?;
    let result = server.call("server_db_query", json!({"sql":sql})).await;
    if !result.ok {
        return Err(result
            .error
            .unwrap_or_else(|| "server_db_query failed".into()));
    }
    definition_rows(&result.json, world_id, table)
}

/// A read may be coherent without proving it belongs to one login generation.
pub fn observed_frame_consistent(
    before_pid: Option<u32>,
    after_pid: Option<u32>,
    found_player_id: Option<u32>,
    after_player_id: Option<u32>,
) -> bool {
    before_pid.is_some()
        && before_pid == after_pid
        && found_player_id.is_some_and(|id| id != 0 && Some(id) == after_player_id)
}

/// Current requires a trusted login generation in addition to equal sampled IDs.
pub fn current_frame(observed_consistent: bool, trusted_generation: bool) -> bool {
    observed_consistent && trusted_generation
}

/// Read a local spatial frame. Server runtime entity rows remain unknown until DW-03S binding.
pub async fn read_spatial(supervisor: &Supervisor, instance: &str) -> Value {
    let started = crate::supervisor::now_ms();
    let mut gaps = Vec::new();
    let before_pid = supervisor.client_pid().await;
    let before = supervisor.player_state().await;
    let world_id = before.as_ref().ok().and_then(|p| p.world_id);
    let server = ServerTools::from_env();
    let regions = if let Some(id) = world_id {
        definitions(server.as_ref(), id, "regions").await
    } else {
        Err("client world id unavailable".into())
    };
    let spawns = if let Some(id) = world_id {
        definitions(server.as_ref(), id, "spawns").await
    } else {
        Err("client world id unavailable".into())
    };
    let mut world = LiveWorld::new(supervisor);
    let frame_at_ms = crate::supervisor::now_ms();
    let found = find::run(
        &mut world,
        FindRequest {
            limit: Some(32),
            include_player: true,
            project: Some(true),
            ..FindRequest::default()
        },
    )
    .await
    .map_err(|e| e.summary());
    let camera = world.camera_readout().await.ok();
    let after_snapshot = world.snapshot().await;
    let after = supervisor.player_state().await;
    let after_pid = supervisor.client_pid().await;
    let now = crate::supervisor::now_ms();
    let world_stable = matches!((&before, &after), (Ok(a), Ok(b)) if a.world_id.is_some() && a.world_id == b.world_id && a.world == b.world);
    let found_player_id = found
        .as_ref()
        .ok()
        .and_then(|rows| rows["player"]["id"].as_u64())
        .and_then(|id| u32::try_from(id).ok());
    let after_player_id = after_snapshot
        .as_ref()
        .ok()
        .map(|snapshot| snapshot.player_id);
    let same_sampled_identity =
        observed_frame_consistent(before_pid, after_pid, found_player_id, after_player_id);
    let frame_stale = !fresh_frame(frame_at_ms, now);
    if !world_stable {
        gaps.push(json!({"source":"client_player_state","code":"world_changed_or_unknown"}));
    }
    if !same_sampled_identity {
        gaps.push(
            json!({"source":"client_process_and_player","code":"identity_changed_or_unknown"}),
        );
    }
    // Same PID and numeric player id can recur after a same-process relog.
    // DW-03S has not yet supplied a trusted login generation for this reader.
    gaps.push(json!({"source":"client_session","code":"client_identity_unbound"}));
    if frame_stale {
        gaps.push(json!({"source":"client_entity_find","code":"stale_frame"}));
    }
    if found.is_err() {
        gaps.push(json!({"source":"client_entity_find","code":"read_failed"}));
    }
    if found
        .as_ref()
        .ok()
        .and_then(|rows| rows["projection_error"].as_str())
        .is_some()
    {
        gaps.push(json!({"source":"client_entity_find.projection","code":"unavailable"}));
    }
    if found.as_ref().ok().is_some_and(|rows| {
        rows["entity_map_truncated"] == true
            || rows["unnamed_beyond_scan"].as_u64().is_some_and(|n| n > 0)
    }) {
        gaps.push(json!({"source":"client_entity_find","code":"partial_entity_frame"}));
    }
    if regions.is_err() {
        gaps.push(json!({"source":"server_db_query.regions","code":"unavailable"}));
    }
    if spawns.is_err() {
        gaps.push(json!({"source":"server_db_query.spawns","code":"unavailable"}));
    }
    gaps.push(json!({"source":"server_entity_query","code":"server_session_unbound","detail":"runtime entity rows cannot be joined to this client yet"}));
    if camera.is_none() {
        gaps.push(json!({"source":"client_camera","code":"unavailable"}));
    }
    for gap in &gaps {
        let source = gap["source"].as_str().unwrap_or("unknown");
        let code = gap["code"].as_str().unwrap_or("unknown");
        if !matches!(code, "client_identity_unbound" | "server_session_unbound") {
            tracing::warn!(target: "lab.workbench.spatial", instance, source, code, "spatial source incomplete");
        }
    }
    let observed_consistent =
        world_stable && same_sampled_identity && !frame_stale && found.is_ok();
    let current = current_frame(observed_consistent, false);
    json!({
        "schema_version":1,
        "context":{"instance":instance,"world_id":world_id,"space_id":null,"at_ms":now,"current":current,"observed_consistent":observed_consistent,"identity_verified":false,"sample_window_ms":now.saturating_sub(started)},
        "data":{
            "player":before.ok().filter(|_| observed_consistent).map(|p| p.to_json()),
            "camera":camera.filter(|_| observed_consistent),
            "client_entities":found.ok().filter(|_| observed_consistent).map(|v| json!({"source":"client_entity_find","at_ms":frame_at_ms,"position_unit":"UE3 uu (100 uu/m)","server_position_unit":"BigWorld m","screen_unit":"UI px","identity_verified":false,"value":v})),
            "server_entities":null,
            "regions":regions.ok().filter(|_| world_stable),
            "spawns":spawns.ok().filter(|_| world_stable),
        },
        "gaps":gaps,
    })
}
