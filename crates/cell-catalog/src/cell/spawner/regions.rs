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
/// arrives from the client at the moment *its* pawn crossed the volume,
/// while the test runs against the last position the server accepted.
///
/// **Why 1.5 is enough.** The client sends position at roughly 10 Hz
/// (`MovementValidator::MAX_SNAP_BACK_CORRECTIONS` documents the same rate)
/// and the fastest populated world's `run_speed` is `8.125` units/second
/// (`MovementValidator::DEFAULT_TOP_SPEED`, from
/// `db/resources/Worlds/Seed/worlds.sql`). One update interval is therefore
/// 0.81 units of travel, and 1.5 units buys 185 ms — one interval plus 84 ms
/// of slack. A player sprinting through a door-width volume is covered; two
/// *consecutive* dropped position packets (203 ms of staleness) would not
/// be, and would surface as a `region_containment_failed` warn rather than
/// as silence. Widening this weakens the gate; narrowing it starts
/// false-rejecting real entries.
pub const GENERIC_REGION_CHECK_THRESHOLD: f32 = 1.5;

/// XZ point-in-polygon (ray casting) against a region's `points`.
///
/// **The single source of truth for exact region containment.** Lives here,
/// beside the loader that produces `points`, so a change to the shape and a
/// change to the test that reads it land in one file. The playtest-friction
/// watcher and the `.bug` bookmark reach it through
/// `playtest_friction::region_contains_xz`, which is a re-export of this.
///
/// Exact, untoleranced and vertical-axis-blind on purpose: its callers ask
/// "which region is this player standing in?", where a tight answer means
/// fewer false *diagnostics*. The security gate asks a different question
/// and wants a tolerance band — see [`is_point_in_region`], which is built
/// on top of this so the two can never disagree about a point that is
/// genuinely inside.
pub fn region_contains_xz(points: &[[f32; 3]], x: f32, z: f32) -> bool {
    if points.len() < 3 {
        return false;
    }
    let mut inside = false;
    let mut j = points.len() - 1;
    for i in 0..points.len() {
        let (xi, zi) = (points[i][0], points[i][2]);
        let (xj, zj) = (points[j][0], points[j][2]);
        if (zi > z) != (zj > z) && x < (xj - xi) * (z - zi) / (zj - zi) + xi {
            inside = !inside;
        }
        j = i;
    }
    inside
}

/// `(min, max)` of `points` on `axis` (0 = X, 1 = Y, 2 = Z).
fn axis_extent(points: &[[f32; 3]], axis: usize) -> (f32, f32) {
    points
        .iter()
        .map(|p| p[axis])
        .fold((f32::INFINITY, f32::NEG_INFINITY), |(min, max), v| {
            (min.min(v), max.max(v))
        })
}

/// Is `point` inside the region described by `points`, allowing for a
/// client whose position the server has not caught up with?
///
/// Port of `deprecated/python/cell/GenericRegion.py:isPointInRegion` — an
/// axis-aligned bounding box over the region's four corners, widened by
/// [`GENERIC_REGION_CHECK_THRESHOLD`] on every axis, **including the
/// vertical one**: the Python tests `minPos.y - thresh <= point.y <=
/// maxPos.y + thresh` alongside X and Z, so a flying or under-floor client
/// does not trigger a ground-level volume. The Python carries a
/// `# TODO: Do precise check, AABB isn't enough!`; this is the same
/// approximation, deliberately, because the AABB is also the shape the
/// client was handed by `addClientHintedGenericRegion` and a tighter server
/// test would reject entries the client legitimately reported.
///
/// **Relationship to [`region_contains_xz`].** This is the tolerance band
/// around it, not a second implementation: anything the exact test accepts,
/// this accepts, because the AABB-plus-slop strictly contains the polygon.
/// The `||` below makes that structural rather than a comment, so the
/// friction watcher can never report "player is dwelling in region X" while
/// this gate is silently refusing X's hints. The Y band applies to both
/// arms — that is the one axis where this is *stricter*, and deliberately:
/// `region_contains_xz` ignores height entirely.
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

    let (min_y, max_y) = axis_extent(points, 1);
    if point[1] < min_y - t || point[1] > max_y + t {
        return false;
    }

    if region_contains_xz(points, point[0], point[2]) {
        return true;
    }

    let (min_x, max_x) = axis_extent(points, 0);
    let (min_z, max_z) = axis_extent(points, 2);
    point[0] >= min_x - t && point[0] <= max_x + t && point[2] >= min_z - t && point[2] <= max_z + t
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
