//! Connectivity and probe-location tests for [`super::NavGraph`].
//!
//! Gap-finder tests live next door in [`super::gaps`].

use super::test_mesh::MeshFixture;
use super::*;

/// Two quads sharing an edge → one component.
#[test]
fn linked_quads_form_one_component() {
    let mut b = MeshFixture::new();
    let p0 = b.quad(0, 5, 0);
    let p1 = b.quad(1, 5, 0);
    // p0 edge 1 is (x+1,z)→(x+1,z+1); p1 edge 3 is (x,z+1)→(x,z).
    b.link(p0, 1, p1, 3);
    let g = NavGraph::from_nav(&b.build());

    assert_eq!(g.component_count, 1, "adjacent quads must share a region");
    assert!(g.asymmetric_links.is_empty());
    assert_eq!(g.component, vec![0, 0]);
}

/// Same two quads with the adjacency slots left at 0xffff → two
/// islands. This is the failure shape CA14 cares about: the mesh
/// loads, has polys, and is still unusable.
#[test]
fn unlinked_quads_are_two_islands() {
    let mut b = MeshFixture::new();
    b.quad(0, 5, 0);
    b.quad(50, 5, 50);
    let g = NavGraph::from_nav(&b.build());

    assert_eq!(g.component_count, 2);
    let stats = g.component_stats();
    assert_eq!(stats.len(), 2);
    for s in &stats {
        assert_eq!(s.poly_count, 1);
        assert!(
            (s.area_xz - 1.0).abs() < 1e-6,
            "unit quad at cs=1 has area 1, got {}",
            s.area_xz
        );
    }
}

/// A chain of quads must collapse to a single component regardless of
/// which end the flood fill seeds from.
#[test]
fn chain_of_quads_is_one_component() {
    let mut b = MeshFixture::new();
    let mut prev = b.quad(0, 5, 0);
    for i in 1..10u16 {
        let cur = b.quad(i, 5, 0);
        b.link(prev, 1, cur, 3);
        prev = cur;
    }
    let g = NavGraph::from_nav(&b.build());
    assert_eq!(g.component_count, 1);
    assert_eq!(g.polys.len(), 10);
    let stats = g.component_stats();
    assert_eq!(stats[0].poly_count, 10);
    assert!((stats[0].area_xz - 10.0).abs() < 1e-6);
    assert_eq!(stats[0].bmin[0], 0.0);
    assert_eq!(stats[0].bmax[0], 10.0);
}

/// Two islands of unequal size: `component_stats` must sort the bigger
/// one first so callers can treat index 0 as "the main region".
#[test]
fn component_stats_sorts_largest_first() {
    let mut b = MeshFixture::new();
    let lone = b.quad(90, 5, 90);
    let _ = lone;
    let mut prev = b.quad(0, 5, 0);
    for i in 1..5u16 {
        let cur = b.quad(i, 5, 0);
        b.link(prev, 1, cur, 3);
        prev = cur;
    }
    let g = NavGraph::from_nav(&b.build());
    let stats = g.component_stats();
    assert_eq!(stats.len(), 2);
    assert_eq!(stats[0].poly_count, 5);
    assert_eq!(stats[1].poly_count, 1);
}

/// Probe inside a quad: zero horizontal distance, exact vertical
/// delta, correct component.
#[test]
fn probe_inside_quad_resolves_exactly() {
    let mut b = MeshFixture::new();
    b.quad(10, 7, 20);
    let g = NavGraph::from_nav(&b.build());
    // cs = ch = 1 and bmin = 0, so grid == world here.
    let hit = g.locate([10.5, 9.0, 20.5]).expect("probe must resolve");
    assert_eq!(hit.poly, 0);
    assert_eq!(hit.component, 0);
    assert_eq!(hit.horizontal_distance, 0.0);
    assert!(
        (hit.vertical_distance - 2.0).abs() < 1e-4,
        "expected +2.0, got {}",
        hit.vertical_distance
    );
}

/// The stacked-mesh false negative: a buried sheet whose footprint
/// covers the probe must not out-rank a real floor beside it.
///
/// Shape taken from the 17-tile Castle interior build — the BSP hull
/// skin at BW y ~79.5 spans whole chunks, so `throne_room` (floor at
/// y 38.5) resolved onto it at dy -41.35 and was reported OUT OF
/// TOLERANCE while standing on the mesh.
#[test]
fn a_roof_sheet_over_the_probe_does_not_hide_the_floor_beside_it() {
    let mut b = MeshFixture::new();
    // Roof: a 3x3 sheet 41 m up, covering everything below it.
    let roof = b.wide_quad(0, 41, 0, 3, 3);
    // Floor: one unit quad at ground level, one unit to the +x side
    // of the probe, so its XZ footprint does NOT contain the probe.
    let floor = b.quad(2, 0, 1);
    let g = NavGraph::from_nav(&b.build());
    assert_ne!(g.component[roof as usize], g.component[floor as usize]);

    // Probe at (1.5, 0.0, 1.5): inside the roof's footprint, 0.5 m
    // horizontally from the floor quad's x=2 edge.
    let p = [1.5, 0.0, 1.5];

    let naive = g.locate(p).expect("some poly");
    assert_eq!(
        naive.poly,
        u32::from(roof),
        "the pre-fix behaviour: the containing roof wins outright"
    );
    assert!(
        (naive.vertical_distance + 41.0).abs() < 1e-3,
        "and reports a 41 m drop: {}",
        naive.vertical_distance
    );

    let fixed = g.locate_within(p, 2.0, 3.0).expect("some poly");
    assert_eq!(
        fixed.poly,
        u32::from(floor),
        "tolerance-first must pick the floor 0.5 m to the side, not \
         the roof 41 m overhead"
    );
    assert!(fixed.vertical_distance.abs() <= 3.0);
    assert!((fixed.horizontal_distance - 0.5).abs() < 1e-3);
}

/// When the probe really is inside a polygon at the right height,
/// tolerance-first must still pick that polygon and not something
/// beside it.
#[test]
fn tolerance_first_still_prefers_the_polygon_under_the_probe() {
    let mut b = MeshFixture::new();
    let under = b.quad(1, 5, 1);
    b.quad(2, 5, 1);
    let g = NavGraph::from_nav(&b.build());
    let hit = g.locate_within([1.5, 5.0, 1.5], 2.0, 3.0).unwrap();
    assert_eq!(hit.poly, u32::from(under));
    assert_eq!(hit.horizontal_distance, 0.0);
}

/// Nothing within tolerance: fall back to the nearest polygon so the
/// caller can still report how far off the probe is.
#[test]
fn tolerance_first_falls_back_when_nothing_qualifies() {
    let mut b = MeshFixture::new();
    b.quad(0, 5, 0);
    let g = NavGraph::from_nav(&b.build());
    let hit = g.locate_within([100.0, 5.0, 0.5], 2.0, 3.0).unwrap();
    assert_eq!(hit.poly, 0);
    assert!(hit.horizontal_distance > 2.0);
}

/// Off-mesh probe: reports the real horizontal gap, not a silent hit.
#[test]
fn probe_off_mesh_reports_horizontal_distance() {
    let mut b = MeshFixture::new();
    b.quad(0, 5, 0);
    let g = NavGraph::from_nav(&b.build());
    let hit = g
        .locate([10.0, 5.0, 0.5])
        .expect("nearest poly still returned");
    assert_eq!(hit.poly, 0);
    assert!(
        (hit.horizontal_distance - 9.0).abs() < 1e-3,
        "expected ~9.0 from the quad's x=1 edge, got {}",
        hit.horizontal_distance
    );
}

/// Two probes on opposite islands must report different component ids
/// — the exact condition `nav_inspect` exits non-zero on.
#[test]
fn probes_on_separate_islands_differ_in_component() {
    let mut b = MeshFixture::new();
    b.quad(0, 5, 0);
    b.quad(50, 5, 50);
    let g = NavGraph::from_nav(&b.build());
    let a = g.locate([0.5, 5.0, 0.5]).unwrap();
    let c = g.locate([50.5, 5.0, 50.5]).unwrap();
    assert_eq!(a.horizontal_distance, 0.0);
    assert_eq!(c.horizontal_distance, 0.0);
    assert_ne!(a.component, c.component);
}

/// A one-sided neighbour link is reported rather than silently
/// unioning the two polys.
#[test]
fn one_sided_link_is_flagged_asymmetric() {
    let mut b = MeshFixture::new();
    b.quad(0, 5, 0);
    b.quad(1, 5, 0);
    b.polys[0][4 + 1] = 1; // only one direction
    let g = NavGraph::from_nav(&b.build());
    assert_eq!(g.asymmetric_links, vec![(0, 1)]);
}

/// Non-zero `bmin`/`cs`/`ch` must be applied on dequantisation — a
/// regression here would report probe hits in the wrong place while
/// every connectivity assertion still passed.
#[test]
fn vertices_dequantise_with_bmin_cs_ch() {
    let mut b = MeshFixture::new();
    b.cs = 0.3;
    b.ch = 0.2;
    b.quad(10, 5, 20);
    let mut nav = b.build();
    nav.bmin = [-400.0, -500.0, -400.0];
    let g = NavGraph::from_nav(&nav);
    assert!((g.verts[0][0] - (-400.0 + 3.0)).abs() < 1e-4);
    assert!((g.verts[0][1] - (-500.0 + 1.0)).abs() < 1e-4);
    assert!((g.verts[0][2] - (-400.0 + 6.0)).abs() < 1e-4);
}

/// `0x8000`-flagged neighbour slots are tile portals, not poly index
/// `0x7fff`. Decoding them as an index would either panic on the
/// out-of-range lookup or fabricate connectivity.
#[test]
fn portal_flagged_neighbour_is_not_a_poly_index() {
    let mut b = MeshFixture::new();
    b.quad(0, 5, 0);
    b.polys[0][4] = RC_PORTAL_FLAG | 2;
    let g = NavGraph::from_nav(&b.build());
    assert_eq!(g.polys[0].neighbours[0], None);
    assert_eq!(g.component_count, 1);
    assert!(g.asymmetric_links.is_empty());
}
