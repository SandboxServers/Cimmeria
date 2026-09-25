//! The explorable-area trim: which navmesh components an entry point
//! claims, and how far the claim grows.

use super::explorable::*;
use crate::nav_components::{NavGraph, NavPoly};

fn point(x: f32, y: f32, z: f32) -> EntryPoint {
    EntryPoint {
        source: "test".into(),
        pos: [x, y, z],
    }
}

/// Four unlinked 10 x 10 quads, one component each: the first at the
/// origin, the second 1 m past it (a door gap), the third 40 m away, the
/// fourth right above the first at y 8 (the storey above).
fn islands() -> NavGraph {
    let mut verts = Vec::new();
    let mut polys = Vec::new();
    for (x, y) in [(0.0, 0.0), (11.0, 0.0), (50.0, 0.0), (0.0, 8.0)] {
        let base = verts.len() as u32;
        verts.extend([
            [x, y, 0.0],
            [x + 10.0, y, 0.0],
            [x + 10.0, y, 10.0],
            [x, y, 10.0],
        ]);
        polys.push(NavPoly {
            verts: (base..base + 4).collect(),
            neighbours: vec![None; 4],
            portal_links: vec![],
            area: 63,
            flags: 1,
            region: 0,
        });
    }
    NavGraph {
        cs: 1.0,
        ch: 1.0,
        bmin: [0.0; 3],
        bmax: [60.0, 8.0, 10.0],
        verts,
        polys,
        component: vec![0, 1, 2, 3],
        component_count: 4,
        asymmetric_links: vec![],
    }
}

#[test]
fn entry_points_claim_their_component_and_growth_crosses_a_door_but_not_a_storey() {
    let g = islands();
    let ex = explorable_components(&g, &[point(2.0, 0.5, 2.0), point(300.0, 0.0, 0.0)]);
    assert_eq!(ex.located, 1);
    assert_eq!(
        ex.unlocated.len(),
        1,
        "a point far off the mesh claims nothing"
    );
    let mut kept = ex.components;
    assert_eq!(kept, [0].into_iter().collect());
    let added = grow_components(&g, &mut kept);
    assert_eq!(added, 1);
    assert!(kept.contains(&1), "the 1 m gap is a door");
    assert!(!kept.contains(&2), "40 m away is another place");
    assert!(!kept.contains(&3), "8 m up is another storey");
    assert_eq!(component_triangles(&g, &kept).len(), 4);
}

#[test]
fn an_entry_point_above_the_floor_within_tolerance_still_counts() {
    let g = islands();
    let ex = explorable_components(&g, &[point(55.0, 3.0, 5.0)]);
    assert_eq!(ex.components, [2].into_iter().collect());
    let ex = explorable_components(&g, &[point(55.0, 20.0, 5.0)]);
    assert!(ex.components.is_empty(), "20 m up is not standing on it");
}

#[test]
fn the_entry_point_file_keeps_this_world_and_wildcards() {
    let dir = std::env::temp_dir().join(format!("occ-entry-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let p = dir.join("e.tsv");
    std::fs::write(
        &p,
        "world\tsource\tx\ty\tz\nharset\ta\t1\t2\t3\ncastle\tb\t4\t5\t6\n*\tc\t7\t8\t9\nharset\tbad\tx\t2\t3\n",
    )
    .unwrap();
    let pts = read_entry_points(&p, "harset").unwrap();
    let srcs: Vec<&str> = pts.iter().map(|p| p.source.as_str()).collect();
    assert_eq!(srcs, ["a", "c"]);
    assert_eq!(pts[0].pos, [1.0, 2.0, 3.0]);
}
