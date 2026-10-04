//! `.gotolocation DebugArea` (world 1300, docs/analysis/debug-area/README.md)
//! through the real `entities/spaces.xml`: the world has no character start,
//! story ring pad or stargate (D-DA4), so naming it alone lands on its
//! lowest-id respawner, 130, the Z1 arrival.

use super::*;
use crate::cell::spawner::RespawnerDef;

const ARRIVAL_130: [f32; 3] = [251.0, 8.0, -962.0];
const RESPAWN_TEST_131: [f32; 3] = [438.0, 10.4, -916.0];

fn respawner(id: i32, world: &str, pos: [f32; 3]) -> RespawnerDef {
    RespawnerDef {
        respawner_id: id,
        world_name: world.to_string(),
        name: format!("respawner {id}"),
        pos,
    }
}

/// `setup_worlds()` plus the shipped world table, the shared DebugArea space
/// (as `cell_spaces.xml` loads it) and the seeded 130/131 rows, pushed in
/// reverse so the rule is visibly "lowest id", not "first row".
fn setup_debug_area() -> (SpaceManager, u32) {
    let (mut mgr, gm, _npc) = setup_worlds();
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../entities/spaces.xml");
    let xml = std::fs::read_to_string(path).expect("entities/spaces.xml readable");
    mgr.parse_spaces_xml(&xml)
        .expect("shipped spaces.xml parses");
    mgr.create_startup_spaces(r#"<Spaces><Space WorldName="DebugArea" /></Spaces>"#)
        .expect("DebugArea startup space");
    mgr.respawners
        .push(respawner(131, "DebugArea", RESPAWN_TEST_131));
    mgr.respawners
        .push(respawner(130, "DebugArea", ARRIVAL_130));
    (mgr, gm)
}

#[tokio::test]
async fn gotolocation_debug_area_alone_lands_on_respawner_130() {
    let (mut mgr, gm) = setup_debug_area();
    let debug_space = mgr.default_space_for_world("DebugArea");
    assert!(debug_space.is_some(), "DebugArea is a shared startup space");

    // Typed the way a GM might type it.
    let t = run("gotolocation", gm, &["debugarea"], None, &mut mgr).await;

    let (subject, world, _space, pos) = t.only_gate_travel().clone();
    assert_eq!(subject, gm);
    assert_eq!(
        world, "DebugArea",
        "the transfer carries the declared spelling"
    );
    assert_eq!(pos, ARRIVAL_130);
    assert!(t.mentions("[respawner]"), "got {:?}", t.feedback);
}
