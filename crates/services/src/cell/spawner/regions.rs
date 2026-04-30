//! Generic region loading.
//!
//! Loads `resources.point_sets` (type='AreaSet') + their points and applies
//! the Python `GenericRegion.workaround()` to expand single-point cylinder
//! regions into a 4-point bounding box for client hit-testing.
//!
//! Reference: `python/cell/GenericRegion.py`

use sqlx::PgPool;

/// Intermediate structure for loading region data before runtime ID assignment.
#[derive(Debug, Clone)]
pub struct RegionLoadData {
    pub set_id: i32,
    pub name: String,
    pub world_name: String,
    pub radius: f32,
    pub height: f32,
    pub flags: i32,
    pub points: Vec<[f32; 3]>,
}

/// Load generic regions from the database.
///
/// Queries `resources.point_sets` (type='AreaSet') joined with `resources.worlds`
/// for region metadata, then `resources.point_set_points` for polygon vertices.
///
/// Also applies the Python `GenericRegion.workaround()` — single-point cylinder
/// regions (radius > 0, 1 point) are expanded to a 4-point bounding box so the
/// client can hit-test them.
///
/// Reference: `python/cell/GenericRegion.py:GenericRegionManager.load()`
pub async fn load_regions_from_db(
    pool: &PgPool,
) -> Result<Vec<RegionLoadData>, sqlx::Error> {
    use sqlx::Row;

    let region_rows = sqlx::query(
        "SELECT ps.set_id, ps.name, ps.radius, ps.height, ps.flags, \
                w.world AS world_name \
         FROM resources.point_sets ps \
         JOIN resources.worlds w ON ps.world_id = w.world_id \
         WHERE ps.type = 'AreaSet' \
         ORDER BY ps.set_id"
    )
    .fetch_all(pool)
    .await?;

    if region_rows.is_empty() {
        return Ok(vec![]);
    }

    // Collect all set_ids to batch-fetch points
    let set_ids: Vec<i32> = region_rows.iter()
        .map(|r| r.get::<i32, _>("set_id"))
        .collect();

    let point_rows = sqlx::query(
        "SELECT set_id, x, y, z \
         FROM resources.point_set_points \
         WHERE set_id = ANY($1) \
         ORDER BY set_id, point_id"
    )
    .bind(&set_ids)
    .fetch_all(pool)
    .await?;

    // Group points by set_id
    let mut points_by_set: std::collections::HashMap<i32, Vec<[f32; 3]>> =
        std::collections::HashMap::new();
    for r in &point_rows {
        let set_id: i32 = r.get("set_id");
        let x: f32 = r.get("x");
        let y: f32 = r.get("y");
        let z: f32 = r.get("z");
        points_by_set.entry(set_id)
            .or_default()
            .push([x, y, z]);
    }

    let mut regions = Vec::with_capacity(region_rows.len());
    for r in &region_rows {
        let set_id: i32 = r.get("set_id");
        let radius: f32 = r.try_get::<f32, _>("radius").unwrap_or(0.0);
        let height: f32 = r.try_get::<f32, _>("height").unwrap_or(0.0);
        let mut points = points_by_set.remove(&set_id).unwrap_or_default();

        // Python workaround: single-point cylinder → 4-point bounding box
        // Reference: GenericRegion.workaround() — if 1 point and radius > 0,
        // expand to an axis-aligned box centered on the point.
        if points.len() == 1 && radius > 0.0 {
            let [px, py, pz] = points[0];
            let r = radius;
            let h = height;
            points = vec![
                [px - r, py,     pz - r],
                [px - r, py,     pz + r],
                [px + r, py,     pz + r],
                [px + r, py + h, pz - r],
            ];
        }

        regions.push(RegionLoadData {
            set_id,
            name: r.get("name"),
            world_name: r.get("world_name"),
            radius: radius as f32,
            height: height as f32,
            flags: r.get("flags"),
            points,
        });
    }

    tracing::info!(count = regions.len(), "Loaded generic regions from database");
    Ok(regions)
}
