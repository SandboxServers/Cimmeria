//! Phase 7 stubs: Despawning + Submit + Error AI states.

use crate::cell::space_manager::SpaceManager;
use cimmeria_entity::cell_entity::{AiState, MobMovementType};
use cimmeria_entity::stats::HEALTH;
use tokio::sync::mpsc;

fn make_castle_mgr() -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
    )
    .unwrap();
    mgr
}

fn spawn_npc(mgr: &mut SpaceManager, npc_id: u32) {
    mgr.spawn_npc(npc_id, "Castle", [0.0; 3], [0.0; 3]).unwrap();
    if let Some(npc) = mgr.get_entity_mut(npc_id) {
        npc.class_id = 0x04;
        npc.is_player = false;
        if let Some(h) = npc.stats.get_mut(HEALTH) {
            h.update(0, 100, 100);
            h.clear_dirty();
        }
    }
}

/// Despawning → NPC removed from space; AoI sees it leave.
#[tokio::test]
async fn despawning_removes_entity_from_space() {
    let mut mgr = make_castle_mgr();
    spawn_npc(&mut mgr, 200);
    if let Some(npc) = mgr.get_entity_mut(200) {
        npc.ai_state = AiState::Despawning;
    }
    assert!(mgr.get_entity(200).is_some(), "fixture invariant");
    let (tx, _rx) = mpsc::channel(16);

    crate::cell::service::npc_ai::npc_ai_tick(&tx, &mut mgr).await;

    assert!(
        mgr.get_entity(200).is_none(),
        "Despawning tick must remove the entity from the space",
    );
}

/// Submit → combat state cleared, NPC holds. Re-running the tick is
/// cheap (`last_movement_type` cache hits the early-return).
#[tokio::test]
async fn submit_clears_combat_state_and_holds() {
    use crate::cell::combat::BSF_IN_COMBAT;
    let mut mgr = make_castle_mgr();
    spawn_npc(&mut mgr, 200);
    if let Some(npc) = mgr.get_entity_mut(200) {
        npc.ai_state = AiState::Submit;
        npc.threat_list.insert(1, 99.0);
        npc.state_field |= BSF_IN_COMBAT;
        npc.last_movement_type = Some(MobMovementType::CombatAdvance); // simulate prior state
    }
    let (tx, _rx) = mpsc::channel(16);

    crate::cell::service::npc_ai::npc_ai_tick(&tx, &mut mgr).await;

    let npc = mgr.get_entity(200).unwrap();
    assert_eq!(npc.ai_state, AiState::Submit);
    assert!(npc.threat_list.is_empty(), "Submit must drop threat");
    assert_eq!(
        npc.state_field & BSF_IN_COMBAT,
        0,
        "Submit must clear BSF_IN_COMBAT",
    );
    assert_eq!(
        npc.last_movement_type, None,
        "Submit must clear movement-type cache",
    );
}

/// Error → handler is a no-op per tick; NPC sits inert. Pin the
/// "no nav, no broadcast" invariant so a refactor that accidentally
/// re-routes Error doesn't ship.
#[tokio::test]
async fn error_state_is_inert_per_tick() {
    let mut mgr = make_castle_mgr();
    spawn_npc(&mut mgr, 200);
    if let Some(npc) = mgr.get_entity_mut(200) {
        npc.ai_state = AiState::Error;
    }
    let (tx, _rx) = mpsc::channel(16);

    crate::cell::service::npc_ai::npc_ai_tick(&tx, &mut mgr).await;

    let npc = mgr.get_entity(200).unwrap();
    assert_eq!(npc.ai_state, AiState::Error, "Error must persist");
    assert!(npc.nav_path.is_empty(), "Error must not queue movement");
}
