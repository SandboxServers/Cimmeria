//! `MotionEvidence` and the walker's per-actor decision against a real
//! synthetic `.umap`: `SeqAct_Interp` → `SeqVar_Object` → actor, with
//! the tracks on an `InterpData`, through the real package reader.

use cimmeria_upk::Package;

use super::*;
use crate::coverage::SkipReason;
use crate::staticmesh::{collect_static_mesh_instances, ArchetypeCache, StaticMeshInstance};
use crate::test_support::{
    index_over, mesh_package, scratch_dir, ChunkFixture, GroupSpec, MoveKeys, StaticMeshPayload,
};

const RING: [[f32; 3]; 3] = [[0.0; 3], [0.0, 0.0, 330.0], [0.0; 3]];
const OPEN: [[f32; 3]; 2] = [[0.0; 3], [0.0, 222.0, 0.0]];

/// Write `chunk`, plus one mesh package holding every mesh name the
/// tests use, and walk it.
fn walk(tag: &str, chunk: &ChunkFixture, mode: InterpActorMode) -> crate::staticmesh::ActorWalk {
    let dir = scratch_dir(&format!("interp-evidence-{tag}"));
    for mesh in ["GLB-RingTransporter00", "SGC_Door03", "LUS-MetalBox00"] {
        // One package per mesh: `mesh_package` writes a whole file.
        mesh_package(
            &dir,
            &format!("P-{mesh}"),
            mesh,
            &StaticMeshPayload::unit_triangle(),
        );
    }
    let path = chunk.write(&dir, "Fix", 0x0000_0001);
    let pkg = Package::open(&path).expect("open chunk");
    collect_static_mesh_instances(
        &pkg,
        Some(&index_over(&dir)),
        &mut ArchetypeCache::default(),
        mode,
    )
}

fn interp(chunk: &mut ChunkFixture, name: &str, mesh: &str) -> i32 {
    let package = format!("P-{mesh}");
    chunk.add_mesh_actor_of_class(
        "InterpActor",
        name,
        [10.0, 20.0, 30.0],
        1.0,
        (&package, mesh),
    )
}

fn baked(walk: &crate::staticmesh::ActorWalk) -> Vec<&str> {
    walk.instances
        .iter()
        .map(|i: &StaticMeshInstance| i.actor_name.as_str())
        .collect()
}

fn verdict_of<'a>(walk: &'a crate::staticmesh::ActorWalk, actor: &str) -> &'a Decision {
    &walk
        .interp_actors
        .iter()
        .find(|r| r.actor == actor)
        .unwrap_or_else(|| panic!("no record for {actor}: {:?}", walk.interp_actors))
        .decision
}

/// Four InterpActors, one per outcome, in one chunk:
/// - `Ring`: a Matinee lifts it 330 cm and puts it back → baked;
/// - `Hatch`: a Matinee slides it 222 cm and leaves it → excluded;
/// - `Door`: nothing references it, but it is a door → excluded;
/// - `Crate`: a `SeqAct_Toggle` targets it → undecided.
fn four_outcomes() -> ChunkFixture {
    let mut chunk = ChunkFixture::new();
    let ring = interp(&mut chunk, "Ring", "GLB-RingTransporter00");
    let hatch = interp(&mut chunk, "Hatch", "LUS-MetalBox00");
    interp(&mut chunk, "Door", "SGC_Door03");
    let crate_ = interp(&mut chunk, "Crate", "LUS-MetalBox00");
    let ring_var = chunk.add_seqvar_object(ring);
    let hatch_var = chunk.add_seqvar_object(hatch);
    let crate_var = chunk.add_seqvar_object(crate_);
    chunk.add_matinee(&[
        GroupSpec {
            name: "ring1",
            seqvars: vec![ring_var],
            moves: vec![MoveKeys::relative(&RING)],
            other_tracks: vec!["InterpTrackEvent"],
        },
        GroupSpec {
            name: "OpenHatch",
            seqvars: vec![hatch_var],
            moves: vec![MoveKeys::relative(&OPEN)],
            other_tracks: vec![],
        },
    ]);
    chunk.add_kismet_op("SeqAct_Toggle", "Target", &[crate_var]);
    chunk
}

#[test]
fn evidence_follows_seq_act_interp_to_the_group_tracks_of_each_actor() {
    let dir = scratch_dir("interp-evidence-direct");
    let path = four_outcomes().write(&dir, "Fix", 0x0000_0001);
    let pkg = Package::open(&path).expect("open chunk");
    let evidence = MotionEvidence::collect(&pkg);
    let index_of = |name: &str| {
        pkg.exports
            .iter()
            .position(|e| e.object_name == name)
            .unwrap()
    };

    let ring = evidence.for_export(index_of("Ring"));
    assert_eq!(ring.matinee.len(), 1);
    assert_eq!(ring.matinee[0].group, "ring1");
    let tracks = ring.matinee[0].tracks.as_ref().expect("group tracks read");
    assert_eq!(tracks.other, vec!["InterpTrackEvent".to_string()]);
    assert_eq!(tracks.moves.len(), 1);
    assert_eq!(tracks.moves[0].frame, MoveFrame::RelativeToInitial);
    assert_eq!(tracks.moves[0].positions, RING.to_vec());

    let hatch = evidence.for_export(index_of("Hatch"));
    assert_eq!(hatch.matinee[0].group, "OpenHatch");
    assert_eq!(
        hatch.matinee[0].tracks.as_ref().unwrap().moves[0].positions,
        OPEN.to_vec()
    );

    assert!(evidence.for_export(index_of("Door")).is_unreferenced());
    assert_eq!(
        evidence.for_export(index_of("Crate")).other_refs,
        vec!["SeqAct_Toggle[Target]".to_string()]
    );
}

#[test]
fn classify_mode_bakes_only_the_actor_whose_matinee_returns_to_rest() {
    let walk = walk("classify", &four_outcomes(), InterpActorMode::Classify);

    assert_eq!(baked(&walk), vec!["Ring"]);
    assert!(matches!(
        verdict_of(&walk, "Ring"),
        Decision::Include(IncludeRule::RestAnchoredMatinee { .. })
    ));
    assert!(matches!(
        verdict_of(&walk, "Hatch"),
        Decision::Exclude(ExcludeRule::MatineeLeavesRest { .. })
    ));
    assert_eq!(
        verdict_of(&walk, "Door"),
        &Decision::Exclude(ExcludeRule::Name(NameRule::Door))
    );
    assert!(matches!(
        verdict_of(&walk, "Crate"),
        Decision::Undecided(UndecidedReason::KismetReference(_))
    ));

    // Every walked actor is accounted for: one baked, two excluded, one
    // undecided — the coverage balance invariant, per reason.
    assert_eq!(walk.actors_total, 4);
    assert_eq!(walk.skips.get(SkipReason::InterpActorExcluded), 2);
    assert_eq!(walk.skips.get(SkipReason::InterpActorUndecided), 1);
    assert_eq!(
        walk.instances.len() as u64 + walk.skips.total(),
        walk.actors_total
    );
}

#[test]
fn off_mode_leaves_every_interp_actor_invisible() {
    // The pre-NA36 extraction, which every map not rebuilt since was
    // built with: not walked, not tallied, no decisions.
    let walk = walk("off", &four_outcomes(), InterpActorMode::Off);
    assert_eq!(walk.actors_total, 0);
    assert!(walk.instances.is_empty());
    assert!(walk.interp_actors.is_empty());
}

#[test]
fn a_door_driven_by_a_rest_anchored_matinee_is_still_not_baked() {
    // The door rule outranks the evidence: a door that swings shut again
    // in one sequence still reads "returns to rest".
    let mut chunk = ChunkFixture::new();
    let door = interp(&mut chunk, "Door", "SGC_Door03");
    let var = chunk.add_seqvar_object(door);
    chunk.add_matinee(&[GroupSpec {
        name: "Swing",
        seqvars: vec![var],
        moves: vec![MoveKeys::relative(&RING)],
        other_tracks: vec![],
    }]);
    let walk = walk("door-rest-anchored", &chunk, InterpActorMode::Classify);
    assert!(baked(&walk).is_empty());
    assert_eq!(
        verdict_of(&walk, "Door"),
        &Decision::Exclude(ExcludeRule::Name(NameRule::Door))
    );
}

#[test]
fn a_camera_style_sweep_is_excluded_through_the_real_reader() {
    let mut chunk = ChunkFixture::new();
    let cam = interp(&mut chunk, "Dish", "LUS-MetalBox00");
    let var = chunk.add_seqvar_object(cam);
    chunk.add_matinee(&[GroupSpec {
        name: "Sweep",
        seqvars: vec![var],
        moves: vec![MoveKeys::turning(&[[0.0; 3], [0.0, 0.0, -45.0], [0.0; 3]])],
        other_tracks: vec![],
    }]);
    let walk = walk("sweep", &chunk, InterpActorMode::Classify);
    assert!(baked(&walk).is_empty());
    assert!(matches!(
        verdict_of(&walk, "Dish"),
        Decision::Exclude(ExcludeRule::MatineeRotates { .. })
    ));
}

#[test]
fn an_event_originator_reference_alone_still_bakes_the_actor() {
    let mut chunk = ChunkFixture::new();
    let lamp = interp(&mut chunk, "Lamp", "LUS-MetalBox00");
    chunk.add_kismet_object_ref("SeqEvent_Touch", "Originator", lamp);
    let walk = walk("originator", &chunk, InterpActorMode::Classify);
    assert_eq!(baked(&walk), vec!["Lamp"]);
    assert_eq!(
        verdict_of(&walk, "Lamp"),
        &Decision::Include(IncludeRule::BenignReferencesOnly)
    );
}
