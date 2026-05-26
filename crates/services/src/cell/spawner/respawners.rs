//! Respawner definitions.
//!
//! Defeat-window respawn locations loaded from `resources.respawners`.
//! The client picks one and sends back the chosen `respawner_id` via
//! `callForAid`.

use sqlx::PgPool;

/// A respawner location loaded from the database.
///
/// Players see a list of these in the Defeat Window (onBeginAidWait).
/// The client sends back the chosen `respawner_id` via `callForAid`.
#[derive(Debug, Clone)]
pub struct RespawnerDef {
    pub respawner_id: i32,
    pub world_name: String,
    pub name: String,
    pub pos: [f32; 3],
}

/// Mirror of the SQL projection for `query_as`. The `pos` array on
/// `RespawnerDef` doesn't have a direct column equivalent, so we map the
/// three position columns onto separate fields here and assemble the array
/// in the conversion below. Using `FromRow` rather than manual `Row::get`
/// turns column-name and type mismatches into compile-/load-time errors
/// with proper diagnostics instead of mid-query runtime panics.
#[derive(sqlx::FromRow)]
struct RespawnerRow {
    respawner_id: i32,
    world_name: String,
    name: String,
    pos_x: f32,
    pos_y: f32,
    pos_z: f32,
}

impl From<RespawnerRow> for RespawnerDef {
    fn from(r: RespawnerRow) -> Self {
        RespawnerDef {
            respawner_id: r.respawner_id,
            world_name: r.world_name,
            name: r.name,
            pos: [r.pos_x, r.pos_y, r.pos_z],
        }
    }
}

/// Load respawner definitions from the database.
///
/// Joins `resources.respawners` with `resources.worlds` to get world names.
pub async fn load_respawners(pool: &PgPool) -> Result<Vec<RespawnerDef>, sqlx::Error> {
    let rows = sqlx::query_as::<_, RespawnerRow>(
        "SELECT r.respawner_id, w.world AS world_name, r.name, \
                r.pos_x, r.pos_y, r.pos_z \
         FROM resources.respawners r \
         JOIN resources.worlds w ON w.world_id = r.world_id",
    )
    .fetch_all(pool)
    .await?;

    let respawners: Vec<RespawnerDef> = rows.into_iter().map(RespawnerDef::from).collect();

    tracing::info!(count = respawners.len(), "Loaded respawner definitions");
    Ok(respawners)
}
