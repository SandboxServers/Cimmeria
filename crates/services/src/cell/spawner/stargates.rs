//! Stargate destination cache.
//!
//! Maps `stargate_id → StargateEntry` for gate-travel target resolution.

use sqlx::PgPool;

/// Cached stargate destination from `resources.stargates` + `resources.worlds`.
#[derive(Debug, Clone)]
pub struct StargateEntry {
    pub world_name: String,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub yaw: f32,
}

/// Load stargate destinations from the database.
///
/// Maps `stargate_id → StargateEntry` for gate travel lookups.
pub async fn load_stargates(
    pool: &PgPool,
) -> Result<std::collections::HashMap<i32, StargateEntry>, sqlx::Error> {
    use sqlx::Row;

    let rows = sqlx::query(
        "SELECT s.stargate_id, w.world AS world_name, \
                s.x_pos, s.y_pos, s.z_pos, s.yaw \
         FROM resources.stargates s \
         JOIN resources.worlds w ON s.world_id = w.world_id",
    )
    .fetch_all(pool)
    .await?;

    let mut map = std::collections::HashMap::with_capacity(rows.len());
    for r in &rows {
        let id: i32 = r.get("stargate_id");
        map.insert(
            id,
            StargateEntry {
                world_name: r.get("world_name"),
                x: r.get::<f64, _>("x_pos") as f32,
                y: r.get::<f64, _>("y_pos") as f32,
                z: r.get::<f64, _>("z_pos") as f32,
                yaw: r.get::<f64, _>("yaw") as f32,
            },
        );
    }

    tracing::info!(count = map.len(), "Loaded stargates cache");
    Ok(map)
}
