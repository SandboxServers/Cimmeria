//! Ring-transport region definitions loaded from the database.
//!
//! Mirrors `python/common/defs/RingTransporterRegion.py`. One row per ring pad
//! across all worlds — the table is global because rings can teleport across
//! worlds, so the destination metadata must be resolvable from any cell.

use std::collections::HashMap;

use sqlx::PgPool;

/// One ring transporter region — a teleporter pad keyed by `region_id`
/// (cross-world unique).
#[derive(Debug, Clone)]
pub struct RingRegion {
    pub region_id: i32,
    pub world_id: i32,
    pub world_name: String,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub tag: String,
    pub height: f32,
    pub radius: f32,
    pub event_set_id: i32,
    pub display_name_id: i32,
    /// Region IDs this pad can teleport to. Already filtered:
    /// self-references and unknown IDs are removed (matches Python `postLoad`).
    pub destination_ids: Vec<i32>,
    /// `point_sets.set_id` for the trigger volume around this pad.
    pub point_set_id: i32,
}

/// Load every `ring_transport_regions` row from the DB and return them keyed
/// by `region_id`.
///
/// The Python `postLoad` step (drop self-references and dangling destination
/// IDs) is performed in a second pass once the full table is in memory.
pub async fn load_ring_regions(
    pool: &PgPool,
) -> Result<HashMap<i32, RingRegion>, sqlx::Error> {
    use sqlx::Row;

    let rows = sqlx::query(
        "SELECT r.region_id, r.world_id, r.x, r.y, r.z, r.tag, \
                r.height, r.radius, r.event_set_id, r.display_name_id, \
                r.destination_region_ids, r.point_set_id, w.world AS world_name \
           FROM resources.ring_transport_regions r \
           JOIN resources.worlds w ON r.world_id = w.world_id \
          ORDER BY r.region_id"
    )
    .fetch_all(pool)
    .await?;

    let mut regions: HashMap<i32, RingRegion> = HashMap::with_capacity(rows.len());
    for r in &rows {
        let region_id: i32 = r.get("region_id");
        let dests: Vec<i32> = r.try_get("destination_region_ids").unwrap_or_default();
        regions.insert(region_id, RingRegion {
            region_id,
            world_id: r.get("world_id"),
            world_name: r.get("world_name"),
            x: r.get("x"),
            y: r.get("y"),
            z: r.get("z"),
            tag: r.get("tag"),
            height: r.try_get::<f32, _>("height").unwrap_or(0.0),
            radius: r.try_get::<f32, _>("radius").unwrap_or(0.0),
            event_set_id: r.get("event_set_id"),
            display_name_id: r.get("display_name_id"),
            destination_ids: dests,
            point_set_id: r.get("point_set_id"),
        });
    }

    // postLoad: filter self-references and dangling destinations.
    let known: std::collections::HashSet<i32> = regions.keys().copied().collect();
    for (id, region) in regions.iter_mut() {
        let mut filtered = Vec::with_capacity(region.destination_ids.len());
        for &dst in &region.destination_ids {
            if dst == *id {
                tracing::warn!(region_id = id, "ring region lists itself as destination — dropping");
                continue;
            }
            if !known.contains(&dst) {
                tracing::warn!(region_id = id, dst, "ring region destination not in table — dropping");
                continue;
            }
            filtered.push(dst);
        }
        region.destination_ids = filtered;
    }

    tracing::info!(count = regions.len(), "Loaded ring transport regions");
    Ok(regions)
}
