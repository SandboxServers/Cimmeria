//! Unit tests for the floor probe — all against synthetic triangle
//! soups, so they run without the cooked asset bundle.

use super::*;
use crate::geometry::TriangleSoup;

fn approx(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-3
}

/// A horizontal quad (two triangles) at UE3 Z = `z_cm`, spanning
/// `x_cm ± half` and `y_cm ± half` in the UE3 horizontal plane.
///
/// Under [`AxisMapping::CA05`] this becomes a BigWorld surface at
/// `y = z_cm / 100`, centred on `(y_cm / 100, _, x_cm / 100)`.
///
/// The winding is **clockwise in the UE3 XY plane**, i.e. the
/// right-hand normal points along −Z. That is what a Recast-walkable
/// floor looks like coming out of this extractor, because NavBuilder's
/// `loadOBJ` reverses the index order before Recast sees it — see
/// [`recast_up`]. [`ue3_ceiling_quad`] is the same quad wound the other
/// way.
fn ue3_floor_quad(x_cm: f32, y_cm: f32, z_cm: f32, half: f32) -> TriangleSoup {
    let mut soup = TriangleSoup::new(None);
    let a = [x_cm - half, y_cm - half, z_cm];
    let b = [x_cm + half, y_cm - half, z_cm];
    let c = [x_cm + half, y_cm + half, z_cm];
    let d = [x_cm - half, y_cm + half, z_cm];
    soup.push([c, b, a]);
    soup.push([d, c, a]);
    soup
}

/// The same quad with the opposite winding — Recast sees this as a
/// downward-facing surface (a ceiling), never as walkable ground.
fn ue3_ceiling_quad(x_cm: f32, y_cm: f32, z_cm: f32, half: f32) -> TriangleSoup {
    let mut soup = TriangleSoup::new(None);
    let a = [x_cm - half, y_cm - half, z_cm];
    let b = [x_cm + half, y_cm - half, z_cm];
    let c = [x_cm + half, y_cm + half, z_cm];
    let d = [x_cm - half, y_cm + half, z_cm];
    soup.push([a, b, c]);
    soup.push([a, c, d]);
    soup
}

// ---------------------------------------------------------------------
// AxisMapping
// ---------------------------------------------------------------------

#[test]
fn ca05_mapping_matches_the_worknote_ground_truth_actors() {
    // docs/analysis/castle-rebuild/worknotes/ca05.md:101-105.
    let stargate = AxisMapping::CA05.apply([54999.79, 76451.27, 6187.99]);
    assert!(approx(stargate[0], 764.5127), "x: {stargate:?}");
    assert!(approx(stargate[1], 61.8799), "y: {stargate:?}");
    assert!(approx(stargate[2], 549.9979), "z: {stargate:?}");

    let pillar = AxisMapping::CA05.apply([63600.0, 35342.0, 3817.0]);
    assert!(approx(pillar[0], 353.42), "x: {pillar:?}");
    assert!(approx(pillar[1], 38.17), "y: {pillar:?}");
    assert!(approx(pillar[2], 636.0), "z: {pillar:?}");
}

/// The mapping NavBuilder's `loadOBJ` currently produces from a raw
/// UE3-cm OBJ feeds BigWorld's up axis from UE3's horizontal Y — so the
/// same Stargate actor lands at y = 764.5 instead of 61.9. Pinning this
/// keeps the "axis convention is a known unknown" README bullet honest.
#[test]
fn navbuilder_on_raw_ue3_puts_a_horizontal_axis_on_bigworld_up() {
    let v = AxisMapping::NAVBUILDER_ON_RAW_UE3.apply([54999.79, 76451.27, 6187.99]);
    assert!(approx(v[1], 764.5127), "expected UE3 Y on BW up, got {v:?}");
    assert_ne!(AxisMapping::NAVBUILDER_ON_RAW_UE3, AxisMapping::CA05);
}

#[test]
fn label_round_trips_through_from_label() {
    for m in AxisMapping::all() {
        let label = m.label();
        assert_eq!(
            AxisMapping::from_label(&label),
            Some(m),
            "label {label} did not round-trip"
        );
    }
}

#[test]
fn ca05_label_is_readable() {
    assert_eq!(AxisMapping::CA05.label(), "+Y+Z+X");
    assert_eq!(AxisMapping::NAVBUILDER_ON_RAW_UE3.label(), "+Z+Y+X");
}

#[test]
fn from_label_rejects_malformed_and_non_permutations() {
    assert!(AxisMapping::from_label("+Y+Z").is_none(), "too short");
    assert!(AxisMapping::from_label("+Y+Z+X+").is_none(), "too long");
    assert!(AxisMapping::from_label("*Y+Z+X").is_none(), "bad sign");
    assert!(AxisMapping::from_label("+Q+Z+X").is_none(), "bad axis");
    assert!(
        AxisMapping::from_label("+X+X+Z").is_none(),
        "repeated axis is not a permutation"
    );
}

#[test]
fn all_yields_48_distinct_mappings_including_the_two_named_ones() {
    let all = AxisMapping::all();
    assert_eq!(all.len(), 48);
    let mut labels: Vec<String> = all.iter().map(|m| m.label()).collect();
    labels.sort();
    labels.dedup();
    assert_eq!(labels.len(), 48, "duplicate mapping in all()");
    assert!(all.contains(&AxisMapping::CA05));
    assert!(all.contains(&AxisMapping::NAVBUILDER_ON_RAW_UE3));
}

#[test]
fn negation_flips_the_sourced_axis_only() {
    let m = AxisMapping::from_label("-Y+Z+X").unwrap();
    let v = m.apply([100.0, 200.0, 300.0]);
    assert!(approx(v[0], -2.0), "{v:?}");
    assert!(approx(v[1], 3.0), "{v:?}");
    assert!(approx(v[2], 1.0), "{v:?}");
}

// ---------------------------------------------------------------------
// Probe math
// ---------------------------------------------------------------------

#[test]
fn triangle_up_is_one_for_a_flat_ccw_floor_and_zero_for_a_wall() {
    let floor = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]];
    assert!(triangle_up(&floor).abs() > 0.99);

    let wall = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
    assert!(triangle_up(&wall).abs() < 1e-5);

    let degenerate = [[0.0; 3], [0.0; 3], [0.0; 3]];
    assert_eq!(triangle_up(&degenerate), 0.0);
}

/// NavBuilder's `loadOBJ` reverses the index order (`mesh.cpp:123-128`),
/// so what Recast rasterises as ground is the *negation* of the
/// right-hand normal of the order we emitted. If this ever collapses
/// to `recast_up == triangle_up`, the probe starts calling ceilings
/// floors and every coverage number it reports is about the wrong
/// surface.
#[test]
fn recast_up_is_the_negation_of_the_emitted_order_normal() {
    let tri = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]];
    assert!(approx(recast_up(&tri), -triangle_up(&tri)));
    let reversed = [tri[2], tri[1], tri[0]];
    assert!(approx(recast_up(&reversed), -recast_up(&tri)));
}

/// The fixture pair must actually disagree — otherwise every
/// walkability assertion below is vacuous.
#[test]
fn the_floor_and_ceiling_fixtures_have_opposite_recast_normals() {
    let floor = ue3_floor_quad(0.0, 0.0, 0.0, 100.0);
    let ceiling = ue3_ceiling_quad(0.0, 0.0, 0.0, 100.0);
    let tri = |s: &TriangleSoup| {
        let m = AxisMapping::CA05;
        [
            m.apply(s.vertices[0]),
            m.apply(s.vertices[1]),
            m.apply(s.vertices[2]),
        ]
    };
    assert!(recast_up(&tri(&floor)) > 0.99, "floor must face up");
    assert!(recast_up(&tri(&ceiling)) < -0.99, "ceiling must face down");
}

/// A surface with the wrong winding directly under the point is not a
/// floor, however flat it is.
#[test]
fn a_downward_facing_surface_under_the_point_is_not_a_floor() {
    let soup = ue3_ceiling_quad(20_000.0, 10_000.0, 5_000.0, 500.0);
    let mut run = ProbeRun::new(
        AxisMapping::CA05,
        ProbeConfig::default(),
        one_point([100.0, 50.0, 200.0], Confidence::High),
    );
    run.add_soup(&soup);
    let r = &run.results()[0];
    assert!(!r.has_floor(), "{r:?}");
    assert_eq!(r.column_tris, 2, "it is still in the vertical column");
    assert_eq!(r.column_walkable, 0, "but it faces the wrong way");
}

#[test]
fn xz_height_interpolates_inside_and_rejects_outside() {
    // A sloped triangle: y rises from 0 at x=0 to 10 at x=10.
    let tri = [[0.0, 0.0, 0.0], [10.0, 10.0, 0.0], [0.0, 0.0, 10.0]];
    assert!(approx(xz_height_at(&tri, 0.0, 0.0).unwrap(), 0.0));
    assert!(approx(xz_height_at(&tri, 5.0, 0.0).unwrap(), 5.0));
    assert!(xz_height_at(&tri, 20.0, 0.0).is_none(), "outside");
    assert!(xz_height_at(&tri, -1.0, -1.0).is_none(), "outside");
}

#[test]
fn xz_height_is_winding_agnostic() {
    let cw = [[0.0, 3.0, 0.0], [10.0, 3.0, 0.0], [0.0, 3.0, 10.0]];
    let ccw = [[0.0, 3.0, 0.0], [0.0, 3.0, 10.0], [10.0, 3.0, 0.0]];
    assert!(approx(xz_height_at(&cw, 1.0, 1.0).unwrap(), 3.0));
    assert!(approx(xz_height_at(&ccw, 1.0, 1.0).unwrap(), 3.0));
}

#[test]
fn xz_height_returns_none_for_a_vertical_triangle() {
    // All three vertices share the same Z and X-span only — no XZ area.
    let wall = [[0.0, 0.0, 5.0], [10.0, 0.0, 5.0], [10.0, 8.0, 5.0]];
    assert!(xz_height_at(&wall, 5.0, 5.0).is_none());
}

// ---------------------------------------------------------------------
// ProbeRun
// ---------------------------------------------------------------------

fn one_point(bw: [f32; 3], confidence: Confidence) -> Vec<ProbePoint> {
    vec![ProbePoint::new("p", confidence, bw, "synthetic")]
}

#[test]
fn floor_directly_under_the_point_is_a_hit() {
    // BW point (100, 50, 200) -> under CA05 that is UE3
    // (x=20000, y=10000, z=5000).
    let soup = ue3_floor_quad(20_000.0, 10_000.0, 5_000.0, 500.0);
    let mut run = ProbeRun::new(
        AxisMapping::CA05,
        ProbeConfig::default(),
        one_point([100.0, 50.0, 200.0], Confidence::High),
    );
    run.add_soup(&soup);

    let r = &run.results()[0];
    assert!(r.has_floor(), "{r:?}");
    // The point sits on the quad's shared diagonal, so the edge epsilon
    // in `xz_height_at` deliberately lets BOTH triangles claim it —
    // better a double count than a point falling through the seam
    // between two floor tiles.
    assert_eq!(r.floor_hits, 2, "both triangles share the centre diagonal");
    assert!(approx(r.best_floor_y.unwrap(), 50.0));
    assert_eq!(run.high_confidence_hits(), 1);
}

/// Off the shared diagonal, exactly one triangle should claim the point.
#[test]
fn a_point_inside_one_triangle_of_the_quad_hits_once() {
    let soup = ue3_floor_quad(20_000.0, 10_000.0, 5_000.0, 500.0);
    // Nudge to BW (99, 50, 202): UE3 (20200, 9900, 5000), inside the
    // quad but off its centre diagonal.
    let mut run = ProbeRun::new(
        AxisMapping::CA05,
        ProbeConfig::default(),
        one_point([99.0, 50.0, 202.0], Confidence::High),
    );
    run.add_soup(&soup);
    assert_eq!(run.results()[0].floor_hits, 1);
}

#[test]
fn the_same_geometry_misses_under_the_wrong_mapping() {
    let soup = ue3_floor_quad(20_000.0, 10_000.0, 5_000.0, 500.0);
    let mut run = ProbeRun::new(
        AxisMapping::NAVBUILDER_ON_RAW_UE3,
        ProbeConfig::default(),
        one_point([100.0, 50.0, 200.0], Confidence::High),
    );
    run.add_soup(&soup);
    let r = &run.results()[0];
    assert!(
        !r.has_floor(),
        "wrong mapping must not score a floor: {r:?}"
    );
    assert_eq!(run.high_confidence_hits(), 0);
}

#[test]
fn a_floor_below_the_window_is_not_a_hit_but_is_reported_as_a_column_gap() {
    // Floor 10 BW units below the point — far outside `below = 1.5`.
    let soup = ue3_floor_quad(20_000.0, 10_000.0, 4_000.0, 500.0);
    let mut run = ProbeRun::new(
        AxisMapping::CA05,
        ProbeConfig::default(),
        one_point([100.0, 50.0, 200.0], Confidence::High),
    );
    run.add_soup(&soup);

    let r = &run.results()[0];
    assert!(!r.has_floor());
    assert_eq!(
        r.column_walkable, 2,
        "both quad triangles sit in the column"
    );
    assert!(approx(r.column_walkable_best_below_y.unwrap(), 40.0));
    assert!(approx(r.column_gap(50.0).unwrap(), 10.0));
}

#[test]
fn a_floor_above_the_point_is_not_claimed_as_its_floor() {
    // Ceiling 5 BW units above the point.
    let soup = ue3_floor_quad(20_000.0, 10_000.0, 5_500.0, 500.0);
    let mut run = ProbeRun::new(
        AxisMapping::CA05,
        ProbeConfig::default(),
        one_point([100.0, 50.0, 200.0], Confidence::High),
    );
    run.add_soup(&soup);

    let r = &run.results()[0];
    assert!(!r.has_floor());
    assert_eq!(r.column_walkable, 2);
    assert!(
        r.column_walkable_best_below_y.is_none(),
        "a ceiling is not a surface below the point"
    );
}

/// The distinguishing case the spike exists to answer: walls around the
/// point, no floor under it. `near_vertical` must be non-zero while
/// `floor_hits` stays zero, so the report can say "the transform is
/// right, the floors are missing" rather than "nothing is here".
#[test]
fn walls_without_a_floor_are_distinguishable_from_empty_space() {
    // A vertical wall in BigWorld terms: constant UE3 X (which CA05 maps
    // to BW z), spanning UE3 Y (BW x) and UE3 Z (BW up).
    let mut soup = TriangleSoup::new(None);
    let x = 20_100.0; // BW z = 201, 1 unit from the point
    soup.push([
        [x, 9_500.0, 4_800.0],
        [x, 10_500.0, 4_800.0],
        [x, 10_500.0, 5_300.0],
    ]);
    soup.push([
        [x, 9_500.0, 4_800.0],
        [x, 10_500.0, 5_300.0],
        [x, 9_500.0, 5_300.0],
    ]);

    let mut run = ProbeRun::new(
        AxisMapping::CA05,
        ProbeConfig::default(),
        one_point([100.0, 50.0, 200.0], Confidence::High),
    );
    run.add_soup(&soup);

    let r = &run.results()[0];
    assert!(!r.has_floor(), "a wall is not a floor");
    assert_eq!(r.column_tris, 0, "a vertical face has no XZ footprint");
    // AABB distance, not vertex distance: every vertex of this wall is
    // >5 units away, so a vertex-based neighbourhood would report the
    // point as standing in empty space. Reverting
    // `point_to_triangle_aabb_distance` to a vertex scan fails here.
    assert!(
        approx(r.nearest_dist.unwrap(), 1.0),
        "wall face is 1 BW unit away: {r:?}"
    );
    assert_eq!(r.near_tris, 2, "the wall is inside the neighbourhood");
    assert_eq!(r.near_vertical, r.near_tris, "all of it is vertical");
    assert_eq!(r.near_walkable, 0);
    assert_eq!(run.points_with_nearby_geometry(), 1);
    assert_eq!(run.points_with_floor(), 0);
}

#[test]
fn an_empty_neighbourhood_reports_the_nearest_distance() {
    // Floor 1000 BW units away on x.
    let soup = ue3_floor_quad(20_000.0, 110_000.0, 5_000.0, 500.0);
    let mut run = ProbeRun::new(
        AxisMapping::CA05,
        ProbeConfig::default(),
        one_point([100.0, 50.0, 200.0], Confidence::High),
    );
    run.add_soup(&soup);

    let r = &run.results()[0];
    assert!(!r.has_floor());
    assert_eq!(r.near_tris, 0);
    assert_eq!(r.column_tris, 0);
    let d = r.nearest_dist.expect("nearest distance recorded");
    assert!(d > 990.0 && d < 1010.0, "nearest distance {d}");
}

#[test]
fn point_to_aabb_distance_is_zero_inside_and_axis_wise_outside() {
    let tri = [[0.0, 0.0, 0.0], [10.0, 0.0, 0.0], [0.0, 4.0, 10.0]];
    assert_eq!(point_to_triangle_aabb_distance(&tri, [5.0, 2.0, 5.0]), 0.0);
    // 3 past the x max, 4 past the z max: sqrt(9 + 16) = 5.
    assert!(approx(
        point_to_triangle_aabb_distance(&tri, [13.0, 2.0, 14.0]),
        5.0
    ));
    // Below the box on y only.
    assert!(approx(
        point_to_triangle_aabb_distance(&tri, [5.0, -2.5, 5.0]),
        2.5
    ));
}

#[test]
fn a_steep_ramp_beyond_the_slope_limit_is_not_walkable() {
    // Rises 10 BW units over 1 BW unit of run — way past 45°. Wound
    // the floor way round, so it is rejected for its slope rather than
    // for facing downwards.
    let mut soup = TriangleSoup::new(None);
    soup.push([
        [19_950.0, 10_050.0, 5_000.0],
        [20_050.0, 9_950.0, 6_000.0],
        [19_950.0, 9_950.0, 5_000.0],
    ]);
    let up = recast_up(&[
        AxisMapping::CA05.apply(soup.vertices[0]),
        AxisMapping::CA05.apply(soup.vertices[1]),
        AxisMapping::CA05.apply(soup.vertices[2]),
    ]);
    assert!(up > 0.0, "the ramp faces up, it is just too steep: {up}");
    assert!(up < ProbeConfig::default().min_up, "up = {up}");

    let mut run = ProbeRun::new(
        AxisMapping::CA05,
        ProbeConfig::default(),
        one_point([99.7, 50.0, 199.8], Confidence::High),
    );
    run.add_soup(&soup);

    let r = &run.results()[0];
    assert!(r.column_tris >= 1, "the ramp does cover the point in XZ");
    assert_eq!(r.column_walkable, 0, "but it exceeds the 45 degree limit");
    assert!(!r.has_floor());
}

#[test]
fn high_and_medium_points_are_scored_separately() {
    let soup = ue3_floor_quad(20_000.0, 10_000.0, 5_000.0, 500.0);
    let points = vec![
        ProbePoint::new("hi", Confidence::High, [100.0, 50.0, 200.0], "s"),
        // Far away — no floor.
        ProbePoint::new("med", Confidence::Medium, [900.0, 50.0, 900.0], "s"),
    ];
    let mut run = ProbeRun::new(AxisMapping::CA05, ProbeConfig::default(), points);
    run.add_soup(&soup);

    assert_eq!(run.points_with_floor(), 1);
    assert_eq!(run.high_confidence_hits(), 1);
    assert!(!run.results()[1].has_floor());
}

#[test]
fn a_soup_with_a_dangling_face_index_is_skipped_not_panicked_on() {
    let mut soup = TriangleSoup::new(None);
    soup.push([[0.0; 3], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]]);
    // Point past the end of `vertices`.
    soup.faces.push([1, 2, 99]);

    let mut run = ProbeRun::new(
        AxisMapping::CA05,
        ProbeConfig::default(),
        one_point([0.0, 0.0, 0.0], Confidence::High),
    );
    run.add_soup(&soup);
    assert_eq!(run.results().len(), 1);
}

// ---------------------------------------------------------------------
// The shipped probe set
// ---------------------------------------------------------------------

#[test]
fn castle_probe_points_have_three_high_confidence_entries_and_unique_labels() {
    let points = castle_probe_points();
    let high = points
        .iter()
        .filter(|p| p.confidence == Confidence::High)
        .count();
    assert_eq!(high, 3, "the rules file pins exactly three HIGH points");

    let mut labels: Vec<&str> = points.iter().map(|p| p.label.as_str()).collect();
    labels.sort_unstable();
    labels.dedup();
    assert_eq!(labels.len(), points.len(), "duplicate probe-point label");

    for p in &points {
        assert!(!p.source.is_empty(), "{} has no cited source", p.label);
    }
}
