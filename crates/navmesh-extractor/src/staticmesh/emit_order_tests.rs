//! NA40 — a chunk's triangles reach the soup in a fixed order.
//!
//! A single-mesh NavBuilder build is sensitive to input triangle order,
//! so two extractions of the same chunk must write the same OBJ bytes.
//! The walker used to group instances in a `HashMap`, whose iteration
//! order changes from run to run; it is a `BTreeMap` keyed on the mesh
//! reference now.

use cimmeria_upk::Package;

use crate::interp_actor::InterpActorMode;
use crate::staticmesh::{extract_chunk_from_package, ArchetypeCache};
use crate::test_support::{index_over, mesh_package, scratch_dir, ChunkFixture, StaticMeshPayload};

#[test]
fn instances_reach_the_soup_in_mesh_reference_order_not_hash_order() {
    let dir = scratch_dir("emit-order");
    let mut chunk = ChunkFixture::new();
    // Eight meshes, placed so that sorted-by-name order is the reverse
    // of export order: a hash map lands on exactly that order about
    // once in 40,000 runs.
    let names: Vec<String> = (0..8).map(|i| format!("Mesh{i}")).collect();
    for (i, name) in names.iter().enumerate().rev() {
        let package = format!("P-{name}");
        mesh_package(&dir, &package, name, &StaticMeshPayload::unit_triangle());
        chunk.add_static_mesh_actor(
            &format!("Actor{i}"),
            [1000.0 * i as f32, 0.0, 0.0],
            1.0,
            (&package, name),
        );
    }
    let path = chunk.write(&dir, "Fix", 0x0000_0001);
    let pkg = Package::open(&path).expect("open chunk");
    let extraction = extract_chunk_from_package(
        &pkg,
        Some(&index_over(&dir)),
        &mut ArchetypeCache::default(),
        InterpActorMode::Off,
    );
    assert_eq!(extraction.triangles_emitted, 8);

    // Each unit triangle's first vertex is its actor's location; P-Mesh0
    // sorts first, so x climbs 0, 1000, ..., 7000.
    let first_x: Vec<f32> = extraction
        .soup
        .faces
        .iter()
        .map(|f| extraction.soup.vertices[f[0] as usize - 1][0])
        .collect();
    let expected: Vec<f32> = (0..8).map(|i| 1000.0 * i as f32).collect();
    assert_eq!(first_x, expected);
}
