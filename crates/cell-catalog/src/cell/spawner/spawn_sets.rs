//! Switchable spawn sets, from `resources.spawn_sets` and
//! `resources.spawnlist.set_name` (Debug Area DA-10, the Visual NPC Lineup).
//!
//! A `spawnlist` row whose `set_name` matches a `spawn_sets.name` in the same
//! world belongs to that set. A member does not spawn at startup: the cell
//! keeps its record and spawns the whole set when a GM switches it on
//! (`activateSpawnSet`, the `.spawnset` console command, or the lineup
//! attendants' content action). Sets of one `type` in one world are shown one
//! at a time.

use sqlx::PgPool;

/// One `resources.spawn_sets` row and the spawn ids that name it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpawnSetDef {
    /// `spawn_sets.set_id`.
    pub set_id: i32,
    /// `spawn_sets.name`: the label a chat line shows, and what
    /// `spawnlist.set_name` holds.
    pub name: String,
    /// `spawn_sets.type`: sets of one kind in one world are exclusive.
    pub kind: String,
    /// `spawn_sets.world_id`.
    pub world_id: i32,
    /// Every `spawnlist.spawn_id` in the set, in id order.
    pub spawn_ids: Vec<i32>,
}

/// Load every spawn set with its members. A set with no member is kept (a
/// switch on it shows nothing and says so).
pub async fn load_spawn_sets(pool: &PgPool) -> Result<Vec<SpawnSetDef>, sqlx::Error> {
    let rows: Vec<(i32, String, String, i32, Vec<i32>)> = sqlx::query_as(
        "SELECT ss.set_id, ss.name, ss.type, ss.world_id, \
                COALESCE(array_agg(s.spawn_id ORDER BY s.spawn_id) \
                         FILTER (WHERE s.spawn_id IS NOT NULL), '{}') \
         FROM resources.spawn_sets ss \
         LEFT JOIN resources.spawnlist s \
           ON s.set_name = ss.name AND s.world_id = ss.world_id \
         GROUP BY ss.set_id, ss.name, ss.type, ss.world_id \
         ORDER BY ss.set_id",
    )
    .fetch_all(pool)
    .await?;
    let sets: Vec<SpawnSetDef> = rows
        .into_iter()
        .map(|(set_id, name, kind, world_id, spawn_ids)| SpawnSetDef {
            set_id,
            name,
            kind,
            world_id,
            spawn_ids,
        })
        .collect();
    tracing::info!(
        target: "spawner",
        event = "spawn_sets_loaded",
        sets = sets.len(),
        members = sets.iter().map(|s| s.spawn_ids.len()).sum::<usize>(),
        "Loaded switchable spawn sets (members stay unspawned until a GM switches a set on)"
    );
    Ok(sets)
}
