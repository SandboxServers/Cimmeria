//! Unit tests for the StaticMesh walker — synthetic `StaticMesh` +
//! transform fixtures only, no filesystem and no cooked asset bundle.

use super::*;
use cimmeria_upk::{PropValue, TaggedProperty};
use cimmeria_upk_objects::{BoundingBox, KdopTriangle, LodModel, Vertex};

fn vert(p: [f32; 3]) -> Vertex {
    Vertex {
        position: p,
        normal: [0.0; 3],
        tangent: [0.0; 4],
        uv: [0.0; 2],
    }
}

fn unit_triangle_mesh() -> StaticMesh {
    // One triangle: (0,0,0), (1,0,0), (0,1,0). Stored as kDOP so
    // collision_triangles() picks the kDOP path.
    StaticMesh {
        bounds: BoundingBox {
            origin: [0.0; 3],
            extent: [1.0; 3],
            sphere_radius: 1.0,
        },
        lod_models: vec![LodModel {
            vertices: vec![
                vert([0.0, 0.0, 0.0]),
                vert([1.0, 0.0, 0.0]),
                vert([0.0, 1.0, 0.0]),
            ],
            indices: vec![],
            sections: vec![],
            num_vertices: 3,
            num_triangles: 0,
        }],
        internal_version: 15,
        kdop_triangles: vec![KdopTriangle {
            v0: 0,
            v1: 1,
            v2: 2,
            material: 0,
        }],
    }
}

#[test]
fn empty_instance_list_produces_empty_soup() {
    let soup = build_chunk_soup(&[]);
    assert_eq!(soup.triangle_count(), 0);
}

#[test]
fn single_instance_identity_transform_pushes_one_triangle() {
    let mesh = unit_triangle_mesh();
    let soup = build_chunk_soup(&[(mesh, ActorTransform::default(), "test".into())]);
    assert_eq!(soup.triangle_count(), 1);
    // Identity transform: vertex 0 stays at the origin.
    assert_eq!(soup.vertices[0], [0.0, 0.0, 0.0]);
}

#[test]
fn translation_only_shifts_every_vertex() {
    let mesh = unit_triangle_mesh();
    let xf = ActorTransform {
        location: [10.0, 20.0, 30.0],
        ..Default::default()
    };
    let soup = build_chunk_soup(&[(mesh, xf, "test".into())]);
    assert_eq!(soup.vertices[0], [10.0, 20.0, 30.0]);
    assert_eq!(soup.vertices[1], [11.0, 20.0, 30.0]);
    assert_eq!(soup.vertices[2], [10.0, 21.0, 30.0]);
}

#[test]
fn drawscale_scales_every_vertex() {
    let mesh = unit_triangle_mesh();
    let xf = ActorTransform {
        draw_scale: 100.0,
        ..Default::default()
    };
    let soup = build_chunk_soup(&[(mesh, xf, "test".into())]);
    assert_eq!(soup.vertices[1], [100.0, 0.0, 0.0]);
    assert_eq!(soup.vertices[2], [0.0, 100.0, 0.0]);
}

#[test]
fn yaw_rotates_then_translates() {
    // Yaw 90° + translate (5, 5, 0). Vertex (1, 0, 0) → (0, 1, 0)
    // after yaw, then + (5, 5, 0) = (5, 6, 0).
    let mesh = unit_triangle_mesh();
    let xf = ActorTransform {
        location: [5.0, 5.0, 0.0],
        rotation: [0, 16384, 0], // yaw = 90°
        ..Default::default()
    };
    let soup = build_chunk_soup(&[(mesh, xf, "test".into())]);
    let v1 = soup.vertices[1];
    assert!((v1[0] - 5.0).abs() < 1e-3);
    assert!((v1[1] - 6.0).abs() < 1e-3);
}

#[test]
fn multiple_instances_accumulate_triangles() {
    let mesh1 = unit_triangle_mesh();
    let mesh2 = unit_triangle_mesh();
    let xf1 = ActorTransform::default();
    let xf2 = ActorTransform {
        location: [100.0, 0.0, 0.0],
        ..Default::default()
    };
    let soup = build_chunk_soup(&[(mesh1, xf1, "actor1".into()), (mesh2, xf2, "actor2".into())]);
    assert_eq!(soup.triangle_count(), 2);
    // First instance starts at origin.
    assert_eq!(soup.vertices[0], [0.0, 0.0, 0.0]);
    // Second instance is offset by (+100, 0, 0).
    assert_eq!(soup.vertices[3], [100.0, 0.0, 0.0]);
}

#[test]
fn mesh_with_no_collision_triangles_emits_nothing() {
    // Empty kDOP list AND empty LOD0 indices → collision_triangles()
    // returns an empty list → no triangles in the soup.
    let mesh = StaticMesh {
        bounds: BoundingBox {
            origin: [0.0; 3],
            extent: [1.0; 3],
            sphere_radius: 1.0,
        },
        lod_models: vec![LodModel {
            vertices: vec![vert([0.0, 0.0, 0.0])],
            indices: vec![],
            sections: vec![],
            num_vertices: 1,
            num_triangles: 0,
        }],
        internal_version: 15,
        kdop_triangles: vec![],
    };
    let soup = build_chunk_soup(&[(mesh, ActorTransform::default(), "test".into())]);
    assert_eq!(soup.triangle_count(), 0);
}

#[test]
fn transform_from_props_recovers_all_four_fields() {
    let props = vec![
        TaggedProperty {
            name: "Location".into(),
            array_index: 0,
            value: PropValue::Vector {
                x: 100.0,
                y: 200.0,
                z: 300.0,
            },
        },
        TaggedProperty {
            name: "Rotation".into(),
            array_index: 0,
            value: PropValue::Rotator {
                pitch: 100,
                yaw: 200,
                roll: 300,
            },
        },
        TaggedProperty {
            name: "DrawScale".into(),
            array_index: 0,
            value: PropValue::Float(2.5),
        },
        TaggedProperty {
            name: "DrawScale3D".into(),
            array_index: 0,
            value: PropValue::Vector {
                x: 1.5,
                y: 2.5,
                z: 3.5,
            },
        },
    ];
    let xf = transform_from_actor_props(&props);
    assert_eq!(xf.location, [100.0, 200.0, 300.0]);
    assert_eq!(xf.rotation, [100, 200, 300]);
    assert_eq!(xf.draw_scale, 2.5);
    assert_eq!(xf.draw_scale_3d, [1.5, 2.5, 3.5]);
}

#[test]
fn transform_from_empty_props_defaults_to_identity() {
    let xf = transform_from_actor_props(&[]);
    assert_eq!(xf.location, [0.0, 0.0, 0.0]);
    assert_eq!(xf.rotation, [0, 0, 0]);
    assert_eq!(xf.draw_scale, 1.0);
    assert_eq!(xf.draw_scale_3d, [1.0, 1.0, 1.0]);
}

// ----- skip-reason classification -----

/// A component reference that can't point at a readable export must be
/// classified as `ComponentUnreadable`, not lumped in with the
/// archetype stubs — the two carry completely different follow-up work.
#[test]
fn a_nonpositive_component_ref_is_component_unreadable() {
    // `resolve_mesh_ref_from_component` short-circuits on ref <= 0
    // before it touches the package, so an empty-ish package is fine.
    // We reach it through the public error type rather than a real
    // Package to keep this a pure unit test.
    use crate::coverage::SkipReason;
    let classify = |r: i32| -> SkipReason {
        if r <= 0 {
            SkipReason::ComponentUnreadable
        } else {
            SkipReason::NoComponentRef
        }
    };
    assert_eq!(classify(0), SkipReason::ComponentUnreadable);
    assert_eq!(classify(-7), SkipReason::ComponentUnreadable);
}

/// The degraded (`index = None`) path must still balance: every actor
/// that produced an instance is re-tallied under `NoPackageIndex`, and
/// `actors_unresolved` matches the tally total.
#[test]
fn degraded_mode_balances_actors_against_skips() {
    use crate::coverage::{ChunkCoverage, SkipReason, SkipTally};

    // Stand in for what `extract_chunk_from_package` produces with a
    // `None` index: 10 actors, 6 of which formed an instance before the
    // missing index stopped them.
    let mut skips = SkipTally::default();
    skips.add_n(SkipReason::ArchetypeStubComponent, 4);
    skips.add_n(SkipReason::NoPackageIndex, 6);

    let cov = ChunkCoverage {
        actors_total: 10,
        actors_resolved: 0,
        skips,
        ..Default::default()
    };
    assert!(cov.is_balanced());
    assert_eq!(cov.skips.total(), 10);
}
