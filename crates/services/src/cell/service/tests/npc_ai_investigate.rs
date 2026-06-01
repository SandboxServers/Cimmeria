//! `npc_ai_investigate` lifecycle: pathfind to POI, dwell, return to Idle.

use crate::cell::space_manager::SpaceManager;
use cimmeria_common::Vector3;
use cimmeria_entity::cell_entity::AiState;
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

fn spawn_npc_at(mgr: &mut SpaceManager, npc_id: u32, pos: [f32; 3]) {
    mgr.spawn_npc(npc_id, "Castle", pos, [0.0; 3]).unwrap();
    if let Some(npc) = mgr.get_entity_mut(npc_id) {
        npc.class_id = 0x04;
        npc.is_player = false;
        if let Some(h) = npc.stats.get_mut(HEALTH) {
            h.update(0, 100, 100);
            h.clear_dirty();
        }
    }
}

/// First tick on Investigating + POI + NPC far from POI → pathfind,
/// queue the waypoint. Dwell is NOT stamped here — that happens on
/// arrival (see `investigate_arrival_stamps_dwell`).
#[tokio::test]
async fn investigate_first_tick_routes_toward_poi() {
    let mut mgr = make_castle_mgr();
    spawn_npc_at(&mut mgr, 200, [0.0; 3]);
    if let Some(npc) = mgr.get_entity_mut(200) {
        npc.ai_state = AiState::Investigating;
        npc.poi = Some(Vector3::new(10.0, 0.0, 10.0));
        npc.investigate_until = None;
    }
    let (tx, _rx) = mpsc::channel(16);

    crate::cell::service::npc_ai::npc_ai_tick(
        &tx,
        &mut mgr,
        &cimmeria_content_engine::chain::ChainEngine::new(),
    )
    .await;

    let npc = mgr.get_entity(200).unwrap();
    assert!(
        !npc.nav_path.is_empty(),
        "Investigate must queue at least one waypoint toward the POI",
    );
    assert!(
        npc.investigate_until.is_none(),
        "Dwell stamps on arrival, not on initial routing — otherwise the effective \
         dwell would be max(0, INVESTIGATE_DWELL_SECS - travel_time)",
    );
}

/// Arrival at POI (close + nav_path empty + no dwell) → stamp dwell.
/// Mirrors the patrol arrival-stamp test.
#[tokio::test]
async fn investigate_arrival_stamps_dwell() {
    let mut mgr = make_castle_mgr();
    spawn_npc_at(&mut mgr, 200, [10.0, 0.0, 10.0]); // at POI
    if let Some(npc) = mgr.get_entity_mut(200) {
        npc.ai_state = AiState::Investigating;
        npc.poi = Some(Vector3::new(10.0, 0.0, 10.0));
        npc.investigate_until = None;
        npc.nav_path.clear();
    }
    let (tx, _rx) = mpsc::channel(16);

    crate::cell::service::npc_ai::npc_ai_tick(
        &tx,
        &mut mgr,
        &cimmeria_content_engine::chain::ChainEngine::new(),
    )
    .await;

    let npc = mgr.get_entity(200).unwrap();
    assert!(
        npc.investigate_until.is_some(),
        "Arrival at POI with no existing dwell must stamp a fresh deadline",
    );
    assert_eq!(
        npc.ai_state,
        AiState::Investigating,
        "Still investigating during the dwell window",
    );
}

/// Elapsed dwell → drop to Idle, clear POI + investigate_until.
#[tokio::test]
async fn investigate_with_elapsed_dwell_returns_to_idle() {
    let mut mgr = make_castle_mgr();
    spawn_npc_at(&mut mgr, 200, [10.0, 0.0, 10.0]);
    if let Some(npc) = mgr.get_entity_mut(200) {
        npc.ai_state = AiState::Investigating;
        npc.poi = Some(Vector3::new(10.0, 0.0, 10.0));
        npc.investigate_until =
            Some(std::time::Instant::now() - std::time::Duration::from_millis(1));
        npc.nav_path.clear(); // at the POI
    }
    let (tx, _rx) = mpsc::channel(16);

    crate::cell::service::npc_ai::npc_ai_tick(
        &tx,
        &mut mgr,
        &cimmeria_content_engine::chain::ChainEngine::new(),
    )
    .await;

    let npc = mgr.get_entity(200).unwrap();
    assert_eq!(npc.ai_state, AiState::Idle);
    assert_eq!(npc.poi, None, "POI must clear after dwell");
    assert_eq!(npc.investigate_until, None);
}

/// Threat preemption: a damaged Investigating NPC transitions to
/// Fighting. The POI persists on the entity (content authors can
/// inspect it post-fight if needed) but `nav_path` is wiped so the
/// fight handler can re-pathfind to the attacker.
#[tokio::test]
async fn investigate_preempted_by_threat_clears_nav_keeps_poi() {
    let mut mgr = make_castle_mgr();
    spawn_npc_at(&mut mgr, 200, [0.0; 3]);
    mgr.create_entity(1, "Castle", [0.0; 3], [0.0; 3]).unwrap();
    if let Some(p) = mgr.get_entity_mut(1) {
        p.is_player = true;
    }
    if let Some(npc) = mgr.get_entity_mut(200) {
        npc.ai_state = AiState::Investigating;
        npc.poi = Some(Vector3::new(20.0, 0.0, 0.0));
        npc.nav_path.push_back(Vector3::new(10.0, 0.0, 0.0));
    }

    let _ = crate::cell::combat::generate_threat(&mut mgr, 1, 200, 50.0);

    let npc = mgr.get_entity(200).unwrap();
    assert_eq!(npc.ai_state, AiState::Fighting);
    assert!(npc.nav_path.is_empty(), "Preemption must clear nav_path");
    assert!(npc.poi.is_some(), "POI persists across preemption");
}

/// POI cleared mid-Investigate (defensive content path) → drop to Idle.
#[tokio::test]
async fn investigate_with_no_poi_drops_to_idle() {
    let mut mgr = make_castle_mgr();
    spawn_npc_at(&mut mgr, 200, [0.0; 3]);
    if let Some(npc) = mgr.get_entity_mut(200) {
        npc.ai_state = AiState::Investigating;
        npc.poi = None;
    }
    let (tx, _rx) = mpsc::channel(16);

    crate::cell::service::npc_ai::npc_ai_tick(
        &tx,
        &mut mgr,
        &cimmeria_content_engine::chain::ChainEngine::new(),
    )
    .await;

    let npc = mgr.get_entity(200).unwrap();
    assert_eq!(npc.ai_state, AiState::Idle);
}

/// **A1 fix pin**: knockback during dwell at the POI must NOT cause
/// an immediate return-to-Idle on re-arrival. Pre-fix: `Some(past)`
/// dwell after the knockback round-trip would hit the
/// "dwell elapsed → return to Idle" branch the moment the NPC walked
/// back. The fix clears `investigate_until` in the "not close"
/// branch so re-arrival re-stamps from scratch.
#[tokio::test]
async fn investigate_knockback_during_dwell_re_stamps_on_re_arrival() {
    let mut mgr = make_castle_mgr();
    spawn_npc_at(&mut mgr, 200, [10.0, 0.0, 10.0]); // start at POI
    let past = std::time::Instant::now() - std::time::Duration::from_secs(60);
    if let Some(npc) = mgr.get_entity_mut(200) {
        npc.ai_state = AiState::Investigating;
        npc.poi = Some(Vector3::new(10.0, 0.0, 10.0));
        npc.investigate_until = Some(past);
        npc.nav_path.clear();
    }
    let (tx, _rx) = mpsc::channel(16);

    // Step 1: knockback — NPC moves far from POI. Handler routes back.
    if let Some(npc) = mgr.get_entity_mut(200) {
        npc.position = Vector3::new(50.0, 0.0, 50.0);
    }
    crate::cell::service::npc_ai::npc_ai_tick(
        &tx,
        &mut mgr,
        &cimmeria_content_engine::chain::ChainEngine::new(),
    )
    .await;

    let after_knockback = mgr.get_entity(200).unwrap();
    assert_eq!(
        after_knockback.ai_state,
        AiState::Investigating,
        "Knockback must not flip the NPC out of Investigating mid-route",
    );
    assert!(
        after_knockback.investigate_until.is_none(),
        "Routing back to the POI must clear the stale dwell",
    );
    assert!(
        !after_knockback.nav_path.is_empty(),
        "Re-route must queue movement back to the POI",
    );

    // Step 2: NPC walks back to POI. Handler observes None dwell →
    // re-stamps a fresh deadline.
    if let Some(npc) = mgr.get_entity_mut(200) {
        npc.position = Vector3::new(10.0, 0.0, 10.0);
        npc.nav_path.clear();
    }
    crate::cell::service::npc_ai::npc_ai_tick(
        &tx,
        &mut mgr,
        &cimmeria_content_engine::chain::ChainEngine::new(),
    )
    .await;

    let after_re_arrival = mgr.get_entity(200).unwrap();
    assert_eq!(
        after_re_arrival.ai_state,
        AiState::Investigating,
        "Re-arrival after knockback must NOT immediately return to Idle",
    );
    assert!(
        after_re_arrival.investigate_until.is_some(),
        "Re-arrival must stamp a fresh dwell",
    );
    assert!(
        after_re_arrival.investigate_until.unwrap() > std::time::Instant::now(),
        "Fresh dwell must be in the future, not the original `past`",
    );
}
