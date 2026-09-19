//! Unit tests for the archetype chain — no cooked assets, no disk.
//!
//! The chain walkers take a `fetch` closure rather than a package, so
//! every branch that a real `.upk` cannot easily produce (a cycle, a
//! chain past the depth budget, a package the index has never heard
//! of) is reachable here. The package-backed half of the same logic is
//! covered by `staticmesh/tests.rs` against `test_support` fixtures.

use super::chain::{walk_actor_chain, walk_mesh_chain, ActorProbe, TemplateProbe};
use super::*;
use crate::coverage::SkipReason;

fn path(s: &str) -> Vec<String> {
    s.split('.').map(str::to_string).collect()
}

/// A template that answers with a mesh and nothing else.
fn leaf(pkg: &str, obj: &str) -> TemplateProbe {
    TemplateProbe {
        mesh: Some(Ok((pkg.to_string(), obj.to_string()))),
        ..Default::default()
    }
}

/// A template that is itself a stub and points one hop further up.
fn stub(next: &str) -> TemplateProbe {
    TemplateProbe {
        next: Some(path(next)),
        ..Default::default()
    }
}

// ---------- mesh chain ----------

#[test]
fn one_hop_chain_returns_the_template_mesh() {
    let want = path("Em-Props.EM-ComputerTower00_Pf0.Arc1.StaticMeshComponent0");
    let mut seen: Vec<Vec<String>> = Vec::new();
    let got = walk_mesh_chain(&want, &mut |c| {
        seen.push(c.to_vec());
        Ok(leaf("Em-Props", "EM-ComputerTower00"))
    });
    assert_eq!(
        got,
        Ok(("Em-Props".to_string(), "EM-ComputerTower00".to_string()))
    );
    assert_eq!(seen, vec![want], "the walk must not climb past an answer");
}

#[test]
fn a_stub_template_climbs_to_its_own_archetype() {
    let start = path("Pkg.Prefab.Arc.StaticMeshComponent0");
    let mut hops = 0;
    let got = walk_mesh_chain(&start, &mut |c| {
        hops += 1;
        Ok(if c[0] == "Pkg" {
            stub("Base.BasePrefab.Arc.StaticMeshComponent0")
        } else {
            leaf("Meshes", "Floor01")
        })
    });
    assert_eq!(got, Ok(("Meshes".to_string(), "Floor01".to_string())));
    assert_eq!(hops, 2, "two-level chain must take exactly two reads");
}

#[test]
fn a_cycle_is_reported_rather_than_spun_on() {
    // A -> B -> A. Without the visited set this never returns.
    let a = "A.Pf.Arc.StaticMeshComponent0";
    let b = "B.Pf.Arc.StaticMeshComponent0";
    let got = walk_mesh_chain(&path(a), &mut |c| {
        Ok(if c[0] == "A" { stub(b) } else { stub(a) })
    });
    assert_eq!(got, Err(SkipReason::ArchetypeChainLoop));
}

#[test]
fn a_chain_longer_than_the_budget_is_reported_as_a_loop() {
    // Every hop is a fresh path, so the visited set never fires; only
    // the depth budget stops this.
    let mut n = 0usize;
    let got = walk_mesh_chain(&path("P0.Pf.Arc.Comp"), &mut |_| {
        n += 1;
        Ok(stub(&format!("P{n}.Pf.Arc.Comp")))
    });
    assert_eq!(got, Err(SkipReason::ArchetypeChainLoop));
    assert_eq!(
        n, MAX_ARCHETYPE_DEPTH,
        "the budget, not the visited set, must be what stops this"
    );
}

#[test]
fn a_missing_package_and_a_missing_export_are_different_reasons() {
    // The distinction matters operationally: one means the asset bundle
    // or the index is incomplete, the other means the path we built is
    // wrong. A single catch-all would hide a resolver bug behind a
    // "missing asset" story.
    assert_eq!(
        walk_mesh_chain(&path("Gone.Pf.Arc.Comp"), &mut |_| Err(
            SkipReason::ArchetypePackageNotFound
        )),
        Err(SkipReason::ArchetypePackageNotFound)
    );
    assert_eq!(
        walk_mesh_chain(&path("Here.Pf.Arc.Comp"), &mut |_| Err(
            SkipReason::ArchetypeExportNotFound
        )),
        Err(SkipReason::ArchetypeExportNotFound)
    );
}

#[test]
fn a_template_with_no_mesh_and_nowhere_to_climb_is_archetype_no_mesh() {
    let got = walk_mesh_chain(&path("Pkg.Pf.Arc.Comp"), &mut |_| {
        Ok(TemplateProbe::default())
    });
    assert_eq!(got, Err(SkipReason::ArchetypeNoMesh));
}

#[test]
fn a_collision_disabled_template_wins_over_its_own_mesh() {
    // Ordering matters: the template *has* a usable mesh, and we must
    // still refuse it. Checking the mesh first would emit geometry the
    // player walks through.
    let got = walk_mesh_chain(&path("Pkg.Pf.Arc.Comp"), &mut |_| {
        Ok(TemplateProbe {
            collide_actors: Some(false),
            ..leaf("Meshes", "Floor01")
        })
    });
    assert_eq!(got, Err(SkipReason::CollisionDisabled));
}

#[test]
fn a_chain_with_no_object_under_the_package_is_unrooted() {
    let got = walk_mesh_chain(&path("JustAPackage"), &mut |_| {
        panic!("must not attempt a read")
    });
    assert_eq!(got, Err(SkipReason::ArchetypeUnrooted));
}

#[test]
fn a_null_mesh_ref_on_the_template_is_not_silently_climbed_past() {
    let got = walk_mesh_chain(&path("Pkg.Pf.Arc.Comp"), &mut |_| {
        Ok(TemplateProbe {
            mesh: Some(Err(SkipReason::NullMeshRef)),
            next: Some(path("Other.Pf.Arc.Comp")),
            ..Default::default()
        })
    });
    assert_eq!(got, Err(SkipReason::NullMeshRef));
}

// ---------- actor chain ----------

fn actor(props: ActorArchetypeProps, next: Option<&str>) -> ActorProbe {
    ActorProbe {
        props,
        next: next.map(path),
    }
}

#[test]
fn actor_chain_takes_the_nearest_definition_of_each_field() {
    let got = walk_actor_chain(&path("Pkg.Pf.Arc"), &mut |c| {
        Some(if c[0] == "Pkg" {
            actor(
                ActorArchetypeProps {
                    rotation: Some([0, 16384, 0]),
                    ..Default::default()
                },
                Some("Base.Pf.Arc"),
            )
        } else {
            actor(
                ActorArchetypeProps {
                    // Both levels define Rotation; the nearer one wins.
                    rotation: Some([0, 0, 0]),
                    collide_actors: Some(false),
                    draw_scale: Some(2.0),
                    ..Default::default()
                },
                None,
            )
        })
    });
    assert_eq!(got.rotation, Some([0, 16384, 0]));
    assert_eq!(got.collide_actors, Some(false));
    assert_eq!(got.draw_scale, Some(2.0));
    assert_eq!(got.draw_scale_3d, None);
}

#[test]
fn actor_chain_stops_once_every_field_is_pinned() {
    let mut reads = 0;
    let got = walk_actor_chain(&path("Pkg.Pf.Arc"), &mut |_| {
        reads += 1;
        Some(actor(
            ActorArchetypeProps {
                collide_actors: Some(true),
                rotation: Some([1, 2, 3]),
                draw_scale: Some(1.5),
                draw_scale_3d: Some([1.0, 1.0, 1.2]),
            },
            Some("Base.Pf.Arc"),
        ))
    });
    assert_eq!(reads, 1, "a complete template makes the rest irrelevant");
    assert_eq!(got.draw_scale_3d, Some([1.0, 1.0, 1.2]));
}

#[test]
fn an_unreadable_actor_archetype_inherits_nothing_rather_than_failing() {
    // There is no `SkipReason` for this on purpose: an actor whose
    // archetype cannot be read must behave exactly as it did before
    // this module existed, which is "use your own properties".
    let got = walk_actor_chain(&path("Gone.Pf.Arc"), &mut |_| None);
    assert_eq!(got, ActorArchetypeProps::default());
}

#[test]
fn a_cyclic_actor_chain_terminates_with_what_it_found() {
    let mut reads = 0;
    let got = walk_actor_chain(&path("A.Pf.Arc"), &mut |c| {
        reads += 1;
        Some(actor(
            ActorArchetypeProps {
                collide_actors: if c[0] == "B" { Some(false) } else { None },
                ..Default::default()
            },
            Some(if c[0] == "A" { "B.Pf.Arc" } else { "A.Pf.Arc" }),
        ))
    });
    assert_eq!(reads, 2);
    assert_eq!(got.collide_actors, Some(false));
}

// ---------- property merge semantics ----------

use cimmeria_upk::{PropValue, TaggedProperty};

fn prop(name: &str, value: PropValue) -> TaggedProperty {
    TaggedProperty {
        name: name.to_string(),
        array_index: 0,
        value,
    }
}

#[test]
fn an_actor_that_says_nothing_anywhere_collides() {
    // UE3's `AActor::bCollideActors` default is true and the cooker
    // omits defaults, so silence means solid. Getting this backwards
    // would delete every prop in the map.
    assert!(ActorArchetypeProps::default().collides(&[]));
}

#[test]
fn the_instance_overrides_a_non_colliding_archetype() {
    let arch = ActorArchetypeProps {
        collide_actors: Some(false),
        ..Default::default()
    };
    assert!(!arch.collides(&[]));
    assert!(arch.collides(&[prop("bCollideActors", PropValue::Bool(true))]));
}

#[test]
fn a_non_colliding_instance_overrides_a_colliding_archetype() {
    let arch = ActorArchetypeProps {
        collide_actors: Some(true),
        ..Default::default()
    };
    assert!(!arch.collides(&[prop("bCollideActors", PropValue::Bool(false))]));
}

#[test]
fn merge_transform_prefers_the_instance_and_falls_back_to_the_archetype() {
    let arch = ActorArchetypeProps {
        rotation: Some([0, 16384, 0]),
        draw_scale: Some(3.0),
        draw_scale_3d: Some([1.0, 1.0, 1.2]),
        ..Default::default()
    };
    let instance = vec![
        prop(
            "Location",
            PropValue::Vector {
                x: 10.0,
                y: 20.0,
                z: 30.0,
            },
        ),
        prop("DrawScale", PropValue::Float(2.0)),
    ];
    let xf = arch.merge_transform(&instance);
    assert_eq!(xf.location, [10.0, 20.0, 30.0]);
    assert_eq!(xf.draw_scale, 2.0, "instance wins");
    assert_eq!(xf.rotation, [0, 16384, 0], "inherited");
    assert_eq!(xf.draw_scale_3d, [1.0, 1.0, 1.2], "inherited");
}

#[test]
fn merge_transform_never_inherits_location() {
    // A template actor's Location is its offset *inside the prefab*.
    // Inheriting it would teleport the instance to that offset from the
    // world origin. `ActorArchetypeProps` has no `location` field so
    // this cannot regress silently — the test pins the resulting
    // behaviour, which is "origin", not "the template's offset".
    let arch = ActorArchetypeProps {
        rotation: Some([0, 1, 0]),
        ..Default::default()
    };
    assert_eq!(arch.merge_transform(&[]).location, [0.0, 0.0, 0.0]);
}

#[test]
fn find_bool_ignores_a_same_named_property_of_the_wrong_type() {
    let props = vec![prop("bCollideActors", PropValue::Int(0))];
    assert_eq!(find_bool(&props, "bCollideActors"), None);
    assert!(
        ActorArchetypeProps::default().collides(&props),
        "a malformed tag must not be read as a collision veto"
    );
}
