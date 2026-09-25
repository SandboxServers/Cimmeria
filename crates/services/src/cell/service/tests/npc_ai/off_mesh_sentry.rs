//! A sentry standing where the navmesh has no coverage can still see, and
//! therefore still fights.
//!
//! The 2012 `harset.nav` was fragmented: 9 of the 13 stationary Harset mobs
//! stood further from any polygon than the raycast's projection box reaches.
//! The strict raycast reported that as "no line of sight" against every
//! target, which parks a stationary NPC in `stationary_holds` for the whole
//! fight — the same silent failure the 2026-09-18 Castle playtest hit with a
//! missing mesh, reached here through a partial one. The NA26 rebuild covers
//! every Harset sentry, so the NPC below stands on a real Harset rooftop
//! position (a player reached it; the server rejected it 41 times) that the
//! rebuilt mesh still has nothing within 3 u of.
//!
//! Uses the real `harset.nav`, injected into the space (the production
//! loader keys off a cwd-relative path the test harness does not satisfy).

use cimmeria_entity::navigation::{LineOfSight, NavMesh};

use crate::cell::space_manager::SpaceManager;

/// An uncovered Harset position and a player 8 units +Z of it: the same
/// pair `cimmeria_entity`'s `line_of_sight_tests` pins against the mesh
/// directly.
const SENTRY_UNCOVERED: [f32; 3] = [-148.3, -28.3, 4.8];
const IN_FRONT_OF_SENTRY: [f32; 3] = [-148.3, -28.3, 12.8];

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
        .create_entity(200, "Harset", SENTRY_UNCOVERED, [0.0; 3])
        .unwrap();
    mgr.create_entity(101, "Harset", IN_FRONT_OF_SENTRY, [0.0; 3])
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
        "the sentry is outside harset.nav coverage: the mesh cannot tell, and \
         must say so rather than report a wall"
    );
    assert!(
        mgr.has_line_of_sight(200, 101),
        "an off-mesh sentry must have line of sight to a player standing in \
         front of it; `false` here is the stationary-NPC-never-fires bug"
    );
}
