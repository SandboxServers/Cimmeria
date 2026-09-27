//! NA36 — the walker resolves `KActor` / `FracturedStaticMeshActor`
//! exports unconditionally through the same code path as
//! `StaticMeshActor`, and still ignores classes outside that family.
//! NA40 — `InterpActor` is walked unless the mode is `Off`, and then
//! baked per actor (the decision itself is tested in
//! `interp_actor::evidence_tests`).
//!
//! Package-backed, like `archetype_walk_tests.rs`: a real synthetic
//! `.umap` + `.upk` through the real parser, no cooked client tree.
//! Evidence for *why* `InterpActor` is gated: `docs/engine/
//! navmesh-build-pipeline.md` §11-12 — baking a mover's cooked
//! (usually closed) pose into a `.nav`/`.occ` risks sealing a doorway
//! or blocking sight through one that is actually open at runtime.

use cimmeria_upk::Package;

use crate::interp_actor::InterpActorMode;
use crate::staticmesh::{
    collect_static_mesh_instances, is_mesh_actor_class, ArchetypeCache,
    CLASSIFIED_MESH_ACTOR_CLASSES, MESH_ACTOR_CLASSES,
};
use crate::test_support::{index_over, mesh_package, scratch_dir, ChunkFixture, StaticMeshPayload};

const MODES: [InterpActorMode; 2] = [InterpActorMode::Off, InterpActorMode::Classify];

fn walk_one_actor_of_class(
    class_name: &str,
    interp_actors: InterpActorMode,
) -> crate::staticmesh::ActorWalk {
    let dir = scratch_dir(&format!(
        "mesh-actor-class-{class_name}-{}",
        interp_actors.label()
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
        interp_actors,
    )
}

#[test]
fn mesh_actor_classes_is_the_three_unconditional_classes_interp_actor_is_classified() {
    assert_eq!(
        MESH_ACTOR_CLASSES,
        &["StaticMeshActor", "KActor", "FracturedStaticMeshActor"]
    );
    assert_eq!(CLASSIFIED_MESH_ACTOR_CLASSES, &["InterpActor"]);
}

#[test]
fn k_actor_resolves_its_mesh_like_a_static_mesh_actor_in_every_mode() {
    for mode in MODES {
        let walk = walk_one_actor_of_class("KActor", mode);
        assert_eq!(walk.actors_total, 1, "{mode:?}");
        assert_eq!(walk.skips.total(), 0, "{:?}", walk.skips);
        assert_eq!(
            walk.instances[0].mesh_ref,
            ("Fx-Props".to_string(), "Fx-Lamp00".to_string())
        );
        assert!(
            walk.interp_actors.is_empty(),
            "only InterpActor is classified"
        );
    }
}

#[test]
fn fractured_static_mesh_actor_resolves_its_mesh_like_a_static_mesh_actor_in_every_mode() {
    for mode in MODES {
        let walk = walk_one_actor_of_class("FracturedStaticMeshActor", mode);
        assert_eq!(walk.actors_total, 1, "{mode:?}");
        assert_eq!(walk.skips.total(), 0, "{:?}", walk.skips);
        assert_eq!(
            walk.instances[0].mesh_ref,
            ("Fx-Props".to_string(), "Fx-Lamp00".to_string())
        );
    }
}

/// `Off` is the pre-NA36 extraction: `InterpActor` is completely
/// invisible — not even into a `SkipReason`, exactly the shape of every
/// other non-family class.
#[test]
fn interp_actor_is_invisible_when_the_mode_is_off() {
    let walk = walk_one_actor_of_class("InterpActor", InterpActorMode::Off);
    assert_eq!(
        walk.actors_total, 0,
        "an InterpActor export must not be counted at all when the mode is Off"
    );
    assert!(walk.instances.is_empty());
    assert!(walk.interp_actors.is_empty());
}

/// Under the default mode an `InterpActor` nothing in its chunk
/// references resolves exactly like a `StaticMeshActor`, and its
/// decision is logged.
#[test]
fn an_unreferenced_interp_actor_resolves_its_mesh_under_the_default_mode() {
    let walk = walk_one_actor_of_class("InterpActor", InterpActorMode::default());
    assert_eq!(walk.actors_total, 1);
    assert_eq!(walk.skips.total(), 0, "{:?}", walk.skips);
    assert_eq!(walk.instances.len(), 1);
    assert_eq!(
        walk.instances[0].mesh_ref,
        ("Fx-Props".to_string(), "Fx-Lamp00".to_string())
    );
    assert_eq!(walk.instances[0].transform.location, [12.0, 34.0, 56.0]);
    assert_eq!(walk.interp_actors.len(), 1);
    assert_eq!(walk.interp_actors[0].mesh, "Fx-Lamp00");
    assert!(walk.interp_actors[0].decision.is_included());
}

#[test]
fn is_mesh_actor_class_matches_the_walker_exactly() {
    for class in ["StaticMeshActor", "KActor", "FracturedStaticMeshActor"] {
        for mode in MODES {
            assert!(is_mesh_actor_class(class, mode), "{class} ({mode:?})");
        }
    }
    assert!(!is_mesh_actor_class("InterpActor", InterpActorMode::Off));
    assert!(is_mesh_actor_class(
        "InterpActor",
        InterpActorMode::Classify
    ));
    for mode in MODES {
        assert!(!is_mesh_actor_class("Pawn", mode));
    }
}

#[test]
fn a_class_outside_the_mesh_actor_family_is_still_ignored_in_every_mode() {
    // `Pawn` is not StaticMeshActor-shaped in any cooked SGW chunk; the
    // walker must keep skipping it entirely, exactly as it does today
    // for `Pawn`, `Light`, `PathNode`, etc. — and the InterpActor mode
    // must not accidentally widen the filter to anything else.
    for mode in MODES {
        let walk = walk_one_actor_of_class("Pawn", mode);
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
    assert!(!CLASSIFIED_MESH_ACTOR_CLASSES.contains(&"StaticMeshCollectionActor"));
}
