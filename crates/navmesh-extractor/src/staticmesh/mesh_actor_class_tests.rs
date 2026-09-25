//! NA36 — the walker resolves `KActor` / `FracturedStaticMeshActor`
//! exports unconditionally through the same code path as
//! `StaticMeshActor`, `InterpActor` only when opted in, and still
//! ignores classes outside that family.
//!
//! Package-backed, like `archetype_walk_tests.rs`: a real synthetic
//! `.umap` + `.upk` through the real parser, no cooked client tree.
//! Evidence for *why* `InterpActor` is gated: `docs/engine/
//! navmesh-build-pipeline.md` §11 — in this content `InterpActor` is
//! disproportionately doors, gates, lifts and elevators, and baking a
//! mover's cooked (usually closed) pose into a `.nav`/`.occ` risks
//! sealing a doorway or blocking sight through one that is actually
//! open at runtime.

use cimmeria_upk::Package;

use crate::staticmesh::{
    collect_static_mesh_instances, is_mesh_actor_class, ArchetypeCache, MESH_ACTOR_CLASSES,
    OPT_IN_MESH_ACTOR_CLASSES,
};
use crate::test_support::{index_over, mesh_package, scratch_dir, ChunkFixture, StaticMeshPayload};

fn walk_one_actor_of_class(
    class_name: &str,
    include_interp_actors: bool,
) -> crate::staticmesh::ActorWalk {
    let dir = scratch_dir(&format!(
        "mesh-actor-class-{class_name}-{include_interp_actors}"
    ));
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
    collect_static_mesh_instances(
        &pkg,
        Some(&index),
        &mut ArchetypeCache::default(),
        include_interp_actors,
    )
}

#[test]
fn mesh_actor_classes_is_the_three_unconditional_classes_interp_actor_is_opt_in() {
    assert_eq!(
        MESH_ACTOR_CLASSES,
        &["StaticMeshActor", "KActor", "FracturedStaticMeshActor"]
    );
    assert_eq!(OPT_IN_MESH_ACTOR_CLASSES, &["InterpActor"]);
}

#[test]
fn k_actor_resolves_its_mesh_like_a_static_mesh_actor_regardless_of_the_flag() {
    for include_interp_actors in [false, true] {
        let walk = walk_one_actor_of_class("KActor", include_interp_actors);
        assert_eq!(
            walk.actors_total, 1,
            "include_interp_actors={include_interp_actors}"
        );
        assert_eq!(walk.skips.total(), 0, "{:?}", walk.skips);
        assert_eq!(
            walk.instances[0].mesh_ref,
            ("Fx-Props".to_string(), "Fx-Lamp00".to_string())
        );
    }
}

#[test]
fn fractured_static_mesh_actor_resolves_its_mesh_like_a_static_mesh_actor_regardless_of_the_flag() {
    for include_interp_actors in [false, true] {
        let walk = walk_one_actor_of_class("FracturedStaticMeshActor", include_interp_actors);
        assert_eq!(
            walk.actors_total, 1,
            "include_interp_actors={include_interp_actors}"
        );
        assert_eq!(walk.skips.total(), 0, "{:?}", walk.skips);
        assert_eq!(
            walk.instances[0].mesh_ref,
            ("Fx-Props".to_string(), "Fx-Lamp00".to_string())
        );
    }
}

/// The whole point of NA36's follow-up: `InterpActor` must be
/// completely invisible — not even into a `SkipReason`, exactly the
/// pre-fix shape for every other non-family class — when the caller
/// does not opt in. This is the default the CLI and `ExtractOptions`
/// both ship with.
#[test]
fn interp_actor_is_invisible_by_default() {
    let walk = walk_one_actor_of_class("InterpActor", false);
    assert_eq!(
        walk.actors_total, 0,
        "an InterpActor export must not be counted at all when include_interp_actors is false"
    );
    assert!(walk.instances.is_empty());
}

/// ...and resolves exactly like a `StaticMeshActor` once opted in.
#[test]
fn interp_actor_resolves_its_mesh_when_opted_in() {
    let walk = walk_one_actor_of_class("InterpActor", true);
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
fn is_mesh_actor_class_matches_the_walker_exactly() {
    for class in ["StaticMeshActor", "KActor", "FracturedStaticMeshActor"] {
        assert!(is_mesh_actor_class(class, false), "{class} (flag off)");
        assert!(is_mesh_actor_class(class, true), "{class} (flag on)");
    }
    assert!(!is_mesh_actor_class("InterpActor", false));
    assert!(is_mesh_actor_class("InterpActor", true));
    assert!(!is_mesh_actor_class("Pawn", false));
    assert!(!is_mesh_actor_class("Pawn", true));
}

#[test]
fn a_class_outside_the_mesh_actor_family_is_still_ignored_regardless_of_the_flag() {
    // `Pawn` is not StaticMeshActor-shaped in any cooked SGW chunk; the
    // walker must keep skipping it entirely, exactly as it does today
    // for `Pawn`, `Light`, `PathNode`, etc. — and `include_interp_actors`
    // must not accidentally widen the filter to anything else.
    for include_interp_actors in [false, true] {
        let walk = walk_one_actor_of_class("Pawn", include_interp_actors);
        assert_eq!(walk.actors_total, 0);
        assert!(walk.instances.is_empty());
    }
}

/// `StaticMeshCollectionActor` is a documented, deliberate exclusion
/// (it owns an array of components, not one) — pin that it still reads
/// as untouched by this walker so a future change to
/// `MESH_ACTOR_CLASSES` that silently added it gets caught by whichever
/// test covers the array shape, not silently double-counted here.
#[test]
fn static_mesh_collection_actor_is_not_in_the_mesh_actor_family_yet() {
    assert!(!MESH_ACTOR_CLASSES.contains(&"StaticMeshCollectionActor"));
    assert!(!OPT_IN_MESH_ACTOR_CLASSES.contains(&"StaticMeshCollectionActor"));
}
