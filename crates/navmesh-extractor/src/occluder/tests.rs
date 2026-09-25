//! Synthetic tests for the occluder walk helpers, the exact tracer and the
//! sweep's navmesh ray. The asset-backed accuracy numbers come from the
//! `occluder_extract measure` CLI, not from here.

use super::exact::{segment_hits_triangle, ExactScene};
use super::sweep::{nav_ray, sample_pairs, sample_points, NavRay, Rng, SweepParams};
use super::*;
use crate::nav_components::{NavGraph, NavPoly};

#[test]
fn ue3_centimetres_map_to_bigworld_metres() {
    // UE3 (x, y, z-up) cm -> BW (x = ue.y, y-up = ue.z, z = ue.x) m.
    assert_eq!(ue3_to_bw([100.0, 200.0, 300.0]), [2.0, 3.0, 1.0]);
}

fn wall(x: f32) -> Vec<BwTriangle> {
    vec![
        [[x, 0.0, -10.0], [x, 3.0, -10.0], [x, 0.0, 10.0]],
        [[x, 3.0, -10.0], [x, 3.0, 10.0], [x, 0.0, 10.0]],
    ]
}

#[test]
fn the_exact_tracer_is_blocked_by_a_wall_and_sees_over_it() {
    let scene = ExactScene::new(wall(5.0));
    assert_eq!(scene.len(), 2);
    assert!(scene.blocked([0.0, 1.5, 0.0], [10.0, 1.5, 0.0]));
    assert!(!scene.blocked([0.0, 4.0, 0.0], [10.0, 4.0, 0.0]));
    // Stops short of the wall.
    assert!(!scene.blocked([0.0, 1.5, 0.0], [4.9, 1.5, 0.0]));
    // A long diagonal crossing many buckets still finds it, and a second
    // query (new stamp) does too.
    assert!(scene.blocked([-30.0, 1.5, -8.0], [40.0, 1.5, 9.0]));
    assert!(scene.blocked([-30.0, 1.5, -8.0], [40.0, 1.5, 9.0]));
    // Both faces count: the same crossing from the other side.
    assert!(segment_hits_triangle(
        [6.0, 0.5, -9.0],
        [4.0, 0.5, -9.0],
        &wall(5.0)[0]
    ));
    assert!(segment_hits_triangle(
        [4.0, 0.5, -9.0],
        [6.0, 0.5, -9.0],
        &wall(5.0)[0]
    ));
}

/// Two 10 x 10 quads side by side along x, optionally linked.
fn two_quads(linked: bool) -> NavGraph {
    let verts = vec![
        [0.0, 0.0, 0.0],
        [10.0, 0.0, 0.0],
        [10.0, 0.0, 10.0],
        [0.0, 0.0, 10.0],
        [20.0, 0.0, 0.0],
        [20.0, 0.0, 10.0],
    ];
    // Edge i joins verts[i] -> verts[i+1]; quad 0's edge 1 is x = 10, and
    // quad 1's edge 3 is x = 10.
    let polys = vec![
        NavPoly {
            verts: vec![0, 1, 2, 3],
            neighbours: vec![None, linked.then_some(1), None, None],
            portal_links: vec![],
            area: 63,
            flags: 1,
            region: 0,
        },
        NavPoly {
            verts: vec![1, 4, 5, 2],
            neighbours: vec![None, None, None, linked.then_some(0)],
            portal_links: vec![],
            area: 63,
            flags: 1,
            region: 0,
        },
    ];
    NavGraph {
        cs: 1.0,
        ch: 1.0,
        bmin: [0.0; 3],
        bmax: [20.0, 0.0, 10.0],
        verts,
        polys,
        component: vec![0, if linked { 0 } else { 1 }],
        component_count: if linked { 1 } else { 2 },
        asymmetric_links: vec![],
    }
}

#[test]
fn the_navmesh_ray_crosses_a_shared_edge_and_stops_at_a_boundary() {
    let linked = two_quads(true);
    assert_eq!(
        nav_ray(&linked, 0, [2.0, 0.0, 5.0], [18.0, 0.0, 6.0]),
        NavRay::Clear
    );
    assert_eq!(
        nav_ray(&linked, 0, [2.0, 0.0, 5.0], [4.0, 0.0, 6.0]),
        NavRay::Clear,
        "ends inside the start polygon"
    );
    assert_eq!(
        nav_ray(&linked, 0, [2.0, 0.0, 5.0], [18.0, 0.0, 12.0]),
        NavRay::Blocked,
        "leaves the mesh through z = 10"
    );
    let split = two_quads(false);
    assert_eq!(
        nav_ray(&split, 0, [2.0, 0.0, 5.0], [18.0, 0.0, 6.0]),
        NavRay::Blocked
    );
    // The same two quads as neighbouring tiles of a tiled mesh: the shared
    // edge is a portal (`None` in `neighbours`, linked in `portal_links`).
    let mut tiled = two_quads(false);
    tiled.polys[0].portal_links = vec![1];
    tiled.polys[1].portal_links = vec![0];
    assert_eq!(
        nav_ray(&tiled, 0, [2.0, 0.0, 5.0], [18.0, 0.0, 6.0]),
        NavRay::Clear,
        "a tile portal is not a wall"
    );
}

#[test]
fn sampled_points_lie_on_the_mesh_and_pairs_honour_the_constraints() {
    let g = two_quads(true);
    let mut rng = Rng::new(7);
    let pts = sample_points(&g, 500, &mut rng, |_| true);
    assert_eq!(pts.len(), 500);
    for p in &pts {
        assert!((0.0..=20.0).contains(&p.pos[0]) && (0.0..=10.0).contains(&p.pos[2]));
        let x_ok = if p.poly == 0 {
            p.pos[0] <= 10.0
        } else {
            p.pos[0] >= 10.0
        };
        assert!(x_ok, "{p:?}");
    }
    let params = SweepParams {
        pairs: 200,
        ..SweepParams::default()
    };
    let pairs = sample_pairs(&g, &params);
    assert_eq!(pairs.len(), 200);
    // Excluding every component leaves nothing to sample.
    let none = SweepParams {
        max_component_area: Some(1.0),
        ..params
    };
    assert!(sample_pairs(&g, &none).is_empty());
    for (a, b) in &pairs {
        let d = ((b.pos[0] - a.pos[0]).powi(2) + (b.pos[2] - a.pos[2]).powi(2)).sqrt();
        assert!((params.min_dist..=params.max_dist).contains(&d));
    }
}
