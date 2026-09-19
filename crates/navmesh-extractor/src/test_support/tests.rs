//! Round-trip proofs for the fixture builder itself.
//!
//! A fixture builder that is wrong in the same direction as the parser
//! would let every test above it pass while the real cooked data still
//! failed. Each test here therefore asserts against the **real**
//! reader — `cimmeria_upk::Package::open` on bytes that went to disk —
//! and pins a value the encoder and the decoder derive independently
//! (a byte count from the module docs, a decoded vertex position, a
//! resolved class name).

use std::path::Path;

use cimmeria_upk::{Package, PropValue};
use cimmeria_upk_objects::{deserialize_model, deserialize_static_mesh, deserialize_terrain};

use super::*;
use crate::bsp::{self, BspOptions};
use crate::geometry::TriangleSoup;

fn open(path: &Path) -> Package {
    Package::open(path).expect("fixture package must parse with the real reader")
}

#[test]
fn a_built_package_round_trips_through_the_real_reader() {
    let dir = scratch_dir("fixture-roundtrip");
    let mut b = PackageBuilder::new();
    let class = b.class_ref("Terrain");
    let export = b.add_export(class, 0, "MyTerrain");
    b.set_payload(export, vec![1, 2, 3, 4, 5]);
    let path = dir.join("Round-00000001.umap");
    b.write_to(&path).unwrap();

    let pkg = open(&path);
    assert_eq!(pkg.header.epic_version, package_bytes::EPIC_VERSION);
    assert!(!pkg.header.is_compressed());
    assert_eq!(pkg.exports.len(), 1);
    assert_eq!(pkg.exports[0].object_name, "MyTerrain");
    assert_eq!(pkg.export_class_name(&pkg.exports[0]), "Terrain");
    // The export body must come back byte-for-byte: a wrong
    // serial_offset is the failure mode that silently hands every
    // decoder the neighbouring export's bytes.
    assert_eq!(
        pkg.read_export_data(&pkg.exports[0]).unwrap(),
        vec![1, 2, 3, 4, 5]
    );
}

#[test]
fn export_bodies_do_not_overlap() {
    let dir = scratch_dir("fixture-offsets");
    let mut b = PackageBuilder::new();
    let class = b.class_ref("Model");
    let first = b.add_export(class, 0, "First");
    let second = b.add_export(class, 0, "Second");
    b.set_payload(first, vec![0xAA; 7]);
    b.set_payload(second, vec![0xBB; 11]);
    let path = dir.join("Off-00000002.umap");
    b.write_to(&path).unwrap();

    let pkg = open(&path);
    assert_eq!(
        pkg.read_export_data(&pkg.exports[0]).unwrap(),
        vec![0xAA; 7]
    );
    assert_eq!(
        pkg.read_export_data(&pkg.exports[1]).unwrap(),
        vec![0xBB; 11]
    );
    assert_eq!(
        pkg.exports[1].serial_offset - pkg.exports[0].serial_offset,
        7,
        "second body must start exactly where the first ends"
    );
}

#[test]
fn tagged_properties_decode_to_the_values_they_were_built_from() {
    let dir = scratch_dir("fixture-props");
    let mut b = PackageBuilder::new();
    let class = b.class_ref("Brush");
    let export = b.add_export(class, 0, "Brush_0");
    let mut body = vec![0u8; ACTOR_PROPS_OFFSET];
    let mut props = b.props();
    props
        .placement([10.0, -20.0, 30.5], [0, 16384, 0], 2.5, [1.0, 2.0, 3.0])
        .vector("PrePivot", [1.0, 1.0, 1.0])
        .int("Tag", 7)
        .object("Ref", -3)
        .boolean("bHidden", true);
    body.extend_from_slice(&props.finish());
    b.set_payload(export, body);
    let path = dir.join("Props-00000003.umap");
    b.write_to(&path).unwrap();

    let pkg = open(&path);
    let data = pkg.read_export_data(&pkg.exports[0]).unwrap();
    let props = cimmeria_upk::parse_tagged_properties(&data, ACTOR_PROPS_OFFSET, &pkg.names);
    let names: Vec<&str> = props.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(
        names,
        vec![
            "Location",
            "Rotation",
            "DrawScale",
            "DrawScale3D",
            "PrePivot",
            "Tag",
            "Ref",
            "bHidden"
        ]
    );

    let xf = crate::staticmesh::transform_from_actor_props(&props);
    assert_eq!(xf.location, [10.0, -20.0, 30.5]);
    assert_eq!(xf.rotation, [0, 16384, 0]);
    assert_eq!(xf.draw_scale, 2.5);
    assert_eq!(xf.draw_scale_3d, [1.0, 2.0, 3.0]);
    assert!(matches!(props[6].value, PropValue::Object(-3)));
    assert!(matches!(props[7].value, PropValue::Bool(true)));
}

#[test]
fn an_empty_model_encodes_to_the_documented_108_bytes() {
    // The module docs derive 108 from the field list (4 NetIndex + 8
    // `None` + 28 Bounds + 17 four-byte scalars). Pinning it here means
    // a dropped or duplicated field in the encoder shows up as
    // arithmetic, not as a decode that happens to work.
    let bytes = ModelPayload::empty().encode();
    assert_eq!(bytes.len(), 108);
    let names = vec![cimmeria_upk::NameEntry {
        name: "None".to_string(),
        flags: 0,
    }];
    let model = deserialize_model(&bytes, &names).expect("empty model decodes");
    assert!(model.nodes.is_empty());
}

#[test]
fn a_model_quad_decodes_back_to_its_corner_positions() {
    let payload = ModelPayload::horizontal_quad(0.0, 100.0, 0.0, 100.0, 50.0, 0, [0.0, 0.0, 1.0]);
    let bytes = payload.encode();
    let names = vec![cimmeria_upk::NameEntry {
        name: "None".to_string(),
        flags: 0,
    }];
    let model = deserialize_model(&bytes, &names).expect("quad model decodes");
    assert_eq!(model.nodes.len(), 1);
    assert_eq!(model.surfs.len(), 1);
    assert_eq!(model.surf_normal(0), Some([0.0, 0.0, 1.0]));

    let t = model.triangulate(cimmeria_upk_objects::CollisionFilter::default());
    assert_eq!(t.triangles.len(), 2, "a quad fans into two triangles");
    assert_eq!(t.triangles[0][0], [0.0, 0.0, 50.0]);
    assert_eq!(t.triangles[0][1], [100.0, 0.0, 50.0]);
    assert_eq!(t.triangles[0][2], [100.0, 100.0, 50.0]);
}

#[test]
fn a_terrain_payload_decodes_to_the_grid_it_was_built_from() {
    let mut b = PackageBuilder::new();
    let payload = TerrainPayload::flat(2, 2)
        .at([800.0, -200.0, 35.0])
        .with_hole(0, 0)
        .raise(2, 2, 128);
    let body = payload.encode(&mut b);
    // Encode into a package so the name table the decoder sees is the
    // one the encoder interned into.
    let dir = scratch_dir("fixture-terrain");
    let class = b.class_ref("Terrain");
    let export = b.add_export(class, 0, "Terrain_0");
    b.set_payload(export, body);
    let path = dir.join("Terr-00000004.umap");
    b.write_to(&path).unwrap();

    let pkg = open(&path);
    let data = pkg.read_export_data(&pkg.exports[0]).unwrap();
    let terrain = deserialize_terrain(&data, &pkg.names).expect("terrain decodes");
    assert_eq!((terrain.num_patches_x, terrain.num_patches_y), (2, 2));
    assert_eq!(terrain.location, [800.0, -200.0, 35.0]);
    assert_eq!(terrain.draw_scale_3d, [100.0, 100.0, 100.0]);
    assert_eq!(terrain.lighting_trailer_bytes, 0);
    assert!(!terrain.quad_visible(0, 0), "quad (0,0) was holed");
    assert!(terrain.quad_visible(1, 1));
    assert_eq!(terrain.local_vertex(2, 2).unwrap()[2], 1.0);
}

#[test]
fn a_static_mesh_payload_decodes_to_its_collision_triangle() {
    let bytes = StaticMeshPayload::unit_triangle().encode();
    let names = vec![cimmeria_upk::NameEntry {
        name: "None".to_string(),
        flags: 0,
    }];
    let mesh = deserialize_static_mesh(&bytes, &names).expect("static mesh decodes");
    assert_eq!(mesh.internal_version, 15);
    let tris = mesh.collision_triangles();
    assert_eq!(tris.len(), 1);
    assert_eq!(
        tris[0],
        [[0.0, 0.0, 0.0], [100.0, 0.0, 0.0], [0.0, 100.0, 0.0]]
    );
}

#[test]
fn a_chunk_fixture_presents_the_owner_graph_the_bsp_walker_classifies_on() {
    let dir = scratch_dir("fixture-chunk");
    let quad = ModelPayload::horizontal_quad(0.0, 100.0, 0.0, 100.0, 0.0, 0, [0.0, 0.0, 1.0]);
    let mut chunk = ChunkFixture::new();
    chunk.add_level_model(&quad);
    chunk.add_owned_model(
        "TriggerVolume",
        "TrigVol_0",
        [0.0; 3],
        [0; 3],
        1.0,
        [1.0; 3],
        [0.0; 3],
        &quad,
    );
    chunk.add_builder_brush_model(&ModelPayload::empty());
    let path = chunk.write(&dir, "Fix", 0x000a_0002);
    assert_eq!(path.file_name().unwrap(), "Fix-000a0002.umap");

    let pkg = open(&path);
    let (instances, stats) = bsp::collect_bsp_models(&pkg);
    assert_eq!(stats.models_total, 3);
    assert_eq!(stats.level_models, 1);
    assert_eq!(stats.actor_models_excluded, 1, "the TriggerVolume");
    assert_eq!(stats.builder_brush_models, 1);
    assert_eq!(stats.models_failed, 0);
    assert_eq!(instances.len(), 1, "only the level model emits");
    assert!(instances[0].is_level_model);

    let mut soup = TriangleSoup::new(None);
    let stats = bsp::collect_bsp_triangles(&pkg, &mut soup, BspOptions::default());
    assert_eq!(stats.triangles_emitted, 2);
    assert_eq!(soup.triangle_count(), 2);
}

#[test]
fn a_static_mesh_actor_resolves_through_a_real_package_index() {
    // The whole cross-package chain, with no cooked client tree:
    // actor -> component -> import -> package import -> .upk on disk ->
    // PackageIndex -> decoded mesh -> transformed triangle.
    let dir = scratch_dir("fixture-smactor");
    let content = dir.join("Content");
    std::fs::create_dir_all(&content).unwrap();
    mesh_package(
        &content,
        "Fx-Props",
        "Fx-Floor01",
        &StaticMeshPayload::unit_triangle(),
    );
    let index = index_over(&content);
    assert!(index.find("Fx-Props", "Fx-Floor01").is_some());

    let maps = dir.join("Maps");
    std::fs::create_dir_all(&maps).unwrap();
    let mut chunk = ChunkFixture::new();
    chunk.add_static_mesh_actor(
        "Floor_0",
        [1000.0, 2000.0, 3000.0],
        2.0,
        ("Fx-Props", "Fx-Floor01"),
    );
    let path = chunk.write(&maps, "Fix", 0x0000_0005);

    let pkg = open(&path);
    let extraction = crate::staticmesh::extract_chunk_from_package(
        &pkg,
        Some(&index),
        &mut crate::staticmesh::ArchetypeCache::default(),
    );
    assert_eq!(extraction.actors_total, 1);
    assert_eq!(extraction.actors_resolved, 1, "{:?}", extraction.skips);
    assert_eq!(extraction.triangles_emitted, 1);
    // DrawScale 2 on a 100 cm mesh, translated by Location.
    assert!(extraction.soup.vertices.contains(&[1000.0, 2000.0, 3000.0]));
    assert!(extraction.soup.vertices.contains(&[1200.0, 2000.0, 3000.0]));
    assert!(extraction.soup.vertices.contains(&[1000.0, 2200.0, 3000.0]));
}

#[test]
fn an_archetype_stub_actor_is_counted_but_not_resolved() {
    let dir = scratch_dir("fixture-archetype");
    let mut chunk = ChunkFixture::new();
    chunk.add_archetype_stub_actor("Lamp_0", "Fx-Props");
    let path = chunk.write(&dir, "Fix", 0x0000_0006);

    let pkg = open(&path);
    // No index: the archetype chain cannot be followed, so the stub
    // stays a stub — which is what this fixture is asserting about.
    let walk = crate::staticmesh::collect_static_mesh_instances(
        &pkg,
        None,
        &mut crate::staticmesh::ArchetypeCache::default(),
    );
    assert_eq!(walk.actors_total, 1);
    assert_eq!(walk.archetype_actors, 1, "export Archetype must be set");
    assert!(walk.instances.is_empty());
    assert_eq!(
        walk.skips
            .get(crate::coverage::SkipReason::ArchetypeStubComponent),
        1
    );
}
