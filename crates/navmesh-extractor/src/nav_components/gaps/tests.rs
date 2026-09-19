use super::super::test_mesh::MeshFixture;
use super::super::NavGraph;

/// Two unlinked quads one metre apart: the gap finder must report the pair
/// once, with the real 1 m horizontal distance and no vertical step.
#[test]
fn two_islands_report_their_horizontal_gap() {
    let mut b = MeshFixture::new();
    b.quad(0, 5, 0);
    b.quad(2, 5, 0); // x in [2,3]; the first quad ends at x = 1
    let g = NavGraph::from_nav(&b.build());
    assert_eq!(g.component_count, 2);

    let gaps = g.component_gaps(3.0, 3.0, 5);
    assert_eq!(gaps.pair_count(), 1, "one pair, not one per edge");
    let best = gaps.best(0, 1).expect("the pair is within 3 m");
    assert!(
        (best.horizontal - 1.0).abs() < 1e-4,
        "expected a 1.00 m gap, got {}",
        best.horizontal
    );
    assert!(best.vertical.abs() < 1e-4);
    assert!((best.point_from[0] - 1.0).abs() < 1e-4);
    assert!((best.point_to[0] - 2.0).abs() < 1e-4);
}

/// Orientation: `best(a, b)` and `best(b, a)` describe the same gap from
/// opposite ends, with the vertical sign flipped.
#[test]
fn approach_orientation_follows_the_argument_order() {
    let mut b = MeshFixture::new();
    b.quad(0, 5, 0);
    b.quad(2, 7, 0); // two metres up
    let g = NavGraph::from_nav(&b.build());
    let gaps = g.component_gaps(3.0, 3.0, 5);

    let fwd = gaps.best(0, 1).unwrap();
    let rev = gaps.best(1, 0).unwrap();
    assert!((fwd.vertical - 2.0).abs() < 1e-4, "got {}", fwd.vertical);
    assert!((rev.vertical + 2.0).abs() < 1e-4, "got {}", rev.vertical);
    assert_eq!(fwd.point_from, rev.point_to);
    assert!((fwd.horizontal - rev.horizontal).abs() < 1e-6);
}

/// Beyond `h_max` the pair must not appear at all — otherwise the chain
/// search would happily "bridge" across a canyon.
#[test]
fn a_pair_beyond_the_horizontal_threshold_is_not_reported() {
    let mut b = MeshFixture::new();
    b.quad(0, 5, 0);
    b.quad(20, 5, 0);
    let g = NavGraph::from_nav(&b.build());
    assert_eq!(g.component_gaps(3.0, 3.0, 5).pair_count(), 0);
    assert_eq!(g.component_gaps(25.0, 3.0, 5).pair_count(), 1);
}

/// Stacked floors: zero horizontal gap, the whole obstacle is the step. This
/// is the `agentClimb` shape, and it must not be reported as "touching".
#[test]
fn stacked_floors_report_zero_horizontal_and_the_full_step() {
    let mut b = MeshFixture::new();
    b.ch = 0.2;
    // Two quads sharing the x = 1 line in XZ, 1.0 m apart vertically
    // (5 cells at ch = 0.2).
    b.quad(0, 0, 0);
    b.quad(1, 5, 0);
    let g = NavGraph::from_nav(&b.build());
    let gaps = g.component_gaps(1.0, 3.0, 5);
    let best = gaps.best(0, 1).expect("rims touch in XZ");
    assert!(
        best.horizontal < 1e-4,
        "shared rim line ⇒ zero XZ gap, got {}",
        best.horizontal
    );
    assert!(
        (best.vertical - 1.0).abs() < 1e-4,
        "expected a 1.00 m step, got {}",
        best.vertical
    );
}

/// A step taller than `v_max` is not a bridgeable gap at all.
#[test]
fn a_step_beyond_the_vertical_threshold_is_not_reported() {
    let mut b = MeshFixture::new();
    b.quad(0, 0, 0);
    b.quad(1, 9, 0); // 9 m up
    let g = NavGraph::from_nav(&b.build());
    assert_eq!(g.component_gaps(2.0, 0.6, 5).pair_count(), 0);
    assert_eq!(g.component_gaps(2.0, 10.0, 5).pair_count(), 1);
}

/// The dedupe: one long rim facing another produces dozens of candidate edge
/// pairs at nearly the same place. They must collapse to one entry, while a
/// genuinely separate approach further along is kept.
#[test]
fn approaches_at_the_same_place_collapse_but_distinct_ones_do_not() {
    let mut b = MeshFixture::new();
    // Left wall: a 1 x 30 strip at x in [0,1].
    b.wide_quad(0, 5, 0, 1, 30);
    // Right side: two separate quads facing it, 20 m apart along z.
    b.wide_quad(2, 5, 0, 1, 2);
    b.wide_quad(2, 5, 25, 1, 2);
    let g = NavGraph::from_nav(&b.build());
    assert_eq!(g.component_count, 3);

    let gaps = g.component_gaps(3.0, 3.0, 5);
    // 0↔1, 0↔2 and 1↔2 (the two right-hand quads are 23 m apart in z, so
    // that last pair is out of range).
    assert_eq!(gaps.pair_count(), 2);
    for (a, b_) in gaps.pair_keys() {
        let list = gaps.approaches(a, b_);
        assert_eq!(
            list.len(),
            1,
            "one rim-to-rim approach per pair, got {} for ({a}, {b_})",
            list.len()
        );
        assert!((list[0].horizontal - 1.0).abs() < 1e-4);
    }
}

/// Three islands in a line, each 1 m from the next and 2 m end to end: the
/// chain search must report two hops, not one impossible 2 m jump, and must
/// find nothing at all when the threshold excludes the individual hops.
#[test]
fn chain_search_returns_the_intermediate_hops() {
    let mut b = MeshFixture::new();
    b.quad(0, 5, 0); // x [0,1]
    b.quad(2, 5, 0); // x [2,3]
    b.quad(4, 5, 0); // x [4,5]
    let g = NavGraph::from_nav(&b.build());
    assert_eq!(g.component_count, 3);

    let gaps = g.component_gaps(1.5, 3.0, 5);
    // End to end is 3 m, over the threshold, so no direct pair.
    assert!(gaps.best(0, 2).is_none());

    let chain = gaps.bottleneck_path(0, 2).expect("reachable in two hops");
    assert_eq!(chain.len(), 2, "two gaps to bridge, not one");
    assert_eq!(chain[0].from, 0);
    assert_eq!(chain[0].to, 1);
    assert_eq!(chain[1].from, 1);
    assert_eq!(chain[1].to, 2);
    for hop in &chain {
        assert!((hop.horizontal - 1.0).abs() < 1e-4);
    }

    // Below the per-hop threshold nothing links at all.
    let tight = g.component_gaps(0.5, 3.0, 5);
    assert!(tight.bottleneck_path(0, 2).is_none());
}

/// The chain search minimises the *widest* hop, not the total: a two-hop
/// route whose worst gap is 1 m beats a one-hop 2.5 m jump.
#[test]
fn chain_search_prefers_the_narrowest_worst_gap() {
    let mut b = MeshFixture::new();
    //   A: x [0,1] z [0,1]
    //   B: x [2,3] z [0,1]   (1 m from A)
    //   C: x [4,5] z [0,1]   (1 m from B, 3 m from A)
    // plus a direct A→C shortcut via z: D bridges nothing, so make the
    // direct A↔C distance 3 m and allow it with h_max = 3.5.
    b.quad(0, 5, 0);
    b.quad(2, 5, 0);
    b.quad(4, 5, 0);
    let g = NavGraph::from_nav(&b.build());
    let gaps = g.component_gaps(3.5, 3.0, 5);

    let direct = gaps.best(0, 2).expect("3 m direct gap is inside 3.5 m");
    assert!((direct.horizontal - 3.0).abs() < 1e-4);

    let chain = gaps.bottleneck_path(0, 2).unwrap();
    assert_eq!(
        chain.len(),
        2,
        "two 1 m hops must beat one 3 m hop on the bottleneck key"
    );
    let widest = chain.iter().fold(0.0f32, |m, h| m.max(h.horizontal));
    assert!(widest < direct.horizontal);
}

/// The storey-jump regression. Two floors of one building whose rims overlap
/// in XZ report `horizontal = 0.00` and the whole obstacle in `dy`. Ranking
/// hops on the horizontal gap alone scores that jump as **free**, so the
/// chain search takes it in preference to a real multi-hop route — which is
/// exactly what `nav_inspect --gaps` did on Castle: "one hop, widest 0.00 m"
/// for a pair whose actual connection is a stairwell 30 m away.
///
/// `Approach::bridge_size` is `max(horizontal, |vertical|)`, so the jump
/// costs 12 and the stair route wins.
#[test]
fn a_stacked_storey_jump_does_not_beat_a_real_route() {
    let mut b = MeshFixture::new();
    // Lower floor and upper floor, footprints overlapping along x = 4,
    // 12 m apart (12 cells at ch = 1.0).
    let lower = b.wide_quad(0, 0, 0, 4, 4);
    let upper = b.wide_quad(4, 12, 0, 4, 4);
    // A stair: three landings climbing 4 m at a time up the +z side, each
    // 1 m from the next in XZ.
    let s1 = b.wide_quad(0, 4, 6, 2, 2);
    let s2 = b.wide_quad(3, 8, 6, 2, 2);
    let s3 = b.wide_quad(6, 12, 6, 2, 2);
    let g = NavGraph::from_nav(&b.build());
    let (lower, upper) = (g.component[lower as usize], g.component[upper as usize]);
    for s in [s1, s2, s3] {
        assert_ne!(
            g.component[s as usize], lower,
            "the stair is its own island"
        );
    }

    let gaps = g.component_gaps(3.0, 13.0, 5);

    // The direct storey jump exists and looks free horizontally.
    let jump = gaps.best(lower, upper).expect("rims overlap in XZ");
    assert!(jump.horizontal < 1e-4, "got {}", jump.horizontal);
    assert!((jump.vertical.abs() - 12.0).abs() < 1e-4);
    assert!(
        (jump.bridge_size() - 12.0).abs() < 1e-4,
        "bridge_size must see the 12 m drop, got {}",
        jump.bridge_size()
    );

    let chain = gaps.bottleneck_path(lower, upper).expect("a route exists");
    assert!(
        chain.len() > 1,
        "the 12 m storey jump must not win as a single free hop; got {chain:?}"
    );
    let widest = chain.iter().fold(0.0f32, |m, h| m.max(h.bridge_size()));
    assert!(
        widest < 12.0,
        "the stair route's worst bridge must beat the jump's 12 m, got {widest}"
    );
}

/// Disconnected in the gap graph too: no chain, and the caller can say so
/// rather than printing a nonsense route.
#[test]
fn chain_search_reports_no_route_when_there_is_none() {
    let mut b = MeshFixture::new();
    b.quad(0, 5, 0);
    b.quad(50, 5, 50);
    let g = NavGraph::from_nav(&b.build());
    let gaps = g.component_gaps(3.0, 3.0, 5);
    assert_eq!(gaps.pair_count(), 0);
    assert!(gaps.bottleneck_path(0, 1).is_none());
    assert_eq!(
        gaps.bottleneck_path(0, 0).map(|c| c.len()),
        Some(0),
        "a component reaches itself in zero hops"
    );
}

/// Polygons that *are* linked never appear as a gap — their shared edge is
/// not a boundary edge.
#[test]
fn linked_polygons_have_no_boundary_edge_between_them() {
    let mut b = MeshFixture::new();
    let p0 = b.quad(0, 5, 0);
    let p1 = b.quad(1, 5, 0);
    b.link(p0, 1, p1, 3);
    let g = NavGraph::from_nav(&b.build());
    assert_eq!(g.component_count, 1);
    // 8 sides total, 2 of them linked ⇒ 6 boundary edges.
    assert_eq!(g.boundary_edges().len(), 6);
    assert_eq!(g.component_gaps(3.0, 3.0, 5).pair_count(), 0);
}

/// The component filter cuts the search down without changing the answer for
/// the components that survive it.
#[test]
fn the_component_filter_only_removes_pairs() {
    let mut b = MeshFixture::new();
    b.quad(0, 5, 0);
    b.quad(2, 5, 0);
    b.quad(4, 5, 0);
    let g = NavGraph::from_nav(&b.build());
    let all = g.component_gaps(1.5, 3.0, 5);
    assert_eq!(all.pair_count(), 2);

    let filtered = g.component_gaps_filtered(1.5, 3.0, 5, |c| c != 1);
    assert_eq!(filtered.pair_count(), 0, "removing the middle breaks both");
    let kept = g.component_gaps_filtered(1.5, 3.0, 5, |c| c != 2);
    assert_eq!(kept.pair_count(), 1);
    assert!((kept.best(0, 1).unwrap().horizontal - 1.0).abs() < 1e-4);
}

/// A diagonal rim: the segment-to-segment solver has to find an interior
/// closest point on both segments, not just compare endpoints.
#[test]
fn closest_approach_can_be_interior_to_both_edges() {
    let mut b = MeshFixture::new();
    b.cs = 0.5;
    // Two long parallel strips 1.0 m apart (2 cells at cs = 0.5), offset in
    // z so that no pair of endpoints is the closest pair.
    b.wide_quad(0, 5, 0, 2, 40);
    b.wide_quad(4, 5, 20, 2, 40);
    let g = NavGraph::from_nav(&b.build());
    let gaps = g.component_gaps(2.0, 3.0, 5);
    let best = gaps.best(0, 1).expect("1 m apart");
    assert!(
        (best.horizontal - 1.0).abs() < 1e-4,
        "expected 1.00 m across the parallel strips, got {}",
        best.horizontal
    );
    // The closest point must be somewhere in the overlapping z range
    // [10, 20], not at a strip end.
    assert!(
        best.point_from[2] >= 10.0 - 1e-3 && best.point_from[2] <= 20.0 + 1e-3,
        "closest point at z = {}, expected it inside the overlap",
        best.point_from[2]
    );
}
