//! Package-backed archetype tests — synthetic `.umap` + `.upk` bytes
//! through the real parser, no cooked client tree.
//!
//! `archetype/tests.rs` covers the chain *control flow* against a
//! `fetch` closure. This file covers the half that closure hides: that
//! a real cooked-shaped chunk, a real prefab package and a real
//! [`PackageIndex`] actually line up — the import chain we build from
//! the export table matches the dotted outer path we compute inside the
//! prefab package, at the tagged-property offsets both sides use.
//!
//! The shapes are traced from `Castle-000a0002` + `Em-Props.upk`; see
//! `archetype`'s module doc.

use cimmeria_upk::Package;
use cimmeria_upk_objects::PackageIndex;

use crate::coverage::SkipReason;
use crate::staticmesh::{collect_static_mesh_instances, ArchetypeCache};
use crate::test_support::{
    index_over, prefab_package, scratch_dir, ChunkFixture, PrefabInstanceSpec, PrefabSpec,
    StaticMeshPayload, TemplateMesh,
};

/// `(chunk package, index)` over a scratch dir holding both.
struct Scene {
    chunk: Package,
    index: PackageIndex,
}

fn walk(scene: &Scene) -> crate::staticmesh::ActorWalk {
    collect_static_mesh_instances(
        &scene.chunk,
        Some(&scene.index),
        &mut ArchetypeCache::default(),
    )
}

#[test]
fn a_prefab_stub_resolves_to_the_template_components_mesh() {
    let dir = scratch_dir("arch-one-hop");
    prefab_package(
        &dir,
        &PrefabSpec::local("Fx-Props", "Fx-Lamp_Pf0", "Fx-Lamp_Pf0_Arc1", "Fx-Lamp00"),
        &StaticMeshPayload::unit_triangle(),
    );

    let mut chunk = ChunkFixture::new();
    chunk.add_prefab_instanced_actor(&PrefabInstanceSpec::new(
        "StaticMeshActor",
        [100.0, 200.0, 300.0],
        ("Fx-Props", "Fx-Lamp_Pf0", "Fx-Lamp_Pf0_Arc1"),
    ));
    let path = chunk.write(&dir, "Fix", 0x000a_0002);

    let scene = Scene {
        chunk: Package::open(&path).expect("open chunk"),
        index: index_over(&dir),
    };
    let walk = walk(&scene);

    assert_eq!(walk.actors_total, 1);
    assert_eq!(walk.skips.total(), 0, "{:?}", walk.skips);
    let inst = &walk.instances[0];
    assert_eq!(
        inst.mesh_ref,
        ("Fx-Props".to_string(), "Fx-Lamp00".to_string())
    );
    assert!(inst.via_archetype, "recovered through the archetype chain");
    assert!(inst.from_archetype);
    assert_eq!(inst.transform.location, [100.0, 200.0, 300.0]);
}

#[test]
fn a_template_mesh_in_a_third_package_keeps_that_packages_key() {
    // `EM_Earth_Military`'s tent prefab references
    // `SGW_Weather:DoorwayPrecipitationPlanes`: the key must name the
    // package the *mesh* lives in, not the prefab's.
    let dir = scratch_dir("arch-imported-mesh");
    prefab_package(
        &dir,
        &PrefabSpec {
            mesh: TemplateMesh::Imported("Fx-Weather", "Fx-Plane00"),
            ..PrefabSpec::local("Fx-Tents", "Fx-Tent_Pf0", "Fx-Tent_Pf0_Arc0", "unused")
        },
        &StaticMeshPayload::unit_triangle(),
    );

    let mut chunk = ChunkFixture::new();
    chunk.add_prefab_instanced_actor(&PrefabInstanceSpec::new(
        "StaticMeshActor",
        [0.0; 3],
        ("Fx-Tents", "Fx-Tent_Pf0", "Fx-Tent_Pf0_Arc0"),
    ));
    let path = chunk.write(&dir, "Fix", 0x000a_0003);

    let scene = Scene {
        chunk: Package::open(&path).expect("open chunk"),
        index: index_over(&dir),
    };
    assert_eq!(
        walk(&scene).instances[0].mesh_ref,
        ("Fx-Weather".to_string(), "Fx-Plane00".to_string())
    );
}

#[test]
fn a_two_level_chain_climbs_past_a_stub_template() {
    let dir = scratch_dir("arch-two-level");
    // Base prefab: has the mesh.
    prefab_package(
        &dir,
        &PrefabSpec::local("Fx-Base", "Fx-Base_Pf0", "Fx-Base_Pf0_Arc0", "Fx-Crate00"),
        &StaticMeshPayload::unit_triangle(),
    );
    // Derived prefab: its template component is itself a stub that
    // points at the base one.
    prefab_package(
        &dir,
        &PrefabSpec {
            mesh: TemplateMesh::None,
            component_archetype: Some(("Fx-Base", "Fx-Base_Pf0", "Fx-Base_Pf0_Arc0")),
            ..PrefabSpec::local("Fx-Derived", "Fx-Der_Pf0", "Fx-Der_Pf0_Arc0", "unused")
        },
        &StaticMeshPayload::unit_triangle(),
    );

    let mut chunk = ChunkFixture::new();
    chunk.add_prefab_instanced_actor(&PrefabInstanceSpec::new(
        "StaticMeshActor",
        [0.0; 3],
        ("Fx-Derived", "Fx-Der_Pf0", "Fx-Der_Pf0_Arc0"),
    ));
    let path = chunk.write(&dir, "Fix", 0x000a_0004);

    let scene = Scene {
        chunk: Package::open(&path).expect("open chunk"),
        index: index_over(&dir),
    };
    let walk = walk(&scene);
    assert_eq!(walk.skips.total(), 0, "{:?}", walk.skips);
    assert_eq!(
        walk.instances[0].mesh_ref,
        ("Fx-Base".to_string(), "Fx-Crate00".to_string())
    );
}

/// The regression guard for the `bCollideActors` gate.
///
/// Reverting the gate makes this test fail with one resolved instance
/// instead of one `CollisionDisabled` skip. On Castle that revert is
/// worth 1,570 phantom obstacles — precipitation planes in doorways,
/// icicles, floor signs — and it split the exterior navmesh from one
/// walkable component into three.
#[test]
fn a_template_with_collision_off_is_skipped_not_emitted() {
    let dir = scratch_dir("arch-no-collide");
    prefab_package(
        &dir,
        &PrefabSpec::local(
            "Fx-Weather",
            "Fx-Precip_Pf0",
            "Fx-Precip_Pf0_Arc0",
            "Fx-Plane00",
        )
        .with_collide_actors(false),
        &StaticMeshPayload::unit_triangle(),
    );

    let mut chunk = ChunkFixture::new();
    chunk.add_prefab_instanced_actor(&PrefabInstanceSpec::new(
        "StaticMeshActor",
        [0.0; 3],
        ("Fx-Weather", "Fx-Precip_Pf0", "Fx-Precip_Pf0_Arc0"),
    ));
    let path = chunk.write(&dir, "Fix", 0x000a_0005);

    let scene = Scene {
        chunk: Package::open(&path).expect("open chunk"),
        index: index_over(&dir),
    };
    let walk = walk(&scene);
    assert!(
        walk.instances.is_empty(),
        "a non-colliding prefab must emit no geometry"
    );
    assert_eq!(walk.skips.get(SkipReason::CollisionDisabled), 1);
    assert_eq!(
        walk.actors_total,
        walk.instances.len() as u64 + walk.skips.total(),
        "balance invariant"
    );
}

#[test]
fn an_instance_can_re_enable_collision_its_archetype_turned_off() {
    let dir = scratch_dir("arch-recollide");
    prefab_package(
        &dir,
        &PrefabSpec::local(
            "Fx-Weather",
            "Fx-Precip_Pf0",
            "Fx-Precip_Pf0_Arc0",
            "Fx-Plane00",
        )
        .with_collide_actors(false),
        &StaticMeshPayload::unit_triangle(),
    );

    let mut chunk = ChunkFixture::new();
    chunk.add_prefab_instanced_actor(
        &PrefabInstanceSpec::new(
            "StaticMeshActor",
            [0.0; 3],
            ("Fx-Weather", "Fx-Precip_Pf0", "Fx-Precip_Pf0_Arc0"),
        )
        .with_instance_collide_actors(true),
    );
    let path = chunk.write(&dir, "Fix", 0x000a_0006);

    let scene = Scene {
        chunk: Package::open(&path).expect("open chunk"),
        index: index_over(&dir),
    };
    let walk = walk(&scene);
    assert_eq!(walk.skips.get(SkipReason::CollisionDisabled), 0);
    assert_eq!(walk.instances.len(), 1);
}

#[test]
fn an_instance_mesh_overrides_the_archetypes() {
    // UE3 property inheritance is override-then-fall-through; a cooked
    // component that *does* carry its own `StaticMesh` must never be
    // sent down the archetype path.
    let dir = scratch_dir("arch-override");
    prefab_package(
        &dir,
        &PrefabSpec::local("Fx-Props", "Fx-Lamp_Pf0", "Fx-Lamp_Pf0_Arc1", "Fx-Lamp00"),
        &StaticMeshPayload::unit_triangle(),
    );

    let mut chunk = ChunkFixture::new();
    chunk.add_prefab_instanced_actor(
        &PrefabInstanceSpec::new(
            "StaticMeshActor",
            [0.0; 3],
            ("Fx-Props", "Fx-Lamp_Pf0", "Fx-Lamp_Pf0_Arc1"),
        )
        .with_instance_mesh("Fx-Props", "Fx-Override00"),
    );
    let path = chunk.write(&dir, "Fix", 0x000a_0007);

    let scene = Scene {
        chunk: Package::open(&path).expect("open chunk"),
        index: index_over(&dir),
    };
    let inst = &walk(&scene).instances[0];
    assert_eq!(
        inst.mesh_ref,
        ("Fx-Props".to_string(), "Fx-Override00".to_string())
    );
    assert!(
        !inst.via_archetype,
        "an overriding instance must not be attributed to the archetype"
    );
    assert!(
        inst.from_archetype,
        "it is still a prefab instance, just not one that inherited its mesh"
    );
}

#[test]
fn an_archetype_package_missing_from_the_index_is_its_own_skip_reason() {
    // No `prefab_package` call: the chunk points at a package that was
    // never written, so the index cannot locate it.
    let dir = scratch_dir("arch-missing-pkg");
    let mut chunk = ChunkFixture::new();
    chunk.add_prefab_instanced_actor(&PrefabInstanceSpec::new(
        "StaticMeshActor",
        [0.0; 3],
        ("Fx-Absent", "Fx-Gone_Pf0", "Fx-Gone_Pf0_Arc0"),
    ));
    let path = chunk.write(&dir, "Fix", 0x000a_0008);

    let scene = Scene {
        chunk: Package::open(&path).expect("open chunk"),
        index: index_over(&dir),
    };
    let walk = walk(&scene);
    assert_eq!(walk.skips.get(SkipReason::ArchetypePackageNotFound), 1);
    assert_eq!(walk.actors_total, 1);
    assert!(walk.instances.is_empty());
}

#[test]
fn an_archetype_package_without_the_named_template_is_export_not_found() {
    let dir = scratch_dir("arch-missing-export");
    prefab_package(
        &dir,
        &PrefabSpec::local("Fx-Props", "Fx-Lamp_Pf0", "Fx-Lamp_Pf0_Arc1", "Fx-Lamp00"),
        &StaticMeshPayload::unit_triangle(),
    );

    let mut chunk = ChunkFixture::new();
    // Right package, wrong Arc.
    chunk.add_prefab_instanced_actor(&PrefabInstanceSpec::new(
        "StaticMeshActor",
        [0.0; 3],
        ("Fx-Props", "Fx-Lamp_Pf0", "Fx-Lamp_Pf0_ArcNope"),
    ));
    let path = chunk.write(&dir, "Fix", 0x000a_0009);

    let scene = Scene {
        chunk: Package::open(&path).expect("open chunk"),
        index: index_over(&dir),
    };
    assert_eq!(
        walk(&scene).skips.get(SkipReason::ArchetypeExportNotFound),
        1
    );
}

#[test]
fn rotation_and_scale_are_inherited_from_the_template_actor() {
    let dir = scratch_dir("arch-inherit-transform");
    prefab_package(
        &dir,
        &PrefabSpec::local("Fx-Props", "Fx-Lamp_Pf0", "Fx-Lamp_Pf0_Arc1", "Fx-Lamp00")
            .with_rotation([0, 16384, 0])
            .with_draw_scale_3d([1.0, 1.0, 1.2]),
        &StaticMeshPayload::unit_triangle(),
    );

    let mut chunk = ChunkFixture::new();
    chunk.add_prefab_instanced_actor(&PrefabInstanceSpec::new(
        "StaticMeshActor",
        [500.0, 0.0, 0.0],
        ("Fx-Props", "Fx-Lamp_Pf0", "Fx-Lamp_Pf0_Arc1"),
    ));
    let path = chunk.write(&dir, "Fix", 0x000a_000a);

    let scene = Scene {
        chunk: Package::open(&path).expect("open chunk"),
        index: index_over(&dir),
    };
    let xf = walk(&scene).instances[0].transform;
    assert_eq!(xf.rotation, [0, 16384, 0]);
    assert_eq!(xf.draw_scale_3d, [1.0, 1.0, 1.2]);
    assert_eq!(
        xf.location,
        [500.0, 0.0, 0.0],
        "Location is never inherited — the template's is prefab-local"
    );
}

#[test]
fn without_an_index_a_stub_stays_an_archetype_stub_skip() {
    // Degraded CI mode: the actor is counted, not resolved, and the
    // reason still names the shape rather than a generic failure.
    let dir = scratch_dir("arch-degraded");
    prefab_package(
        &dir,
        &PrefabSpec::local("Fx-Props", "Fx-Lamp_Pf0", "Fx-Lamp_Pf0_Arc1", "Fx-Lamp00"),
        &StaticMeshPayload::unit_triangle(),
    );
    let mut chunk = ChunkFixture::new();
    chunk.add_prefab_instanced_actor(&PrefabInstanceSpec::new(
        "StaticMeshActor",
        [0.0; 3],
        ("Fx-Props", "Fx-Lamp_Pf0", "Fx-Lamp_Pf0_Arc1"),
    ));
    let path = chunk.write(&dir, "Fix", 0x000a_000b);

    let pkg = Package::open(&path).expect("open chunk");
    let walk = collect_static_mesh_instances(&pkg, None, &mut ArchetypeCache::default());
    assert_eq!(walk.skips.get(SkipReason::ArchetypeStubComponent), 1);
    assert!(walk.instances.is_empty());
}

#[test]
fn the_cache_turns_repeated_archetype_paths_into_one_resolution() {
    let dir = scratch_dir("arch-cache");
    prefab_package(
        &dir,
        &PrefabSpec::local("Fx-Props", "Fx-Lamp_Pf0", "Fx-Lamp_Pf0_Arc1", "Fx-Lamp00"),
        &StaticMeshPayload::unit_triangle(),
    );
    let mut chunk = ChunkFixture::new();
    for i in 0..5 {
        chunk.add_prefab_instanced_actor(&PrefabInstanceSpec::new(
            &format!("StaticMeshActor_{i}"),
            [i as f32 * 100.0, 0.0, 0.0],
            ("Fx-Props", "Fx-Lamp_Pf0", "Fx-Lamp_Pf0_Arc1"),
        ));
    }
    let path = chunk.write(&dir, "Fix", 0x000a_000c);

    let pkg = Package::open(&path).expect("open chunk");
    let index = index_over(&dir);
    let mut cache = ArchetypeCache::default();
    let walk = collect_static_mesh_instances(&pkg, Some(&index), &mut cache);
    assert_eq!(walk.instances.len(), 5);
    // Two distinct paths (the actor's and the component's), resolved
    // once each; the other eight lookups are hits.
    let (hits, misses) = cache.stats();
    assert_eq!(misses, 2, "one miss per distinct archetype path");
    assert_eq!(hits, 8);
    assert_eq!(
        walk.prefab_packages_opened, 1,
        "the prefab package is opened once for the whole chunk"
    );
}
