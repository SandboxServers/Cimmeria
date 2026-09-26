//! The NA02 detector tests' fixtures: a meshless Castle space, the rebuilt
//! Cellblock mesh, one NPC and one threat player, and a real AI tick.
//!
//! Shared by `cell::service::npc_ai::detector_tests` here and by the two
//! detector test files that drive the movement tick in `cimmeria-cell`.

use std::path::Path;

use cimmeria_common::Vector3;
use cimmeria_entity::cell_entity::AiState;
use cimmeria_entity::navigation::NavMesh;
use cimmeria_entity::stats::HEALTH;

use crate::cell::space_manager::SpaceManager;

pub const NPC: u32 = 200;
pub const PLAYER: u32 = 101;

/// A meshless, non-instanced "Castle" space (bounds ±800).
pub fn castle_mgr() -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
    )
    .unwrap();
    // Castle's `resources.worlds` id: cover is indexed per world (NA21).
    mgr.worlds.get_mut("Castle").unwrap().world_id = Some(8);
    mgr
}

/// The real rebuilt Cellblock mesh. `data/spaces/castle_cellblock.nav` is
/// tracked in git and CI loads it, so a missing file is a failure, not a
/// skip: a silent skip would pass every mesh-backed guard vacuously.
pub fn cellblock_nav() -> NavMesh {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/spaces/castle_cellblock.nav");
    NavMesh::load(&p).unwrap_or_else(|e| panic!("load {}: {e}", p.display()))
}

/// A non-instanced "Castle_CellBlock" space with the real mesh injected.
/// Returns the manager and the space id.
pub fn cellblock_mgr() -> (SpaceManager, u32) {
    let mesh = cellblock_nav();
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" /></Spaces>"#,
    )
    .unwrap();
    mgr.worlds.get_mut("Castle_CellBlock").unwrap().world_id = Some(12);
    let space_id = mgr.space_id_for_world("Castle_CellBlock").unwrap();
    mgr.spaces.get_mut(&space_id).unwrap().navmesh = Some(mesh);
    (mgr, space_id)
}

/// A mob (class 0x04, so the ticks see it) at `pos` in `world`, full HP,
/// in `state`, with its spawn at `spawn`.
pub fn add_npc(
    mgr: &mut SpaceManager,
    world: &str,
    pos: [f32; 3],
    spawn: Option<[f32; 3]>,
    state: AiState,
) {
    mgr.create_entity(NPC, world, pos, [0.0; 3]).unwrap();
    let npc = mgr.get_entity_mut(NPC).unwrap();
    npc.is_player = false;
    npc.class_id = 0x04;
    npc.tag = Some("Test_Guard".into());
    npc.template_id = Some(24);
    npc.spawn_position = spawn.map(|s| Vector3::new(s[0], s[1], s[2]));
    crate::cell::service::npc_ai::force_ai_state(npc, state);
    if let Some(h) = npc.stats.get_mut(HEALTH) {
        h.update(0, 100, 100);
        h.clear_dirty();
    }
}

/// A connected player at `pos`, on the NPC's threat list, in the NPC's AoI.
pub fn add_threat_player(mgr: &mut SpaceManager, world: &str, pos: [f32; 3]) {
    mgr.create_entity(PLAYER, world, pos, [0.0; 3]).unwrap();
    let p = mgr.get_entity_mut(PLAYER).unwrap();
    p.is_player = true;
    p.player_id = Some(PLAYER as i32);
    p.faction = 0;
    if let Some(h) = p.stats.get_mut(HEALTH) {
        h.update(0, 100, 100);
        h.clear_dirty();
    }
    mgr.connect_entity(PLAYER);
    let _ = mgr.compute_aoi_changes();
    if let Some(npc) = mgr.get_entity_mut(NPC) {
        npc.threat_list.insert(PLAYER, 10.0);
    }
}

/// One NPC AI tick with no content chains (`NoContentEvents`).
pub async fn ai_tick(mgr: &mut SpaceManager) {
    let (tx, _rx) = tokio::sync::mpsc::channel(256);
    crate::cell::service::npc_ai::npc_ai_tick(
        &tx,
        mgr,
        &cimmeria_cell_world::test_fixtures::NoContentEvents,
    )
    .await;
}
