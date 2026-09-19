//! World-name → `world_id` startup cache.
//!
//! `entities/spaces.xml` carries world *names* only; the numeric
//! `world_id` that `spawnlist`, `stargates`, `ring_transport_regions` and
//! content-engine `world` condition rows all reference lives exclusively
//! in `resources.worlds`. Every other cell-side loader JOINs that table to
//! recover the name and throws the id away, so this is the one place the
//! mapping is kept.
//!
//! Consumed by [`SpaceManager::stamp_world_ids`](crate::cell::space_manager::SpaceManager::stamp_world_ids),
//! which writes each id onto the matching `WorldDef`.

use std::collections::HashMap;

use sqlx::PgPool;

/// Load `world name → world_id` from `resources.worlds`.
///
/// The column is `world`, not `world_name` — every sibling loader aliases
/// it the same way (`spawner/stargates.rs`, `spawner/respawners.rs`).
pub async fn load_world_ids(pool: &PgPool) -> Result<HashMap<String, i32>, sqlx::Error> {
    let rows: Vec<(i32, String)> =
        sqlx::query_as("SELECT world_id, world FROM resources.worlds ORDER BY world_id")
            .fetch_all(pool)
            .await?;

    Ok(rows
        .into_iter()
        .map(|(world_id, world)| (world, world_id))
        .collect())
}
