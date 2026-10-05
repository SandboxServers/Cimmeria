//! Every gate a GM at the Debug Area dial hub is offered, against the real
//! seed, the real `entities/*spaces.xml` and the real navmeshes (DA-07).
//!
//! The arrival rules (`cell::arrival`) only refuse a destination whose world
//! *enforces* its navmesh, and every gated world is `advisory`, so in
//! production none of these dials is refused for `arrival_unrecoverable`.
//! This test applies the rules as if each world enforced its mesh, so the
//! gates a GM would land on off-mesh are named, and pinned: fixing one, or
//! breaking another, fails here and the list in
//! `docs/gameplay/gate-travel.md#debug-area-dial-out` moves with it.

use super::super::dial_hub::top_up_gm_dial_hub;
use super::*;
use crate::cell::arrival::resolve_arrival_with;
use crate::cell::spawner::{load_respawners, load_stargates};
use crate::test_support::require_db_or_skip;
use cimmeria_entity::navigation::NavMesh;

const HUB: i32 = 29;
const GM: u32 = 2;

/// Gates on worlds with a `resources.worlds` row but no space this server
/// can create (not in `cell_spaces.xml`, not instanced in `spaces.xml`):
/// CombatSim, Ihpet (E1), Hebridan, Dakara E2/E3, Pen-Lai, Beta Site E2,
/// SGC W2, Yotunheim, Vitrus, Meridian, Egypt, Pertho, Asgard High Council.
const EXPECTED_UNENTERABLE: [i32; 14] = [1, 9, 11, 12, 13, 14, 16, 17, 18, 19, 21, 24, 26, 28];

/// Gates on enterable worlds the hub still leaves out (`HUB_EXCLUDED_GATES`):
/// 22 `Men'fa (SGU)`, whose row is ~192 m under `menfa_light.nav`'s
/// playable surface.
const EXPECTED_EXCLUDED: [i32; 1] = [22];

/// Offered gates whose arrival is off their world's mesh with no respawner
/// to fall back on (measured 2026-10-04 on the NA28 meshes):
/// - 27 `SGC W1` / SGC_W1: nearest mesh 3.4 m away, 1.3 m below.
///
/// SGC_W1 is `advisory`, so the dial is accepted and the GM lands at the
/// gate row, a little above the floor beside it.
const EXPECTED_OFF_MESH: [i32; 1] = [27];

fn repo_file(rel: &str) -> String {
    let path = format!("{}/../../{rel}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"))
}

#[tokio::test]
async fn live_db_every_gate_the_debug_area_offers_is_enterable_and_standable_or_known() {
    let pool = require_db_or_skip!();

    let mut mgr = SpaceManager::new(1);
    mgr.parse_spaces_xml(&repo_file("entities/spaces.xml"))
        .expect("spaces.xml parses");
    mgr.create_startup_spaces(&repo_file("entities/cell_spaces.xml"))
        .expect("cell_spaces.xml creates its spaces");
    mgr.stargates = load_stargates(&pool).await.expect("load_stargates");
    let respawners = load_respawners(&pool).await.expect("load_respawners");
    mgr.create_entity(1, "DebugArea", [251.0, 8.0, -962.0], [0.0; 3])
        .expect("DebugArea is a startup space (cell_spaces.xml)");
    mgr.get_entity_mut(1).unwrap().access_level = GM;

    let (tx, _rx) = tokio::sync::mpsc::channel(64);
    let top = top_up_gm_dial_hub(1, HUB, &tx, &mut mgr)
        .await
        .expect("gate 29 is a hub and the caller a GM");

    assert_eq!(
        top.unenterable, EXPECTED_UNENTERABLE,
        "the gates left out of the Debug Area DHD changed — a world became \
         loadable or stopped being so; update this list and gate-travel.md"
    );
    assert_eq!(top.excluded, EXPECTED_EXCLUDED);
    assert!(!top.granted.contains(&HUB));
    assert_eq!(
        top.granted.len() + top.unenterable.len() + top.excluded.len(),
        mgr.stargates.len() - 1,
        "every gate but the hub is offered, unenterable or excluded"
    );

    let mut off_mesh = Vec::new();
    let mut no_mesh = Vec::new();
    for id in top.granted.iter().chain(&top.excluded) {
        let gate = &mgr.stargates[id];
        let nav = format!(
            "{}/../../data/spaces/{}.nav",
            env!("CARGO_MANIFEST_DIR"),
            gate.world_name.to_lowercase()
        );
        let path = std::path::Path::new(&nav);
        if !path.exists() {
            no_mesh.push((*id, gate.world_name.clone()));
            continue;
        }
        let mesh = NavMesh::load(path).expect("navmesh loads");
        let (desired, yaw) = gate.desired_arrival();
        let resolved =
            resolve_arrival_with(Some(&mesh), &respawners, &gate.world_name, desired, yaw);
        if !resolved.is_usable() {
            off_mesh.push(*id);
        }
    }
    if !no_mesh.is_empty() {
        // A checkout without `data/spaces` cannot measure; every offered
        // world has a mesh in the repo.
        assert_eq!(
            no_mesh.len(),
            top.granted.len() + top.excluded.len(),
            "partial meshes: {no_mesh:?}"
        );
        return;
    }
    let (excluded_off, offered_off): (Vec<i32>, Vec<i32>) = off_mesh
        .into_iter()
        .partition(|id| top.excluded.contains(id));
    assert_eq!(
        excluded_off, EXPECTED_EXCLUDED,
        "an excluded gate's arrival became usable — drop it from HUB_EXCLUDED_GATES so the Debug Area offers it again"
    );
    assert_eq!(
        offered_off, EXPECTED_OFF_MESH,
        "the offered gates whose arrival is unstandable changed; re-measure, \
         then update EXPECTED_OFF_MESH and gate-travel.md"
    );
}
