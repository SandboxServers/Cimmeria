//! NA02 detector tests. Each WARN test reproduces the bug shape the
//! detector exists for, on today's code, and fails if the detector is
//! removed (see the revert notes on each test).
//!
//! - [`stale_velocity`] — running in place after `attack_in_place` and after
//!   a leash snap, the throttle, and the moving-NPC negative.
//! - [`leash`] — `enter`, `snap_fallback` and the aggro/leash `loop`.
//! - [`ground`] — `ground_deviation` on a lerped chord over the real
//!   `castle_cellblock.nav`.
//! - [`path`] — `npc_ai.path` rows and the partial path across two mesh
//!   islands.
//! - [`cover`] — `no_cover` reasons and the `cover.coverage` WARN on
//!   today's seed.
//! - [`state`] — `idle_parked`, `cleared_without_exit`, the aggro-scan
//!   rejects, `spawn_off_mesh`, `stuck`, `npc_ai.los`, and teardown.

use std::path::Path;

use cimmeria_common::Vector3;
use cimmeria_entity::cell_entity::AiState;
use cimmeria_entity::navigation::NavMesh;
use cimmeria_entity::stats::HEALTH;

use crate::cell::space_manager::SpaceManager;
use crate::test_support::{Captured, LogCaptureGuard};

mod cover;
mod ground;
mod leash;
mod path;
mod stale_velocity;
mod state;

pub(super) const NPC: u32 = 200;
pub(super) const PLAYER: u32 = 101;

/// A meshless, non-instanced "Castle" space (bounds ±800).
pub(super) fn castle_mgr() -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
    )
    .unwrap();
    mgr
}

/// The real rebuilt Cellblock mesh, or `None` on a checkout without it.
pub(super) fn cellblock_nav() -> Option<NavMesh> {
    let p = Path::new("../../data/spaces/castle_cellblock.nav");
    p.exists()
        .then(|| NavMesh::load(p).expect("castle_cellblock.nav loads"))
}

/// A non-instanced "Castle_CellBlock" space with the real mesh injected.
/// Returns the manager and the space id.
pub(super) fn cellblock_mgr() -> Option<(SpaceManager, u32)> {
    let mesh = cellblock_nav()?;
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" /></Spaces>"#,
    )
    .unwrap();
    let space_id = mgr.space_id_for_world("Castle_CellBlock").unwrap();
    mgr.spaces.get_mut(&space_id).unwrap().navmesh = Some(mesh);
    Some((mgr, space_id))
}

/// A mob (class 0x04, so the ticks see it) at `pos` in `world`, full HP,
/// in `state`, with its spawn at `spawn`.
pub(super) fn add_npc(
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
pub(super) fn add_threat_player(mgr: &mut SpaceManager, world: &str, pos: [f32; 3]) {
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

/// Every captured row with this `target` and `event`.
pub(super) fn rows(logs: &LogCaptureGuard, target: &str, event: &str) -> Vec<Captured> {
    logs.all()
        .into_iter()
        .filter(|c| c.target == target && c.has_field("event", event))
        .collect()
}

pub(super) async fn ai_tick(mgr: &mut SpaceManager) {
    let (tx, _rx) = tokio::sync::mpsc::channel(256);
    crate::cell::service::npc_ai::npc_ai_tick(
        &tx,
        mgr,
        &cimmeria_content_engine::chain::ChainEngine::new(),
    )
    .await;
}

/// One 100 ms movement tick plus the detector pass that follows it in the
/// message loop.
pub(super) fn movement_tick(mgr: &mut SpaceManager) {
    crate::cell::service::ticks::npc_movement_tick(mgr);
    super::movement::after_movement_tick(mgr, std::time::Instant::now());
}
