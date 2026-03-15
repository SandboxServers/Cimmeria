//! Database commands — connect to PostgreSQL and load/modify entity placements.
//!
//! All entity placement tables live in the `resources` schema. Coordinates are
//! stored in BigWorld meters (server coordinate system) and converted to/from
//! UE3 centimeters (client viewport system) at the boundary:
//!
//!     UE3_X = BW_Z * 100
//!     UE3_Y = BW_X * 100
//!     UE3_Z = BW_Y * 100

use crate::state::AppState;
use serde::Serialize;
use sqlx::postgres::PgPoolOptions;
use sqlx::Row;

// ---------------------------------------------------------------------------
// Coordinate conversion
// ---------------------------------------------------------------------------

/// Convert BigWorld (meters, X-forward) to UE3 (centimeters, Z-up).
///
/// BigWorld and UE3 use different axis conventions. The mapping is:
///   BW_X -> UE3_Y,  BW_Y -> UE3_Z,  BW_Z -> UE3_X
/// with a cm/m scale factor of 100.
fn bw_to_ue3(bw_x: f64, bw_y: f64, bw_z: f64) -> (f64, f64, f64) {
    (bw_z * 100.0, bw_x * 100.0, bw_y * 100.0)
}

/// Convert UE3 (centimeters, Z-up) to BigWorld (meters, X-forward).
fn ue3_to_bw(ue3_x: f64, ue3_y: f64, ue3_z: f64) -> (f64, f64, f64) {
    (ue3_y / 100.0, ue3_z / 100.0, ue3_x / 100.0)
}

// ---------------------------------------------------------------------------
// IPC response types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct DbStatus {
    pub connected: bool,
    pub host: String,
    pub dbname: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct WorldInfo {
    pub world_id: i32,
    pub world_name: String,
    pub client_map: String,
}

/// A unified entity placement from the database.
/// All positions are in UE3 centimeters (converted from BigWorld on read).
#[derive(Debug, Clone, Serialize)]
pub struct DbEntity {
    pub id: i32,
    /// Which table this came from: "spawnlist", "spawn_points", "generic_regions", "stargates"
    pub source_table: String,
    /// Entity class from entity_templates, or "Region" / "Stargate" / "SpawnPoint"
    pub entity_type: String,
    /// Display name: template_name, handler, stargate name, or spawn_set_name
    pub name: String,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    /// Heading/orientation in degrees (converted from radians on read, back to radians on write)
    pub heading: f64,
    pub world_id: i32,
    // Spawn-specific
    pub template_id: Option<i32>,
    pub tag: Option<String>,
    // Region-specific
    pub radius: Option<f64>,
    pub height: Option<f64>,
    pub handler: Option<String>,
    // Stargate-specific — formatted as "1-2-3-4-5-6"
    pub gate_address: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EntityTemplate {
    pub template_id: i32,
    pub template_name: String,
    pub class: String,
    pub name: Option<String>,
    pub static_mesh: Option<String>,
    pub level: Option<i32>,
}

// ---------------------------------------------------------------------------
// Connection management
// ---------------------------------------------------------------------------

/// Connect to the PostgreSQL database.
#[tauri::command]
pub async fn connect_db(
    state: tauri::State<'_, AppState>,
    host: String,
    port: u16,
    dbname: String,
    username: String,
    password: String,
) -> Result<DbStatus, String> {
    tracing::info!("connect_db: {}@{}:{}/{}", username, host, port, dbname);

    // Drop any existing pool before connecting
    {
        let mut pool_guard = state.db_pool.lock().unwrap();
        *pool_guard = None;
    }

    let url = format!(
        "postgres://{}:{}@{}:{}/{}",
        username, password, host, port, dbname
    );

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&url)
        .await
        .map_err(|e| format!("Database connection failed: {e}"))?;

    // Verify the connection works and the resources schema exists
    sqlx::query("SELECT 1 FROM information_schema.schemata WHERE schema_name = 'resources'")
        .fetch_optional(&pool)
        .await
        .map_err(|e| format!("Database query failed: {e}"))?
        .ok_or_else(|| "Connected, but 'resources' schema not found".to_string())?;

    let status = DbStatus {
        connected: true,
        host: host.clone(),
        dbname: dbname.clone(),
    };

    *state.db_pool.lock().unwrap() = Some(pool);

    tracing::info!("Database connected: {}:{}/{}", host, port, dbname);
    Ok(status)
}

/// Disconnect from the database.
#[tauri::command]
pub async fn disconnect_db(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let pool = state.db_pool.lock().unwrap().take();
    if let Some(pool) = pool {
        pool.close().await;
        tracing::info!("Database disconnected");
    }
    // Clear cached entities
    state.db_entities.lock().unwrap().clear();
    Ok(())
}

/// Get the database connection status.
#[tauri::command]
pub async fn db_status(state: tauri::State<'_, AppState>) -> Result<DbStatus, String> {
    let guard = state.db_pool.lock().unwrap();
    match guard.as_ref() {
        Some(pool) if !pool.is_closed() => {
            // We don't store host/dbname separately, but we can tell the frontend
            // we're connected. The actual host/dbname were shown on connect.
            Ok(DbStatus {
                connected: true,
                host: String::new(),
                dbname: String::new(),
            })
        }
        _ => Ok(DbStatus {
            connected: false,
            host: String::new(),
            dbname: String::new(),
        }),
    }
}

// ---------------------------------------------------------------------------
// Read queries
// ---------------------------------------------------------------------------

/// Helper: clone the pool out of the mutex, or return an error.
fn get_pool(state: &AppState) -> Result<sqlx::PgPool, String> {
    state
        .db_pool
        .lock()
        .unwrap()
        .clone()
        .ok_or_else(|| "Not connected to database".to_string())
}

/// List all worlds, returning world_id and client_map for zone matching.
#[tauri::command]
pub async fn list_worlds(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<WorldInfo>, String> {
    let pool = get_pool(&state)?;

    let rows = sqlx::query(
        "SELECT world_id, world, client_map FROM resources.worlds ORDER BY world_id",
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| format!("Failed to query worlds: {e}"))?;

    let worlds = rows
        .iter()
        .map(|row| WorldInfo {
            world_id: row.get("world_id"),
            world_name: row.get("world"),
            client_map: row.get("client_map"),
        })
        .collect();

    Ok(worlds)
}

/// Load all entity placements for a given world_id.
///
/// Queries spawnlist, spawn_points, generic_regions, and stargates, converts
/// all coordinates from BigWorld meters to UE3 centimeters, and returns a
/// unified list of DbEntity.
#[tauri::command]
pub async fn load_db_entities(
    state: tauri::State<'_, AppState>,
    world_id: i32,
) -> Result<Vec<DbEntity>, String> {
    let pool = get_pool(&state)?;
    let mut entities = Vec::new();

    // -- Spawnlist (with template join for class/name) --
    let spawn_rows = sqlx::query(
        "SELECT s.spawn_id, s.x, s.y, s.z, s.heading, s.world_id, s.template_id,
                s.tag, t.template_name, t.class
         FROM resources.spawnlist s
         JOIN resources.entity_templates t ON s.template_id = t.template_id
         WHERE s.world_id = $1",
    )
    .bind(world_id)
    .fetch_all(&pool)
    .await
    .map_err(|e| format!("Failed to query spawnlist: {e}"))?;

    for row in &spawn_rows {
        let bw_x: f32 = row.get("x");
        let bw_y: f32 = row.get("y");
        let bw_z: f32 = row.get("z");
        let (ue3_x, ue3_y, ue3_z) = bw_to_ue3(bw_x as f64, bw_y as f64, bw_z as f64);
        let heading_rad: f32 = row.get("heading");

        entities.push(DbEntity {
            id: row.get("spawn_id"),
            source_table: "spawnlist".into(),
            entity_type: row.get("class"),
            name: row.get("template_name"),
            x: ue3_x,
            y: ue3_y,
            z: ue3_z,
            heading: (heading_rad as f64).to_degrees(),
            world_id,
            template_id: Some(row.get("template_id")),
            tag: row.get("tag"),
            radius: None,
            height: None,
            handler: None,
            gate_address: None,
        });
    }

    // -- Spawn points --
    let sp_rows = sqlx::query(
        "SELECT point_id, x, y, z, orientation, spawn_set_name
         FROM resources.spawn_points
         WHERE spawn_set_name IN (
             SELECT name FROM resources.spawn_sets WHERE world_id = $1
         )",
    )
    .bind(world_id)
    .fetch_all(&pool)
    .await
    .map_err(|e| format!("Failed to query spawn_points: {e}"))?;

    for row in &sp_rows {
        let bw_x: f32 = row.get("x");
        let bw_y: f32 = row.get("y");
        let bw_z: f32 = row.get("z");
        let (ue3_x, ue3_y, ue3_z) = bw_to_ue3(bw_x as f64, bw_y as f64, bw_z as f64);
        let orientation_rad: f32 = row.get("orientation");

        entities.push(DbEntity {
            id: row.get("point_id"),
            source_table: "spawn_points".into(),
            entity_type: "SpawnPoint".into(),
            name: row.get("spawn_set_name"),
            x: ue3_x,
            y: ue3_y,
            z: ue3_z,
            heading: (orientation_rad as f64).to_degrees(),
            world_id,
            template_id: None,
            tag: None,
            radius: None,
            height: None,
            handler: None,
            gate_address: None,
        });
    }

    // -- Generic regions --
    let region_rows = sqlx::query(
        "SELECT region_id, height, radius, flags, handler,
                x1, y1, z1, world_id, tag
         FROM resources.generic_regions
         WHERE world_id = $1",
    )
    .bind(world_id)
    .fetch_all(&pool)
    .await
    .map_err(|e| format!("Failed to query generic_regions: {e}"))?;

    for row in &region_rows {
        let bw_x: f32 = row.get("x1");
        let bw_y: f32 = row.get("y1");
        let bw_z: f32 = row.get("z1");
        let (ue3_x, ue3_y, ue3_z) = bw_to_ue3(bw_x as f64, bw_y as f64, bw_z as f64);
        let radius: f32 = row.get("radius");
        let height: f32 = row.get("height");

        entities.push(DbEntity {
            id: row.get("region_id"),
            source_table: "generic_regions".into(),
            entity_type: "Region".into(),
            name: row.get("handler"),
            x: ue3_x,
            y: ue3_y,
            z: ue3_z,
            heading: 0.0,
            world_id,
            template_id: None,
            tag: row.get("tag"),
            // Convert radius/height from BW meters to UE3 cm
            radius: Some(radius as f64 * 100.0),
            height: Some(height as f64 * 100.0),
            handler: Some(row.get::<String, _>("handler")),
            gate_address: None,
        });
    }

    // -- Stargates --
    let gate_rows = sqlx::query(
        "SELECT stargate_id, name, x_pos, y_pos, z_pos, yaw,
                address1, address2, address3, address4, address5, address6
         FROM resources.stargates
         WHERE world_id = $1",
    )
    .bind(world_id)
    .fetch_all(&pool)
    .await
    .map_err(|e| format!("Failed to query stargates: {e}"))?;

    for row in &gate_rows {
        let bw_x: f64 = row.get("x_pos");
        let bw_y: f64 = row.get("y_pos");
        let bw_z: f64 = row.get("z_pos");
        let (ue3_x, ue3_y, ue3_z) = bw_to_ue3(bw_x, bw_y, bw_z);
        let yaw: f64 = row.get("yaw");

        let addr: String = format!(
            "{}-{}-{}-{}-{}-{}",
            row.get::<i32, _>("address1"),
            row.get::<i32, _>("address2"),
            row.get::<i32, _>("address3"),
            row.get::<i32, _>("address4"),
            row.get::<i32, _>("address5"),
            row.get::<i32, _>("address6"),
        );

        entities.push(DbEntity {
            id: row.get("stargate_id"),
            source_table: "stargates".into(),
            entity_type: "Stargate".into(),
            name: row.get("name"),
            x: ue3_x,
            y: ue3_y,
            z: ue3_z,
            heading: yaw.to_degrees(),
            world_id,
            template_id: None,
            tag: None,
            radius: None,
            height: None,
            handler: None,
            gate_address: Some(addr),
        });
    }

    tracing::info!(
        "Loaded {} db entities for world_id {}",
        entities.len(),
        world_id
    );

    // Cache in state for potential future use
    *state.db_entities.lock().unwrap() = entities.clone();

    Ok(entities)
}

/// List entity templates for the entity palette, optionally filtered by class.
#[tauri::command]
pub async fn list_entity_templates(
    state: tauri::State<'_, AppState>,
    class_filter: Option<String>,
) -> Result<Vec<EntityTemplate>, String> {
    let pool = get_pool(&state)?;

    let rows = match &class_filter {
        Some(class) => {
            sqlx::query(
                "SELECT template_id, template_name, class, name, static_mesh, level
                 FROM resources.entity_templates
                 WHERE class = $1
                 ORDER BY template_name",
            )
            .bind(class)
            .fetch_all(&pool)
            .await
        }
        None => {
            sqlx::query(
                "SELECT template_id, template_name, class, name, static_mesh, level
                 FROM resources.entity_templates
                 ORDER BY class, template_name",
            )
            .fetch_all(&pool)
            .await
        }
    }
    .map_err(|e| format!("Failed to query entity_templates: {e}"))?;

    let templates = rows
        .iter()
        .map(|row| EntityTemplate {
            template_id: row.get("template_id"),
            template_name: row.get("template_name"),
            class: row.get("class"),
            name: row.get("name"),
            static_mesh: row.get("static_mesh"),
            level: row.get("level"),
        })
        .collect();

    Ok(templates)
}

// ---------------------------------------------------------------------------
// Write commands
// ---------------------------------------------------------------------------

/// Create a new spawn in the spawnlist table.
///
/// Position is in UE3 centimeters -- converted to BigWorld meters before INSERT.
/// Returns the newly created DbEntity (with UE3 coordinates) including the
/// generated spawn_id.
#[tauri::command]
pub async fn create_spawn(
    state: tauri::State<'_, AppState>,
    world_id: i32,
    template_id: i32,
    x: f64,
    y: f64,
    z: f64,
    heading: f64,
    tag: Option<String>,
) -> Result<DbEntity, String> {
    let pool = get_pool(&state)?;
    let (bw_x, bw_y, bw_z) = ue3_to_bw(x, y, z);

    let row = sqlx::query(
        "INSERT INTO resources.spawnlist (x, y, z, heading, world_id, template_id, tag)
         VALUES ($1, $2, $3, $4, $5, $6, $7)
         RETURNING spawn_id",
    )
    .bind(bw_x as f32)
    .bind(bw_y as f32)
    .bind(bw_z as f32)
    .bind(heading.to_radians() as f32)
    .bind(world_id)
    .bind(template_id)
    .bind(&tag)
    .fetch_one(&pool)
    .await
    .map_err(|e| format!("Failed to insert spawn: {e}"))?;

    let spawn_id: i32 = row.get("spawn_id");

    // Fetch the template info so we can return a complete DbEntity
    let tmpl_row = sqlx::query(
        "SELECT template_name, class FROM resources.entity_templates WHERE template_id = $1",
    )
    .bind(template_id)
    .fetch_one(&pool)
    .await
    .map_err(|e| format!("Failed to fetch template: {e}"))?;

    let entity = DbEntity {
        id: spawn_id,
        source_table: "spawnlist".into(),
        entity_type: tmpl_row.get("class"),
        name: tmpl_row.get("template_name"),
        x,
        y,
        z,
        heading,
        world_id,
        template_id: Some(template_id),
        tag,
        radius: None,
        height: None,
        handler: None,
        gate_address: None,
    };

    tracing::info!("Created spawn {} (template_id={})", spawn_id, template_id);
    Ok(entity)
}

/// Update a spawn's position. Coordinates are in UE3 centimeters, converted
/// to BigWorld meters before writing.
#[tauri::command]
pub async fn update_spawn_position(
    state: tauri::State<'_, AppState>,
    spawn_id: i32,
    x: f64,
    y: f64,
    z: f64,
    heading: f64,
) -> Result<(), String> {
    let pool = get_pool(&state)?;
    let (bw_x, bw_y, bw_z) = ue3_to_bw(x, y, z);

    let result = sqlx::query(
        "UPDATE resources.spawnlist
         SET x = $1, y = $2, z = $3, heading = $4
         WHERE spawn_id = $5",
    )
    .bind(bw_x as f32)
    .bind(bw_y as f32)
    .bind(bw_z as f32)
    .bind(heading.to_radians() as f32)
    .bind(spawn_id)
    .execute(&pool)
    .await
    .map_err(|e| format!("Failed to update spawn: {e}"))?;

    if result.rows_affected() == 0 {
        return Err(format!("Spawn {} not found", spawn_id));
    }

    tracing::debug!("Updated spawn {} position", spawn_id);
    Ok(())
}

/// Delete a spawn from the spawnlist table.
#[tauri::command]
pub async fn delete_spawn(
    state: tauri::State<'_, AppState>,
    spawn_id: i32,
) -> Result<(), String> {
    let pool = get_pool(&state)?;

    let result = sqlx::query("DELETE FROM resources.spawnlist WHERE spawn_id = $1")
        .bind(spawn_id)
        .execute(&pool)
        .await
        .map_err(|e| format!("Failed to delete spawn: {e}"))?;

    if result.rows_affected() == 0 {
        return Err(format!("Spawn {} not found", spawn_id));
    }

    tracing::info!("Deleted spawn {}", spawn_id);
    Ok(())
}

/// Create a new generic region in the database.
///
/// Position is in UE3 centimeters — converted to BigWorld meters before INSERT.
/// Radius and height are also in UE3 centimeters.
#[tauri::command]
pub async fn create_region(
    state: tauri::State<'_, AppState>,
    world_id: i32,
    x: f64,
    y: f64,
    z: f64,
    radius: f64,
    height: f64,
    handler: String,
    tag: Option<String>,
) -> Result<DbEntity, String> {
    let pool = get_pool(&state)?;
    let (bw_x, bw_y, bw_z) = ue3_to_bw(x, y, z);

    let row = sqlx::query(
        "INSERT INTO resources.generic_regions (x1, y1, z1, radius, height, flags, handler, world_id, tag)
         VALUES ($1, $2, $3, $4, $5, 0, $6, $7, $8)
         RETURNING region_id",
    )
    .bind(bw_x as f32)
    .bind(bw_y as f32)
    .bind(bw_z as f32)
    .bind((radius / 100.0) as f32)  // UE3 cm → BW meters
    .bind((height / 100.0) as f32)
    .bind(&handler)
    .bind(world_id)
    .bind(&tag)
    .fetch_one(&pool)
    .await
    .map_err(|e| format!("Failed to insert region: {e}"))?;

    let region_id: i32 = row.get("region_id");

    let entity = DbEntity {
        id: region_id,
        source_table: "generic_regions".into(),
        entity_type: "Region".into(),
        name: handler.clone(),
        x,
        y,
        z,
        heading: 0.0,
        world_id,
        template_id: None,
        tag,
        radius: Some(radius),
        height: Some(height),
        handler: Some(handler),
        gate_address: None,
    };

    tracing::info!("Created region {} (handler={})", region_id, entity.handler.as_deref().unwrap_or(""));
    Ok(entity)
}

/// Delete a generic region from the database.
#[tauri::command]
pub async fn delete_region(
    state: tauri::State<'_, AppState>,
    region_id: i32,
) -> Result<(), String> {
    let pool = get_pool(&state)?;

    let result = sqlx::query("DELETE FROM resources.generic_regions WHERE region_id = $1")
        .bind(region_id)
        .execute(&pool)
        .await
        .map_err(|e| format!("Failed to delete region: {e}"))?;

    if result.rows_affected() == 0 {
        return Err(format!("Region {} not found", region_id));
    }

    tracing::info!("Deleted region {}", region_id);
    Ok(())
}

/// Delete a DB entity by its source table and ID.
///
/// Dispatches to the correct table-specific delete based on `source_table`.
#[tauri::command]
pub async fn delete_db_entity(
    state: tauri::State<'_, AppState>,
    entity_id: i32,
    source_table: Option<String>,
) -> Result<(), String> {
    let pool = get_pool(&state)?;
    let table = source_table.as_deref().unwrap_or("spawnlist");

    let query = match table {
        "spawnlist" => "DELETE FROM resources.spawnlist WHERE spawn_id = $1",
        "generic_regions" => "DELETE FROM resources.generic_regions WHERE region_id = $1",
        "spawn_points" => "DELETE FROM resources.spawn_points WHERE point_id = $1",
        "stargates" => "DELETE FROM resources.stargates WHERE stargate_id = $1",
        _ => return Err(format!("Unknown source table: {table}")),
    };

    let result = sqlx::query(query)
        .bind(entity_id)
        .execute(&pool)
        .await
        .map_err(|e| format!("Failed to delete {table} entity {entity_id}: {e}"))?;

    if result.rows_affected() == 0 {
        return Err(format!("{table} entity {} not found", entity_id));
    }

    tracing::info!("Deleted {table} entity {entity_id}");
    Ok(())
}
