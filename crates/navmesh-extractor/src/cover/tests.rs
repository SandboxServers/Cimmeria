//! Synthetic-fixture tests for the cover extractor. Nothing here reads
//! the client tree, so all of it runs in CI (the PR #683 lesson: an
//! asset-only test self-skips and covers nothing).

use std::f32::consts::{FRAC_PI_2, PI, TAU};

use super::*;
use crate::test_support::{scratch_dir, ChunkFixture, CoverComponentSpec, CoverMarker, Placement};

const EPS: f32 = 1e-4;

/// Positions: f32 at ~600 m resolves to ~6e-5 m, so 1 mm.
fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-3
}

/// Angular distance, wrap-aware.
fn angle_close(a: f32, b: f32) -> bool {
    let d = (a - b).rem_euclid(TAU);
    d < EPS || TAU - d < EPS
}

fn deg(units_degrees: f32) -> i32 {
    (units_degrees * 65536.0 / 360.0).round() as i32
}

fn extract(fx: &ChunkFixture, tag: &str) -> MapCoverExtraction {
    let dir = scratch_dir(tag);
    fx.write(&dir, "Synth", 0xfffe_fffd);
    extract_map_cover(&dir).expect("extract synthetic map")
}

// ---- axis conversion and facing ---------------------------------------

#[test]
fn ue_to_bw_swaps_axes_and_scales_cm_to_m() {
    assert_eq!(ue_to_bw([100.0, 200.0, 300.0]), [2.0, 3.0, 1.0]);
    // The NA20 desk node, exactly as the finding's Q3 table converts it.
    let bw = ue_to_bw([-12470.96, -23470.83, 6547.17]);
    assert!(close(bw[0], -234.7083) && close(bw[1], 65.4717) && close(bw[2], -124.7096));
}

#[test]
fn orient_is_measured_from_bw_x_toward_bw_z() {
    let at = |yaw_deg: f32| {
        orient_from_transform(&ActorTransform {
            rotation: [0, deg(yaw_deg), 0],
            ..Default::default()
        })
        .unwrap()
    };
    // UE +X is BW +Z: (cos, sin) = (0, 1) -> pi/2.
    assert!(angle_close(at(0.0), FRAC_PI_2), "{}", at(0.0));
    // UE +Y is BW +X -> 0.
    assert!(angle_close(at(90.0), 0.0), "{}", at(90.0));
    // UE -X is BW -Z -> 3pi/2.
    assert!(angle_close(at(180.0), 3.0 * FRAC_PI_2), "{}", at(180.0));
    // Rotators are not normalised in the cook (the desk has a 450.63 deg
    // marker); the result must still land in [0, 2pi).
    let wrapped = at(450.0);
    assert!((0.0..TAU).contains(&wrapped) && angle_close(wrapped, 0.0));
}

#[test]
fn mirrored_x_scale_flips_facing_but_mirrored_y_does_not() {
    let base = ActorTransform {
        rotation: [0, deg(30.0), 0],
        ..Default::default()
    };
    let plain = orient_from_transform(&base).unwrap();
    let mirror_x = orient_from_transform(&ActorTransform {
        draw_scale_3d: [-1.0, 1.3, 1.067],
        ..base
    })
    .unwrap();
    let mirror_y = orient_from_transform(&ActorTransform {
        draw_scale_3d: [1.0, -1.3, 1.067],
        ..base
    })
    .unwrap();
    assert!(angle_close(mirror_x, plain + PI), "{plain} vs {mirror_x}");
    assert!(angle_close(mirror_y, plain), "{plain} vs {mirror_y}");
}

#[test]
fn zero_x_scale_has_no_facing() {
    assert!(orient_from_transform(&ActorTransform {
        draw_scale_3d: [0.0, 1.0, 1.0],
        ..Default::default()
    })
    .is_none());
}

// ---- Pattern A ----------------------------------------------------------

#[test]
fn spec_cover_node_is_read_from_its_actor_placement() {
    let mut fx = ChunkFixture::new();
    fx.add_spec_cover_node(
        Placement::at([-12470.96, -23470.83, 6547.17])
            .with_rotation([0, deg(89.74), 0])
            .with_draw_scale_3d([1.0, 0.376, 1.067]),
        CoverMarker::new(1, 1, 0.376),
    );
    let x = extract(&fx, "cover-a");
    assert_eq!(x.nodes.len(), 1);
    assert_eq!(x.stats.spec_nodes, 1);
    let n = &x.nodes[0];
    assert_eq!(n.pattern, CoverPattern::SpecNode);
    assert!(close(n.pos[0], -234.7083) && close(n.pos[1], 65.4717) && close(n.pos[2], -124.7096));
    // Yaw ~90 deg faces UE +Y = BW +X: orient ~0 (0.26 deg off).
    assert!(n.orient.min(TAU - n.orient) < 0.01, "{}", n.orient);
    assert_eq!((n.height, n.quality), (Height::Mid, Quality::Better));
    assert!(close(n.width, 0.376));
}

#[test]
fn absent_cover_properties_take_the_archetype_values_not_zero() {
    let mut fx = ChunkFixture::new();
    fx.add_spec_cover_node(Placement::at([0.0, 0.0, 0.0]), CoverMarker::default());
    let x = extract(&fx, "cover-defaults");
    let n = &x.nodes[0];
    assert_eq!(n.height, Height::Low);
    assert_eq!(
        n.quality,
        Quality::None_,
        "an omitted CoverQuality is the archetype's 3, never byte 0 (Good)"
    );
    assert!(close(n.width, 1.0));
    assert_eq!(
        (
            x.stats.height_defaulted,
            x.stats.quality_defaulted,
            x.stats.width_defaulted
        ),
        (1, 1, 1)
    );
}

#[test]
fn out_of_range_quality_is_emitted_as_none_and_counted() {
    let mut fx = ChunkFixture::new();
    fx.add_spec_cover_node(Placement::at([0.0; 3]), CoverMarker::new(1, 4, 1.7));
    let x = extract(&fx, "cover-q4");
    assert_eq!(x.nodes[0].quality, Quality::None_);
    assert_eq!(x.stats.quality_out_of_range, 1);
}

#[test]
fn out_of_range_height_skips_the_node_and_balances() {
    let mut fx = ChunkFixture::new();
    fx.add_spec_cover_node(Placement::at([0.0; 3]), CoverMarker::new(9, 1, 1.0));
    fx.add_spec_cover_node(
        Placement::at([500.0, 0.0, 0.0]),
        CoverMarker::new(2, 2, 1.0),
    );
    let x = extract(&fx, "cover-h9");
    assert_eq!(x.nodes.len(), 1);
    assert_eq!(x.stats.skipped_bad_height, 1);
    assert!(x.stats.is_balanced(), "{:?}", x.stats);
}

// ---- Pattern B ----------------------------------------------------------

fn child(t: [f32; 3], yaw_deg: f32, absolute: bool) -> CoverComponentSpec {
    CoverComponentSpec {
        translation: t,
        rotation: [0, deg(yaw_deg), 0],
        scale_3d: [1.5, 0.762246, 1.067],
        absolute,
        marker: CoverMarker::new(1, 1, 0.762246),
    }
}

#[test]
fn absolute_array_children_are_used_verbatim_not_composed() {
    let mut fx = ChunkFixture::new();
    // Owner rotated and far away: composing with it would move the nodes.
    let owner = Placement::at([61320.94, 56967.805, 2391.3513]).with_rotation([0, deg(90.0), 0]);
    fx.add_cover_node_array(
        owner,
        &[
            child([61474.31, 56928.684, 2391.3513], 146.25, true),
            child([61400.0, 57000.0, 2391.3513], 0.0, true),
        ],
    );
    let x = extract(&fx, "cover-b-abs");
    assert_eq!(x.stats.array_nodes, 2);
    assert_eq!(x.stats.array_nodes_composed, 0);
    assert_eq!(x.stats.array_nodes_unlisted, 0);
    let n = &x.nodes[0];
    assert_eq!(n.pattern, CoverPattern::NodeArray);
    assert!(
        close(n.pos[0], 569.28684) && close(n.pos[2], 614.7431),
        "{:?}",
        n.pos
    );
    // Yaw 0, absolute rotation: faces BW +Z whatever the owner's yaw.
    assert!(
        angle_close(x.nodes[1].orient, FRAC_PI_2),
        "{}",
        x.nodes[1].orient
    );
}

#[test]
fn relative_array_children_are_composed_with_the_owner() {
    let mut fx = ChunkFixture::new();
    let owner = Placement::at([1000.0, 2000.0, 300.0]).with_rotation([0, deg(90.0), 0]);
    fx.add_cover_node_array(owner, &[child([100.0, 0.0, 0.0], 0.0, false)]);
    let x = extract(&fx, "cover-b-rel");
    assert_eq!(x.stats.array_nodes_composed, 1);
    let n = &x.nodes[0];
    // Owner yaw 90 turns local +X (100 cm) into UE +Y: (1000, 2100, 300).
    assert!(
        close(n.pos[0], 21.0) && close(n.pos[1], 3.0) && close(n.pos[2], 10.0),
        "{:?}",
        n.pos
    );
    // And the child's facing turns with it: UE +Y = BW +X -> 0.
    assert!(angle_close(n.orient, 0.0), "{}", n.orient);
}

// ---- grouping -----------------------------------------------------------

/// The seven med-station desk markers from NA20 Q3, as UE3 placements.
fn desk_fixture(fx: &mut ChunkFixture) {
    let desk: [([f32; 3], f32, f32); 7] = [
        ([-12423.1, -23175.27, 6544.1], 179.96, 2.286),
        ([-12071.37, -23175.9, 6547.17], 180.62, 0.387),
        ([-12470.96, -23470.83, 6547.17], 89.74, 0.376),
        ([-12293.49, -23127.07, 6547.17], 269.57, 1.721),
        ([-12289.19, -23225.85, 6547.17], 450.63, 1.716),
        ([-12471.72, -22880.54, 6547.17], 270.79, 0.401),
        ([-12520.92, -23175.27, 6544.1], 0.0, 2.286),
    ];
    for (loc, yaw, width) in desk {
        fx.add_spec_cover_node(
            Placement::at(loc)
                .with_rotation([0, deg(yaw), 0])
                .with_draw_scale_3d([1.0, width, 1.067]),
            CoverMarker::new(1, 1, width),
        );
    }
}

#[test]
fn the_med_station_desk_is_one_set_of_seven() {
    let mut fx = ChunkFixture::new();
    desk_fixture(&mut fx);
    // A marker 20 m away, and one directly above the desk on the next floor.
    fx.add_spec_cover_node(
        Placement::at([-12470.0, -21000.0, 6547.17]),
        CoverMarker::new(1, 1, 1.0),
    );
    fx.add_spec_cover_node(
        Placement::at([-12470.96, -23470.83, 6547.17 + 400.0]),
        CoverMarker::new(1, 1, 1.0),
    );
    let x = extract(&fx, "cover-desk");
    let sets = group_into_sets(12, "Castle_CellBlock", x.nodes);
    assert_eq!(
        sets.len(),
        3,
        "{:?}",
        sets.iter().map(|s| s.nodes.len()).collect::<Vec<_>>()
    );
    assert_eq!(sets[0].nodes.len(), 7);
    assert_eq!(sets[0].set_id, 1_200_001);
    assert_eq!(sets[1].set_id, 1_200_002);
    assert!(sets.iter().all(|s| s.world_id == 12));
    // The desk's inward facing: the marker at the desk's -Y end has yaw
    // ~90 (UE +Y, toward the others), the +Y end has yaw ~270.
    let desk = &sets[0].nodes;
    assert!(desk[2].orient.min(TAU - desk[2].orient) < 0.01);
    let off = (desk[5].orient - PI).abs();
    assert!(off < 0.02, "{}", desk[5].orient);
}

#[test]
fn a_cover_node_array_is_one_set_however_spread_out() {
    let mut fx = ChunkFixture::new();
    fx.add_cover_node_array(
        Placement::at([0.0; 3]),
        &[
            child([0.0, 0.0, 0.0], 0.0, true),
            child([5000.0, 0.0, 0.0], 0.0, true),
        ],
    );
    // A spec marker right next to the first child stays in its own set:
    // an explicit array is never merged with implicit clusters.
    fx.add_spec_cover_node(Placement::at([50.0, 0.0, 0.0]), CoverMarker::new(1, 1, 1.0));
    let x = extract(&fx, "cover-b-group");
    let sets = group_into_sets(8, "Castle", x.nodes);
    assert_eq!(sets.len(), 2);
    assert_eq!(sets[0].nodes.len(), 2);
    assert_eq!(sets[0].pattern, CoverPattern::NodeArray);
    assert_eq!(sets[0].set_id, 800_001);
}

// ---- SQL ----------------------------------------------------------------

#[test]
fn seed_rows_carry_the_world_and_the_new_columns() {
    let mut fx = ChunkFixture::new();
    fx.add_spec_cover_node(
        Placement::at([100.0, 200.0, 300.0]).with_rotation([0, deg(90.0), 0]),
        CoverMarker::new(2, 2, 0.5),
    );
    let x = extract(&fx, "cover-sql");
    let sets = group_into_sets(12, "Castle_CellBlock", x.nodes);
    let lines = vec!["test line".to_string()];
    let prov = sql::Provenance {
        command: "cover_extract --test",
        client_build: "synthetic",
        map_lines: &lines,
    };
    let s = sql::render_cover_sets(&sets, &prov);
    assert!(s.contains("-- Regenerate: cover_extract --test"));
    assert!(s.contains("-- Client build: synthetic"));
    assert!(s.contains(
        "  (1200001, 12, 'Castle_CellBlock.Synth-fffefffd.SpecNode.2', 'cover-extract', false, 'Synth-fffefffd.umap');"
    ), "{s}");
    let n = sql::render_cover_nodes(&sets, &prov);
    assert!(n.contains(
        "  (1200001, 0, 2.0000, 3.0000, 1.0000, 0.000000, 'HEIGHT_High', 'QUALITY_Best', 0.5000, '\\x00000000'::bytea);"
    ), "{n}");
}
