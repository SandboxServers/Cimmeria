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

/// Load respawner definitions from the database.
///
/// Joins `resources.respawners` with `resources.worlds` to get world names.
pub async fn load_respawners(
    pool: &PgPool,
) -> Result<Vec<RespawnerDef>, sqlx::Error> {
    use sqlx::Row;

    let rows = sqlx::query(
        "SELECT r.respawner_id, w.world AS world_name, r.name, \
                r.pos_x, r.pos_y, r.pos_z \
         FROM resources.respawners r \
         JOIN resources.worlds w ON w.world_id = r.world_id"
    )
    .fetch_all(pool)
    .await?;

    let respawners: Vec<RespawnerDef> = rows.iter().map(|r| {
        RespawnerDef {
            respawner_id: r.get("respawner_id"),
            world_name: r.get("world_name"),
            name: r.get("name"),
            pos: [
                r.get::<f32, _>("pos_x"),
                r.get::<f32, _>("pos_y"),
                r.get::<f32, _>("pos_z"),
            ],
        }
    }).collect();

    tracing::info!(count = respawners.len(), "Loaded respawner definitions");
    Ok(respawners)
}
