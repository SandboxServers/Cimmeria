//! NA40 against the shipped client: classify every `InterpActor` in
//! Castle_CellBlock, the one enforcing world, and check the verdicts the
//! committed `.occ` was built from.
//!
//! Castle_CellBlock carries every kind the classifier has to tell apart:
//! 12 doors (ten prison-cell doors and the two castle-entrance leaves,
//! the only `IMF_World` `CloseDoor` Matinee in the client), 20
//! security-camera heads that sweep, and 21 ring-transport rings whose
//! Matinee lifts them and puts them back. A door or a camera baked into
//! the mesh would seal a cell or block sight down a corridor.
//!
//! Needs the cooked client tree and a `PackageIndex` (the rings' mesh
//! only resolves through it); self-skips without either.

use cimmeria_navmesh_extractor::interp_actor::{name_rule, Decision, InterpActorMode};
use cimmeria_navmesh_extractor::staticmesh::extract_chunk;
use cimmeria_navmesh_extractor::umap::enumerate_chunks;

use super::staticmesh_castle_cellblock::{
    castle_cellblock_dir, skip_if_missing, try_load_package_index,
};

#[test]
fn castle_cellblock_bakes_the_rings_and_never_a_door_or_camera() {
    let dir = castle_cellblock_dir();
    if skip_if_missing(
        &dir,
        "castle_cellblock_bakes_the_rings_and_never_a_door_or_camera",
    ) {
        return;
    }
    let Some(index) = try_load_package_index() else {
        eprintln!(
            "Skipping castle_cellblock_bakes_the_rings_and_never_a_door_or_camera — no \
             cached PackageIndex (set CIMMERIA_PACKAGE_INDEX)"
        );
        return;
    };

    let mut records = Vec::new();
    for chunk in enumerate_chunks(&dir).expect("enumerate_chunks") {
        let extraction =
            extract_chunk(&chunk, Some(&index), InterpActorMode::Classify).expect("extract_chunk");
        records.extend(extraction.interp_actors);
    }
    let count = |mesh: &str, included: bool| {
        records
            .iter()
            .filter(|r| r.mesh == mesh && r.decision.is_included() == included)
            .count()
    };

    for r in &records {
        if name_rule(&r.mesh).is_some() {
            assert!(
                !r.decision.is_included(),
                "{} ({}) is a door, gate part or camera head and was baked: {:?}",
                r.actor,
                r.mesh,
                r.decision
            );
        }
        assert!(
            !matches!(r.decision, Decision::Undecided(_)),
            "{} ({}) is undecided: {}",
            r.actor,
            r.mesh,
            r.decision.rule()
        );
    }
    assert_eq!(
        count("EM-Door_Prison00", false),
        10,
        "prison doors left out"
    );
    assert_eq!(count("CA-CastleEntrance_Door00", false), 1);
    assert_eq!(count("CA-CastleEntrance_Door01", false), 1);
    assert_eq!(
        count("EM-SecurityCam01_Top", false),
        20,
        "camera heads left out"
    );
    assert_eq!(count("GLB-RingTransporter00", true), 21, "rings baked");
    assert_eq!(
        records.len(),
        53,
        "every InterpActor with a mesh classified"
    );
}
