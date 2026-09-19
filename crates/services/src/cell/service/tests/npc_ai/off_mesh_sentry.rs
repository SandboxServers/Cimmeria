//! A sentry standing where the navmesh has no coverage can still see, and
//! therefore still fights.
//!
//! `harset.nav` is fragmented: 9 of the 13 stationary Harset mobs stand
//! further from any polygon than the raycast's projection box reaches. The
//! strict raycast reported that as "no line of sight" against every target,
//! which parks a stationary NPC in `stationary_holds` for the whole fight —
//! the same silent failure the 2026-09-18 Castle playtest hit with a missing
//! mesh, reached here through a partial one.
//!
//! Uses the real `harset.nav`, injected into the space (the production
//! loader keys off a cwd-relative path the test harness does not satisfy).

use cimmeria_entity::navigation::{LineOfSight, NavMesh};

use crate::cell::space_manager::SpaceManager;

/// Spawn 232 (template 159, stationary) and a player 8 units in front of it.
const SENTRY_232: [f32; 3] = [4.695_996, -58.654_087, -188.246_61];
const IN_FRONT_OF_232: [f32; 3] = [4.695_996, -58.654_087, -180.246_61];

#[test]
fn off_mesh_sentry_has_line_of_sight_to_the_player_in_front_of_it() {
    let nav_path = std::path::Path::new("../../data/spaces/harset.nav");
    if !nav_path.exists() {
        return; // fixture-less checkout — skip
    }
    let navmesh = NavMesh::load(nav_path).expect("load harset.nav");

    let mut mgr = SpaceManager::new(1);
    mgr.parse_spaces_xml(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Harset" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Harset" /></Spaces>"#,
    )
    .unwrap();
    let space_id = mgr
        .create_entity(200, "Harset", SENTRY_232, [0.0; 3])
        .unwrap();
    mgr.create_entity(101, "Harset", IN_FRONT_OF_232, [0.0; 3])
        .unwrap();

    // Control: with no mesh attached the verdict is Unknown for the boring
    // reason. Attaching the mesh must not turn it into Blocked.
    assert_eq!(mgr.line_of_sight(200, 101), LineOfSight::Unknown);
    mgr.spaces.get_mut(&space_id).unwrap().navmesh = Some(navmesh);
    assert!(
        mgr.space_has_navmesh(200),
        "precondition: the mesh is attached, so the next verdict comes from it"
    );

    assert_eq!(
        mgr.line_of_sight(200, 101),
        LineOfSight::Unknown,
        "spawn 232 is outside harset.nav coverage: the mesh cannot tell, and \
         must say so rather than report a wall"
    );
    assert!(
        mgr.has_line_of_sight(200, 101),
        "an off-mesh sentry must have line of sight to a player standing in \
         front of it; `false` here is the stationary-NPC-never-fires bug"
    );
}
