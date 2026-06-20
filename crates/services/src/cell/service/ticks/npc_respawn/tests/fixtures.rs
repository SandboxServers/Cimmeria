//! Shared test fixtures for the `npc_respawn` tick tests.
//!
//! Split out of the monolithic `npc_respawn/tests.rs` (issue #529) — the
//! fixture bodies are byte-identical to the original; only their visibility
//! was widened to `pub(super)` so the sibling test files can call them.

use super::super::*;
use crate::cell::combat::{BSF_DEAD, BSF_MOVEMENT_LOCK};
use crate::cell::space_manager::SpaceManager;
use cimmeria_entity::cell_entity::AiState;
use cimmeria_entity::stats::{FOCUS, HEALTH};

/// One player + one NPC in a Castle space, both connected and
/// co-located so the AoI tick captures the player as a witness of
/// the NPC. The NPC is in the Dead state with full damage applied
/// (HP=0), BSF_DEAD + BSF_MOVEMENT_LOCK set, interaction_type with
/// `INT_NormalLoot` OR-merged in, and `original_interaction_type_flags`
/// preserving the pre-death snapshot.
pub(super) fn make_mgr_with_dead_npc(
    respawn_secs: Option<u32>,
    respawn_at: Option<std::time::Instant>,
) -> SpaceManager {
    use crate::cell::abilities::INT_NORMAL_LOOT;

    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
    )
    .unwrap();
    // Player witness at the origin.
    mgr.create_entity(1, "Castle", [0.0; 3], [0.0; 3]).unwrap();
    if let Some(p) = mgr.get_entity_mut(1) {
        p.is_player = true;
        p.player_id = Some(100);
    }
    // NPC spawned at (10, 0, 0) facing yaw=1.57 (east-ish — any
    // non-zero heading so the post-respawn direction-restore
    // assertion is meaningful). Corpse stays at this position
    // until the respawn tick snaps it back.
    mgr.spawn_npc(50, "Castle", [10.0, 0.0, 0.0], [0.0, 1.57, 0.0])
        .unwrap();
    if let Some(npc) = mgr.get_entity_mut(50) {
        // Snapshot the pre-death interaction_type, then mimic the
        // death path's OR-merge of INT_NormalLoot.
        npc.original_interaction_type_flags = 1 << 5; // arbitrary content bit
        npc.interaction_type_flags = (1 << 5) | INT_NORMAL_LOOT;
        // Death state.
        npc.set_state_flag(BSF_DEAD);
        npc.set_state_flag(BSF_MOVEMENT_LOCK);
        npc.ai_state = AiState::Dead;
        // HP=0 so the post-respawn assertion that HP=max is
        // meaningful.
        if let Some(hp) = npc.stats.get_mut(HEALTH) {
            hp.set_current(0);
        }
        if let Some(focus) = npc.stats.get_mut(FOCUS) {
            focus.set_current(0);
        }
        // Move the corpse away from the spawn so the position snap
        // is observable.
        npc.position = cimmeria_common::Vector3::new(50.0, 0.0, 50.0);
        // Respawn opt-in (or not, per arg).
        npc.respawn_secs = respawn_secs;
        npc.respawn_at = respawn_at;
        // Stale combat scratch the tick should wipe.
        npc.threat_list.insert(1, 99.0);
        npc.nav_path
            .push_back(cimmeria_common::Vector3::new(123.0, 0.0, 456.0));
    }
    mgr.connect_entity(1);
    let _ = mgr.compute_aoi_changes();
    mgr
}

pub(super) fn drain(rx: &mut mpsc::Receiver<CellToBaseMsg>) -> Vec<CellToBaseMsg> {
    let mut out = Vec::new();
    while let Ok(m) = rx.try_recv() {
        out.push(m);
    }
    out
}
