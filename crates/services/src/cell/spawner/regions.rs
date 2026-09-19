//! Generic region loading.
//!
//! Loads `resources.point_sets` (type='AreaSet') + their points and applies
//! the Python `GenericRegion.workaround()` to expand single-point cylinder
//! regions into a 4-point bounding box for client hit-testing.
//!
//! Reference: `python/cell/GenericRegion.py`

use sqlx::PgPool;

/// Slop, in world units, applied to every axis of a region's AABB before
/// testing server-known containment.
///
/// Verbatim from `deprecated/python/common/Config.py:15`
/// (`GENERIC_REGION_CHECK_THRESHOLD = 1.5`). It exists because the trigger
/// arrives from the client at the moment *its* capsule crossed the volume,
/// while the test runs against the last position the server accepted — one
/// movement packet behind at worst. Widening it weakens the gate; narrowing
/// it starts false-rejecting real entries on a laggy link.
pub const GENERIC_REGION_CHECK_THRESHOLD: f32 = 1.5;

/// Is `point` inside the region described by `points`?
///
/// Port of `deprecated/python/cell/GenericRegion.py:isPointInRegion` — an
/// axis-aligned bounding box over the region's four corners, widened by
/// [`GENERIC_REGION_CHECK_THRESHOLD`] on every axis. The Python carries a
/// `# TODO: Do precise check, AABB isn't enough!`; this is the same
/// approximation, deliberately, because the AABB is also the shape the
/// client was handed by `addClientHintedGenericRegion` and a tighter server
/// test would reject entries the client legitimately reported.
///
/// **Exactly four points or nothing.** A region with any other count cannot
/// be hit-tested (the Python warns at load and returns `False` from every
/// containment call), so it fails closed here too. Every `AreaSet` in the
/// shipped seed reaches four: 43 of 62 are single-point cylinders expanded
/// by [`load_regions_from_db`]'s workaround, the other 19 are authored with
/// four corners.
///
/// Note the AABB is taken over all four corners *after* the workaround, so
/// it inherits the deliberate `py + h` asymmetry on the fourth corner — that
/// asymmetry is what gives an expanded cylinder its ceiling. Do not
/// "normalize" it here or in the loader.
pub fn is_point_in_region(points: &[[f32; 3]], point: [f32; 3]) -> bool {
    if points.len() != 4 {
        return false;
    }
    let t = GENERIC_REGION_CHECK_THRESHOLD;
    (0..3).all(|axis| {
        let min = points.iter().map(|p| p[axis]).fold(f32::INFINITY, f32::min);
        let max = points
            .iter()
            .map(|p| p[axis])
            .fold(f32::NEG_INFINITY, f32::max);
        point[axis] >= min - t && point[axis] <= max + t
    })
}

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
pub async fn load_regions_from_db(pool: &PgPool) -> Result<Vec<RegionLoadData>, sqlx::Error> {
    use sqlx::Row;

    let region_rows = sqlx::query(
        "SELECT ps.set_id, ps.name, ps.radius, ps.height, ps.flags, \
                w.world AS world_name \
         FROM resources.point_sets ps \
         JOIN resources.worlds w ON ps.world_id = w.world_id \
         WHERE ps.type = 'AreaSet' \
         ORDER BY ps.set_id",
    )
    .fetch_all(pool)
    .await?;

    if region_rows.is_empty() {
        return Ok(vec![]);
    }

    // Collect all set_ids to batch-fetch points
    let set_ids: Vec<i32> = region_rows
        .iter()
        .map(|r| r.get::<i32, _>("set_id"))
        .collect();

    let point_rows = sqlx::query(
        "SELECT set_id, x, y, z \
         FROM resources.point_set_points \
         WHERE set_id = ANY($1) \
         ORDER BY set_id, point_id",
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
        points_by_set.entry(set_id).or_default().push([x, y, z]);
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
        //
        // The asymmetric elevation on the fourth corner (only one corner uses
        // py + h while the other three use py) is intentional and matches the
        // Python original — do not "normalize" by raising all four corners.
        if points.len() == 1 && radius > 0.0 {
            let [px, py, pz] = points[0];
            let r = radius;
            let h = height;
            points = vec![
                [px - r, py, pz - r],
                [px - r, py, pz + r],
                [px + r, py, pz + r],
                [px + r, py + h, pz - r],
            ];
        }

        regions.push(RegionLoadData {
            set_id,
            name: r.get("name"),
            world_name: r.get("world_name"),
            radius,
            height,
            flags: r.get("flags"),
            points,
        });
    }

    tracing::info!(
        count = regions.len(),
        "Loaded generic regions from database"
    );
    Ok(regions)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The shape the loader's cylinder workaround produces: a 5-unit-radius
    /// cylinder at the origin with height 3, expanded to four corners with
    /// the deliberate `py + h` asymmetry on the fourth.
    fn expanded_cylinder() -> Vec<[f32; 3]> {
        vec![
            [-5.0, 0.0, -5.0],
            [-5.0, 0.0, 5.0],
            [5.0, 0.0, 5.0],
            [5.0, 3.0, -5.0],
        ]
    }

    #[test]
    fn a_point_at_the_centre_is_inside() {
        assert!(is_point_in_region(&expanded_cylinder(), [0.0, 1.0, 0.0]));
    }

    /// The exploit shape: a forged `triggerClientHintedGenericRegion` sent
    /// from the far side of the map. Well outside the slop on every axis.
    #[test]
    fn a_point_across_the_map_is_outside() {
        assert!(!is_point_in_region(
            &expanded_cylinder(),
            [400.0, 0.0, -180.0]
        ));
    }

    /// The threshold is load-bearing in both directions: 6.4 is outside the
    /// raw box (max X = 5) but inside the 1.5-unit slop; 6.6 is outside both.
    #[test]
    fn the_threshold_widens_the_box_by_exactly_its_value() {
        let pts = expanded_cylinder();
        assert!(is_point_in_region(&pts, [6.4, 0.0, 0.0]), "inside the slop");
        assert!(
            !is_point_in_region(&pts, [6.6, 0.0, 0.0]),
            "past the slop — a 0.2-unit widening must not let this through"
        );
    }

    /// Y is the vertical axis and the fourth corner is the only one carrying
    /// `py + h`, so the AABB's ceiling comes from that corner alone.
    #[test]
    fn the_ceiling_comes_from_the_asymmetric_fourth_corner() {
        let pts = expanded_cylinder();
        assert!(
            is_point_in_region(&pts, [0.0, 4.0, 0.0]),
            "4.0 is under the 3.0 ceiling plus 1.5 slop"
        );
        assert!(
            !is_point_in_region(&pts, [0.0, 6.0, 0.0]),
            "6.0 is above the ceiling — a flying client must not trigger"
        );
    }

    /// 2009 warns and refuses for any point count other than four. Fail
    /// closed rather than accepting an un-hit-testable volume.
    #[test]
    fn a_region_without_four_points_is_never_contained() {
        assert!(!is_point_in_region(&[], [0.0, 0.0, 0.0]));
        assert!(!is_point_in_region(&[[0.0, 0.0, 0.0]], [0.0, 0.0, 0.0]));
        assert!(!is_point_in_region(
            &[
                [-1.0, 0.0, -1.0],
                [-1.0, 0.0, 1.0],
                [1.0, 0.0, 1.0],
                [1.0, 0.0, -1.0],
                [2.0, 0.0, 2.0],
            ],
            [0.0, 0.0, 0.0]
        ));
    }
}
