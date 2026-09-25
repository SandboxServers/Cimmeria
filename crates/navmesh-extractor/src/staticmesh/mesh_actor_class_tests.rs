//! NA36 — the walker resolves `InterpActor` / `KActor` /
//! `FracturedStaticMeshActor` exports through the same code path as
//! `StaticMeshActor`, and still ignores classes outside that family.
//!
//! Package-backed, like `archetype_walk_tests.rs`: a real synthetic
//! `.umap` + `.upk` through the real parser, no cooked client tree.
//! Evidence for *why* this matters: `docs/reverse-engineering/findings/
//! navmesh-extractor-mesh-actor-gap.md` — 31 `InterpActor` exports in
//! the shipped `Harset` chunks carried real collision geometry under
//! raised platforms that a `StaticMeshActor`-only class filter dropped
//! entirely (not even into a `SkipReason` — the walker's `for` loop
//! never visited them).

use cimmeria_upk::Package;

use crate::staticmesh::{collect_static_mesh_instances, ArchetypeCache, MESH_ACTOR_CLASSES};
use crate::test_support::{index_over, mesh_package, scratch_dir, ChunkFixture, StaticMeshPayload};

fn walk_one_actor_of_class(class_name: &str) -> crate::staticmesh::ActorWalk {
    let dir = scratch_dir(&format!("mesh-actor-class-{class_name}"));
    mesh_package(
        &dir,
        "Fx-Props",
        "Fx-Lamp00",
        &StaticMeshPayload::unit_triangle(),
    );

    let mut chunk = ChunkFixture::new();
    chunk.add_mesh_actor_of_class(
        class_name,
        "TestActor",
        [12.0, 34.0, 56.0],
        1.0,
        ("Fx-Props", "Fx-Lamp00"),
    );
    let path = chunk.write(&dir, "Fix", 0x0000_0001);

    let pkg = Package::open(&path).expect("open chunk");
    let index = index_over(&dir);
    collect_static_mesh_instances(&pkg, Some(&index), &mut ArchetypeCache::default())
}

#[test]
fn mesh_actor_classes_lists_the_four_static_mesh_actor_shaped_classes() {
    assert_eq!(
        MESH_ACTOR_CLASSES,
        &[
            "StaticMeshActor",
            "InterpActor",
            "KActor",
            "FracturedStaticMeshActor"
        ]
    );
}

#[test]
fn interp_actor_resolves_its_mesh_like_a_static_mesh_actor() {
    let walk = walk_one_actor_of_class("InterpActor");
    assert_eq!(walk.actors_total, 1);
    assert_eq!(walk.skips.total(), 0, "{:?}", walk.skips);
    assert_eq!(walk.instances.len(), 1);
    assert_eq!(
        walk.instances[0].mesh_ref,
        ("Fx-Props".to_string(), "Fx-Lamp00".to_string())
    );
    assert_eq!(walk.instances[0].transform.location, [12.0, 34.0, 56.0]);
}

#[test]
fn k_actor_resolves_its_mesh_like_a_static_mesh_actor() {
    let walk = walk_one_actor_of_class("KActor");
    assert_eq!(walk.actors_total, 1);
    assert_eq!(walk.skips.total(), 0, "{:?}", walk.skips);
    assert_eq!(
        walk.instances[0].mesh_ref,
        ("Fx-Props".to_string(), "Fx-Lamp00".to_string())
    );
}

#[test]
fn fractured_static_mesh_actor_resolves_its_mesh_like_a_static_mesh_actor() {
    let walk = walk_one_actor_of_class("FracturedStaticMeshActor");
    assert_eq!(walk.actors_total, 1);
    assert_eq!(walk.skips.total(), 0, "{:?}", walk.skips);
    assert_eq!(
        walk.instances[0].mesh_ref,
        ("Fx-Props".to_string(), "Fx-Lamp00".to_string())
    );
}

/// The regression this whole file exists to guard: before NA36, the
/// walker's `for` loop tested `pkg.export_class_name(export) !=
/// "StaticMeshActor"` and `continue`d — so an `InterpActor` export
/// never even incremented `actors_total`, let alone produced an
/// instance. Reverting `MESH_ACTOR_CLASSES` back to a single literal
/// `"StaticMeshActor"` string (or an exact-match check against it)
/// must make this fail.
#[test]
fn interp_actor_is_not_silently_invisible_to_the_walker() {
    let walk = walk_one_actor_of_class("InterpActor");
    assert_eq!(
        walk.actors_total, 1,
        "an InterpActor export must be counted, not skipped over invisibly"
    );
    assert!(
        !walk.instances.is_empty(),
        "an InterpActor with a resolvable StaticMeshComponent must produce a triangle-soup instance"
    );
}

#[test]
fn a_class_outside_the_mesh_actor_family_is_still_ignored() {
    // `Pawn` is not StaticMeshActor-shaped in any cooked SGW chunk; the
    // walker must keep skipping it entirely, exactly as it does today
    // for `Pawn`, `Light`, `PathNode`, etc.
    let walk = walk_one_actor_of_class("Pawn");
    assert_eq!(walk.actors_total, 0);
    assert!(walk.instances.is_empty());
}

/// `StaticMeshCollectionActor` is a documented, deliberate exclusion
/// (it owns an array of components, not one) — pin that it still reads
/// as untouched by this walker so a future change to
/// `MESH_ACTOR_CLASSES` that silently added it gets caught by whichever
/// test covers the array shape, not silently double-counted here.
#[test]
fn static_mesh_collection_actor_is_not_in_the_mesh_actor_family_yet() {
    assert!(!MESH_ACTOR_CLASSES.contains(&"StaticMeshCollectionActor"));
}
