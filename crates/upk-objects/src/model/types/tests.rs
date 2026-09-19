//! Unit tests for the decoded `UModel` types and the node-fan
//! triangulator in [`super`].

use super::*;

/// A model with one square face in the XY plane at z = 0, wound
/// counter-clockwise when viewed from +Z. Right-hand-rule normal
/// of that order is +Z.
fn square_model(poly_flags: u32, node_flags: u8) -> Model {
    Model {
        vectors: vec![[0.0, 0.0, 1.0]],
        points: vec![
            [0.0, 0.0, 0.0],
            [100.0, 0.0, 0.0],
            [100.0, 100.0, 0.0],
            [0.0, 100.0, 0.0],
        ],
        verts: vec![
            BspVert { p_vertex: 0 },
            BspVert { p_vertex: 1 },
            BspVert { p_vertex: 2 },
            BspVert { p_vertex: 3 },
        ],
        surfs: vec![BspSurf {
            poly_flags,
            v_normal: 0,
            plane: [0.0, 0.0, 1.0, 0.0],
            ..Default::default()
        }],
        nodes: vec![BspNode {
            i_vert_pool: 0,
            i_surf: 0,
            num_vertices: 4,
            node_flags,
            ..Default::default()
        }],
        ..Default::default()
    }
}

fn normal_of(t: [[f32; 3]; 3]) -> [f32; 3] {
    let u = [t[1][0] - t[0][0], t[1][1] - t[0][1], t[1][2] - t[0][2]];
    let v = [t[2][0] - t[0][0], t[2][1] - t[0][1], t[2][2] - t[0][2]];
    [
        u[1] * v[2] - u[2] * v[1],
        u[2] * v[0] - u[0] * v[2],
        u[0] * v[1] - u[1] * v[0],
    ]
}

#[test]
fn quad_node_fans_into_two_triangles_in_vert_pool_order() {
    // WINDING PIN. The fan is `(corners[0], corners[k], corners[k+1])`
    // over the node's `iVertPool` slice **as stored** — the
    // triangulator never reverses. `bsp.rs`'s real-data winding
    // check (floors must emit with UE3 n.z < 0 so NavBuilder sees
    // them as walkable) is only meaningful if this stays true, so a
    // change here must be a deliberate, re-measured decision.
    let m = square_model(0, 0);
    let t = m.triangulate(CollisionFilter::default());
    assert_eq!(t.triangles.len(), 2);
    assert_eq!(
        t.triangles[0],
        [[0.0, 0.0, 0.0], [100.0, 0.0, 0.0], [100.0, 100.0, 0.0]]
    );
    assert_eq!(
        t.triangles[1],
        [[0.0, 0.0, 0.0], [100.0, 100.0, 0.0], [0.0, 100.0, 0.0]]
    );
    // Both emitted triangles carry the same right-hand-rule normal
    // direction as the stored vertex order (+Z here).
    assert!(normal_of(t.triangles[0])[2] > 0.0);
    assert!(normal_of(t.triangles[1])[2] > 0.0);
    assert_eq!(t.triangle_surf, vec![0, 0]);
}

#[test]
fn surf_normal_is_independent_of_emitted_winding() {
    // The floor probe must not depend on fan direction: it reads
    // the authored normal out of `Vectors[vNormal]`.
    let mut m = square_model(0, 0);
    m.vectors = vec![[0.0, 0.0, -1.0]];
    let t = m.triangulate(CollisionFilter::default());
    // Emitted winding still says +Z…
    assert!(normal_of(t.triangles[0])[2] > 0.0);
    // …but the authored surface normal says -Z.
    assert_eq!(m.surf_normal(0), Some([0.0, 0.0, -1.0]));
}

#[test]
fn surf_normal_falls_back_to_the_plane_when_vnormal_is_out_of_range() {
    let mut m = square_model(0, 0);
    m.surfs[0].v_normal = 99;
    assert_eq!(m.surf_normal(0), Some([0.0, 0.0, 1.0]));
}

#[test]
fn default_filter_drops_invisible_and_not_solid_surfaces() {
    for bit in [PF_INVISIBLE, PF_NOT_SOLID, PF_PORTAL] {
        let m = square_model(bit, 0);
        let t = m.triangulate(CollisionFilter::default());
        assert!(
            t.triangles.is_empty(),
            "flag {bit:#x} should have been filtered out"
        );
        assert_eq!(t.nodes_excluded, 1);
        assert_eq!(t.triangles_excluded, 2);
    }
}

#[test]
fn default_filter_keeps_semisolid_and_two_sided_surfaces() {
    // Both still block; excluding them would punch holes in walls.
    for bit in [PF_SEMISOLID, PF_TWO_SIDED, 0x0000_0E00, 0x0000_0200] {
        let m = square_model(bit, 0);
        let t = m.triangulate(CollisionFilter::default());
        assert_eq!(t.triangles.len(), 2, "flag {bit:#x} should be kept");
    }
}

#[test]
fn keep_all_filter_emits_everything() {
    let m = square_model(PF_INVISIBLE | PF_NOT_SOLID, 0xff);
    let t = m.triangulate(CollisionFilter::KEEP_ALL);
    assert_eq!(t.triangles.len(), 2);
    assert_eq!(t.triangles_excluded, 0);
}

#[test]
fn per_flag_exclusion_counts_are_reported_even_when_the_flag_is_not_filtered() {
    // This is the "make a wrong assumption visible" mechanism: the
    // counts are computed for every entry in the table regardless
    // of whether the active filter uses that bit.
    let m = square_model(PF_INVISIBLE, 0);
    let t = m.triangulate(CollisionFilter::KEEP_ALL);
    let inv = t
        .excluded_by_flag
        .iter()
        .find(|e| e.0 == "PF_Invisible")
        .expect("PF_Invisible must be in the report table");
    assert_eq!(inv.2, 2, "both fan triangles carry the flag");
    let ns = t
        .excluded_by_flag
        .iter()
        .find(|e| e.0 == "PF_NotSolid")
        .expect("PF_NotSolid must be in the report table");
    assert_eq!(ns.2, 0);
}

#[test]
fn node_flag_filter_is_off_by_default_but_configurable() {
    let m = square_model(0, NF_NOT_CSG);
    assert_eq!(
        m.triangulate(CollisionFilter::default()).triangles.len(),
        2,
        "NF_NotCsg must NOT be excluded by default — every brush-local \
         Model node is outside the level's CSG set"
    );
    let strict = CollisionFilter {
        poly_flag_mask: 0,
        node_flag_mask: NF_NOT_CSG,
    };
    assert!(m.triangulate(strict).triangles.is_empty());
}

#[test]
fn splitter_nodes_with_zero_vertices_are_counted_not_emitted() {
    let mut m = square_model(0, 0);
    m.nodes[0].num_vertices = 0;
    let t = m.triangulate(CollisionFilter::default());
    assert!(t.triangles.is_empty());
    assert_eq!(t.nodes_without_vertices, 1);
    assert_eq!(t.nodes_out_of_range, 0);
}

#[test]
fn nodes_with_fewer_than_three_vertices_are_degenerate() {
    let mut m = square_model(0, 0);
    m.nodes[0].num_vertices = 2;
    let t = m.triangulate(CollisionFilter::default());
    assert!(t.triangles.is_empty());
    assert_eq!(t.nodes_degenerate, 1);
}

#[test]
fn out_of_range_indices_are_counted_not_panicked_on() {
    // A bad iSurf, a bad iVertPool and a bad pVertex each drop the
    // node. Non-zero `nodes_out_of_range` on real data means the
    // deserializer's field offsets are wrong.
    let mut bad_surf = square_model(0, 0);
    bad_surf.nodes[0].i_surf = 99;
    assert_eq!(
        bad_surf
            .triangulate(CollisionFilter::default())
            .nodes_out_of_range,
        1
    );

    let mut bad_pool = square_model(0, 0);
    bad_pool.nodes[0].i_vert_pool = 3; // 3 + 4 > verts.len()
    assert_eq!(
        bad_pool
            .triangulate(CollisionFilter::default())
            .nodes_out_of_range,
        1
    );

    let mut bad_point = square_model(0, 0);
    bad_point.verts[2].p_vertex = 77;
    assert_eq!(
        bad_point
            .triangulate(CollisionFilter::default())
            .nodes_out_of_range,
        1
    );

    let mut negative = square_model(0, 0);
    negative.nodes[0].i_vert_pool = -1;
    assert_eq!(
        negative
            .triangulate(CollisionFilter::default())
            .nodes_out_of_range,
        1
    );
}

#[test]
fn histograms_group_face_carrying_nodes_by_flag_value() {
    let mut m = square_model(0x0000_0E00, 0);
    // Second surf/node pair with a different PolyFlags value.
    m.surfs.push(BspSurf {
        poly_flags: 0x0000_0200,
        v_normal: 0,
        ..Default::default()
    });
    m.nodes.push(BspNode {
        i_vert_pool: 0,
        i_surf: 1,
        num_vertices: 4,
        node_flags: 0,
        ..Default::default()
    });
    m.nodes.push(BspNode {
        i_vert_pool: 0,
        i_surf: 1,
        num_vertices: 4,
        node_flags: 0,
        ..Default::default()
    });
    let t = m.triangulate(CollisionFilter::default());
    // Descending by count: 0x200 appears on two nodes, 0xE00 on one.
    assert_eq!(
        t.poly_flag_histogram,
        vec![(0x0000_0200, 2), (0x0000_0E00, 1)]
    );
    assert_eq!(t.node_flag_histogram, vec![(0u8, 3)]);
    assert_eq!(t.nodes_total, 3);
    assert_eq!(t.triangles.len(), 6);
}

#[test]
fn default_filter_mask_matches_the_published_table() {
    // Guards the const-fold in `Default` against the table drifting
    // out from under it.
    let expected = NON_COLLIDING_POLY_FLAGS.iter().fold(0u32, |a, e| a | e.1);
    assert_eq!(CollisionFilter::default().poly_flag_mask, expected);
    assert_eq!(CollisionFilter::default().node_flag_mask, 0);
}

#[test]
fn poly_fan_matches_node_fan_for_the_same_corner_order() {
    // The Polys path is a redundant copy of the same surfaces; a
    // quad must fan the same way from either source.
    let p = Poly {
        vertices: vec![
            [0.0, 0.0, 0.0],
            [100.0, 0.0, 0.0],
            [100.0, 100.0, 0.0],
            [0.0, 100.0, 0.0],
        ],
        ..Default::default()
    };
    let node_tris = square_model(0, 0)
        .triangulate(CollisionFilter::default())
        .triangles;
    assert_eq!(p.triangles(), node_tris);
}

#[test]
fn degenerate_poly_fans_to_nothing() {
    let p = Poly {
        vertices: vec![[0.0; 3], [1.0, 0.0, 0.0]],
        ..Default::default()
    };
    assert!(p.triangles().is_empty());
    assert_eq!(Polys { elements: vec![p] }.triangle_count(), 0);
}
